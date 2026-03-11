/// 会话管理器
///
/// 管理 WebSocket 会话生命周期，包括：
/// - 用户→设备→会话三级映射
/// - 在线状态同步（Redis）
/// - 路由注册（Nacos）
use crate::types::{ClientId, SessionId, UserId};
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// 心跳超时时间（秒）
const HEARTBEAT_TIMEOUT: u64 = 30;

/// WebSocket 会话
#[derive(Debug)]
pub struct Session {
    /// 会话 ID
    pub id: SessionId,
    /// 用户 ID
    pub uid: UserId,
    /// 客户端 ID（设备指纹）
    pub client_id: ClientId,
    /// 发送通道（有界通道，容量 1000）
    pub tx: mpsc::Sender<axum::extract::ws::Message>,
    /// 关闭通道（有界通道，容量 1）
    pub shutdown_tx: mpsc::Sender<()>,
    /// 最后活跃时间（Unix 时间戳，秒）
    last_seen: AtomicU64,
}

impl Session {
    /// 创建新会话
    pub fn new(
        id: SessionId,
        uid: UserId,
        client_id: ClientId,
        tx: mpsc::Sender<axum::extract::ws::Message>,
        shutdown_tx: mpsc::Sender<()>,
    ) -> Self {
        Self {
            id,
            uid,
            client_id,
            tx,
            shutdown_tx,
            last_seen: AtomicU64::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
        }
    }

    /// 更新最后活跃时间
    pub fn touch(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_seen.store(now, Ordering::Relaxed);
    }

    /// 获取最后活跃时间
    pub fn last_seen(&self) -> u64 {
        self.last_seen.load(Ordering::Relaxed)
    }

    /// 发送消息（异步方法，如果通道满了会等待）
    pub async fn send(
        &self,
        msg: axum::extract::ws::Message,
    ) -> Result<(), mpsc::error::SendError<axum::extract::ws::Message>> {
        self.tx.send(msg).await
    }

    /// 尝试发送消息（同步方法，如果通道满了立即返回错误）
    pub fn try_send(
        &self,
        msg: axum::extract::ws::Message,
    ) -> Result<(), mpsc::error::TrySendError<axum::extract::ws::Message>> {
        self.tx.try_send(msg)
    }
}

/// 会话管理器
#[derive(Clone)]
pub struct SessionManager {
    /// 会话 ID → 会话映射
    pub(crate) sessions: Arc<DashMap<SessionId, Arc<Session>>>,
    /// 用户 ID → (客户端 ID → 会话集合) 三级映射
    user_device_sessions: Arc<DashMap<UserId, DashMap<ClientId, HashSet<SessionId>>>>,
    /// 会话 ID → 用户 ID 反向映射
    session_user: Arc<DashMap<SessionId, UserId>>,
    /// 会话 ID → 客户端 ID 反向映射
    session_client: Arc<DashMap<SessionId, ClientId>>,
    /// 是否接受新连接
    accepting_new_connections: Arc<AtomicBool>,
    /// 节点 ID（从环境变量获取）
    node_id: String,
    /// AppState 引用（用于访问 Redis）
    app_state: Option<Arc<fbc_starter::AppState>>,
    /// 本地路由缓存
    local_router_cache: Arc<crate::cache::LocalRouterCache>,
    /// 时间轮（用于高效心跳检查）
    timing_wheel: Arc<crate::websocket::TimingWheel>,
    /// PushService（延迟注入，解决循环依赖）
    push_service: Arc<OnceLock<std::sync::Weak<crate::service::PushService>>>,
}

impl SessionManager {
    /// 创建新的会话管理器
    pub fn new() -> Self {
        let node_id = std::env::var("NODE_ID").unwrap_or_else(|_| "1".to_string());

        let manager = Self {
            sessions: Arc::new(DashMap::new()),
            user_device_sessions: Arc::new(DashMap::new()),
            session_user: Arc::new(DashMap::new()),
            session_client: Arc::new(DashMap::new()),
            accepting_new_connections: Arc::new(AtomicBool::new(true)),
            node_id,
            app_state: None,
            local_router_cache: Arc::new(crate::cache::LocalRouterCache::default()),
            timing_wheel: Arc::new(crate::websocket::TimingWheel::new()),
            push_service: Arc::new(OnceLock::new()),
        };

        // 启动心跳检查任务
        manager.start_heartbeat_check_task();

        manager
    }

    /// 设置 AppState（在初始化后调用）
    pub fn set_app_state(&mut self, app_state: Arc<fbc_starter::AppState>) {
        self.app_state = Some(app_state);
    }

    /// 设置 PushService（延迟注入，解决 SessionManager ↔ PushService 循环依赖）
    pub fn set_push_service(&self, push_service: Arc<crate::service::PushService>) {
        let _ = self.push_service.set(Arc::downgrade(&push_service));
    }

    /// 获取 PushService
    fn get_push_service(&self) -> Option<Arc<crate::service::PushService>> {
        self.push_service.get().and_then(|weak| weak.upgrade())
    }

    /// 启动心跳检查任务（使用时间轮算法）
    fn start_heartbeat_check_task(&self) -> JoinHandle<()> {
        let timing_wheel = self.timing_wheel.clone();
        let manager = self.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1)); // 每秒 tick 一次
            loop {
                interval.tick().await;

                // 时间轮前进一个槽位，获取超时的会话
                let expired_sessions = timing_wheel.tick().await;

                // 清理超时的会话
                for session_id in expired_sessions {
                    warn!("会话超时: session_id={}", session_id);
                    manager.cleanup_session(&session_id);
                }
            }
        })
    }

    /// 设置是否接受新连接
    pub fn set_accepting_new_connections(&self, accepting: bool) {
        self.accepting_new_connections
            .store(accepting, Ordering::Relaxed);
        info!("新连接接入状态: {}", accepting);
    }

    /// 是否接受新连接
    pub fn is_accepting_new_connections(&self) -> bool {
        self.accepting_new_connections.load(Ordering::Relaxed)
    }

    /// 获取会话数量
    pub fn get_session_count(&self) -> usize {
        self.sessions.len()
    }

    /// 刷新会话心跳（更新时间轮）
    ///
    /// # 参数
    /// - `session_id`: 会话 ID
    pub fn refresh_session(&self, session_id: &SessionId) {
        // 更新会话的 last_seen
        if let Some(session) = self.sessions.get(session_id) {
            session.touch();

            // 刷新时间轮
            let timing_wheel = self.timing_wheel.clone();
            let session_id_clone = session_id.clone();
            tokio::spawn(async move {
                timing_wheel.refresh(&session_id_clone, HEARTBEAT_TIMEOUT).await;
            });
        }
    }

    /// 获取用户的所有会话
    pub fn get_user_sessions(&self, uid: UserId) -> Vec<Arc<Session>> {
        let Some(device_map) = self.user_device_sessions.get(&uid) else {
            return vec![];
        };

        // 先收集所有会话 ID，避免借用冲突
        let mut session_ids = Vec::new();
        for entry in device_map.iter() {
            let sessions = entry.value();
            for session_id in sessions.iter() {
                session_ids.push(session_id.clone());
            }
        }

        // 然后从 sessions 中获取会话
        let mut result = Vec::new();
        for session_id in session_ids {
            if let Some(session) = self.sessions.get(&session_id) {
                result.push(session.clone());
            }
        }
        result
    }

    /// 获取所有客户端 ID
    pub fn get_client_ids(&self) -> Vec<ClientId> {
        let mut client_ids = Vec::new();
        for entry in self.user_device_sessions.iter() {
            for client_id in entry.value().iter().map(|e| e.key().clone()) {
                client_ids.push(client_id);
            }
        }
        client_ids
    }

    /// 获取所有在线用户及其会话数信息（用于测试接口）
    pub fn get_online_users_info(&self) -> Vec<crate::routes::test_push::OnlineUserInfo> {
        self.user_device_sessions
            .iter()
            .map(|entry| {
                let uid = *entry.key();
                let session_count: usize = entry
                    .value()
                    .iter()
                    .map(|device| device.value().len())
                    .sum();
                crate::routes::test_push::OnlineUserInfo { uid, session_count }
            })
            .collect()
    }

    /// 注册会话
    ///
    /// # 参数
    /// - `session`: 会话对象（包含 uid 和 client_id）
    pub fn register_session(&self, session: Arc<Session>) {
        let session_id = session.id.clone();
        let uid = session.uid;
        let client_id = session.client_id.clone();

        // 1. 添加到用户→设备→会话映射
        let is_first_device = {
            let device_map = self
                .user_device_sessions
                .entry(uid)
                .or_insert_with(|| DashMap::new());
            let mut sessions = device_map
                .entry(client_id.clone())
                .or_insert_with(|| HashSet::new());
            let was_empty = sessions.is_empty();
            sessions.insert(session_id.clone());
            was_empty
        };

        // 2. 添加到反向索引
        self.session_user.insert(session_id.clone(), uid);
        self.session_client
            .insert(session_id.clone(), client_id.clone());
        self.sessions.insert(session_id.clone(), session);

        // 3. 添加到时间轮
        let timing_wheel = self.timing_wheel.clone();
        let session_id_clone = session_id.clone();
        tokio::spawn(async move {
            timing_wheel.add(session_id_clone, HEARTBEAT_TIMEOUT).await;
        });

        if is_first_device {
            // 注册设备到 Redis 路由表
            let manager = self.clone();
            let node_id = self.node_id.clone();
            let uid_clone = uid;
            let client_id_clone = client_id.clone();

            tokio::spawn(async move {
                if let Err(e) = manager.register_device_to_redis(uid_clone, &client_id_clone, &node_id).await {
                    error!("注册设备到 Redis 失败: uid={}, client_id={}, error={}", uid_clone, client_id_clone, e);
                }
                // 同步上线状态
                if let Err(e) = manager.sync_online(uid_clone, &client_id_clone, true).await {
                    error!("同步上线状态失败: uid={}, client_id={}, error={}", uid_clone, client_id_clone, e);
                }
            });

            info!(
                "会话注册: clientId={}, uid={}, 会话数={}, 已注册到 Redis",
                client_id,
                uid,
                self.get_user_sessions(uid).len()
            );
        } else {
            // 获取当前设备会话数（避免借用冲突）
            let device_session_count = {
                if let Some(device_map) = self.user_device_sessions.get(&uid) {
                    if let Some(sessions) = device_map.get(&client_id) {
                        sessions.len()
                    } else {
                        0
                    }
                } else {
                    0
                }
            };
            info!(
                "新增会话: clientId={}, uid={}, 当前设备会话数={}, 用户总会话数={}",
                client_id,
                uid,
                device_session_count,
                self.get_user_sessions(uid).len()
            );
        }
    }

    /// 清理会话
    pub fn cleanup_session(&self, session_id: &SessionId) {
        // 直接 remove，避免竞态条件
        let Some((_, session)) = self.sessions.remove(session_id) else {
            return;
        };

        let uid = session.uid;
        let client_id = session.client_id.clone();

        // 从时间轮移除
        let timing_wheel = self.timing_wheel.clone();
        let session_id_clone = session_id.clone();
        tokio::spawn(async move {
            timing_wheel.remove(&session_id_clone).await;
        });

        // 发送 Close 消息，通知 writer_task 退出
        // 注意：即使通道满了或 writer_task 已退出，try_send 也不会阻塞
        if let Err(e) = session.try_send(axum::extract::ws::Message::Close(None)) {
            warn!("发送关闭写任务信号失败: session_id={}, error={}", session_id, e);
        } else {
            info!("发送关闭写任务信号: session_id={}", session_id);
        }

        // 发送关闭信号，通知主循环退出
        // 使用 try_send 避免阻塞，如果通道满了就记录警告
        if let Err(e) = session.shutdown_tx.try_send(()) {
            warn!("发送关闭后台循环信号失败: session_id={}, error={}", session_id, e);
        } else {
            info!("发送关闭后台循环信号: session_id={}", session_id);
        }

        // 从反向索引中移除
        self.session_user.remove(session_id);
        self.session_client.remove(session_id);

        // 从用户→设备→会话映射中移除，并统计剩余会话数
        // 注意：DashMap 使用分片锁，每个 shard 内部有 RwLock
        // 必须先释放 get_mut 的 RefMut，才能调用 remove，避免死锁
        let (should_remove_device, remaining_sessions) = {
            let mut count = 0usize;
            let mut should_remove = false;
            if let Some(device_map) = self.user_device_sessions.get(&uid) {
                // 先检查是否需要移除设备（使用读锁）
                if let Some(sessions) = device_map.get(&client_id) {
                    if sessions.len() == 1 && sessions.contains(session_id) {
                        should_remove = true;
                    }
                }

                // 移除会话（get_mut 会获取写锁，作用域结束后自动释放）
                if let Some(mut sessions) = device_map.get_mut(&client_id) {
                    sessions.remove(session_id);
                }
                // 这里 get_mut 的 RefMut 已经被 drop，写锁已释放

                // 统计剩余会话数（使用读锁）
                for entry in device_map.iter() {
                    count += entry.value().len();
                }
            }
            (should_remove, count)
        };

        // 移除设备（必须在 get_mut 的 RefMut 释放后）
        if should_remove_device {
            if let Some(device_map) = self.user_device_sessions.get_mut(&uid) {
                device_map.remove(&client_id);
                if device_map.is_empty() {
                    drop(device_map); // 显式释放写锁
                    self.user_device_sessions.remove(&uid);
                }
            }
        }

        // 如果是最后一个会话，清理路由和在线状态
        if remaining_sessions == 0 {
            // 从 Redis 中注销设备 + 同步下线状态
            let manager = self.clone();
            let uid_clone = uid;
            let client_id_clone = client_id.clone();
            tokio::spawn(async move {
                if let Err(e) = manager.unregister_device_from_redis(uid_clone, &client_id_clone).await {
                    error!("从 Redis 注销设备失败: uid={}, client_id={}, error={}", uid_clone, client_id_clone, e);
                }
                // 同步下线状态
                if let Err(e) = manager.sync_online(uid_clone, &client_id_clone, false).await {
                    error!("同步下线状态失败: uid={}, client_id={}, error={}", uid_clone, client_id_clone, e);
                }
            });

            info!("用户 {} 的所有会话已断开，已从 Redis 注销", uid);
        } else {
            info!(
                "会话清理: session_id={}, uid={}, 剩余会话数={}",
                session_id, uid, remaining_sessions
            );
        }
    }

    /// 发送消息到设备
    ///
    /// # 参数
    /// - `uid`: 用户 ID
    /// - `client_id`: 客户端 ID
    /// - `msg`: 消息
    pub async fn send_to_device(
        &self,
        uid: UserId,
        client_id: &ClientId,
        msg: axum::extract::ws::Message,
    ) -> usize {
        let Some(device_map) = self.user_device_sessions.get(&uid) else {
            return 0;
        };

        let Some(sessions) = device_map.get(client_id) else {
            return 0;
        };

        let mut sent = 0;
        for session_id in sessions.iter() {
            if let Some(session) = self.sessions.get(session_id) {
                if session.send(msg.clone()).await.is_ok() {
                    sent += 1;
                }
            }
        }
        sent
    }

    /// 发送消息到用户的所有设备
    pub async fn send_to_user(&self, uid: UserId, msg: axum::extract::ws::Message) -> usize {
        let mut sent = 0;
        for session in self.get_user_sessions(uid) {
            if session.send(msg.clone()).await.is_ok() {
                sent += 1;
            }
        }
        sent
    }

    /// 获取节点 ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// 注册设备到 Redis 路由表
    ///
    /// 对应 Java NacosSessionRegistry.addUserRoute
    /// 1. 设备指纹→节点映射（全局 Hash）
    /// 2. 节点→设备指纹映射（节点 Set）
    ///
    /// # 参数
    /// - `uid`: 用户 ID
    /// - `client_id`: 客户端 ID
    /// - `node_id`: 节点 ID
    async fn register_device_to_redis(
        &self,
        uid: UserId,
        client_id: &str,
        node_id: &str,
    ) -> anyhow::Result<()> {
        use crate::cache::RouterCacheKeyBuilder;
        use redis::AsyncCommands;

        let device_field = format!("{}:{}", uid, client_id);

        let app_state = self.app_state.as_ref()
            .ok_or_else(|| anyhow::anyhow!("AppState 未初始化"))?;
        let mut conn = app_state.redis().await?;

        // 1. 设备→节点映射（全局 Hash）
        let cache_key = RouterCacheKeyBuilder::build_device_node_map(String::new());
        let _: () = conn.hset(&cache_key.key, &device_field, node_id).await?;

        // 2. 节点→设备映射（节点 Set）
        let node_devices_key = RouterCacheKeyBuilder::build_node_devices(node_id);
        let _: () = conn.sadd(&node_devices_key.key, &device_field).await?;

        // 更新本地缓存
        self.local_router_cache.set(uid, client_id, node_id.to_string());

        info!(
            "设备已注册到 Redis: uid={}, client_id={}, node_id={}",
            uid, client_id, node_id
        );

        Ok(())
    }

    // ========== 在线状态同步（P1: Presence Management） ==========

    /// 同步在线状态
    ///
    /// 对应 Java SessionManager.syncOnline
    /// 功能：
    /// 1. 维护全局在线设备 ZSet 和全局在线用户 ZSet
    /// 2. 首次设备上线时：添加用户在线状态 + 更新群组在线成员 + 推送上下线通知
    /// 3. 最后设备下线时：移除用户在线状态 + 更新群组在线成员 + 推送上下线通知
    pub async fn sync_online(
        &self,
        uid: UserId,
        client_id: &str,
        online: bool,
    ) -> anyhow::Result<()> {
        use crate::cache::PresenceCacheKeyBuilder;
        use crate::enums::WsMsgTypeEnum;
        use redis::AsyncCommands;

        let app_state = self.app_state.as_ref()
            .ok_or_else(|| anyhow::anyhow!("AppState 未初始化"))?;
        let mut conn = app_state.redis().await?;

        // 1. 生成用户设备 key、全局在线状态 key
        let device_key = format!("{}:{}", uid, client_id);
        let online_devices_key = PresenceCacheKeyBuilder::global_online_devices_key();
        let online_users_key = PresenceCacheKeyBuilder::global_online_users_key();

        // 2. 获取用户所有群组
        let room_ids = self.get_room_ids(uid).await?;

        // 3. 检查是否为首个/最后一个设备（原子操作）
        let no_other_devices = self.is_first_or_last_device(uid, &device_key).await?;

        if online {
            // 上线逻辑
            let millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as f64;
            let _: () = conn.zadd(&online_devices_key.key, &device_key, millis).await?;

            // 仅首个设备登录时才添加用户在线状态
            if no_other_devices {
                let _: () = conn.zadd(&online_users_key.key, uid, millis).await?;
                self.update_group_presence(&room_ids, uid, true).await?;
                self.push_device_status_change(
                    &room_ids,
                    uid,
                    client_id,
                    WsMsgTypeEnum::Online.as_i32(),
                    &online_users_key.key,
                ).await;
            }

            info!("用户上线同步完成: uid={}, client_id={}, first_device={}", uid, client_id, no_other_devices);
        } else {
            // 下线逻辑
            let _: () = conn.zrem(&online_devices_key.key, &device_key).await?;

            // 所有设备都下线后移除用户的在线状态
            if no_other_devices {
                let _: () = conn.zrem(&online_users_key.key, uid).await?;
                self.update_group_presence(&room_ids, uid, false).await?;
                self.push_device_status_change(
                    &room_ids,
                    uid,
                    client_id,
                    WsMsgTypeEnum::Offline.as_i32(),
                    &online_users_key.key,
                ).await;
            }

            info!("用户下线同步完成: uid={}, client_id={}, last_device={}", uid, client_id, no_other_devices);
        }

        Ok(())
    }

    /// 检查是否为首个或最后一个在线设备
    ///
    /// 扫描全局在线设备 ZSet，查找同一 uid 的其他设备
    /// 返回 true 表示没有其他设备在线（即首次上线或最后下线）
    async fn is_first_or_last_device(
        &self,
        uid: UserId,
        exclude_device_key: &str,
    ) -> anyhow::Result<bool> {
        use crate::cache::PresenceCacheKeyBuilder;
        use redis::AsyncCommands;

        let app_state = self.app_state.as_ref()
            .ok_or_else(|| anyhow::anyhow!("AppState 未初始化"))?;
        let mut conn = app_state.redis().await?;

        let online_devices_key = PresenceCacheKeyBuilder::global_online_devices_key();
        let prefix = format!("{}:", uid);

        // 分批获取设备列表
        let batch_size: isize = 1000;
        let total: i64 = conn.zcard(&online_devices_key.key).await?;

        let mut offset: isize = 0;
        while (offset as i64) < total {
            let devices: Vec<String> = conn.zrangebyscore_limit(
                &online_devices_key.key,
                f64::NEG_INFINITY,
                f64::INFINITY,
                offset,
                batch_size,
            ).await?;

            for device in &devices {
                if device.starts_with(&prefix) && device != exclude_device_key {
                    return Ok(false); // 发现其他设备
                }
            }

            offset += batch_size;
        }

        Ok(true)
    }

    /// 更新群组在线状态
    ///
    /// 对应 Java SessionManager.updateGroupPresence
    /// - 上线：将用户添加到各群的在线成员 Set + 更新用户在线群组映射
    /// - 下线：将用户从各群的在线成员 Set 移除 + 清理用户在线群组映射
    async fn update_group_presence(
        &self,
        room_ids: &[u64],
        uid: UserId,
        online: bool,
    ) -> anyhow::Result<()> {
        use crate::cache::PresenceCacheKeyBuilder;
        use redis::AsyncCommands;

        if room_ids.is_empty() {
            return Ok(());
        }

        let app_state = self.app_state.as_ref()
            .ok_or_else(|| anyhow::anyhow!("AppState 未初始化"))?;
        let mut conn = app_state.redis().await?;

        // 批量更新群组在线状态
        for &room_id in room_ids {
            let online_group_key = PresenceCacheKeyBuilder::online_group_members_key(room_id);
            if online {
                let _: () = conn.sadd(&online_group_key.key, uid).await?;
            } else {
                let _: () = conn.srem(&online_group_key.key, uid).await?;
            }
        }

        // 更新用户群组在线映射
        let online_user_groups_key = PresenceCacheKeyBuilder::online_user_groups_key(uid);
        if online {
            for &room_id in room_ids {
                let _: () = conn.sadd(&online_user_groups_key.key, room_id).await?;
            }
        } else {
            for &room_id in room_ids {
                let _: () = conn.srem(&online_user_groups_key.key, room_id).await?;
            }
        }

        Ok(())
    }

    /// 获取用户所有群聊 room_id
    ///
    /// 对应 Java SessionManager.getRoomIds
    async fn get_room_ids(&self, uid: UserId) -> anyhow::Result<Vec<u64>> {
        use crate::cache::PresenceCacheKeyBuilder;
        use redis::AsyncCommands;

        let app_state = self.app_state.as_ref()
            .ok_or_else(|| anyhow::anyhow!("AppState 未初始化"))?;
        let mut conn = app_state.redis().await?;

        let ug_key = PresenceCacheKeyBuilder::user_groups_key(uid);
        let members: Vec<String> = conn.smembers(&ug_key.key).await?;

        let room_ids: Vec<u64> = members
            .iter()
            .filter_map(|s| s.parse::<u64>().ok())
            .collect();

        Ok(room_ids)
    }

    /// 推送设备状态变更通知
    ///
    /// 对应 Java SessionManager.pushDeviceStatusChange
    /// 1. 通知所有反向好友：该用户上/下线 + 好友在线人数
    /// 2. 通知所有所在群的在线成员：该用户上/下线 + 群在线人数
    async fn push_device_status_change(
        &self,
        room_ids: &[u64],
        uid: UserId,
        client_id: &str,
        notify_type: i32,
        online_key: &str,
    ) {
        use crate::cache::{FriendCacheKeyBuilder, PresenceCacheKeyBuilder};
        use crate::model::vo::ws_online_notify::WSOnlineNotify;
        use crate::model::ws_base_resp::WsBaseResp;
        use redis::AsyncCommands;

        let push_service = match self.get_push_service() {
            Some(ps) => ps,
            None => {
                warn!("PushService 未注入，跳过状态变更通知: uid={}", uid);
                return;
            }
        };

        let app_state = match self.app_state.as_ref() {
            Some(state) => state,
            None => {
                error!("AppState 未初始化，跳过状态变更通知: uid={}", uid);
                return;
            }
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // ===== 1. 好友上下线通知 =====
        let mut conn = match app_state.redis().await {
            Ok(c) => c,
            Err(e) => {
                error!("获取 Redis 连接失败: {}", e);
                return;
            }
        };

        // 获取反向好友列表（需要知道该用户在线状态的 uid）
        let reverse_friends_key = FriendCacheKeyBuilder::reverse_friends_key(uid);
        let friends: Vec<String> = match conn.smembers::<_, Vec<String>>(&reverse_friends_key.key).await {
            Ok(f) => f,
            Err(e) => {
                error!("获取反向好友列表失败: uid={}, error={}", uid, e);
                Vec::new()
            }
        };

        for friend_str in &friends {
            let friend_uid = match friend_str.parse::<u64>() {
                Ok(id) => id,
                Err(_) => continue,
            };

            // 获取该好友的所有好友列表
            let friends_key = FriendCacheKeyBuilder::user_friends_key(friend_uid);
            let his_friends: Vec<String> = match conn.smembers::<_, Vec<String>>(&friends_key.key).await {
                Ok(f) => f,
                Err(_) => continue,
            };

            // 管道批量查询分数（判断在线）
            let mut online_count = 0i64;
            for his_friend_str in &his_friends {
                let score: Option<f64> = conn.zscore(online_key, his_friend_str).await.unwrap_or(None);
                if score.is_some() {
                    online_count += 1;
                }
            }

            // 构建好友推送消息
            let notify = WSOnlineNotify::friend_notify(uid, client_id.to_string(), now, online_count);
            let resp = match WsBaseResp::from_data(notify_type, &notify) {
                Ok(r) => r,
                Err(e) => {
                    error!("序列化好友通知失败: {}", e);
                    continue;
                }
            };

            // 定向推送给好友
            if let Err(e) = push_service.send_push_msg_single(resp, friend_uid, uid).await {
                error!("推送好友上下线通知失败: friend_uid={}, error={}", friend_uid, e);
            }
        }

        // ===== 2. 群组上下线通知 =====
        if room_ids.is_empty() {
            return;
        }

        // 批量获取各群在线人数
        let mut room_counts: Vec<(u64, i64)> = Vec::new();
        for &room_id in room_ids {
            let online_group_key = PresenceCacheKeyBuilder::online_group_members_key(room_id);
            let count: i64 = conn.scard(&online_group_key.key).await.unwrap_or(0);
            if count > 0 {
                room_counts.push((room_id, count));
            }
        }

        // 逐群推送给在线成员
        const BATCH_SIZE: usize = 200;
        for (room_id, count) in room_counts {
            let online_group_key = PresenceCacheKeyBuilder::online_group_members_key(room_id);
            let member_ids: Vec<String> = match conn.smembers::<_, Vec<String>>(&online_group_key.key).await {
                Ok(m) => m,
                Err(_) => continue,
            };

            let notify = WSOnlineNotify::group_notify(room_id, uid, client_id.to_string(), now, count);
            let resp = match WsBaseResp::from_data(notify_type, &notify) {
                Ok(r) => r,
                Err(e) => {
                    error!("序列化群组通知失败: {}", e);
                    continue;
                }
            };

            // 分批发送
            let member_uid_list: Vec<u64> = member_ids
                .iter()
                .filter_map(|s| s.parse::<u64>().ok())
                .collect();

            for chunk in member_uid_list.chunks(BATCH_SIZE) {
                if let Err(e) = push_service.send_push_msg(resp.clone(), chunk.to_vec(), uid).await {
                    error!("推送群组上下线通知失败: room_id={}, error={}", room_id, e);
                }
            }
        }
    }

    /// 从 Redis 路由表注销设备
    ///
    /// 对应 Java NacosSessionRegistry.removeDeviceRoute
    /// 1. 清理设备→节点映射（全局 Hash）
    /// 2. 清理节点→设备映射（节点 Set）
    ///
    /// # 参数
    /// - `uid`: 用户 ID
    /// - `client_id`: 客户端 ID
    async fn unregister_device_from_redis(&self, uid: UserId, client_id: &str) -> anyhow::Result<()> {
        use crate::cache::RouterCacheKeyBuilder;
        use redis::AsyncCommands;

        let device_field = format!("{}:{}", uid, client_id);

        let app_state = self.app_state.as_ref()
            .ok_or_else(|| anyhow::anyhow!("AppState 未初始化"))?;
        let mut conn = app_state.redis().await?;

        // 1. 清理设备→节点映射（全局 Hash）
        let cache_key = RouterCacheKeyBuilder::build_device_node_map(String::new());
        let _: () = conn.hdel(&cache_key.key, &device_field).await?;

        // 2. 清理节点→设备映射（节点 Set）
        let node_devices_key = RouterCacheKeyBuilder::build_node_devices(&self.node_id);
        let _: () = conn.srem(&node_devices_key.key, &device_field).await?;

        // 删除本地缓存
        self.local_router_cache.remove(uid, client_id);

        info!(
            "设备已从 Redis 注销: uid={}, client_id={}",
            uid, client_id
        );

        Ok(())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

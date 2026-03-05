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
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// 心跳超时时间（秒）
const HEARTBEAT_TIMEOUT: u64 = 30;

/// 心跳检查间隔（秒）
const HEARTBEAT_CHECK_INTERVAL: u64 = 10;

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
        };

        // 启动心跳检查任务
        manager.start_heartbeat_check_task();

        manager
    }

    /// 设置 AppState（在初始化后调用）
    pub fn set_app_state(&mut self, app_state: Arc<fbc_starter::AppState>) {
        self.app_state = Some(app_state);
    }

    /// 启动心跳检查任务
    fn start_heartbeat_check_task(&self) -> JoinHandle<()> {
        let sessions = self.sessions.clone();
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(HEARTBEAT_CHECK_INTERVAL));
            loop {
                interval.tick().await;

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                // 先收集超时的会话 ID，避免在迭代时修改 sessions
                let mut expired_sessions = Vec::new();
                for entry in sessions.iter() {
                    let session = entry.value();
                    let last_seen = session.last_seen();
                    if now.saturating_sub(last_seen) > HEARTBEAT_TIMEOUT {
                        expired_sessions.push(entry.key().clone());
                    }
                }

                // 统一清理超时的会话
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

        // TODO: 如果是最后一个会话，清理路由和在线状态
        if remaining_sessions == 0 {
            // 从 Redis 中注销设备
            let manager = self.clone();
            let uid_clone = uid;
            let client_id_clone = client_id.clone();
            tokio::spawn(async move {
                if let Err(e) = manager.unregister_device_from_redis(uid_clone, &client_id_clone).await {
                    error!("从 Redis 注销设备失败: uid={}, client_id={}, error={}", uid_clone, client_id_clone, e);
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

        // 构建 Redis Hash 键
        let cache_key = RouterCacheKeyBuilder::build_device_node_map(String::new());
        let field = format!("{}:{}", uid, client_id);

        // 获取 Redis 连接并设置映射
        let app_state = self.app_state.as_ref()
            .ok_or_else(|| anyhow::anyhow!("AppState 未初始化"))?;
        let mut conn = app_state.redis().await?;
        let _: () = conn.hset(&cache_key.key, &field, node_id).await?;

        info!(
            "设备已注册到 Redis: uid={}, client_id={}, node_id={}, key={}, field={}",
            uid, client_id, node_id, cache_key.key, field
        );

        Ok(())
    }

    /// 从 Redis 路由表注销设备
    ///
    /// # 参数
    /// - `uid`: 用户 ID
    /// - `client_id`: 客户端 ID
    async fn unregister_device_from_redis(&self, uid: UserId, client_id: &str) -> anyhow::Result<()> {
        use crate::cache::RouterCacheKeyBuilder;
        use redis::AsyncCommands;

        // 构建 Redis Hash 键
        let cache_key = RouterCacheKeyBuilder::build_device_node_map(String::new());
        let field = format!("{}:{}", uid, client_id);

        // 获取 Redis 连接并删除映射
        let app_state = self.app_state.as_ref()
            .ok_or_else(|| anyhow::anyhow!("AppState 未初始化"))?;
        let mut conn = app_state.redis().await?;
        let _: () = conn.hdel(&cache_key.key, &field).await?;

        info!(
            "设备已从 Redis 注销: uid={}, client_id={}, key={}, field={}",
            uid, client_id, cache_key.key, field
        );

        Ok(())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

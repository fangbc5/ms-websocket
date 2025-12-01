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
use tokio::sync::mpsc;
use tracing::info;

/// WebSocket 会话
#[derive(Debug)]
pub struct Session {
    /// 会话 ID
    pub id: SessionId,
    /// 用户 ID
    pub uid: UserId,
    /// 客户端 ID（设备指纹）
    pub client_id: ClientId,
    /// 发送通道
    pub tx: mpsc::UnboundedSender<axum::extract::ws::Message>,
    /// 最后活跃时间（Unix 时间戳，秒）
    last_seen: AtomicU64,
}

impl Session {
    /// 创建新会话
    pub fn new(
        id: SessionId,
        uid: UserId,
        client_id: ClientId,
        tx: mpsc::UnboundedSender<axum::extract::ws::Message>,
    ) -> Self {
        Self {
            id,
            uid,
            client_id,
            tx,
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

    /// 发送消息
    pub fn send(
        &self,
        msg: axum::extract::ws::Message,
    ) -> Result<(), mpsc::error::SendError<axum::extract::ws::Message>> {
        self.tx.send(msg)
    }
}

/// 会话管理器
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
}

impl SessionManager {
    /// 创建新的会话管理器
    pub fn new() -> Self {
        let node_id = std::env::var("NODE_ID").unwrap_or_else(|_| "1".to_string());

        Self {
            sessions: Arc::new(DashMap::new()),
            user_device_sessions: Arc::new(DashMap::new()),
            session_user: Arc::new(DashMap::new()),
            session_client: Arc::new(DashMap::new()),
            accepting_new_connections: Arc::new(AtomicBool::new(true)),
            node_id,
        }
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
            // TODO: 注册到 Nacos 路由
            // TODO: 同步在线状态到 Redis
            info!(
                "会话注册: clientId={}, uid={}, 会话数={}",
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
        let Some((_, session)) = self.sessions.remove(session_id) else {
            return;
        };

        let uid = session.uid;
        let client_id = session.client_id.clone();

        // 从反向索引中移除
        self.session_user.remove(session_id);
        self.session_client.remove(session_id);

        // 从用户→设备→会话映射中移除
        if let Some(device_map) = self.user_device_sessions.get_mut(&uid) {
            if let Some(mut sessions) = device_map.get_mut(&client_id) {
                sessions.remove(session_id);
                if sessions.is_empty() {
                    drop(sessions);
                    device_map.remove(&client_id);
                }
            }
            if device_map.is_empty() {
                drop(device_map);
                self.user_device_sessions.remove(&uid);
            }
        }

        // TODO: 如果是最后一个会话，清理路由和在线状态
        let remaining_sessions = self.get_user_sessions(uid).len();
        if remaining_sessions == 0 {
            // TODO: 从 Nacos 路由中移除
            // TODO: 同步离线状态到 Redis
            info!("用户 {} 的所有会话已断开", uid);
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
    pub fn send_to_device(
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
                if session.send(msg.clone()).is_ok() {
                    sent += 1;
                }
            }
        }
        sent
    }

    /// 发送消息到用户的所有设备
    pub fn send_to_user(&self, uid: UserId, msg: axum::extract::ws::Message) -> usize {
        let mut sent = 0;
        for session in self.get_user_sessions(uid) {
            if session.send(msg.clone()).is_ok() {
                sent += 1;
            }
        }
        sent
    }

    /// 获取节点 ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

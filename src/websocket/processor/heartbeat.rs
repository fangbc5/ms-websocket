/// 心跳消息处理器
use crate::model::ws_base_resp::WsBaseReq;
use crate::types::{ClientId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
use std::sync::Arc;
use tracing::info;

/// 心跳处理器
///
/// 处理客户端发送的心跳消息，更新会话活跃时间
pub struct HeartbeatProcessor;

impl HeartbeatProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl MessageProcessor for HeartbeatProcessor {
    fn supports(&self, req: &WsBaseReq) -> bool {
        // 心跳消息类型通常是 "1" 或 "heartbeat"
        req.r#type == "1" || req.r#type == "heartbeat"
    }

    async fn process(
        &self,
        session: &Arc<Session>,
        session_id: &SessionId,
        uid: UserId,
        _client_id: &ClientId,
        _req: WsBaseReq,
    ) {
        // 更新会话活跃时间
        session.touch();
        info!("收到用户 {} 的心跳，会话: {}", uid, session_id);
    }
}

impl Default for HeartbeatProcessor {
    fn default() -> Self {
        Self::new()
    }
}

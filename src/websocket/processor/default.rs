/// 默认消息处理器
use crate::model::ws_base_resp::WsBaseReq;
use crate::types::{ClientId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
use std::sync::Arc;
use tracing::warn;

/// 默认消息处理器
///
/// 处理所有未被其他处理器处理的消息
/// 通常放在处理链的最后
pub struct DefaultMessageProcessor;

impl DefaultMessageProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl MessageProcessor for DefaultMessageProcessor {
    fn supports(&self, _req: &WsBaseReq) -> bool {
        // 默认处理器总是返回 true，作为兜底处理器
        true
    }

    async fn process(
        &self,
        _session: &Arc<Session>,
        session_id: &SessionId,
        uid: UserId,
        _client_id: &ClientId,
        req: WsBaseReq,
    ) {
        warn!(
            "未处理的消息: type={}, uid={}, session_id={}",
            req.r#type, uid, session_id
        );
    }
}

impl Default for DefaultMessageProcessor {
    fn default() -> Self {
        Self::new()
    }
}

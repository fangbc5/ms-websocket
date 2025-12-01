/// 已读消息处理器
use crate::model::dto::ReadMessageDto;
use crate::model::ws_base_resp::WsBaseReq;
use crate::types::{ClientId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
use fbc_starter::AppState;
use std::sync::Arc;
use tracing::info;

/// 已读消息处理器
///
/// 处理用户已读消息
pub struct ReadProcessor {
    app_state: Arc<AppState>,
}

impl ReadProcessor {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }
}

#[async_trait::async_trait]
impl MessageProcessor for ReadProcessor {
    fn supports(&self, req: &WsBaseReq) -> bool {
        req.r#type == "read" || req.r#type == "READ"
    }

    async fn process(
        &self,
        _session: &Arc<Session>,
        _session_id: &SessionId,
        uid: UserId,
        _client_id: &ClientId,
        req: WsBaseReq,
    ) {
        // 解析已读消息 DTO
        let mut read: ReadMessageDto = match serde_json::from_value(req.data.clone()) {
            Ok(read) => read,
            Err(e) => {
                tracing::warn!("解析已读消息失败: {}", e);
                return;
            }
        };

        // 设置用户 ID
        read.uid = Some(uid);

        info!(
            "收到已读消息: uid={}, room_id={}, msg_ids={:?}",
            uid, read.room_id, read.msg_ids
        );

        // TODO: 发送到 Kafka 主题 MSG_PUSH_READ_TOPIC
        if let Ok(producer) = self.app_state.message_producer() {
            let message = fbc_starter::Message::new(
                "msg_push_read".to_string(),
                uid.to_string(),
                serde_json::to_value(&read).unwrap_or_default(),
            );
            if let Err(e) = producer.publish("msg_push_read_topic", message).await {
                tracing::error!("发送已读消息到 Kafka 失败: {}", e);
            }
        }
    }
}

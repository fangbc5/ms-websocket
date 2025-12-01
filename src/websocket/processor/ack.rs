/// 消息确认处理器
use crate::model::dto::AckMessageDto;
use crate::model::ws_base_resp::WsBaseReq;
use crate::types::{ClientId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
// use axum::extract::ws::Message;
use fbc_starter::AppState;
use std::sync::Arc;
use tracing::info;

/// 消息确认处理器
///
/// 处理客户端确认收到消息
pub struct AckProcessor {
    app_state: Arc<AppState>,
}

impl AckProcessor {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }
}

#[async_trait::async_trait]
impl MessageProcessor for AckProcessor {
    fn supports(&self, req: &WsBaseReq) -> bool {
        req.r#type == "ack" || req.r#type == "ACK"
    }

    async fn process(
        &self,
        _session: &Arc<Session>,
        _session_id: &SessionId,
        uid: UserId,
        _client_id: &ClientId,
        req: WsBaseReq,
    ) {
        // 解析消息确认 DTO
        let mut ack: AckMessageDto = match serde_json::from_value(req.data.clone()) {
            Ok(ack) => ack,
            Err(e) => {
                tracing::warn!("解析消息确认失败: {}", e);
                return;
            }
        };

        // 设置用户 ID
        ack.uid = Some(uid);

        info!("收到消息确认: uid={}, msg_id={:?}", uid, ack.msg_id);

        // TODO: 发送到 Kafka 主题 MSG_PUSH_ACK_TOPIC
        // 这里需要从 app_state 获取 Kafka producer
        if let Ok(producer) = self.app_state.message_producer() {
            let message = fbc_starter::Message::new(
                "msg_push_ack".to_string(),
                uid.to_string(),
                serde_json::to_value(&ack).unwrap_or_default(),
            );
            if let Err(e) = producer.publish("msg_push_ack_topic", message).await {
                tracing::error!("发送消息确认到 Kafka 失败: {}", e);
            }
        }
    }
}

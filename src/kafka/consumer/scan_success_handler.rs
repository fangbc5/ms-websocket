/// 扫码成功消息处理器
///
/// 将扫码成功的信息发送给对应的用户，等待授权
use async_trait::async_trait;
use fbc_starter::{KafkaMessageHandler, Message};
use std::sync::Arc;
use tracing::{error, info};

use crate::model::dto::scan_success_message_dto::ScanSuccessMessageDTO;
use crate::websocket::SessionManager;

pub struct ScanSuccessHandler {
    session_manager: Arc<SessionManager>,
}

impl ScanSuccessHandler {
    pub fn new(session_manager: Arc<SessionManager>) -> Self {
        Self { session_manager }
    }

    /// 处理扫码成功消息
    async fn handle_scan_success(&self, dto: ScanSuccessMessageDTO) {
        info!("收到扫码成功消息: code={}", dto.code);

        // TODO: 扫码成功逻辑
        // 1. 查找对应的 WebSocket 会话
        // 2. 发送扫码成功消息给用户
        // 3. 等待用户授权确认

        // TODO: 使用 SessionManager 发送扫码成功消息
        info!("处理扫码成功: code={}", dto.code);
    }
}

#[async_trait]
impl KafkaMessageHandler for ScanSuccessHandler {
    fn topics(&self) -> Vec<String> {
        vec!["user_scan_send_msg".to_string()]
    }

    async fn handle(&self, message: Message) {
        match serde_json::from_value::<ScanSuccessMessageDTO>(message.data) {
            Ok(dto) => {
                self.handle_scan_success(dto).await;
            }
            Err(e) => {
                error!("解析扫码成功消息失败: {}, topic={}", e, message.topic);
            }
        }
    }
}

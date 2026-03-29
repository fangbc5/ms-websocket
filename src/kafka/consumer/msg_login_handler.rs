/// 用户登录消息处理器
///
/// 在本地服务上找寻对应 channel，将对应用户登录，并触发所有用户收到上线事件
use async_trait::async_trait;
use fbc_starter::{KafkaMessageHandler, Message};
use std::sync::Arc;
use tracing::{error, info};

use crate::model::dto::login_message_dto::LoginMessageDTO;
use crate::websocket::SessionManager;

pub struct MsgLoginHandler {
    _session_manager: Arc<SessionManager>,
}

impl MsgLoginHandler {
    pub fn new(session_manager: Arc<SessionManager>) -> Self {
        Self { _session_manager: session_manager }
    }

    /// 处理用户登录消息
    async fn handle_login(&self, dto: LoginMessageDTO) {
        info!("收到用户登录消息: uid={}, code={}", dto.uid, dto.code);

        // TODO: 尝试登录逻辑
        // 1. 查找对应的 WebSocket 会话
        // 2. 通知用户登录成功/失败
        // 3. 触发其他用户收到上线事件

        // TODO: 使用 SessionManager 发送登录成功消息
        // 可以通过 session_manager.send_to_user() 发送消息
        info!("处理用户登录: uid={}, code={}", dto.uid, dto.code);
    }
}

#[async_trait]
impl KafkaMessageHandler for MsgLoginHandler {
    fn topics(&self) -> Vec<String> {
        vec!["user_login_send_msg".to_string()]
    }

    fn group_id(&self) -> String {
        "ws-login-group".to_string()
    }

    async fn handle(&self, message: Message) {
        match serde_json::from_value::<LoginMessageDTO>(message.data) {
            Ok(dto) => {
                self.handle_login(dto).await;
            }
            Err(e) => {
                error!("解析用户登录消息失败: {}, topic={}", e, message.topic);
            }
        }
    }
}

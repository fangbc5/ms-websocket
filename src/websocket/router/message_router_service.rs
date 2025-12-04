/// 消息路由服务
///
/// 消息中转工具，用于其他没有依赖路由服务的服务（如 oauth 服务）
/// 需要将消息推送给用户时，先将消息推送到当前消费者，再由当前消费者将消息推送到目标 uidList 所在的 ws 节点
///
/// 功能特点：
/// - 动态路由: 使用 Redis 存储设备节点映射关系，批量查询用户所在节点
/// - 高效分发: 从本质上避免广播风暴，减少网络开销
/// - 节点隔离: 每个节点只处理自己的消息，推送时只处理本节点连接的用户
use async_trait::async_trait;
use fbc_starter::{KafkaMessageHandler, Message};
use std::sync::Arc;
use tracing::{error, warn};

use crate::model::dto::RouterPushDto;
use crate::state::WsState;

/// 消息路由服务
pub struct MessageRouterService {
    ws_state: Arc<WsState>,
}

impl MessageRouterService {
    /// 创建新的消息路由服务
    pub fn new(ws_state: Arc<WsState>) -> Self {
        Self { ws_state }
    }

    /// 处理路由推送消息
    async fn handle_router_push(&self, dto: RouterPushDto) {
        // 1. 获取推送的成员
        if dto.uid_list.is_empty() {
            warn!("路由推送消息的用户列表为空，跳过处理");
            return;
        }
        // 2. 推送消息
        if let Err(e) = self
            .ws_state
            .services
            .push_service
            .send_push_msg(dto.ws_base_msg, dto.uid_list, dto.uid)
            .await
        {
            error!("推送消息失败: {}", e);
        }
    }
}

#[async_trait]
impl KafkaMessageHandler for MessageRouterService {
    /// 获取 Kafka 主题列表
    ///
    /// 对应 Java 中的 `MqConstant.PUSH_TOPIC`
    fn topics(&self) -> Vec<String> {
        vec!["websocket_push".to_string()]
    }

    fn group_id(&self) -> String {
        "websocket_push_group".to_string()
    }

    /// 处理消息
    async fn handle(&self, message: Message) {
        match serde_json::from_value::<RouterPushDto>(message.data) {
            Ok(dto) => {
                self.handle_router_push(dto).await;
            }
            Err(e) => {
                error!("解析路由推送消息失败: {}, topic={}", e, message.topic);
            }
        }
    }
}

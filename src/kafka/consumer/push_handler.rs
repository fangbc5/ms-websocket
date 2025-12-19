/// 节点消息推送处理器
///
/// 负责处理推送到本节点的消息，并将消息分发给对应的用户会话
/// 使用并发处理，保证高吞吐量
use async_trait::async_trait;
use fbc_starter::{KafkaMessageHandler, Message};
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::model::dto::NodePushDTO;
use crate::state::WsState;

pub struct PushHandler {
    ws_state: Arc<WsState>,
}

impl PushHandler {
    pub fn new(ws_state: Arc<WsState>) -> Self {
        Self { ws_state }
    }

    /// 处理节点推送消息
    async fn handle_push(&self, dto: NodePushDTO) {
        info!(
            "收到节点消息: hash_id={}, uid={}, 设备数={}",
            dto.hash_id,
            dto.uid,
            dto.device_user_map.len()
        );
        let device_count = dto.device_user_map.len();
        let hash_id = dto.hash_id;
        let operator_uid = dto.uid;

        // 并行推送到所有设备
        let results: Vec<_> = stream::iter(dto.device_user_map.into_iter())
            .map(|(device_id, user_id)| {
                let session_manager = self.ws_state.session_manager.clone();
                let msg = dto.ws_base_msg.clone();
                async move {
                    // 使用 SessionManager 发送消息到设备
                    let ws_msg = axum::extract::ws::Message::Text(
                        serde_json::to_string(&msg).unwrap_or_default().into(),
                    );
                    let sent = session_manager
                        .send_to_device(user_id, &device_id, ws_msg)
                        .await;
                    debug!(
                        "发送消息到设备 {} (用户 {}), 成功发送到 {} 个会话",
                        device_id, user_id, sent
                    );

                    Ok::<(), anyhow::Error>(())
                }
            })
            .buffer_unordered(10) // 并发度为 10
            .collect()
            .await;

        // 统计结果
        let failed = results.iter().filter(|r| r.is_err()).count();
        if failed > 0 {
            error!(
                "推送失败数量: {}/{} (hash_id={}, operator_uid={})",
                failed, device_count, hash_id, operator_uid
            );
        } else {
            debug!(
                "节点推送完成 (设备数: {}, hash_id={}, operator_uid={})",
                device_count, hash_id, operator_uid
            );
        }
    }
}

#[async_trait]
impl KafkaMessageHandler for PushHandler {
    fn topics(&self) -> Vec<String> {
        // 动态 topic：push_topic + node_id
        vec![format!(
            "websocket_push_{}",
            self.ws_state.session_manager.node_id()
        )]
    }

    fn group_id(&self) -> String {
        // 每个节点使用自己的 consumer group
        format!(
            "websocket_push_group_{}",
            self.ws_state.session_manager.node_id()
        )
    }

    async fn handle(&self, message: Message) {
        match serde_json::from_value::<NodePushDTO>(message.data) {
            Ok(dto) => {
                self.handle_push(dto).await;
            }
            Err(e) => {
                error!("解析节点推送消息失败: {}, topic={}", e, message.topic);
            }
        }
    }
}

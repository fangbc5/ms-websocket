/// 消息处理链
///
/// 责任链模式实现，按顺序遍历所有处理器，找到第一个支持的消息处理器并执行
use crate::model::ws_base_resp::WsBaseReq;
use crate::types::{ClientId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
use std::sync::Arc;
use tracing::{error, warn};

/// 消息处理链
pub struct MessageHandlerChain {
    pub(crate) processors: Vec<Arc<dyn MessageProcessor>>,
}

impl MessageHandlerChain {
    /// 创建新的消息处理链
    pub fn new(processors: Vec<Arc<dyn MessageProcessor>>) -> Self {
        Self { processors }
    }

    /// 处理消息
    ///
    /// # 参数
    /// - `session`: 会话对象（用于发送响应）
    /// - `session_id`: 会话 ID
    /// - `uid`: 用户 ID
    /// - `client_id`: 客户端 ID
    /// - `payload`: 消息负载（JSON 字符串）
    pub async fn handle_message(
        &self,
        session: &Arc<Session>,
        session_id: &SessionId,
        uid: UserId,
        client_id: &ClientId,
        payload: &str,
    ) {
        // 解析消息
        let req: WsBaseReq = match serde_json::from_str(payload) {
            Ok(req) => req,
            Err(e) => {
                warn!("解析 WebSocket 消息失败: {}, payload: {}", e, payload);
                return;
            }
        };

        // 遍历所有处理器，找到第一个支持的
        for processor in &self.processors {
            // 检查是否支持（捕获 panic）
            let supports = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                processor.supports(&req)
            })) {
                Ok(supports) => supports,
                Err(_) => {
                    error!(
                        "处理器 {} 的 supports 方法发生 panic",
                        std::any::type_name_of_val(processor)
                    );
                    continue;
                }
            };

            if supports {
                // 找到支持的处理器，执行处理
                processor
                    .process(session, session_id, uid, client_id, req.clone())
                    .await;
                return; // 找到处理器后立即返回
            }
        }

        // 没有找到支持的处理器
        warn!(
            "未找到支持消息类型的处理器: type={}, payload={}",
            req.r#type, payload
        );
    }
}

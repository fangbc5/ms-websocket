/// Kafka 模块
pub mod consumer;

use crate::{state::WsState, websocket::MessageRouterService};
use fbc_starter::KafkaMessageHandler;
use std::sync::Arc;

/// 初始化所有 Kafka 消息处理器
pub fn init_handlers(ws_state: Arc<WsState>) -> Vec<Arc<dyn KafkaMessageHandler>> {
    let mut handlers: Vec<Arc<dyn KafkaMessageHandler>> = Vec::new();
    // 消息路由服务（对应 MessageRouterService）
    handlers.push(Arc::new(MessageRouterService::new(ws_state.clone())));

    // 用户登录消息处理器
    // handlers.push(Arc::new(consumer::MsgLoginHandler::new(session_manager.clone())));

    // 扫码成功消息处理器
    // handlers.push(Arc::new(consumer::ScanSuccessHandler::new(session_manager.clone())));

    // 节点推送消息处理器
    handlers.push(Arc::new(consumer::PushHandler::new(ws_state.clone())));

    handlers
}

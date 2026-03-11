/// 路由配置模块
pub mod test_push;

use axum::Router;
use std::sync::Arc;

use crate::service::Services;
use crate::websocket::processor::meet::{
    MediaControlProcessor, QualityMonitorProcessor, RoomAdminProcessor, VideoCallProcessor,
    VideoProcessor,
};
use crate::websocket::processor::{
    AckProcessor, DefaultMessageProcessor, HeartbeatProcessor, ReadProcessor,
};
use crate::websocket::MessageHandlerChain;
use crate::state::WsState;

/// 创建应用路由
///
/// # 参数
/// - `ws_state`: WebSocket 状态（包含 AppState、SessionManager、Services、HandlerChain）
pub fn create_routes(
    ws_state: Arc<WsState>,
) -> Router {
    Router::new()
        .route("/ws", axum::routing::get(crate::websocket::ws_route))
        // 测试接口：推送消息到指定用户
        .route("/api/test/push", axum::routing::post(test_push::test_push_handler))
        // 测试接口：查询在线用户列表
        .route("/api/test/online", axum::routing::get(test_push::online_users_handler))
        .with_state(ws_state)
}

/// 创建消息处理链
///
/// # 参数
/// - `app_state`: fbc-starter 的 AppState
/// - `services`: 业务服务容器
///
/// # 返回
/// 消息处理链，按优先级排序
pub fn create_handler_chain(
    app_state: Arc<fbc_starter::AppState>,
    services: &Arc<Services>,
) -> Arc<MessageHandlerChain> {
    let processors: Vec<Arc<dyn crate::websocket::MessageProcessor>> = vec![
        // Order 1: 心跳处理器（最高优先级）
        Arc::new(HeartbeatProcessor::new()),
        // Order 10: 视频信令处理器
        Arc::new(VideoProcessor::new(
            services.video_chat_service.clone(),
            services.room_timeout_service.clone(),
        )),
        // Order 11: 视频呼叫处理器
        Arc::new(VideoCallProcessor::new(
            services.video_chat_service.clone(),
            services.push_service.clone(),
            services.room_timeout_service.clone(),
        )),
        // Order 12: 媒体控制处理器
        Arc::new(MediaControlProcessor::new(
            services.video_chat_service.clone(),
        )),
        // Order 13: 消息确认处理器
        Arc::new(AckProcessor::new(app_state.clone())),
        // Order 15: 已读消息处理器
        Arc::new(ReadProcessor::new(app_state.clone())),
        // Order 12: 质量监控处理器（与媒体控制同级）
        Arc::new(QualityMonitorProcessor::new(
            services.video_chat_service.clone(),
            services.push_service.clone(),
        )),
        // Order: 房间管理处理器
        Arc::new(RoomAdminProcessor::new(
            services.video_chat_service.clone(),
            services.push_service.clone(),
            services.room_timeout_service.clone(),
        )),
        // Order 100: 默认处理器（最低优先级，兜底）
        Arc::new(DefaultMessageProcessor::new()),
    ];

    Arc::new(MessageHandlerChain::new(processors))
}

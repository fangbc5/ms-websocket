// WS 服务器主入口
use ms_websocket::*;

use fbc_starter::{AppResult, Server};
use std::sync::Arc;

use crate::state::WsState;

#[tokio::main]
async fn main() -> AppResult<()> {
    Server::run(|builder| {
        // 获取配置和状态
        let app_state = builder.app_state().clone();

        // 创建会话管理器
        let mut session_manager = websocket::SessionManager::new();
        session_manager.set_app_state(app_state.clone());
        let session_manager = Arc::new(session_manager);

        // 初始化所有服务
        let services = Arc::new(
            service::Services::new(app_state.clone(), session_manager.clone())
                .expect("服务初始化失败"),
        );

        // 创建消息处理链
        let handler_chain = routes::create_handler_chain(app_state.clone(), &services);

        // 创建应用数据
        let ws_state = Arc::new(WsState::new(
            app_state,
            session_manager,
            services,
            handler_chain,
        ));

        // 创建路由
        let routes = routes::create_routes(ws_state.clone());

        // 初始化并注册 Kafka handlers
        let kafka_handlers = kafka::init_handlers(ws_state.clone());

        // 设置路由和 Kafka handlers
        builder
            .with_kafka_handlers(kafka_handlers)
            .http_router(routes)
    })
    .await
}

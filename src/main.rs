// WS 服务器主入口
mod cache;
mod enums;
pub mod grpc;
mod kafka;
mod model;
mod routes;
mod service;
mod state;
mod types;
pub mod websocket;

use fbc_starter::Server;
use std::sync::Arc;

use crate::state::WsState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Server::run(|builder| {
        // 获取配置和状态
        let app_state = builder.app_state().clone();

        // 创建会话管理器
        let session_manager = Arc::new(websocket::SessionManager::new());

        // 初始化所有服务
        let services = Arc::new(
            service::Services::new(app_state.clone(), session_manager.clone())
                .expect("服务初始化失败"),
        );

        // 创建消息处理链
        let handler_chain = routes::create_handler_chain(app_state.clone());

        // 创建应用数据
        let ws_state = Arc::new(WsState::new(
            app_state,
            session_manager,
            services,
            handler_chain,
        ));

        // 创建路由
        let routes = routes::create_routes(ws_state.clone());

        let _kafka_handlers = kafka::init_handlers(ws_state.clone());

        // 设置路由
        builder.http_router(routes)
    })
    .await
}

use std::sync::Arc;

use fbc_starter::AppState;

use crate::config::WsConfig;
use crate::service::Services;
use crate::websocket::{MessageHandlerChain, SessionManager};

/// WebSocket 应用状态
pub struct WsState {
    pub app_state: Arc<AppState>,
    pub config: Arc<WsConfig>,
    pub session_manager: Arc<SessionManager>,
    pub services: Arc<Services>,
    pub handler_chain: Arc<MessageHandlerChain>,
}

impl WsState {
    pub fn new(
        app_state: Arc<AppState>,
        config: Arc<WsConfig>,
        session_manager: Arc<SessionManager>,
        services: Arc<Services>,
        handler_chain: Arc<MessageHandlerChain>,
    ) -> Self {
        Self {
            app_state,
            config,
            session_manager,
            services,
            handler_chain,
        }
    }
}
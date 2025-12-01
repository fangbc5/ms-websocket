use std::sync::Arc;

use fbc_starter::AppState;

use crate::{service::Services, websocket::{MessageHandlerChain, SessionManager}};


pub struct WsState {
    pub app_state: Arc<AppState>,
    pub session_manager: Arc<SessionManager>,
    pub services: Arc<Services>,
    pub handler_chain: Arc<MessageHandlerChain>,
}

impl WsState {
    pub fn new(app_state: Arc<AppState>, session_manager: Arc<SessionManager>, services: Arc<Services>, handler_chain: Arc<MessageHandlerChain>) -> Self {
        Self {
            app_state,
            session_manager,
            services,
            handler_chain,
        }
    }
}
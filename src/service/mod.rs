/// 服务模块
///
/// 包含业务逻辑服务
pub mod push_service;
pub mod room_metadata_service;
pub mod room_timeout_service;
pub mod session_recovery_service;
pub mod video_chat_service;

pub use push_service::PushService;
pub use room_metadata_service::RoomMetadataService;
pub use room_timeout_service::RoomTimeoutService;
pub use session_recovery_service::SessionRecoveryService;
pub use video_chat_service::VideoChatService;

use crate::websocket::SessionManager;
use fbc_starter::AppState;
use std::sync::Arc;

/// 所有服务的容器
pub struct Services {
    pub push_service: Arc<PushService>,
    pub room_metadata_service: Arc<RoomMetadataService>,
    pub video_chat_service: Arc<VideoChatService>,
    pub room_timeout_service: Arc<RoomTimeoutService>,
    pub session_recovery_service: Arc<SessionRecoveryService>,
}

impl Services {
    /// 初始化所有服务
    ///
    /// 按照依赖关系顺序初始化：
    /// 1. RoomMetadataService (基础服务)
    /// 2. PushService (依赖 SessionManager 和 AppState)
    /// 3. VideoChatService (依赖 PushService 和 RoomMetadataService)
    /// 4. RoomTimeoutService (依赖 VideoChatService, PushService, RoomMetadataService)
    /// 5. SessionRecoveryService (依赖 VideoChatService)
    pub fn new(
        app_state: Arc<AppState>,
        session_manager: Arc<SessionManager>,
    ) -> anyhow::Result<Self> {
        // 1. 初始化基础服务
        let room_metadata_service = Arc::new(RoomMetadataService::new(app_state.clone()));

        // 2. 获取 node_id（从 SessionManager 或环境变量）
        let node_id = session_manager.node_id().to_string();

        // 3. 初始化 PushService
        let push_service = Arc::new(PushService::new(
            session_manager.clone(),
            app_state.clone(),
            node_id,
        ));

        // 4. 初始化 VideoChatService
        let video_chat_service: Arc<VideoChatService> = {
            Arc::new(VideoChatService::new(
                app_state.clone(),
                push_service.clone(),
                room_metadata_service.clone(),
            ))
        };

        // 5. 初始化 RoomTimeoutService
        let room_timeout_service = Arc::new(RoomTimeoutService::new(
            video_chat_service.clone(),
            push_service.clone(),
            room_metadata_service.clone(),
            app_state.clone(),
        ));

        // 6. 初始化 SessionRecoveryService
        let session_recovery_service =
            Arc::new(SessionRecoveryService::new(video_chat_service.clone()));

        Ok(Self {
            push_service,
            room_metadata_service,
            video_chat_service,
            room_timeout_service,
            session_recovery_service,
        })
    }
}

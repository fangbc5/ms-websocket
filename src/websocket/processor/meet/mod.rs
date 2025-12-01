/// 视频会议相关处理器模块
/// 
/// 包含所有与视频通话、会议相关的消息处理器

pub mod media_control;
pub mod quality_monitor;
pub mod room_admin;
pub mod video;
pub mod video_call;

pub use media_control::MediaControlProcessor;
pub use quality_monitor::QualityMonitorProcessor;
pub use room_admin::RoomAdminProcessor;
pub use video::VideoProcessor;
pub use video_call::VideoCallProcessor;


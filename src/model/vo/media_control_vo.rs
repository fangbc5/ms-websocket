use crate::types::RoomId;

/// 媒体控制 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MediaControlVO {
    /// 房间 ID
    pub room_id: RoomId,
    /// 音频是否静音
    pub audio_muted: bool,
    /// 视频是否静音
    pub video_muted: bool,
}

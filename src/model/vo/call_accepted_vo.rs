use crate::types::{RoomId, UserId};

/// 接受呼叫 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallAcceptedVO {
    /// 接受呼叫的用户 ID
    pub accepted_by: UserId,
    /// 房间 ID
    pub room_id: RoomId,
    /// LiveKit Access Token（用于连接 LiveKit 媒体服务器）
    pub token: String,
    /// LiveKit WebSocket URL
    pub livekit_url: String,
}

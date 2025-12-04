use crate::types::{RoomId, UserId};

/// 屏幕共享状态 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScreenSharingVO {
    /// 房间 ID
    pub room_id: RoomId,
    /// 共享者用户 ID
    pub user_id: UserId,
    /// 是否正在共享
    pub sharing: bool,
}

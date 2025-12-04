use crate::types::{RoomId, UserId};

/// 用户被踢出通知 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserKickedVO {
    /// 房间 ID
    pub room_id: RoomId,
    /// 被踢用户 ID
    pub kicked_uid: UserId,
    /// 操作者 ID
    pub operator_id: UserId,
    /// 踢出原因
    pub reason: String,
}

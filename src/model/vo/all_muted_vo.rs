use crate::types::{RoomId, UserId};

/// 全体静音状态 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AllMutedVO {
    /// 房间 ID
    pub room_id: RoomId,
    /// 是否静音
    pub muted: bool,
    /// 操作者 ID
    pub operator_id: UserId,
}

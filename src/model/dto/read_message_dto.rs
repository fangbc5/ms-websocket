/// 已读消息 DTO
use crate::types::{RoomId, UserId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReadMessageDto {
    /// 用户 ID
    pub uid: Option<UserId>,
    /// 房间 ID
    pub room_id: RoomId,
    /// 消息 ID 列表
    pub msg_ids: Vec<u64>,
}

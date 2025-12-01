/// 已读消息 DTO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReadMessageDto {
    /// 用户 ID
    pub uid: Option<u64>,
    /// 房间 ID
    pub room_id: u64,
    /// 消息 ID 列表
    pub msg_ids: Vec<u64>,
}


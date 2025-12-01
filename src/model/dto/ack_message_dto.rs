/// 消息确认 DTO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AckMessageDto {
    /// 用户 ID
    pub uid: Option<u64>,
    /// 消息 ID
    pub msg_id: u64,
}


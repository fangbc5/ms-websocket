/// 用户被踢出通知 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserKickedVO {
    /// 房间 ID
    pub room_id: u64,
    /// 被踢用户 ID
    pub kicked_uid: u64,
    /// 操作者 ID
    pub operator_id: u64,
    /// 踢出原因
    pub reason: String,
}

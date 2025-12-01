/// 接受呼叫 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallAcceptedVO {
    /// 接受呼叫的用户 ID
    pub accepted_by: String,
    /// 房间 ID
    pub room_id: String,
}

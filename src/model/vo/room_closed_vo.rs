/// 房间关闭 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoomClosedVO {
    /// 房间 ID
    pub room_id: String,
    /// 关闭原因（如：超时关闭、发起通话的人关闭）
    pub reason: String,
}

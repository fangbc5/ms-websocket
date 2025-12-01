/// 开始信令 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StartSignalingVO {
    /// 房间 ID
    pub room_id: u64,
}

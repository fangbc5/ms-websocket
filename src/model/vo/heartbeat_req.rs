/// 心跳请求
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeartbeatReq {
    /// 房间 ID
    pub room_id: u64,
}

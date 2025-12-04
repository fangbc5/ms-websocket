use crate::types::RoomId;

/// 心跳请求
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeartbeatReq {
    /// 房间 ID
    pub room_id: RoomId,
}

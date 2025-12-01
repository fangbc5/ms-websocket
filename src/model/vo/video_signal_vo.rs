/// 视频信令 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VideoSignalVO {
    /// 信令发送者 ID
    pub sender_id: u64,
    /// 房间 ID
    pub room_id: u64,
    /// WebRTC 信令内容
    pub signal: String,
    /// 信令类型
    pub signal_type: String,
    /// 发送时间戳（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

impl VideoSignalVO {
    /// 创建新的视频信令（自动设置当前时间戳）
    pub fn new(sender_id: u64, room_id: u64, signal_type: String, signal: String) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Self {
            sender_id,
            room_id,
            signal,
            signal_type,
            timestamp: Some(timestamp),
        }
    }
}

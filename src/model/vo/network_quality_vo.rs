/// 网络质量报告 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkQualityVO {
    /// 房间 ID
    pub room_id: u64,
    /// 用户 ID
    pub user_id: u64,
    /// 网络质量评分 (0.0-1.0)
    pub quality: f64,
    /// 报告时间戳（毫秒）
    pub timestamp: i64,
}

impl NetworkQualityVO {
    /// 创建新的网络质量报告（自动设置当前时间戳）
    pub fn new(room_id: u64, user_id: u64, quality: f64) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Self {
            room_id,
            user_id,
            quality,
            timestamp,
        }
    }
}

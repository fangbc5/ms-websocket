/// 用户加入房间 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserJoinRoomVO {
    /// 用户 ID
    pub uid: String,
    /// 房间 ID
    pub room_id: String,
    /// 用户名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 用户头像
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// 加入时间戳（毫秒）
    pub timestamp: i64,
}

impl UserJoinRoomVO {
    /// 创建新的用户加入房间消息（自动设置当前时间戳）
    pub fn new(uid: u64, room_id: u64) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Self {
            uid: uid.to_string(),
            room_id: room_id.to_string(),
            name: None,
            avatar: None,
            timestamp,
        }
    }

    /// 设置用户名称
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// 设置用户头像
    pub fn with_avatar(mut self, avatar: String) -> Self {
        self.avatar = Some(avatar);
        self
    }
}

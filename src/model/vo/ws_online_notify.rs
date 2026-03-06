/// 在线状态通知 VO
///
/// 用于通知好友/群成员用户的上下线状态变更
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WSOnlineNotify {
    /// 触发事件的用户 ID
    pub uid: u64,
    /// 触发事件的客户端 ID（设备指纹）
    pub client_id: String,
    /// 群聊房间 ID（仅群组通知时有值）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<u64>,
    /// 最后操作时间（毫秒时间戳）
    pub last_opt_time: i64,
    /// 在线人数（好友通知=该用户在线好友数，群组通知=群在线成员数）
    pub online_num: i64,
    /// 通知类型：1=群组通知，2=好友通知
    pub notify_type: i32,
}

/// 通知类型常量
pub const NOTIFY_TYPE_GROUP: i32 = 1;
pub const NOTIFY_TYPE_FRIEND: i32 = 2;

impl WSOnlineNotify {
    /// 创建好友上下线通知（无 room_id）
    pub fn friend_notify(uid: u64, client_id: String, last_opt_time: i64, online_num: i64) -> Self {
        Self {
            uid,
            client_id,
            room_id: None,
            last_opt_time,
            online_num,
            notify_type: NOTIFY_TYPE_FRIEND,
        }
    }

    /// 创建群组上下线通知（带 room_id）
    pub fn group_notify(
        room_id: u64,
        uid: u64,
        client_id: String,
        last_opt_time: i64,
        online_num: i64,
    ) -> Self {
        Self {
            uid,
            client_id,
            room_id: Some(room_id),
            last_opt_time,
            online_num,
            notify_type: NOTIFY_TYPE_GROUP,
        }
    }
}

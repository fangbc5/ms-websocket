/// 缓存过期时间常量，避免魔法数字
use std::time::Duration;

/// 视频/用户房间列表缓存：15 分钟
pub const EXPIRE_VIDEO_USER_ROOMS: Duration = Duration::from_secs(15 * 60);

/// 房间管理员元数据：60 分钟
pub const EXPIRE_ROOM_ADMIN_META: Duration = Duration::from_secs(60 * 60);

/// 关闭房间标记：5 小时
pub const EXPIRE_CLOSE_ROOM: Duration = Duration::from_secs(5 * 60 * 60);

/// 在线状态缓存：30 天（需要跟 token 单次在线时长一致）
pub const EXPIRE_PRESENCE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// 好友关系状态缓存：7 天
pub const EXPIRE_FRIEND_STATUS: Duration = Duration::from_secs(7 * 24 * 60 * 60);

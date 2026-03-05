/// 缓存过期时间常量，避免魔法数字
use std::time::Duration;

/// 视频/用户房间列表缓存：15 分钟
pub const EXPIRE_VIDEO_USER_ROOMS: Duration = Duration::from_secs(15 * 60);

/// 房间管理员元数据：60 分钟
pub const EXPIRE_ROOM_ADMIN_META: Duration = Duration::from_secs(60 * 60);

/// 关闭房间标记：5 小时
pub const EXPIRE_CLOSE_ROOM: Duration = Duration::from_secs(5 * 60 * 60);

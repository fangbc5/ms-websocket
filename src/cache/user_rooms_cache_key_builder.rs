/// 用户房间列表缓存键
use fbc_starter::cache::{get_cache_prefix, CacheKey, CacheKeyBuilder, VIDEO_CALL};

use crate::cache::constants::EXPIRE_VIDEO_USER_ROOMS;
use crate::types::UserId;

/// 用户房间缓存键构建器
pub struct UserRoomsCacheKeyBuilder;

impl UserRoomsCacheKeyBuilder {
    /// 构建缓存键
    pub fn build(key: UserId) -> CacheKey {
        UserRoomsCacheKeyBuilder.key(&[&key])
    }
}

impl CacheKeyBuilder for UserRoomsCacheKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_table(&self) -> &str {
        "user_rooms"
    }

    fn get_modular(&self) -> Option<&str> {
        Some(VIDEO_CALL)
    }

    fn get_tenant(&self) -> Option<&str> {
        None
    }

    fn get_expire(&self) -> Option<std::time::Duration> {
        Some(EXPIRE_VIDEO_USER_ROOMS)
    }
}

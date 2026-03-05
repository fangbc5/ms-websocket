/// 视频会议房间列表缓存键
use fbc_starter::cache::{get_cache_prefix, CacheKey, CacheKeyBuilder, VIDEO_CALL};

use crate::cache::constants::EXPIRE_VIDEO_USER_ROOMS;

/// 视频房间缓存键构建器
pub struct VideoRoomsCacheKeyBuilder;

impl VideoRoomsCacheKeyBuilder {
    /// 构建缓存键
    pub fn build(key: u64) -> CacheKey {
        VideoRoomsCacheKeyBuilder.key(&[&key])
    }
}

impl CacheKeyBuilder for VideoRoomsCacheKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_table(&self) -> &str {
        "rooms"
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

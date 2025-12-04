/// 视频会议参数 KEY
use fbc_starter::cache::{CacheKey, CacheKeyBuilder, VIDEO_CALL, get_cache_prefix};
use std::time::Duration;

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

    fn get_expire(&self) -> Option<Duration> {
        Some(Duration::from_secs(15 * 60)) // 15 分钟
    }
}

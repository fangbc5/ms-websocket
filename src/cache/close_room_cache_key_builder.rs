use fbc_starter::ID_FIELD;
/// 关闭房间参数 KEY
use fbc_starter::cache::{CHAT, CacheKey, CacheKeyBuilder, ValueType, chat, get_cache_prefix};
use std::time::Duration;

use crate::types::RoomId;

/// 关闭房间缓存键构建器
pub struct CloseRoomCacheKeyBuilder;

impl CloseRoomCacheKeyBuilder {
    /// 构建缓存键
    pub fn builder(room_id: RoomId) -> CacheKey {
        CloseRoomCacheKeyBuilder.key(&[&room_id])
    }
}

impl CacheKeyBuilder for CloseRoomCacheKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_tenant(&self) -> Option<&str> {
        None
    }

    fn get_table(&self) -> &str {
        chat::CLOSE_ROOM
    }

    fn get_modular(&self) -> Option<&str> {
        Some(CHAT)
    }

    fn get_field(&self) -> Option<&str> {
        Some(ID_FIELD)
    }

    fn get_value_type(&self) -> ValueType {
        ValueType::String
    }

    fn get_expire(&self) -> Option<Duration> {
        Some(Duration::from_secs(5 * 60 * 60)) // 5 小时
    }
}

/// 关闭房间标记缓存键
use fbc_starter::cache::{chat, get_cache_prefix, CHAT, CacheKey, CacheKeyBuilder, ValueType};
use fbc_starter::ID_FIELD;

use crate::cache::constants::EXPIRE_CLOSE_ROOM;
use crate::types::RoomId;

/// 关闭房间缓存键构建器
pub struct CloseRoomCacheKeyBuilder;

impl CloseRoomCacheKeyBuilder {
    /// 构建缓存键
    pub fn build(room_id: RoomId) -> CacheKey {
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

    fn get_expire(&self) -> Option<std::time::Duration> {
        Some(EXPIRE_CLOSE_ROOM)
    }
}

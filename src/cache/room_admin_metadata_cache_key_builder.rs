/// 房间管理员元数据缓存键
use fbc_starter::cache::{
    get_cache_prefix, video_call, CacheKey, CacheKeyBuilder, ValueType, VIDEO_CALL,
};

use crate::cache::constants::EXPIRE_ROOM_ADMIN_META;
use crate::types::RoomId;

/// 房间管理员元数据缓存键构建器
pub struct RoomAdminMetadataCacheKeyBuilder;

impl RoomAdminMetadataCacheKeyBuilder {
    /// 构建缓存键
    pub fn build(room_id: RoomId) -> CacheKey {
        RoomAdminMetadataCacheKeyBuilder.key(&[&room_id])
    }
}

impl CacheKeyBuilder for RoomAdminMetadataCacheKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_tenant(&self) -> Option<&str> {
        Some("") // StrPool.EMPTY
    }

    fn get_modular(&self) -> Option<&str> {
        Some(VIDEO_CALL)
    }

    fn get_table(&self) -> &str {
        video_call::META_DATA_ADMIN
    }

    fn get_field(&self) -> Option<&str> {
        Some("id")
    }

    fn get_value_type(&self) -> ValueType {
        ValueType::Obj
    }

    fn get_expire(&self) -> Option<std::time::Duration> {
        Some(EXPIRE_ROOM_ADMIN_META)
    }
}


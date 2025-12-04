/// 房间管理员的元数据
use fbc_starter::cache::{
    get_cache_prefix, video_call, CacheKey, CacheKeyBuilder, ValueType, VIDEO_CALL,
};
use std::time::Duration;

use crate::types::RoomId;

/// 房间管理员元数据缓存键构建器
pub struct RoomAdminMetadataCacheKeyBuilder;

impl RoomAdminMetadataCacheKeyBuilder {
    /// 构建缓存键
    pub fn builder(room_id: RoomId) -> CacheKey {
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

    fn get_expire(&self) -> Option<Duration> {
        Some(Duration::from_secs(60 * 60)) // 60 分钟
    }
}


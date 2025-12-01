/// 房间元数据服务参数 KEY
use fbc_starter::cache::{chat, get_cache_prefix, CacheHashKey, CacheKeyBuilder, CHAT, ValueType};
use fbc_starter::ID_FIELD;

/// 房间元数据缓存键构建器
pub struct RoomMetadataCacheKeyBuilder;

impl RoomMetadataCacheKeyBuilder {
    /// 构建缓存 Hash 键
    pub fn builder(room_id: i64, item_key: &str) -> CacheHashKey {
        RoomMetadataCacheKeyBuilder.hash_field_key(&item_key, &[&room_id])
    }
}

impl CacheKeyBuilder for RoomMetadataCacheKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_tenant(&self) -> Option<&str> {
        None
    }

    fn get_table(&self) -> &str {
        chat::ROOM_META
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
}


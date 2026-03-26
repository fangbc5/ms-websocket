/// 路由模块缓存键构建器
///
/// 设备-节点映射存于 Redis Hash，key 由本模块构建，field 由调用方指定（如 uid:client_id）。
use fbc_starter::cache::{CacheHashKey, CacheKey, CacheKeyBuilder, ValueType};

use crate::types::ClientId;

/// 路由相关 key 的前缀（项目/环境）
const ROUTER_KEY_PREFIX: &str = "xilulu";

/// 路由缓存键构建器
pub struct RouterCacheKeyBuilder;

impl RouterCacheKeyBuilder {
    /// 构建设备-节点映射的 Redis Hash 键（调用方使用返回的 key，field 自拟如 `uid:client_id`）
    pub fn build_device_node_map(client_id: ClientId) -> CacheHashKey {
        let field_name = "device-node-mapping";
        DeviceNodeMapping.hash_field_key(&client_id, &[&field_name as &dyn ToString])
    }

    /// 构建节点设备集合的缓存键
    #[allow(dead_code)]
    pub fn build_node_devices(node_id: &str) -> CacheKey {
        NodeDevices.key(&[&node_id])
    }
}

/// 设备-节点映射表（Hash，永不过期）
pub struct DeviceNodeMapping;

impl CacheKeyBuilder for DeviceNodeMapping {
    fn get_prefix(&self) -> Option<&str> {
        Some(ROUTER_KEY_PREFIX)
    }

    fn get_tenant(&self) -> Option<&str> {
        None
    }

    fn get_table(&self) -> &str {
        "router"
    }

    fn get_value_type(&self) -> ValueType {
        ValueType::String
    }
}

/// 节点设备集合（永不过期）
pub struct NodeDevices;

impl CacheKeyBuilder for NodeDevices {
    fn get_prefix(&self) -> Option<&str> {
        Some(ROUTER_KEY_PREFIX)
    }

    fn get_tenant(&self) -> Option<&str> {
        None
    }

    fn get_modular(&self) -> Option<&str> {
        Some("router")
    }

    fn get_table(&self) -> &str {
        "node-devices"
    }

    fn get_value_type(&self) -> ValueType {
        ValueType::String
    }
}

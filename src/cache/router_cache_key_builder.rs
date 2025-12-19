/// 路由模块缓存键构建器
use fbc_starter::cache::{CacheHashKey, CacheKey, CacheKeyBuilder, ValueType};
use std::time::Duration;

use crate::types::ClientId;

/// 路由缓存键构建器
pub struct RouterCacheKeyBuilder;

impl RouterCacheKeyBuilder {
    /// 构建设备-节点映射的缓存 Hash 键
    ///
    /// # 参数
    /// - `client_id`: 客户端 ID（作为 hash field）
    pub fn build_device_node_map(client_id: ClientId) -> CacheHashKey {
        let field_name = "device-node-mapping";
        DeviceNodeMapping.hash_field_key(&client_id, &[&field_name as &dyn ToString])
    }

    /// 构建节点设备集合的缓存键
    ///
    /// # 参数
    /// - `node_id`: 节点 ID
    #[allow(dead_code)]
    pub fn build_node_devices(node_id: &str) -> CacheKey {
        NodeDevices.key(&[&node_id])
    }
}

/// 设备-节点映射表
pub struct DeviceNodeMapping;

impl CacheKeyBuilder for DeviceNodeMapping {
    fn get_prefix(&self) -> Option<&str> {
        Some("luohuo")
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

    fn get_expire(&self) -> Option<Duration> {
        None // -1 表示永不过期
    }
}

/// 节点设备集合
pub struct NodeDevices;

impl CacheKeyBuilder for NodeDevices {
    fn get_prefix(&self) -> Option<&str> {
        Some("luohuo")
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

    fn get_expire(&self) -> Option<Duration> {
        None // -1 表示永不过期
    }
}

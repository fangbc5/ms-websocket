/// 本地路由缓存
///
/// 用于缓存 Redis 中的设备-节点映射，减少 Redis 查询次数
///
/// 性能提升：
/// - Redis 查询延迟：1-2ms
/// - 本地缓存查询延迟：< 0.01ms
/// - 预期减少 90% 的 Redis 查询
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    /// 节点 ID
    node_id: String,
    /// 过期时间
    expires_at: Instant,
}

/// 本地路由缓存
#[derive(Clone)]
pub struct LocalRouterCache {
    /// 缓存映射：uid:client_id -> (node_id, expires_at)
    cache: Arc<DashMap<String, CacheEntry>>,
    /// 缓存过期时间（默认 30 秒）
    ttl: Duration,
}

impl LocalRouterCache {
    /// 创建新的本地路由缓存
    pub fn new(ttl: Duration) -> Self {
        let cache = Self {
            cache: Arc::new(DashMap::new()),
            ttl,
        };

        // 启动后台清理任务
        cache.start_cleanup_task();

        cache
    }

    /// 获取缓存的节点 ID
    ///
    /// # 参数
    /// - `uid`: 用户 ID
    /// - `client_id`: 客户端 ID
    ///
    /// # 返回
    /// 如果缓存命中且未过期，返回 Some(node_id)，否则返回 None
    pub fn get(&self, uid: u64, client_id: &str) -> Option<String> {
        let key = format!("{}:{}", uid, client_id);

        if let Some(entry) = self.cache.get(&key) {
            // 检查是否过期
            if entry.expires_at > Instant::now() {
                return Some(entry.node_id.clone());
            } else {
                // 过期则删除
                drop(entry);
                self.cache.remove(&key);
            }
        }

        None
    }

    /// 设置缓存
    ///
    /// # 参数
    /// - `uid`: 用户 ID
    /// - `client_id`: 客户端 ID
    /// - `node_id`: 节点 ID
    pub fn set(&self, uid: u64, client_id: &str, node_id: String) {
        let key = format!("{}:{}", uid, client_id);
        let entry = CacheEntry {
            node_id,
            expires_at: Instant::now() + self.ttl,
        };
        self.cache.insert(key, entry);
    }

    /// 删除缓存
    ///
    /// # 参数
    /// - `uid`: 用户 ID
    /// - `client_id`: 客户端 ID
    pub fn remove(&self, uid: u64, client_id: &str) {
        let key = format!("{}:{}", uid, client_id);
        self.cache.remove(&key);
    }

    /// 清空所有缓存
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// 获取缓存大小
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// 判断缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// 启动后台清理任务
    fn start_cleanup_task(&self) {
        let cache = self.cache.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;

                // 清理过期条目
                let now = Instant::now();
                let mut expired_keys = Vec::new();

                for entry in cache.iter() {
                    if entry.value().expires_at <= now {
                        expired_keys.push(entry.key().clone());
                    }
                }

                for key in expired_keys {
                    cache.remove(&key);
                }
            }
        });
    }
}

impl Default for LocalRouterCache {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic() {
        let cache = LocalRouterCache::new(Duration::from_secs(1));

        // 测试设置和获取
        cache.set(123, "device1", "node1".to_string());
        assert_eq!(cache.get(123, "device1"), Some("node1".to_string()));

        // 测试不存在的键
        assert_eq!(cache.get(456, "device2"), None);

        // 测试删除
        cache.remove(123, "device1");
        assert_eq!(cache.get(123, "device1"), None);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = LocalRouterCache::new(Duration::from_millis(100));

        cache.set(123, "device1", "node1".to_string());
        assert_eq!(cache.get(123, "device1"), Some("node1".to_string()));

        // 等待过期
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(cache.get(123, "device1"), None);
    }
}

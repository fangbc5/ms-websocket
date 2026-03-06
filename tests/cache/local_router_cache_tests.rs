/// 本地路由缓存扩展测试
///
/// 测试 LocalRouterCache 的并发操作、TTL 过期、批量操作等
use ms_websocket::cache::LocalRouterCache;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ========================
// 基本功能测试
// ========================

#[tokio::test]
async fn test_cache_set_and_get() {
    let cache = LocalRouterCache::new(Duration::from_secs(10));

    cache.set(1001, "device_a", "node_1".to_string());
    assert_eq!(cache.get(1001, "device_a"), Some("node_1".to_string()));
}

#[tokio::test]
async fn test_cache_get_nonexistent_key() {
    let cache = LocalRouterCache::new(Duration::from_secs(10));

    assert_eq!(cache.get(9999, "nonexistent"), None);
}

#[tokio::test]
async fn test_cache_remove() {
    let cache = LocalRouterCache::new(Duration::from_secs(10));

    cache.set(1001, "device_a", "node_1".to_string());
    assert_eq!(cache.len(), 1);

    cache.remove(1001, "device_a");
    assert_eq!(cache.get(1001, "device_a"), None);
    assert_eq!(cache.len(), 0);
}

#[tokio::test]
async fn test_cache_remove_nonexistent_key() {
    let cache = LocalRouterCache::new(Duration::from_secs(10));

    // 删除不存在的键不应 panic
    cache.remove(9999, "nonexistent");
    assert_eq!(cache.len(), 0);
}

#[tokio::test]
async fn test_cache_clear() {
    let cache = LocalRouterCache::new(Duration::from_secs(10));

    cache.set(1001, "device_a", "node_1".to_string());
    cache.set(1002, "device_b", "node_2".to_string());
    cache.set(1003, "device_c", "node_3".to_string());
    assert_eq!(cache.len(), 3);

    cache.clear();
    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
}

#[tokio::test]
async fn test_cache_len_and_is_empty() {
    let cache = LocalRouterCache::new(Duration::from_secs(10));

    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());

    cache.set(1001, "device_a", "node_1".to_string());
    assert_eq!(cache.len(), 1);
    assert!(!cache.is_empty());

    cache.set(1002, "device_b", "node_2".to_string());
    assert_eq!(cache.len(), 2);

    cache.remove(1001, "device_a");
    assert_eq!(cache.len(), 1);
    assert!(!cache.is_empty());
}

// ========================
// 缓存覆盖测试
// ========================

#[tokio::test]
async fn test_cache_overwrite_same_key() {
    let cache = LocalRouterCache::new(Duration::from_secs(10));

    cache.set(1001, "device_a", "node_1".to_string());
    assert_eq!(cache.get(1001, "device_a"), Some("node_1".to_string()));

    // 覆盖同一个键
    cache.set(1001, "device_a", "node_2".to_string());
    assert_eq!(cache.get(1001, "device_a"), Some("node_2".to_string()));

    // 缓存大小不应增加
    assert_eq!(cache.len(), 1);
}

#[tokio::test]
async fn test_cache_same_uid_different_devices() {
    let cache = LocalRouterCache::new(Duration::from_secs(10));

    cache.set(1001, "device_a", "node_1".to_string());
    cache.set(1001, "device_b", "node_2".to_string());

    assert_eq!(cache.get(1001, "device_a"), Some("node_1".to_string()));
    assert_eq!(cache.get(1001, "device_b"), Some("node_2".to_string()));
    assert_eq!(cache.len(), 2);
}

#[tokio::test]
async fn test_cache_different_uids_same_device() {
    let cache = LocalRouterCache::new(Duration::from_secs(10));

    cache.set(1001, "device_a", "node_1".to_string());
    cache.set(1002, "device_a", "node_2".to_string());

    assert_eq!(cache.get(1001, "device_a"), Some("node_1".to_string()));
    assert_eq!(cache.get(1002, "device_a"), Some("node_2".to_string()));
    assert_eq!(cache.len(), 2);
}

// ========================
// TTL 过期测试
// ========================

#[tokio::test]
async fn test_cache_ttl_expiration() {
    let cache = LocalRouterCache::new(Duration::from_millis(100));

    cache.set(1001, "device_a", "node_1".to_string());
    assert_eq!(cache.get(1001, "device_a"), Some("node_1".to_string()));

    // 等待过期
    sleep(Duration::from_millis(150)).await;

    // 过期后 get 应返回 None
    assert_eq!(cache.get(1001, "device_a"), None);
}

#[tokio::test]
async fn test_cache_ttl_not_yet_expired() {
    let cache = LocalRouterCache::new(Duration::from_secs(5));

    cache.set(1001, "device_a", "node_1".to_string());

    // 等一小段时间（不到过期）
    sleep(Duration::from_millis(50)).await;

    // 应该仍然可以获取
    assert_eq!(cache.get(1001, "device_a"), Some("node_1".to_string()));
}

#[tokio::test]
async fn test_cache_overwrite_resets_ttl() {
    let cache = LocalRouterCache::new(Duration::from_millis(200));

    cache.set(1001, "device_a", "node_1".to_string());

    // 等待接近过期
    sleep(Duration::from_millis(150)).await;

    // 覆盖应重置 TTL
    cache.set(1001, "device_a", "node_2".to_string());

    // 再等 100ms（原始应该已过期，但覆盖后不应过期）
    sleep(Duration::from_millis(100)).await;

    assert_eq!(cache.get(1001, "device_a"), Some("node_2".to_string()));
}

#[tokio::test]
async fn test_cache_mixed_expired_and_valid() {
    let cache = LocalRouterCache::new(Duration::from_millis(100));

    cache.set(1001, "device_a", "node_1".to_string());

    // 等待过期
    sleep(Duration::from_millis(150)).await;

    // 添加新的（未过期的）
    cache.set(1002, "device_b", "node_2".to_string());

    // 旧的已过期
    assert_eq!(cache.get(1001, "device_a"), None);
    // 新的还在
    assert_eq!(cache.get(1002, "device_b"), Some("node_2".to_string()));
}

// ========================
// 并发测试
// ========================

#[tokio::test]
async fn test_cache_concurrent_writes() {
    let cache = Arc::new(LocalRouterCache::new(Duration::from_secs(10)));
    let mut handles = vec![];

    // 并发写入 1000 个不同的键
    for i in 0..1000u64 {
        let cache_clone = cache.clone();
        let handle = tokio::spawn(async move {
            cache_clone.set(i, &format!("device_{}", i), format!("node_{}", i % 10));
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(cache.len(), 1000);
}

#[tokio::test]
async fn test_cache_concurrent_reads_and_writes() {
    let cache = Arc::new(LocalRouterCache::new(Duration::from_secs(10)));

    // 预写入一些数据
    for i in 0..100u64 {
        cache.set(i, "device_0", format!("node_{}", i));
    }

    let mut handles = vec![];

    // 并发读写
    for i in 0..200u64 {
        let cache_clone = cache.clone();
        let handle = tokio::spawn(async move {
            if i < 100 {
                // 读操作
                let _ = cache_clone.get(i, "device_0");
            } else {
                // 写操作
                cache_clone.set(i, "device_0", format!("node_{}", i));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // 应该有 200 个条目（100 预写入 + 100 新写入）
    assert_eq!(cache.len(), 200);
}

#[tokio::test]
async fn test_cache_concurrent_write_same_key() {
    let cache = Arc::new(LocalRouterCache::new(Duration::from_secs(10)));
    let mut handles = vec![];

    // 并发写入同一个键
    for i in 0..100 {
        let cache_clone = cache.clone();
        let handle = tokio::spawn(async move {
            cache_clone.set(1001, "device_a", format!("node_{}", i));
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // 只应该有一个条目
    assert_eq!(cache.len(), 1);
    // 值应该是最后写入的某个值（不确定具体顺序）
    assert!(cache.get(1001, "device_a").is_some());
}

#[tokio::test]
async fn test_cache_concurrent_remove() {
    let cache = Arc::new(LocalRouterCache::new(Duration::from_secs(10)));

    // 预写入
    for i in 0..100u64 {
        cache.set(i, "device_0", format!("node_{}", i));
    }

    let mut handles = vec![];

    // 并发移除所有
    for i in 0..100u64 {
        let cache_clone = cache.clone();
        let handle = tokio::spawn(async move {
            cache_clone.remove(i, "device_0");
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(cache.len(), 0);
}

// ========================
// Default trait 测试
// ========================

#[tokio::test]
async fn test_cache_default_ttl() {
    let cache = LocalRouterCache::default();

    cache.set(1001, "device_a", "node_1".to_string());
    assert_eq!(cache.get(1001, "device_a"), Some("node_1".to_string()));

    // Default TTL 是 30 秒（不会在测试中等待过期）
    // 只验证创建和基本功能正常
    assert_eq!(cache.len(), 1);
}

// ========================
// 批量操作测试
// ========================

#[tokio::test]
async fn test_cache_batch_set_and_get() {
    let cache = LocalRouterCache::new(Duration::from_secs(10));

    // 批量写入
    for i in 0..500u64 {
        cache.set(i, &format!("dev_{}", i), format!("node_{}", i % 5));
    }

    assert_eq!(cache.len(), 500);

    // 批量读取
    for i in 0..500u64 {
        let expected_node = format!("node_{}", i % 5);
        assert_eq!(
            cache.get(i, &format!("dev_{}", i)),
            Some(expected_node),
            "缓存 uid={} 读取失败",
            i
        );
    }
}

#[tokio::test]
async fn test_cache_clear_after_batch_operations() {
    let cache = LocalRouterCache::new(Duration::from_secs(10));

    for i in 0..100u64 {
        cache.set(i, "device", format!("node_{}", i));
    }

    assert_eq!(cache.len(), 100);
    assert!(!cache.is_empty());

    cache.clear();

    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());

    // 确认所有键都不存在
    for i in 0..100u64 {
        assert_eq!(cache.get(i, "device"), None);
    }
}

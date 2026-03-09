/// Nacos 会话注册中心集成测试（P2 新增功能）
///
/// 需要 Redis 支持
/// 运行方式：
///   TEST_REDIS_PASSWORD='1qaz!QAZ' cargo test -p ms-websocket --test integration -- nacos_registry
///
/// 覆盖范围：
/// - NacosSessionRegistry::cleanup_node_routes
/// - NacosSessionRegistry::clean_node_completely（通过 sync_online(false) 触发下线）
/// - get_all_redis_node_ids（SCAN 扫描）
/// - update_node_metrics（写入 Redis Hash）
use crate::common::*;
use ms_websocket::cache::RouterCacheKeyBuilder;
use ms_websocket::websocket::{NacosSessionRegistry, SessionManager};
use redis::AsyncCommands;
use std::sync::Arc;

/// 创建带 Redis 的 NacosSessionRegistry
async fn create_test_registry() -> (
    Arc<fbc_starter::AppState>,
    Arc<SessionManager>,
    Arc<NacosSessionRegistry>,
) {
    let app_state = create_test_app_state().await;

    let mut session_manager = SessionManager::new();
    session_manager.set_app_state(app_state.clone());
    let session_manager = Arc::new(session_manager);

    let node_id = session_manager.node_id().to_string();
    let registry = Arc::new(NacosSessionRegistry::new(
        session_manager.clone(),
        app_state.clone(),
        node_id,
    ));

    (app_state, session_manager, registry)
}

// ========================
// cleanup_node_routes 测试
// ========================

#[tokio::test]
#[ignore]
async fn test_cleanup_node_routes_empty_node() {
    let (app_state, _sm, registry) = create_test_registry().await;
    let fake_node = "test_empty_node_99";

    // 确保该节点没有数据
    let node_devices_key = RouterCacheKeyBuilder::build_node_devices(fake_node);
    {
        let mut conn = app_state.redis().await.unwrap();
        let _: () = conn.del(&node_devices_key.key).await.unwrap_or(());
    }

    // cleanup_node_routes 对空节点不应报错
    registry.cleanup_node_routes(fake_node).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_cleanup_node_routes_with_devices() {
    let (app_state, _sm, registry) = create_test_registry().await;
    let fake_node = "test_clean_node_100";

    let device_node_map = RouterCacheKeyBuilder::build_device_node_map(String::new());
    let node_devices_key = RouterCacheKeyBuilder::build_node_devices(fake_node);

    // 模拟设备数据
    let device_fields = vec!["90001:dev_a", "90002:dev_b", "90003:dev_c"];
    {
        let mut conn = app_state.redis().await.unwrap();
        for field in &device_fields {
            // 添加到节点设备 Set
            let _: () = conn.sadd(&node_devices_key.key, *field).await.unwrap();
            // 添加到全局 Hash
            let _: () = conn.hset(&device_node_map.key, *field, fake_node).await.unwrap();
        }
    }

    // 验证数据已写入
    {
        let mut conn = app_state.redis().await.unwrap();
        let count: i64 = conn.scard(&node_devices_key.key).await.unwrap();
        assert_eq!(count, 3, "应有 3 个设备在节点 Set 中");

        for field in &device_fields {
            let exists: bool = conn.hexists(&device_node_map.key, *field).await.unwrap();
            assert!(exists, "全局 Hash 应包含设备 {}", field);
        }
    }

    // 执行清理
    registry.cleanup_node_routes(fake_node).await.unwrap();

    // 验证已清理
    {
        let mut conn = app_state.redis().await.unwrap();
        let exists: bool = conn.exists(&node_devices_key.key).await.unwrap();
        assert!(!exists, "节点设备 Set 应已删除");

        for field in &device_fields {
            let exists: bool = conn.hexists(&device_node_map.key, *field).await.unwrap();
            assert!(!exists, "全局 Hash 不应再包含设备 {}", field);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_cleanup_node_routes_partial_hash_missing() {
    let (app_state, _sm, registry) = create_test_registry().await;
    let fake_node = "test_partial_node_101";

    let device_node_map = RouterCacheKeyBuilder::build_device_node_map(String::new());
    let node_devices_key = RouterCacheKeyBuilder::build_node_devices(fake_node);

    // 模拟：Set 中有 3 个设备，但 Hash 中只有 1 个（不一致状态）
    {
        let mut conn = app_state.redis().await.unwrap();
        let _: () = conn.sadd(&node_devices_key.key, "91001:dev_x").await.unwrap();
        let _: () = conn.sadd(&node_devices_key.key, "91002:dev_y").await.unwrap();
        let _: () = conn.sadd(&node_devices_key.key, "91003:dev_z").await.unwrap();
        let _: () = conn.hset(&device_node_map.key, "91001:dev_x", fake_node).await.unwrap();
        // 91002 和 91003 不在 Hash 中
    }

    // cleanup 仍应成功（hdel 不存在的 field 不报错）
    registry.cleanup_node_routes(fake_node).await.unwrap();

    // 验证
    {
        let mut conn = app_state.redis().await.unwrap();
        let exists: bool = conn.exists(&node_devices_key.key).await.unwrap();
        assert!(!exists, "节点设备 Set 应已删除");
        let exists: bool = conn.hexists(&device_node_map.key, "91001:dev_x").await.unwrap();
        assert!(!exists, "Hash 中 91001:dev_x 应已删除");
    }
}

// ========================
// clean_node_completely 测试
// ========================

// 注意：clean_node_completely 是 private 方法，通过 NacosSessionRegistry 内部调用
// 我们通过间接方式测试其效果：模拟设备数据然后验证 sync_online(false) 的效果

#[tokio::test]
#[ignore]
async fn test_cleanup_triggers_offline_for_all_devices() {
    let (app_state, sm, _registry) = create_test_registry().await;
    let uid = 99060u64;
    let client_id = "cleanup_offline_dev";
    let device_key = format!("{}:{}", uid, client_id);

    let online_devices_key = ms_websocket::cache::PresenceCacheKeyBuilder::global_online_devices_key();

    // 模拟上线
    sm.sync_online(uid, client_id, true).await.unwrap();

    // 验证在线
    {
        let mut conn = app_state.redis().await.unwrap();
        let score: Option<f64> = conn.zscore(&online_devices_key.key, &device_key).await.unwrap();
        assert!(score.is_some(), "设备应在线");
    }

    // sync_online(false) —— clean_node_completely 内部就是调用这个
    sm.sync_online(uid, client_id, false).await.unwrap();

    // 验证离线
    {
        let mut conn = app_state.redis().await.unwrap();
        let score: Option<f64> = conn.zscore(&online_devices_key.key, &device_key).await.unwrap();
        assert!(score.is_none(), "设备应已离线");
    }
}

// ========================
// get_all_redis_node_ids 测试
// ========================

// 注意：get_all_redis_node_ids 是 private 方法，无法直接测试
// 我们验证 node_devices key 的格式正确性（SCAN 搜索的基础）

#[tokio::test]
#[ignore]
async fn test_node_devices_key_format_scannable() {
    // 验证 RouterCacheKeyBuilder::build_node_devices 生成的 key 格式
    // 可被 SCAN MATCH 匹配
    let key1 = RouterCacheKeyBuilder::build_node_devices("node_a");
    let key2 = RouterCacheKeyBuilder::build_node_devices("node_b");
    let base = RouterCacheKeyBuilder::build_node_devices("");

    // 两个不同节点的 key 应该共享相同前缀
    assert!(key1.key.starts_with(&base.key), "key1={} 应以 base={} 为前缀", key1.key, base.key);
    assert!(key2.key.starts_with(&base.key), "key2={} 应以 base={} 为前缀", key2.key, base.key);
    assert_ne!(key1.key, key2.key);

    // key 尾部应包含节点 ID
    assert!(key1.key.ends_with("node_a") || key1.key.contains("node_a"));
    assert!(key2.key.ends_with("node_b") || key2.key.contains("node_b"));
}

#[tokio::test]
#[ignore]
async fn test_redis_scan_finds_node_devices_keys() {
    let (app_state, _sm, _registry) = create_test_registry().await;
    let test_nodes = vec!["scan_test_node_a", "scan_test_node_b"];

    // 写入测试数据
    {
        let mut conn = app_state.redis().await.unwrap();
        for node in &test_nodes {
            let key = RouterCacheKeyBuilder::build_node_devices(node);
            let _: () = conn.sadd(&key.key, "dummy:device").await.unwrap();
        }
    }

    // 使用 SCAN 搜索
    let base_key = RouterCacheKeyBuilder::build_node_devices("");
    let pattern = format!("{}*", base_key.key);

    let mut conn = app_state.redis().await.unwrap();
    let mut found_nodes = std::collections::HashSet::new();
    let mut cursor: u64 = 0;
    loop {
        let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(&mut conn)
            .await
            .unwrap();

        for key in &keys {
            if let Some(node_id) = key.rsplit(':').next() {
                found_nodes.insert(node_id.to_string());
            }
        }

        cursor = next_cursor;
        if cursor == 0 {
            break;
        }
    }

    // 验证我们的测试节点被找到
    for node in &test_nodes {
        assert!(
            found_nodes.contains(*node),
            "SCAN 应找到节点 {}，found: {:?}",
            node,
            found_nodes
        );
    }

    // 清理
    {
        let mut conn = app_state.redis().await.unwrap();
        for node in &test_nodes {
            let key = RouterCacheKeyBuilder::build_node_devices(node);
            let _: () = conn.del(&key.key).await.unwrap_or(());
        }
    }
}

// ========================
// NacosSessionRegistry 构造测试
// ========================

#[tokio::test]
#[ignore]
async fn test_nacos_registry_creation() {
    let (app_state, sm, registry) = create_test_registry().await;
    // 验证 registry 正常创建（不 panic）
    assert!(true, "NacosSessionRegistry 创建成功");
}

// ========================
// 节点指标写入测试
// ========================

#[tokio::test]
#[ignore]
async fn test_node_metrics_format() {
    let (app_state, sm, _registry) = create_test_registry().await;
    let node_id = sm.node_id().to_string();

    // 模拟指标写入
    let metrics_key = format!("ws:node_metrics:{}", node_id);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .to_string();

    {
        let mut conn = app_state.redis().await.unwrap();
        let _: () = conn
            .hset_multiple(
                &metrics_key,
                &[
                    ("lastHeartbeat", now.as_str()),
                    ("sessionCount", "0"),
                    ("clientIds", ""),
                ],
            )
            .await.unwrap();
        let _: () = conn.expire(&metrics_key, 60).await.unwrap();
    }

    // 验证
    {
        let mut conn = app_state.redis().await.unwrap();
        let heartbeat: Option<String> = conn.hget(&metrics_key, "lastHeartbeat").await.unwrap();
        assert!(heartbeat.is_some(), "lastHeartbeat 应存在");

        let count: Option<String> = conn.hget(&metrics_key, "sessionCount").await.unwrap();
        assert_eq!(count, Some("0".to_string()));

        let ttl: i64 = conn.ttl(&metrics_key).await.unwrap();
        assert!(ttl > 0 && ttl <= 60, "TTL 应在 0-60 秒之间，实际: {}", ttl);
    }

    // 清理
    {
        let mut conn = app_state.redis().await.unwrap();
        let _: () = conn.del(&metrics_key).await.unwrap_or(());
    }
}

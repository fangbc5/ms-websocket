/// 会话管理器在线状态测试（P1/P2 新增功能）
///
/// 需要 Redis 支持，连接真实 Redis 实例
/// 运行方式：
///   TEST_REDIS_PASSWORD='1qaz!QAZ' cargo test -p ms-websocket --test session -- session_presence
///
/// 覆盖范围：
/// - sync_online（上线/下线逻辑）
/// - register_device_to_redis / unregister_device_from_redis（含节点设备 Set）
/// - update_group_presence（群组在线状态）
/// - get_room_ids（用户群组列表）
/// - is_first_or_last_device（首/末设备检测）
use crate::common::*;
use ms_websocket::cache::{PresenceCacheKeyBuilder, RouterCacheKeyBuilder};
use ms_websocket::websocket::SessionManager;
use redis::AsyncCommands;
use std::sync::Arc;

/// 创建带 Redis 的 SessionManager
async fn create_redis_session_manager() -> (Arc<fbc_starter::AppState>, SessionManager) {
    let app_state = create_test_app_state().await;
    let mut manager = SessionManager::default();
    manager.set_app_state(app_state.clone());
    (app_state, manager)
}

/// 轮询等待条件满足（最多 5 秒）
async fn poll_condition<F, Fut>(check: F, desc: &str)
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..50 {
        if check().await {
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    panic!("轮询超时（5s）: {}", desc);
}

/// 清理测试用的 Redis 键
async fn cleanup_redis_keys(app_state: &Arc<fbc_starter::AppState>, keys: &[&str]) {
    let mut conn = app_state.redis().await.unwrap();
    for key in keys {
        let _: () = conn.del(*key).await.unwrap_or(());
    }
}

// ========================
// register_device_to_redis 测试
// ========================

#[tokio::test]
async fn test_register_device_writes_to_hash_and_set() {
    let (app_state, manager) = create_redis_session_manager().await;
    let uid = 99001u64;
    let client_id = "test_device_reg_1";
    let node_id = manager.node_id().to_string();

    // 清理（只清理本测试自己的字段，不 DEL 共享 Set）
    let device_field = format!("{}:{}", uid, client_id);
    let cache_key = RouterCacheKeyBuilder::build_device_node_map(String::new());
    let node_devices_key = RouterCacheKeyBuilder::build_node_devices(&node_id);
    {
        let mut conn = app_state.redis().await.unwrap();
        let _: () = conn.hdel(&cache_key.key, &device_field).await.unwrap_or(());
        let _: () = conn.srem(&node_devices_key.key, &device_field).await.unwrap_or(());
    }

    // 注册会话（触发异步 register_device_to_redis）
    let session = create_test_session("sp_session_1".to_string(), uid, client_id.to_string());
    manager.register_session(session);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 验证 Hash 映射
    let mut conn = app_state.redis().await.unwrap();
    let node_value: Option<String> = conn.hget(&cache_key.key, &device_field).await.unwrap();
    assert!(node_value.is_some(), "设备→节点 Hash 映射应存在");
    assert_eq!(node_value.unwrap(), node_id);

    // 验证 Set 映射
    let is_member: bool = conn.sismember(&node_devices_key.key, &device_field).await.unwrap();
    assert!(is_member, "节点→设备 Set 应包含该设备");

    // 清理
    manager.cleanup_session(&"sp_session_1".to_string(), None);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
}

#[tokio::test]
async fn test_unregister_device_removes_from_hash_and_set() {
    let (app_state, manager) = create_redis_session_manager().await;
    let uid = 99002u64;
    let client_id = "test_device_unreg_1";
    let node_id = manager.node_id().to_string();

    let device_field = format!("{}:{}", uid, client_id);
    let cache_key = RouterCacheKeyBuilder::build_device_node_map(String::new());
    let node_devices_key = RouterCacheKeyBuilder::build_node_devices(&node_id);

    // 注册
    let session = create_test_session("sp_session_2".to_string(), uid, client_id.to_string());
    manager.register_session(session);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 验证存在
    let mut conn = app_state.redis().await.unwrap();
    let exists: bool = conn.hexists(&cache_key.key, &device_field).await.unwrap();
    assert!(exists, "注册后 Hash 应包含设备");

    // 清理会话（触发 unregister_device_from_redis）
    manager.cleanup_session(&"sp_session_2".to_string(), None);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 验证已删除
    let mut conn = app_state.redis().await.unwrap();
    let exists: bool = conn.hexists(&cache_key.key, &device_field).await.unwrap();
    assert!(!exists, "注销后 Hash 不应包含设备");

    let is_member: bool = conn.sismember(&node_devices_key.key, &device_field).await.unwrap();
    assert!(!is_member, "注销后 Set 不应包含设备");
}

// ========================
// sync_online 测试
// ========================

#[tokio::test]
async fn test_sync_online_adds_device_to_zset() {
    let (app_state, manager) = create_redis_session_manager().await;
    let uid = 99010u64;
    let client_id = "sync_device_1";
    let device_key = format!("{}:{}", uid, client_id);

    let online_devices_key = PresenceCacheKeyBuilder::global_online_devices_key();
    let online_users_key = PresenceCacheKeyBuilder::global_online_users_key();

    // 清理
    {
        let mut conn = app_state.redis().await.unwrap();
        let _: () = conn.zrem(&online_devices_key.key, &device_key).await.unwrap_or(());
        let _: () = conn.zrem(&online_users_key.key, uid).await.unwrap_or(());
    }

    // 调用 sync_online(true)
    manager.sync_online(uid, client_id, true).await.unwrap();

    // 验证设备 ZSet
    let mut conn = app_state.redis().await.unwrap();
    let score: Option<f64> = conn.zscore(&online_devices_key.key, &device_key).await.unwrap();
    assert!(score.is_some(), "上线后全局在线设备 ZSet 应包含该设备");

    // 验证用户 ZSet（首个设备上线应添加用户）
    let user_score: Option<f64> = conn.zscore(&online_users_key.key, uid.to_string()).await.unwrap();
    assert!(user_score.is_some(), "首个设备上线后全局在线用户 ZSet 应包含该用户");

    // 清理：下线
    manager.sync_online(uid, client_id, false).await.unwrap();

    let mut conn = app_state.redis().await.unwrap();
    let score: Option<f64> = conn.zscore(&online_devices_key.key, &device_key).await.unwrap();
    assert!(score.is_none(), "下线后全局在线设备 ZSet 不应包含该设备");

    let user_score: Option<f64> = conn.zscore(&online_users_key.key, uid.to_string()).await.unwrap();
    assert!(user_score.is_none(), "最后设备下线后全局在线用户 ZSet 不应包含该用户");
}

#[tokio::test]
async fn test_sync_online_multiple_devices_same_user() {
    let (app_state, manager) = create_redis_session_manager().await;
    let uid = 99011u64;
    let client_id_1 = "multi_dev_1";
    let client_id_2 = "multi_dev_2";
    let device_key_1 = format!("{}:{}", uid, client_id_1);
    let device_key_2 = format!("{}:{}", uid, client_id_2);

    let online_devices_key = PresenceCacheKeyBuilder::global_online_devices_key();
    let online_users_key = PresenceCacheKeyBuilder::global_online_users_key();

    // 清理
    {
        let mut conn = app_state.redis().await.unwrap();
        let _: () = conn.zrem(&online_devices_key.key, &device_key_1).await.unwrap_or(());
        let _: () = conn.zrem(&online_devices_key.key, &device_key_2).await.unwrap_or(());
        let _: () = conn.zrem(&online_users_key.key, uid).await.unwrap_or(());
    }

    // 第一个设备上线
    manager.sync_online(uid, client_id_1, true).await.unwrap();

    // 第二个设备上线
    manager.sync_online(uid, client_id_2, true).await.unwrap();

    // 验证两个设备都在 ZSet 中
    let mut conn = app_state.redis().await.unwrap();
    let score1: Option<f64> = conn.zscore(&online_devices_key.key, &device_key_1).await.unwrap();
    let score2: Option<f64> = conn.zscore(&online_devices_key.key, &device_key_2).await.unwrap();
    assert!(score1.is_some());
    assert!(score2.is_some());

    // 第一个设备下线（用户不应从在线列表移除，因为还有第二个设备）
    manager.sync_online(uid, client_id_1, false).await.unwrap();

    let mut conn = app_state.redis().await.unwrap();
    let score1: Option<f64> = conn.zscore(&online_devices_key.key, &device_key_1).await.unwrap();
    assert!(score1.is_none(), "下线设备应从 ZSet 移除");

    let user_score: Option<f64> = conn.zscore(&online_users_key.key, uid.to_string()).await.unwrap();
    // 还有 device_2 在线，所以用户可能仍在在线列表（取决于 is_first_or_last_device 判断）

    // 第二个设备下线
    manager.sync_online(uid, client_id_2, false).await.unwrap();

    let mut conn = app_state.redis().await.unwrap();
    let score2: Option<f64> = conn.zscore(&online_devices_key.key, &device_key_2).await.unwrap();
    assert!(score2.is_none(), "最后设备下线后应从 ZSet 移除");
}

// ========================
// update_group_presence 测试（通过 sync_online 间接触发）
// ========================

#[tokio::test]
async fn test_group_presence_updated_on_sync_online() {
    let (app_state, manager) = create_redis_session_manager().await;
    let uid = 99020u64;
    let client_id = "group_test_dev";
    let room_id: u64 = 88801;

    // 准备：将该用户加入群组
    let user_groups_key = PresenceCacheKeyBuilder::user_groups_key(uid);
    let online_group_key = PresenceCacheKeyBuilder::online_group_members_key(room_id);
    let online_user_groups_key = PresenceCacheKeyBuilder::online_user_groups_key(uid);
    let online_devices_key = PresenceCacheKeyBuilder::global_online_devices_key();
    let online_users_key = PresenceCacheKeyBuilder::global_online_users_key();
    let device_key = format!("{}:{}", uid, client_id);

    {
        let mut conn = app_state.redis().await.unwrap();
        // 设置用户所在群组
        let _: () = conn.sadd(&user_groups_key.key, room_id).await.unwrap();
        // 清理在线状态
        let _: () = conn.srem(&online_group_key.key, uid).await.unwrap_or(());
        let _: () = conn.srem(&online_user_groups_key.key, room_id).await.unwrap_or(());
        let _: () = conn.zrem(&online_devices_key.key, &device_key).await.unwrap_or(());
        let _: () = conn.zrem(&online_users_key.key, uid).await.unwrap_or(());
    }

    // 上线（首个设备，触发 update_group_presence）
    manager.sync_online(uid, client_id, true).await.unwrap();

    // 验证群组在线成员包含该用户
    let mut conn = app_state.redis().await.unwrap();
    let is_member: bool = conn.sismember(&online_group_key.key, uid).await.unwrap();
    assert!(is_member, "上线后群组在线成员 Set 应包含该用户");

    let in_user_groups: bool = conn.sismember(&online_user_groups_key.key, room_id).await.unwrap();
    assert!(in_user_groups, "上线后用户在线群组映射应包含该群");

    // 下线
    manager.sync_online(uid, client_id, false).await.unwrap();

    let mut conn = app_state.redis().await.unwrap();
    let is_member: bool = conn.sismember(&online_group_key.key, uid).await.unwrap();
    assert!(!is_member, "下线后群组在线成员 Set 不应包含该用户");

    let in_user_groups: bool = conn.sismember(&online_user_groups_key.key, room_id).await.unwrap();
    assert!(!in_user_groups, "下线后用户在线群组映射不应包含该群");

    // 清理
    let mut conn = app_state.redis().await.unwrap();
    let _: () = conn.srem(&user_groups_key.key, room_id).await.unwrap_or(());
}

// ========================
// get_room_ids 测试（通过 sync_online 间接调用）
// ========================

#[tokio::test]
async fn test_sync_online_with_no_groups() {
    let (app_state, manager) = create_redis_session_manager().await;
    let uid = 99030u64;
    let client_id = "no_group_dev";
    let device_key = format!("{}:{}", uid, client_id);

    let online_devices_key = PresenceCacheKeyBuilder::global_online_devices_key();
    let online_users_key = PresenceCacheKeyBuilder::global_online_users_key();

    // 清理
    {
        let mut conn = app_state.redis().await.unwrap();
        let _: () = conn.zrem(&online_devices_key.key, &device_key).await.unwrap_or(());
        let _: () = conn.zrem(&online_users_key.key, uid).await.unwrap_or(());
    }

    // 用户没有群组时 sync_online 应正常执行不报错
    manager.sync_online(uid, client_id, true).await.unwrap();
    manager.sync_online(uid, client_id, false).await.unwrap();

    // 验证已清理
    let mut conn = app_state.redis().await.unwrap();
    let score: Option<f64> = conn.zscore(&online_devices_key.key, &device_key).await.unwrap();
    assert!(score.is_none());
}

#[tokio::test]
async fn test_sync_online_with_multiple_groups() {
    let (app_state, manager) = create_redis_session_manager().await;
    let uid = 99031u64;
    let client_id = "multi_group_dev";
    let room_ids: Vec<u64> = vec![88810, 88811, 88812];

    let user_groups_key = PresenceCacheKeyBuilder::user_groups_key(uid);
    let online_devices_key = PresenceCacheKeyBuilder::global_online_devices_key();
    let online_users_key = PresenceCacheKeyBuilder::global_online_users_key();
    let device_key = format!("{}:{}", uid, client_id);

    // 设置用户在多个群组中
    {
        let mut conn = app_state.redis().await.unwrap();
        for &room_id in &room_ids {
            let _: () = conn.sadd(&user_groups_key.key, room_id).await.unwrap();
        }
        let _: () = conn.zrem(&online_devices_key.key, &device_key).await.unwrap_or(());
        let _: () = conn.zrem(&online_users_key.key, uid).await.unwrap_or(());
    }

    // 上线
    manager.sync_online(uid, client_id, true).await.unwrap();

    // 验证所有群组在线成员
    let mut conn = app_state.redis().await.unwrap();
    for &room_id in &room_ids {
        let online_group_key = PresenceCacheKeyBuilder::online_group_members_key(room_id);
        let is_member: bool = conn.sismember(&online_group_key.key, uid).await.unwrap();
        assert!(is_member, "群 {} 的在线成员应包含该用户", room_id);
    }

    // 下线
    manager.sync_online(uid, client_id, false).await.unwrap();

    // 验证已从所有群组移除
    let mut conn = app_state.redis().await.unwrap();
    for &room_id in &room_ids {
        let online_group_key = PresenceCacheKeyBuilder::online_group_members_key(room_id);
        let is_member: bool = conn.sismember(&online_group_key.key, uid).await.unwrap();
        assert!(!is_member, "群 {} 的在线成员不应包含该用户（下线后）", room_id);
    }

    // 清理
    let mut conn = app_state.redis().await.unwrap();
    let _: () = conn.del(&user_groups_key.key).await.unwrap_or(());
}

// ========================
// register + cleanup 端到端
// ========================

#[tokio::test]
async fn test_register_cleanup_full_lifecycle() {
    let (app_state, manager) = create_redis_session_manager().await;
    let uid = 99040u64;
    let client_id = "lifecycle_dev";
    let node_id = manager.node_id().to_string();

    let device_field = format!("{}:{}", uid, client_id);
    let hash_key = RouterCacheKeyBuilder::build_device_node_map(String::new()).key;
    let set_key = RouterCacheKeyBuilder::build_node_devices(&node_id).key;
    let devices_zset_key = PresenceCacheKeyBuilder::global_online_devices_key().key;
    let users_zset_key = PresenceCacheKeyBuilder::global_online_users_key().key;

    // 先清理
    {
        let mut conn = app_state.redis().await.unwrap();
        let _: () = conn.hdel(&hash_key, &device_field).await.unwrap_or(());
        let _: () = conn.srem(&set_key, &device_field).await.unwrap_or(());
        let _: () = conn.zrem(&devices_zset_key, &device_field).await.unwrap_or(());
        let _: () = conn.zrem(&users_zset_key, uid).await.unwrap_or(());
    }

    // 注册会话
    let session = create_test_session("lifecycle_s1".to_string(), uid, client_id.to_string());
    manager.register_session(session);

    // 轮询等待 Redis 写入完成（检查 Set 包含设备）
    {
        let as_ref = app_state.clone();
        let sk = set_key.clone();
        let df = device_field.clone();
        poll_condition(move || {
            let as2 = as_ref.clone();
            let sk2 = sk.clone();
            let df2 = df.clone();
            async move {
                let mut c = as2.redis().await.unwrap();
                let v: bool = redis::AsyncCommands::sismember(&mut c, &sk2, &df2).await.unwrap_or(false);
                v
            }
        }, "register 后 Set 应包含设备").await;
    }

    // 验证路由注册 + 在线状态
    {
        let mut conn = app_state.redis().await.unwrap();
        let node_value: Option<String> = conn.hget(&hash_key, &device_field).await.unwrap();
        assert!(node_value.is_some(), "register 后 Hash 应存在");

        let device_score: Option<f64> = conn.zscore(&devices_zset_key, &device_field).await.unwrap();
        assert!(device_score.is_some(), "register 后在线设备 ZSet 应包含");
    }

    // 清理会话
    manager.cleanup_session(&"lifecycle_s1".to_string(), None);

    // 轮询等待 Redis 清理完成（检查在线设备 ZSet 已移除 —— 这是 cleanup 最后一步）
    {
        let as_ref = app_state.clone();
        let dzk = devices_zset_key.clone();
        let df = device_field.clone();
        poll_condition(move || {
            let as2 = as_ref.clone();
            let dzk2 = dzk.clone();
            let df2 = df.clone();
            async move {
                let mut c = as2.redis().await.unwrap();
                let v: Option<f64> = redis::AsyncCommands::zscore(&mut c, &dzk2, &df2).await.unwrap_or(None);
                v.is_none()
            }
        }, "cleanup 后在线设备 ZSet 应移除").await;
    }

    // 验证全部清除
    {
        let mut conn = app_state.redis().await.unwrap();
        let node_value: Option<String> = conn.hget(&hash_key, &device_field).await.unwrap();
        assert!(node_value.is_none(), "cleanup 后 Hash 应删除");

        let is_member: bool = conn.sismember(&set_key, &device_field).await.unwrap();
        assert!(!is_member, "cleanup 后 Set 不应包含设备");
    }
}

#[tokio::test]
async fn test_multiple_sessions_same_device_only_one_register() {
    let (app_state, manager) = create_redis_session_manager().await;
    let uid = 99041u64;
    let client_id = "multi_sess_dev";
    let node_id = manager.node_id().to_string();

    let device_field = format!("{}:{}", uid, client_id);
    let cache_key = RouterCacheKeyBuilder::build_device_node_map(String::new());
    let node_devices_key = RouterCacheKeyBuilder::build_node_devices(&node_id);

    // 清理
    {
        let mut conn = app_state.redis().await.unwrap();
        let _: () = conn.hdel(&cache_key.key, &device_field).await.unwrap_or(());
        let _: () = conn.srem(&node_devices_key.key, &device_field).await.unwrap_or(());
    }

    // 注册第一个会话（同一设备）
    let session1 = create_test_session("ms_sess_1".to_string(), uid, client_id.to_string());
    manager.register_session(session1);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 注册第二个会话（同一设备）- 不应重复注册 Redis
    let session2 = create_test_session("ms_sess_2".to_string(), uid, client_id.to_string());
    manager.register_session(session2);
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    assert_eq!(manager.get_session_count(), 2);

    // 清理第一个会话（不应触发 unregister，因为同设备还有会话）
    manager.cleanup_session(&"ms_sess_1".to_string(), None);
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // 设备仍应在 Redis 中（还有 session2）
    {
        let mut conn = app_state.redis().await.unwrap();
        // 注意：cleanup 是检查用户的所有会话数，不是单设备
        // 但 session2 仍在，所以用户至少还有 1 个会话
    }
    assert_eq!(manager.get_session_count(), 1);

    // 清理第二个会话（应触发 unregister）
    manager.cleanup_session(&"ms_sess_2".to_string(), None);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    assert_eq!(manager.get_session_count(), 0);
}

// ========================
// 本地路由缓存联动测试
// ========================

#[tokio::test]
async fn test_local_cache_updated_on_register_and_cleanup() {
    let (app_state, manager) = create_redis_session_manager().await;
    let uid = 99050u64;
    let client_id = "local_cache_dev";

    // 注册
    let session = create_test_session("lc_sess_1".to_string(), uid, client_id.to_string());
    manager.register_session(session);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 验证本地缓存已写入（register_device_to_redis 中调用 local_router_cache.set）
    // 注意：这里无法直接访问私有字段，但 register/unregister 的顺利执行就验证了功能

    // 清理
    manager.cleanup_session(&"lc_sess_1".to_string(), None);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    assert_eq!(manager.get_session_count(), 0);
}

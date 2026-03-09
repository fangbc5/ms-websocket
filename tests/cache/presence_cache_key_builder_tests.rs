/// 在线状态缓存键构建器测试
///
/// 覆盖 PresenceCacheKeyBuilder 的 6 个方法：
/// - global_online_users_key
/// - global_online_devices_key
/// - online_group_members_key
/// - online_user_groups_key
/// - group_members_key
/// - user_groups_key
use ms_websocket::cache::PresenceCacheKeyBuilder;
use std::time::Duration;

// ========================
// 全局在线用户 ZSet
// ========================

#[test]
fn test_global_online_users_key_format() {
    let cache_key = PresenceCacheKeyBuilder::global_online_users_key();
    // key 格式: [prefix:]presence:global_users_online:obj
    assert!(cache_key.key.contains("presence"));
    assert!(cache_key.key.contains("global_users_online"));
    assert!(cache_key.key.contains("obj"));
}

#[test]
fn test_global_online_users_key_expire() {
    let cache_key = PresenceCacheKeyBuilder::global_online_users_key();
    // 30 天过期
    assert!(cache_key.expire.is_some());
    assert_eq!(cache_key.expire.unwrap(), Duration::from_secs(30 * 24 * 60 * 60));
}

#[test]
fn test_global_online_users_key_stable() {
    // 多次调用应返回相同的 key
    let key1 = PresenceCacheKeyBuilder::global_online_users_key();
    let key2 = PresenceCacheKeyBuilder::global_online_users_key();
    assert_eq!(key1.key, key2.key);
}

// ========================
// 全局在线设备 ZSet
// ========================

#[test]
fn test_global_online_devices_key_format() {
    let cache_key = PresenceCacheKeyBuilder::global_online_devices_key();
    assert!(cache_key.key.contains("presence"));
    assert!(cache_key.key.contains("global_devices_online"));
    assert!(cache_key.key.contains("obj"));
}

#[test]
fn test_global_online_devices_key_expire() {
    let cache_key = PresenceCacheKeyBuilder::global_online_devices_key();
    assert!(cache_key.expire.is_some());
    assert_eq!(cache_key.expire.unwrap(), Duration::from_secs(30 * 24 * 60 * 60));
}

#[test]
fn test_global_online_devices_key_stable() {
    let key1 = PresenceCacheKeyBuilder::global_online_devices_key();
    let key2 = PresenceCacheKeyBuilder::global_online_devices_key();
    assert_eq!(key1.key, key2.key);
}

#[test]
fn test_global_keys_are_different() {
    let users_key = PresenceCacheKeyBuilder::global_online_users_key();
    let devices_key = PresenceCacheKeyBuilder::global_online_devices_key();
    assert_ne!(users_key.key, devices_key.key);
}

// ========================
// 群组在线成员 Set
// ========================

#[test]
fn test_online_group_members_key_format() {
    let cache_key = PresenceCacheKeyBuilder::online_group_members_key(100);
    assert!(cache_key.key.contains("presence"));
    assert!(cache_key.key.contains("group_members_online"));
    assert!(cache_key.key.contains("id"));
    assert!(cache_key.key.contains("100"));
}

#[test]
fn test_online_group_members_key_expire() {
    let cache_key = PresenceCacheKeyBuilder::online_group_members_key(100);
    assert!(cache_key.expire.is_some());
    assert_eq!(cache_key.expire.unwrap(), Duration::from_secs(30 * 24 * 60 * 60));
}

#[test]
fn test_online_group_members_different_rooms() {
    let key1 = PresenceCacheKeyBuilder::online_group_members_key(100);
    let key2 = PresenceCacheKeyBuilder::online_group_members_key(200);
    assert_ne!(key1.key, key2.key);
    assert!(key1.key.contains("100"));
    assert!(key2.key.contains("200"));
}

#[test]
fn test_online_group_members_same_room_stable() {
    let key1 = PresenceCacheKeyBuilder::online_group_members_key(100);
    let key2 = PresenceCacheKeyBuilder::online_group_members_key(100);
    assert_eq!(key1.key, key2.key);
}

// ========================
// 用户在线群组映射 Set
// ========================

#[test]
fn test_online_user_groups_key_format() {
    let cache_key = PresenceCacheKeyBuilder::online_user_groups_key(1001);
    assert!(cache_key.key.contains("presence"));
    assert!(cache_key.key.contains("users_group_online"));
    assert!(cache_key.key.contains("groups"));
    assert!(cache_key.key.contains("1001"));
}

#[test]
fn test_online_user_groups_key_expire() {
    let cache_key = PresenceCacheKeyBuilder::online_user_groups_key(1001);
    assert!(cache_key.expire.is_some());
    assert_eq!(cache_key.expire.unwrap(), Duration::from_secs(30 * 24 * 60 * 60));
}

#[test]
fn test_online_user_groups_different_users() {
    let key1 = PresenceCacheKeyBuilder::online_user_groups_key(1001);
    let key2 = PresenceCacheKeyBuilder::online_user_groups_key(2002);
    assert_ne!(key1.key, key2.key);
}

// ========================
// 群组成员 Set（离线也包含）
// ========================

#[test]
fn test_group_members_key_format() {
    let cache_key = PresenceCacheKeyBuilder::group_members_key(300);
    assert!(cache_key.key.contains("presence"));
    assert!(cache_key.key.contains("group_members"));
    assert!(cache_key.key.contains("id"));
    assert!(cache_key.key.contains("300"));
    // 不应和 online 版本冲突
    let online_key = PresenceCacheKeyBuilder::online_group_members_key(300);
    assert_ne!(cache_key.key, online_key.key);
}

#[test]
fn test_group_members_key_expire() {
    let cache_key = PresenceCacheKeyBuilder::group_members_key(300);
    assert!(cache_key.expire.is_some());
    assert_eq!(cache_key.expire.unwrap(), Duration::from_secs(30 * 24 * 60 * 60));
}

#[test]
fn test_group_members_different_rooms() {
    let key1 = PresenceCacheKeyBuilder::group_members_key(300);
    let key2 = PresenceCacheKeyBuilder::group_members_key(400);
    assert_ne!(key1.key, key2.key);
}

// ========================
// 用户群组映射 Set（离线也包含）
// ========================

#[test]
fn test_user_groups_key_format() {
    let cache_key = PresenceCacheKeyBuilder::user_groups_key(1001);
    assert!(cache_key.key.contains("presence"));
    assert!(cache_key.key.contains("users_group"));
    assert!(cache_key.key.contains("groups"));
    assert!(cache_key.key.contains("1001"));
    // 不应和 online 版本冲突
    let online_key = PresenceCacheKeyBuilder::online_user_groups_key(1001);
    assert_ne!(cache_key.key, online_key.key);
}

#[test]
fn test_user_groups_key_expire() {
    let cache_key = PresenceCacheKeyBuilder::user_groups_key(1001);
    assert!(cache_key.expire.is_some());
    assert_eq!(cache_key.expire.unwrap(), Duration::from_secs(30 * 24 * 60 * 60));
}

#[test]
fn test_user_groups_different_users() {
    let key1 = PresenceCacheKeyBuilder::user_groups_key(1001);
    let key2 = PresenceCacheKeyBuilder::user_groups_key(2002);
    assert_ne!(key1.key, key2.key);
}

// ========================
// 跨类型键不冲突性
// ========================

#[test]
fn test_all_key_types_distinct() {
    let keys = vec![
        PresenceCacheKeyBuilder::global_online_users_key().key,
        PresenceCacheKeyBuilder::global_online_devices_key().key,
        PresenceCacheKeyBuilder::online_group_members_key(1).key,
        PresenceCacheKeyBuilder::online_user_groups_key(1).key,
        PresenceCacheKeyBuilder::group_members_key(1).key,
        PresenceCacheKeyBuilder::user_groups_key(1).key,
    ];

    // 所有 key 相互不同
    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            assert_ne!(
                keys[i], keys[j],
                "key[{}]={} 与 key[{}]={} 重复",
                i, keys[i], j, keys[j]
            );
        }
    }
}

#[test]
fn test_all_keys_contain_presence_modular() {
    let keys = vec![
        PresenceCacheKeyBuilder::global_online_users_key().key,
        PresenceCacheKeyBuilder::global_online_devices_key().key,
        PresenceCacheKeyBuilder::online_group_members_key(1).key,
        PresenceCacheKeyBuilder::online_user_groups_key(1).key,
        PresenceCacheKeyBuilder::group_members_key(1).key,
        PresenceCacheKeyBuilder::user_groups_key(1).key,
    ];

    for key in &keys {
        assert!(key.contains("presence"), "key {} 应包含 'presence' 模块标识", key);
    }
}

#[test]
fn test_all_presence_keys_have_30_day_expire() {
    let expire_30_days = Duration::from_secs(30 * 24 * 60 * 60);

    let keys = vec![
        PresenceCacheKeyBuilder::global_online_users_key(),
        PresenceCacheKeyBuilder::global_online_devices_key(),
        PresenceCacheKeyBuilder::online_group_members_key(1),
        PresenceCacheKeyBuilder::online_user_groups_key(1),
        PresenceCacheKeyBuilder::group_members_key(1),
        PresenceCacheKeyBuilder::user_groups_key(1),
    ];

    for cache_key in &keys {
        assert_eq!(
            cache_key.expire,
            Some(expire_30_days),
            "key {} expire 应为 30 天",
            cache_key.key
        );
    }
}

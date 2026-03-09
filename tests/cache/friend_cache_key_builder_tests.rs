/// 好友关系缓存键构建器测试
///
/// 覆盖 FriendCacheKeyBuilder 的 3 个方法：
/// - user_friends_key
/// - reverse_friends_key
/// - friend_status_key
use ms_websocket::cache::FriendCacheKeyBuilder;
use std::time::Duration;

// ========================
// 用户好友列表 Set
// ========================

#[test]
fn test_user_friends_key_format() {
    let cache_key = FriendCacheKeyBuilder::user_friends_key(1001);
    assert!(cache_key.key.contains("friend"));
    assert!(cache_key.key.contains("user_friends"));
    assert!(cache_key.key.contains("number"));
    assert!(cache_key.key.contains("1001"));
}

#[test]
fn test_user_friends_key_no_expire() {
    let cache_key = FriendCacheKeyBuilder::user_friends_key(1001);
    // 好友列表不过期
    assert!(cache_key.expire.is_none());
}

#[test]
fn test_user_friends_key_different_users() {
    let key1 = FriendCacheKeyBuilder::user_friends_key(1001);
    let key2 = FriendCacheKeyBuilder::user_friends_key(2002);
    assert_ne!(key1.key, key2.key);
    assert!(key1.key.contains("1001"));
    assert!(key2.key.contains("2002"));
}

#[test]
fn test_user_friends_key_stable() {
    let key1 = FriendCacheKeyBuilder::user_friends_key(1001);
    let key2 = FriendCacheKeyBuilder::user_friends_key(1001);
    assert_eq!(key1.key, key2.key);
}

// ========================
// 反向好友关系 Set
// ========================

#[test]
fn test_reverse_friends_key_format() {
    let cache_key = FriendCacheKeyBuilder::reverse_friends_key(1001);
    assert!(cache_key.key.contains("friend"));
    assert!(cache_key.key.contains("reverse_friends"));
    assert!(cache_key.key.contains("number"));
    assert!(cache_key.key.contains("1001"));
}

#[test]
fn test_reverse_friends_key_no_expire() {
    let cache_key = FriendCacheKeyBuilder::reverse_friends_key(1001);
    assert!(cache_key.expire.is_none());
}

#[test]
fn test_reverse_friends_key_different_users() {
    let key1 = FriendCacheKeyBuilder::reverse_friends_key(1001);
    let key2 = FriendCacheKeyBuilder::reverse_friends_key(2002);
    assert_ne!(key1.key, key2.key);
}

#[test]
fn test_reverse_friends_vs_user_friends() {
    // 同一用户的 user_friends 和 reverse_friends 应该是不同 key
    let user_key = FriendCacheKeyBuilder::user_friends_key(1001);
    let reverse_key = FriendCacheKeyBuilder::reverse_friends_key(1001);
    assert_ne!(user_key.key, reverse_key.key);
}

// ========================
// 好友关系状态 String
// ========================

#[test]
fn test_friend_status_key_format() {
    let cache_key = FriendCacheKeyBuilder::friend_status_key(1001, 2002);
    assert!(cache_key.key.contains("friend"));
    assert!(cache_key.key.contains("relation_status"));
    assert!(cache_key.key.contains("string"));
    assert!(cache_key.key.contains("1001"));
    assert!(cache_key.key.contains("2002"));
}

#[test]
fn test_friend_status_key_expire_7_days() {
    let cache_key = FriendCacheKeyBuilder::friend_status_key(1001, 2002);
    assert!(cache_key.expire.is_some());
    assert_eq!(cache_key.expire.unwrap(), Duration::from_secs(7 * 24 * 60 * 60));
}

#[test]
fn test_friend_status_key_order_matters() {
    // friend_status_key(a, b) != friend_status_key(b, a)
    let key1 = FriendCacheKeyBuilder::friend_status_key(1001, 2002);
    let key2 = FriendCacheKeyBuilder::friend_status_key(2002, 1001);
    assert_ne!(key1.key, key2.key);
}

#[test]
fn test_friend_status_key_same_user_stable() {
    let key1 = FriendCacheKeyBuilder::friend_status_key(1001, 2002);
    let key2 = FriendCacheKeyBuilder::friend_status_key(1001, 2002);
    assert_eq!(key1.key, key2.key);
}

#[test]
fn test_friend_status_key_different_pairs() {
    let key1 = FriendCacheKeyBuilder::friend_status_key(1001, 2002);
    let key2 = FriendCacheKeyBuilder::friend_status_key(1001, 3003);
    assert_ne!(key1.key, key2.key);
}

// ========================
// 跨类型键不冲突性
// ========================

#[test]
fn test_all_friend_key_types_distinct() {
    let keys = vec![
        FriendCacheKeyBuilder::user_friends_key(1001).key,
        FriendCacheKeyBuilder::reverse_friends_key(1001).key,
        FriendCacheKeyBuilder::friend_status_key(1001, 2002).key,
    ];

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
fn test_all_friend_keys_contain_friend_modular() {
    let keys = vec![
        FriendCacheKeyBuilder::user_friends_key(1001).key,
        FriendCacheKeyBuilder::reverse_friends_key(1001).key,
        FriendCacheKeyBuilder::friend_status_key(1001, 2002).key,
    ];

    for key in &keys {
        assert!(key.contains("friend"), "key {} 应包含 'friend' 模块标识", key);
    }
}

#[test]
fn test_friend_vs_presence_keys_distinct() {
    // 确保 friend 和 presence 的 key 不碰撞
    use ms_websocket::cache::PresenceCacheKeyBuilder;

    let friend_key = FriendCacheKeyBuilder::user_friends_key(1001).key;
    let presence_key = PresenceCacheKeyBuilder::user_groups_key(1001).key;
    assert_ne!(friend_key, presence_key);
}

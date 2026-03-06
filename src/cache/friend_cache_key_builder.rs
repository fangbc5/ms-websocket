/// 好友关系缓存键构建器
///
/// 管理好友列表、反向好友关系、好友状态等缓存键
use fbc_starter::cache::{get_cache_prefix, CacheKey, CacheKeyBuilder, ValueType, FRIEND};
use std::time::Duration;

use crate::cache::constants::EXPIRE_FRIEND_STATUS;
use crate::types::UserId;

/// 好友关系缓存键构建器
pub struct FriendCacheKeyBuilder;

impl FriendCacheKeyBuilder {
    /// 用户好友列表 Set（member=friendUid）
    pub fn user_friends_key(uid: UserId) -> CacheKey {
        UserFriendsKeyBuilder.key(&[&uid])
    }

    /// 反向好友关系 Set（即"谁把我加为好友"，member=uid）
    pub fn reverse_friends_key(uid: UserId) -> CacheKey {
        ReverseFriendsKeyBuilder.key(&[&uid])
    }

    /// 好友关系状态（1/0）
    pub fn friend_status_key(uid1: UserId, uid2: UserId) -> CacheKey {
        FriendStatusKeyBuilder.key(&[&uid1, &uid2])
    }
}

// ========== 内部构建器 ==========

/// 用户好友列表 Set
struct UserFriendsKeyBuilder;

impl CacheKeyBuilder for UserFriendsKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_modular(&self) -> Option<&str> {
        Some(FRIEND)
    }

    fn get_table(&self) -> &str {
        fbc_starter::cache::friend::USER_FRIENDS
    }

    fn get_value_type(&self) -> ValueType {
        ValueType::Number
    }
}

/// 反向好友关系 Set
struct ReverseFriendsKeyBuilder;

impl CacheKeyBuilder for ReverseFriendsKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_modular(&self) -> Option<&str> {
        Some(FRIEND)
    }

    fn get_table(&self) -> &str {
        fbc_starter::cache::friend::REVERSE_FRIENDS
    }

    fn get_value_type(&self) -> ValueType {
        ValueType::Number
    }
}

/// 好友关系状态
struct FriendStatusKeyBuilder;

impl CacheKeyBuilder for FriendStatusKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_modular(&self) -> Option<&str> {
        Some(FRIEND)
    }

    fn get_table(&self) -> &str {
        fbc_starter::cache::friend::RELATION_STATUS
    }

    fn get_value_type(&self) -> ValueType {
        ValueType::String
    }

    fn get_expire(&self) -> Option<Duration> {
        Some(EXPIRE_FRIEND_STATUS)
    }
}

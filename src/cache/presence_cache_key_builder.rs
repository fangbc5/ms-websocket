/// 在线状态缓存键构建器
///
/// 管理用户/设备在线状态、群组在线成员等缓存键
use fbc_starter::cache::{get_cache_prefix, CacheKey, CacheKeyBuilder, ValueType, PRESENCE};
use std::time::Duration;

use crate::cache::constants::EXPIRE_PRESENCE;
use crate::types::UserId;

/// 全局在线用户 ZSet 构建器
pub struct PresenceCacheKeyBuilder;

impl PresenceCacheKeyBuilder {
    /// 全局在线用户 ZSet（score=时间戳，member=uid）
    pub fn global_online_users_key() -> CacheKey {
        GlobalOnlineUsersKeyBuilder.key(&[] as &[&dyn ToString])
    }

    /// 全局在线设备 ZSet（score=时间戳，member=uid:clientId）
    pub fn global_online_devices_key() -> CacheKey {
        GlobalOnlineDevicesKeyBuilder.key(&[] as &[&dyn ToString])
    }

    /// 群组在线成员 Set（member=uid）
    pub fn online_group_members_key(room_id: u64) -> CacheKey {
        OnlineGroupMembersKeyBuilder.key(&[&room_id])
    }

    /// 用户在线群组映射 Set（member=roomId）
    pub fn online_user_groups_key(uid: UserId) -> CacheKey {
        OnlineUserGroupsKeyBuilder.key(&[&uid])
    }

    /// 群组成员 Set（member=uid）
    pub fn group_members_key(room_id: u64) -> CacheKey {
        GroupMembersKeyBuilder.key(&[&room_id])
    }

    /// 用户群组映射 Set（member=roomId）
    pub fn user_groups_key(uid: UserId) -> CacheKey {
        UserGroupsKeyBuilder.key(&[&uid])
    }
}

// ========== 内部构建器 ==========

/// 全局在线用户 ZSet
struct GlobalOnlineUsersKeyBuilder;

impl CacheKeyBuilder for GlobalOnlineUsersKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_modular(&self) -> Option<&str> {
        Some(PRESENCE)
    }

    fn get_table(&self) -> &str {
        fbc_starter::cache::presence::GLOBAL_USERS_ONLINE
    }

    fn get_value_type(&self) -> ValueType {
        ValueType::Obj
    }

    fn get_expire(&self) -> Option<Duration> {
        Some(EXPIRE_PRESENCE)
    }
}

/// 全局在线设备 ZSet
struct GlobalOnlineDevicesKeyBuilder;

impl CacheKeyBuilder for GlobalOnlineDevicesKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_modular(&self) -> Option<&str> {
        Some(PRESENCE)
    }

    fn get_table(&self) -> &str {
        fbc_starter::cache::presence::GLOBAL_DEVICES_ONLINE
    }

    fn get_value_type(&self) -> ValueType {
        ValueType::Obj
    }

    fn get_expire(&self) -> Option<Duration> {
        Some(EXPIRE_PRESENCE)
    }
}

/// 群组在线成员 Set
struct OnlineGroupMembersKeyBuilder;

impl CacheKeyBuilder for OnlineGroupMembersKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_modular(&self) -> Option<&str> {
        Some(PRESENCE)
    }

    fn get_table(&self) -> &str {
        fbc_starter::cache::presence::GROUP_MEMBERS_ONLINE
    }

    fn get_field(&self) -> Option<&str> {
        Some("id")
    }

    fn get_value_type(&self) -> ValueType {
        ValueType::Obj
    }

    fn get_expire(&self) -> Option<Duration> {
        Some(EXPIRE_PRESENCE)
    }
}

/// 用户在线群组映射 Set
struct OnlineUserGroupsKeyBuilder;

impl CacheKeyBuilder for OnlineUserGroupsKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_modular(&self) -> Option<&str> {
        Some(PRESENCE)
    }

    fn get_table(&self) -> &str {
        fbc_starter::cache::presence::USERS_GROUP_ONLINE
    }

    fn get_field(&self) -> Option<&str> {
        Some("groups")
    }

    fn get_value_type(&self) -> ValueType {
        ValueType::String
    }

    fn get_expire(&self) -> Option<Duration> {
        Some(EXPIRE_PRESENCE)
    }
}

/// 群组成员 Set
struct GroupMembersKeyBuilder;

impl CacheKeyBuilder for GroupMembersKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_modular(&self) -> Option<&str> {
        Some(PRESENCE)
    }

    fn get_table(&self) -> &str {
        fbc_starter::cache::presence::GROUP_MEMBERS
    }

    fn get_field(&self) -> Option<&str> {
        Some("id")
    }

    fn get_value_type(&self) -> ValueType {
        ValueType::Obj
    }

    fn get_expire(&self) -> Option<Duration> {
        Some(EXPIRE_PRESENCE)
    }
}

/// 用户群组映射 Set
struct UserGroupsKeyBuilder;

impl CacheKeyBuilder for UserGroupsKeyBuilder {
    fn get_prefix(&self) -> Option<&str> {
        get_cache_prefix().map(|s| s.as_str())
    }

    fn get_modular(&self) -> Option<&str> {
        Some(PRESENCE)
    }

    fn get_table(&self) -> &str {
        fbc_starter::cache::presence::USERS_GROUP
    }

    fn get_field(&self) -> Option<&str> {
        Some("groups")
    }

    fn get_value_type(&self) -> ValueType {
        ValueType::Number
    }

    fn get_expire(&self) -> Option<Duration> {
        Some(EXPIRE_PRESENCE)
    }
}

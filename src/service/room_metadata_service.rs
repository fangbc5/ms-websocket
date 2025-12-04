/// 房间元数据服务
///
/// 功能：
/// 1. 管理房间元数据（创建者、管理员、房间类型等）
/// 2. 维护屏幕共享状态
/// 3. 管理全体静音状态
use crate::{
    cache::{
        CloseRoomCacheKeyBuilder, RoomAdminMetadataCacheKeyBuilder, RoomMetadataCacheKeyBuilder,
    },
    types::{RoomId, UserId},
};
use fbc_starter::AppState;
use redis::AsyncCommands;
use std::sync::Arc;

/// 房间元数据服务
pub struct RoomMetadataService {
    app_state: Arc<AppState>,
}

impl RoomMetadataService {
    /// 创建新的房间元数据服务
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }

    /// 打开房间
    pub async fn open_room(&self, room_id: RoomId) -> anyhow::Result<()> {
        let mut conn = self.app_state.redis()?;
        let key = CloseRoomCacheKeyBuilder::builder(room_id);
        if let Some(expire) = key.expire {
            let _: () = conn.set_ex(&key.key, "false", expire.as_secs()).await?;
        } else {
            let _: () = conn.set(&key.key, "false").await?;
        }
        Ok(())
    }

    /// 检查房间是否已关闭
    pub async fn is_room_closed(&self, room_id: RoomId) -> anyhow::Result<bool> {
        let mut conn = self.app_state.redis()?;
        let key = CloseRoomCacheKeyBuilder::builder(room_id);
        let result: Option<String> = conn.get(&key.key).await?;
        match result {
            Some(val) => Ok(val != "true"),
            None => Ok(true), // 不存在视为已关闭
        }
    }

    /// 标记房间已关闭
    pub async fn mark_room_closed(&self, room_id: RoomId) -> anyhow::Result<()> {
        // 清除开始时间
        self.set_room_metadata(room_id, "startTime", None::<String>)
            .await?;
        // 删除关闭房间键
        let mut conn = self.app_state.redis()?;
        let key = CloseRoomCacheKeyBuilder::builder(room_id);
        let _: () = conn.del(&key.key).await?;
        Ok(())
    }

    /// 设置房间元数据字段
    pub async fn set_room_metadata<T: serde::Serialize>(
        &self,
        room_id: RoomId,
        field: &str,
        value: Option<T>,
    ) -> anyhow::Result<()> {
        let mut conn = self.app_state.redis()?;
        let key = RoomMetadataCacheKeyBuilder::builder(room_id, field);
        let hash_field = key.field.as_deref().unwrap_or("");
        if let Some(val) = value {
            let json = serde_json::to_string(&val)?;
            let _: () = conn.hset(&key.key, hash_field, &json).await?;
            // 设置过期时间
            if let Some(expire) = key.expire {
                let _: () = conn.expire(&key.key, expire.as_secs() as i64).await?;
            }
        } else {
            let _: () = conn.hdel(&key.key, hash_field).await?;
        }
        Ok(())
    }

    /// 获取房间元数据字段
    pub async fn get_room_metadata<T: serde::de::DeserializeOwned>(
        &self,
        room_id: RoomId,
        field: &str,
    ) -> anyhow::Result<Option<T>> {
        let mut conn = self.app_state.redis()?;
        let key = RoomMetadataCacheKeyBuilder::builder(room_id, field);
        let hash_field = key.field.as_deref().unwrap_or("");
        let result: Option<String> = conn.hget(&key.key, hash_field).await?;
        match result {
            Some(json) => Ok(serde_json::from_str(&json).ok()),
            None => Ok(None),
        }
    }

    /// 获取房间创建者
    pub async fn get_room_creator(&self, room_id: RoomId) -> anyhow::Result<Option<UserId>> {
        self.get_room_metadata(room_id, "creator").await
    }

    /// 设置房间创建者
    pub async fn set_room_creator(&self, room_id: RoomId, creator_uid: UserId) -> anyhow::Result<()> {
        self.set_room_metadata(room_id, "creator", Some(creator_uid))
            .await
    }

    /// 获取房间接通电话时间
    pub async fn get_room_start_time(&self, room_id: RoomId) -> anyhow::Result<Option<i64>> {
        self.get_room_metadata(room_id, "startTime").await
    }

    /// 设置房间接通电话时间
    pub async fn set_room_start_time(&self, room_id: RoomId) -> anyhow::Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.set_room_metadata(room_id, "startTime", Some(now))
            .await
    }

    /// 添加房间管理员
    pub async fn add_room_admin(&self, room_id: RoomId, admin_uid: UserId) -> anyhow::Result<()> {
        let mut conn = self.app_state.redis()?;
        let key = RoomAdminMetadataCacheKeyBuilder::builder(room_id);
        let _: () = conn.sadd(&key.key, admin_uid.to_string()).await?;
        // 设置过期时间
        if let Some(expire) = key.expire {
            let _: () = conn.expire(&key.key, expire.as_secs() as i64).await?;
        }
        Ok(())
    }

    /// 获取房间媒体类型
    pub async fn get_room_medium_type(&self, room_id: RoomId) -> anyhow::Result<Option<bool>> {
        self.get_room_metadata(room_id, "mediumType").await
    }

    /// 设置房间媒体类型
    pub async fn set_room_medium_type(&self, room_id: RoomId, is_video: bool) -> anyhow::Result<()> {
        self.set_room_metadata(room_id, "mediumType", Some(is_video))
            .await
    }

    /// 设置租户 ID
    pub async fn set_tenant_id(&self, room_id: RoomId, tenant_id: u64) -> anyhow::Result<()> {
        self.set_room_metadata(room_id, "tenantId", Some(tenant_id))
            .await
    }

    /// 获取租户 ID
    pub async fn get_tenant_id(&self, room_id: RoomId) -> anyhow::Result<Option<i64>> {
        self.get_room_metadata(room_id, "tenantId").await
    }

    /// 设置房间类型
    pub async fn set_room_type(&self, room_id: RoomId, r#type: u8) -> anyhow::Result<()> {
        self.set_room_metadata(room_id, "type", Some(r#type)).await
    }

    /// 获取房间类型
    pub async fn get_room_type(&self, room_id: RoomId) -> anyhow::Result<Option<u8>> {
        self.get_room_metadata(room_id, "type").await
    }

    /// 获取房间管理员列表
    pub async fn get_room_admins(
        &self,
        room_id: RoomId,
    ) -> anyhow::Result<std::collections::HashSet<UserId>> {
        let mut conn = self.app_state.redis()?;
        let key = RoomAdminMetadataCacheKeyBuilder::builder(room_id);
        let members: Vec<String> = conn.smembers(&key.key).await?;
        let admins: std::collections::HashSet<UserId> =
            members.into_iter().filter_map(|s| s.parse::<UserId>().ok()).collect();
        Ok(admins)
    }

    /// 检查用户是否是房间管理员
    pub async fn is_room_admin(&self, room_id: RoomId, uid: UserId) -> anyhow::Result<bool> {
        // 创建者也是管理员
        if let Some(creator) = self.get_room_creator(room_id).await? {
            if creator == uid {
                return Ok(true);
            }
        }
        // 检查管理员列表
        let admins = self.get_room_admins(room_id).await?;
        Ok(admins.contains(&uid))
    }

    /// 设置全体静音状态
    pub async fn set_all_muted(&self, room_id: RoomId, muted: bool) -> anyhow::Result<()> {
        self.set_room_metadata(room_id, "allMuted", Some(muted))
            .await
    }

    /// 获取全体静音状态
    pub async fn is_all_muted(&self, room_id: RoomId) -> anyhow::Result<bool> {
        Ok(self
            .get_room_metadata::<bool>(room_id, "allMuted")
            .await?
            .unwrap_or(false))
    }

    /// 设置屏幕共享状态
    pub async fn set_screen_sharing(
        &self,
        room_id: RoomId,
        user_id: UserId,
        sharing: bool,
    ) -> anyhow::Result<()> {
        if sharing {
            self.set_room_metadata(room_id, "screenSharingUser", Some(user_id))
                .await
        } else {
            self.set_room_metadata(room_id, "screenSharingUser", None::<i64>)
                .await
        }
    }

    /// 获取当前屏幕共享用户 ID
    pub async fn get_screen_sharing_user(&self, room_id: RoomId) -> anyhow::Result<Option<UserId>> {
        self.get_room_metadata(room_id, "screenSharingUser").await
    }

    /// 检查是否正在屏幕共享
    pub async fn is_screen_sharing(&self, room_id: RoomId) -> anyhow::Result<bool> {
        Ok(self.get_screen_sharing_user(room_id).await?.is_some())
    }
}

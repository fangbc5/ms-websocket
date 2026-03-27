use std::{collections::HashSet, sync::Arc};

use fbc_starter::AppState;
use redis::AsyncCommands;
use tracing::{info, warn};

use crate::{
    cache::{UserRoomsCacheKeyBuilder, VideoRoomsCacheKeyBuilder},
    enums::ws_req_type::WsMsgTypeEnum,
    model::{
        WsBaseResp,
        entity::Room,
        vo::{user_join_room_vo::UserJoinRoomVO, video_signal_vo::VideoSignalVO},
    },
    service::{PushService, RoomMetadataService, RoomTimeoutService},
    types::{RoomId, UserId},
};

/// 视频聊天服务
pub struct VideoChatService {
    app_state: Arc<AppState>,
    push_service: Arc<PushService>,
    room_metadata_service: Arc<RoomMetadataService>,
    /// 延迟注入：避免 VideoChatService ↔ RoomTimeoutService 循环引用
    room_timeout_service: std::sync::OnceLock<std::sync::Weak<RoomTimeoutService>>,
}

impl VideoChatService {
    /// 创建新的视频聊天服务
    pub fn new(
        app_state: Arc<AppState>,
        push_service: Arc<PushService>,
        room_metadata_service: Arc<RoomMetadataService>,
    ) -> Self {
        Self {
            app_state,
            push_service,
            room_metadata_service,
            room_timeout_service: std::sync::OnceLock::new(),
        }
    }

    /// 延迟注入 RoomTimeoutService（解决循环依赖）
    ///
    /// 在 Services::new() 中所有服务构建完成后调用
    pub fn set_room_timeout_service(&self, service: Arc<RoomTimeoutService>) {
        let _ = self
            .room_timeout_service
            .set(Arc::downgrade(&service));
    }

    /// 获取 RoomTimeoutService（从 Weak 升级）
    fn get_room_timeout_service(&self) -> Option<Arc<RoomTimeoutService>> {
        self.room_timeout_service
            .get()
            .and_then(|weak| weak.upgrade())
    }

    /// 获取房间元数据
    pub async fn get_room_metadata(&self, room_id: RoomId) -> anyhow::Result<Option<Room>> {
        // TODO: 从缓存或数据库获取房间信息
        // 这里需要实现缓存查询逻辑
        warn!("get_room_metadata 未实现: room_id={}", room_id);
        Ok(None)
    }

    /// 用户加入视频房间
    pub async fn join_room(&self, uid: UserId, room: Room) -> anyhow::Result<Vec<UserId>> {
        let room_id = room.id;
        let room_type = room.room_type;

        // 1. 处理房间类型
        let resp = UserJoinRoomVO::new(uid as u64, room_id as u64);

        if room_type == 1 {
            // 群聊
            // TODO: 非群成员无法加入
            // TODO: 显示群名称和头像
        } else if room_type == 2 {
            // 单聊
            // TODO: 大于2人，私聊房间已满
            // TODO: 显示对方用户信息
        }

        // 3. 添加用户到房间的 Redis 集合中
        {
            // 添加用户到房间成员列表
            let key = VideoRoomsCacheKeyBuilder::build(room_id);
            let mut conn = self.app_state.redis().await?;
            let _: () = conn.sadd(&key.key, &uid.to_string()).await?;

            // 添加房间到用户房间列表
            let key = UserRoomsCacheKeyBuilder::build(uid);
            let _: () = conn.sadd(&key.key, &room_id.to_string()).await?;
        }

        // 4. 刷新房间活跃时间
        // TODO: 调用 RoomTimeoutService.refresh_room_activity

        // 5. 通知房间内其他用户
        let push_uids = self
            .notify_room_members(room_id, uid, WsMsgTypeEnum::JoinVideo.as_i32(), &resp)
            .await?;

        info!(
            "用户加入房间: uid={}, room={}, type={}",
            uid,
            room_id,
            if room_type == 1 { "群聊" } else { "私聊" }
        );

        Ok(push_uids)
    }

    /// 用户离开视频房间
    pub async fn leave_room(&self, uid: UserId, room_id: RoomId) -> anyhow::Result<()> {
        if self.room_metadata_service.is_room_closed(room_id).await? {
            return Ok(()); // 房间已关闭，无需操作
        }

        // 1. 检查用户是否在房间中
        if !self.is_user_in_room(uid, room_id).await? {
            return Ok(());
        }

        // 2. 从用户房间列表中移除
        {
            let key = UserRoomsCacheKeyBuilder::build(uid);
            let mut conn = self.app_state.redis().await?;
            let _: () = conn.srem(&key.key, &room_id.to_string()).await?;
        }

        // 3. 从房间用户列表中移除
        {
            let key = VideoRoomsCacheKeyBuilder::build(room_id);
            let mut conn = self.app_state.redis().await?;
            let _: () = conn.srem(&key.key, &uid.to_string()).await?;
        }

        // 4. 通知房间内其他用户
        let resp = UserJoinRoomVO::new(uid as u64, room_id as u64);
        self.notify_room_members(room_id, uid, WsMsgTypeEnum::LeaveVideo.as_i32(), &resp)
            .await?;

        // 5. 如果房间为空，触发清理
        if self.get_room_members(room_id).await?.is_empty() {
            if !self.room_metadata_service.is_room_closed(room_id).await? {
                // 延迟 60 秒清理（等待用户可能重新加入）
                if let Some(rts) = self.get_room_timeout_service() {
                    rts.schedule_room_cleanup(room_id, 60).await?;
                } else {
                    warn!("RoomTimeoutService 未注入，无法调度房间清理: room_id={}", room_id);
                }
            }
        }

        Ok(())
    }

    /// 转发视频信令
    pub async fn forward_signal(
        &self,
        sender_uid: UserId,
        room_id: RoomId,
        signal: String,
        signal_type: String,
    ) -> anyhow::Result<()> {
        // 1. 获取房间内其他成员
        let mut uid_list = self.get_user_list(room_id).await?;
        uid_list.retain(|&x| x != sender_uid);

        if uid_list.is_empty() {
            return Ok(());
        }

        // 2. 构造信令消息
        let signal_vo = VideoSignalVO::new(sender_uid as u64, room_id as u64, signal_type, signal);
        let resp = WsBaseResp::from_data(WsMsgTypeEnum::WebrtcSignal.as_i32(), signal_vo)?;

        // 3. 批量推送
        let uid_list_u64: Vec<u64> = uid_list.iter().map(|&id| id as u64).collect();
        self.push_service
            .send_push_msg(resp, uid_list_u64, sender_uid as u64)
            .await?;

        Ok(())
    }

    /// 获取房间内所有人员 ID
    pub async fn get_user_list(&self, room_id: RoomId) -> anyhow::Result<Vec<UserId>> {
        // TODO: 实现 OnlineService.get_group_members
        // 暂时返回房间成员
        self.get_room_members(room_id).await
    }

    /// 转发媒体控制信号
    pub async fn forward_control_signal(
        &self,
        sender_uid: UserId,
        room_id: RoomId,
        control_resp: WsBaseResp,
    ) -> anyhow::Result<()> {
        // 1. 获取房间内其他成员
        let mut members = self.get_room_members(room_id).await?;
        members.retain(|&x| x != sender_uid);

        if members.is_empty() {
            return Ok(());
        }

        // 2. 批量推送
        let members_u64: Vec<u64> = members.iter().map(|&id| id as u64).collect();
        self.push_service
            .send_push_msg(control_resp, members_u64, sender_uid as u64)
            .await?;

        Ok(())
    }

    /// 创建群视频房间
    pub async fn create_group_room(
        &self,
        room_id: RoomId,
        creator_uid: UserId,
    ) -> anyhow::Result<RoomId> {
        let room = self.get_room_metadata(room_id).await?;
        if room.is_none() {
            return Err(anyhow::anyhow!("房间不存在"));
        }

        // 初始化房间
        {
            let key = VideoRoomsCacheKeyBuilder::build(room_id);
            let mut conn = self.app_state.redis().await?;
            let _: () = conn.sadd(&key.key, &creator_uid.to_string()).await?;
        }
        {
            let key = UserRoomsCacheKeyBuilder::build(creator_uid);
            let mut conn = self.app_state.redis().await?;
            let _: () = conn.sadd(&key.key, &room_id.to_string()).await?;
        }

        // 设置房间元数据
        self.room_metadata_service
            .set_room_start_time(room_id)
            .await?;
        self.room_metadata_service
            .set_room_type(room_id, 1)
            .await?;
        self.room_metadata_service
            .set_room_creator(room_id, creator_uid)
            .await?;
        self.room_metadata_service
            .add_room_admin(room_id, creator_uid)
            .await?;

        Ok(room_id)
    }

    /// 清理房间数据
    pub async fn clean_room_data(&self, room_id: RoomId) -> anyhow::Result<()> {
        // 1. 获取房间所有成员
        let members = self.get_room_members(room_id).await?;

        // 2. 从所有成员的房间列表中移除该房间
        {
            for uid in &members {
                let key = UserRoomsCacheKeyBuilder::build(*uid);
                let mut conn = self.app_state.redis().await?;
                let _: () = conn.srem(&key.key, &room_id.to_string()).await?;
            }
        }

        // 3. 删除房间成员集合
        {
            let key = VideoRoomsCacheKeyBuilder::build(room_id);
            let mut conn = self.app_state.redis().await?;
            let _: () = conn.del(&key.key).await?;
        }

        Ok(())
    }

    /// 获取用户加入的所有视频房间
    pub async fn get_user_rooms(&self, uid: UserId) -> anyhow::Result<HashSet<RoomId>> {
        let key = UserRoomsCacheKeyBuilder::build(uid);
        let mut conn = self.app_state.redis().await?;
        let members: Vec<String> = conn.smembers(&key.key).await?;
        let rooms: HashSet<RoomId> = members
            .into_iter()
            .filter_map(|s| s.parse::<RoomId>().ok())
            .collect();
        Ok(rooms)
    }

    /// 获取视频房间内所有成员
    pub async fn get_room_members(&self, room_id: RoomId) -> anyhow::Result<Vec<UserId>> {
        let key = VideoRoomsCacheKeyBuilder::build(room_id);
        let mut conn = self.app_state.redis().await?;
        let members: Vec<String> = conn.smembers(&key.key).await?;
        let mut uids: Vec<UserId> = members
            .into_iter()
            .filter_map(|s| s.parse::<UserId>().ok())
            .collect();
        uids.sort();
        uids.dedup();
        Ok(uids)
    }

    /// 检查用户是否在房间中
    pub async fn is_user_in_room(&self, uid: UserId, room_id: RoomId) -> anyhow::Result<bool> {
        let key = VideoRoomsCacheKeyBuilder::build(room_id);
        let mut conn = self.app_state.redis().await?;
        Ok(conn.sismember(&key.key, &uid.to_string()).await?)
    }

    /// 通知房间内其他成员
    async fn notify_room_members<T: serde::Serialize>(
        &self,
        room_id: RoomId,
        exclude_uid: UserId,
        resp_type: i32,
        data: &T,
    ) -> anyhow::Result<Vec<UserId>> {
        // 1. 获取房间内所有成员
        let uid_list = self.get_user_list(room_id).await?;
        if uid_list.is_empty() {
            return Ok(Vec::new());
        }

        // 2. 排除当前用户
        let push_uids: Vec<UserId> = uid_list.into_iter().filter(|&x| x != exclude_uid).collect();

        if push_uids.is_empty() {
            return Ok(Vec::new());
        }

        // 3. 构造通知消息
        let resp = WsBaseResp::from_data(resp_type, data)?;

        // 4. 批量推送
        let push_uids_u64: Vec<u64> = push_uids.iter().map(|&id| id as u64).collect();
        self.push_service
            .send_push_msg(resp, push_uids_u64, exclude_uid as u64)
            .await?;

        Ok(push_uids)
    }

    /// 检查用户是否是房间创建者或群管理员
    pub async fn is_room_admin(&self, uid: UserId, room_id: RoomId) -> anyhow::Result<bool> {
        self.room_metadata_service.is_room_admin(room_id, uid).await
    }

    /// 设置全体静音状态
    pub async fn set_all_muted(&self, room_id: RoomId, muted: bool) -> anyhow::Result<()> {
        self.room_metadata_service
            .set_all_muted(room_id, muted)
            .await
    }

    /// 设置屏幕共享状态
    pub async fn set_screen_sharing(
        &self,
        room_id: RoomId,
        user_id: UserId,
        sharing: bool,
    ) -> anyhow::Result<()> {
        self.room_metadata_service
            .set_screen_sharing(room_id, user_id, sharing)
            .await
    }

    /// 保存网络质量数据
    pub async fn save_network_quality(
        &self,
        uid: UserId,
        room_id: RoomId,
        quality: f64,
    ) -> anyhow::Result<()> {
        info!(
            "存储网络质量数据: uid={}, room={}, quality={}",
            uid, room_id, quality
        );
        // TODO: 存储到数据库或缓存
        Ok(())
    }

    /// 获取房间管理员列表
    pub async fn get_room_admins(&self, room_id: RoomId) -> anyhow::Result<Vec<UserId>> {
        let creator = self.room_metadata_service.get_room_creator(room_id).await?;
        Ok(creator.map(|c| vec![c]).unwrap_or_default())
    }
}

use crate::enums::WsMsgTypeEnum;
/// 房间超时管理服务
///
/// 功能：
/// 1. 设置和管理房间超时任务
/// 2. 刷新房间活跃时间
/// 3. 清理空闲房间
/// 4. 处理呼叫超时
use crate::model::vo::{call_timeout_vo::CallTimeoutVO, room_closed_vo::RoomClosedVO};
use crate::model::ws_base_resp::WsBaseResp;
use crate::service::{
    push_service::PushService, room_metadata_service::RoomMetadataService,
    video_chat_service::VideoChatService,
};
use crate::types::{RoomId, UserId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::warn;

/// 房间超时任务
struct TimeoutTask {
    cancel_handle: tokio::task::JoinHandle<()>,
}

/// 房间超时管理服务
pub struct RoomTimeoutService {
    video_service: Arc<VideoChatService>,
    push_service: Arc<PushService>,
    room_metadata_service: Arc<RoomMetadataService>,
    app_state: Arc<fbc_starter::AppState>,
    /// 房间ID -> 超时任务
    timeout_tasks: Arc<RwLock<HashMap<RoomId, TimeoutTask>>>,
}

impl RoomTimeoutService {
    /// 创建新的房间超时管理服务
    pub fn new(
        video_service: Arc<VideoChatService>,
        push_service: Arc<PushService>,
        room_metadata_service: Arc<RoomMetadataService>,
        app_state: Arc<fbc_starter::AppState>,
    ) -> Self {
        Self {
            video_service,
            push_service,
            room_metadata_service,
            app_state,
            timeout_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取房间接通电话时间
    pub async fn get_room_start_time(&self, room_id: RoomId) -> anyhow::Result<Option<i64>> {
        self.room_metadata_service
            .get_room_start_time(room_id)
            .await
    }

    /// 检查房间是否已关闭
    pub async fn is_close(&self, room_id: RoomId) -> anyhow::Result<bool> {
        self.room_metadata_service.is_room_closed(room_id).await
    }

    /// 设置房间超时、无成员时自动清理
    pub async fn schedule_room_cleanup(
        &self,
        room_id: RoomId,
        timeout_seconds: u64,
    ) -> anyhow::Result<()> {
        // 取消现有任务
        self.cancel_timeout_task(room_id).await;

        let video_service = self.video_service.clone();
        let room_metadata_service = self.room_metadata_service.clone();
        let timeout_tasks = self.timeout_tasks.clone();

        // 创建新任务
        let cancel_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(timeout_seconds)).await;

            // 二次状态校验、60秒延迟内用户重新加入
            if !room_metadata_service
                .is_room_closed(room_id)
                .await
                .unwrap_or(true)
                && video_service
                    .get_room_members(room_id)
                    .await
                    .unwrap_or_default()
                    .is_empty()
            {
                // TODO: 调用 clean_room
                warn!("房间超时清理: room_id={}", room_id);
            }

            // 移除任务
            timeout_tasks.write().await.remove(&room_id);
        });

        let task = TimeoutTask { cancel_handle };
        self.timeout_tasks.write().await.insert(room_id, task);

        Ok(())
    }

    /// 刷新房间活跃时间，5分钟无活动自动清理
    pub async fn refresh_room_activity(&self, room_id: RoomId) -> anyhow::Result<()> {
        self.schedule_room_cleanup(room_id, 300).await
    }

    /// 取消房间超时任务
    pub async fn cancel_timeout_task(&self, room_id: RoomId) {
        if let Some(task) = self.timeout_tasks.write().await.remove(&room_id) {
            task.cancel_handle.abort();
        }
    }

    /// 注入房间元数据
    pub async fn set_room_meta(
        &self,
        room: crate::model::entity::Room,
        uid: UserId,
        is_video: bool,
    ) -> anyhow::Result<()> {
        let room_id = room.id;

        self.room_metadata_service.open_room(room_id).await?;
        self.room_metadata_service
            .set_tenant_id(room_id, room.tenant_id)
            .await?;
        self.room_metadata_service
            .set_room_medium_type(room_id, is_video)
            .await?;
        self.room_metadata_service
            .set_room_type(room_id, room.room_type)
            .await?;
        self.room_metadata_service
            .set_room_creator(room_id, uid)
            .await?;
        self.room_metadata_service
            .add_room_admin(room_id, uid)
            .await?;

        // 群聊时
        if room.room_type == 1 && is_video {
            // TODO: 发送开始消息
            // self.send_start_msg(uid, room_id, "ONGOING").await?;
        }

        Ok(())
    }

    /// 初始化房间接通时间
    pub async fn set_room_start_time(&self, room_id: RoomId) -> anyhow::Result<()> {
        self.room_metadata_service
            .set_room_start_time(room_id)
            .await
    }

    /// 清理房间资源
    pub async fn clean_room(
        &self,
        room_id: RoomId,
        uid: Option<UserId>,
        reason: String,
    ) -> anyhow::Result<()> {
        // 1. 检查房间是否已关闭
        if self.room_metadata_service.is_room_closed(room_id).await? {
            return Ok(());
        }

        // 2. 取消所有相关超时任务
        self.cancel_timeout_task(room_id).await;

        // 3. 发送音视频消息到房间
        // TODO: self.send_msg(room_id, uid, reason).await?;

        // 4. 标记关闭
        self.room_metadata_service.mark_room_closed(room_id).await?;

        // 5. 获取房间所有成员
        let members = self.video_service.get_room_members(room_id).await?;

        // 6. 清理房间数据
        self.video_service.clean_room_data(room_id).await?;

        // 7. 发送房间关闭通知
        if !members.is_empty() {
            let closed_vo = RoomClosedVO {
                room_id: room_id,
                reason,
            };
            let resp = WsBaseResp::from_data(WsMsgTypeEnum::CloseRoom.as_i32(), closed_vo)?;

            let members_u64: Vec<u64> = members.iter().map(|&id| id as u64).collect();
            self.push_service
                .send_push_msg(resp, members_u64, 0)
                .await?;
        }

        Ok(())
    }

    /// 设置呼叫超时（30秒无应答）
    pub async fn schedule_call_timeout(
        &self,
        caller: UserId,
        receiver: UserId,
        room_id: RoomId,
    ) -> anyhow::Result<()> {
        // 取消现有任务
        self.cancel_timeout_task(room_id).await;

        let room_metadata_service = self.room_metadata_service.clone();
        let push_service = self.push_service.clone();
        let video_service = self.video_service.clone();
        let timeout_tasks = self.timeout_tasks.clone();

        let cancel_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await;

            // 检查房间是否已关闭
            if room_metadata_service
                .is_room_closed(room_id)
                .await
                .unwrap_or(true)
            {
                return;
            }

            // 通知主叫方呼叫超时
            let timeout_vo = CallTimeoutVO {
                target_uid: receiver,
            };
            let resp = WsBaseResp::from_data(WsMsgTypeEnum::Timeout.as_i32(), timeout_vo).unwrap();
            let _ = push_service
                .send_push_msg_single(resp, caller as u64, receiver as u64)
                .await;

            // 清理房间、通知超时
            // TODO: clean_room(room_id, None, "TIMEOUT".to_string()).await;
            warn!(
                "呼叫超时: caller={}, receiver={}, room_id={}",
                caller, receiver, room_id
            );

            // 移除任务
            timeout_tasks.write().await.remove(&room_id);
        });

        let task = TimeoutTask { cancel_handle };
        self.timeout_tasks.write().await.insert(room_id, task);

        Ok(())
    }
}

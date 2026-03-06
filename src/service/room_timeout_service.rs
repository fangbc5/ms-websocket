use crate::enums::WsMsgTypeEnum;
/// 房间超时管理服务
///
/// 功能：
/// 1. 设置和管理房间超时任务
/// 2. 刷新房间活跃时间
/// 3. 清理空闲房间
/// 4. 处理呼叫超时
use crate::model::dto::CallEndReq;
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
use tracing::{error, info};

/// Kafka topic: 音视频消息发送到 IM 服务
const FRONTEND_MSG_INPUT_TOPIC: &str = "frontend_msg_input_topic";

/// 房间超时任务
struct TimeoutTask {
    cancel_handle: tokio::task::JoinHandle<()>,
}

/// 房间超时管理服务
///
/// 所有字段均为 Arc，Clone 仅克隆引用计数
#[derive(Clone)]
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

        // Clone self (cheap: all fields are Arc)
        let self_clone = self.clone();

        // 创建新任务
        let cancel_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(timeout_seconds)).await;

            // 二次状态校验：60秒延迟内用户重新加入则不清理
            let is_closed = self_clone
                .room_metadata_service
                .is_room_closed(room_id)
                .await
                .unwrap_or(true);

            let members_empty = self_clone
                .video_service
                .get_room_members(room_id)
                .await
                .unwrap_or_default()
                .is_empty();

            if !is_closed && members_empty {
                info!("房间超时清理: room_id={}", room_id);
                if let Err(e) = self_clone
                    .clean_room(room_id, None, "TIMEOUT".to_string())
                    .await
                {
                    error!("房间超时清理失败: room_id={}, error={}", room_id, e);
                }
            }

            // 移除任务
            self_clone.timeout_tasks.write().await.remove(&room_id);
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
            // 发送群聊通话开始消息到 IM 服务
            self.send_start_msg(uid, room_id, "ONGOING".to_string())
                .await?;
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

        // 3. 发送音视频消息到 IM 服务（通过 Kafka）
        self.send_msg(room_id, uid, reason.clone()).await?;

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

        // Clone self (cheap: all fields are Arc)
        let self_clone = self.clone();

        let cancel_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await;

            // 检查房间是否已关闭
            if self_clone
                .room_metadata_service
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
            let _ = self_clone
                .push_service
                .send_push_msg_single(resp, caller as u64, receiver as u64)
                .await;

            // 清理房间、通知超时
            if let Err(e) = self_clone
                .clean_room(room_id, None, "TIMEOUT".to_string())
                .await
            {
                error!(
                    "呼叫超时清理房间失败: caller={}, receiver={}, room_id={}, error={}",
                    caller, receiver, room_id, e
                );
            }

            // 移除任务
            self_clone.timeout_tasks.write().await.remove(&room_id);
        });

        let task = TimeoutTask { cancel_handle };
        self.timeout_tasks.write().await.insert(room_id, task);

        Ok(())
    }

    /// 发送音视频结束消息到 IM 服务
    ///
    /// 对应 Java: RoomTimeoutService.senMsg()
    /// 通过 Kafka 将 CallEndReq 发送到 frontend_msg_input_topic，
    /// 由 IM 服务的 FrontendMsgConsumer 消费并生成通话记录消息
    async fn send_msg(
        &self,
        room_id: RoomId,
        uid: Option<UserId>,
        reason: String,
    ) -> anyhow::Result<()> {
        // 获取房间元数据
        let start_time = self
            .room_metadata_service
            .get_room_start_time(room_id)
            .await?;
        let creator = self
            .room_metadata_service
            .get_room_creator(room_id)
            .await?;
        let tenant_id = self.room_metadata_service.get_tenant_id(room_id).await?;
        let room_type = self.room_metadata_service.get_room_type(room_id).await?;
        let medium_type = self
            .room_metadata_service
            .get_room_medium_type(room_id)
            .await?;

        // 构造 CallEndReq
        let is_group = room_type.map(|t| t == 1).unwrap_or(false);
        let call_end = CallEndReq::new_end(
            uid,
            room_id,
            tenant_id,
            is_group,
            medium_type,
            creator,
            start_time,
            reason,
        );

        // 发送到 Kafka
        let producer = self.app_state.message_producer()?;
        let message = fbc_starter::Message::new(
            FRONTEND_MSG_INPUT_TOPIC,
            uid.map(|u| u.to_string()).unwrap_or_default(),
            serde_json::to_value(&call_end)?,
        );
        producer
            .publish(FRONTEND_MSG_INPUT_TOPIC, message)
            .await
            .map_err(|e| anyhow::anyhow!("发送音视频消息到 Kafka 失败: {}", e))?;

        info!(
            "发送通话结束消息: room_id={}, uid={:?}, reason={}",
            room_id, uid, call_end.state
        );

        Ok(())
    }

    /// 群聊时立即发送群聊通话开始消息
    ///
    /// 对应 Java: RoomTimeoutService.senStartMsg()
    async fn send_start_msg(
        &self,
        uid: UserId,
        room_id: RoomId,
        reason: String,
    ) -> anyhow::Result<()> {
        let creator = self
            .room_metadata_service
            .get_room_creator(room_id)
            .await?;
        let tenant_id = self.room_metadata_service.get_tenant_id(room_id).await?;

        let call_start = CallEndReq::new_start(uid, creator, room_id, tenant_id, reason);

        // 发送到 Kafka
        let producer = self.app_state.message_producer()?;
        let message = fbc_starter::Message::new(
            FRONTEND_MSG_INPUT_TOPIC,
            uid.to_string(),
            serde_json::to_value(&call_start)?,
        );
        producer
            .publish(FRONTEND_MSG_INPUT_TOPIC, message)
            .await
            .map_err(|e| anyhow::anyhow!("发送群聊开始消息到 Kafka 失败: {}", e))?;

        info!(
            "发送群聊通话开始消息: room_id={}, uid={}",
            room_id, uid
        );

        Ok(())
    }
}

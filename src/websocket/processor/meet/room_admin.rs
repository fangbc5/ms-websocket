use crate::enums::WsMsgTypeEnum;
/// 房间管理处理器

use crate::model::vo::{all_muted_vo::AllMutedVO, user_kicked_vo::UserKickedVO};
use crate::model::ws_base_resp::{WsBaseReq, WsBaseResp};
use crate::service::{PushService, RoomTimeoutService, VideoChatService};
use crate::types::{ClientId, RoomId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};

/// 关闭房间请求
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CloseRoomReq {
    /// 房间 ID
    pub room_id: RoomId,
}

/// 踢出用户请求
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KickUserReq {
    /// 房间 ID
    pub room_id: RoomId,
    /// 目标用户 ID
    pub target_uid: UserId,
    /// 原因
    pub reason: Option<String>,
}

/// 全体静音请求
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MuteAllReq {
    /// 房间 ID
    pub room_id: RoomId,
    /// 是否静音
    pub muted: bool,
}

/// 房间管理处理器
///
/// 功能：
/// 1. 关闭房间
/// 2. 踢出用户
/// 3. 全体静音
pub struct RoomAdminProcessor {
    video_service: Arc<VideoChatService>,
    push_service: Arc<PushService>,
    room_timeout_service: Arc<RoomTimeoutService>,
}

impl RoomAdminProcessor {
    pub fn new(
        video_service: Arc<VideoChatService>,
        push_service: Arc<PushService>,
        room_timeout_service: Arc<RoomTimeoutService>,
    ) -> Self {
        Self {
            video_service,
            push_service,
            room_timeout_service,
        }
    }
}

#[async_trait::async_trait]
impl MessageProcessor for RoomAdminProcessor {
    fn supports(&self, req: &WsBaseReq) -> bool {
        WsMsgTypeEnum::CloseRoom.eq(req.r#type)
            || WsMsgTypeEnum::KickUser.eq(req.r#type)
            || WsMsgTypeEnum::MediaMuteAll.eq(req.r#type)
    }

    async fn process(
        &self,
        _session: &Arc<Session>,
        _session_id: &SessionId,
        uid: UserId,
        _client_id: &ClientId,
        req: WsBaseReq,
    ) {
        match WsMsgTypeEnum::from(req.r#type) {
            Some(WsMsgTypeEnum::CloseRoom) => self.handle_close_room(uid, &req).await,
            Some(WsMsgTypeEnum::KickUser) => self.handle_kick_user(uid, &req).await,
            Some(WsMsgTypeEnum::MediaMuteAll) => self.handle_mute_all(uid, &req).await,
            _ => warn!("未知的房间管理消息类型: {}", req.r#type),
        }
    }
}

impl RoomAdminProcessor {
    /// 处理关闭房间
    async fn handle_close_room(&self, operator_id: UserId, req: &WsBaseReq) {
        let close_req: CloseRoomReq = match serde_json::from_value(req.data.clone()) {
            Ok(req) => req,
            Err(e) => {
                warn!("解析关闭房间请求失败: {}", e);
                return;
            }
        };

        info!(
            "收到关闭房间请求: operator={}, room_id={}",
            operator_id, close_req.room_id
        );

        // 1. 验证操作者权限（需是房间创建者或管理员）
        match self
            .video_service
            .is_room_admin(operator_id, close_req.room_id)
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                warn!(
                    "用户无权限关闭房间: uid={}, room={}",
                    operator_id, close_req.room_id
                );
                return;
            }
            Err(e) => {
                error!("验证权限失败: {}", e);
                return;
            }
        }

        // 2. 关闭房间
        if let Err(e) = self
            .room_timeout_service
            .clean_room(
                close_req.room_id,
                Some(operator_id),
                "MANAGER_CLOSE".to_string(),
            )
            .await
        {
            error!("关闭房间失败: room_id={}, error={}", close_req.room_id, e);
        }
    }

    /// 处理踢出用户
    async fn handle_kick_user(&self, operator_id: UserId, req: &WsBaseReq) {
        let kick_req: KickUserReq = match serde_json::from_value(req.data.clone()) {
            Ok(req) => req,
            Err(e) => {
                warn!("解析踢出用户请求失败: {}", e);
                return;
            }
        };

        info!(
            "收到踢出用户请求: operator={}, room_id={}, target={}",
            operator_id, kick_req.room_id, kick_req.target_uid
        );

        // 1. 验证操作者权限
        match self
            .video_service
            .is_room_admin(operator_id, kick_req.room_id)
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                warn!(
                    "用户无权限踢出成员: uid={}, room={}",
                    operator_id, kick_req.room_id
                );
                return;
            }
            Err(e) => {
                error!("验证权限失败: {}", e);
                return;
            }
        }

        // 2. 强制用户离开房间
        if let Err(e) = self
            .video_service
            .leave_room(kick_req.target_uid, kick_req.room_id)
            .await
        {
            error!("踢出用户失败: {}", e);
            return;
        }

        // 3. 通知被踢用户和其他成员
        let reason = kick_req.reason.unwrap_or_default();
        self.notify_user_kicked(
            kick_req.room_id,
            kick_req.target_uid,
            operator_id,
            reason,
        )
        .await;
    }

    /// 处理全体静音
    async fn handle_mute_all(&self, operator_id: UserId, req: &WsBaseReq) {
        let mute_req: MuteAllReq = match serde_json::from_value(req.data.clone()) {
            Ok(req) => req,
            Err(e) => {
                warn!("解析全体静音请求失败: {}", e);
                return;
            }
        };

        info!(
            "收到全体静音请求: operator={}, room_id={}, muted={}",
            operator_id, mute_req.room_id, mute_req.muted
        );

        // 1. 验证操作者权限
        match self
            .video_service
            .is_room_admin(operator_id, mute_req.room_id)
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                warn!(
                    "用户无权限全体静音: uid={}, room={}",
                    operator_id, mute_req.room_id
                );
                return;
            }
            Err(e) => {
                error!("验证权限失败: {}", e);
                return;
            }
        }

        // 2. 设置全体静音状态
        if let Err(e) = self
            .video_service
            .set_all_muted(mute_req.room_id, mute_req.muted)
            .await
        {
            error!("设置全体静音失败: {}", e);
            return;
        }

        // 3. 通知房间成员
        self.notify_all_muted(mute_req.room_id, mute_req.muted, operator_id)
            .await;
    }

    /// 通知被踢用户和其他成员
    async fn notify_user_kicked(
        &self,
        room_id: RoomId,
        target_uid: UserId,
        operator_id: UserId,
        reason: String,
    ) {
        let kicked_vo = UserKickedVO {
            room_id,
            kicked_uid: target_uid,
            operator_id,
            reason,
        };

        let resp = match WsBaseResp::from_data(WsMsgTypeEnum::UserKicked.as_i32(), &kicked_vo) {
            Ok(resp) => resp,
            Err(e) => {
                error!("序列化踢出通知失败: {}", e);
                return;
            }
        };

        // 1. 通知被踢用户
        let _ = self
            .push_service
            .send_push_msg(resp.clone(), vec![target_uid as u64], operator_id as u64)
            .await;

        // 2. 通知其他成员
        let mut members = self
            .video_service
            .get_room_members(room_id)
            .await
            .unwrap_or_default();
        members.retain(|&uid| uid != target_uid);

        if !members.is_empty() {
            let members_u64: Vec<u64> = members.iter().map(|&id| id as u64).collect();
            let _ = self
                .push_service
                .send_push_msg(resp, members_u64, operator_id as u64)
                .await;
        }
    }

    /// 通知房间成员全体静音状态变更
    async fn notify_all_muted(&self, room_id: RoomId, muted: bool, operator_id: UserId) {
        let muted_vo = AllMutedVO {
            room_id,
            muted,
            operator_id,
        };

        let resp = match WsBaseResp::from_data(WsMsgTypeEnum::AllMuted.as_i32(), muted_vo) {
            Ok(resp) => resp,
            Err(e) => {
                error!("序列化全体静音通知失败: {}", e);
                return;
            }
        };

        let members = self
            .video_service
            .get_room_members(room_id)
            .await
            .unwrap_or_default();

        if !members.is_empty() {
            let members_u64: Vec<u64> = members.iter().map(|&id| id as u64).collect();
            let _ = self
                .push_service
                .send_push_msg(resp, members_u64, operator_id as u64)
                .await;
        }
    }
}


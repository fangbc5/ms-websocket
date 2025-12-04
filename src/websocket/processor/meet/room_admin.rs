use crate::enums::WsMsgTypeEnum;
/// 房间管理处理器

use crate::model::ws_base_resp::WsBaseReq;
use crate::types::{ClientId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// 关闭房间请求
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CloseRoomReq {
    /// 房间 ID
    pub room_id: u64,
}

/// 踢出用户请求
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KickUserReq {
    /// 房间 ID
    pub room_id: u64,
    /// 目标用户 ID
    pub target_uid: u64,
    /// 原因
    pub reason: Option<String>,
}

/// 全体静音请求
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MuteAllReq {
    /// 房间 ID
    pub room_id: u64,
    /// 是否静音
    pub muted: bool,
}

/// 房间管理处理器
/// 
/// 功能：
/// 1. 关闭房间
/// 2. 踢出用户
/// 3. 全体静音
pub struct RoomAdminProcessor;

impl RoomAdminProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl MessageProcessor for RoomAdminProcessor {
    fn supports(&self, req: &WsBaseReq) -> bool {
        WsMsgTypeEnum::CloseRoom.eq(req.r#type) || WsMsgTypeEnum::KickUser.eq(req.r#type) || WsMsgTypeEnum::MediaMuteAll.eq(req.r#type)
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
            _ => tracing::warn!("未知的房间管理消息类型: {}", req.r#type),
        }
    }
}

impl RoomAdminProcessor {
    async fn handle_close_room(&self, operator_id: UserId, req: &WsBaseReq) {
        let close_req: CloseRoomReq = match serde_json::from_value(req.data.clone()) {
            Ok(req) => req,
            Err(e) => {
                tracing::warn!("解析关闭房间请求失败: {}", e);
                return;
            }
        };

        info!(
            "收到关闭房间请求: operator={}, room_id={}",
            operator_id, close_req.room_id
        );

        // TODO: 1. 验证操作者权限（需是房间创建者或管理员）
        // if !video_service.isRoomAdmin(operator_id, close_req.room_id) {
        //     return;
        // }

        // TODO: 2. 关闭房间
        // room_timeout_service.cleanRoom(close_req.room_id, operator_id, CallStatusEnum::MANAGER_CLOSE);
    }

    async fn handle_kick_user(&self, operator_id: UserId, req: &WsBaseReq) {
        let kick_req: KickUserReq = match serde_json::from_value(req.data.clone()) {
            Ok(req) => req,
            Err(e) => {
                tracing::warn!("解析踢出用户请求失败: {}", e);
                return;
            }
        };

        info!(
            "收到踢出用户请求: operator={}, room_id={}, target={}",
            operator_id, kick_req.room_id, kick_req.target_uid
        );

        // TODO: 1. 验证操作者权限
        // if !video_service.isRoomAdmin(operator_id, kick_req.room_id) {
        //     return;
        // }

        // TODO: 2. 强制用户离开房间
        // video_service.leaveRoom(kick_req.target_uid, kick_req.room_id);

        // TODO: 3. 通知被踢用户和其他成员
        // notify_user_kicked(kick_req.room_id, kick_req.target_uid, operator_id, kick_req.reason);
    }

    async fn handle_mute_all(&self, operator_id: UserId, req: &WsBaseReq) {
        let mute_req: MuteAllReq = match serde_json::from_value(req.data.clone()) {
            Ok(req) => req,
            Err(e) => {
                tracing::warn!("解析全体静音请求失败: {}", e);
                return;
            }
        };

        info!(
            "收到全体静音请求: operator={}, room_id={}, muted={}",
            operator_id, mute_req.room_id, mute_req.muted
        );

        // TODO: 1. 验证操作者权限
        // if !video_service.isRoomAdmin(operator_id, mute_req.room_id) {
        //     return;
        // }

        // TODO: 2. 设置全体静音状态
        // video_service.setAllMuted(mute_req.room_id, mute_req.muted);

        // TODO: 3. 通知房间成员
        // notify_all_muted(mute_req.room_id, mute_req.muted, operator_id);
    }
}

impl Default for RoomAdminProcessor {
    fn default() -> Self {
        Self::new()
    }
}


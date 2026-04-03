use crate::enums::{CallResponseStatus, WsMsgTypeEnum};
/// 视频呼叫处理器

use crate::model::vo::{
    call_accepted_vo::CallAcceptedVO,
    call_rejected_vo::CallRejectedVO,
    call_req_vo::CallReqVO,
    call_request_vo::CallRequestVO,
    call_response_vo::CallResponseVO,
};
use crate::model::ws_base_resp::{WsBaseReq, WsBaseResp};
use crate::service::{PushService, RoomTimeoutService, VideoChatService};
use crate::types::{ClientId, RoomId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
use std::sync::Arc;
use tracing::{error, info, warn};

/// 视频呼叫处理器
///
/// 功能：
/// 1. 处理视频呼叫请求
/// 2. 处理呼叫响应（接受/拒绝）
/// 3. 管理呼叫超时
/// 4. 通知呼叫结果
pub struct VideoCallProcessor {
    video_service: Arc<VideoChatService>,
    push_service: Arc<PushService>,
    room_timeout_service: Arc<RoomTimeoutService>,
}

impl VideoCallProcessor {
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
impl MessageProcessor for VideoCallProcessor {
    fn supports(&self, req: &WsBaseReq) -> bool {
        WsMsgTypeEnum::VideoCallRequest.eq(req.r#type)
            || WsMsgTypeEnum::VideoCallResponse.eq(req.r#type)
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
            Some(WsMsgTypeEnum::VideoCallRequest) => self.handle_call_request(uid, &req).await,
            Some(WsMsgTypeEnum::VideoCallResponse) => self.handle_call_response(uid, &req).await,
            _ => warn!("未知的视频呼叫消息类型: {}", req.r#type),
        }
    }
}

impl VideoCallProcessor {
    /// 处理发起通话请求
    async fn handle_call_request(&self, caller_uid: UserId, req: &WsBaseReq) {
        let call_request: CallRequestVO = match serde_json::from_value(req.data.clone()) {
            Ok(req) => req,
            Err(e) => {
                warn!("解析视频呼叫请求失败: {}", e);
                return;
            }
        };

        info!(
            "收到视频呼叫请求: caller={}, target={}, room_id={}, is_video={}",
            caller_uid, call_request.target_uid, call_request.room_id, call_request.is_video
        );

        // 1. 获取房间元数据
        let room = match self
            .video_service
            .get_room_metadata(call_request.room_id)
            .await
        {
            Ok(Some(room)) => room,
            Ok(None) => {
                error!("房间不存在: room_id={}", call_request.room_id);
                return;
            }
            Err(e) => {
                error!("获取房间元数据失败: {}", e);
                return;
            }
        };

        // 2. 主叫方加入房间
        let push_ids = match self.video_service.join_room(caller_uid, room.clone()).await {
            Ok(ids) => ids,
            Err(e) => {
                error!("主叫方加入房间失败: {}", e);
                return;
            }
        };

        // 3. 注入房间元数据
        if let Err(e) = self
            .room_timeout_service
            .set_room_meta(room.clone(), caller_uid, call_request.is_video)
            .await
        {
            error!("注入房间元数据失败: {}", e);
            return;
        }

        // 4. 发送呼叫请求给被叫方
        let call_req_vo = CallReqVO {
            room_id: call_request.room_id,
            caller_uid,
            target_uid: call_request.target_uid,
            is_video: call_request.is_video,
        };
        let resp = match WsBaseResp::from_data(
            WsMsgTypeEnum::VideoCallRequest.as_i32(),
            call_req_vo,
        ) {
            Ok(resp) => resp,
            Err(e) => {
                error!("序列化呼叫请求失败: {}", e);
                return;
            }
        };

        let target_uids = vec![call_request.target_uid];
        if let Err(e) = self
            .push_service
            .send_push_msg(resp, target_uids, caller_uid as u64)
            .await
        {
            error!("发送呼叫请求失败: {}", e);
        }

        // 5. 设置呼叫超时（30秒）
        if let Err(e) = self
            .room_timeout_service
            .schedule_call_timeout(caller_uid, call_request.target_uid, room.id)
            .await
        {
            error!("设置呼叫超时失败: {}", e);
        }
    }

    /// 处理通话响应（双方都可能调用此方法）
    async fn handle_call_response(&self, uid: UserId, req: &WsBaseReq) {
        let response: CallResponseVO = match serde_json::from_value(req.data.clone()) {
            Ok(resp) => resp,
            Err(e) => {
                warn!("解析视频呼叫响应失败: {}", e);
                return;
            }
        };

        let room_id = response.room_id;

        info!(
            "收到视频呼叫响应: uid={}, room_id={}, accepted={}",
            uid, room_id, response.accepted
        );

        match CallResponseStatus::of(response.accepted) {
            Some(CallResponseStatus::Accepted) => {
                // 1. 立即取消超时任务
                self.room_timeout_service
                    .cancel_timeout_task(room_id)
                    .await;

                // 2. 被叫方加入房间
                let room = match self.video_service.get_room_metadata(room_id).await {
                    Ok(Some(room)) => room,
                    _ => {
                        error!("接受呼叫时获取房间失败: room_id={}", room_id);
                        return;
                    }
                };
                if let Err(e) = self.video_service.join_room(uid, room).await {
                    error!("被叫方加入房间失败: {}", e);
                    return;
                }

                // 3. 通知双方呼叫已接受
                self.notify_call_accepted(response.caller_uid, uid, room_id)
                    .await;

                // 4. 设置房间接通时间
                if let Err(e) = self.room_timeout_service.set_room_start_time(room_id).await {
                    error!("设置房间接通时间失败: {}", e);
                }
            }
            Some(CallResponseStatus::Timeout)
            | Some(CallResponseStatus::Rejected)
            | Some(CallResponseStatus::Hangup) => {
                // 检查房间是否已关闭
                if self
                    .room_timeout_service
                    .is_close(room_id)
                    .await
                    .unwrap_or(true)
                {
                    return;
                }

                // 取消超时任务
                self.room_timeout_service
                    .cancel_timeout_task(room_id)
                    .await;

                // 通知对方呼叫被拒绝/超时/挂断
                let call_status = self
                    .notify_call_rejected(room_id, response.caller_uid, uid, response.accepted)
                    .await;

                // 清理房间
                if let Err(e) = self
                    .room_timeout_service
                    .clean_room(room_id, Some(uid), call_status)
                    .await
                {
                    error!("清理房间失败: room_id={}, error={}", room_id, e);
                }
            }
            None => {
                warn!("未知的呼叫响应状态: {}", response.accepted);
            }
        }
    }

    /// 通知双方呼叫已接通
    async fn notify_call_accepted(
        &self,
        caller_uid: UserId,
        responder_uid: UserId,
        room_id: RoomId,
    ) {
        let room_name = format!("call_{}", room_id);
        let livekit_url = self.video_service.livekit_ws_url().to_string();

        // 生成主叫方 Token
        let caller_token = match self.video_service.generate_livekit_token(caller_uid, &room_name) {
            Ok(token) => token,
            Err(e) => {
                error!("生成主叫方 LiveKit Token 失败: {}", e);
                return;
            }
        };

        // 生成被叫方 Token
        let responder_token = match self.video_service.generate_livekit_token(responder_uid, &room_name) {
            Ok(token) => token,
            Err(e) => {
                error!("生成被叫方 LiveKit Token 失败: {}", e);
                return;
            }
        };

        // 通知主叫方
        let resp_to_caller = CallAcceptedVO {
            accepted_by: responder_uid,
            room_id,
            token: caller_token,
            livekit_url: livekit_url.clone(),
        };
        if let Ok(resp) =
            WsBaseResp::from_data(WsMsgTypeEnum::CallAccepted.as_i32(), resp_to_caller)
        {
            let _ = self
                .push_service
                .send_push_msg(resp, vec![caller_uid as u64], responder_uid as u64)
                .await;
        }

        // 通知被叫方
        let resp_to_responder = CallAcceptedVO {
            accepted_by: caller_uid,
            room_id,
            token: responder_token,
            livekit_url,
        };
        if let Ok(resp) = WsBaseResp::from_data(
            WsMsgTypeEnum::CallAccepted.as_i32(),
            resp_to_responder,
        ) {
            let _ = self
                .push_service
                .send_push_msg(resp, vec![responder_uid as u64], responder_uid as u64)
                .await;
        }
    }

    /// 通知对方呼叫被拒绝/超时/挂断，并返回通话状态字符串
    async fn notify_call_rejected(
        &self,
        room_id: RoomId,
        opposite_uid: UserId,
        uid: UserId,
        accepted: i32,
    ) -> String {
        let (resp_type, call_status) = match CallResponseStatus::of(accepted) {
            Some(CallResponseStatus::Rejected) => {
                (WsMsgTypeEnum::CallRejected, "REJECTED".to_string())
            }
            Some(CallResponseStatus::Hangup) => {
                // 判断是取消还是挂断：有接通时间=挂断，无接通时间=取消
                let room_start_time = self
                    .room_timeout_service
                    .get_room_start_time(room_id)
                    .await
                    .unwrap_or(None);
                if room_start_time.is_none() {
                    (WsMsgTypeEnum::Cancel, "CANCEL".to_string())
                } else {
                    (WsMsgTypeEnum::Dropped, "DROPPED".to_string())
                }
            }
            _ => (WsMsgTypeEnum::Timeout, "TIMEOUT".to_string()),
        };

        let rejected_vo = CallRejectedVO { rejected_by: uid };
        if let Ok(resp) = WsBaseResp::from_data(resp_type.as_i32(), rejected_vo) {
            let _ = self
                .push_service
                .send_push_msg(resp, vec![opposite_uid as u64], opposite_uid as u64)
                .await;
        }

        call_status
    }
}


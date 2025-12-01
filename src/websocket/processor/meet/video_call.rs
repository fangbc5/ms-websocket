/// 视频呼叫处理器

use crate::model::vo::{
    call_request_vo::CallRequestVO, call_response_vo::CallResponseVO,
};
use crate::model::ws_base_resp::WsBaseReq;
use crate::types::{ClientId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
use std::sync::Arc;
use tracing::info;

/// 视频呼叫处理器
/// 
/// 功能：
/// 1. 处理视频呼叫请求
/// 2. 处理呼叫响应（接受/拒绝）
/// 3. 管理呼叫超时
/// 4. 通知呼叫结果
pub struct VideoCallProcessor;

impl VideoCallProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl MessageProcessor for VideoCallProcessor {
    fn supports(&self, req: &WsBaseReq) -> bool {
        req.r#type == "video_call_request"
            || req.r#type == "VIDEO_CALL_REQUEST"
            || req.r#type == "video_call_response"
            || req.r#type == "VIDEO_CALL_RESPONSE"
    }

    async fn process(
        &self,
        _session: &Arc<Session>,
        _session_id: &SessionId,
        uid: UserId,
        _client_id: &ClientId,
        req: WsBaseReq,
    ) {
        match req.r#type.as_str() {
            "video_call_request" | "VIDEO_CALL_REQUEST" => {
                self.handle_call_request(uid, &req).await;
            }
            "video_call_response" | "VIDEO_CALL_RESPONSE" => {
                self.handle_call_response(uid, &req).await;
            }
            _ => {
                tracing::warn!("未知的视频呼叫消息类型: {}", req.r#type);
            }
        }
    }
}

impl VideoCallProcessor {
    async fn handle_call_request(&self, caller_uid: UserId, req: &WsBaseReq) {
        let call_request: CallRequestVO = match serde_json::from_value(req.data.clone()) {
            Ok(req) => req,
            Err(e) => {
                tracing::warn!("解析视频呼叫请求失败: {}", e);
                return;
            }
        };

        info!(
            "收到视频呼叫请求: caller={}, target={}, room_id={}, is_video={}",
            caller_uid, call_request.target_uid, call_request.room_id, call_request.is_video
        );

        // TODO: 1. 获取房间元数据
        // let room = video_service.getRoomMetadata(call_request.room_id);

        // TODO: 2. 主叫方加入房间
        // let push_ids = video_service.joinRoom(caller_uid, room);

        // TODO: 3. 注入房间元数据
        // room_timeout_service.setRoomMeta(room, caller_uid, call_request.is_video);

        // TODO: 4. 发送呼叫请求给被叫方
        // push_service.sendPushMsg(resp, push_ids, caller_uid);

        // TODO: 5. 设置呼叫超时（30秒）
        // room_timeout_service.scheduleCallTimeout(caller_uid, call_request.target_uid, call_request.room_id);
    }

    async fn handle_call_response(&self, uid: UserId, req: &WsBaseReq) {
        let response: CallResponseVO = match serde_json::from_value(req.data.clone()) {
            Ok(resp) => resp,
            Err(e) => {
                tracing::warn!("解析视频呼叫响应失败: {}", e);
                return;
            }
        };

        info!(
            "收到视频呼叫响应: uid={}, room_id={}, accepted={}",
            uid, response.room_id, response.accepted
        );

        // TODO: 根据响应状态处理
        // match CallResponseStatus::of(response.accepted) {
        //     ACCEPTED => {
        //         // 取消超时任务
        //         // 加入房间
        //         // 通知双方呼叫已接受
        //         // 设置房间接通时间
        //     }
        //     TIMEOUT | REJECTED | HANGUP => {
        //         // 通知主叫方呼叫被拒绝/超时
        //         // 清理房间
        //     }
        // }
    }
}

impl Default for VideoCallProcessor {
    fn default() -> Self {
        Self::new()
    }
}


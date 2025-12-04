use crate::enums::WsMsgTypeEnum;
/// 视频信令处理器

use crate::model::vo::heartbeat_req::HeartbeatReq;
use crate::model::ws_base_resp::WsBaseReq;
use crate::types::{ClientId, RoomId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// 视频信令请求
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VideoSignalReq {
    /// 目标用户 ID
    pub target_uid: Option<UserId>,
    /// 房间 ID
    pub room_id: RoomId,
    /// WebRTC 信令内容
    pub signal: String,
    /// 信令类型 (offer/answer/candidate)
    pub signal_type: String,
}

/// 视频信令处理器
/// 
/// 功能：
/// 1. 处理用户加入/离开视频房间
/// 2. 转发点对点和群组视频信令
/// 3. 处理视频心跳保活
pub struct VideoProcessor;

impl VideoProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl MessageProcessor for VideoProcessor {
    fn supports(&self, req: &WsBaseReq) -> bool {
        WsMsgTypeEnum::WebrtcSignal.eq(req.r#type) || WsMsgTypeEnum::VideoHeartbeat.eq(req.r#type)
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
            Some(WsMsgTypeEnum::WebrtcSignal) => {
                // 处理 WebRTC 信令
                let signal_req: VideoSignalReq = match serde_json::from_value(req.data.clone()) {
                    Ok(req) => req,
                    Err(e) => {
                        tracing::warn!("解析视频信令失败: {}", e);
                        return;
                    }
                };

                info!(
                    "收到视频信令: uid={}, room_id={}, signal_type={}",
                    uid, signal_req.room_id, signal_req.signal_type
                );

                // TODO: 转发信令到房间成员
                // video_service.forwardSignal(uid, signal_req.room_id, signal_req.signal, signal_req.signal_type);
            }
            Some(WsMsgTypeEnum::VideoHeartbeat) => {
                // 处理视频心跳
                let heartbeat: HeartbeatReq = match serde_json::from_value(req.data.clone()) {
                    Ok(hb) => hb,
                    Err(e) => {
                        tracing::warn!("解析视频心跳失败: {}", e);
                        return;
                    }
                };

                info!(
                    "收到视频心跳: uid={}, room_id={}",
                    uid, heartbeat.room_id
                );

                // TODO: 刷新房间活跃时间
                // room_timeout_service.refreshRoomActivity(heartbeat.room_id);
            }
            _ => {
                tracing::warn!("未知的视频消息类型: {}", req.r#type);
            }
        }
    }
}

impl Default for VideoProcessor {
    fn default() -> Self {
        Self::new()
    }
}


use crate::enums::WsMsgTypeEnum;
/// 媒体控制处理器

use crate::model::vo::media_control_vo::MediaControlVO;
use crate::model::ws_base_resp::WsBaseReq;
use crate::types::{ClientId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
use std::sync::Arc;
use tracing::info;

/// 媒体控制处理器
/// 
/// 功能：
/// 1. 处理音频静音控制
/// 2. 处理视频摄像头开关控制
/// 3. 转发媒体控制指令给房间成员
pub struct MediaControlProcessor;

impl MediaControlProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl MessageProcessor for MediaControlProcessor {
    fn supports(&self, req: &WsBaseReq) -> bool {
        WsMsgTypeEnum::MediaMuteAudio.eq(req.r#type)
    }

    async fn process(
        &self,
        _session: &Arc<Session>,
        _session_id: &SessionId,
        uid: UserId,
        _client_id: &ClientId,
        req: WsBaseReq,
    ) {
        // 解析媒体控制 VO
        let control: MediaControlVO = match serde_json::from_value(req.data.clone()) {
            Ok(control) => control,
            Err(e) => {
                tracing::warn!("解析媒体控制消息失败: {}", e);
                return;
            }
        };

        info!(
            "收到媒体控制: uid={}, room_id={}, type={}",
            uid, control.room_id, req.r#type
        );

        // TODO: 转发媒体控制指令给房间内其他成员
        // video_service.forwardControlSignal(uid, control.room_id, resp);
    }
}

impl Default for MediaControlProcessor {
    fn default() -> Self {
        Self::new()
    }
}


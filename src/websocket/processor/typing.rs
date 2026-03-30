/// 正在输入状态处理器
///
/// 处理用户的 "正在输入" 信令，将其转发给目标用户
/// 纯 WS 层实现，不经过 ms-im，不持久化
use crate::enums::WsMsgTypeEnum;
use crate::model::ws_base_resp::{WsBaseReq, WsBaseResp};
use crate::service::PushService;
use crate::types::{ClientId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// 正在输入 DTO（前端发送）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TypingReqDto {
    /// 目标用户 ID（对方）
    target_uid: u64,
    /// 房间 ID
    room_id: u64,
}

/// 正在输入通知 VO（推送给对方）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TypingNotifyVo {
    /// 发送者用户 ID
    uid: u64,
    /// 房间 ID
    room_id: u64,
}

/// 正在输入处理器
pub struct TypingProcessor {
    push_service: Arc<PushService>,
}

impl TypingProcessor {
    pub fn new(push_service: Arc<PushService>) -> Self {
        Self { push_service }
    }
}

#[async_trait::async_trait]
impl MessageProcessor for TypingProcessor {
    fn supports(&self, req: &WsBaseReq) -> bool {
        WsMsgTypeEnum::Typing.eq(req.r#type)
    }

    async fn process(
        &self,
        _session: &Arc<Session>,
        _session_id: &SessionId,
        uid: UserId,
        _client_id: &ClientId,
        req: WsBaseReq,
    ) {
        // 解析正在输入 DTO
        let dto: TypingReqDto = match serde_json::from_value(req.data.clone()) {
            Ok(dto) => dto,
            Err(e) => {
                warn!("解析正在输入消息失败: {}", e);
                return;
            }
        };

        // 不允许给自己发送 typing
        if dto.target_uid == uid {
            return;
        }

        info!(
            "收到正在输入信令: from={}, to={}, room_id={}",
            uid, dto.target_uid, dto.room_id
        );

        // 构建推送给对方的消息
        let notify = TypingNotifyVo {
            uid,
            room_id: dto.room_id,
        };

        let resp = match WsBaseResp::from_data(WsMsgTypeEnum::Typing.as_i32(), &notify) {
            Ok(resp) => resp,
            Err(e) => {
                warn!("构建正在输入响应失败: {}", e);
                return;
            }
        };

        // 推送给目标用户
        if let Err(e) = self
            .push_service
            .send_push_msg_single(resp, dto.target_uid, uid)
            .await
        {
            warn!("推送正在输入消息失败: {}", e);
        }
    }
}

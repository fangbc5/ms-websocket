use crate::enums::WsMsgTypeEnum;
/// 通话质量监控处理器

use crate::model::vo::{network_quality_vo::NetworkQualityVO, screen_sharing_vo::ScreenSharingVO};
use crate::model::ws_base_resp::{WsBaseReq, WsBaseResp};
use crate::service::{PushService, VideoChatService};
use crate::types::{ClientId, RoomId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};

/// 网络质量报告请求
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkReportReq {
    /// 房间 ID
    pub room_id: RoomId,
    /// 网络质量 (0.0 ~ 1.0)
    pub quality: f64,
}

/// 屏幕共享请求
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScreenSharingReq {
    /// 房间 ID
    pub room_id: RoomId,
    /// 是否正在共享
    pub sharing: bool,
}

/// 通话质量监控处理器
///
/// 功能：
/// 1. 处理网络质量报告
/// 2. 管理屏幕共享状态
pub struct QualityMonitorProcessor {
    video_service: Arc<VideoChatService>,
    push_service: Arc<PushService>,
}

impl QualityMonitorProcessor {
    pub fn new(video_service: Arc<VideoChatService>, push_service: Arc<PushService>) -> Self {
        Self {
            video_service,
            push_service,
        }
    }
}

#[async_trait::async_trait]
impl MessageProcessor for QualityMonitorProcessor {
    fn supports(&self, req: &WsBaseReq) -> bool {
        WsMsgTypeEnum::NetworkReport.eq(req.r#type) || WsMsgTypeEnum::ScreenSharing.eq(req.r#type)
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
            Some(WsMsgTypeEnum::NetworkReport) => {
                self.handle_network_report(uid, &req).await;
            }
            Some(WsMsgTypeEnum::ScreenSharing) => {
                self.handle_screen_sharing(uid, &req).await;
            }
            _ => {
                warn!("未知的质量监控消息类型: {}", req.r#type);
            }
        }
    }
}

/// 网络质量阈值 — 低于此值通知管理员
const POOR_QUALITY_THRESHOLD: f64 = 0.3;

impl QualityMonitorProcessor {
    /// 处理网络质量报告
    async fn handle_network_report(&self, uid: UserId, req: &WsBaseReq) {
        let report: NetworkReportReq = match serde_json::from_value(req.data.clone()) {
            Ok(report) => report,
            Err(e) => {
                warn!("解析网络质量报告失败: {}", e);
                return;
            }
        };

        info!(
            "收到网络质量报告: uid={}, room_id={}, quality={}",
            uid, report.room_id, report.quality
        );

        // 1. 存储网络质量数据
        if let Err(e) = self
            .video_service
            .save_network_quality(uid, report.room_id, report.quality)
            .await
        {
            error!("存储网络质量数据失败: {}", e);
        }

        // 2. 如果质量差，通知房间管理员
        if report.quality < POOR_QUALITY_THRESHOLD {
            self.notify_poor_quality(uid, report.room_id, report.quality)
                .await;
        }
    }

    /// 处理屏幕共享状态
    async fn handle_screen_sharing(&self, uid: UserId, req: &WsBaseReq) {
        let sharing: ScreenSharingReq = match serde_json::from_value(req.data.clone()) {
            Ok(sharing) => sharing,
            Err(e) => {
                warn!("解析屏幕共享消息失败: {}", e);
                return;
            }
        };

        info!(
            "收到屏幕共享状态: uid={}, room_id={}, sharing={}",
            uid, sharing.room_id, sharing.sharing
        );

        // 1. 验证用户是否在房间中
        match self
            .video_service
            .is_user_in_room(uid, sharing.room_id)
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                warn!(
                    "用户不在房间中，无法操作屏幕共享: uid={}, room={}",
                    uid, sharing.room_id
                );
                return;
            }
            Err(e) => {
                error!("验证用户房间状态失败: {}", e);
                return;
            }
        }

        // 2. 更新屏幕共享状态
        if let Err(e) = self
            .video_service
            .set_screen_sharing(sharing.room_id, uid, sharing.sharing)
            .await
        {
            error!("更新屏幕共享状态失败: {}", e);
            return;
        }

        // 3. 通知房间成员
        self.notify_screen_sharing(sharing.room_id, uid, sharing.sharing)
            .await;
    }

    /// 通知房间管理员网络质量差
    async fn notify_poor_quality(&self, uid: UserId, room_id: RoomId, quality: f64) {
        let quality_vo = NetworkQualityVO {
            room_id,
            user_id: uid,
            quality,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        let resp = match WsBaseResp::from_data(WsMsgTypeEnum::NetworkPoor.as_i32(), &quality_vo) {
            Ok(resp) => resp,
            Err(e) => {
                error!("序列化网络质量通知失败: {}", e);
                return;
            }
        };

        // 通知管理员
        let admins = self
            .video_service
            .get_room_admins(room_id)
            .await
            .unwrap_or_default();

        if !admins.is_empty() {
            let admins_u64: Vec<u64> = admins.iter().map(|&id| id as u64).collect();
            let _ = self
                .push_service
                .send_push_msg(resp, admins_u64, uid as u64)
                .await;
        }
    }

    /// 通知房间成员屏幕共享状态
    async fn notify_screen_sharing(&self, room_id: RoomId, uid: UserId, sharing: bool) {
        let sharing_vo = ScreenSharingVO {
            room_id,
            user_id: uid,
            sharing,
        };

        let msg_type = if sharing {
            WsMsgTypeEnum::ScreenSharingStarted
        } else {
            WsMsgTypeEnum::ScreenSharingStopped
        };

        let resp = match WsBaseResp::from_data(msg_type.as_i32(), &sharing_vo) {
            Ok(resp) => resp,
            Err(e) => {
                error!("序列化屏幕共享通知失败: {}", e);
                return;
            }
        };

        // 通知房间中除操作者外的所有成员
        let mut members = self
            .video_service
            .get_room_members(room_id)
            .await
            .unwrap_or_default();
        members.retain(|&m| m != uid);

        if !members.is_empty() {
            let members_u64: Vec<u64> = members.iter().map(|&id| id as u64).collect();
            let _ = self
                .push_service
                .send_push_msg(resp, members_u64, uid as u64)
                .await;
        }
    }
}

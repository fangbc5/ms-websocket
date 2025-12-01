/// 通话质量监控处理器

// use crate::model::vo::{network_quality_vo::NetworkQualityVO, screen_sharing_vo::ScreenSharingVO};
use crate::model::ws_base_resp::WsBaseReq;
use crate::types::{ClientId, SessionId, UserId};
use crate::websocket::processor::message_processor::MessageProcessor;
use crate::websocket::session_manager::Session;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// 网络质量报告请求
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkReportReq {
    /// 房间 ID
    pub room_id: u64,
    /// 网络质量 (0.0 ~ 1.0)
    pub quality: f64,
}

/// 屏幕共享请求
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScreenSharingReq {
    /// 房间 ID
    pub room_id: u64,
    /// 是否正在共享
    pub sharing: bool,
}

/// 通话质量监控处理器
/// 
/// 功能：
/// 1. 处理网络质量报告
/// 2. 管理屏幕共享状态
pub struct QualityMonitorProcessor;

impl QualityMonitorProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl MessageProcessor for QualityMonitorProcessor {
    fn supports(&self, req: &WsBaseReq) -> bool {
        req.r#type == "network_report"
            || req.r#type == "NETWORK_REPORT"
            || req.r#type == "screen_sharing"
            || req.r#type == "SCREEN_SHARING"
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
            "network_report" | "NETWORK_REPORT" => {
                self.handle_network_report(uid, &req).await;
            }
            "screen_sharing" | "SCREEN_SHARING" => {
                self.handle_screen_sharing(uid, &req).await;
            }
            _ => {
                tracing::warn!("未知的质量监控消息类型: {}", req.r#type);
            }
        }
    }
}

impl QualityMonitorProcessor {
    async fn handle_network_report(&self, uid: UserId, req: &WsBaseReq) {
        let report: NetworkReportReq = match serde_json::from_value(req.data.clone()) {
            Ok(report) => report,
            Err(e) => {
                tracing::warn!("解析网络质量报告失败: {}", e);
                return;
            }
        };

        info!(
            "收到网络质量报告: uid={}, room_id={}, quality={}",
            uid, report.room_id, report.quality
        );

        // TODO: 存储网络质量数据
        // video_service.saveNetworkQuality(uid, report.room_id, report.quality);

        // 如果质量差，通知房间管理员
        if report.quality < 0.3 {
            // TODO: 通知房间管理员
            // notify_poor_quality(uid, report.room_id, report.quality);
        }
    }

    async fn handle_screen_sharing(&self, uid: UserId, req: &WsBaseReq) {
        let sharing: ScreenSharingReq = match serde_json::from_value(req.data.clone()) {
            Ok(sharing) => sharing,
            Err(e) => {
                tracing::warn!("解析屏幕共享消息失败: {}", e);
                return;
            }
        };

        info!(
            "收到屏幕共享状态: uid={}, room_id={}, sharing={}",
            uid, sharing.room_id, sharing.sharing
        );

        // TODO: 验证用户是否在房间中
        // if !video_service.isUserInRoom(uid, sharing.room_id) {
        //     return;
        // }

        // TODO: 更新屏幕共享状态
        // video_service.setScreenSharing(sharing.room_id, uid, sharing.sharing);

        // TODO: 通知房间成员
        // notify_screen_sharing(sharing.room_id, uid, sharing.sharing);
    }
}

impl Default for QualityMonitorProcessor {
    fn default() -> Self {
        Self::new()
    }
}


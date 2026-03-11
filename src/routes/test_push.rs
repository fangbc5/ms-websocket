/// 测试推送接口
///
/// 仅用于开发/测试环境，提供 HTTP 接口直接向在线用户推送消息
/// 模拟 ms-im 等业务服务的消息推送流程
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

use crate::model::ws_base_resp::WsBaseResp;
use crate::state::WsState;

/// 测试推送请求体
#[derive(Debug, Deserialize)]
pub struct TestPushRequest {
    /// 消息类型（对应 WSRespTypeEnum）
    pub r#type: i32,
    /// 消息数据（任意 JSON）
    pub data: serde_json::Value,
    /// 目标用户 UID 列表
    pub target_uids: Vec<u64>,
    /// 发送者 UID
    pub sender_uid: u64,
}

/// 推送结果响应
#[derive(Debug, Serialize)]
pub struct TestPushResponse {
    pub success: bool,
    pub message: String,
    /// 成功推送的用户数
    pub delivered_count: usize,
    /// 目标用户总数
    pub target_count: usize,
}

/// 在线用户信息
#[derive(Debug, Serialize)]
pub struct OnlineUserInfo {
    pub uid: u64,
    pub session_count: usize,
}

/// 在线用户响应
#[derive(Debug, Serialize)]
pub struct OnlineUsersResponse {
    pub total_users: usize,
    pub total_sessions: usize,
    pub users: Vec<OnlineUserInfo>,
}

/// POST /api/test/push
///
/// 测试接口：推送消息到指定用户
///
/// ## 请求示例
/// ```json
/// {
///   "type": 1001,
///   "data": {"content": "你好", "from_uid": 1001, "msg_id": 1},
///   "target_uids": [1002],
///   "sender_uid": 1001
/// }
/// ```
///
/// ## 流程
/// 1. 构建 WsBaseResp 消息
/// 2. 通过 PushService 推送（支持本地+跨节点）
/// 3. 返回推送结果
pub async fn test_push_handler(
    State(ws_state): State<Arc<WsState>>,
    Json(req): Json<TestPushRequest>,
) -> impl IntoResponse {
    info!(
        "📨 测试推送: sender={}, targets={:?}, type={}",
        req.sender_uid, req.target_uids, req.r#type
    );

    if req.target_uids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(TestPushResponse {
                success: false,
                message: "target_uids 不能为空".into(),
                delivered_count: 0,
                target_count: 0,
            }),
        );
    }

    let target_count = req.target_uids.len();

    // 构建 WsBaseResp
    let ws_msg = WsBaseResp::new(req.r#type, req.data);

    // 通过 PushService 推送
    match ws_state
        .services
        .push_service
        .send_push_msg(ws_msg, req.target_uids, req.sender_uid)
        .await
    {
        Ok(delivered_count) => {
            info!("📨 测试推送成功: sender={}, delivered={}/{}", req.sender_uid, delivered_count, target_count);
            (
                StatusCode::OK,
                Json(TestPushResponse {
                    success: true,
                    message: if delivered_count > 0 {
                        "推送成功".into()
                    } else {
                        "推送已处理，但目标用户可能不在线或路由信息不存在".into()
                    },
                    delivered_count,
                    target_count,
                }),
            )
        }
        Err(e) => {
            warn!("📨 测试推送失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TestPushResponse {
                    success: false,
                    message: format!("推送失败: {}", e),
                    delivered_count: 0,
                    target_count,
                }),
            )
        }
    }
}

/// GET /api/test/online
///
/// 测试接口：查询当前节点在线用户列表
pub async fn online_users_handler(
    State(ws_state): State<Arc<WsState>>,
) -> impl IntoResponse {
    let session_manager = &ws_state.session_manager;
    let total_sessions = session_manager.get_session_count();

    // 获取所有在线用户及其会话数
    let online_users = session_manager.get_online_users_info();

    Json(OnlineUsersResponse {
        total_users: online_users.len(),
        total_sessions,
        users: online_users,
    })
}

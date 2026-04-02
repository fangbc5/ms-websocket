//! WebSocket 服务统一错误处理模块
//!
//! 参考组织内的其他服务（如 ms-team），抽象统一的错误处理码。

pub mod error_code {
    // ============ WebSocket 异常断开状态码 (4000-4999 私有定义范围) ============
    /// 4009 被其他设备挤占下线
    pub const KICKED_BY_OTHER_DEVICE: u16 = 4009;

    // ============ API 业务错误码 ============
    /// 7001 会话不存在
    pub const SESSION_NOT_FOUND: i32 = 7001;
    /// 7002 消息发送失败
    pub const MESSAGE_SEND_FAILED: i32 = 7002;
    /// 7901 系统内部错误
    pub const INTERNAL_SERVER_ERROR: i32 = 7901;
}

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use fbc_starter::R;
use thiserror::Error;

/// WebSocket 服务错误枚举（与企业级规范看齐）
#[derive(Debug, Error)]
pub enum WsError {
    // ============ WebSocket 特属断开原因 ============
    // 前端会提取 message 用作具体解释，此处复用原来的提示信息
    #[error("Kicked by another device")]
    KickedByOtherDevice,

    // ============ 普通业务错误 ============
    #[error("会话不存在")]
    SessionNotFound,

    #[error("消息发送失败：{0}")]
    MessageSendFailed(String),

    #[error("系统内部错误：{0}")]
    InternalServerError(String),
}

impl WsError {
    /// 获取业务错误码
    pub fn code(&self) -> i32 {
        use error_code::*;
        match self {
            WsError::KickedByOtherDevice => KICKED_BY_OTHER_DEVICE as i32,
            WsError::SessionNotFound => SESSION_NOT_FOUND,
            WsError::MessageSendFailed(_) => MESSAGE_SEND_FAILED,
            WsError::InternalServerError(_) => INTERNAL_SERVER_ERROR,
        }
    }

    /// 获取 HTTP 状态码
    pub fn status_code(&self) -> StatusCode {
        match self {
            WsError::SessionNotFound => StatusCode::NOT_FOUND,
            WsError::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

/// 实现 IntoResponse 以便在 HTTP Handler 中当做普通业务借口错误返回
impl IntoResponse for WsError {
    fn into_response(self) -> Response {
        let code = self.code();
        let message = self.to_string();
        let status = self.status_code();

        tracing::warn!(
            error_code = code,
            error_message = %message,
            status = %status,
            "WebSocket API错误响应"
        );

        (status, Json(R::<()>::fail_with_code(code, message))).into_response()
    }
}

/// 统一的 Result 类型别名
pub type Result<T> = std::result::Result<T, WsError>;

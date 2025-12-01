/// WebSocket 请求类型枚举
///
/// 定义前端发送的 WebSocket 请求类型
use serde::{Deserialize, Serialize};

/// WebSocket 请求类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WSReqTypeEnum {
    /// 请求登录二维码
    Login = 1,
    /// 心跳包
    Heartbeat = 2,
    /// 登录认证
    Authorize = 3,
    /// 视频心跳
    VideoHeartbeat = 4,
    /// 视频通话请求
    VideoCallRequest = 5,
    /// 视频通话响应
    VideoCallResponse = 6,
    /// 媒体静音音频
    MediaMuteAudio = 7,
    /// 静音视频
    MediaMuteVideo = 8,
    /// 静音全部用户
    MediaMuteAll = 9,
    /// 屏幕共享
    ScreenSharing = 10,
    /// 关闭房间
    CloseRoom = 11,
    /// 踢出用户
    KickUser = 12,
    /// 通话质量监控
    NetworkReport = 13,
    /// 信令消息
    WebrtcSignal = 14,
    /// 消息确认接收 ack
    Ack = 15,
    /// 消息已读
    Read = 16,
}

impl WSReqTypeEnum {
    /// 获取类型值
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// 获取描述
    pub fn desc(&self) -> &'static str {
        match self {
            WSReqTypeEnum::Login => "请求登录二维码",
            WSReqTypeEnum::Heartbeat => "心跳包",
            WSReqTypeEnum::Authorize => "登录认证",
            WSReqTypeEnum::VideoHeartbeat => "视频心跳",
            WSReqTypeEnum::VideoCallRequest => "视频通话请求",
            WSReqTypeEnum::VideoCallResponse => "视频通话响应",
            WSReqTypeEnum::MediaMuteAudio => "媒体静音音频",
            WSReqTypeEnum::MediaMuteVideo => "静音视频",
            WSReqTypeEnum::MediaMuteAll => "静音全部用户",
            WSReqTypeEnum::ScreenSharing => "屏幕共享",
            WSReqTypeEnum::CloseRoom => "关闭房间",
            WSReqTypeEnum::KickUser => "踢出用户",
            WSReqTypeEnum::NetworkReport => "通话质量监控",
            WSReqTypeEnum::WebrtcSignal => "信令消息",
            WSReqTypeEnum::Ack => "消息确认接收ack",
            WSReqTypeEnum::Read => "消息已读",
        }
    }

    /// 判断是否等于指定类型值
    pub fn eq(&self, type_val: i32) -> bool {
        self.as_i32() == type_val
    }
}

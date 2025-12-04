/// WebSocket 请求类型枚举
///
/// 定义前端发送的 WebSocket 请求类型
use serde::{Deserialize, Serialize};

/// WebSocket 请求类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WsMsgTypeEnum {
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
    /// 超时
    Timeout = 17,
    /// 用户加入房间
    JoinVideo = 18,
    /// 用户离开房间
    LeaveVideo = 19,
}

impl WsMsgTypeEnum {
    /// 从 i32 值转换为枚举类型
    pub fn from(value: i32) -> Option<Self> {
        match value {
            1 => Some(WsMsgTypeEnum::Login),
            2 => Some(WsMsgTypeEnum::Heartbeat),
            3 => Some(WsMsgTypeEnum::Authorize),
            4 => Some(WsMsgTypeEnum::VideoHeartbeat),
            5 => Some(WsMsgTypeEnum::VideoCallRequest),
            6 => Some(WsMsgTypeEnum::VideoCallResponse),
            7 => Some(WsMsgTypeEnum::MediaMuteAudio),
            8 => Some(WsMsgTypeEnum::MediaMuteVideo),
            9 => Some(WsMsgTypeEnum::MediaMuteAll),
            10 => Some(WsMsgTypeEnum::ScreenSharing),
            11 => Some(WsMsgTypeEnum::CloseRoom),
            12 => Some(WsMsgTypeEnum::KickUser),
            13 => Some(WsMsgTypeEnum::NetworkReport),
            14 => Some(WsMsgTypeEnum::WebrtcSignal),
            15 => Some(WsMsgTypeEnum::Ack),
            16 => Some(WsMsgTypeEnum::Read),
            17 => Some(WsMsgTypeEnum::Timeout),
            18 => Some(WsMsgTypeEnum::JoinVideo),
            19 => Some(WsMsgTypeEnum::LeaveVideo),
            _ => None,
        }
    }

    /// 获取类型值
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// 获取描述
    pub fn desc(&self) -> &'static str {
        match self {
            WsMsgTypeEnum::Login => "请求登录二维码",
            WsMsgTypeEnum::Heartbeat => "心跳包",
            WsMsgTypeEnum::Authorize => "登录认证",
            WsMsgTypeEnum::VideoHeartbeat => "视频心跳",
            WsMsgTypeEnum::VideoCallRequest => "视频通话请求",
            WsMsgTypeEnum::VideoCallResponse => "视频通话响应",
            WsMsgTypeEnum::MediaMuteAudio => "媒体静音音频",
            WsMsgTypeEnum::MediaMuteVideo => "静音视频",
            WsMsgTypeEnum::MediaMuteAll => "静音全部用户",
            WsMsgTypeEnum::ScreenSharing => "屏幕共享",
            WsMsgTypeEnum::CloseRoom => "关闭房间",
            WsMsgTypeEnum::KickUser => "踢出用户",
            WsMsgTypeEnum::NetworkReport => "通话质量监控",
            WsMsgTypeEnum::WebrtcSignal => "信令消息",
            WsMsgTypeEnum::Ack => "消息确认接收ack",
            WsMsgTypeEnum::Read => "消息已读",
            WsMsgTypeEnum::Timeout => "超时",
            WsMsgTypeEnum::JoinVideo => "用户加入房间",
            WsMsgTypeEnum::LeaveVideo => "用户离开房间",
        }
    }

    /// 判断是否等于指定类型值
    pub fn eq(&self, type_val: i32) -> bool {
        self.as_i32() == type_val
    }
}

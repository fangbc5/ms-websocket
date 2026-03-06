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

    // === 响应专用类型（20+） ===
    /// 通话已接通（推送给双方）
    CallAccepted = 20,
    /// 呼叫被拒绝
    CallRejected = 21,
    /// 取消通话
    Cancel = 22,
    /// 挂断通话
    Dropped = 23,
    /// 媒体控制变更通知
    MediaControl = 24,
    /// 开始信令
    StartSignaling = 25,
    /// 开始屏幕共享
    ScreenSharingStarted = 26,
    /// 停止屏幕共享
    ScreenSharingStopped = 27,
    /// 网络质量差
    NetworkPoor = 28,
    /// 用户被踢出
    UserKicked = 29,
    /// 全体静音
    AllMuted = 30,

    // === 在线状态通知类型（40+） ===
    /// 上线通知
    Online = 40,
    /// 下线通知
    Offline = 41,
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
            20 => Some(WsMsgTypeEnum::CallAccepted),
            21 => Some(WsMsgTypeEnum::CallRejected),
            22 => Some(WsMsgTypeEnum::Cancel),
            23 => Some(WsMsgTypeEnum::Dropped),
            24 => Some(WsMsgTypeEnum::MediaControl),
            25 => Some(WsMsgTypeEnum::StartSignaling),
            26 => Some(WsMsgTypeEnum::ScreenSharingStarted),
            27 => Some(WsMsgTypeEnum::ScreenSharingStopped),
            28 => Some(WsMsgTypeEnum::NetworkPoor),
            29 => Some(WsMsgTypeEnum::UserKicked),
            30 => Some(WsMsgTypeEnum::AllMuted),
            40 => Some(WsMsgTypeEnum::Online),
            41 => Some(WsMsgTypeEnum::Offline),
            _ => None,
        }
    }

    /// 获取类型值
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// 获取描述
    #[allow(dead_code)]
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
            WsMsgTypeEnum::CallAccepted => "通话已接通",
            WsMsgTypeEnum::CallRejected => "呼叫被拒绝",
            WsMsgTypeEnum::Cancel => "取消通话",
            WsMsgTypeEnum::Dropped => "挂断通话",
            WsMsgTypeEnum::MediaControl => "媒体控制变更",
            WsMsgTypeEnum::StartSignaling => "开始信令",
            WsMsgTypeEnum::ScreenSharingStarted => "开始屏幕共享",
            WsMsgTypeEnum::ScreenSharingStopped => "停止屏幕共享",
            WsMsgTypeEnum::NetworkPoor => "网络质量差",
            WsMsgTypeEnum::UserKicked => "用户被踢出",
            WsMsgTypeEnum::AllMuted => "全体静音",
            WsMsgTypeEnum::Online => "上线通知",
            WsMsgTypeEnum::Offline => "下线通知",
        }
    }

    /// 判断是否等于指定类型值
    pub fn eq(&self, type_val: i32) -> bool {
        self.as_i32() == type_val
    }
}

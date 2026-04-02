/// 枚举模块测试
use ms_websocket::enums::{CallResponseStatus, WsMsgTypeEnum, WsPushTypeEnum};

// ========================
// WsMsgTypeEnum 测试
// ========================

#[test]
fn test_ws_msg_type_from_all_variants() {
    let cases = vec![
        (1, WsMsgTypeEnum::Login),
        (2, WsMsgTypeEnum::Heartbeat),
        (3, WsMsgTypeEnum::Authorize),
        (4, WsMsgTypeEnum::VideoHeartbeat),
        (5, WsMsgTypeEnum::VideoCallRequest),
        (6, WsMsgTypeEnum::VideoCallResponse),
        (7, WsMsgTypeEnum::MediaMuteAudio),
        (8, WsMsgTypeEnum::MediaMuteVideo),
        (9, WsMsgTypeEnum::MediaMuteAll),
        (10, WsMsgTypeEnum::ScreenSharing),
        (11, WsMsgTypeEnum::CloseRoom),
        (12, WsMsgTypeEnum::KickUser),
        (13, WsMsgTypeEnum::NetworkReport),
        (14, WsMsgTypeEnum::WebrtcSignal),
        (15, WsMsgTypeEnum::Ack),
        (16, WsMsgTypeEnum::Read),
        (17, WsMsgTypeEnum::Timeout),
        (18, WsMsgTypeEnum::JoinVideo),
        (19, WsMsgTypeEnum::LeaveVideo),
    ];

    for (value, expected) in cases {
        assert_eq!(
            WsMsgTypeEnum::from(value),
            Some(expected),
            "from({}) 应返回 {:?}",
            value,
            expected
        );
    }
}

#[test]
fn test_ws_msg_type_from_invalid() {
    assert_eq!(WsMsgTypeEnum::from(0), None);
    assert_eq!(WsMsgTypeEnum::from(-1), None);
    assert_eq!(WsMsgTypeEnum::from(32), None);
    assert_eq!(WsMsgTypeEnum::from(100), None);
    assert_eq!(WsMsgTypeEnum::from(i32::MAX), None);
    assert_eq!(WsMsgTypeEnum::from(i32::MIN), None);
}

#[test]
fn test_ws_msg_type_as_i32_roundtrip() {
    // 1..=19 原有请求类型
    for i in 1..=19 {
        let variant = WsMsgTypeEnum::from(i).unwrap();
        assert_eq!(variant.as_i32(), i, "as_i32() 应返回 {}", i);
    }
    // 20..=31 新增的响应类型与信令类型
    for i in 20..=31 {
        let variant = WsMsgTypeEnum::from(i).unwrap();
        assert_eq!(variant.as_i32(), i, "as_i32() 应返回 {}", i);
    }
    // 40..=41 在线状态通知类型
    for i in 40..=41 {
        let variant = WsMsgTypeEnum::from(i).unwrap();
        assert_eq!(variant.as_i32(), i, "as_i32() 应返回 {}", i);
    }
}

#[test]
fn test_ws_msg_type_desc_non_empty() {
    for i in 1..=19 {
        let variant = WsMsgTypeEnum::from(i).unwrap();
        let desc = variant.desc();
        assert!(!desc.is_empty(), "type {} 的描述不应为空", i);
    }
}

#[test]
fn test_ws_msg_type_desc_values() {
    assert_eq!(WsMsgTypeEnum::Login.desc(), "请求登录二维码");
    assert_eq!(WsMsgTypeEnum::Heartbeat.desc(), "心跳包");
    assert_eq!(WsMsgTypeEnum::Authorize.desc(), "登录认证");
    assert_eq!(WsMsgTypeEnum::VideoHeartbeat.desc(), "视频心跳");
    assert_eq!(WsMsgTypeEnum::VideoCallRequest.desc(), "视频通话请求");
    assert_eq!(WsMsgTypeEnum::VideoCallResponse.desc(), "视频通话响应");
    assert_eq!(WsMsgTypeEnum::MediaMuteAudio.desc(), "媒体静音音频");
    assert_eq!(WsMsgTypeEnum::MediaMuteVideo.desc(), "静音视频");
    assert_eq!(WsMsgTypeEnum::MediaMuteAll.desc(), "静音全部用户");
    assert_eq!(WsMsgTypeEnum::ScreenSharing.desc(), "屏幕共享");
    assert_eq!(WsMsgTypeEnum::CloseRoom.desc(), "关闭房间");
    assert_eq!(WsMsgTypeEnum::KickUser.desc(), "踢出用户");
    assert_eq!(WsMsgTypeEnum::NetworkReport.desc(), "通话质量监控");
    assert_eq!(WsMsgTypeEnum::WebrtcSignal.desc(), "信令消息");
    assert_eq!(WsMsgTypeEnum::Ack.desc(), "消息确认接收ack");
    assert_eq!(WsMsgTypeEnum::Read.desc(), "消息已读");
    assert_eq!(WsMsgTypeEnum::Timeout.desc(), "超时");
    assert_eq!(WsMsgTypeEnum::JoinVideo.desc(), "用户加入房间");
    assert_eq!(WsMsgTypeEnum::LeaveVideo.desc(), "用户离开房间");
}

#[test]
fn test_ws_msg_type_eq_method() {
    assert!(WsMsgTypeEnum::Heartbeat.eq(2));
    assert!(!WsMsgTypeEnum::Heartbeat.eq(1));
    assert!(WsMsgTypeEnum::Login.eq(1));
    assert!(!WsMsgTypeEnum::Login.eq(0));
    assert!(WsMsgTypeEnum::LeaveVideo.eq(19));
    assert!(!WsMsgTypeEnum::LeaveVideo.eq(18));
}

#[test]
fn test_ws_msg_type_clone() {
    let original = WsMsgTypeEnum::Heartbeat;
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

#[test]
fn test_ws_msg_type_debug() {
    let debug_str = format!("{:?}", WsMsgTypeEnum::Heartbeat);
    assert!(debug_str.contains("Heartbeat"));
}

#[test]
fn test_ws_msg_type_serde_serialize() {
    let value = serde_json::to_value(WsMsgTypeEnum::Heartbeat).unwrap();
    assert_eq!(value, serde_json::json!("HEARTBEAT"));

    let value = serde_json::to_value(WsMsgTypeEnum::Login).unwrap();
    assert_eq!(value, serde_json::json!("LOGIN"));

    let value = serde_json::to_value(WsMsgTypeEnum::WebrtcSignal).unwrap();
    assert_eq!(value, serde_json::json!("WEBRTC_SIGNAL"));
}

#[test]
fn test_ws_msg_type_serde_deserialize() {
    let heartbeat: WsMsgTypeEnum = serde_json::from_str("\"HEARTBEAT\"").unwrap();
    assert_eq!(heartbeat, WsMsgTypeEnum::Heartbeat);

    let login: WsMsgTypeEnum = serde_json::from_str("\"LOGIN\"").unwrap();
    assert_eq!(login, WsMsgTypeEnum::Login);
}

#[test]
fn test_ws_msg_type_serde_roundtrip() {
    for i in 1..=19 {
        let variant = WsMsgTypeEnum::from(i).unwrap();
        let json = serde_json::to_string(&variant).unwrap();
        let deserialized: WsMsgTypeEnum = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, deserialized, "type {} serde 往返失败", i);
    }
}

#[test]
fn test_ws_msg_type_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(WsMsgTypeEnum::Heartbeat);
    set.insert(WsMsgTypeEnum::Login);
    set.insert(WsMsgTypeEnum::Heartbeat); // 重复
    assert_eq!(set.len(), 2);
}

// ========================
// WsPushTypeEnum 测试
// ========================

#[test]
fn test_ws_push_type_as_i32() {
    assert_eq!(WsPushTypeEnum::User.as_i32(), 1);
    assert_eq!(WsPushTypeEnum::All.as_i32(), 2);
}

#[test]
fn test_ws_push_type_desc() {
    assert_eq!(WsPushTypeEnum::User.desc(), "个人");
    assert_eq!(WsPushTypeEnum::All.desc(), "全部连接用户");
}

#[test]
fn test_ws_push_type_clone() {
    let original = WsPushTypeEnum::User;
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

#[test]
fn test_ws_push_type_debug() {
    let debug_str = format!("{:?}", WsPushTypeEnum::User);
    assert!(debug_str.contains("User"));
}

#[test]
fn test_ws_push_type_serde_serialize() {
    let value = serde_json::to_value(WsPushTypeEnum::User).unwrap();
    assert_eq!(value, serde_json::json!("USER"));

    let value = serde_json::to_value(WsPushTypeEnum::All).unwrap();
    assert_eq!(value, serde_json::json!("ALL"));
}

#[test]
fn test_ws_push_type_serde_deserialize() {
    let user: WsPushTypeEnum = serde_json::from_str("\"USER\"").unwrap();
    assert_eq!(user, WsPushTypeEnum::User);

    let all: WsPushTypeEnum = serde_json::from_str("\"ALL\"").unwrap();
    assert_eq!(all, WsPushTypeEnum::All);
}

#[test]
fn test_ws_push_type_serde_roundtrip() {
    let variants = vec![WsPushTypeEnum::User, WsPushTypeEnum::All];
    for variant in variants {
        let json = serde_json::to_string(&variant).unwrap();
        let deserialized: WsPushTypeEnum = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, deserialized);
    }
}

#[test]
fn test_ws_push_type_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(WsPushTypeEnum::User);
    set.insert(WsPushTypeEnum::All);
    set.insert(WsPushTypeEnum::User); // 重复
    assert_eq!(set.len(), 2);
}

// ========================
// P3 响应类型枚举测试（20-30）
// ========================

#[test]
fn test_ws_msg_type_p3_response_variants() {
    let cases = vec![
        (20, WsMsgTypeEnum::CallAccepted),
        (21, WsMsgTypeEnum::CallRejected),
        (22, WsMsgTypeEnum::Cancel),
        (23, WsMsgTypeEnum::Dropped),
        (24, WsMsgTypeEnum::MediaControl),
        (25, WsMsgTypeEnum::StartSignaling),
        (26, WsMsgTypeEnum::ScreenSharingStarted),
        (27, WsMsgTypeEnum::ScreenSharingStopped),
        (28, WsMsgTypeEnum::NetworkPoor),
        (29, WsMsgTypeEnum::UserKicked),
        (30, WsMsgTypeEnum::AllMuted),
        (31, WsMsgTypeEnum::Typing),
    ];

    for (value, expected) in cases {
        assert_eq!(
            WsMsgTypeEnum::from(value),
            Some(expected),
            "from({}) 应返回 {:?}",
            value,
            expected
        );
    }
}

#[test]
fn test_ws_msg_type_p3_desc_values() {
    assert_eq!(WsMsgTypeEnum::CallAccepted.desc(), "通话已接通");
    assert_eq!(WsMsgTypeEnum::CallRejected.desc(), "呼叫被拒绝");
    assert_eq!(WsMsgTypeEnum::Cancel.desc(), "取消通话");
    assert_eq!(WsMsgTypeEnum::Dropped.desc(), "挂断通话");
    assert_eq!(WsMsgTypeEnum::MediaControl.desc(), "媒体控制变更");
    assert_eq!(WsMsgTypeEnum::StartSignaling.desc(), "开始信令");
    assert_eq!(WsMsgTypeEnum::ScreenSharingStarted.desc(), "开始屏幕共享");
    assert_eq!(WsMsgTypeEnum::ScreenSharingStopped.desc(), "停止屏幕共享");
    assert_eq!(WsMsgTypeEnum::NetworkPoor.desc(), "网络质量差");
    assert_eq!(WsMsgTypeEnum::UserKicked.desc(), "用户被踢出");
    assert_eq!(WsMsgTypeEnum::AllMuted.desc(), "全体静音");
    assert_eq!(WsMsgTypeEnum::Typing.desc(), "正在输入");
}

#[test]
fn test_ws_msg_type_p3_serde_roundtrip() {
    for i in 20..=31 {
        let variant = WsMsgTypeEnum::from(i).unwrap();
        let json = serde_json::to_string(&variant).unwrap();
        let deserialized: WsMsgTypeEnum = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, deserialized, "type {} serde 往返失败", i);
    }
}

#[test]
fn test_ws_msg_type_p3_eq_method() {
    assert!(WsMsgTypeEnum::CallAccepted.eq(20));
    assert!(!WsMsgTypeEnum::CallAccepted.eq(21));
    assert!(WsMsgTypeEnum::Typing.eq(31));
    assert!(!WsMsgTypeEnum::Typing.eq(30));
}

// ========================
// P1 在线状态通知类型测试（40-41）
// ========================

#[test]
fn test_ws_msg_type_online_offline_variants() {
    assert_eq!(WsMsgTypeEnum::from(40), Some(WsMsgTypeEnum::Online));
    assert_eq!(WsMsgTypeEnum::from(41), Some(WsMsgTypeEnum::Offline));
}

#[test]
fn test_ws_msg_type_online_offline_desc() {
    assert_eq!(WsMsgTypeEnum::Online.desc(), "上线通知");
    assert_eq!(WsMsgTypeEnum::Offline.desc(), "下线通知");
}

#[test]
fn test_ws_msg_type_online_offline_as_i32() {
    assert_eq!(WsMsgTypeEnum::Online.as_i32(), 40);
    assert_eq!(WsMsgTypeEnum::Offline.as_i32(), 41);
}

#[test]
fn test_ws_msg_type_online_offline_eq() {
    assert!(WsMsgTypeEnum::Online.eq(40));
    assert!(!WsMsgTypeEnum::Online.eq(41));
    assert!(WsMsgTypeEnum::Offline.eq(41));
    assert!(!WsMsgTypeEnum::Offline.eq(40));
}

#[test]
fn test_ws_msg_type_online_offline_serde_roundtrip() {
    let online = WsMsgTypeEnum::Online;
    let json = serde_json::to_string(&online).unwrap();
    let deserialized: WsMsgTypeEnum = serde_json::from_str(&json).unwrap();
    assert_eq!(online, deserialized);

    let offline = WsMsgTypeEnum::Offline;
    let json = serde_json::to_string(&offline).unwrap();
    let deserialized: WsMsgTypeEnum = serde_json::from_str(&json).unwrap();
    assert_eq!(offline, deserialized);
}

#[test]
fn test_ws_msg_type_gap_32_to_39_invalid() {
    // 32-39 之间没有定义值
    for i in 32..=39 {
        assert_eq!(
            WsMsgTypeEnum::from(i),
            None,
            "from({}) 应返回 None（未定义区间）",
            i
        );
    }
}

#[test]
fn test_ws_msg_type_42_plus_invalid() {
    // 42+ 没有定义值
    for i in 42..=50 {
        assert_eq!(WsMsgTypeEnum::from(i), None, "from({}) 应返回 None", i);
    }
}

// ========================
// CallResponseStatus 枚举测试（P3 新增）
// ========================

#[test]
fn test_call_response_status_of_all_variants() {
    assert_eq!(CallResponseStatus::of(-1), Some(CallResponseStatus::Timeout));
    assert_eq!(CallResponseStatus::of(0), Some(CallResponseStatus::Rejected));
    assert_eq!(CallResponseStatus::of(1), Some(CallResponseStatus::Accepted));
    assert_eq!(CallResponseStatus::of(2), Some(CallResponseStatus::Hangup));
}

#[test]
fn test_call_response_status_of_invalid() {
    assert_eq!(CallResponseStatus::of(-2), None);
    assert_eq!(CallResponseStatus::of(3), None);
    assert_eq!(CallResponseStatus::of(100), None);
    assert_eq!(CallResponseStatus::of(i32::MAX), None);
    assert_eq!(CallResponseStatus::of(i32::MIN), None);
}

#[test]
fn test_call_response_status_clone_eq() {
    let status = CallResponseStatus::Accepted;
    let cloned = status.clone();
    assert_eq!(status, cloned);
    assert_ne!(CallResponseStatus::Accepted, CallResponseStatus::Rejected);
    assert_ne!(CallResponseStatus::Timeout, CallResponseStatus::Hangup);
}

#[test]
fn test_call_response_status_debug() {
    let debug_str = format!("{:?}", CallResponseStatus::Accepted);
    assert!(debug_str.contains("Accepted"));
    let debug_str = format!("{:?}", CallResponseStatus::Timeout);
    assert!(debug_str.contains("Timeout"));
}

#[test]
fn test_call_response_status_as_i32() {
    // 通过 of 反向验证值
    assert_eq!(CallResponseStatus::Timeout as i32, -1);
    assert_eq!(CallResponseStatus::Rejected as i32, 0);
    assert_eq!(CallResponseStatus::Accepted as i32, 1);
    assert_eq!(CallResponseStatus::Hangup as i32, 2);
}

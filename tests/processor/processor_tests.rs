/// 消息处理器测试
///
/// 覆盖所有 Processor 的 supports() 和 process() 方法
use crate::common::*;
use ms_websocket::model::ws_base_resp::WsBaseReq;
use ms_websocket::service::Services;
use ms_websocket::websocket::processor::{
    DefaultMessageProcessor, HeartbeatProcessor, MessageProcessor,
};
use ms_websocket::websocket::processor::meet::{
    MediaControlProcessor, QualityMonitorProcessor, RoomAdminProcessor, VideoCallProcessor,
    VideoProcessor,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::OnceCell;

type TestServices = (
    Arc<fbc_starter::AppState>,
    Arc<Services>,
    Arc<ms_websocket::websocket::SessionManager>,
);

static TEST_SERVICES: OnceCell<TestServices> = OnceCell::const_new();

async fn get_services() -> &'static TestServices {
    TEST_SERVICES
        .get_or_init(|| async { create_test_services().await })
        .await
}

fn mk_video(s: &Arc<Services>) -> VideoProcessor {
    VideoProcessor::new(s.video_chat_service.clone(), s.room_timeout_service.clone())
}

fn mk_video_call(s: &Arc<Services>) -> VideoCallProcessor {
    VideoCallProcessor::new(
        s.video_chat_service.clone(),
        s.push_service.clone(),
        s.room_timeout_service.clone(),
    )
}

fn mk_media_control(s: &Arc<Services>) -> MediaControlProcessor {
    MediaControlProcessor::new(s.video_chat_service.clone())
}

fn mk_room_admin(s: &Arc<Services>) -> RoomAdminProcessor {
    RoomAdminProcessor::new(
        s.video_chat_service.clone(),
        s.push_service.clone(),
        s.room_timeout_service.clone(),
    )
}

fn mk_quality_monitor(s: &Arc<Services>) -> QualityMonitorProcessor {
    QualityMonitorProcessor::new(s.video_chat_service.clone(), s.push_service.clone())
}

// ========================
// HeartbeatProcessor 测试
// ========================

#[tokio::test]
async fn test_heartbeat_processor_supports_heartbeat() {
    let processor = HeartbeatProcessor::new();
    let req = WsBaseReq { r#type: 2, data: json!({}) };
    assert!(processor.supports(&req));
}

#[tokio::test]
async fn test_heartbeat_processor_rejects_other_types() {
    let processor = HeartbeatProcessor::new();
    for t in [1, 3, 4, 5, 14, 15, 16, 99] {
        let req = WsBaseReq { r#type: t, data: json!({}) };
        assert!(!processor.supports(&req), "HeartbeatProcessor 不应支持 type={}", t);
    }
}

#[tokio::test]
async fn test_heartbeat_processor_process_updates_last_seen() {
    let processor = HeartbeatProcessor::new();
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    let initial = session.last_seen();

    // 稍等片刻保证时间推进
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let req = WsBaseReq { r#type: 2, data: json!({}) };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;

    // last_seen 至少应该 >= initial（可能在同一秒内）
    assert!(session.last_seen() >= initial);
}

#[tokio::test]
async fn test_heartbeat_processor_default() {
    let processor = HeartbeatProcessor::default();
    let req = WsBaseReq { r#type: 2, data: json!({}) };
    assert!(processor.supports(&req));
}

// ========================
// DefaultMessageProcessor 测试
// ========================

#[tokio::test]
async fn test_default_processor_supports_all_types() {
    let processor = DefaultMessageProcessor::new();
    for t in [0, 1, 2, 3, 99, -1, i32::MAX] {
        let req = WsBaseReq { r#type: t, data: json!({}) };
        assert!(processor.supports(&req), "DefaultMessageProcessor 应支持所有类型");
    }
}

#[tokio::test]
async fn test_default_processor_process_does_not_panic() {
    let processor = DefaultMessageProcessor::new();
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq { r#type: 999, data: json!({"unknown": true}) };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
    // 只要不 panic 就算通过
}

#[tokio::test]
async fn test_default_processor_default_trait() {
    let processor = DefaultMessageProcessor::default();
    let req = WsBaseReq { r#type: 0, data: json!({}) };
    assert!(processor.supports(&req));
}

// ========================
// VideoProcessor 测试
// ========================

#[tokio::test]
async fn test_video_processor_supports_webrtc_signal() {
    let (_, services, _) = get_services().await;
    let processor = mk_video(services);
    let req = WsBaseReq { r#type: 14, data: json!({}) }; // WebrtcSignal
    assert!(processor.supports(&req));
}

#[tokio::test]
async fn test_video_processor_supports_video_heartbeat() {
    let (_, services, _) = get_services().await;
    let processor = mk_video(services);
    let req = WsBaseReq { r#type: 4, data: json!({}) }; // VideoHeartbeat
    assert!(processor.supports(&req));
}

#[tokio::test]
async fn test_video_processor_rejects_other_types() {
    let (_, services, _) = get_services().await;
    let processor = mk_video(services);
    for t in [1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 17, 18, 19] {
        let req = WsBaseReq { r#type: t, data: json!({}) };
        assert!(!processor.supports(&req), "VideoProcessor 不应支持 type={}", t);
    }
}

#[tokio::test]
async fn test_video_processor_process_signal_valid() {
    let (_, services, _) = get_services().await;
    let processor = mk_video(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 14,
        data: json!({
            "target_uid": 1002,
            "room_id": 100,
            "signal": "offer_sdp_data",
            "signal_type": "offer"
        }),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_video_processor_process_signal_invalid_data() {
    let (_, services, _) = get_services().await;
    let processor = mk_video(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 14,
        data: json!({"invalid": "data"}),
    };
    // 不应 panic
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_video_processor_process_heartbeat_valid() {
    let (_, services, _) = get_services().await;
    let processor = mk_video(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 4,
        data: json!({"room_id": 100}),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_video_processor_process_heartbeat_invalid() {
    let (_, services, _) = get_services().await;
    let processor = mk_video(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 4,
        data: json!({"bad_field": "value"}),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_video_processor_default() {
    let (_, services, _) = get_services().await;
    let processor = mk_video(services);
    let req = WsBaseReq { r#type: 14, data: json!({}) };
    assert!(processor.supports(&req));
}

// ========================
// VideoCallProcessor 测试
// ========================

#[tokio::test]
async fn test_video_call_processor_supports_request() {
    let (_, services, _) = get_services().await;
    let processor = mk_video_call(services);
    let req = WsBaseReq { r#type: 5, data: json!({}) }; // VideoCallRequest
    assert!(processor.supports(&req));
}

#[tokio::test]
async fn test_video_call_processor_supports_response() {
    let (_, services, _) = get_services().await;
    let processor = mk_video_call(services);
    let req = WsBaseReq { r#type: 6, data: json!({}) }; // VideoCallResponse
    assert!(processor.supports(&req));
}

#[tokio::test]
async fn test_video_call_processor_rejects_other_types() {
    let (_, services, _) = get_services().await;
    let processor = mk_video_call(services);
    for t in [1, 2, 3, 4, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19] {
        let req = WsBaseReq { r#type: t, data: json!({}) };
        assert!(!processor.supports(&req));
    }
}

#[tokio::test]
async fn test_video_call_processor_process_request_valid() {
    let (_, services, _) = get_services().await;
    let processor = mk_video_call(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 5,
        data: json!({
            "target_uid": 1002,
            "room_id": 200,
            "is_video": true
        }),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_video_call_processor_process_request_invalid() {
    let (_, services, _) = get_services().await;
    let processor = mk_video_call(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 5,
        data: json!({"bad": true}),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_video_call_processor_process_response_valid() {
    let (_, services, _) = get_services().await;
    let processor = mk_video_call(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 6,
        data: json!({
            "caller_uid": 1001,
            "room_id": 200,
            "accepted": 1
        }),
    };
    processor.process(&session, &"s1".to_string(), 1002, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_video_call_processor_process_response_rejected() {
    let (_, services, _) = get_services().await;
    let processor = mk_video_call(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 6,
        data: json!({
            "caller_uid": 1001,
            "room_id": 200,
            "accepted": 0
        }),
    };
    processor.process(&session, &"s1".to_string(), 1002, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_video_call_processor_process_response_invalid() {
    let (_, services, _) = get_services().await;
    let processor = mk_video_call(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 6,
        data: json!({"invalid": 123}),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_video_call_processor_default() {
    let (_, services, _) = get_services().await;
    let processor = mk_video_call(services);
    assert!(processor.supports(&WsBaseReq { r#type: 5, data: json!({}) }));
}

// ========================
// MediaControlProcessor 测试
// ========================

#[tokio::test]
async fn test_media_control_processor_supports_mute_audio() {
    let (_, services, _) = get_services().await;
    let processor = mk_media_control(services);
    let req = WsBaseReq { r#type: 7, data: json!({}) }; // MediaMuteAudio
    assert!(processor.supports(&req));
}

#[tokio::test]
async fn test_media_control_processor_supports_mute_video() {
    let (_, services, _) = get_services().await;
    let processor = mk_media_control(services);
    let req = WsBaseReq { r#type: 8, data: json!({}) }; // MediaMuteVideo
    assert!(processor.supports(&req));
}

#[tokio::test]
async fn test_media_control_processor_rejects_other_types() {
    let (_, services, _) = get_services().await;
    let processor = mk_media_control(services);
    for t in [1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19] {
        let req = WsBaseReq { r#type: t, data: json!({}) };
        assert!(!processor.supports(&req));
    }
}

#[tokio::test]
async fn test_media_control_processor_process_valid() {
    let (_, services, _) = get_services().await;
    let processor = mk_media_control(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 7,
        data: json!({
            "room_id": 100,
            "audio_muted": true,
            "video_muted": false
        }),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_media_control_processor_process_invalid() {
    let (_, services, _) = get_services().await;
    let processor = mk_media_control(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 7,
        data: json!({"wrong": "data"}),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_media_control_processor_default() {
    let (_, services, _) = get_services().await;
    let processor = mk_media_control(services);
    assert!(processor.supports(&WsBaseReq { r#type: 7, data: json!({}) }));
}

// ========================
// RoomAdminProcessor 测试
// ========================

#[tokio::test]
async fn test_room_admin_processor_supports_close_room() {
    let (_, services, _) = get_services().await;
    let processor = mk_room_admin(services);
    assert!(processor.supports(&WsBaseReq { r#type: 11, data: json!({}) })); // CloseRoom
}

#[tokio::test]
async fn test_room_admin_processor_supports_kick_user() {
    let (_, services, _) = get_services().await;
    let processor = mk_room_admin(services);
    assert!(processor.supports(&WsBaseReq { r#type: 12, data: json!({}) })); // KickUser
}

#[tokio::test]
async fn test_room_admin_processor_supports_mute_all() {
    let (_, services, _) = get_services().await;
    let processor = mk_room_admin(services);
    assert!(processor.supports(&WsBaseReq { r#type: 9, data: json!({}) })); // MediaMuteAll
}

#[tokio::test]
async fn test_room_admin_processor_rejects_other_types() {
    let (_, services, _) = get_services().await;
    let processor = mk_room_admin(services);
    for t in [1, 2, 3, 4, 5, 6, 7, 8, 10, 13, 14, 15, 16, 17, 18, 19] {
        let req = WsBaseReq { r#type: t, data: json!({}) };
        assert!(!processor.supports(&req));
    }
}

#[tokio::test]
async fn test_room_admin_processor_close_room_valid() {
    let (_, services, _) = get_services().await;
    let processor = mk_room_admin(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 11,
        data: json!({"room_id": 100}),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_room_admin_processor_close_room_invalid() {
    let (_, services, _) = get_services().await;
    let processor = mk_room_admin(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 11,
        data: json!({"wrong": "field"}),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_room_admin_processor_kick_user_valid() {
    let (_, services, _) = get_services().await;
    let processor = mk_room_admin(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 12,
        data: json!({
            "room_id": 100,
            "target_uid": 1002,
            "reason": "违规"
        }),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_room_admin_processor_kick_user_without_reason() {
    let (_, services, _) = get_services().await;
    let processor = mk_room_admin(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 12,
        data: json!({
            "room_id": 100,
            "target_uid": 1002,
            "reason": null
        }),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_room_admin_processor_kick_user_invalid() {
    let (_, services, _) = get_services().await;
    let processor = mk_room_admin(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 12,
        data: json!({"bad": "data"}),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_room_admin_processor_mute_all_valid() {
    let (_, services, _) = get_services().await;
    let processor = mk_room_admin(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 9,
        data: json!({
            "room_id": 100,
            "muted": true
        }),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_room_admin_processor_mute_all_unmute() {
    let (_, services, _) = get_services().await;
    let processor = mk_room_admin(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 9,
        data: json!({
            "room_id": 100,
            "muted": false
        }),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_room_admin_processor_mute_all_invalid() {
    let (_, services, _) = get_services().await;
    let processor = mk_room_admin(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 9,
        data: json!({"invalid": true}),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_room_admin_processor_default() {
    let (_, services, _) = get_services().await;
    let processor = mk_room_admin(services);
    assert!(processor.supports(&WsBaseReq { r#type: 11, data: json!({}) }));
}

// ========================
// QualityMonitorProcessor 测试
// ========================

#[tokio::test]
async fn test_quality_monitor_supports_network_report() {
    let (_, services, _) = get_services().await;
    let processor = mk_quality_monitor(services);
    assert!(processor.supports(&WsBaseReq { r#type: 13, data: json!({}) })); // NetworkReport
}

#[tokio::test]
async fn test_quality_monitor_supports_screen_sharing() {
    let (_, services, _) = get_services().await;
    let processor = mk_quality_monitor(services);
    assert!(processor.supports(&WsBaseReq { r#type: 10, data: json!({}) })); // ScreenSharing
}

#[tokio::test]
async fn test_quality_monitor_rejects_other_types() {
    let (_, services, _) = get_services().await;
    let processor = mk_quality_monitor(services);
    for t in [1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 14, 15, 16, 17, 18, 19] {
        let req = WsBaseReq { r#type: t, data: json!({}) };
        assert!(!processor.supports(&req));
    }
}

#[tokio::test]
async fn test_quality_monitor_network_report_valid() {
    let (_, services, _) = get_services().await;
    let processor = mk_quality_monitor(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 13,
        data: json!({
            "room_id": 100,
            "quality": 0.95
        }),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_quality_monitor_network_report_poor_quality() {
    let (_, services, _) = get_services().await;
    let processor = mk_quality_monitor(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 13,
        data: json!({
            "room_id": 100,
            "quality": 0.1
        }),
    };
    // 质量 < 0.3，会触发通知逻辑（目前只是 TODO）
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_quality_monitor_network_report_invalid() {
    let (_, services, _) = get_services().await;
    let processor = mk_quality_monitor(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 13,
        data: json!({"bad": "data"}),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_quality_monitor_screen_sharing_start() {
    let (_, services, _) = get_services().await;
    let processor = mk_quality_monitor(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 10,
        data: json!({
            "room_id": 100,
            "sharing": true
        }),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_quality_monitor_screen_sharing_stop() {
    let (_, services, _) = get_services().await;
    let processor = mk_quality_monitor(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 10,
        data: json!({
            "room_id": 100,
            "sharing": false
        }),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_quality_monitor_screen_sharing_invalid() {
    let (_, services, _) = get_services().await;
    let processor = mk_quality_monitor(services);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let req = WsBaseReq {
        r#type: 10,
        data: json!({"wrong_field": true}),
    };
    processor.process(&session, &"s1".to_string(), 1001, &"d1".to_string(), req).await;
}

#[tokio::test]
async fn test_quality_monitor_default() {
    let (_, services, _) = get_services().await;
    let processor = mk_quality_monitor(services);
    assert!(processor.supports(&WsBaseReq { r#type: 13, data: json!({}) }));
}

// ========================
// AckProcessor supports 测试
// ========================

// 注意：AckProcessor::process() 需要 AppState（Kafka Producer），无法在单元测试中调用
// 但可以测试 supports() 方法

#[test]
fn test_ack_processor_supports_type() {
    // AckProcessor 需要 Arc<AppState> 构造，这里使用 WsMsgTypeEnum::Ack.eq() 逻辑验证
    // 由于无法构造 AppState，直接验证 WsMsgTypeEnum::Ack 的值
    use ms_websocket::enums::WsMsgTypeEnum;
    assert!(WsMsgTypeEnum::Ack.eq(15));
    assert!(!WsMsgTypeEnum::Ack.eq(1));
    assert!(!WsMsgTypeEnum::Ack.eq(16));
}

// ========================
// ReadProcessor supports 测试
// ========================

#[test]
fn test_read_processor_supports_type() {
    use ms_websocket::enums::WsMsgTypeEnum;
    assert!(WsMsgTypeEnum::Read.eq(16));
    assert!(!WsMsgTypeEnum::Read.eq(1));
    assert!(!WsMsgTypeEnum::Read.eq(15));
}

// ========================
// Session 结构体测试
// ========================

#[tokio::test]
async fn test_session_new() {
    let session = create_test_session("test_id".to_string(), 42, "device_1".to_string());
    assert_eq!(session.id, "test_id");
    assert_eq!(session.uid, 42);
    assert_eq!(session.client_id, "device_1");
    assert!(session.last_seen() > 0);
}

#[tokio::test]
async fn test_session_touch() {
    let session = create_test_session("s1".to_string(), 1, "d1".to_string());
    let before = session.last_seen();
    // touch 不保证在同一秒内改变值，但不应 panic
    session.touch();
    assert!(session.last_seen() >= before);
}

#[tokio::test]
async fn test_session_send_success() {
    let (session, mut rx, _srx) = create_test_session_with_rx("s1".to_string(), 1, "d1".to_string());
    let msg = axum::extract::ws::Message::Text("hello".to_string().into());
    session.send(msg).await.unwrap();
    let received = rx.recv().await.unwrap();
    match received {
        axum::extract::ws::Message::Text(text) => assert_eq!(text.to_string(), "hello"),
        _ => panic!("应该收到 Text 消息"),
    }
}

#[tokio::test]
async fn test_session_send_failure_when_rx_dropped() {
    let session = create_test_session("s1".to_string(), 1, "d1".to_string());
    // rx 已在 create_test_session 中被丢弃
    let msg = axum::extract::ws::Message::Text("hello".to_string().into());
    let result = session.send(msg).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_session_try_send_success() {
    let (session, mut rx, _srx) = create_test_session_with_rx("s1".to_string(), 1, "d1".to_string());
    let msg = axum::extract::ws::Message::Text("hello".to_string().into());
    session.try_send(msg).unwrap();
    let received = rx.recv().await.unwrap();
    match received {
        axum::extract::ws::Message::Text(text) => assert_eq!(text.to_string(), "hello"),
        _ => panic!("应该收到 Text 消息"),
    }
}

#[tokio::test]
async fn test_session_try_send_failure_when_rx_dropped() {
    let session = create_test_session("s1".to_string(), 1, "d1".to_string());
    let msg = axum::extract::ws::Message::Text("hello".to_string().into());
    let result = session.try_send(msg);
    assert!(result.is_err());
}

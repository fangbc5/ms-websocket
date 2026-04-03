/// 模型模块测试
///
/// 覆盖：WsBaseReq, WsBaseResp, 所有 DTO, 所有 VO, Room 实体
use serde_json::json;

// ========================
// WsBaseReq / WsBaseResp 测试
// ========================

mod ws_base {
    use super::*;
    use ms_websocket::model::ws_base_resp::{WsBaseReq, WsBaseResp};

    #[test]
    fn test_ws_base_req_deserialize() {
        let json = r#"{"type": 1, "data": {"key": "value"}}"#;
        let req: WsBaseReq = serde_json::from_str(json).unwrap();
        assert_eq!(req.r#type, 1);
        assert_eq!(req.data["key"], "value");
    }

    #[test]
    fn test_ws_base_req_deserialize_empty_data() {
        let json = r#"{"type": 2, "data": {}}"#;
        let req: WsBaseReq = serde_json::from_str(json).unwrap();
        assert_eq!(req.r#type, 2);
        assert!(req.data.is_object());
    }

    #[test]
    fn test_ws_base_req_deserialize_null_data() {
        let json = r#"{"type": 3, "data": null}"#;
        let req: WsBaseReq = serde_json::from_str(json).unwrap();
        assert_eq!(req.r#type, 3);
        assert!(req.data.is_null());
    }

    #[test]
    fn test_ws_base_req_deserialize_array_data() {
        let json = r#"{"type": 4, "data": [1, 2, 3]}"#;
        let req: WsBaseReq = serde_json::from_str(json).unwrap();
        assert_eq!(req.r#type, 4);
        assert!(req.data.is_array());
    }

    #[test]
    fn test_ws_base_req_serialize() {
        let req = WsBaseReq {
            r#type: 1,
            data: json!({"msg_id": 123}),
        };
        let json_str = serde_json::to_string(&req).unwrap();
        assert!(json_str.contains("\"type\":1"));
    }

    #[test]
    fn test_ws_base_req_clone() {
        let req = WsBaseReq {
            r#type: 5,
            data: json!({"test": true}),
        };
        let cloned = req.clone();
        assert_eq!(cloned.r#type, 5);
        assert_eq!(cloned.data["test"], true);
    }

    #[test]
    fn test_ws_base_req_debug() {
        let req = WsBaseReq {
            r#type: 1,
            data: json!({}),
        };
        let debug_str = format!("{:?}", req);
        assert!(debug_str.contains("WsBaseReq"));
    }

    #[test]
    fn test_ws_base_resp_new() {
        let resp = WsBaseResp::new(1, json!({"success": true}));
        assert_eq!(resp.r#type, 1);
        assert_eq!(resp.data["success"], true);
    }

    #[test]
    fn test_ws_base_resp_from_data() {
        #[derive(serde::Serialize)]
        struct TestData {
            value: i32,
            name: String,
        }

        let data = TestData {
            value: 42,
            name: "test".to_string(),
        };
        let resp = WsBaseResp::from_data(10, data).unwrap();
        assert_eq!(resp.r#type, 10);
        assert_eq!(resp.data["value"], 42);
        assert_eq!(resp.data["name"], "test");
    }

    #[test]
    fn test_ws_base_resp_from_data_simple_types() {
        let resp = WsBaseResp::from_data(1, "hello").unwrap();
        assert_eq!(resp.data, json!("hello"));

        let resp = WsBaseResp::from_data(2, 42i32).unwrap();
        assert_eq!(resp.data, json!(42));

        let resp = WsBaseResp::from_data(3, true).unwrap();
        assert_eq!(resp.data, json!(true));
    }

    #[test]
    fn test_ws_base_resp_serialize() {
        let resp = WsBaseResp::new(1, json!({"key": "value"}));
        let json_str = serde_json::to_string(&resp).unwrap();
        let deserialized: WsBaseResp = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.r#type, 1);
        assert_eq!(deserialized.data["key"], "value");
    }

    #[test]
    fn test_ws_base_resp_clone() {
        let resp = WsBaseResp::new(1, json!({"msg": "hello"}));
        let cloned = resp.clone();
        assert_eq!(cloned.r#type, resp.r#type);
        assert_eq!(cloned.data, resp.data);
    }

    #[test]
    fn test_ws_base_resp_debug() {
        let resp = WsBaseResp::new(1, json!({}));
        let debug_str = format!("{:?}", resp);
        assert!(debug_str.contains("WsBaseResp"));
    }
}

// ========================
// Room 实体测试
// ========================

mod room {
    use chrono::Utc;
    use ms_websocket::model::entity::Room;

    #[test]
    fn test_room_get_type() {
        let room = create_test_room();
        assert_eq!(room.get_type(), 1);
    }

    #[test]
    fn test_room_get_id() {
        let room = create_test_room();
        assert_eq!(room.get_id(), 100);
    }

    #[test]
    fn test_room_get_tenant_id() {
        let room = create_test_room();
        assert_eq!(room.get_tenant_id(), 1000);
    }

    #[test]
    fn test_room_serialize_deserialize() {
        let room = create_test_room();
        let json = serde_json::to_string(&room).unwrap();
        let deserialized: Room = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, room.id);
        assert_eq!(deserialized.room_type, room.room_type);
        assert_eq!(deserialized.tenant_id, room.tenant_id);
    }

    #[test]
    fn test_room_clone() {
        let room = create_test_room();
        let cloned = room.clone();
        assert_eq!(cloned.id, room.id);
        assert_eq!(cloned.room_type, room.room_type);
    }

    #[test]
    fn test_room_debug() {
        let room = create_test_room();
        let debug_str = format!("{:?}", room);
        assert!(debug_str.contains("Room"));
    }

    fn create_test_room() -> Room {
        let now = Utc::now();
        Room {
            id: 100,
            room_type: 1,
            hot_flag: 0,
            active_time: now,
            last_msg_id: 0,
            ext_json: None,
            create_time: now,
            create_by: 1001,
            update_time: now,
            update_by: 1001,
            is_del: 0,
            tenant_id: 1000,
        }
    }
}

// ========================
// DTO 测试
// ========================

mod dto {
    use super::*;
    use ms_websocket::model::dto::AckMessageDto;
    use ms_websocket::model::dto::NodePushDTO;
    use ms_websocket::model::dto::ReadMessageDto;
    use ms_websocket::model::dto::RouterPushDto;
    use ms_websocket::model::dto::login_message_dto::LoginMessageDTO;
    use ms_websocket::model::dto::scan_success_message_dto::ScanSuccessMessageDTO;
    use ms_websocket::model::ws_base_resp::WsBaseResp;

    #[test]
    fn test_ack_message_dto_serialize_deserialize() {
        let dto = AckMessageDto {
            uid: Some(1001),
            msg_id: 12345,
        };
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: AckMessageDto = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uid, Some(1001));
        assert_eq!(deserialized.msg_id, 12345);
    }

    #[test]
    fn test_ack_message_dto_uid_none() {
        let dto = AckMessageDto {
            uid: None,
            msg_id: 99999,
        };
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: AckMessageDto = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uid, None);
        assert_eq!(deserialized.msg_id, 99999);
    }

    #[test]
    fn test_login_message_dto_serialize_deserialize() {
        let dto = LoginMessageDTO {
            uid: 2001,
            code: 200,
        };
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: LoginMessageDTO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uid, 2001);
        assert_eq!(deserialized.code, 200);
    }

    #[test]
    fn test_node_push_dto_serialize_deserialize() {
        let mut device_user_map = std::collections::HashMap::new();
        device_user_map.insert("device1".to_string(), 1001u64);
        device_user_map.insert("device2".to_string(), 1002u64);

        let dto = NodePushDTO {
            ws_base_msg: WsBaseResp::new(1, json!({"msg": "hello"})),
            device_user_map,
            hash_id: 99,
            uid: 1001,
        };
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: NodePushDTO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.hash_id, 99);
        assert_eq!(deserialized.uid, 1001);
        assert_eq!(deserialized.device_user_map.len(), 2);
    }

    #[test]
    fn test_read_message_dto_serialize_deserialize() {
        let dto = ReadMessageDto {
            uid: Some(1001),
            room_id: 100,
            msg_ids: vec![1, 2, 3],
        };
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: ReadMessageDto = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uid, Some(1001));
        assert_eq!(deserialized.room_id, 100);
        assert_eq!(deserialized.msg_ids, vec![1, 2, 3]);
    }

    #[test]
    fn test_read_message_dto_uid_none() {
        let dto = ReadMessageDto {
            uid: None,
            room_id: 200,
            msg_ids: vec![],
        };
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: ReadMessageDto = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uid, None);
        assert!(deserialized.msg_ids.is_empty());
    }

    #[test]
    fn test_router_push_dto_serialize_deserialize() {
        let dto = RouterPushDto {
            ws_base_msg: WsBaseResp::new(2, json!({"data": "test"})),
            uid_list: vec![1001, 1002, 1003],
            uid: 999,
        };
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: RouterPushDto = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uid, 999);
        assert_eq!(deserialized.uid_list.len(), 3);
    }

    #[test]
    fn test_scan_success_message_dto_serialize_deserialize() {
        let dto = ScanSuccessMessageDTO { code: 200 };
        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: ScanSuccessMessageDTO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.code, 200);
    }

    #[test]
    fn test_all_dtos_debug() {
        let ack = AckMessageDto { uid: Some(1), msg_id: 1 };
        assert!(format!("{:?}", ack).contains("AckMessageDto"));

        let login = LoginMessageDTO { uid: 1, code: 200 };
        assert!(format!("{:?}", login).contains("LoginMessageDTO"));

        let scan = ScanSuccessMessageDTO { code: 200 };
        assert!(format!("{:?}", scan).contains("ScanSuccessMessageDTO"));

        let read = ReadMessageDto { uid: None, room_id: 1, msg_ids: vec![] };
        assert!(format!("{:?}", read).contains("ReadMessageDto"));
    }

    #[test]
    fn test_all_dtos_clone() {
        let ack = AckMessageDto { uid: Some(1), msg_id: 1 };
        let _ = ack.clone();

        let login = LoginMessageDTO { uid: 1, code: 200 };
        let _ = login.clone();

        let scan = ScanSuccessMessageDTO { code: 200 };
        let _ = scan.clone();

        let read = ReadMessageDto { uid: None, room_id: 1, msg_ids: vec![] };
        let _ = read.clone();
    }
}

// ========================
// VO 测试
// ========================

mod vo {
    use super::*;
    use ms_websocket::model::vo::{
        all_muted_vo::AllMutedVO,
        call_accepted_vo::CallAcceptedVO,
        call_rejected_vo::CallRejectedVO,
        call_req_vo::CallReqVO,
        call_request_vo::CallRequestVO,
        call_response_vo::CallResponseVO,
        call_timeout_vo::CallTimeoutVO,
        heartbeat_req::HeartbeatReq,
        media_control_vo::MediaControlVO,
        network_quality_vo::NetworkQualityVO,
        room_closed_vo::RoomClosedVO,
        screen_sharing_vo::ScreenSharingVO,
        start_signaling_vo::StartSignalingVO,
        user_join_room_vo::UserJoinRoomVO,
        user_kicked_vo::UserKickedVO,
        video_signal_vo::VideoSignalVO,
    };

    #[test]
    fn test_all_muted_vo() {
        let vo = AllMutedVO {
            room_id: 100,
            muted: true,
            operator_id: 1001,
        };
        let json = serde_json::to_string(&vo).unwrap();
        let deserialized: AllMutedVO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.room_id, 100);
        assert!(deserialized.muted);
        assert_eq!(deserialized.operator_id, 1001);
    }

    #[test]
    fn test_call_accepted_vo() {
        let vo = CallAcceptedVO {
            accepted_by: 1001,
            room_id: 200,
            token: "test_token".to_string(),
            livekit_url: "wss://livekit.example.com".to_string(),
        };
        let json = serde_json::to_string(&vo).unwrap();
        let deserialized: CallAcceptedVO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.accepted_by, 1001);
        assert_eq!(deserialized.room_id, 200);
    }

    #[test]
    fn test_call_rejected_vo() {
        let vo = CallRejectedVO {
            rejected_by: 1002,
        };
        let json = serde_json::to_string(&vo).unwrap();
        let deserialized: CallRejectedVO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.rejected_by, 1002);
    }

    #[test]
    fn test_call_req_vo() {
        let vo = CallReqVO {
            caller_uid: 1001,
            target_uid: 1002,
            room_id: 300,
            is_video: true,
        };
        let json = serde_json::to_string(&vo).unwrap();
        let deserialized: CallReqVO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.caller_uid, 1001);
        assert_eq!(deserialized.target_uid, 1002);
        assert_eq!(deserialized.room_id, 300);
        assert!(deserialized.is_video);
    }

    #[test]
    fn test_call_request_vo() {
        let vo = CallRequestVO {
            target_uid: 2001,
            room_id: 400,
            is_video: false,
        };
        let json = serde_json::to_string(&vo).unwrap();
        let deserialized: CallRequestVO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.target_uid, 2001);
        assert_eq!(deserialized.room_id, 400);
        assert!(!deserialized.is_video);
    }

    #[test]
    fn test_call_response_vo() {
        // 测试所有响应状态
        for status in [-1, 0, 1, 2] {
            let vo = CallResponseVO {
                caller_uid: 1001,
                room_id: 500,
                accepted: status,
            };
            let json = serde_json::to_string(&vo).unwrap();
            let deserialized: CallResponseVO = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.accepted, status);
        }
    }

    #[test]
    fn test_call_timeout_vo() {
        let vo = CallTimeoutVO {
            target_uid: 3001,
        };
        let json = serde_json::to_string(&vo).unwrap();
        let deserialized: CallTimeoutVO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.target_uid, 3001);
    }

    #[test]
    fn test_heartbeat_req() {
        let vo = HeartbeatReq { room_id: 600 };
        let json = serde_json::to_string(&vo).unwrap();
        let deserialized: HeartbeatReq = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.room_id, 600);
    }

    #[test]
    fn test_media_control_vo() {
        let vo = MediaControlVO {
            room_id: 700,
            audio_muted: true,
            video_muted: false,
        };
        let json = serde_json::to_string(&vo).unwrap();
        let deserialized: MediaControlVO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.room_id, 700);
        assert!(deserialized.audio_muted);
        assert!(!deserialized.video_muted);
    }

    #[test]
    fn test_network_quality_vo_new() {
        let vo = NetworkQualityVO::new(800, 1001, 0.95);
        assert_eq!(vo.room_id, 800);
        assert_eq!(vo.user_id, 1001);
        assert!((vo.quality - 0.95).abs() < f64::EPSILON);
        assert!(vo.timestamp > 0);
    }

    #[test]
    fn test_network_quality_vo_serialize() {
        let vo = NetworkQualityVO {
            room_id: 800,
            user_id: 1001,
            quality: 0.85,
            timestamp: 1234567890,
        };
        let json = serde_json::to_string(&vo).unwrap();
        let deserialized: NetworkQualityVO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.room_id, 800);
        assert!((deserialized.quality - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_room_closed_vo() {
        let vo = RoomClosedVO {
            room_id: 900,
            reason: "超时关闭".to_string(),
        };
        let json = serde_json::to_string(&vo).unwrap();
        let deserialized: RoomClosedVO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.room_id, 900);
        assert_eq!(deserialized.reason, "超时关闭");
    }

    #[test]
    fn test_screen_sharing_vo() {
        let vo = ScreenSharingVO {
            room_id: 1000,
            user_id: 1001,
            sharing: true,
        };
        let json = serde_json::to_string(&vo).unwrap();
        let deserialized: ScreenSharingVO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.room_id, 1000);
        assert!(deserialized.sharing);
    }

    #[test]
    fn test_start_signaling_vo() {
        let vo = StartSignalingVO { room_id: 1100 };
        let json = serde_json::to_string(&vo).unwrap();
        let deserialized: StartSignalingVO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.room_id, 1100);
    }

    #[test]
    fn test_user_join_room_vo_new() {
        let vo = UserJoinRoomVO::new(1001, 1200);
        assert_eq!(vo.uid, 1001);
        assert_eq!(vo.room_id, 1200);
        assert!(vo.name.is_none());
        assert!(vo.avatar.is_none());
        assert!(vo.timestamp > 0);
    }

    #[test]
    fn test_user_join_room_vo_with_name_and_avatar() {
        let vo = UserJoinRoomVO::new(1001, 1200)
            .with_name("张三".to_string())
            .with_avatar("https://example.com/avatar.png".to_string());
        assert_eq!(vo.name, Some("张三".to_string()));
        assert_eq!(vo.avatar, Some("https://example.com/avatar.png".to_string()));
    }

    #[test]
    fn test_user_join_room_vo_serialize_skip_none() {
        let vo = UserJoinRoomVO::new(1001, 1200);
        let json_str = serde_json::to_string(&vo).unwrap();
        // name 和 avatar 为 None 时不应序列化
        assert!(!json_str.contains("name"));
        assert!(!json_str.contains("avatar"));
    }

    #[test]
    fn test_user_join_room_vo_serialize_with_values() {
        let vo = UserJoinRoomVO::new(1001, 1200)
            .with_name("test".to_string());
        let json_str = serde_json::to_string(&vo).unwrap();
        assert!(json_str.contains("name"));
        assert!(json_str.contains("test"));
    }

    #[test]
    fn test_user_kicked_vo() {
        let vo = UserKickedVO {
            room_id: 1300,
            kicked_uid: 2001,
            operator_id: 1001,
            reason: "违规".to_string(),
        };
        let json = serde_json::to_string(&vo).unwrap();
        let deserialized: UserKickedVO = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.room_id, 1300);
        assert_eq!(deserialized.kicked_uid, 2001);
        assert_eq!(deserialized.operator_id, 1001);
        assert_eq!(deserialized.reason, "违规");
    }

    #[test]
    fn test_video_signal_vo_new() {
        let vo = VideoSignalVO::new(1001, 1400, "offer".to_string(), "sdp_data".to_string());
        assert_eq!(vo.sender_id, 1001);
        assert_eq!(vo.room_id, 1400);
        assert_eq!(vo.signal_type, "offer");
        assert_eq!(vo.signal, "sdp_data");
        assert!(vo.timestamp.is_some());
        assert!(vo.timestamp.unwrap() > 0);
    }

    #[test]
    fn test_video_signal_vo_serialize_timestamp_skip() {
        let vo = VideoSignalVO {
            sender_id: 1001,
            room_id: 1400,
            signal: "test".to_string(),
            signal_type: "candidate".to_string(),
            timestamp: None,
        };
        let json_str = serde_json::to_string(&vo).unwrap();
        assert!(!json_str.contains("timestamp"));
    }

    // 所有 VO 的 Clone 和 Debug 测试
    #[test]
    fn test_all_vos_clone_and_debug() {
        let _ = AllMutedVO { room_id: 1, muted: true, operator_id: 1 }.clone();
        let _ = CallAcceptedVO { accepted_by: 1, room_id: 1, token: "t".to_string(), livekit_url: "u".to_string() }.clone();
        let _ = CallRejectedVO { rejected_by: 1 }.clone();
        let _ = CallReqVO { caller_uid: 1, target_uid: 2, room_id: 1, is_video: true }.clone();
        let _ = CallRequestVO { target_uid: 1, room_id: 1, is_video: true }.clone();
        let _ = CallResponseVO { caller_uid: 1, room_id: 1, accepted: 1 }.clone();
        let _ = CallTimeoutVO { target_uid: 1 }.clone();
        let _ = HeartbeatReq { room_id: 1 }.clone();
        let _ = MediaControlVO { room_id: 1, audio_muted: true, video_muted: false }.clone();
        let _ = NetworkQualityVO::new(1, 1, 0.5).clone();
        let _ = RoomClosedVO { room_id: 1, reason: "test".to_string() }.clone();
        let _ = ScreenSharingVO { room_id: 1, user_id: 1, sharing: true }.clone();
        let _ = StartSignalingVO { room_id: 1 }.clone();
        let _ = UserJoinRoomVO::new(1, 1).clone();
        let _ = UserKickedVO { room_id: 1, kicked_uid: 1, operator_id: 1, reason: "test".to_string() }.clone();
        let _ = VideoSignalVO::new(1, 1, "offer".to_string(), "sdp".to_string()).clone();

        // Debug 测试
        assert!(format!("{:?}", AllMutedVO { room_id: 1, muted: true, operator_id: 1 }).contains("AllMutedVO"));
        assert!(format!("{:?}", HeartbeatReq { room_id: 1 }).contains("HeartbeatReq"));
        assert!(format!("{:?}", RoomClosedVO { room_id: 1, reason: "r".to_string() }).contains("RoomClosedVO"));
    }
}

// ========================
// WSOnlineNotify VO 测试（P1 新增）
// ========================

mod ws_online_notify {
    use ms_websocket::model::vo::ws_online_notify::{
        WSOnlineNotify, NOTIFY_TYPE_FRIEND, NOTIFY_TYPE_GROUP,
    };
    use serde_json;

    #[test]
    fn test_notify_type_constants() {
        assert_eq!(NOTIFY_TYPE_GROUP, 1);
        assert_eq!(NOTIFY_TYPE_FRIEND, 2);
    }

    #[test]
    fn test_friend_notify_constructor() {
        let notify = WSOnlineNotify::friend_notify(1001, "device_abc".to_string(), 1700000000000, 5);
        assert_eq!(notify.uid, 1001);
        assert_eq!(notify.client_id, "device_abc");
        assert!(notify.room_id.is_none());
        assert_eq!(notify.last_opt_time, 1700000000000);
        assert_eq!(notify.online_num, 5);
        assert_eq!(notify.notify_type, NOTIFY_TYPE_FRIEND);
    }

    #[test]
    fn test_group_notify_constructor() {
        let notify = WSOnlineNotify::group_notify(100, 1001, "device_xyz".to_string(), 1700000000000, 10);
        assert_eq!(notify.uid, 1001);
        assert_eq!(notify.client_id, "device_xyz");
        assert_eq!(notify.room_id, Some(100));
        assert_eq!(notify.last_opt_time, 1700000000000);
        assert_eq!(notify.online_num, 10);
        assert_eq!(notify.notify_type, NOTIFY_TYPE_GROUP);
    }

    #[test]
    fn test_friend_notify_serialize_camel_case() {
        let notify = WSOnlineNotify::friend_notify(1001, "dev1".to_string(), 1700000000000, 3);
        let json_str = serde_json::to_string(&notify).unwrap();

        // camelCase 字段名
        assert!(json_str.contains("\"clientId\""));
        assert!(json_str.contains("\"lastOptTime\""));
        assert!(json_str.contains("\"onlineNum\""));
        assert!(json_str.contains("\"notifyType\""));

        // 不应包含 snake_case
        assert!(!json_str.contains("client_id"));
        assert!(!json_str.contains("last_opt_time"));
        assert!(!json_str.contains("online_num"));
        assert!(!json_str.contains("notify_type"));
    }

    #[test]
    fn test_friend_notify_skip_none_room_id() {
        let notify = WSOnlineNotify::friend_notify(1001, "dev1".to_string(), 1700000000000, 3);
        let json_str = serde_json::to_string(&notify).unwrap();

        // room_id 为 None 时不应序列化
        assert!(!json_str.contains("roomId"));
    }

    #[test]
    fn test_group_notify_includes_room_id() {
        let notify = WSOnlineNotify::group_notify(500, 1001, "dev1".to_string(), 1700000000000, 8);
        let json_str = serde_json::to_string(&notify).unwrap();

        // room_id 为 Some 时应序列化
        assert!(json_str.contains("\"roomId\":500"));
    }

    #[test]
    fn test_friend_notify_deserialize_roundtrip() {
        let original = WSOnlineNotify::friend_notify(1001, "device_123".to_string(), 1700000000000, 5);
        let json_str = serde_json::to_string(&original).unwrap();
        let deserialized: WSOnlineNotify = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.uid, original.uid);
        assert_eq!(deserialized.client_id, original.client_id);
        assert_eq!(deserialized.room_id, original.room_id);
        assert_eq!(deserialized.last_opt_time, original.last_opt_time);
        assert_eq!(deserialized.online_num, original.online_num);
        assert_eq!(deserialized.notify_type, original.notify_type);
    }

    #[test]
    fn test_group_notify_deserialize_roundtrip() {
        let original = WSOnlineNotify::group_notify(200, 2002, "device_456".to_string(), 1700000001000, 15);
        let json_str = serde_json::to_string(&original).unwrap();
        let deserialized: WSOnlineNotify = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.uid, original.uid);
        assert_eq!(deserialized.client_id, original.client_id);
        assert_eq!(deserialized.room_id, Some(200));
        assert_eq!(deserialized.online_num, 15);
        assert_eq!(deserialized.notify_type, NOTIFY_TYPE_GROUP);
    }

    #[test]
    fn test_notify_clone() {
        let original = WSOnlineNotify::friend_notify(1001, "dev".to_string(), 123, 1);
        let cloned = original.clone();
        assert_eq!(cloned.uid, 1001);
        assert_eq!(cloned.client_id, "dev");
    }

    #[test]
    fn test_notify_debug() {
        let notify = WSOnlineNotify::friend_notify(1001, "dev".to_string(), 123, 1);
        let debug = format!("{:?}", notify);
        assert!(debug.contains("WSOnlineNotify"));
    }

    #[test]
    fn test_notify_edge_case_zero_online() {
        let notify = WSOnlineNotify::friend_notify(1001, "dev".to_string(), 0, 0);
        assert_eq!(notify.online_num, 0);
        assert_eq!(notify.last_opt_time, 0);
    }

    #[test]
    fn test_notify_edge_case_large_uid() {
        let notify = WSOnlineNotify::group_notify(u64::MAX, u64::MAX, "d".to_string(), i64::MAX, i64::MAX);
        assert_eq!(notify.uid, u64::MAX);
        assert_eq!(notify.room_id, Some(u64::MAX));
        assert_eq!(notify.online_num, i64::MAX);
    }
}

// ========================
// CallEndReq DTO 测试（P0 新增）
// ========================

mod call_end_req {
    use ms_websocket::model::dto::call_end_req::CallEndReq;
    use serde_json;

    #[test]
    fn test_call_end_req_new_end() {
        let req = CallEndReq::new_end(
            Some(1001),
            100,
            Some(1),
            true,
            Some(true),
            Some(2001),
            Some(1700000000000),
            "COMPLETED".to_string(),
        );
        assert!(!req.begin);
        assert_eq!(req.uid, Some(1001));
        assert_eq!(req.room_id, 100);
        assert_eq!(req.tenant_id, Some(1));
        assert_eq!(req.is_group, Some(true));
        assert_eq!(req.medium_type, Some(true));
        assert_eq!(req.creator, Some(2001));
        assert_eq!(req.start_time, Some(1700000000000));
        assert!(req.end_time.is_some()); // auto-set
        assert_eq!(req.state, "COMPLETED");
    }

    #[test]
    fn test_call_end_req_new_start() {
        let req = CallEndReq::new_start(
            1001,
            Some(2001),
            200,
            Some(1),
            "RINGING".to_string(),
        );
        assert!(req.begin);
        assert_eq!(req.uid, Some(1001));
        assert_eq!(req.room_id, 200);
        assert_eq!(req.tenant_id, Some(1));
        assert_eq!(req.creator, Some(2001));
        assert!(req.start_time.is_some()); // auto-set
        assert!(req.end_time.is_none());
        assert_eq!(req.state, "RINGING");
    }

    #[test]
    fn test_call_end_req_serialize_deserialize() {
        let original = CallEndReq::new_end(
            Some(1001),
            100,
            None,
            false,
            None,
            None,
            None,
            "TIMEOUT".to_string(),
        );
        let json_str = serde_json::to_string(&original).unwrap();
        let deserialized: CallEndReq = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.begin, original.begin);
        assert_eq!(deserialized.uid, original.uid);
        assert_eq!(deserialized.room_id, original.room_id);
        assert_eq!(deserialized.state, original.state);
    }

    #[test]
    fn test_call_end_req_clone_and_debug() {
        let req = CallEndReq::new_start(1001, None, 100, None, "ONGOING".to_string());
        let cloned = req.clone();
        assert_eq!(cloned.state, "ONGOING");
        let debug = format!("{:?}", req);
        assert!(debug.contains("CallEndReq"));
    }

    #[test]
    fn test_call_end_req_end_time_auto_set() {
        let before = chrono::Utc::now().timestamp_millis();
        let req = CallEndReq::new_end(None, 100, None, false, None, None, None, "X".to_string());
        let after = chrono::Utc::now().timestamp_millis();

        let end_time = req.end_time.unwrap();
        assert!(end_time >= before);
        assert!(end_time <= after);
    }

    #[test]
    fn test_call_end_req_start_time_auto_set() {
        let before = chrono::Utc::now().timestamp_millis();
        let req = CallEndReq::new_start(1001, None, 100, None, "X".to_string());
        let after = chrono::Utc::now().timestamp_millis();

        let start_time = req.start_time.unwrap();
        assert!(start_time >= before);
        assert!(start_time <= after);
    }
}

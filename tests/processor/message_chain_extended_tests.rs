/// 消息处理链扩展测试
///
/// 测试 MessageHandlerChain 和完整处理器链的集成场景
use crate::common::*;
use async_trait::async_trait;
use ms_websocket::model::ws_base_resp::WsBaseReq;
use ms_websocket::types::{ClientId, SessionId, UserId};
use ms_websocket::websocket::processor::{MessageHandlerChain, MessageProcessor};
use ms_websocket::websocket::Session;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

// ========================
// 辅助工具
// ========================

/// 计数器处理器：记录被调用的次数
struct CountingProcessor {
    supports_type: i32,
    call_count: Arc<AtomicU32>,
    processed: Arc<AtomicBool>,
}

impl CountingProcessor {
    fn new(supports_type: i32) -> Self {
        Self {
            supports_type,
            call_count: Arc::new(AtomicU32::new(0)),
            processed: Arc::new(AtomicBool::new(false)),
        }
    }

    fn count(&self) -> u32 {
        self.call_count.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl MessageProcessor for CountingProcessor {
    fn supports(&self, req: &WsBaseReq) -> bool {
        req.r#type == self.supports_type
    }

    async fn process(
        &self,
        _session: &Arc<Session>,
        _session_id: &SessionId,
        _uid: UserId,
        _client_id: &ClientId,
        _req: WsBaseReq,
    ) {
        self.call_count.fetch_add(1, Ordering::Relaxed);
        self.processed.store(true, Ordering::Relaxed);
    }
}

/// 始终支持的处理器（模拟 DefaultMessageProcessor）
struct AlwaysSupportProcessor {
    call_count: Arc<AtomicU32>,
}

impl AlwaysSupportProcessor {
    fn new() -> Self {
        Self {
            call_count: Arc::new(AtomicU32::new(0)),
        }
    }

    fn count(&self) -> u32 {
        self.call_count.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl MessageProcessor for AlwaysSupportProcessor {
    fn supports(&self, _req: &WsBaseReq) -> bool {
        true
    }

    async fn process(
        &self,
        _session: &Arc<Session>,
        _session_id: &SessionId,
        _uid: UserId,
        _client_id: &ClientId,
        _req: WsBaseReq,
    ) {
        self.call_count.fetch_add(1, Ordering::Relaxed);
    }
}

// ========================
// 处理链路由测试
// ========================

#[tokio::test]
async fn test_chain_routes_to_correct_processor() {
    let p1 = Arc::new(CountingProcessor::new(1));
    let p2 = Arc::new(CountingProcessor::new(2));
    let p3 = Arc::new(CountingProcessor::new(3));

    let chain = MessageHandlerChain::new(vec![
        p1.clone() as Arc<dyn MessageProcessor>,
        p2.clone() as Arc<dyn MessageProcessor>,
        p3.clone() as Arc<dyn MessageProcessor>,
    ]);

    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    // 发送 type=3 的消息
    chain
        .handle_message(
            &session,
            &"s1".to_string(),
            1001,
            &"d1".to_string(),
            r#"{"type": 3, "data": {}}"#,
        )
        .await;

    assert_eq!(p1.count(), 0);
    assert_eq!(p2.count(), 0);
    assert_eq!(p3.count(), 1);
}

#[tokio::test]
async fn test_chain_multiple_messages_different_types() {
    let p1 = Arc::new(CountingProcessor::new(1));
    let p2 = Arc::new(CountingProcessor::new(2));
    let fallback = Arc::new(AlwaysSupportProcessor::new());

    let chain = MessageHandlerChain::new(vec![
        p1.clone() as Arc<dyn MessageProcessor>,
        p2.clone() as Arc<dyn MessageProcessor>,
        fallback.clone() as Arc<dyn MessageProcessor>,
    ]);

    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    // 发送多条不同类型的消息
    chain
        .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), r#"{"type": 1, "data": {}}"#)
        .await;
    chain
        .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), r#"{"type": 2, "data": {}}"#)
        .await;
    chain
        .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), r#"{"type": 99, "data": {}}"#)
        .await;

    assert_eq!(p1.count(), 1);
    assert_eq!(p2.count(), 1);
    assert_eq!(fallback.count(), 1); // type=99 被 fallback 处理
}

#[tokio::test]
async fn test_chain_same_type_called_multiple_times() {
    let p = Arc::new(CountingProcessor::new(2));

    let chain = MessageHandlerChain::new(vec![p.clone() as Arc<dyn MessageProcessor>]);

    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    // 连续发送同一类型的消息
    for _ in 0..10 {
        chain
            .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), r#"{"type": 2, "data": {}}"#)
            .await;
    }

    assert_eq!(p.count(), 10);
}

// ========================
// 并发消息处理测试
// ========================

#[tokio::test]
async fn test_chain_concurrent_message_handling() {
    let p1 = Arc::new(CountingProcessor::new(1));
    let p2 = Arc::new(CountingProcessor::new(2));

    let chain = Arc::new(MessageHandlerChain::new(vec![
        p1.clone() as Arc<dyn MessageProcessor>,
        p2.clone() as Arc<dyn MessageProcessor>,
    ]));

    let session = Arc::new(create_test_session("s1".to_string(), 1001, "d1".to_string()));

    let mut handles = vec![];

    // 并发发送 100 条消息（50 条 type=1, 50 条 type=2）
    for i in 0..100 {
        let chain = chain.clone();
        let session = session.clone();
        let t = if i % 2 == 0 { 1 } else { 2 };
        let payload = format!(r#"{{"type": {}, "data": {{}}}}"#, t);

        let handle = tokio::spawn(async move {
            chain
                .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), &payload)
                .await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(p1.count(), 50);
    assert_eq!(p2.count(), 50);
}

// ========================
// 各种 JSON 负载测试
// ========================

#[tokio::test]
async fn test_chain_with_complex_data_payload() {
    let p = Arc::new(CountingProcessor::new(1));
    let chain = MessageHandlerChain::new(vec![p.clone() as Arc<dyn MessageProcessor>]);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    // 复杂的 data 字段
    let payload = r#"{"type": 1, "data": {"nested": {"key": "value"}, "array": [1, 2, 3], "number": 42.5, "bool": true, "null_val": null}}"#;
    chain
        .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), payload)
        .await;

    assert_eq!(p.count(), 1);
}

#[tokio::test]
async fn test_chain_with_empty_data() {
    let p = Arc::new(CountingProcessor::new(1));
    let chain = MessageHandlerChain::new(vec![p.clone() as Arc<dyn MessageProcessor>]);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    chain
        .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), r#"{"type": 1, "data": null}"#)
        .await;

    assert_eq!(p.count(), 1);
}

#[tokio::test]
async fn test_chain_with_string_data() {
    let p = Arc::new(CountingProcessor::new(1));
    let chain = MessageHandlerChain::new(vec![p.clone() as Arc<dyn MessageProcessor>]);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    chain
        .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), r#"{"type": 1, "data": "hello"}"#)
        .await;

    assert_eq!(p.count(), 1);
}

#[tokio::test]
async fn test_chain_with_negative_type() {
    let p = Arc::new(CountingProcessor::new(-1));
    let chain = MessageHandlerChain::new(vec![p.clone() as Arc<dyn MessageProcessor>]);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    chain
        .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), r#"{"type": -1, "data": {}}"#)
        .await;

    assert_eq!(p.count(), 1);
}

#[tokio::test]
async fn test_chain_with_zero_type() {
    let p = Arc::new(CountingProcessor::new(0));
    let chain = MessageHandlerChain::new(vec![p.clone() as Arc<dyn MessageProcessor>]);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    chain
        .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), r#"{"type": 0, "data": {}}"#)
        .await;

    assert_eq!(p.count(), 1);
}

// ========================
// 无效 JSON 处理
// ========================

#[tokio::test]
async fn test_chain_malformed_json() {
    let p = Arc::new(CountingProcessor::new(1));
    let chain = MessageHandlerChain::new(vec![p.clone() as Arc<dyn MessageProcessor>]);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    // 各种无效 JSON
    let invalid_payloads = vec![
        "",
        "not json at all",
        "{",
        r#"{"type": }"#,
        r#"{"type": "string_type", "data": {}}"#, // type 应该是 i32
        r#"{"data": {}}"#,                         // 缺少 type
        r#"[]"#,                                    // 数组而非对象
    ];

    for payload in invalid_payloads {
        chain
            .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), payload)
            .await;
    }

    // 所有无效 JSON 都不应触发处理器
    assert_eq!(p.count(), 0);
}

// ========================
// 大链测试
// ========================

#[tokio::test]
async fn test_chain_with_many_processors() {
    let mut processors: Vec<Arc<dyn MessageProcessor>> = Vec::new();
    let mut target_processor = None;

    // 创建 100 个处理器，目标处理器在最后
    for i in 0..100 {
        let p = Arc::new(CountingProcessor::new(i));
        if i == 99 {
            target_processor = Some(p.clone());
        }
        processors.push(p as Arc<dyn MessageProcessor>);
    }

    let chain = MessageHandlerChain::new(processors);
    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    // 发送给最后一个处理器
    chain
        .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), r#"{"type": 99, "data": {}}"#)
        .await;

    assert_eq!(target_processor.unwrap().count(), 1);
}

#[tokio::test]
async fn test_chain_fallback_is_last_resort() {
    let p1 = Arc::new(CountingProcessor::new(1));
    let p2 = Arc::new(CountingProcessor::new(2));
    let fallback = Arc::new(AlwaysSupportProcessor::new());

    let chain = MessageHandlerChain::new(vec![
        p1.clone() as Arc<dyn MessageProcessor>,
        p2.clone() as Arc<dyn MessageProcessor>,
        fallback.clone() as Arc<dyn MessageProcessor>,
    ]);

    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    // type=1 应该由 p1 处理（不触及 fallback）
    chain
        .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), r#"{"type": 1, "data": {}}"#)
        .await;

    assert_eq!(p1.count(), 1);
    assert_eq!(fallback.count(), 0);

    // type=99 应该落到 fallback
    chain
        .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), r#"{"type": 99, "data": {}}"#)
        .await;

    assert_eq!(fallback.count(), 1);
}

// ========================
// 使用真实处理器的集成测试
// ========================

#[tokio::test]
async fn test_real_heartbeat_and_default_chain() {
    use ms_websocket::websocket::processor::{DefaultMessageProcessor, HeartbeatProcessor};

    let chain = MessageHandlerChain::new(vec![
        Arc::new(HeartbeatProcessor::new()) as Arc<dyn MessageProcessor>,
        Arc::new(DefaultMessageProcessor::new()) as Arc<dyn MessageProcessor>,
    ]);

    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    // 心跳消息（type=2）应该被 HeartbeatProcessor 处理
    chain
        .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), r#"{"type": 2, "data": {}}"#)
        .await;

    // 未知类型（type=999）应该被 DefaultMessageProcessor 处理
    chain
        .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), r#"{"type": 999, "data": {}}"#)
        .await;

    // 不应 panic，验证整个链路正常运行
}

#[tokio::test]
async fn test_real_all_basic_processors_chain() {
    use ms_websocket::websocket::processor::{DefaultMessageProcessor, HeartbeatProcessor};
    use ms_websocket::websocket::processor::meet::{
        MediaControlProcessor, QualityMonitorProcessor, RoomAdminProcessor, VideoCallProcessor,
        VideoProcessor,
    };

    let (_, services, _) = create_test_services().await;

    let chain = MessageHandlerChain::new(vec![
        Arc::new(HeartbeatProcessor::new()) as Arc<dyn MessageProcessor>,
        Arc::new(VideoProcessor::new(
            services.video_chat_service.clone(),
            services.room_timeout_service.clone(),
        )) as Arc<dyn MessageProcessor>,
        Arc::new(VideoCallProcessor::new(
            services.video_chat_service.clone(),
            services.push_service.clone(),
            services.room_timeout_service.clone(),
        )) as Arc<dyn MessageProcessor>,
        Arc::new(MediaControlProcessor::new(
            services.video_chat_service.clone(),
        )) as Arc<dyn MessageProcessor>,
        Arc::new(QualityMonitorProcessor::new(
            services.video_chat_service.clone(),
            services.push_service.clone(),
        )) as Arc<dyn MessageProcessor>,
        Arc::new(RoomAdminProcessor::new(
            services.video_chat_service.clone(),
            services.push_service.clone(),
            services.room_timeout_service.clone(),
        )) as Arc<dyn MessageProcessor>,
        Arc::new(DefaultMessageProcessor::new()) as Arc<dyn MessageProcessor>,
    ]);

    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());

    // 测试所有消息类型都能被正确路由而不 panic
    let test_messages = vec![
        (2, r#"{"type": 2, "data": {}}"#),                                          // Heartbeat
        (14, r#"{"type": 14, "data": {"target_uid": 1002, "room_id": 100}}"#),      // WebrtcSignal
        (4, r#"{"type": 4, "data": {"room_id": 100}}"#),                            // VideoHeartbeat
        (5, r#"{"type": 5, "data": {"target_uid": 1002}}"#),                        // VideoCallRequest
        (6, r#"{"type": 6, "data": {"caller_uid": 1001, "accepted": 1}}"#),         // VideoCallResponse
        (7, r#"{"type": 7, "data": {"room_id": 100, "audio_muted": true}}"#),       // MediaMuteAudio
        (13, r#"{"type": 13, "data": {"room_id": 100, "quality": 0.9}}"#),          // NetworkReport
        (10, r#"{"type": 10, "data": {"room_id": 100, "sharing": true}}"#),         // ScreenSharing
        (11, r#"{"type": 11, "data": {"room_id": 100}}"#),                          // CloseRoom
        (12, r#"{"type": 12, "data": {"room_id": 100, "target_uid": 1002}}"#),      // KickUser
        (9, r#"{"type": 9, "data": {"room_id": 100, "muted": true}}"#),             // MediaMuteAll
        (999, r#"{"type": 999, "data": {}}"#),                                       // Unknown -> Default
    ];

    for (msg_type, payload) in test_messages {
        chain
            .handle_message(&session, &"s1".to_string(), 1001, &"d1".to_string(), payload)
            .await;
        // 每条消息都不应 panic
    }
}

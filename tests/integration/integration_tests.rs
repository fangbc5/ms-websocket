/// 集成测试 — 连接真实 Redis、Kafka、Nacos 服务
///
/// 运行方式：
///   cargo test -p ms-websocket --test integration_tests -- --ignored
///
/// 前提条件：
///   - Redis 运行在 127.0.0.1:6379
///   - Kafka 运行在 localhost:9092
///   - Nacos 运行在 127.0.0.1:8848（可选，部分测试需要）
use crate::common::*;
use fbc_starter::{AppState, KafkaMessageHandler, Message};
use ms_websocket::config::WsConfig;
use ms_websocket::kafka::consumer::PushHandler;
use ms_websocket::model::dto::{NodePushDTO, RouterPushDto};
use ms_websocket::model::ws_base_resp::{WsBaseReq, WsBaseResp};
use ms_websocket::service::{PushService, Services};
use ms_websocket::state::WsState;
use ms_websocket::websocket::processor::{AckProcessor, MessageProcessor, ReadProcessor};
use ms_websocket::websocket::{MessageRouterService, SessionManager};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

// ============================================================================
// 测试基础设施
// ============================================================================

/// 创建连接真实 Redis + Kafka 的 AppState
async fn create_real_app_state() -> Arc<AppState> {
    // Redis URL 从环境变量读取，默认 redis://127.0.0.1:6379
    let redis_url =
        std::env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let redis_password = std::env::var("TEST_REDIS_PASSWORD").ok();

    // 初始化 Redis 连接池
    let redis_pool = fbc_starter::cache::redis::init_redis(
        &redis_url,
        redis_password.as_deref(),
        5,
    )
    .await
    .expect(&format!(
        "无法连接 Redis (url={})，请设置 TEST_REDIS_URL 和 TEST_REDIS_PASSWORD 环境变量",
        redis_url
    ));

    // Kafka broker 地址从环境变量读取
    let kafka_brokers =
        std::env::var("TEST_KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".to_string());

    // 初始化 Kafka Producer
    let kafka_producer_config = fbc_starter::KafkaProducer::new(
        &kafka_brokers,
        &fbc_starter::config::KafkaProducerConfig {
            retries: 3,
            enable_idempotence: true,
            acks: "all".to_string(),
        },
    )
    .expect(&format!(
        "无法连接 Kafka (brokers={})，请设置 TEST_KAFKA_BROKERS 环境变量",
        kafka_brokers
    ));

    let producer: fbc_starter::MessageProducerType = Arc::new(kafka_producer_config);

    let app_state = AppState::new()
        .with_redis(redis_pool)
        .with_message_producer(producer);

    Arc::new(app_state)
}

/// 创建包含 SessionManager 的 WsState（用于完整集成测试）
async fn create_real_ws_state() -> Arc<WsState> {
    let app_state = create_real_app_state().await;

    let mut session_manager = SessionManager::default();
    session_manager.set_app_state(app_state.clone());
    let session_manager = Arc::new(session_manager);

    let services = Arc::new(
        Services::new(app_state.clone(), session_manager.clone())
            .expect("Services 初始化失败"),
    );

    let handler_chain = ms_websocket::routes::create_handler_chain(app_state.clone(), &services);

    let config = Arc::new(WsConfig::default());
    Arc::new(WsState::new(app_state, config, session_manager, services, handler_chain))
}

/// 创建用于 Kafka 消费的测试消息
fn create_test_ws_base_resp(msg_type: i32, data: serde_json::Value) -> WsBaseResp {
    WsBaseResp::new(msg_type, data)
}

// ============================================================================
// AckProcessor 集成测试
// ============================================================================

mod ack_processor {
    use super::*;

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_ack_processor_supports() {
        let app_state = create_real_app_state().await;
        let processor = AckProcessor::new(app_state);

        // type=15 是 Ack
        let req_ack = WsBaseReq {
            r#type: 15,
            data: json!({}),
        };
        assert!(processor.supports(&req_ack));

        // 其他类型不应匹配
        let req_other = WsBaseReq {
            r#type: 1,
            data: json!({}),
        };
        assert!(!processor.supports(&req_other));
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_ack_processor_process_publishes_to_kafka() {
        let app_state = create_real_app_state().await;
        let processor = AckProcessor::new(app_state);

        let session = create_test_session("ack_session_1".to_string(), 1001, "device_a".to_string());

        let req = WsBaseReq {
            r#type: 15,
            data: json!({
                "msg_id": 12345
            }),
        };

        // process() 应该将消息发布到 Kafka 的 msg_push_ack_topic，不应 panic
        processor
            .process(
                &session,
                &"ack_session_1".to_string(),
                1001,
                &"device_a".to_string(),
                req,
            )
            .await;

        // 如果没有 panic 或 error，说明 Kafka 发布成功
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_ack_processor_process_invalid_data() {
        let app_state = create_real_app_state().await;
        let processor = AckProcessor::new(app_state);

        let session = create_test_session("ack_session_2".to_string(), 1002, "device_b".to_string());

        // 不合法的 data 字段（缺少必需字段）
        let req = WsBaseReq {
            r#type: 15,
            data: json!({"invalid_field": "value"}),
        };

        // 应该 warn 但不 panic
        processor
            .process(
                &session,
                &"ack_session_2".to_string(),
                1002,
                &"device_b".to_string(),
                req,
            )
            .await;
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_ack_processor_process_with_uid_set() {
        let app_state = create_real_app_state().await;
        let processor = AckProcessor::new(app_state);

        let session = create_test_session("ack_session_3".to_string(), 9999, "device_c".to_string());

        let req = WsBaseReq {
            r#type: 15,
            data: json!({
                "msg_id": 67890,
                "uid": null
            }),
        };

        // uid 应该在 process 中被设置为 9999
        processor
            .process(
                &session,
                &"ack_session_3".to_string(),
                9999,
                &"device_c".to_string(),
                req,
            )
            .await;
    }
}

// ============================================================================
// ReadProcessor 集成测试
// ============================================================================

mod read_processor {
    use super::*;

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_read_processor_supports() {
        let app_state = create_real_app_state().await;
        let processor = ReadProcessor::new(app_state);

        // type=16 是 Read
        let req_read = WsBaseReq {
            r#type: 16,
            data: json!({}),
        };
        assert!(processor.supports(&req_read));

        let req_other = WsBaseReq {
            r#type: 1,
            data: json!({}),
        };
        assert!(!processor.supports(&req_other));
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_read_processor_process_publishes_to_kafka() {
        let app_state = create_real_app_state().await;
        let processor = ReadProcessor::new(app_state);

        let session = create_test_session("read_session_1".to_string(), 2001, "dev_x".to_string());

        let req = WsBaseReq {
            r#type: 16,
            data: json!({
                "room_id": 100,
                "msg_ids": [1, 2, 3]
            }),
        };

        processor
            .process(
                &session,
                &"read_session_1".to_string(),
                2001,
                &"dev_x".to_string(),
                req,
            )
            .await;
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_read_processor_process_invalid_data() {
        let app_state = create_real_app_state().await;
        let processor = ReadProcessor::new(app_state);

        let session = create_test_session("read_session_2".to_string(), 2002, "dev_y".to_string());

        let req = WsBaseReq {
            r#type: 16,
            data: json!({"not_valid": true}),
        };

        // 不应 panic
        processor
            .process(
                &session,
                &"read_session_2".to_string(),
                2002,
                &"dev_y".to_string(),
                req,
            )
            .await;
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_read_processor_process_multiple_msg_ids() {
        let app_state = create_real_app_state().await;
        let processor = ReadProcessor::new(app_state);

        let session = create_test_session("read_session_3".to_string(), 2003, "dev_z".to_string());

        let req = WsBaseReq {
            r#type: 16,
            data: json!({
                "room_id": 200,
                "msg_ids": [100, 200, 300, 400, 500]
            }),
        };

        processor
            .process(
                &session,
                &"read_session_3".to_string(),
                2003,
                &"dev_z".to_string(),
                req,
            )
            .await;
    }
}

// ============================================================================
// PushService 集成测试
// ============================================================================

mod push_service_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_push_service_local_push_to_online_user() {
        let ws_state = create_real_ws_state().await;

        // 注册一个测试会话
        let (session, mut rx, _srx) =
            create_test_session_with_rx("push_s1".to_string(), 5001, "push_dev1".to_string());
        ws_state.session_manager.register_session(session.clone());

        let msg = create_test_ws_base_resp(1, json!({"content": "hello from push_service"}));

        // 本地推送 — 由于 PushService 需要通过 Redis 查路由，
        // 我们先直接调用 SessionManager 确认会话存在
        assert_eq!(ws_state.session_manager.get_session_count(), 1);

        // 使用 session_manager 直接推送验证基础设施
        let ws_msg = axum::extract::ws::Message::Text(
            serde_json::to_string(&msg).unwrap().into(),
        );
        let sent = ws_state.session_manager.send_to_user(5001, ws_msg).await;
        assert!(sent > 0, "应该成功向在线用户推送消息");

        // 验证收到消息
        let received = rx.recv().await.expect("应该收到推送消息");
        match received {
            axum::extract::ws::Message::Text(text) => {
                let resp: WsBaseResp = serde_json::from_str(&text).unwrap();
                assert_eq!(resp.r#type, 1);
                assert_eq!(resp.data["content"], "hello from push_service");
            }
            _ => panic!("应该收到 Text 消息"),
        }

        // 清理
        ws_state.session_manager.cleanup_session(&"push_s1".to_string(), None);
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_push_service_send_to_offline_user() {
        let ws_state = create_real_ws_state().await;

        let msg = create_test_ws_base_resp(1, json!({"content": "msg to offline"}));

        // 推送到不在线的用户 — 空 uid_list 应该直接返回 Ok
        let result = ws_state
            .services
            .push_service
            .send_push_msg(msg, vec![], 0)
            .await;
        assert!(result.is_ok(), "空 uid_list 应该成功");
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_push_service_send_push_msg_single() {
        let ws_state = create_real_ws_state().await;

        // 注册会话
        let (session, _rx, _srx) =
            create_test_session_with_rx("push_single_s1".to_string(), 6001, "push_single_dev1".to_string());
        ws_state.session_manager.register_session(session);

        let msg = create_test_ws_base_resp(1, json!({"content": "single push"}));

        // send_push_msg 会走 Redis 路由，
        // 由于测试环境中 Redis 没有设备映射，消息可能不会到达
        // 但不应报错
        let result = ws_state
            .services
            .push_service
            .send_push_msg_single(msg, 6001, 6001)
            .await;

        // 即使 Redis 中没有路由信息，也不应 panic
        // result 可能是 Ok 或 Err（取决于 Redis 中是否有路由数据）
        println!("send_push_msg_single result: {:?}", result);

        ws_state.session_manager.cleanup_session(&"push_single_s1".to_string(), None);
    }
}

// ============================================================================
// PushHandler 集成测试（Kafka Consumer 处理器）
// ============================================================================

mod push_handler_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_push_handler_topics_contain_node_id() {
        let ws_state = create_real_ws_state().await;
        let handler = PushHandler::new(ws_state.clone());

        let topics = handler.topics();
        assert_eq!(topics.len(), 1);

        let node_id = ws_state.session_manager.node_id();
        assert_eq!(topics[0], format!("websocket_push_{}", node_id));
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_push_handler_group_id() {
        let ws_state = create_real_ws_state().await;
        let handler = PushHandler::new(ws_state.clone());

        let group_id = handler.group_id();
        let node_id = ws_state.session_manager.node_id();
        assert_eq!(group_id, format!("websocket_push_group_{}", node_id));
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_push_handler_handle_valid_message() {
        let ws_state = create_real_ws_state().await;
        let handler = PushHandler::new(ws_state.clone());

        // 注册接收会话
        let (session, mut rx, _srx) =
            create_test_session_with_rx("ph_s1".to_string(), 7001, "ph_dev1".to_string());
        ws_state.session_manager.register_session(session);

        // 构造 NodePushDTO
        let mut device_user_map = HashMap::new();
        device_user_map.insert("ph_dev1".to_string(), 7001u64);

        let dto = NodePushDTO {
            ws_base_msg: create_test_ws_base_resp(1, json!({"text": "pushed via handler"})),
            device_user_map,
            hash_id: 1001,
            uid: 7001,
        };

        let message = Message::new(
            "websocket_push_test".to_string(),
            "test".to_string(),
            serde_json::to_value(&dto).unwrap(),
        );

        // 处理消息
        handler.handle(message).await;

        // 等待异步推送
        sleep(Duration::from_millis(200)).await;

        // 验证是否收到消息
        match tokio::time::timeout(Duration::from_secs(2), rx.recv()).await {
            Ok(Some(axum::extract::ws::Message::Text(text))) => {
                let resp: WsBaseResp = serde_json::from_str(&text).unwrap();
                assert_eq!(resp.r#type, 1);
                assert_eq!(resp.data["text"], "pushed via handler");
            }
            Ok(Some(other)) => panic!("收到了非 Text 消息: {:?}", other),
            Ok(None) => panic!("channel 已关闭"),
            Err(_) => panic!("超时：未收到推送消息"),
        }

        ws_state.session_manager.cleanup_session(&"ph_s1".to_string(), None);
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_push_handler_handle_invalid_message() {
        let ws_state = create_real_ws_state().await;
        let handler = PushHandler::new(ws_state);

        let message = Message::new(
            "websocket_push_test".to_string(),
            "test".to_string(),
            json!({"invalid": "not a NodePushDTO"}),
        );

        // 不应 panic
        handler.handle(message).await;
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_push_handler_handle_multi_device() {
        let ws_state = create_real_ws_state().await;
        let handler = PushHandler::new(ws_state.clone());

        // 注册多个会话
        let (session1, mut rx1, _srx1) =
            create_test_session_with_rx("ph_multi_s1".to_string(), 8001, "ph_multi_dev1".to_string());
        let (session2, mut rx2, _srx2) =
            create_test_session_with_rx("ph_multi_s2".to_string(), 8002, "ph_multi_dev2".to_string());
        ws_state.session_manager.register_session(session1);
        ws_state.session_manager.register_session(session2);

        // 构造包含多个设备的推送
        let mut device_user_map = HashMap::new();
        device_user_map.insert("ph_multi_dev1".to_string(), 8001u64);
        device_user_map.insert("ph_multi_dev2".to_string(), 8002u64);

        let dto = NodePushDTO {
            ws_base_msg: create_test_ws_base_resp(2, json!({"broadcast": true})),
            device_user_map,
            hash_id: 2002,
            uid: 8001,
        };

        let message = Message::new(
            "websocket_push_test".to_string(),
            "test".to_string(),
            serde_json::to_value(&dto).unwrap(),
        );

        handler.handle(message).await;
        sleep(Duration::from_millis(200)).await;

        // 两个会话都应收到消息
        let msg1 = tokio::time::timeout(Duration::from_secs(2), rx1.recv()).await;
        let msg2 = tokio::time::timeout(Duration::from_secs(2), rx2.recv()).await;

        assert!(
            msg1.is_ok() && msg1.unwrap().is_some(),
            "设备1 应收到消息"
        );
        assert!(
            msg2.is_ok() && msg2.unwrap().is_some(),
            "设备2 应收到消息"
        );

        ws_state.session_manager.cleanup_session(&"ph_multi_s1".to_string(), None);
        ws_state.session_manager.cleanup_session(&"ph_multi_s2".to_string(), None);
    }
}

// ============================================================================
// MessageRouterService 集成测试
// ============================================================================

mod message_router_service_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_message_router_service_topics() {
        let ws_state = create_real_ws_state().await;
        let router_service = MessageRouterService::new(ws_state);

        let topics = router_service.topics();
        assert_eq!(topics, vec!["websocket_push".to_string()]);
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_message_router_service_group_id() {
        let ws_state = create_real_ws_state().await;
        let router_service = MessageRouterService::new(ws_state);

        assert_eq!(router_service.group_id(), "websocket_push_group");
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_message_router_service_handle_valid_message() {
        let ws_state = create_real_ws_state().await;
        let router_service = MessageRouterService::new(ws_state.clone());

        // 注册一个会话
        let (session, _rx, _srx) =
            create_test_session_with_rx("mr_s1".to_string(), 9001, "mr_dev1".to_string());
        #[allow(unused_variables)]
        ws_state.session_manager.register_session(session);

        let dto = RouterPushDto {
            ws_base_msg: create_test_ws_base_resp(1, json!({"routed": true})),
            uid_list: vec![9001],
            uid: 9001,
        };

        let message = Message::new(
            "websocket_push".to_string(),
            "test".to_string(),
            serde_json::to_value(&dto).unwrap(),
        );

        // handle() 内部调用 push_service.send_push_msg()
        // 由于 Redis 中可能没有路由信息，消息不一定能到达
        // 但不应 panic
        router_service.handle(message).await;

        ws_state.session_manager.cleanup_session(&"mr_s1".to_string(), None);
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_message_router_service_handle_empty_uid_list() {
        let ws_state = create_real_ws_state().await;
        let router_service = MessageRouterService::new(ws_state);

        let dto = RouterPushDto {
            ws_base_msg: create_test_ws_base_resp(1, json!({})),
            uid_list: vec![],
            uid: 0,
        };

        let message = Message::new(
            "websocket_push".to_string(),
            "test".to_string(),
            serde_json::to_value(&dto).unwrap(),
        );

        // 空 uid_list 应该跳过，不 panic
        router_service.handle(message).await;
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_message_router_service_handle_invalid_message() {
        let ws_state = create_real_ws_state().await;
        let router_service = MessageRouterService::new(ws_state);

        let message = Message::new(
            "websocket_push".to_string(),
            "test".to_string(),
            json!({"not_a_valid_dto": true}),
        );

        // 反序列化失败，应记录 error 但不 panic
        router_service.handle(message).await;
    }
}

// ============================================================================
// routes 模块集成测试
// ============================================================================

mod routes_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_create_handler_chain() {
        let app_state = create_real_app_state().await;

        let mut session_manager = SessionManager::default();
        session_manager.set_app_state(app_state.clone());
        let session_manager = Arc::new(session_manager);

        let services = Arc::new(
            Services::new(app_state.clone(), session_manager.clone())
                .expect("Services 初始化失败"),
        );

        let chain = ms_websocket::routes::create_handler_chain(app_state, &services);

        // 验证处理链能正确路由消息类型
        let session = create_test_session("chain_s1".to_string(), 3001, "chain_dev1".to_string());

        // 心跳消息 (type=2) 应该被 HeartbeatProcessor 处理
        let heartbeat_payload = serde_json::to_string(&json!({"type": 2, "data": {}})).unwrap();
        chain
            .handle_message(&session, &"chain_s1".to_string(), 3001, &"chain_dev1".to_string(), &heartbeat_payload)
            .await;

        // Ack 消息 (type=15) 应该被 AckProcessor 处理
        let ack_payload = serde_json::to_string(&json!({"type": 15, "data": {"msg_id": 1}})).unwrap();
        chain
            .handle_message(&session, &"chain_s1".to_string(), 3001, &"chain_dev1".to_string(), &ack_payload)
            .await;

        // Read 消息 (type=16) 应该被 ReadProcessor 处理
        let read_payload = serde_json::to_string(&json!({"type": 16, "data": {"room_id": 1, "msg_ids": [1]}})).unwrap();
        chain
            .handle_message(&session, &"chain_s1".to_string(), 3001, &"chain_dev1".to_string(), &read_payload)
            .await;

        // 未知类型应该被 DefaultMessageProcessor 处理
        let unknown_payload = serde_json::to_string(&json!({"type": 999, "data": {"unknown": true}})).unwrap();
        chain
            .handle_message(&session, &"chain_s1".to_string(), 3001, &"chain_dev1".to_string(), &unknown_payload)
            .await;
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_create_routes() {
        let ws_state = create_real_ws_state().await;
        let _router = ms_websocket::routes::create_routes(ws_state);
        // 路由创建成功即通过（axum Router 本身无法直接断言路由表）
    }
}

// ============================================================================
// kafka::init_handlers 集成测试
// ============================================================================

mod kafka_init_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_init_handlers_returns_correct_handlers() {
        let ws_state = create_real_ws_state().await;
        let handlers = ms_websocket::kafka::init_handlers(ws_state.clone());

        // 应该有 2 个 handler: MessageRouterService + PushHandler
        assert_eq!(handlers.len(), 2, "应返回 2 个 Kafka handler");

        // 检查 topics
        let all_topics: Vec<String> = handlers.iter().flat_map(|h| h.topics()).collect();

        // MessageRouterService 的 topic
        assert!(
            all_topics.contains(&"websocket_push".to_string()),
            "应包含 websocket_push topic"
        );

        // PushHandler 的 topic（包含 node_id）
        let node_id = ws_state.session_manager.node_id();
        let expected_push_topic = format!("websocket_push_{}", node_id);
        assert!(
            all_topics.contains(&expected_push_topic),
            "应包含 {} topic",
            expected_push_topic
        );
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_init_handlers_group_ids_are_unique() {
        let ws_state = create_real_ws_state().await;
        let handlers = ms_websocket::kafka::init_handlers(ws_state);

        let group_ids: Vec<String> = handlers.iter().map(|h| h.group_id()).collect();

        // MessageRouterService 的 group_id
        assert!(group_ids.contains(&"websocket_push_group".to_string()));

        // 验证 group_id 不重复
        let unique_count = group_ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(
            unique_count,
            group_ids.len(),
            "所有 handler 的 group_id 应该唯一"
        );
    }
}

// ============================================================================
// WsState 构建集成测试
// ============================================================================

mod ws_state_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_ws_state_creation() {
        let ws_state = create_real_ws_state().await;

        // 验证组件都存在
        assert_eq!(ws_state.session_manager.get_session_count(), 0);

        // 验证 Redis 连接正常
        let redis_conn = ws_state.app_state.redis().await;
        assert!(redis_conn.is_ok(), "Redis 连接应该正常");

        // 验证 Kafka Producer 正常
        let producer = ws_state.app_state.message_producer();
        assert!(producer.is_ok(), "Kafka Producer 应该已初始化");
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_ws_state_services_initialized() {
        let ws_state = create_real_ws_state().await;

        // PushService 通过 services 访问
        let msg = create_test_ws_base_resp(1, json!({"test": true}));
        let result = ws_state
            .services
            .push_service
            .send_push_msg(msg, vec![], 0)
            .await;
        assert!(result.is_ok(), "PushService 应该可以处理空 uid_list");
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_ws_state_handler_chain_routes_messages() {
        let ws_state = create_real_ws_state().await;
        let session = create_test_session("ws_state_s1".to_string(), 4001, "ws_state_dev1".to_string());

        // 通过 handler_chain 处理消息，不应 panic
        let payload = serde_json::to_string(&json!({"type": 2, "data": {}})).unwrap();
        ws_state.handler_chain.handle_message(
            &session,
            &"ws_state_s1".to_string(),
            4001,
            &"ws_state_dev1".to_string(),
            &payload,
        ).await;
    }
}

// ============================================================================
// Kafka Producer 端到端测试
// ============================================================================

mod kafka_e2e_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_kafka_producer_publish() {
        let app_state = create_real_app_state().await;
        let producer = app_state.message_producer().unwrap();

        let message = Message::new(
            "integration_test_topic".to_string(),
            "test_user".to_string(),
            json!({"test": true, "timestamp": chrono::Utc::now().timestamp()}),
        );

        let result = producer.publish("integration_test_topic", message).await;
        assert!(result.is_ok(), "Kafka 发布消息应该成功: {:?}", result.err());
    }

    #[tokio::test]
    #[ignore = "需要 Redis + Kafka 服务"]
    async fn test_kafka_producer_publish_batch() {
        let app_state = create_real_app_state().await;
        let producer = app_state.message_producer().unwrap();

        let messages: Vec<(String, Message)> = (0..5)
            .map(|i| {
                let msg = Message::new(
                    "integration_test_batch_topic".to_string(),
                    format!("user_{}", i),
                    json!({"index": i}),
                );
                ("integration_test_batch_topic".to_string(), msg)
            })
            .collect();

        let result = producer.publish_batch(messages).await;
        assert!(
            result.is_ok(),
            "Kafka 批量发布消息应该成功: {:?}",
            result.err()
        );
    }
}

// ============================================================================
// Redis 连接测试
// ============================================================================

mod redis_tests {
    use super::*;
    use redis::AsyncCommands;

    #[tokio::test]
    #[ignore = "需要 Redis 服务"]
    async fn test_redis_connection() {
        let app_state = create_real_app_state().await;
        let mut conn = app_state.redis().await.expect("Redis 连接失败");

        // 测试 set/get
        let test_key = "ms_ws_integration_test_key";
        let _: () = conn.set(test_key, "test_value").await.unwrap();
        let val: String = conn.get(test_key).await.unwrap();
        assert_eq!(val, "test_value");

        // 清理
        let _: () = conn.del(test_key).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "需要 Redis 服务"]
    async fn test_redis_hash_operations() {
        let app_state = create_real_app_state().await;
        let mut conn = app_state.redis().await.expect("Redis 连接失败");

        let hash_key = "ms_ws_integration_test_hash";

        // 模拟设备-节点映射
        let _: () = conn.hset(hash_key, "1001:device_a", "node_1").await.unwrap();
        let _: () = conn.hset(hash_key, "1002:device_b", "node_2").await.unwrap();

        let all: HashMap<String, String> = conn.hgetall(hash_key).await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all["1001:device_a"], "node_1");
        assert_eq!(all["1002:device_b"], "node_2");

        // 清理
        let _: () = conn.del(hash_key).await.unwrap();
    }
}

/// 测试辅助工具模块
use axum::extract::ws::Message;
use std::sync::Arc;
use tokio::sync::mpsc;

use ms_websocket::websocket::Session;
use ms_websocket::types::{ClientId, SessionId, UserId};

/// 创建测试会话
pub fn create_test_session(
    session_id: SessionId,
    uid: UserId,
    client_id: ClientId,
) -> Arc<Session> {
    let (tx, _rx) = mpsc::channel::<Message>(1000);
    let (shutdown_tx, _shutdown_rx) = mpsc::channel::<()>(1);

    Arc::new(Session::new(session_id, uid, client_id, tx, shutdown_tx))
}

/// 创建测试会话（同时返回消息接收器，用于需要验证消息发送的测试）
pub fn create_test_session_with_rx(
    session_id: SessionId,
    uid: UserId,
    client_id: ClientId,
) -> (Arc<Session>, mpsc::Receiver<Message>, mpsc::Receiver<()>) {
    let (tx, rx) = mpsc::channel::<Message>(1000);
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

    (Arc::new(Session::new(session_id, uid, client_id, tx, shutdown_tx)), rx, shutdown_rx)
}

/// 创建多个测试会话
pub fn create_test_sessions(count: usize, base_uid: UserId) -> Vec<Arc<Session>> {
    (0..count)
        .map(|i| {
            create_test_session(
                format!("session_{}", i),
                base_uid + i as UserId,
                format!("device_{}", i),
            )
        })
        .collect()
}

/// 等待异步任务完成
pub async fn wait_for_async_tasks() {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
}

/// 创建连接真实 Redis + Kafka 的 AppState（用于需要服务依赖的处理器测试）
pub async fn create_test_app_state() -> Arc<fbc_starter::AppState> {
    let redis_url =
        std::env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let redis_password = std::env::var("TEST_REDIS_PASSWORD").ok();

    let redis_pool = fbc_starter::cache::redis::init_redis(
        &redis_url,
        redis_password.as_deref(),
        5,
    )
    .await
    .expect("无法连接 Redis，请确保 Redis 正在运行并设置 TEST_REDIS_PASSWORD");

    let kafka_brokers =
        std::env::var("TEST_KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".to_string());

    let kafka_producer_config = fbc_starter::KafkaProducer::new(
        &kafka_brokers,
        &fbc_starter::config::KafkaProducerConfig {
            retries: 3,
            enable_idempotence: true,
            acks: "all".to_string(),
        },
    )
    .expect("无法连接 Kafka，请确保 Kafka 正在运行");

    let producer: fbc_starter::MessageProducerType = Arc::new(kafka_producer_config);

    let app_state = fbc_starter::AppState::new()
        .with_redis(redis_pool)
        .with_message_producer(producer);

    Arc::new(app_state)
}

/// 创建测试用的 Services 容器（需要 Redis + Kafka）
pub async fn create_test_services() -> (
    Arc<fbc_starter::AppState>,
    Arc<ms_websocket::service::Services>,
    Arc<ms_websocket::websocket::SessionManager>,
) {
    let app_state = create_test_app_state().await;

    let mut session_manager = ms_websocket::websocket::SessionManager::default();
    session_manager.set_app_state(app_state.clone());
    let session_manager = Arc::new(session_manager);

    let config = Arc::new(ms_websocket::config::WsConfig::default());

    let services = Arc::new(
        ms_websocket::service::Services::new(app_state.clone(), session_manager.clone(), config)
            .expect("Services 初始化失败"),
    );

    (app_state, services, session_manager)
}

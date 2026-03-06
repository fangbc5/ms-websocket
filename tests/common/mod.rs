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

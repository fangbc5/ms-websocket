/// 会话管理器集成测试
mod common;

use common::*;
use ms_websocket::websocket::SessionManager;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_session_registration() {
    let manager = SessionManager::new();
    let session = create_test_session("session1".to_string(), 1001, "device1".to_string());

    manager.register_session(session.clone());

    assert_eq!(manager.get_session_count(), 1);
    assert_eq!(manager.get_user_sessions(1001).len(), 1);
}

#[tokio::test]
async fn test_session_cleanup() {
    let manager = SessionManager::new();
    let session = create_test_session("session1".to_string(), 1001, "device1".to_string());

    manager.register_session(session.clone());
    assert_eq!(manager.get_session_count(), 1);

    manager.cleanup_session(&"session1".to_string());
    wait_for_async_tasks().await;

    assert_eq!(manager.get_session_count(), 0);
    assert_eq!(manager.get_user_sessions(1001).len(), 0);
}

#[tokio::test]
async fn test_multi_device_sessions() {
    let manager = SessionManager::new();

    // 同一用户，不同设备
    let session1 = create_test_session("session1".to_string(), 1001, "device1".to_string());
    let session2 = create_test_session("session2".to_string(), 1001, "device2".to_string());

    manager.register_session(session1);
    manager.register_session(session2);

    assert_eq!(manager.get_session_count(), 2);
    assert_eq!(manager.get_user_sessions(1001).len(), 2);

    // 获取所有客户端 ID
    let client_ids = manager.get_client_ids();
    assert_eq!(client_ids.len(), 2);
    assert!(client_ids.contains(&"device1".to_string()));
    assert!(client_ids.contains(&"device2".to_string()));
}

#[tokio::test]
async fn test_same_device_multiple_sessions() {
    let manager = SessionManager::new();

    // 同一用户，同一设备，多个会话（例如多个浏览器标签页）
    let session1 = create_test_session("session1".to_string(), 1001, "device1".to_string());
    let session2 = create_test_session("session2".to_string(), 1001, "device1".to_string());

    manager.register_session(session1);
    manager.register_session(session2);

    assert_eq!(manager.get_session_count(), 2);
    assert_eq!(manager.get_user_sessions(1001).len(), 2);
}

#[tokio::test]
async fn test_session_cleanup_multi_device() {
    let manager = SessionManager::new();

    let session1 = create_test_session("session1".to_string(), 1001, "device1".to_string());
    let session2 = create_test_session("session2".to_string(), 1001, "device2".to_string());

    manager.register_session(session1);
    manager.register_session(session2);

    // 清理一个设备的会话
    manager.cleanup_session(&"session1".to_string());
    wait_for_async_tasks().await;

    assert_eq!(manager.get_session_count(), 1);
    assert_eq!(manager.get_user_sessions(1001).len(), 1);

    // 清理最后一个会话
    manager.cleanup_session(&"session2".to_string());
    wait_for_async_tasks().await;

    assert_eq!(manager.get_session_count(), 0);
    assert_eq!(manager.get_user_sessions(1001).len(), 0);
}

#[tokio::test]
async fn test_session_heartbeat_refresh() {
    let manager = SessionManager::new();
    let session = create_test_session("session1".to_string(), 1001, "device1".to_string());

    manager.register_session(session.clone());

    // 记录初始时间
    let initial_last_seen = session.last_seen();

    // 等待超过 1 秒（last_seen 以秒为单位存储）
    sleep(Duration::from_millis(1100)).await;

    // 刷新心跳
    manager.refresh_session(&"session1".to_string());
    wait_for_async_tasks().await;

    // 验证时间已更新
    let updated_last_seen = session.last_seen();
    assert!(updated_last_seen > initial_last_seen);
}

#[tokio::test]
async fn test_concurrent_session_registration() {
    let manager = Arc::new(SessionManager::new());
    let mut handles = vec![];

    // 并发注册 100 个会话
    for i in 0..100 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let session = create_test_session(
                format!("session_{}", i),
                1000 + i as u64,
                format!("device_{}", i),
            );
            manager_clone.register_session(session);
        });
        handles.push(handle);
    }

    // 等待所有任务完成
    for handle in handles {
        handle.await.unwrap();
    }

    wait_for_async_tasks().await;

    assert_eq!(manager.get_session_count(), 100);
}

#[tokio::test]
async fn test_concurrent_session_cleanup() {
    let manager = Arc::new(SessionManager::new());

    // 先注册 100 个会话
    for i in 0..100 {
        let session = create_test_session(
            format!("session_{}", i),
            1000 + i as u64,
            format!("device_{}", i),
        );
        manager.register_session(session);
    }

    wait_for_async_tasks().await;
    assert_eq!(manager.get_session_count(), 100);

    // 并发清理所有会话
    let mut handles = vec![];
    for i in 0..100 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            manager_clone.cleanup_session(&format!("session_{}", i));
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    wait_for_async_tasks().await;

    assert_eq!(manager.get_session_count(), 0);
}

#[tokio::test]
async fn test_accepting_new_connections() {
    let manager = SessionManager::new();

    // 默认接受新连接
    assert!(manager.is_accepting_new_connections());

    // 停止接受新连接
    manager.set_accepting_new_connections(false);
    assert!(!manager.is_accepting_new_connections());

    // 恢复接受新连接
    manager.set_accepting_new_connections(true);
    assert!(manager.is_accepting_new_connections());
}

#[tokio::test]
async fn test_send_to_user() {
    let manager = Arc::new(SessionManager::new());

    // 注册两个设备（保持 rx 存活，否则 tx.send() 会失败）
    let (session1, _rx1, _srx1) = create_test_session_with_rx("session1".to_string(), 1001, "device1".to_string());
    let (session2, _rx2, _srx2) = create_test_session_with_rx("session2".to_string(), 1001, "device2".to_string());

    manager.register_session(session1);
    manager.register_session(session2);

    // 发送消息到用户的所有设备
    let msg = axum::extract::ws::Message::Text("test message".to_string().into());
    let sent_count = manager.send_to_user(1001, msg).await;

    // 应该发送到 2 个会话
    assert_eq!(sent_count, 2);
}

#[tokio::test]
async fn test_send_to_device() {
    let manager = Arc::new(SessionManager::new());

    // 同一设备两个会话（保持 rx 存活，否则 tx.send() 会失败）
    let (session1, _rx1, _srx1) = create_test_session_with_rx("session1".to_string(), 1001, "device1".to_string());
    let (session2, _rx2, _srx2) = create_test_session_with_rx("session2".to_string(), 1001, "device1".to_string());

    manager.register_session(session1);
    manager.register_session(session2);

    // 发送消息到指定设备
    let msg = axum::extract::ws::Message::Text("test message".to_string().into());
    let sent_count = manager
        .send_to_device(1001, &"device1".to_string(), msg)
        .await;

    // 应该发送到 2 个会话（同一设备）
    assert_eq!(sent_count, 2);
}

#[tokio::test]
async fn test_node_id() {
    let manager = SessionManager::new();
    let node_id = manager.node_id();

    // 应该有节点 ID（从环境变量或默认值）
    assert!(!node_id.is_empty());
}

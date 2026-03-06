/// 会话管理器扩展测试
///
/// 补充测试边界情况、错误处理和复杂场景
use crate::common::*;
use ms_websocket::websocket::SessionManager;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

// ========================
// 边界情况：清理不存在的会话
// ========================

#[tokio::test]
async fn test_cleanup_nonexistent_session() {
    let manager = SessionManager::new();

    // 清理不存在的会话不应 panic
    manager.cleanup_session(&"nonexistent".to_string());
    assert_eq!(manager.get_session_count(), 0);
}

#[tokio::test]
async fn test_cleanup_same_session_twice() {
    let manager = SessionManager::new();
    let session = create_test_session("session1".to_string(), 1001, "device1".to_string());

    manager.register_session(session);
    assert_eq!(manager.get_session_count(), 1);

    manager.cleanup_session(&"session1".to_string());
    wait_for_async_tasks().await;
    assert_eq!(manager.get_session_count(), 0);

    // 再次清理同一个会话不应 panic
    manager.cleanup_session(&"session1".to_string());
    assert_eq!(manager.get_session_count(), 0);
}

// ========================
// 边界情况：刷新不存在的会话
// ========================

#[tokio::test]
async fn test_refresh_nonexistent_session() {
    let manager = SessionManager::new();

    // 刷新不存在的会话不应 panic
    manager.refresh_session(&"nonexistent".to_string());
}

#[tokio::test]
async fn test_refresh_after_cleanup() {
    let manager = SessionManager::new();
    let session = create_test_session("session1".to_string(), 1001, "device1".to_string());

    manager.register_session(session);
    manager.cleanup_session(&"session1".to_string());
    wait_for_async_tasks().await;

    // 清理后刷新不应 panic
    manager.refresh_session(&"session1".to_string());
}

// ========================
// 边界情况：发送消息到不存在的用户/设备
// ========================

#[tokio::test]
async fn test_send_to_nonexistent_user() {
    let manager = Arc::new(SessionManager::new());

    let msg = axum::extract::ws::Message::Text("test".to_string().into());
    let sent = manager.send_to_user(9999, msg).await;
    assert_eq!(sent, 0);
}

#[tokio::test]
async fn test_send_to_nonexistent_device() {
    let manager = Arc::new(SessionManager::new());

    // 注册一个会话
    let (session, _rx, _srx) =
        create_test_session_with_rx("session1".to_string(), 1001, "device1".to_string());
    manager.register_session(session);

    // 发送消息到不存在的设备
    let msg = axum::extract::ws::Message::Text("test".to_string().into());
    let sent = manager
        .send_to_device(1001, &"nonexistent_device".to_string(), msg)
        .await;
    assert_eq!(sent, 0);
}

#[tokio::test]
async fn test_send_to_device_wrong_user() {
    let manager = Arc::new(SessionManager::new());

    let (session, _rx, _srx) =
        create_test_session_with_rx("session1".to_string(), 1001, "device1".to_string());
    manager.register_session(session);

    // 使用错误的 uid 发送
    let msg = axum::extract::ws::Message::Text("test".to_string().into());
    let sent = manager
        .send_to_device(9999, &"device1".to_string(), msg)
        .await;
    assert_eq!(sent, 0);
}

// ========================
// 获取用户会话边界
// ========================

#[tokio::test]
async fn test_get_sessions_nonexistent_user() {
    let manager = SessionManager::new();

    let sessions = manager.get_user_sessions(9999);
    assert!(sessions.is_empty());
}

#[tokio::test]
async fn test_get_sessions_after_all_cleaned_up() {
    let manager = SessionManager::new();

    let session = create_test_session("session1".to_string(), 1001, "device1".to_string());
    manager.register_session(session);

    manager.cleanup_session(&"session1".to_string());
    wait_for_async_tasks().await;

    let sessions = manager.get_user_sessions(1001);
    assert!(sessions.is_empty());
}

// ========================
// 多用户交叉操作
// ========================

#[tokio::test]
async fn test_multi_user_independent_sessions() {
    let manager = SessionManager::new();

    // 注册不同用户的会话
    let session1 = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let session2 = create_test_session("s2".to_string(), 1002, "d2".to_string());
    let session3 = create_test_session("s3".to_string(), 1003, "d3".to_string());

    manager.register_session(session1);
    manager.register_session(session2);
    manager.register_session(session3);

    assert_eq!(manager.get_session_count(), 3);
    assert_eq!(manager.get_user_sessions(1001).len(), 1);
    assert_eq!(manager.get_user_sessions(1002).len(), 1);
    assert_eq!(manager.get_user_sessions(1003).len(), 1);

    // 清理用户2的会话，不影响其他用户
    manager.cleanup_session(&"s2".to_string());
    wait_for_async_tasks().await;

    assert_eq!(manager.get_session_count(), 2);
    assert_eq!(manager.get_user_sessions(1001).len(), 1);
    assert_eq!(manager.get_user_sessions(1002).len(), 0);
    assert_eq!(manager.get_user_sessions(1003).len(), 1);
}

#[tokio::test]
async fn test_multi_user_same_device_id() {
    let manager = SessionManager::new();

    // 不同用户使用相同的 client_id（理论上每个设备有唯一指纹，但测试边界情况）
    let session1 = create_test_session("s1".to_string(), 1001, "shared_device".to_string());
    let session2 = create_test_session("s2".to_string(), 1002, "shared_device".to_string());

    manager.register_session(session1);
    manager.register_session(session2);

    assert_eq!(manager.get_session_count(), 2);
    assert_eq!(manager.get_user_sessions(1001).len(), 1);
    assert_eq!(manager.get_user_sessions(1002).len(), 1);

    // 清理用户1不应影响用户2
    manager.cleanup_session(&"s1".to_string());
    wait_for_async_tasks().await;

    assert_eq!(manager.get_session_count(), 1);
    assert_eq!(manager.get_user_sessions(1001).len(), 0);
    assert_eq!(manager.get_user_sessions(1002).len(), 1);
}

// ========================
// 同一设备多会话的复杂场景
// ========================

#[tokio::test]
async fn test_same_device_three_sessions_cleanup_middle() {
    let manager = SessionManager::new();

    // 同一个设备3个会话
    let s1 = create_test_session("s1".to_string(), 1001, "device1".to_string());
    let s2 = create_test_session("s2".to_string(), 1001, "device1".to_string());
    let s3 = create_test_session("s3".to_string(), 1001, "device1".to_string());

    manager.register_session(s1);
    manager.register_session(s2);
    manager.register_session(s3);

    assert_eq!(manager.get_session_count(), 3);
    assert_eq!(manager.get_user_sessions(1001).len(), 3);

    // 清理中间的会话
    manager.cleanup_session(&"s2".to_string());
    wait_for_async_tasks().await;

    assert_eq!(manager.get_session_count(), 2);
    assert_eq!(manager.get_user_sessions(1001).len(), 2);
}

#[tokio::test]
async fn test_mixed_devices_cleanup_order() {
    let manager = SessionManager::new();

    // 用户在两个设备上各有2个会话
    let s1 = create_test_session("s1".to_string(), 1001, "deviceA".to_string());
    let s2 = create_test_session("s2".to_string(), 1001, "deviceA".to_string());
    let s3 = create_test_session("s3".to_string(), 1001, "deviceB".to_string());
    let s4 = create_test_session("s4".to_string(), 1001, "deviceB".to_string());

    manager.register_session(s1);
    manager.register_session(s2);
    manager.register_session(s3);
    manager.register_session(s4);

    assert_eq!(manager.get_session_count(), 4);
    assert_eq!(manager.get_user_sessions(1001).len(), 4);

    // 清理 deviceA 的所有会话
    manager.cleanup_session(&"s1".to_string());
    manager.cleanup_session(&"s2".to_string());
    wait_for_async_tasks().await;

    assert_eq!(manager.get_session_count(), 2);
    assert_eq!(manager.get_user_sessions(1001).len(), 2);

    // 清理 deviceB 的所有会话
    manager.cleanup_session(&"s3".to_string());
    manager.cleanup_session(&"s4".to_string());
    wait_for_async_tasks().await;

    assert_eq!(manager.get_session_count(), 0);
    assert_eq!(manager.get_user_sessions(1001).len(), 0);
}

// ========================
// 并发注册和清理混合
// ========================

#[tokio::test]
async fn test_concurrent_register_and_cleanup() {
    let manager = Arc::new(SessionManager::new());

    // 先注册 50 个
    for i in 0..50 {
        let session = create_test_session(
            format!("s_{}", i),
            1000 + i as u64,
            format!("d_{}", i),
        );
        manager.register_session(session);
    }

    let mut handles = vec![];

    // 并发：注册 50 个新的 + 清理 50 个旧的
    for i in 0..50 {
        let m = manager.clone();
        handles.push(tokio::spawn(async move {
            m.cleanup_session(&format!("s_{}", i));
        }));
    }
    for i in 50..100 {
        let m = manager.clone();
        handles.push(tokio::spawn(async move {
            let session = create_test_session(
                format!("s_{}", i),
                1000 + i as u64,
                format!("d_{}", i),
            );
            m.register_session(session);
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    wait_for_async_tasks().await;

    // 应该剩 50 个（新注册的）
    assert_eq!(manager.get_session_count(), 50);
}

// ========================
// 消息发送和接收验证
// ========================

#[tokio::test]
async fn test_send_to_user_message_content() {
    let manager = Arc::new(SessionManager::new());

    let (session, mut rx, _srx) =
        create_test_session_with_rx("s1".to_string(), 1001, "d1".to_string());
    manager.register_session(session);

    let test_msg = r#"{"type":1,"data":{"hello":"world"}}"#;
    let msg = axum::extract::ws::Message::Text(test_msg.to_string().into());
    let sent = manager.send_to_user(1001, msg).await;
    assert_eq!(sent, 1);

    // 验证接收到的消息内容
    let received = rx.recv().await.unwrap();
    match received {
        axum::extract::ws::Message::Text(text) => {
            assert_eq!(text.to_string(), test_msg);
        }
        _ => panic!("预期收到 Text 消息"),
    }
}

#[tokio::test]
async fn test_send_to_device_message_content() {
    let manager = Arc::new(SessionManager::new());

    let (session, mut rx, _srx) =
        create_test_session_with_rx("s1".to_string(), 1001, "d1".to_string());
    manager.register_session(session);

    let msg = axum::extract::ws::Message::Text("device_specific_msg".to_string().into());
    let sent = manager
        .send_to_device(1001, &"d1".to_string(), msg)
        .await;
    assert_eq!(sent, 1);

    let received = rx.recv().await.unwrap();
    match received {
        axum::extract::ws::Message::Text(text) => {
            assert_eq!(text.to_string(), "device_specific_msg");
        }
        _ => panic!("预期收到 Text 消息"),
    }
}

#[tokio::test]
async fn test_send_to_user_multiple_devices_verify_all_receive() {
    let manager = Arc::new(SessionManager::new());

    let (session1, mut rx1, _srx1) =
        create_test_session_with_rx("s1".to_string(), 1001, "d1".to_string());
    let (session2, mut rx2, _srx2) =
        create_test_session_with_rx("s2".to_string(), 1001, "d2".to_string());

    manager.register_session(session1);
    manager.register_session(session2);

    let msg = axum::extract::ws::Message::Text("broadcast".to_string().into());
    let sent = manager.send_to_user(1001, msg).await;
    assert_eq!(sent, 2);

    // 两个设备都应收到消息
    let r1 = rx1.recv().await.unwrap();
    let r2 = rx2.recv().await.unwrap();

    match (r1, r2) {
        (axum::extract::ws::Message::Text(t1), axum::extract::ws::Message::Text(t2)) => {
            assert_eq!(t1.to_string(), "broadcast");
            assert_eq!(t2.to_string(), "broadcast");
        }
        _ => panic!("预期两个设备都收到 Text 消息"),
    }
}

// ========================
// 获取客户端 ID 列表
// ========================

#[tokio::test]
async fn test_get_client_ids_empty() {
    let manager = SessionManager::new();
    let ids = manager.get_client_ids();
    assert!(ids.is_empty());
}

#[tokio::test]
async fn test_get_client_ids_multiple_users() {
    let manager = SessionManager::new();

    let s1 = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let s2 = create_test_session("s2".to_string(), 1002, "d2".to_string());
    let s3 = create_test_session("s3".to_string(), 1001, "d3".to_string());

    manager.register_session(s1);
    manager.register_session(s2);
    manager.register_session(s3);

    let ids = manager.get_client_ids();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&"d1".to_string()));
    assert!(ids.contains(&"d2".to_string()));
    assert!(ids.contains(&"d3".to_string()));
}

// ========================
// 节点 ID
// ========================

#[tokio::test]
async fn test_node_id_default() {
    let manager = SessionManager::new();
    let node_id = manager.node_id();
    assert!(!node_id.is_empty());
}

// ========================
// 大规模会话管理
// ========================

#[tokio::test]
async fn test_large_scale_single_user_many_devices() {
    let manager = SessionManager::new();

    // 一个用户，50 个不同设备
    for i in 0..50 {
        let session = create_test_session(
            format!("s_{}", i),
            1001,
            format!("device_{}", i),
        );
        manager.register_session(session);
    }

    assert_eq!(manager.get_session_count(), 50);
    assert_eq!(manager.get_user_sessions(1001).len(), 50);
    assert_eq!(manager.get_client_ids().len(), 50);

    // 逐个清理
    for i in 0..50 {
        manager.cleanup_session(&format!("s_{}", i));
    }
    wait_for_async_tasks().await;

    assert_eq!(manager.get_session_count(), 0);
    assert_eq!(manager.get_user_sessions(1001).len(), 0);
}

#[tokio::test]
async fn test_large_scale_many_users() {
    let manager = SessionManager::new();

    // 200 个不同用户
    for i in 0..200 {
        let session = create_test_session(
            format!("s_{}", i),
            1000 + i as u64,
            format!("d_{}", i),
        );
        manager.register_session(session);
    }

    assert_eq!(manager.get_session_count(), 200);

    // 验证每个用户都只有一个会话
    for i in 0..200 {
        assert_eq!(
            manager.get_user_sessions(1000 + i as u64).len(),
            1,
            "用户 {} 的会话数不正确",
            1000 + i as u64
        );
    }
}

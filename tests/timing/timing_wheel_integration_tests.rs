/// 时间轮与会话管理器集成测试
///
/// 测试时间轮在会话管理器中的实际工作场景
use crate::common::*;
use ms_websocket::websocket::{SessionManager, TimingWheel};
use std::sync::Arc;
use tokio::time::{Duration, sleep};

// ========================
// 时间轮完整轮转测试
// ========================

#[tokio::test]
async fn test_timing_wheel_full_rotation() {
    let wheel = TimingWheel::new();

    // 添加到不同的槽位
    wheel.add("s1".to_string(), 10).await;
    wheel.add("s2".to_string(), 30).await;
    wheel.add("s3".to_string(), 59).await;

    assert_eq!(wheel.len().await, 3);

    // 转 10 次
    for _ in 0..10 {
        wheel.tick().await;
    }

    // s1 应该过期 (在第 11 次 tick 时)
    let expired = wheel.tick().await;
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0], "s1");
    assert_eq!(wheel.len().await, 2);
}

#[tokio::test]
async fn test_timing_wheel_add_at_different_offsets() {
    let wheel = TimingWheel::new();

    // 先 tick 5 次移动 current_slot
    for _ in 0..5 {
        wheel.tick().await;
    }

    // 从 slot 5 开始添加，timeout=3 应该放到 slot 8
    wheel.add("s1".to_string(), 3).await;

    // 3 次 tick 到 slot 8
    for _ in 0..3 {
        let expired = wheel.tick().await;
        assert!(expired.is_empty());
    }

    // 第4次 tick 到达 slot 8
    let expired = wheel.tick().await;
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0], "s1");
}

#[tokio::test]
async fn test_timing_wheel_rapid_add_remove() {
    let wheel = TimingWheel::new();

    // 快速添加和删除
    for i in 0..100 {
        wheel.add(format!("session_{}", i), 10).await;
    }
    assert_eq!(wheel.len().await, 100);

    for i in 0..100 {
        wheel.remove(&format!("session_{}", i)).await;
    }
    assert_eq!(wheel.len().await, 0);

    // tick 也不应返回任何过期会话
    for _ in 0..60 {
        let expired = wheel.tick().await;
        assert!(expired.is_empty());
    }
}

#[tokio::test]
async fn test_timing_wheel_multiple_refreshes() {
    let wheel = TimingWheel::new();

    wheel.add("s1".to_string(), 2).await;

    // 连续刷新 5 次（每次延长到 2 个 tick 后）
    for _ in 0..5 {
        wheel.tick().await;
        wheel.refresh(&"s1".to_string(), 2).await;
    }

    // s1 仍然存在
    assert_eq!(wheel.len().await, 1);
}

#[tokio::test]
async fn test_timing_wheel_refresh_nonexistent() {
    let wheel = TimingWheel::new();

    // 刷新不存在的会话不应 panic
    wheel.refresh(&"nonexistent".to_string(), 10).await;

    // 但会导致该会话被添加到时间轮
    // (因为 refresh = remove + add，remove 不存在的是无操作，add 会添加)
    assert_eq!(wheel.len().await, 1);
}

// ========================
// 并发 tick 和操作测试
// ========================

#[tokio::test]
async fn test_timing_wheel_concurrent_add_and_tick() {
    let wheel = Arc::new(TimingWheel::new());

    // 先添加一些会话
    for i in 0..50 {
        wheel.add(format!("session_{}", i), 30).await;
    }

    let mut handles = vec![];

    // 并发添加更多会话
    for i in 50..100 {
        let wheel_clone = wheel.clone();
        handles.push(tokio::spawn(async move {
            wheel_clone.add(format!("session_{}", i), 30).await;
        }));
    }

    // 同时进行 tick
    let wheel_clone = wheel.clone();
    handles.push(tokio::spawn(async move {
        for _ in 0..5 {
            wheel_clone.tick().await;
        }
    }));

    for handle in handles {
        handle.await.unwrap();
    }

    // 不应 panic，会话数取决于时序
    assert!(wheel.len().await > 0);
}

// ========================
// SessionManager 心跳超时场景
// ========================

#[tokio::test]
async fn test_session_manager_heartbeat_refresh_keeps_alive() {
    let manager = SessionManager::new();

    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    manager.register_session(session.clone());

    // 持续刷新心跳
    for _ in 0..3 {
        sleep(Duration::from_millis(100)).await;
        manager.refresh_session(&"s1".to_string());
    }

    wait_for_async_tasks().await;

    // 会话应该仍然存在（因为持续刷新）
    assert_eq!(manager.get_session_count(), 1);
}

#[tokio::test]
async fn test_session_register_adds_to_timing_wheel() {
    let manager = SessionManager::new();

    let s1 = create_test_session("s1".to_string(), 1001, "d1".to_string());
    let s2 = create_test_session("s2".to_string(), 1002, "d2".to_string());

    manager.register_session(s1);
    manager.register_session(s2);

    wait_for_async_tasks().await;

    // 两个会话都应注册成功
    assert_eq!(manager.get_session_count(), 2);
}

#[tokio::test]
async fn test_session_cleanup_removes_from_timing_wheel() {
    let manager = SessionManager::new();

    let session = create_test_session("s1".to_string(), 1001, "d1".to_string());
    manager.register_session(session);

    wait_for_async_tasks().await;

    manager.cleanup_session(&"s1".to_string());

    wait_for_async_tasks().await;

    assert_eq!(manager.get_session_count(), 0);
}

// ========================
// 时间轮精度测试
// ========================

#[tokio::test]
async fn test_timing_wheel_timeout_of_1() {
    let wheel = TimingWheel::new();

    wheel.add("s1".to_string(), 1).await;

    // 第 1 次 tick：current_slot 前进，不会触发 s1
    let expired = wheel.tick().await;
    assert_eq!(expired.len(), 0);

    // 第 2 次 tick：到达 s1 所在的槽位
    let expired = wheel.tick().await;
    assert_eq!(expired.len(), 1);
}

#[tokio::test]
async fn test_timing_wheel_timeout_of_0() {
    let wheel = TimingWheel::new();

    // timeout=0 意味着放在当前槽位
    wheel.add("s1".to_string(), 0).await;

    // 第 1 次 tick 应该立即返回
    let expired = wheel.tick().await;
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0], "s1");
}

#[tokio::test]
async fn test_timing_wheel_timeout_equals_wheel_size() {
    let wheel = TimingWheel::new();

    // timeout=60 等于 WHEEL_SIZE，环绕到 slot 0
    wheel.add("s1".to_string(), 60).await;

    // 第 1 次 tick 就应该过期（因为放在当前槽位 0，tick 后就检查 slot 0）
    let expired = wheel.tick().await;
    assert_eq!(expired.len(), 1);
}

#[tokio::test]
async fn test_timing_wheel_scatter_and_collect() {
    let wheel = TimingWheel::new();

    // 分散在不同槽位
    wheel.add("s1".to_string(), 1).await;
    wheel.add("s2".to_string(), 2).await;
    wheel.add("s3".to_string(), 3).await;
    wheel.add("s4".to_string(), 4).await;
    wheel.add("s5".to_string(), 5).await;

    let mut all_expired = Vec::new();

    // 逐个 tick 收集过期会话
    for _ in 0..6 {
        let expired = wheel.tick().await;
        all_expired.extend(expired);
    }

    assert_eq!(all_expired.len(), 5);
    assert!(all_expired.contains(&"s1".to_string()));
    assert!(all_expired.contains(&"s2".to_string()));
    assert!(all_expired.contains(&"s3".to_string()));
    assert!(all_expired.contains(&"s4".to_string()));
    assert!(all_expired.contains(&"s5".to_string()));
}

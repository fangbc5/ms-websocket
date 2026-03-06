/// 时间轮扩展测试
use ms_websocket::websocket::TimingWheel;
use std::sync::Arc;

#[tokio::test]
async fn test_timing_wheel_concurrent_add() {
    let wheel = Arc::new(TimingWheel::new());
    let mut handles = vec![];

    // 并发添加 1000 个会话
    for i in 0..1000 {
        let wheel_clone = wheel.clone();
        let handle = tokio::spawn(async move {
            wheel_clone.add(format!("session_{}", i), 30).await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(wheel.len().await, 1000);
}

#[tokio::test]
async fn test_timing_wheel_concurrent_remove() {
    let wheel = Arc::new(TimingWheel::new());

    // 先添加 1000 个会话
    for i in 0..1000 {
        wheel.add(format!("session_{}", i), 30).await;
    }

    assert_eq!(wheel.len().await, 1000);

    // 并发移除
    let mut handles = vec![];
    for i in 0..1000 {
        let wheel_clone = wheel.clone();
        let handle = tokio::spawn(async move {
            wheel_clone.remove(&format!("session_{}", i)).await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(wheel.len().await, 0);
}

#[tokio::test]
async fn test_timing_wheel_performance() {
    let wheel = TimingWheel::new();
    let start = std::time::Instant::now();

    // 添加 10,000 个会话
    for i in 0..10_000 {
        wheel.add(format!("session_{}", i), 30).await;
    }

    let elapsed = start.elapsed();
    println!("添加 10,000 个会话耗时: {:?}", elapsed);

    // 应该在 1 秒内完成
    assert!(elapsed.as_secs() < 1);
    assert_eq!(wheel.len().await, 10_000);
}

#[tokio::test]
async fn test_timing_wheel_boundary_wraparound() {
    let wheel = TimingWheel::new();

    // 时间轮有 60 个槽位（0-59）
    // 当前槽位是 0，添加 5 秒超时，会放到槽位 (0 + 5) % 60 = 5
    wheel.add("session1".to_string(), 5).await;

    assert_eq!(wheel.len().await, 1);

    // Tick 5 次，到达槽位 5
    for i in 0..5 {
        let expired = wheel.tick().await;
        assert_eq!(expired.len(), 0, "会话不应该在第 {} 次 tick 时超时", i + 1);
    }

    // 第 6 次 tick，当前槽位变为 5，会话应该超时
    let expired = wheel.tick().await;
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0], "session1");
    assert_eq!(wheel.len().await, 0);
}

#[tokio::test]
async fn test_timing_wheel_wraparound_large_timeout() {
    let wheel = TimingWheel::new();

    // 添加超过 60 秒的超时（会环绕）
    // 当前槽位 0，添加 65 秒超时，会放到槽位 (0 + 65) % 60 = 5
    wheel.add("session1".to_string(), 65).await;

    assert_eq!(wheel.len().await, 1);

    // 注意：时间轮只支持最多 60 秒的超时
    // 65 秒会被当作 5 秒处理（环绕）
    for i in 0..5 {
        let expired = wheel.tick().await;
        assert_eq!(expired.len(), 0, "会话不应该在第 {} 次 tick 时超时", i + 1);
    }

    // 第 6 次 tick，会话应该超时
    let expired = wheel.tick().await;
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0], "session1");
}

#[tokio::test]
async fn test_timing_wheel_multiple_sessions_same_slot() {
    let wheel = TimingWheel::new();

    // 添加多个会话到同一个槽位
    for i in 0..10 {
        wheel.add(format!("session_{}", i), 5).await;
    }

    assert_eq!(wheel.len().await, 10);

    // Tick 5 次
    for _ in 0..5 {
        wheel.tick().await;
    }

    // 下一次 tick 应该返回所有 10 个会话
    let expired = wheel.tick().await;
    assert_eq!(expired.len(), 10);
    assert_eq!(wheel.len().await, 0);
}

#[tokio::test]
async fn test_timing_wheel_refresh_extends_timeout() {
    let wheel = TimingWheel::new();

    wheel.add("session1".to_string(), 2).await;

    // Tick 1 次
    wheel.tick().await;

    // 刷新会话，延长到 5 秒
    wheel.refresh(&"session1".to_string(), 5).await;

    // 再 Tick 1 次（原本应该超时）
    let expired = wheel.tick().await;
    assert_eq!(expired.len(), 0); // 不应该超时

    // 继续 Tick 4 次
    for _ in 0..4 {
        wheel.tick().await;
    }

    // 现在应该超时
    let expired = wheel.tick().await;
    assert_eq!(expired.len(), 1);
}

#[tokio::test]
async fn test_timing_wheel_remove_nonexistent() {
    let wheel = TimingWheel::new();

    // 移除不存在的会话不应该 panic
    wheel.remove(&"nonexistent".to_string()).await;

    assert_eq!(wheel.len().await, 0);
}

#[tokio::test]
async fn test_timing_wheel_is_empty() {
    let wheel = TimingWheel::new();

    assert!(wheel.is_empty().await);

    wheel.add("session1".to_string(), 5).await;
    assert!(!wheel.is_empty().await);

    wheel.remove(&"session1".to_string()).await;
    assert!(wheel.is_empty().await);
}

#[tokio::test]
async fn test_timing_wheel_stress_test() {
    let wheel = Arc::new(TimingWheel::new());
    let mut handles = vec![];

    // 并发添加、刷新、移除
    for i in 0..100 {
        let wheel_clone = wheel.clone();
        let handle = tokio::spawn(async move {
            let session_id = format!("session_{}", i);

            // 添加
            wheel_clone.add(session_id.clone(), 10).await;

            // 刷新
            wheel_clone.refresh(&session_id, 20).await;

            // 移除
            if i % 2 == 0 {
                wheel_clone.remove(&session_id).await;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // 应该剩下 50 个会话（奇数索引）
    assert_eq!(wheel.len().await, 50);
}

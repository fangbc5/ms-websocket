/// 消息处理链测试
mod common;

use common::*;
use ms_websocket::model::ws_base_resp::WsBaseReq;
use ms_websocket::websocket::processor::{MessageHandlerChain, MessageProcessor};
use ms_websocket::websocket::Session;
use ms_websocket::types::{ClientId, SessionId, UserId};
use std::sync::Arc;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};

/// 测试用的消息处理器
struct TestProcessor {
    supports_type: i32,
    processed: Arc<AtomicBool>,
}

impl TestProcessor {
    fn new(supports_type: i32) -> Self {
        Self {
            supports_type,
            processed: Arc::new(AtomicBool::new(false)),
        }
    }

    fn was_processed(&self) -> bool {
        self.processed.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl MessageProcessor for TestProcessor {
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
        self.processed.store(true, Ordering::Relaxed);
    }
}

#[tokio::test]
async fn test_message_chain_finds_processor() {
    let processor1 = Arc::new(TestProcessor::new(1));
    let processor2 = Arc::new(TestProcessor::new(2));
    let processor3 = Arc::new(TestProcessor::new(3));

    let chain = MessageHandlerChain::new(vec![
        processor1.clone() as Arc<dyn MessageProcessor>,
        processor2.clone() as Arc<dyn MessageProcessor>,
        processor3.clone() as Arc<dyn MessageProcessor>,
    ]);

    let session = create_test_session("session1".to_string(), 1001, "device1".to_string());

    // 发送类型为 2 的消息
    let payload = r#"{"type": 2, "data": {}}"#;

    chain
        .handle_message(&session, &"session1".to_string(), 1001, &"device1".to_string(), payload)
        .await;

    // 只有 processor2 应该被处理
    assert!(!processor1.was_processed());
    assert!(processor2.was_processed());
    assert!(!processor3.was_processed());
}

#[tokio::test]
async fn test_message_chain_no_processor_found() {
    let processor1 = Arc::new(TestProcessor::new(1));
    let processor2 = Arc::new(TestProcessor::new(2));

    let chain = MessageHandlerChain::new(vec![
        processor1.clone() as Arc<dyn MessageProcessor>,
        processor2.clone() as Arc<dyn MessageProcessor>,
    ]);

    let session = create_test_session("session1".to_string(), 1001, "device1".to_string());

    // 发送类型为 99 的消息（没有处理器支持）
    let payload = r#"{"type": 99, "data": {}}"#;

    chain
        .handle_message(&session, &"session1".to_string(), 1001, &"device1".to_string(), payload)
        .await;

    // 没有处理器应该被调用
    assert!(!processor1.was_processed());
    assert!(!processor2.was_processed());
}

#[tokio::test]
async fn test_message_chain_invalid_json() {
    let processor1 = Arc::new(TestProcessor::new(1));

    let chain = MessageHandlerChain::new(vec![processor1.clone() as Arc<dyn MessageProcessor>]);

    let session = create_test_session("session1".to_string(), 1001, "device1".to_string());

    // 发送无效的 JSON
    let payload = r#"invalid json"#;

    chain
        .handle_message(&session, &"session1".to_string(), 1001, &"device1".to_string(), payload)
        .await;

    // 不应该 panic，也不应该调用处理器
    assert!(!processor1.was_processed());
}

#[tokio::test]
async fn test_message_chain_first_match_wins() {
    let processor1 = Arc::new(TestProcessor::new(1));
    let processor2 = Arc::new(TestProcessor::new(1)); // 同样支持类型 1

    let chain = MessageHandlerChain::new(vec![
        processor1.clone() as Arc<dyn MessageProcessor>,
        processor2.clone() as Arc<dyn MessageProcessor>,
    ]);

    let session = create_test_session("session1".to_string(), 1001, "device1".to_string());

    let payload = r#"{"type": 1, "data": {}}"#;

    chain
        .handle_message(&session, &"session1".to_string(), 1001, &"device1".to_string(), payload)
        .await;

    // 只有第一个处理器应该被调用
    assert!(processor1.was_processed());
    assert!(!processor2.was_processed());
}

#[tokio::test]
async fn test_message_chain_empty_processors() {
    let chain = MessageHandlerChain::new(vec![]);

    let session = create_test_session("session1".to_string(), 1001, "device1".to_string());

    let payload = r#"{"type": 1, "data": {}}"#;

    // 不应该 panic
    chain
        .handle_message(&session, &"session1".to_string(), 1001, &"device1".to_string(), payload)
        .await;
}

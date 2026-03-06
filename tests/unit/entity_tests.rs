/// NodeDownMessage 实体测试
use ms_websocket::websocket::entity::NodeDownMessage;

#[test]
fn test_node_down_message_new() {
    let msg = NodeDownMessage::new("node-1".to_string());
    assert_eq!(msg.node_id, "node-1");
}

#[test]
fn test_node_down_message_channel() {
    assert_eq!(NodeDownMessage::channel(), "ws-node-down");
}

#[test]
fn test_node_down_message_serialize() {
    let msg = NodeDownMessage::new("test-node".to_string());
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("test-node"));
    assert!(json.contains("node_id"));
}

#[test]
fn test_node_down_message_deserialize() {
    let json = r#"{"node_id":"node-abc"}"#;
    let msg: NodeDownMessage = serde_json::from_str(json).unwrap();
    assert_eq!(msg.node_id, "node-abc");
    assert_eq!(NodeDownMessage::channel(), "ws-node-down");
}

#[test]
fn test_node_down_message_roundtrip() {
    let original = NodeDownMessage::new("round-trip-node".to_string());
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: NodeDownMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(original.node_id, deserialized.node_id);
}

#[test]
fn test_node_down_message_clone() {
    let msg = NodeDownMessage::new("clone-test".to_string());
    let cloned = msg.clone();
    assert_eq!(msg.node_id, cloned.node_id);
}

#[test]
fn test_node_down_message_debug() {
    let msg = NodeDownMessage::new("debug-test".to_string());
    let debug_str = format!("{:?}", msg);
    assert!(debug_str.contains("NodeDownMessage"));
    assert!(debug_str.contains("debug-test"));
}

#[test]
fn test_node_down_message_channel_constant() {
    // channel() 应始终返回相同值，与 node_id 无关
    // channel() 是关联函数，不依赖实例
    assert_eq!(NodeDownMessage::channel(), NodeDownMessage::channel());
}

#[test]
fn test_node_down_message_empty_node_id() {
    let msg = NodeDownMessage::new(String::new());
    assert_eq!(msg.node_id, "");
    assert_eq!(NodeDownMessage::channel(), "ws-node-down");
}

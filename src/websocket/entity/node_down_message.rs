/// 节点下线消息
///
/// 用于通知其他节点某个节点已下线，需要转移会话
use serde::{Deserialize, Serialize};

/// 节点下线消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDownMessage {
    /// 节点 ID
    pub node_id: String,
}

impl NodeDownMessage {
    /// 创建新的节点下线消息
    pub fn new(node_id: String) -> Self {
        Self { node_id }
    }

    /// 获取 Redis 频道名称
    pub fn channel() -> &'static str {
        "ws-node-down"
    }
}


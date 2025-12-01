/// 路由推送 DTO
///
/// 用于消息中转工具，其他服务（如 oauth）需要推送消息时使用
/// 功能：将消息推送到目标用户列表所在的 ws 节点
use crate::model::ws_base_resp::WsBaseResp;
use serde::{Deserialize, Serialize};

/// 路由推送 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterPushDto {
    /// WebSocket 基础消息
    pub ws_base_msg: WsBaseResp,
    /// 目标用户 ID 列表
    pub uid_list: Vec<u64>,
    /// 操作人用户 ID
    pub uid: u64,
}


use crate::model::WsBaseResp;

/// 指纹级别的精确路由 DTO
///
/// 用于节点间推送消息，支持设备级别的精确路由
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodePushDTO {
    /// 推送的 WebSocket 消息
    pub ws_base_msg: WsBaseResp,
    /// 指纹与 uid 的映射（设备ID -> 用户ID）
    pub device_user_map: std::collections::HashMap<String, u64>,
    /// 消息唯一 hashId
    pub hash_id: u64,
    /// 操作人 uid
    pub uid: u64,
}
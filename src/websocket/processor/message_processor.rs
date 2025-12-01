/// 消息处理器 Trait
///
/// 所有消息处理器必须实现此 trait
/// 采用责任链模式，每个处理器检查是否支持该消息类型
use crate::model::ws_base_resp::WsBaseReq;
use crate::types::{ClientId, SessionId, UserId};
use crate::websocket::session_manager::Session;
use std::sync::Arc;

/// 消息处理器接口
#[async_trait::async_trait]
pub trait MessageProcessor: Send + Sync {
    /// 检查是否支持该消息类型
    ///
    /// # 参数
    /// - `req`: WebSocket 请求消息
    ///
    /// # 返回
    /// 如果支持该消息类型，返回 `true`
    fn supports(&self, req: &WsBaseReq) -> bool;

    /// 处理消息
    ///
    /// # 参数
    /// - `session`: 会话对象（用于发送响应）
    /// - `session_id`: 会话 ID
    /// - `uid`: 用户 ID
    /// - `client_id`: 客户端 ID（设备指纹）
    /// - `req`: 消息内容
    async fn process(
        &self,
        session: &Arc<Session>,
        session_id: &SessionId,
        uid: UserId,
        client_id: &ClientId,
        req: WsBaseReq,
    );
}

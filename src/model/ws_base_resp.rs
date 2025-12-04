/// WebSocket 基本请求信息体
///
/// 用于接收前端发送的 WebSocket 消息格式
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WsBaseReq {
    /// WebSocket 请求消息类型
    ///
    /// @see WSReqTypeEnum
    pub r#type: i32,
    /// 消息数据（使用 serde_json::Value 以支持任意类型）
    pub data: serde_json::Value,
}

/// WebSocket 基本返回信息体
///
/// 用于 WebSocket 推送给前端的消息格式
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WsBaseResp {
    /// WebSocket 推送给前端的消息类型
    ///
    /// @see WSRespTypeEnum
    pub r#type: i32,
    /// 消息数据（使用 serde_json::Value 以支持任意类型）
    pub data: serde_json::Value,
}

impl WsBaseResp {
    /// 创建新的 WebSocket 响应
    pub fn new(r#type: i32, data: serde_json::Value) -> Self {
        Self { r#type, data }
    }

    /// 从任意可序列化的数据创建响应
    pub fn from_data<T: serde::Serialize>(r#type: i32, data: T) -> serde_json::Result<Self> {
        Ok(Self {
            r#type,
            data: serde_json::to_value(data)?,
        })
    }
}

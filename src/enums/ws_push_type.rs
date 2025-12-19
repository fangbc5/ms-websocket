/// WebSocket 推送类型枚举
///
/// 定义 WebSocket 消息推送的类型
use serde::{Deserialize, Serialize};

/// WebSocket 推送类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WsPushTypeEnum {
    /// 个人
    User = 1,
    /// 全部连接用户
    All = 2,
}

impl WsPushTypeEnum {
    /// 获取类型值
    #[allow(dead_code)]
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// 获取描述
    #[allow(dead_code)]
    pub fn desc(&self) -> &'static str {
        match self {
            WsPushTypeEnum::User => "个人",
            WsPushTypeEnum::All => "全部连接用户",
        }
    }
}

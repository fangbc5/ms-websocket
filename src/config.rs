use fbc_starter::Config as BaseConfig;
use serde::{Deserialize, Serialize};
use tracing::info;

/// WebSocket 服务配置
/// 扩展 fbc-starter 的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsConfig {
    /// 基础配置（继承自 fbc-starter）
    #[serde(flatten)]
    pub base: BaseConfig,
    /// WebSocket 服务配置
    #[serde(default = "default_websocket_config")]
    pub websocket: WebSocketServiceConfig,
}

/// WebSocket 服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketServiceConfig {
    /// 节点 ID
    #[serde(default = "default_node_id")]
    pub node_id: String,
    /// 是否允许同设备多会话
    #[serde(default)]
    pub allow_multi_session_per_device: bool,
    /// 心跳超时时间（秒）
    #[serde(default = "default_heartbeat_timeout_secs")]
    pub heartbeat_timeout_secs: u64,
    /// 写入通道容量
    #[serde(default = "default_write_channel_cap")]
    pub write_channel_cap: usize,
}

fn default_websocket_config() -> WebSocketServiceConfig {
    WebSocketServiceConfig::default()
}

fn default_node_id() -> String {
    "1".to_string()
}

fn default_heartbeat_timeout_secs() -> u64 {
    30
}

fn default_write_channel_cap() -> usize {
    1024
}

impl Default for WebSocketServiceConfig {
    fn default() -> Self {
        Self {
            node_id: default_node_id(),
            allow_multi_session_per_device: false,
            heartbeat_timeout_secs: default_heartbeat_timeout_secs(),
            write_channel_cap: default_write_channel_cap(),
        }
    }
}

impl WsConfig {
    /// 从 BaseConfig + 环境变量加载配置
    pub fn new(base_config: BaseConfig) -> Result<Self, config::ConfigError> {
        let websocket = WebSocketServiceConfig {
            node_id: std::env::var("APP__WEBSOCKET__NODE_ID")
                .unwrap_or_else(|_| default_node_id()),
            allow_multi_session_per_device: std::env::var(
                "APP__WEBSOCKET__ALLOW_MULTI_SESSION_PER_DEVICE",
            )
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
            heartbeat_timeout_secs: std::env::var("APP__WEBSOCKET__HEARTBEAT_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_heartbeat_timeout_secs),
            write_channel_cap: std::env::var("APP__WEBSOCKET__WRITE_CHANNEL_CAP")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_write_channel_cap),
        };

        info!(
            node_id = %websocket.node_id,
            allow_multi_session_per_device = websocket.allow_multi_session_per_device,
            heartbeat_timeout_secs = websocket.heartbeat_timeout_secs,
            write_channel_cap = websocket.write_channel_cap,
            "WebSocket 配置加载完成"
        );

        Ok(Self {
            base: base_config,
            websocket,
        })
    }
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            base: BaseConfig::default(),
            websocket: WebSocketServiceConfig::default(),
        }
    }
}

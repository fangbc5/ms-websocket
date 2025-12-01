pub mod client;

// 重新导出 health_proto 供外部使用
pub use client::ImHealthClient;
pub use client::health_proto;

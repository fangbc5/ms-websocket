/// 消息路由模块
///
/// 提供消息路由服务，用于将消息推送到目标用户所在的 ws 节点

pub mod message_router_service;

pub use message_router_service::MessageRouterService;


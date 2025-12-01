/// WebSocket 模块
///
/// 重新设计的 WebSocket 架构，包含：
/// - 会话管理（用户→设备→会话三级映射）
/// - 消息处理链（责任链模式）
/// - Nacos 节点监听（处理节点下线）
pub mod entity;
pub mod handler;
pub mod processor;
pub mod router;
pub mod session_manager;

pub use entity::NodeDownMessage;
pub use handler::ws_route;
pub use processor::{MessageHandlerChain, MessageProcessor};
pub use router::MessageRouterService;
pub use session_manager::{Session, SessionManager};

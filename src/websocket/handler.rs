/// WebSocket 连接处理器
///
/// 处理 WebSocket 连接的建立、消息接收和连接关闭
use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::Query;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::state::WsState;
use crate::types::{ClientId, UserId};
use crate::websocket::session_manager::Session;

/// WebSocket 连接处理器
struct ConnectionHandler {
    session_id: String,
    session_manager: Arc<crate::websocket::SessionManager>,
    handler_chain: Arc<crate::websocket::MessageHandlerChain>,
}

impl ConnectionHandler {
    fn new(
        session_id: String,
        session_manager: Arc<crate::websocket::SessionManager>,
        handler_chain: Arc<crate::websocket::MessageHandlerChain>,
    ) -> Self {
        Self {
            session_id,
            session_manager,
            handler_chain,
        }
    }

    /// 从 SessionManager 获取 Session，如果获取不到返回 None
    fn get_session(&self) -> Option<Arc<Session>> {
        self.session_manager
            .sessions
            .get(&self.session_id)
            .map(|entry| entry.value().clone())
    }

    /// 启动写入任务：从通道读取消息并写入 WebSocket
    fn spawn_writer_task(
        &self,
        mut rx: mpsc::Receiver<Message>,
        mut sender: futures::stream::SplitSink<WebSocket, Message>,
    ) -> JoinHandle<()> {
        let session_id = self.session_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::Close(close_frame) => {
                        // 发送 Close 消息到 socket，然后退出
                        let _ = sender.send(Message::Close(close_frame)).await;
                        let _ = sender.close().await;
                        break;
                    }
                    _ => {
                        if sender.send(msg).await.is_err() {
                            error!(session_id = %session_id, "发送消息失败");
                            break;
                        }
                    }
                }
            }
            info!(session_id = %session_id, "写入任务结束");
        })
    }

    /// 处理接收到的消息
    async fn handle_message(&self, msg: Message) -> bool {
        // 从 SessionManager 获取 Session，如果获取不到说明已被清理，不执行操作
        let Some(session) = self.get_session() else {
            warn!(session_id = %self.session_id, "会话不存在，忽略消息");
            return false;
        };

        match msg {
            Message::Text(text) => {
                // 刷新会话心跳（更新时间轮）
                self.session_manager.refresh_session(&self.session_id);

                self.handler_chain
                    .handle_message(
                        &session,
                        &session.id,
                        session.uid,
                        &session.client_id,
                        &text,
                    )
                    .await;
                true
            }
            Message::Binary(_) => {
                // 刷新会话心跳（更新时间轮）
                self.session_manager.refresh_session(&self.session_id);

                // TODO: 处理二进制消息
                true
            }
            Message::Ping(payload) => {
                session.touch();
                if let Err(e) = session.try_send(Message::Pong(payload)) {
                    warn!(error = %e, "发送 Pong 失败（通道可能已满）");
                }
                true
            }
            Message::Pong(_) => {
                session.touch();
                true
            }
            Message::Close(_) => {
                info!(session_id = %self.session_id, "客户端关闭连接");
                false
            }
        }
    }

    /// 处理消息读取循环
    async fn run_message_loop(
        &self,
        mut receiver: futures::stream::SplitStream<WebSocket>,
        mut shutdown: mpsc::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                // 接收 WebSocket 消息
                result = receiver.next() => {
                    match result {
                        Some(Ok(msg)) => {
                            if !self.handle_message(msg).await {
                                // 收到 Close 消息，退出循环
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            warn!(session_id = %self.session_id, error = %e, "接收消息错误");
                            break;
                        }
                        None => {
                            // socket 已关闭
                            info!(session_id = %self.session_id, "主循环退出（socket 已关闭）");
                            break;
                        }
                    }
                }
                // 收到关闭信号
                _ = shutdown.recv() => {
                    info!(session_id = %self.session_id, "主循环退出（收到关闭信号）");
                    break;
                }
            }
        }
    }

    /// 清理连接
    async fn cleanup(&self) {
        info!(session_id = %self.session_id, "清理会话");
        self.session_manager.cleanup_session(&self.session_id, None);
    }
}

/// WebSocket 路由处理器
pub async fn ws_route(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    axum::extract::State(state): axum::extract::State<Arc<WsState>>,
) -> Response {
    // 检查是否接受新连接
    if !state.session_manager.is_accepting_new_connections() {
        return ws.on_upgrade(|mut socket| async move {
            let _ = socket.close().await;
        });
    }

    // 从查询参数中提取客户端 ID 和用户 ID
    let client_id = params
        .get("clientId")
        .cloned()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // TODO: 从认证信息中获取用户 ID，这里暂时使用查询参数
    let uid: UserId = params.get("uid").and_then(|s| s.parse().ok()).unwrap_or(0);

    if uid == 0 {
        return ws.on_upgrade(|mut socket| async move {
            let _ = socket.close().await;
        });
    }

    // 同设备重连：踢掉旧会话，允许新连接
    if !state.session_manager.allow_multi_session_per_device()
        && state.session_manager.has_device_session(uid, &client_id)
    {
        warn!(
            uid = uid,
            client_id = %client_id,
            "检测到同设备重复连接，踢掉旧会话"
        );
        state.session_manager.kick_device_sessions(uid, &client_id);
    }

    let session_id = Uuid::new_v4().to_string();
    let write_channel_cap = state.config.websocket.write_channel_cap;

    ws.on_upgrade(move |socket| {
        handle_connection(
            socket,
            state.session_manager.clone(),
            state.handler_chain.clone(),
            session_id,
            uid,
            client_id,
            write_channel_cap,
        )
    })
}

/// 处理 WebSocket 连接
async fn handle_connection(
    socket: WebSocket,
    session_manager: Arc<crate::websocket::SessionManager>,
    handler_chain: Arc<crate::websocket::MessageHandlerChain>,
    session_id: String,
    uid: UserId,
    client_id: ClientId,
    write_channel_cap: usize,
) {
    // 创建发送通道（有界通道，容量来自配置）
    let (tx, rx) = mpsc::channel(write_channel_cap);

    // 创建关闭信号
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

    // 创建并注册会话
    let session = Arc::new(Session::new(
        session_id.clone(),
        uid,
        client_id.clone(),
        tx,
        shutdown_tx,
    ));
    session_manager.register_session(session.clone());

    info!(
        session_id = %session_id,
        uid = uid,
        client_id = %client_id,
        "WebSocket 连接建立"
    );

    // 创建连接处理器
    let handler =
        ConnectionHandler::new(session_id.clone(), session_manager.clone(), handler_chain);

    // 分离 socket 为发送者和接收者
    let (sender, receiver) = socket.split();

    // 启动后台任务
    let writer_task = handler.spawn_writer_task(rx, sender);

    // 处理消息循环
    handler.run_message_loop(receiver, shutdown_rx).await;

    // 清理连接
    handler.cleanup().await;

    // 等待后台任务完成
    let _ = writer_task.await;

    info!(session_id = %session_id, "连接处理结束");
}

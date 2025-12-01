use crate::state::WsState;
/// WebSocket 连接处理器
///
/// 处理 WebSocket 连接的建立、消息接收和连接关闭
use crate::types::{ClientId, UserId};
use crate::websocket::session_manager::Session;
use axum::extract::Query;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

/// 心跳超时时间（秒）
const HEARTBEAT_TIMEOUT: u64 = 30;

/// WebSocket 路由处理器
pub async fn ws_route(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    axum::extract::State(state): axum::extract::State<Arc<WsState>>,
) -> Response {
    // 检查是否接受新连接
    if !state.session_manager.is_accepting_new_connections() {
        return ws.on_upgrade(|socket| async move {
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
        return ws.on_upgrade(|socket| async move {
            let _ = socket.close().await;
        });
    }

    let session_id = Uuid::new_v4().to_string();

    ws.on_upgrade(move |socket| {
        handle_connection(
            socket,
            state.session_manager.clone(),
            state.handler_chain.clone(),
            session_id,
            uid,
            client_id,
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
) {
    // 创建发送通道
    let (tx, mut rx) = mpsc::unbounded_channel();

    // 创建会话
    let session = Arc::new(Session::new(session_id.clone(), uid, client_id.clone(), tx));

    // 注册会话
    session_manager.register_session(session.clone());

    info!(
        "WebSocket 连接建立: session_id={}, uid={}, client_id={}",
        session_id, uid, client_id
    );

    // 分离 socket 为发送者和接收者
    let (mut sender, mut receiver) = socket.split();

    // 启动写入任务：从通道读取并写入 WebSocket
    let writer_session = session.clone();
    let writer_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
        info!("写入任务结束: session_id={}", writer_session.id);
    });

    // 启动心跳检查任务
    let hb_session_manager = session_manager.clone();
    let hb_session_id = session_id.clone();
    let heartbeat_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;

            // 检查会话是否存在
            if let Some(s) = hb_session_manager.sessions.get(&hb_session_id) {
                let last = s.last_seen();
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                if now.saturating_sub(last) > HEARTBEAT_TIMEOUT {
                    warn!("会话超时: session_id={}, last_seen={}", hb_session_id, last);
                    hb_session_manager.cleanup_session(&hb_session_id);
                    break;
                }
            } else {
                // 会话已不存在
                break;
            }
        }
        info!("心跳任务结束: session_id={}", hb_session_id);
    });

    // 读取循环：处理接收到的消息
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                // 更新会话活跃时间
                session.touch();

                // 处理文本消息
                handler_chain
                    .handle_message(&session, &session_id, uid, &client_id, &text)
                    .await;
            }
            Message::Binary(_) => {
                // 更新会话活跃时间
                session.touch();
                // TODO: 处理二进制消息
            }
            Message::Ping(payload) => {
                session.touch();
                // 通过 session.tx 发送 Pong（因为 sender 已经被移动到 writer_task）
                if let Err(e) = session.send(Message::Pong(payload)) {
                    warn!("发送 Pong 失败: {}", e);
                    break;
                }
            }
            Message::Pong(_) => {
                session.touch();
            }
            Message::Close(_) => {
                info!("客户端关闭连接: session_id={}", session_id);
                break;
            }
        }
    }

    // 连接结束时的清理
    info!("清理会话: session_id={}", session_id);
    session_manager.cleanup_session(&session_id);

    // 关闭通道，写入任务将结束
    drop(session);

    // 等待任务完成
    let _ = writer_task.await;
    let _ = heartbeat_task.await;

    info!("连接处理结束: session_id={}", session_id);
}

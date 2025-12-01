/// 消息推送服务
///
/// 消息推送升级版 [结合消息路由] ws服务专用
/// 功能：
/// - 单用户推送
/// - 批量用户推送
/// - 跨节点消息路由
/// - 本地节点直接推送
use crate::model::dto::NodePushDTO;
use crate::model::ws_base_resp::WsBaseResp;
use crate::websocket::SessionManager;
use fbc_starter::AppState;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

/// 消息推送服务
pub struct PushService {
    session_manager: Arc<SessionManager>,
    app_state: Arc<AppState>,
    node_id: String,
    /// 本地推送并发控制（最大32线程并发）
    local_push_semaphore: Arc<Semaphore>,
}

impl PushService {
    /// 创建新的消息推送服务
    pub fn new(
        session_manager: Arc<SessionManager>,
        app_state: Arc<AppState>,
        node_id: String,
    ) -> Self {
        Self {
            session_manager,
            app_state,
            node_id,
            local_push_semaphore: Arc::new(Semaphore::new(32)),
        }
    }

    /// 单用户推送
    pub async fn send_push_msg_single(
        &self,
        msg: WsBaseResp,
        uid: i64,
        cuid: i64,
    ) -> anyhow::Result<()> {
        self.send_push_msg(msg, vec![uid], cuid).await
    }

    /// 将消息推送到对应的用户
    pub async fn send_push_msg(
        &self,
        msg: WsBaseResp,
        uid_list: Vec<i64>,
        cuid: i64,
    ) -> anyhow::Result<()> {
        if uid_list.is_empty() {
            return Ok(());
        }

        // TODO: 1. 构建三级映射: 节点 → 设备指纹 → 用户ID
        // Map<String, Map<String, Long>> nodeDeviceUser = routerService.findNodeDeviceUser(uids);
        // 这里需要实现 NacosRouterService.find_node_device_user

        // 暂时简化：直接推送到本地节点
        // 2. 本地节点直接推送
        self.local_push(uid_list, &msg).await?;

        Ok(())
    }

    /// 本地节点直接推送
    async fn local_push(&self, uid_list: Vec<i64>, msg: &WsBaseResp) -> anyhow::Result<()> {
        let ws_msg =
            axum::extract::ws::Message::Text(serde_json::to_string(msg).unwrap_or_else(|e| {
                error!("序列化 WebSocket 消息失败: {}", e);
                String::new()
            }));

        // 按设备数动态调整并行度，最大32线程并发
        let parallelism = std::cmp::min(uid_list.len(), 32);

        // 使用信号量控制并发
        let semaphore = Arc::clone(&self.local_push_semaphore);
        let mut handles = Vec::new();

        for uid in uid_list {
            let permit = semaphore.clone().acquire_owned().await?;
            let session_manager = self.session_manager.clone();
            let msg = ws_msg.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;
                let sent = session_manager.send_to_user(uid as u64, msg);
                if sent == 0 {
                    warn!("推送失败: 用户 {} 不在线", uid);
                }
                sent
            });

            handles.push(handle);
        }

        // 等待所有任务完成
        let mut success_count = 0;
        for handle in handles {
            if let Ok(sent) = handle.await {
                if sent > 0 {
                    success_count += 1;
                }
            }
        }

        info!("本地推送完成: 成功={}/{}", success_count, parallelism);

        Ok(())
    }

    /// 将消息推送到指定节点（通过 MQ）
    async fn send_to_node_via_mq(
        &self,
        node_id: &str,
        msg: &WsBaseResp,
        device_user_map: HashMap<String, i64>,
        cuid: i64,
    ) -> anyhow::Result<()> {
        // TODO: 这里要解决一下唯一标识的问题
        let device_user_map: std::collections::HashMap<String, u64> = device_user_map
            .into_iter()
            .map(|(k, v)| (k, v as u64))
            .collect();
        let dto = NodePushDTO {
            ws_base_msg: msg.clone(),
            device_user_map,
            hash_id: cuid as u64, // 临时使用 cuid 作为 hash_id
            uid: cuid as u64,
        };

        // 发送到 Kafka
        if let Ok(producer) = self.app_state.message_producer() {
            let topic = format!("push_topic_{}", node_id);
            let message = fbc_starter::Message::new(
                topic.clone(),
                cuid.to_string(),
                serde_json::to_value(&dto)?,
            );
            producer
                .publish(&topic, message)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        } else {
            error!("Kafka Producer 未初始化，无法发送消息到节点: {}", node_id);
        }

        Ok(())
    }
}

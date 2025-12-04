/// 消息推送服务
///
/// 消息推送升级版 [结合消息路由] ws服务专用
/// 功能：
/// - 单用户推送
/// - 批量用户推送
/// - 跨节点消息路由
/// - 本地节点直接推送
use crate::cache::RouterCacheKeyBuilder;
use crate::model::dto::NodePushDTO;
use crate::model::ws_base_resp::WsBaseResp;
use crate::websocket::SessionManager;
use fbc_starter::{AppState, get_service_instances};
use redis::AsyncCommands;
use std::collections::{HashMap, HashSet};
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
        uid: u64,
        cuid: u64,
    ) -> anyhow::Result<()> {
        self.send_push_msg(msg, vec![uid], cuid).await
    }

    /// 将消息推送到对应的用户
    pub async fn send_push_msg(
        &self,
        msg: WsBaseResp,
        uid_list: Vec<u64>,
        cuid: u64,
    ) -> anyhow::Result<()> {
        if uid_list.is_empty() {
            return Ok(());
        }

        // 1. 构建三级映射: 节点 → 设备指纹 → 用户ID
        let node_device_user = self.find_node_device_user(&uid_list).await?;

        // 2. 按节点分组推送
        for (node_id, device_user_map) in node_device_user {
            if node_id == self.node_id {
                // 本地节点直接推送
                let local_uids: Vec<u64> = device_user_map.values().copied().collect();
                self.local_push(local_uids, &msg).await?;
            } else {
                // 跨节点推送（通过 MQ）
                self.send_to_node_via_mq(&node_id, &msg, device_user_map, cuid)
                    .await?;
            }
        }

        Ok(())
    }

    /// 聚合节点 → 设备 → 用户映射
    ///
    /// # 参数
    /// - `uids`: 用户 ID 列表
    ///
    /// # 返回
    /// 返回映射：节点 ID -> 设备 ID -> 用户 ID
    async fn find_node_device_user(
        &self,
        uids: &[u64],
    ) -> anyhow::Result<HashMap<String, HashMap<String, u64>>> {
        // 0. 前置校验
        if uids.is_empty() {
            return Ok(HashMap::new());
        }

        // 1. 提取目标 UID 集合
        let target_uids: HashSet<u64> = uids.iter().copied().collect();

        // 2. 获取全局设备-节点映射（使用 HSCAN 分批加载）
        let device_node_map = RouterCacheKeyBuilder::build_device_node_map(String::new());
        let mut result: HashMap<String, HashMap<String, u64>> = HashMap::new();

        // 3. 过滤活跃节点
        let active_nodes = self.get_all_active_nodes().await?;

        // 4. 使用 HGETALL 获取所有设备-节点映射（如果数据量大，可以考虑使用 HSCAN）
        let mut conn = self.app_state.redis()?;
        let items: HashMap<String, String> = conn.hgetall(&device_node_map.key).await?;

        for (field, node_id) in items {
            // 4.1 检查节点是否活跃
            if !active_nodes.contains(&node_id) {
                continue;
            }

            // 4.2 按 uid 过滤目标用户
            let parts: Vec<&str> = field.split(':').collect();
            if parts.len() != 2 {
                continue;
            }

            let uid = match parts[0].parse::<u64>() {
                Ok(id) => id,
                Err(_) => continue,
            };
            let client_id = parts[1];

            // 4.3 判断是否是目标用户
            if !target_uids.contains(&uid) {
                continue;
            }

            // 4.4 构建映射：节点 → 设备 → UID
            result
                .entry(node_id)
                .or_insert_with(HashMap::new)
                .insert(client_id.to_string(), uid);
        }

        Ok(result)
    }

    /// 获取所有活跃节点
    ///
    /// # 返回
    /// 返回所有活跃节点的 nodeId 集合
    async fn get_all_active_nodes(&self) -> anyhow::Result<HashSet<String>> {
        // 从 Nacos 获取服务实例
        // 优先使用配置的服务名称，如果没有则从已订阅的服务中查找
        let service_name = "ws-server"; // 默认服务名称，可以从配置中获取

        let instances = if let Some(instances) = get_service_instances(service_name) {
            instances
        } else {
            warn!("未找到服务实例: {}，且没有已订阅的服务", service_name);
            return Ok(HashSet::new());
        };

        // 过滤健康的实例并提取 nodeId
        let active_nodes: HashSet<String> = instances
            .iter()
            .filter(|instance| instance.healthy)
            .filter_map(|instance| {
                instance
                    .metadata
                    .as_ref()
                    .and_then(|meta| meta.get("nodeId"))
                    .map(|s| s.to_string())
            })
            .collect();

        Ok(active_nodes)
    }

    /// 本地节点直接推送
    async fn local_push(&self, uid_list: Vec<u64>, msg: &WsBaseResp) -> anyhow::Result<()> {
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
                let sent = session_manager.send_to_user(uid, msg).await;
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
        device_user_map: HashMap<String, u64>,
        cuid: u64,
    ) -> anyhow::Result<()> {
        // TODO: 这里要解决一下唯一标识的问题
        let dto = NodePushDTO {
            ws_base_msg: msg.clone(),
            device_user_map,
            hash_id: cuid, // 临时使用 cuid 作为 hash_id
            uid: cuid,
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

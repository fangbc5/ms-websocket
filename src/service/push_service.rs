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
    /// 本地路由缓存
    local_router_cache: Arc<crate::cache::LocalRouterCache>,
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
            local_router_cache: Arc::new(crate::cache::LocalRouterCache::default()),
        }
    }

    /// 单用户推送
    pub async fn send_push_msg_single(
        &self,
        msg: WsBaseResp,
        uid: u64,
        cuid: u64,
    ) -> anyhow::Result<()> {
        self.send_push_msg(msg, vec![uid], cuid).await.map(|_| ())
    }

    /// 将消息推送到对应的用户
    ///
    /// 返回实际投递的本地用户数（不含通过 MQ 转发到远程节点的）
    pub async fn send_push_msg(
        &self,
        msg: WsBaseResp,
        uid_list: Vec<u64>,
        cuid: u64,
    ) -> anyhow::Result<usize> {
        if uid_list.is_empty() {
            return Ok(0);
        }

        // 1. 构建三级映射: 节点 → 设备指纹 → 用户ID
        let node_device_user = self.find_node_device_user(&uid_list).await?;

        if node_device_user.is_empty() {
            warn!("推送路由为空: 目标用户 {:?} 在 Redis 中无路由信息", uid_list);
            return Ok(0);
        }

        // 2. 分离本地节点和远程节点
        let mut local_uids = Vec::new();
        let mut remote_nodes = Vec::new();

        for (node_id, device_user_map) in node_device_user {
            if node_id == self.node_id {
                // 收集本地用户
                local_uids.extend(device_user_map.values().copied());
            } else {
                // 收集远程节点信息
                remote_nodes.push((node_id, device_user_map));
            }
        }

        // 3. 本地节点直接推送
        let mut delivered = 0;
        if !local_uids.is_empty() {
            delivered = self.local_push(local_uids, &msg).await?;
        }

        // 4. 批量发送到远程节点（通过 MQ）
        if !remote_nodes.is_empty() {
            self.batch_send_to_nodes_via_mq(remote_nodes, &msg, cuid).await?;
        }

        Ok(delivered)
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

        // 2. 获取全局设备-节点映射（优先从本地缓存获取）
        let device_node_map = RouterCacheKeyBuilder::build_device_node_map(String::new());
        let mut result: HashMap<String, HashMap<String, u64>> = HashMap::new();

        // 3. 过滤活跃节点
        let active_nodes = self.get_all_active_nodes().await?;

        // 4. 从 Redis 获取所有设备-节点映射
        let mut conn = self.app_state.redis().await?;
        let items: HashMap<String, String> = conn.hgetall(&device_node_map.key).await?;

        // 5. 统计缓存命中率（用于监控）
        let mut cache_hits = 0;
        let mut cache_misses = 0;

        for (field, node_id) in items {
            // 5.1 检查节点是否活跃
            if !active_nodes.contains(&node_id) {
                continue;
            }

            // 5.2 按 uid 过滤目标用户
            let parts: Vec<&str> = field.split(':').collect();
            if parts.len() != 2 {
                continue;
            }

            let uid = match parts[0].parse::<u64>() {
                Ok(id) => id,
                Err(_) => continue,
            };
            let client_id = parts[1];

            // 5.3 判断是否是目标用户
            if !target_uids.contains(&uid) {
                continue;
            }

            // 5.4 检查本地缓存
            if let Some(cached_node_id) = self.local_router_cache.get(uid, client_id) {
                cache_hits += 1;
                // 使用缓存的节点 ID
                result
                    .entry(cached_node_id)
                    .or_insert_with(HashMap::new)
                    .insert(client_id.to_string(), uid);
            } else {
                cache_misses += 1;
                // 更新本地缓存
                self.local_router_cache.set(uid, client_id, node_id.clone());

                // 使用 Redis 的节点 ID
                result
                    .entry(node_id)
                    .or_insert_with(HashMap::new)
                    .insert(client_id.to_string(), uid);
            }
        }

        // 6. 记录缓存命中率
        if cache_hits + cache_misses > 0 {
            let hit_rate = (cache_hits as f64 / (cache_hits + cache_misses) as f64) * 100.0;
            info!(
                "路由缓存命中率: {:.2}% (命中={}, 未命中={})",
                hit_rate, cache_hits, cache_misses
            );
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
        let service_name = "ms-websocket"; // 默认服务名称，与 Nacos 注册的服务名一致

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
                    .and_then(|meta| {
                        // 尝试获取 nodeid（小写，环境变量转换后的键名）
                        // 或 nodeId（大小写混合，配置文件中的键名）
                        meta.get("nodeid")
                            .or_else(|| meta.get("nodeId"))
                            .map(|s| s.to_string())
                    })
            })
            .collect();

        Ok(active_nodes)
    }

    /// 本地节点直接推送
    ///
    /// 返回实际成功推送的用户数
    async fn local_push(&self, uid_list: Vec<u64>, msg: &WsBaseResp) -> anyhow::Result<usize> {
        let ws_msg = axum::extract::ws::Message::Text(
            serde_json::to_string(msg)
                .unwrap_or_else(|e| {
                    error!("序列化 WebSocket 消息失败: {}", e);
                    String::new()
                })
                .into(),
        );

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

        Ok(success_count)
    }

    /// 批量将消息推送到多个节点（通过 MQ）
    async fn batch_send_to_nodes_via_mq(
        &self,
        nodes: Vec<(String, HashMap<String, u64>)>,
        msg: &WsBaseResp,
        cuid: u64,
    ) -> anyhow::Result<()> {
        if nodes.is_empty() {
            return Ok(());
        }

        // 获取 Kafka Producer
        let producer = self.app_state.message_producer()
            .map_err(|_| anyhow::anyhow!("Kafka Producer 未初始化"))?;

        // 构建所有消息
        let mut messages = Vec::new();
        for (node_id, device_user_map) in &nodes {
            let dto = NodePushDTO {
                ws_base_msg: msg.clone(),
                device_user_map: device_user_map.clone(),
                hash_id: cuid,
                uid: cuid,
            };

            let topic = format!("websocket_push_{}", node_id);
            let message = fbc_starter::Message::new(
                topic.clone(),
                cuid.to_string(),
                serde_json::to_value(&dto)?,
            );

            messages.push((topic, message));
        }

        // 批量发送所有消息
        for (topic, message) in messages {
            producer
                .publish(&topic, message)
                .await
                .map_err(|e| anyhow::anyhow!("Kafka 发送失败: {}", e))?;
        }

        info!("批量推送到 {} 个远程节点完成", nodes.len());

        Ok(())
    }

    /// 将消息推送到指定节点（通过 MQ）
    ///
    /// 注意：此方法已被 batch_send_to_nodes_via_mq 替代，保留用于兼容性
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
            let topic = format!("websocket_push_{}", node_id);
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

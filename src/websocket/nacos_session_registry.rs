/// Nacos 会话注册中心
///
/// 负责维护节点路由信息和定期清理残留数据。
/// 对应 Java NacosSessionRegistry，功能包括：
/// - 定时更新节点元数据（会话数量、客户端 ID 列表）
/// - 定时清理残留节点的路由数据
/// - 节点完全清理（触发设备下线通知）
use crate::cache::RouterCacheKeyBuilder;
use crate::websocket::SessionManager;
use fbc_starter::{get_service_instances, AppState};
use redis::AsyncCommands;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

/// 节点指标更新间隔（秒）
const METRICS_UPDATE_INTERVAL: u64 = 5;

/// 残留路由清理间隔（秒）
const STALE_ROUTE_CLEANUP_INTERVAL: u64 = 30;

/// WS 集群服务名称
const WS_SERVICE_NAME: &str = "ms-websocket";

/// Nacos 会话注册中心
pub struct NacosSessionRegistry {
    session_manager: Arc<SessionManager>,
    app_state: Arc<AppState>,
    node_id: String,
}

impl NacosSessionRegistry {
    /// 创建新的 Nacos 会话注册中心
    pub fn new(
        session_manager: Arc<SessionManager>,
        app_state: Arc<AppState>,
        node_id: String,
    ) -> Self {
        Self {
            session_manager,
            app_state,
            node_id,
        }
    }

    /// 启动后台任务（指标更新 + 残留清理）
    pub fn start_background_tasks(self: &Arc<Self>) {
        self.start_metrics_updater();
        self.start_stale_route_cleaner();
    }

    /// 启动指标更新定时任务（每 5 秒）
    ///
    /// 对应 Java @Scheduled(fixedRate = 5000) updateNodeMetrics
    fn start_metrics_updater(self: &Arc<Self>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(METRICS_UPDATE_INTERVAL));
            loop {
                interval.tick().await;
                if let Err(e) = this.update_node_metrics().await {
                    error!("节点指标更新失败: {}", e);
                }
            }
        });
    }

    /// 启动残留路由清理定时任务（每 30 秒）
    ///
    /// 对应 Java @Scheduled(fixedDelay = 30000) cleanStaleRoutes
    fn start_stale_route_cleaner(self: &Arc<Self>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(STALE_ROUTE_CLEANUP_INTERVAL));
            loop {
                interval.tick().await;
                if let Err(e) = this.clean_stale_routes().await {
                    error!("残留路由清理失败: {}", e);
                }
            }
        });
    }

    /// 更新节点元数据到 Nacos
    ///
    /// 对应 Java updateNodeMetrics
    /// 注：在 Rust fbc_starter 中，节点注册由 Server 启动时完成。
    /// 此处通过 Redis 记录节点心跳和会话数，供其他节点查阅。
    async fn update_node_metrics(&self) -> anyhow::Result<()> {
        let session_count = self.session_manager.get_session_count();
        let client_ids = self.session_manager.get_client_ids();
        let client_ids_preview: Vec<_> = client_ids.iter().take(10).collect();

        // 写入 Redis Hash（node_metrics:{nodeId}）
        let metrics_key = format!("ws:node_metrics:{}", self.node_id);
        let mut conn = self.app_state.redis().await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();

        let _: () = conn
            .hset_multiple(
                &metrics_key,
                &[
                    ("lastHeartbeat", now.as_str()),
                    ("sessionCount", &session_count.to_string()),
                    (
                        "clientIds",
                        &client_ids_preview
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(","),
                    ),
                ],
            )
            .await?;

        // 设置过期时间，防止节点下线后数据残留
        let _: () = conn.expire(&metrics_key, 60).await?;

        Ok(())
    }

    /// 清理残留节点路由
    ///
    /// 对应 Java cleanStaleRoutes
    /// 比较 Nacos 活跃节点与 Redis 中记录的节点，清理已下线节点的数据
    async fn clean_stale_routes(&self) -> anyhow::Result<()> {
        // 1. 获取所有 Nacos 活跃节点
        let active_nodes = self.get_all_active_node_ids();

        // 2. 获取所有 Redis 中记录的节点
        let redis_nodes = self.get_all_redis_node_ids().await?;

        // 3. 计算需要清理的节点
        let stale_nodes: Vec<String> = redis_nodes
            .into_iter()
            .filter(|node| !active_nodes.contains(node))
            .collect();

        if stale_nodes.is_empty() {
            return Ok(());
        }

        // 4. 批量清理
        for node_id in &stale_nodes {
            info!("发现残留节点数据，开始清理: {}", node_id);
            if let Err(e) = self.cleanup_node_routes(node_id).await {
                error!("节点路由清理失败: {}, error={}", node_id, e);
            }
            if let Err(e) = self.clean_node_completely(node_id).await {
                error!("节点完全清理失败: {}, error={}", node_id, e);
            }
        }

        Ok(())
    }

    /// 获取所有 Nacos 活跃节点 ID
    ///
    /// 对应 Java getAllActiveNodeIds
    fn get_all_active_node_ids(&self) -> HashSet<String> {
        let instances = match get_service_instances(WS_SERVICE_NAME) {
            Some(instances) => instances,
            None => {
                warn!("未找到 WS 服务实例: {}", WS_SERVICE_NAME);
                return HashSet::new();
            }
        };

        instances
            .iter()
            .filter(|instance| instance.healthy)
            .filter_map(|instance| {
                instance
                    .metadata
                    .as_ref()
                    .and_then(|meta| {
                        meta.get("nodeid")
                            .or_else(|| meta.get("nodeId"))
                            .map(|s| s.to_string())
                    })
            })
            .collect()
    }

    /// 获取所有 Redis 中记录的节点 ID
    ///
    /// 对应 Java getAllRedisNodeIds
    /// 通过 SCAN 命令扫描 node-devices:{nodeId} 形式的 Set Key
    async fn get_all_redis_node_ids(&self) -> anyhow::Result<HashSet<String>> {
        let mut conn = self.app_state.redis().await?;

        // 构建扫描模式：node_devices 的 key 模式
        let base_key = RouterCacheKeyBuilder::build_node_devices("");
        let pattern = format!("{}*", base_key.key);

        // SCAN 扫描
        let mut node_ids = HashSet::new();
        let mut cursor: u64 = 0;
        loop {
            let (next_cursor, keys): (u64, Vec<String>) =
                redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(&pattern)
                    .arg("COUNT")
                    .arg(100)
                    .query_async(&mut conn)
                    .await?;

            for key in &keys {
                // 提取纯节点 ID（key 的最后一段）
                if let Some(node_id) = key.rsplit(':').next() {
                    if !node_id.is_empty() {
                        node_ids.insert(node_id.to_string());
                    }
                }
            }

            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }

        Ok(node_ids)
    }

    /// 清理节点在 Redis 中的所有路由信息
    ///
    /// 对应 Java cleanupNodeRoutes
    pub async fn cleanup_node_routes(&self, clean_node_id: &str) -> anyhow::Result<()> {
        let mut conn = self.app_state.redis().await?;

        // 1. 获取节点→设备映射
        let node_devices_key = RouterCacheKeyBuilder::build_node_devices(clean_node_id);
        let device_fields: Vec<String> = conn.smembers(&node_devices_key.key).await?;

        if !device_fields.is_empty() {
            // 2. 批量删除全局 Hash 中的映射
            let device_node_map = RouterCacheKeyBuilder::build_device_node_map(String::new());
            for field in &device_fields {
                let _: () = conn.hdel(&device_node_map.key, field).await?;
            }

            // 3. 删除节点设备集合
            let _: () = conn.del(&node_devices_key.key).await?;
        }

        info!(
            "节点路由清理完成: nodeId={}, 清理设备数={}",
            clean_node_id,
            device_fields.len()
        );

        Ok(())
    }

    /// 完全清理节点（触发设备下线通知）
    ///
    /// 对应 Java cleanNodeCompletely
    /// 解析节点所有设备的 uid:clientId，调用 sync_online(false) 触发下线通知
    async fn clean_node_completely(&self, node_id: &str) -> anyhow::Result<()> {
        let mut conn = self.app_state.redis().await?;

        // 1. 获取节点所有设备
        let node_devices_key = RouterCacheKeyBuilder::build_node_devices(node_id);
        let device_fields: Vec<String> = conn.smembers(&node_devices_key.key).await?;

        if device_fields.is_empty() {
            return Ok(());
        }

        // 2. 解析 uid → clients 映射
        let mut uid_to_clients: HashMap<u64, Vec<String>> = HashMap::new();
        for field in &device_fields {
            let parts: Vec<&str> = field.split(':').collect();
            if parts.len() == 2 {
                if let Ok(uid) = parts[0].parse::<u64>() {
                    uid_to_clients
                        .entry(uid)
                        .or_default()
                        .push(parts[1].to_string());
                }
            }
        }

        // 3. 逐个触发下线通知
        for (uid, clients) in &uid_to_clients {
            for client in clients {
                if let Err(e) = self.session_manager.sync_online(*uid, client, false).await {
                    error!(
                        "节点完全清理 - 同步下线状态失败: uid={}, client={}, error={}",
                        uid, client, e
                    );
                }
            }
        }

        info!(
            "节点完全清理完成: node={}, 设备数={}",
            node_id,
            device_fields.len()
        );

        Ok(())
    }
}

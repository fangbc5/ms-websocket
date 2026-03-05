# WebSocket 服务生产级稳定性改进计划

## 当前架构评估

### 已实现功能 ✅
- 多节点 WebSocket 集群支持
- 基于 Redis 的设备路由表
- 基于 Kafka 的跨节点消息分发
- 基于 Nacos 的服务发现
- 心跳检测与超时清理
- 会话三级映射（用户→设备→会话）
- 并发推送控制（信号量限流）

### 架构优势
- **水平扩展**：支持多节点部署，通过 Nacos 自动发现
- **高并发**：使用 DashMap 无锁并发，支持大量连接
- **消息路由**：Redis + Kafka 实现精准的跨节点消息投递
- **资源隔离**：每个节点独立的 Kafka consumer group

---

## 性能评估

### 理论性能指标

#### 单节点容量
- **并发连接数**：50,000 - 100,000（取决于硬件）
  - 每个连接占用：~4KB 内存（TCP buffer + 应用层状态）
  - 8GB 内存可支持约 100,000 连接

- **消息吞吐量**：
  - 本地推送：50,000 msg/s（单核）
  - 跨节点推送：受 Kafka 限制，约 10,000 msg/s

- **延迟**：
  - 本地推送：< 1ms
  - 跨节点推送：5-20ms（Kafka 往返）

#### 集群容量
- **3 节点集群**：300,000 并发连接
- **10 节点集群**：1,000,000 并发连接

### 性能瓶颈分析

#### 1. Redis 单点瓶颈 🔴
**问题**：
- 所有节点共享一个 Redis 实例存储路由表
- `HGETALL` 操作在设备数量大时（>100万）会阻塞
- 单个 Redis 实例 QPS 上限约 100,000

**影响**：
- 当设备数 > 100 万时，`find_node_device_user()` 延迟 > 100ms
- 高并发推送时 Redis 成为瓶颈

#### 2. Kafka 延迟 🟡
**问题**：
- 跨节点消息需要经过 Kafka，增加 5-20ms 延迟
- Kafka 批量发送机制可能导致消息堆积

**影响**：
- 实时性要求高的场景（如游戏、交易）可能不满足

#### 3. 心跳检查粒度 🟡
**问题**：
- 当前每 10 秒检查一次，超时时间 30 秒
- 遍历所有会话，时间复杂度 O(n)

**影响**：
- 10 万连接时，心跳检查耗时约 100ms

#### 4. 会话清理竞态 🟡
**问题**：
- Redis 注册/注销使用 `tokio::spawn`，无等待
- 快速重连可能导致注册被注销覆盖

**影响**：
- 极端情况下用户在线但路由表中无记录

---

## 生产级改进计划

### 阶段一：稳定性增强（P0 - 必须完成）

#### 1.1 Redis 路由表优化 🔴
**目标**：解决 Redis 单点瓶颈，支持千万级设备

**方案 A：Redis Cluster 分片**
```rust
// 使用一致性哈希将设备分散到多个 Redis 节点
let shard_id = hash(uid) % redis_cluster_size;
let cache_key = format!("router:device_node_map:{}", shard_id);
```

**方案 B：本地缓存 + Redis**
```rust
// 每个节点维护本地路由缓存，定期同步
struct RouterCache {
    local_cache: Arc<DashMap<String, String>>, // uid:client_id -> node_id
    redis: Arc<AppState>,
    ttl: Duration,
}
```

**推荐**：方案 B，减少 90% Redis 查询

**工作量**：3 天

---

#### 1.2 会话清理竞态修复 🔴
**目标**：保证 Redis 路由表与会话状态一致

**方案**：使用 Redis 事务 + 版本号
```rust
async fn register_device_to_redis(&self, uid: UserId, client_id: &str, node_id: &str) -> anyhow::Result<()> {
    let field = format!("{}:{}", uid, client_id);
    let version = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();

    // 使用 Lua 脚本保证原子性
    let script = r#"
        local current = redis.call('HGET', KEYS[1], ARGV[1])
        if not current or tonumber(ARGV[3]) > tonumber(current:match(':(%d+)$')) then
            redis.call('HSET', KEYS[1], ARGV[1], ARGV[2] .. ':' .. ARGV[3])
            return 1
        end
        return 0
    "#;

    conn.eval(script, &[&cache_key.key], &[&field, node_id, &version.to_string()]).await?;
    Ok(())
}
```

**工作量**：2 天

---

#### 1.3 优雅关闭机制 🔴
**目标**：节点下线时清理所有路由表记录

**方案**：
```rust
// 在 main.rs 中添加信号处理
#[tokio::main]
async fn main() -> AppResult<()> {
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel(1);

    // 监听 SIGTERM/SIGINT
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        shutdown_tx.send(()).ok();
    });

    // 启动服务器
    let server_handle = tokio::spawn(async move {
        Server::run(|builder| { /* ... */ }).await
    });

    // 等待关闭信号
    shutdown_rx.recv().await.ok();

    // 优雅关闭
    info!("收到关闭信号，开始优雅关闭...");
    session_manager.set_accepting_new_connections(false);
    session_manager.cleanup_all_sessions().await;

    server_handle.await??;
    Ok(())
}

// 在 SessionManager 中添加
pub async fn cleanup_all_sessions(&self) {
    let client_ids = self.get_client_ids();
    for client_id in client_ids {
        // 批量删除 Redis 路由
    }
}
```

**工作量**：2 天

---

#### 1.4 健康检查与熔断 🔴
**目标**：依赖服务故障时自动降级

**方案**：
```rust
pub struct CircuitBreaker {
    failure_count: AtomicU64,
    last_failure_time: AtomicU64,
    state: AtomicU8, // 0=Closed, 1=Open, 2=HalfOpen
}

impl PushService {
    async fn send_to_node_via_mq_with_circuit_breaker(&self, ...) -> anyhow::Result<()> {
        if self.kafka_circuit_breaker.is_open() {
            warn!("Kafka 熔断中，消息丢弃");
            return Err(anyhow::anyhow!("Circuit breaker open"));
        }

        match self.send_to_node_via_mq(...).await {
            Ok(_) => {
                self.kafka_circuit_breaker.record_success();
                Ok(())
            }
            Err(e) => {
                self.kafka_circuit_breaker.record_failure();
                Err(e)
            }
        }
    }
}
```

**工作量**：3 天

---

### 阶段二：性能优化（P1 - 重要）

#### 2.1 心跳检查优化 🟡
**目标**：降低心跳检查 CPU 开销

**方案**：时间轮算法
```rust
// 使用 tokio-timer-wheel 替代全量扫描
struct HeartbeatWheel {
    wheel: TimerWheel<SessionId>,
}

impl SessionManager {
    pub fn touch_session(&self, session_id: &SessionId) {
        // 重置定时器
        self.heartbeat_wheel.reset_timer(session_id, Duration::from_secs(30));
    }
}
```

**性能提升**：心跳检查从 O(n) 降到 O(1)

**工作量**：2 天

---

#### 2.2 消息批量推送 🟡
**目标**：减少系统调用次数

**方案**：
```rust
impl SessionManager {
    pub async fn send_to_users_batch(&self, uids: Vec<u64>, msg: Message) -> usize {
        // 按节点分组
        let mut node_sessions: HashMap<String, Vec<Arc<Session>>> = HashMap::new();

        for uid in uids {
            for session in self.get_user_sessions(uid) {
                node_sessions.entry(session.id.clone())
                    .or_default()
                    .push(session);
            }
        }

        // 批量发送
        let mut total_sent = 0;
        for (_, sessions) in node_sessions {
            for session in sessions {
                if session.try_send(msg.clone()).is_ok() {
                    total_sent += 1;
                }
            }
        }
        total_sent
    }
}
```

**性能提升**：批量推送吞吐量提升 3-5 倍

**工作量**：2 天

---

#### 2.3 零拷贝消息序列化 🟡
**目标**：减少内存分配和拷贝

**方案**：
```rust
// 使用 bytes::Bytes 替代 String
pub async fn send_to_user(&self, uid: UserId, msg: Bytes) -> usize {
    let ws_msg = axum::extract::ws::Message::Binary(msg);
    // ...
}

// 预序列化消息
let serialized = serde_json::to_vec(&msg)?;
let bytes = Bytes::from(serialized);
```

**性能提升**：减少 30% 内存分配

**工作量**：1 天

---

### 阶段三：可观测性（P1 - 重要）

#### 3.1 Metrics 指标 📊
**目标**：实时监控服务状态

**指标清单**：
```rust
// 使用 prometheus 库
lazy_static! {
    static ref ACTIVE_CONNECTIONS: IntGauge = register_int_gauge!(
        "websocket_active_connections",
        "当前活跃连接数"
    ).unwrap();

    static ref MESSAGE_SENT_TOTAL: IntCounter = register_int_counter!(
        "websocket_message_sent_total",
        "发送消息总数"
    ).unwrap();

    static ref MESSAGE_LATENCY: Histogram = register_histogram!(
        "websocket_message_latency_seconds",
        "消息推送延迟"
    ).unwrap();

    static ref REDIS_ERRORS: IntCounter = register_int_counter!(
        "websocket_redis_errors_total",
        "Redis 错误次数"
    ).unwrap();
}
```

**工作量**：3 天

---

#### 3.2 分布式追踪 🔍
**目标**：追踪跨节点消息链路

**方案**：集成 OpenTelemetry
```rust
use opentelemetry::trace::{Tracer, SpanKind};

impl PushService {
    pub async fn send_push_msg(&self, msg: WsBaseResp, uid_list: Vec<u64>, cuid: u64) -> anyhow::Result<()> {
        let tracer = global::tracer("websocket");
        let span = tracer.start_with_context("push_message", &Context::current());

        // 业务逻辑
        let result = self.send_push_msg_internal(msg, uid_list, cuid).await;

        span.end();
        result
    }
}
```

**工作量**：3 天

---

#### 3.3 日志结构化 📝
**目标**：便于日志分析和告警

**方案**：
```rust
// 使用 tracing 的结构化日志
info!(
    uid = uid,
    client_id = %client_id,
    node_id = %node_id,
    session_count = session_count,
    "会话注册成功"
);

// 配置 JSON 输出
tracing_subscriber::fmt()
    .json()
    .with_current_span(true)
    .init();
```

**工作量**：1 天

---

### 阶段四：高可用（P2 - 可选）

#### 4.1 Redis 哨兵/集群 🔴
**目标**：Redis 高可用

**方案**：
- 使用 Redis Sentinel 实现主从切换
- 或使用 Redis Cluster 实现分片

**工作量**：5 天

---

#### 4.2 Kafka 多副本 🔴
**目标**：Kafka 高可用

**配置**：
```properties
# Kafka topic 配置
replication.factor=3
min.insync.replicas=2
```

**工作量**：1 天（配置）

---

#### 4.3 节点故障自动摘除 🟡
**目标**：故障节点自动从路由表移除

**方案**：
```rust
// 定期检查 Nacos 实例健康状态
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;

        let active_nodes = get_all_active_nodes().await;
        let cached_nodes = router_cache.get_all_nodes();

        // 找出已下线的节点
        for node in cached_nodes {
            if !active_nodes.contains(&node) {
                // 清理该节点的所有路由记录
                router_cache.remove_node_routes(&node).await;
            }
        }
    }
});
```

**工作量**：2 天

---

## 测试计划

### 单元测试
- [ ] SessionManager 并发测试
- [ ] Redis 路由表一致性测试
- [ ] 心跳超时测试
- [ ] 消息推送测试

### 集成测试
- [ ] 跨节点消息路由测试
- [ ] Kafka 消息丢失测试
- [ ] Redis 故障恢复测试

### 压力测试
- [ ] 10 万并发连接测试
- [ ] 10 万 QPS 消息推送测试
- [ ] 节点故障切换测试
- [ ] 长连接稳定性测试（24 小时）

### 混沌测试
- [ ] 随机杀死节点
- [ ] 网络分区测试
- [ ] Redis/Kafka 故障注入

---

## 部署建议

### 硬件配置（单节点）
- **CPU**：8 核
- **内存**：16GB
- **网络**：万兆网卡
- **磁盘**：SSD（日志存储）

### 容器资源限制
```yaml
resources:
  requests:
    cpu: 4
    memory: 8Gi
  limits:
    cpu: 8
    memory: 16Gi
```

### 系统参数调优
```bash
# 增加文件描述符限制
ulimit -n 1000000

# TCP 参数优化
sysctl -w net.ipv4.tcp_tw_reuse=1
sysctl -w net.ipv4.tcp_fin_timeout=30
sysctl -w net.core.somaxconn=65535
sysctl -w net.ipv4.tcp_max_syn_backlog=8192
```

---

## 监控告警

### 关键指标告警
- 连接数 > 80,000（单节点）
- 消息推送延迟 > 100ms
- Redis 错误率 > 1%
- Kafka 消费延迟 > 1000 条
- 内存使用率 > 80%
- CPU 使用率 > 80%

### 告警渠道
- 钉钉/企业微信
- PagerDuty
- 邮件

---

## 总结

### 优先级排序
1. **P0（必须）**：Redis 优化、竞态修复、优雅关闭、熔断机制（10 天）
2. **P1（重要）**：性能优化、可观测性（11 天）
3. **P2（可选）**：高可用、混沌测试（8 天）

### 总工作量
- **最小可行版本（MVP）**：10 天
- **生产就绪版本**：21 天
- **企业级版本**：29 天

### 预期性能（优化后）
- **单节点**：100,000 并发连接，50,000 msg/s
- **3 节点集群**：300,000 并发连接，150,000 msg/s
- **消息延迟**：本地 < 1ms，跨节点 < 10ms
- **可用性**：99.99%（年停机时间 < 53 分钟）

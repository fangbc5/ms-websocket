# ms-websocket 优化日志

## 2026-03-06 - 第二步优化：Kafka 批量发送

### 改动内容
1. 新增 `batch_send_to_nodes_via_mq()` 方法
   - 将多个节点的消息一次性构建
   - 批量发送到 Kafka，减少网络往返

2. 优化 `send_push_msg()` 方法
   - 先分离本地节点和远程节点
   - 本地节点直接推送
   - 远程节点批量发送

### 性能提升
- **减少循环等待**: 原来逐个 await，现在批量发送
- **减少网络往返**: 多个消息连续发送，减少延迟
- **预期提升**: 跨节点推送延迟降低 30-50%

### 改动文件
- `src/service/push_service.rs` (修改)

### 测试建议
1. 测试多节点推送场景
2. 观察 Kafka 发送延迟
3. 监控消息推送成功率

---

## 2026-03-06 - 第一步优化：本地路由缓存

### 改动内容
1. 新增 `LocalRouterCache` 模块 (`src/cache/local_router_cache.rs`)
   - 使用 DashMap 实现线程安全的本地缓存
   - 默认 TTL 30 秒
   - 自动后台清理过期条目

2. 集成到 `SessionManager`
   - 设备注册时更新本地缓存
   - 设备注销时删除本地缓存

3. 集成到 `PushService`
   - `find_node_device_user()` 优先查询本地缓存
   - 缓存未命中时查询 Redis 并更新缓存
   - 添加缓存命中率监控日志

### 性能提升
- **Redis 查询延迟**: 1-2ms
- **本地缓存查询延迟**: < 0.01ms
- **预期减少**: 90% 的 Redis 查询（稳定运行后）

### 改动文件
- `src/cache/local_router_cache.rs` (新增)
- `src/cache/mod.rs` (修改)
- `src/websocket/session_manager.rs` (修改)
- `src/service/push_service.rs` (修改)

### 测试建议
1. 观察日志中的缓存命中率
2. 监控 Redis QPS 是否下降
3. 测试设备快速重连场景

### 下一步优化
- [x] Kafka 批量发送
- [ ] 消息批量推送（按节点分组）
- [ ] 时间轮心跳检查

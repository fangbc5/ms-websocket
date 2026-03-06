# ms-websocket 优化日志

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
- [ ] Kafka 批量发送
- [ ] 消息批量推送
- [ ] 时间轮心跳检查

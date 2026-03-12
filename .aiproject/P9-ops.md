# P9 — 运维

> 🔴 影响服务稳定性和运维效率。

---

## 1. 健康检查

每个微服务必须暴露 `/health` 端点，供负载均衡器和容器编排探活：

```rust
// router.rs
Router::new()
    .route("/health", get(|| async { Json(R::<()>::ok()) }))
    .nest("/api/v1", api_router)
```

Docker 容器配置 HEALTHCHECK：

```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s \
    CMD curl -f http://localhost:3000/health || exit 1
```

---

## 2. 数据库迁移

推荐使用 `sqlx migrate` 管理 schema 变更：

```bash
# 创建迁移
sqlx migrate add create_user_table

# 运行迁移
sqlx migrate run --database-url mysql://user:pass@localhost/db
```

- 迁移文件放在 `migrations/` 目录，命名格式 `{timestamp}_{描述}.sql`
- 所有 schema 变更必须有版本化的 SQL 脚本
- **禁止直接操作生产数据库**

---

## 3. 优雅停机

`Server::run` 内置优雅停机支持（监听 SIGTERM / SIGINT）。确保：

- 停机前完成进行中的请求处理
- 停机前关闭数据库连接池
- Kafka 消费者停止消费并提交偏移量
- 从 Nacos 注销服务实例

---

## 4. 监控告警

| 指标 | 建议阈值 |
|------|----------|
| API 响应时间 P99 | < 500ms |
| 错误率 | < 1% |
| CPU 使用率 | < 80% |
| 内存使用率 | < 80% |
| 数据库连接池使用率 | < 80% |

---

## 5. 容器资源配置

```yaml
# K8s / Docker Compose 资源限制示例
resources:
  requests:
    memory: "128Mi"
    cpu: "100m"
  limits:
    memory: "512Mi"
    cpu: "500m"
```

- 根据服务负载调整资源限制
- 设置合理的副本数（至少 2 个实例保证高可用）
- 配置 Pod 反亲和性，避免同一节点运行所有副本

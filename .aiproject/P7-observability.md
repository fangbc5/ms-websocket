# P7 — 可观测性

> 🟢 影响问题排查效率和系统可观测能力。

---

## 1. 日志级别

| 级别 | 用于 |
|------|------|
| `error!` | 不可恢复错误（数据库断连、关键操作失败） |
| `warn!` | 可恢复异常（缓存未命中、降级处理） |
| `info!` | 关键业务事件（用户注册、服务启动） |
| `debug!` | 调试信息（SQL 参数、中间变量） |

---

## 2. 结构化日志

**禁止** `println!` / `eprintln!`，使用 `tracing` 结构化日志：

```rust
// ✅ 正确：结构化字段
tracing::info!(user_id = %id, action = "login", "用户登录成功");
tracing::error!(error = ?e, user_id = %id, "查询用户失败");

// ❌ 错误
println!("用户 {} 登录成功", id);
```

---

## 3. 链路追踪

关键路径使用 `#[tracing::instrument]`：

```rust
#[tracing::instrument(skip(self, pool), fields(user_id = %id))]
pub async fn get_user_info(&self, pool: &Pool<MySql>, id: i64) -> Result<User> {
    // 自动记录函数入口、出口和耗时
}
```

---

## 4. 日志格式

| 环境 | 格式 | 配置 |
|------|------|------|
| 开发 | 可读文本 | `APP__LOG__JSON=false` |
| 生产 | JSON | `APP__LOG__JSON=true` |

生产环境建议 JSON 格式，便于日志收集系统（ELK / Loki / Grafana）解析。

---

## 5. 日志规范

- 错误日志必须包含上下文信息（用户 ID、请求 ID、操作名称）
- 日志中**禁止**输出密码、Token、密钥等敏感信息
- 避免在循环中打印大量日志（使用 `debug!` 或采样）
- 文件日志配置滚动策略（按天 / 按大小），防止磁盘耗尽

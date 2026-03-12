# P8 — 性能

> 🟠 影响系统响应速度和资源利用效率。

---

## 1. 数据库性能

- 查询必须使用索引，避免全表扫描
- 批量操作优于循环单条操作
- 大结果集使用分页（`paginate`），禁止一次性加载
- `SELECT` 只查询需要的字段，避免 `SELECT *`（ORM 场景可放宽）

---

## 2. 连接复用

- HTTP 客户端在循环外创建，复用连接
- 数据库连接池由框架管理，禁止手动创建
- gRPC Channel 尽可能复用（避免每次请求重新连接）

```rust
// ✅ 正确：复用客户端
let client = reqwest::Client::new();
for item in items {
    client.post(url).json(&item).send().await?;
}

// ❌ 错误：循环内创建
for item in items {
    let client = reqwest::Client::new();  // 每次都创建，浪费资源
    client.post(url).json(&item).send().await?;
}
```

---

## 3. 异步优化

- I/O 密集操作使用 `tokio::spawn` 或 `tokio::join!` 并发执行
- CPU 密集操作使用 `tokio::task::spawn_blocking` 避免阻塞 runtime
- 非关键路径（缓存刷新、审计日志）使用异步后台任务

---

## 4. 缓存策略

使用 `CacheKeyBuilder` 构建键，**禁止手动拼接**：

```rust
let key = SimpleCacheKeyBuilder::new("user")
    .with_modular("identity").with_field("id")
    .with_value_type(ValueType::Obj)
    .with_expire(Duration::from_secs(3600))
    .key(&[&user_id]);
```

| 策略 | 规则 |
|------|------|
| 写后删除 | 写操作后删除相关缓存 |
| 强制 TTL | 所有缓存必须设过期时间 |
| 缓存穿透 | 查询不存在的数据时，缓存空值并设短 TTL |
| 批量操作 | 批量写操作后使用 `UNLINK` 批量删除 |

---

## 5. Feature 精简

只启用必要的 `fbc-starter` feature，减少编译时间和二进制体积：

```toml
# ✅ 精简
fbc-starter = { path = "../fbc-starter", features = ["nacos", "mysql"] }

# ❌ 冗余
fbc-starter = { path = "../fbc-starter", features = ["nacos", "mysql", "redis", "grpc", "producer", "consumer"] }
```

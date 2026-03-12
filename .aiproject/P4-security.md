# P4 — 安全

> 🟡 影响系统安全性和数据保护。

---

## 1. 通用安全规则

- 错误响应**不暴露** SQL / 堆栈 / 文件路径 / 内部错误详情
- 密码使用 argon2 / bcrypt，**禁止** MD5 / 明文存储
- Handler 层**校验用户输入**（长度、格式、范围）
- 定期执行 `cargo audit` 检查依赖安全漏洞
- 敏感配置（密钥、密码、Token）**禁止硬编码**，必须通过环境变量传入

---

## 2. HTTP 安全

| 机制 | 说明 |
|------|------|
| CORS | 使用 `tower-http::CorsLayer` 配置 |
| 认证 | 自定义 middleware 验证 Token |
| 限流 | 关键接口使用限流中间件保护 |
| 请求体大小 | 限制请求体大小，防止大载荷攻击 |

```rust
use tower_http::cors::{CorsLayer, Any};

let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);

Router::new()
    .nest("/api/v1", api_router)
    .layer(cors)
```

---

## 3. 数据安全

- SQL 查询使用参数化（`QueryBuilder` / `sqlx::query!`），禁止字符串拼接
- 用户输入必须转义或校验后才能使用
- 日志中禁止输出密码、Token 等敏感信息
- 生产环境关闭 debug 日志级别

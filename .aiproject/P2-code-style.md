# P2 — 代码风格

> 🔵 影响代码一致性和可读性。

---

## 1. 命名规范

| 元素 | 风格 | 示例 |
|------|------|------|
| 包名 | `ms-{名称}` | `ms-identity` |
| 结构体 | `PascalCase` | `UserService` |
| 函数 | `snake_case` | `get_user_info` |
| 常量 | `SCREAMING_SNAKE` | `USER_NOT_FOUND` |
| 路由 | `kebab-case` | `/user-tenants` |
| 环境变量 | `APP__XX__YY` | `APP__SERVER__PORT` |
| 错误枚举变体 | `PascalCase` | `UserNotFound` |

---

## 2. 导入顺序

```rust
// 1. 标准库
use std::sync::Arc;

// 2. 第三方
use axum::{Json, extract::State};
use serde::{Serialize, Deserialize};

// 3. fbc-starter
use fbc_starter::{AppResult, R};

// 4. 本 crate
use crate::state::AppState;
use super::service::UserService;
```

---

## 3. 文档与注释

- 注释使用**中文**
- `pub` 项必须添加 `///` 文档注释
- 可选字段使用 `skip_serializing_if`

```rust
/// 用户信息响应
#[derive(Serialize)]
pub struct UserInfo {
    /// 用户 ID
    pub id: i64,
    /// 用户名
    pub username: String,
    /// 邮箱（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}
```

---

## 4. Clippy 配置

在 workspace `Cargo.toml` 中配置 lint 规则：

```toml
[workspace.lints.clippy]
print_stdout = "deny"       # 禁止 println!
print_stderr = "warn"       # 警告 eprintln!
unwrap_used = "warn"        # 警告 unwrap()
```

每个微服务的 `Cargo.toml` 继承：

```toml
[lints]
workspace = true
```

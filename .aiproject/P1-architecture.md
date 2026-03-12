# P1 — 架构设计

> 🟠 影响代码组织和长期可维护性。

---

## 1. 分层架构

```
Handler → Service → Repository / CRUD
```

| 层 | 允许调用 | 禁止调用 |
|----|----------|----------|
| Handler | Service | Repository、sqlx |
| Service | Repository、CRUD、**其他模块的 Service** | Handler |
| Repository | sqlx、CRUD | Service、Handler |

---

## 2. 跨模块调用

Service 间可互相调用，**禁止循环依赖**。通过 `AppState` 注入：

```rust
// ✅ 正确：通过 AppState 注入
pub async fn create_org(&self, app_state: &AppState, dto: CreateOrgDto) -> Result<()> {
    let user = app_state.user_service.get_user_info(dto.creator_id).await?;
    // ...
}

// ❌ 错误：跨模块直接访问 Repo
use crate::modules::user::repository::UserRepo;
```

---

## 3. 模块导出

```rust
// modules/user/mod.rs
mod handler; mod model; mod repository; mod service;
pub use model::*;
pub use repository::UserRepo;
pub use service::UserService;
pub use handler::*;
```

---

## 4. Repository

**只实现 CRUD trait 不存在的方法。** 以下方法由 `sqlxplus::CRUD` 提供，禁止重复实现：

| 方法 | 说明 |
|------|------|
| `T::find_by_id(pool, id)` | 按主键查询 |
| `T::find_one(pool, builder)` | 条件查单条 |
| `T::find_list(pool, builder)` | 条件查列表 |
| `T::paginate(pool, builder, page, size)` | 分页查询 |
| `entity.insert(pool)` | 插入 |
| `entity.update(pool)` | 更新 |
| `T::delete_by_id(pool, id)` | 按主键删除 |

自定义查询用 `QueryBuilder`：

```rust
pub struct UserRepo;
impl UserRepo {
    pub async fn find_by_username(pool: &Pool<MySql>, name: &str) -> Result<User> {
        let builder = QueryBuilder::new("SELECT * FROM `user`").and_eq("username", name);
        User::find_one(pool, builder).await?.ok_or_else(|| error::user_not_found())
    }
}
```

---

## 5. Service

持有 `Arc<DbPool>`，调用 Repository 和 CRUD 方法。**禁止直接写 SQL。**

```rust
// 事务
sqlxplus::with_transaction(self.db_pool.mysql_pool(), |tx| async move {
    User::find_by_id(tx.as_mysql_executor(), id).await?;
    user.update(tx.as_mysql_executor()).await?;
    Ok(())
}).await

// 分页
let result = User::paginate(pool, builder, page, size).await?;
Ok((result.items, result.total as i64))
```

---

## 6. 实体定义

使用 `sqlxplus` 派生宏，字段全部 `Option<T>`（插入时跳过 None、更新时部分更新）：

```rust
#[derive(Debug, Default, sqlx::FromRow, serde::Serialize, serde::Deserialize,
         sqlxplus::ModelMeta, sqlxplus::CRUD)]
#[model(table = "user", pk = "id", soft_delete = "is_del")]
pub struct User {
    pub id: Option<i64>,
    pub username: Option<String>,
    pub is_del: Option<i32>,
}
```

连接池从 `fbc_app_state` 获取，用 `DbPool::from_mysql_pool()` 转换。禁止手动创建连接。

---

## 7. 配置管理

环境变量 `APP__` 前缀，`__` 分隔层级。每个服务提供 `.env.example`：

```rust
impl XxxConfig {
    pub fn new(base: BaseConfig) -> Result<Self> {
        Ok(Self {
            base,
            // 敏感配置必须 expect（防止生产遗漏）
            jwt_secret: std::env::var("APP__SERVICE__JWT__SECRET")
                .expect("缺少 APP__SERVICE__JWT__SECRET"),
            // 非敏感配置可设默认值
            page_size: std::env::var("APP__SERVICE__PAGE_SIZE")
                .unwrap_or_else(|_| "20".into()).parse().unwrap_or(20),
        })
    }
}
```

禁止硬编码敏感信息，禁止重复使用 config crate 加载。

# P0 — 项目基础

> 🔴 新建微服务必须遵守。违反将导致项目无法正常集成到工作空间。

---

## 1. 依赖管理

### Workspace 依赖

所有微服务加入 `hula-server` 工作空间，共享依赖使用 `workspace = true`。
**禁止**自行指定 workspace 中已声明的版本。

```toml
[package]
name = "ms-xxx"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
fbc-starter = { path = "../fbc-starter", features = ["nacos", "mysql"] }
axum.workspace = true
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true
sqlx.workspace = true
sqlxplus.workspace = true
```

### 版本管理

**所有依赖版本以 workspace 根目录 `Cargo.toml` 中 `[workspace.dependencies]` 声明为准。**
微服务 `Cargo.toml` 中只写 `xxx.workspace = true`，禁止覆盖版本号。

### Feature 选择

只启用必要 feature：`mysql` / `redis` / `nacos` / `balance` / `grpc` / `producer` / `consumer`

---

## 2. 项目结构

```
ms-xxx/
├── Cargo.toml
├── build.rs               # 如需 gRPC，配置 tonic_build
├── .env / .env.example
├── Dockerfile             # Docker 多阶段构建
├── proto/                 # 如需 gRPC，存放 .proto 文件
│   └── xxx.proto
└── src/
    ├── main.rs            # 仅启动逻辑
    ├── lib.rs             # 可选：集成测试需导出的模块
    ├── config.rs          # 服务配置
    ├── error.rs           # 服务错误
    ├── state.rs           # AppState
    ├── router.rs          # 路由定义
    ├── grpc/              # 可选：gRPC 服务实现
    ├── middleware/         # 可选：自定义中间件
    └── modules/           # 业务模块
        └── {模块名}/
            ├── mod.rs
            ├── model/
            │   ├── entity/
            │   └── dto.rs
            ├── repository.rs
            ├── service.rs
            └── handler.rs
```

**核心规则：**
- DTO / Handler / Repository **必须**在 `modules/{模块}/` 内，禁止独立目录
- `grpc/`、`middleware/`、`lib.rs` 等按需添加

---

## 3. 启动模式

使用 `Server::run` 启动，**禁止手动初始化日志/数据库/Nacos**。

> **启动阶段**：依赖未就绪属不可恢复错误，允许 `panic!` / `expect()`。
> **运行时**：必须返回 `Result`，禁止 `unwrap()` / `expect()`。

```rust
#[tokio::main]
async fn main() -> AppResult<()> {
    Server::run(|builder| {
        let fbc_app_state = builder.app_state().clone();
        let mysql_pool = fbc_app_state.mysql.as_ref()
            .expect("MySQL 连接池未初始化").clone();
        let db_pool = Arc::new(DbPool::from_mysql_pool(mysql_pool)
            .expect("创建 DbPool 失败"));
        let config = config::XxxConfig::new(builder.config().clone())
            .expect("加载配置失败");
        let app_state = Arc::new(state::AppState::new(fbc_app_state, db_pool, config));
        builder.http_router(router::create_router(app_state))
    }).await
}
```

---

## 4. AppState

```rust
#[derive(Clone)]
pub struct AppState {
    pub fbc: FbcAppState,       // 必须持有框架 AppState
    pub db_pool: Arc<DbPool>,
    pub config: XxxConfig,
    pub user_service: Arc<UserService>,
}
```

- 必须 `#[derive(Clone)]`，每个 Service 用 `Arc` 包装
- 必须持有 `fbc_app_state`（框架通过它访问 Redis、Kafka 等）

# P3 — API 与集成

> 🟣 影响前后端交互一致性和微服务间通信。

---

## 1. 响应格式

所有 HTTP 接口使用 `R<T>` 包装，分页使用 `CursorPageBaseResp<T>`：

```rust
R::ok_with_data(data)              // 成功
R::fail_with_message(msg)          // 失败
R::fail_with_code(code, msg)       // 带错误码
CursorPageBaseResp::init(next_cursor, is_last, list, total)  // 分页
```

禁止自定义响应/分页结构体。

---

## 2. 错误处理

每个微服务定义错误枚举，通过 `From` 转换到 `AppError`：

```rust
#[derive(Debug, thiserror::Error)]
pub enum XxxError {
    #[error("用户不存在")]  UserNotFound,
    #[error("数据库错误: {0}")]  DatabaseError(String),
}

impl From<XxxError> for AppError {
    fn from(err: XxxError) -> Self {
        match err {
            XxxError::UserNotFound => AppError::biz_error(4001, err.to_string()),
            XxxError::DatabaseError(msg) => AppError::common_error(5001, msg),
        }
    }
}
```

工厂方法：`biz_error(code, msg)` / `common_error(code, msg)` / `customer_error(code, msg)`

`AppError` 已实现 `IntoResponse`，Handler 中可用 `?` 运算符简化。

---

## 3. 错误码分配

| 段位 | 微服务 | 示例 |
|------|--------|------|
| 40xx | ms-identity | 4001 用户不存在 |
| 41xx | ms-organization | 4101 部门不存在 |
| 42xx | ms-auth | 4201 认证失败 |
| 43xx | ms-im | 4301 消息发送失败 |
| 44xx | ms-notify | 4401 通知发送失败 |
| 5xxx | 通用系统错误 | 5001 数据库错误 |

---

## 4. 路由

```rust
Router::new().nest("/api/v1", Router::new()
    .nest("/users", Router::new()
        .route("/{id}", get(user::get_user))
        .route("/", post(user::create_user))
    )
).with_state(app_state)
```

路径 `kebab-case`，参数 `{id}`（非 `:id`），版本 `/api/v1/...`。

---

## 5. gRPC — Proto 与调用

Proto 文件存放在**提供服务方**的 `proto/` 目录，消费方通过相对路径引用：

```rust
// 提供方 build.rs: build_server(true), build_client(false)
// 消费方 build.rs: build_server(false), build_client(true)
tonic_build::configure()
    .compile_protos(&["../ms-identity/proto/identity.proto"], &["../ms-identity/proto"])?;
```

调用使用负载均衡器：

```rust
use fbc_starter::{get_load_balancer, LoadBalancer};
let endpoint = get_load_balancer("ms-identity")
    .next_endpoint().ok_or_else(|| anyhow!("无可用实例"))?;
let mut client = XxxClient::new(endpoint.endpoint.connect().await?);
```

---

## 6. Kafka — 消费者

实现 `KafkaMessageHandler` trait，在 `Server::run` 中注册：

```rust
#[async_trait]
impl KafkaMessageHandler for NotificationHandler {
    fn topics(&self) -> Vec<String> { vec!["notification".into()] }
    fn group_id(&self) -> String { "ms-notify-group".into() }
    async fn handle(&self, message: Message) {
        // 处理消息
    }
}

// 注册
builder.with_kafka_handler(Arc::new(handler)).http_router(router)
```

---

## 7. Kafka — 生产者

通过 `fbc_app_state` 获取（需启用 `producer` feature）：

```rust
let producer = app_state.fbc.kafka_producer.as_ref().expect("Kafka Producer 未初始化");
let msg = Message::new("notification", "ms-identity", serde_json::json!({ "event": "created" }));
producer.publish("notification", msg).await?;
```

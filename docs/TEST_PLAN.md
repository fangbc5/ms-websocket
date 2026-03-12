# ms-websocket 服务测试覆盖计划

## 目标

为 ms-websocket 服务实现 100% 的测试覆盖，确保 WebSocket 实时通信功能的稳定性和可靠性。

## 项目结构分析

```
ms-websocket/
├── src/
│   ├── cache/                    # 缓存层 (9 个文件)
│   ├── enums/                    # 枚举类型 (2 个文件)
│   ├── grpc/                     # gRPC 客户端 (2 个文件)
│   ├── kafka/                    # Kafka 消息处理 (4 个文件)
│   ├── model/                    # 数据模型 (30+ 个文件)
│   ├── service/                  # 业务服务 (5 个文件)
│   ├── websocket/                # WebSocket 核心 (15 个文件)
│   ├── main.rs
│   ├── routes.rs
│   ├── state.rs
│   └── types.rs
└── tests/                        # 测试目录（待创建）
```

**代码统计**: 78 个 Rust 源文件

---

## 测试策略

### 测试层次

1. **单元测试** (Unit Tests)
   - 位置: 每个模块的 `#[cfg(test)] mod tests`
   - 覆盖: 纯函数、工具类、数据结构
   - 目标: 100% 代码覆盖

2. **集成测试** (Integration Tests)
   - 位置: `tests/` 目录
   - 覆盖: Service 层、消息处理链、会话管理
   - 目标: 核心业务流程 100%

3. **端到端测试** (E2E Tests)
   - 位置: `tests/e2e/` 目录
   - 覆盖: WebSocket 连接、消息收发、多设备场景
   - 目标: 主要用户场景 100%

---

## 详细测试计划

## Phase 1: 核心基础设施测试

### 1.1 时间轮 (timing_wheel.rs)

**状态**: ✅ 已有基础测试

**现有测试**:
- [x] 基本添加/移除
- [x] Tick 超时检测
- [x] 刷新会话

**需补充测试**:
- [ ] 并发添加/移除
- [ ] 大量会话性能测试 (10,000+ 会话)
- [ ] 边界条件 (槽位边界、环绕)
- [ ] 内存泄漏测试
- [ ] 多线程竞态条件

**测试文件**: `src/websocket/timing_wheel.rs` (已有测试)

**新增测试**:
```rust
#[tokio::test]
async fn test_timing_wheel_concurrent_operations() {
    // 并发添加/移除测试
}

#[tokio::test]
async fn test_timing_wheel_performance() {
    // 10,000 会话性能测试
}

#[tokio::test]
async fn test_timing_wheel_boundary_conditions() {
    // 槽位边界测试
}
```

---

### 1.2 会话管理器 (session_manager.rs)

**核心功能**:
- 会话注册/清理
- 用户→设备→会话三级映射
- Redis 路由表同步
- 心跳检查

**测试项**:

#### 1.2.1 会话生命周期
- [ ] 会话创建和注册
- [ ] 会话查找 (按 session_id/uid/client_id)
- [ ] 会话清理
- [ ] 会话超时自动清理
- [ ] 会话心跳刷新

#### 1.2.2 多设备管理
- [ ] 同一用户多设备连接
- [ ] 同一设备多会话连接
- [ ] 设备断开时清理所有会话
- [ ] 用户所有设备断开时清理路由

#### 1.2.3 消息发送
- [ ] 发送到单个会话
- [ ] 发送到用户所有设备
- [ ] 发送到指定设备
- [ ] 发送失败处理 (通道满/会话已关闭)

#### 1.2.4 Redis 路由同步
- [ ] 首次连接注册到 Redis
- [ ] 断开连接从 Redis 注销
- [ ] 本地缓存同步
- [ ] Redis 连接失败处理

#### 1.2.5 并发安全
- [ ] 并发注册会话
- [ ] 并发清理会话
- [ ] 并发发送消息
- [ ] 竞态条件测试

**测试文件**: `tests/session_manager_tests.rs`

**示例测试**:
```rust
#[tokio::test]
async fn test_session_registration() {
    let manager = SessionManager::new();
    let (tx, _rx) = mpsc::channel(1000);
    let (shutdown_tx, _shutdown_rx) = mpsc::channel(1);

    let session = Arc::new(Session::new(
        "session1".to_string(),
        1001,
        "device1".to_string(),
        tx,
        shutdown_tx,
    ));

    manager.register_session(session.clone());

    assert_eq!(manager.get_session_count(), 1);
    assert_eq!(manager.get_user_sessions(1001).len(), 1);
}

#[tokio::test]
async fn test_multi_device_sessions() {
    // 测试同一用户多设备场景
}

#[tokio::test]
async fn test_session_timeout_cleanup() {
    // 测试会话超时自动清理
}
```

---

### 1.3 消息处理链 (processor/message_chain.rs)

**核心功能**:
- 责任链模式
- 消息路由到处理器
- 错误处理

**测试项**:
- [ ] 消息解析成功
- [ ] 消息解析失败 (格式错误)
- [ ] 找到匹配的处理器
- [ ] 未找到处理器
- [ ] 处理器执行成功
- [ ] 处理器执行失败
- [ ] 处理器 panic 捕获
- [ ] 多个处理器优先级

**测试文件**: `tests/message_chain_tests.rs`

---

### 1.4 消息处理器 (processor/)

#### 1.4.1 心跳处理器 (heartbeat.rs)
**测试项**:
- [ ] 心跳消息识别
- [ ] 会话心跳刷新
- [ ] 心跳响应发送

**测试文件**: `tests/processor/heartbeat_tests.rs`

#### 1.4.2 ACK 处理器 (ack.rs)
**测试项**:
- [ ] ACK 消息处理
- [ ] 消息确认逻辑

**测试文件**: `tests/processor/ack_tests.rs`

#### 1.4.3 已读处理器 (read.rs)
**测试项**:
- [ ] 已读消息处理
- [ ] 已读状态更新

**测试文件**: `tests/processor/read_tests.rs`

#### 1.4.4 视频通话处理器 (meet/)
**文件**: `video.rs`, `video_call.rs`, `media_control.rs`, `quality_monitor.rs`, `room_admin.rs`

**测试项**:
- [ ] 视频信令转发
- [ ] 通话请求/接受/拒绝
- [ ] 媒体控制 (静音/关闭摄像头)
- [ ] 网络质量监控
- [ ] 房间管理员操作

**测试文件**: `tests/processor/meet_tests.rs`

---

## Phase 2: 业务服务测试

### 2.1 视频聊天服务 (video_chat_service.rs)

**核心功能**:
- 房间管理 (创建/加入/离开)
- 成员管理
- 信令转发
- Redis 缓存操作

**测试项**:

#### 2.1.1 房间生命周期
- [ ] 创建群视频房间
- [ ] 用户加入房间
- [ ] 用户离开房间
- [ ] 房间为空时自动清理
- [ ] 房间数据清理

#### 2.1.2 成员管理
- [ ] 获取房间成员列表
- [ ] 检查用户是否在房间中
- [ ] 获取用户加入的所有房间
- [ ] 房间成员上限检查

#### 2.1.3 信令转发
- [ ] 转发视频信令
- [ ] 转发媒体控制信令
- [ ] 批量推送消息

#### 2.1.4 房间权限
- [ ] 检查房间管理员
- [ ] 全体静音设置
- [ ] 屏幕共享控制

#### 2.1.5 Redis 缓存
- [ ] 房间成员缓存 (SADD/SREM/SMEMBERS)
- [ ] 用户房间缓存
- [ ] 缓存一致性
- [ ] Redis 连接失败处理

**测试文件**: `tests/video_chat_service_tests.rs`

**Mock 依赖**:
- Redis (使用 testcontainers 或 mock)
- PushService
- RoomMetadataService

**示例测试**:
```rust
#[tokio::test]
async fn test_join_room() {
    let (app_state, push_service, room_metadata_service) = setup_mocks().await;
    let service = VideoChatService::new(app_state, push_service, room_metadata_service);

    let room = Room { id: 1, room_type: 2, ..Default::default() };
    let result = service.join_room(1001, room).await;

    assert!(result.is_ok());

    // 验证 Redis 缓存
    let members = service.get_room_members(1).await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0], 1001);
}

#[tokio::test]
async fn test_leave_room_cleanup() {
    // 测试最后一个用户离开时房间清理
}
```

---

### 2.2 推送服务 (push_service.rs)

**测试项**:
- [ ] 单用户推送
- [ ] 批量用户推送
- [ ] 本地会话推送
- [ ] 跨节点推送 (Kafka)
- [ ] 推送失败重试

**测试文件**: `tests/push_service_tests.rs`

---

### 2.3 房间元数据服务 (room_metadata_service.rs)

**测试项**:
- [ ] 房间元数据存储/读取
- [ ] 房间管理员管理
- [ ] 房间状态管理 (关闭/活跃)
- [ ] 全体静音状态
- [ ] 屏幕共享状态

**测试文件**: `tests/room_metadata_service_tests.rs`

---

### 2.4 房间超时服务 (room_timeout_service.rs)

**测试项**:
- [ ] 房间活跃时间刷新
- [ ] 房间超时检测
- [ ] 超时房间清理
- [ ] 定时任务调度

**测试文件**: `tests/room_timeout_service_tests.rs`

---

### 2.5 会话恢复服务 (session_recovery_service.rs)

**测试项**:
- [ ] 会话状态保存
- [ ] 断线重连恢复
- [ ] 未读消息推送
- [ ] 恢复超时处理

**测试文件**: `tests/session_recovery_service_tests.rs`

---

## Phase 3: 缓存层测试

### 3.1 缓存键构建器

**文件**:
- `router_cache_key_builder.rs`
- `room_metadata_cache_key_builder.rs`
- `video_rooms_cache_key_builder.rs`
- `user_rooms_cache_key_builder.rs`
- `room_admin_metadata_cache_key_builder.rs`
- `close_room_cache_key_builder.rs`

**测试项**:
- [ ] 缓存键格式正确
- [ ] 缓存键唯一性
- [ ] 参数验证
- [ ] 边界条件

**测试文件**: `tests/cache_key_builder_tests.rs`

**示例测试**:
```rust
#[test]
fn test_router_cache_key_builder() {
    let key = RouterCacheKeyBuilder::build_device_node_map("tenant1".to_string());
    assert_eq!(key.key, "ws:router:device_node_map:tenant1");
    assert!(key.ttl.is_none()); // 永久有效
}

#[test]
fn test_video_rooms_cache_key_builder() {
    let key = VideoRoomsCacheKeyBuilder::build(12345);
    assert_eq!(key.key, "ws:video:room:12345:members");
}
```

---

### 3.2 本地路由缓存 (local_router_cache.rs)

**测试项**:
- [ ] 缓存设置/获取
- [ ] 缓存删除
- [ ] 缓存过期
- [ ] 并发访问
- [ ] 内存限制

**测试文件**: `tests/local_router_cache_tests.rs`

---

## Phase 4: Kafka 消息处理测试

### 4.1 消息登录处理器 (msg_login_handler.rs)

**测试项**:
- [ ] 登录消息解析
- [ ] 会话创建
- [ ] 在线状态更新

**测试文件**: `tests/kafka/msg_login_handler_tests.rs`

---

### 4.2 推送处理器 (push_handler.rs)

**测试项**:
- [ ] 推送消息解析
- [ ] 路由查找
- [ ] 消息转发
- [ ] 批量推送

**测试文件**: `tests/kafka/push_handler_tests.rs`

---

### 4.3 扫码成功处理器 (scan_success_handler.rs)

**测试项**:
- [ ] 扫码消息处理
- [ ] 通知相关会话

**测试文件**: `tests/kafka/scan_success_handler_tests.rs`

---

## Phase 5: 数据模型测试

### 5.1 DTO 模型

**文件**: `model/dto/*.rs`

**测试项**:
- [ ] 序列化/反序列化
- [ ] 字段验证
- [ ] 默认值
- [ ] 边界条件

**测试文件**: `tests/model/dto_tests.rs`

---

### 5.2 VO 模型

**文件**: `model/vo/*.rs` (20+ 个文件)

**测试项**:
- [ ] 序列化/反序列化
- [ ] JSON 格式正确性
- [ ] 字段完整性

**测试文件**: `tests/model/vo_tests.rs`

---

### 5.3 实体模型

**文件**: `model/entity/*.rs`

**测试项**:
- [ ] 实体创建
- [ ] 字段验证
- [ ] 关系映射

**测试文件**: `tests/model/entity_tests.rs`

---

## Phase 6: 枚举类型测试

### 6.1 消息类型枚举 (ws_req_type.rs, ws_push_type.rs)

**测试项**:
- [ ] 枚举值正确性
- [ ] 枚举转换 (i32 ↔ Enum)
- [ ] 所有枚举值覆盖

**测试文件**: `tests/enum_tests.rs`

**示例测试**:
```rust
#[test]
fn test_ws_msg_type_enum() {
    assert_eq!(WsMsgTypeEnum::Heartbeat.as_i32(), 1);
    assert_eq!(WsMsgTypeEnum::Login.as_i32(), 2);
    // ... 测试所有枚举值
}

#[test]
fn test_ws_msg_type_from_i32() {
    assert_eq!(WsMsgTypeEnum::from_i32(1), Some(WsMsgTypeEnum::Heartbeat));
    assert_eq!(WsMsgTypeEnum::from_i32(999), None);
}
```

---

## Phase 7: 端到端测试

### 7.1 WebSocket 连接测试

**测试场景**:
- [ ] 建立 WebSocket 连接
- [ ] 发送登录消息
- [ ] 接收登录响应
- [ ] 发送心跳
- [ ] 接收心跳响应
- [ ] 正常断开连接
- [ ] 异常断开连接

**测试文件**: `tests/e2e/websocket_connection_tests.rs`

**示例测试**:
```rust
#[tokio::test]
async fn test_websocket_connection_lifecycle() {
    // 1. 启动测试服务器
    let server = start_test_server().await;

    // 2. 建立 WebSocket 连接
    let (mut ws_stream, _) = connect_async(&server.ws_url()).await.unwrap();

    // 3. 发送登录消息
    let login_msg = json!({
        "type": 2,
        "data": {
            "token": "test_token",
            "clientId": "device1"
        }
    });
    ws_stream.send(Message::Text(login_msg.to_string())).await.unwrap();

    // 4. 接收登录响应
    let response = ws_stream.next().await.unwrap().unwrap();
    assert!(response.is_text());

    // 5. 发送心跳
    let heartbeat_msg = json!({"type": 1});
    ws_stream.send(Message::Text(heartbeat_msg.to_string())).await.unwrap();

    // 6. 接收心跳响应
    let response = ws_stream.next().await.unwrap().unwrap();
    assert!(response.is_text());

    // 7. 关闭连接
    ws_stream.close(None).await.unwrap();
}
```

---

### 7.2 多设备场景测试

**测试场景**:
- [ ] 同一用户多设备同时在线
- [ ] 消息推送到所有设备
- [ ] 单个设备断开
- [ ] 所有设备断开

**测试文件**: `tests/e2e/multi_device_tests.rs`

---

### 7.3 视频通话场景测试

**测试场景**:
- [ ] 创建视频房间
- [ ] 多用户加入房间
- [ ] 视频信令转发
- [ ] 媒体控制 (静音/关闭摄像头)
- [ ] 用户离开房间
- [ ] 房间关闭

**测试文件**: `tests/e2e/video_call_tests.rs`

---

### 7.4 消息推送场景测试

**测试场景**:
- [ ] 单聊消息推送
- [ ] 群聊消息推送
- [ ] 离线消息推送
- [ ] 跨节点推送

**测试文件**: `tests/e2e/message_push_tests.rs`

---

### 7.5 压力测试

**测试场景**:
- [ ] 1,000 并发连接
- [ ] 10,000 并发连接
- [ ] 100,000 并发连接
- [ ] 高频消息发送 (1000 msg/s)
- [ ] 内存使用监控
- [ ] CPU 使用监控

**测试文件**: `tests/e2e/stress_tests.rs`

---

## 测试基础设施

### 1. 测试依赖

在 `Cargo.toml` 中添加：

```toml
[dev-dependencies]
# 测试框架
tokio-test = "0.4"
mockall = "0.13"
wiremock = "0.6"
fake = "2.9"

# WebSocket 测试
tokio-tungstenite = "0.24"

# Redis 测试
testcontainers = "0.23"
testcontainers-modules = { version = "0.11", features = ["redis"] }

# HTTP 测试
axum-test = "16.7"
tower = { version = "0.5", features = ["util"] }

# 断言增强
assert_matches = "1.5"
pretty_assertions = "1.4"

# 性能测试
criterion = "0.5"
```

---

### 2. 测试辅助工具

创建 `tests/common/mod.rs`:

```rust
// 测试服务器
pub struct TestServer {
    addr: SocketAddr,
    shutdown_tx: mpsc::Sender<()>,
}

impl TestServer {
    pub async fn start() -> Self {
        // 启动测试服务器
    }

    pub fn ws_url(&self) -> String {
        format!("ws://{}/ws", self.addr)
    }
}

// Redis Mock
pub async fn setup_redis() -> RedisContainer {
    // 启动 Redis 容器
}

// 测试数据生成
pub fn create_test_session() -> Arc<Session> {
    // 创建测试会话
}

pub fn create_test_room() -> Room {
    // 创建测试房间
}
```

---

### 3. Mock 服务

创建 `tests/mocks/`:

```rust
// mock_push_service.rs
pub struct MockPushService {
    // Mock 实现
}

// mock_room_metadata_service.rs
pub struct MockRoomMetadataService {
    // Mock 实现
}

// mock_redis.rs
pub struct MockRedis {
    // Mock 实现
}
```

---

## 测试执行计划

### Week 1: 核心基础设施
- [x] 时间轮测试补充
- [ ] 会话管理器完整测试
- [ ] 消息处理链测试

### Week 2: 消息处理器
- [ ] 心跳/ACK/已读处理器
- [ ] 视频通话处理器
- [ ] 默认处理器

### Week 3: 业务服务
- [ ] 视频聊天服务
- [ ] 推送服务
- [ ] 房间元数据服务
- [ ] 房间超时服务
- [ ] 会话恢复服务

### Week 4: 缓存和 Kafka
- [ ] 缓存层测试
- [ ] Kafka 消息处理测试
- [ ] 本地路由缓存测试

### Week 5: 数据模型和枚举
- [ ] DTO/VO/Entity 测试
- [ ] 枚举类型测试
- [ ] 序列化/反序列化测试

### Week 6: 端到端测试
- [ ] WebSocket 连接测试
- [ ] 多设备场景测试
- [ ] 视频通话场景测试
- [ ] 消息推送场景测试

### Week 7: 压力测试和优化
- [ ] 并发连接压力测试
- [ ] 高频消息压力测试
- [ ] 性能优化
- [ ] 内存泄漏检测

### Week 8: 集成和文档
- [ ] CI/CD 集成
- [ ] 测试文档完善
- [ ] 测试覆盖率报告
- [ ] 测试维护指南

---

## 测试覆盖率目标

| 模块 | 单元测试 | 集成测试 | E2E 测试 | 总体目标 |
|------|---------|---------|---------|---------|
| websocket/session_manager | 100% | 100% | 100% | 100% |
| websocket/timing_wheel | 100% | - | - | 100% |
| websocket/processor | 100% | 100% | 90% | 97% |
| service/ | 100% | 100% | 100% | 100% |
| cache/ | 100% | 90% | - | 95% |
| kafka/ | 100% | 100% | 80% | 93% |
| model/ | 100% | - | - | 100% |
| enums/ | 100% | - | - | 100% |
| **整体** | **100%** | **98%** | **92%** | **97%** |

---

## 测试质量标准

### 1. 代码覆盖率
- 行覆盖率: ≥ 95%
- 分支覆盖率: ≥ 90%
- 函数覆盖率: 100%

### 2. 测试质量
- 每个测试必须有清晰的命名 (`test_<功能>_<场景>`)
- 每个测试必须独立运行
- 测试必须可重复执行
- 单元测试执行时间 < 100ms
- 集成测试执行时间 < 5s

### 3. 测试文档
- 每个测试文件必须有模块级文档
- 复杂测试必须有注释说明测试步骤
- 测试数据必须有说明

---

## CI/CD 集成

### GitHub Actions 配置

创建 `.github/workflows/ms-websocket-test.yml`:

```yaml
name: ms-websocket Tests

on:
  push:
    paths:
      - 'ms-websocket/**'
  pull_request:
    paths:
      - 'ms-websocket/**'

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      redis:
        image: redis:7-alpine
        ports:
          - 6379:6379

    steps:
      - uses: actions/checkout@v4

      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run unit tests
        run: cargo test --package ms-websocket --lib

      - name: Run integration tests
        run: cargo test --package ms-websocket --test '*'

      - name: Generate coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --package ms-websocket --out Xml

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml
          flags: ms-websocket
```

---

## 测试命令

```bash
# 运行所有测试
cargo test --package ms-websocket

# 运行单元测试
cargo test --package ms-websocket --lib

# 运行集成测试
cargo test --package ms-websocket --test '*'

# 运行特定测试
cargo test --package ms-websocket test_session_registration

# 生成覆盖率报告
cargo tarpaulin --package ms-websocket --out Html

# 运行压力测试
cargo test --package ms-websocket --release stress_test -- --ignored

# 运行性能基准测试
cargo bench --package ms-websocket
```

---

## 风险和挑战

### 1. 技术挑战
- WebSocket 异步测试复杂性
- 时间轮并发测试难度
- Redis 依赖的 Mock 复杂度
- Kafka 消息测试的异步性

### 2. 应对措施
- 使用 testcontainers 提供真实 Redis 环境
- 使用 tokio-test 简化异步测试
- 使用 mockall 创建 Mock 服务
- 使用 wiremock 模拟外部服务

---

## 下一步行动

1. **立即开始**:
   - 创建 `tests/` 目录结构
   - 添加测试依赖到 `Cargo.toml`
   - 创建测试辅助工具

2. **本周完成**:
   - 会话管理器完整测试
   - 时间轮测试补充
   - 消息处理链测试

3. **持续跟进**:
   - 每日测试覆盖率报告
   - 每周测试进度评审

---

**文档版本**: v1.0
**创建日期**: 2026-03-06
**服务**: ms-websocket
**目标**: 100% 测试覆盖

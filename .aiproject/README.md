# fbc-starter 微服务开发规范

> AI 工具在生成、修改或审查代码时**必须遵守**本规范体系。
>
> 📅 最后更新: 2026-03-11

## 规范文件索引

| 优先级 | 文件 | 领域 |
|--------|------|------|
| 🔴 P0 | [P0-product.md](P0-product.md) | 项目基础 · 依赖管理 · 项目结构 · 启动模式 · AppState |
| 🟠 P1 | [P1-architecture.md](P1-architecture.md) | 分层架构 · 模块组织 · 跨模块调用 · 数据层 · 配置 |
| 🔵 P2 | [P2-code-style.md](P2-code-style.md) | 命名规范 · 代码风格 · 导入顺序 · 文档注释 |
| 🟣 P3 | [P3-api.md](P3-api.md) | 响应格式 · 错误处理 · 错误码 · 路由 · gRPC · Kafka |
| 🟡 P4 | [P4-security.md](P4-security.md) | 安全实践 · HTTP 安全 · 认证 · 输入校验 |
| 🟤 P5 | [P5-testing.md](P5-testing.md) | 测试策略 · 分层测试 · 测试规范 |
| 🔵 P6 | [P6-deploy.md](P6-deploy.md) | Docker 容器化 · CI/CD · 环境管理 |
| 🟢 P7 | [P7-observability.md](P7-observability.md) | 日志规范 · 结构化日志 · 链路追踪 |
| 🟠 P8 | [P8-performance.md](P8-performance.md) | 性能优化 · 连接复用 · 批量操作 · 缓存策略 |
| 🔴 P9 | [P9-ops.md](P9-ops.md) | 健康检查 · 数据库迁移 · 运维实践 |

## 场景速查

| 开发场景 | 必读 |
|----------|------|
| 新建微服务 | P0 → P1 |
| 新增业务模块 | P1 → P3 |
| HTTP 接口开发 | P3 §1-3 |
| 数据库操作 | P1 §3-5 |
| gRPC 调用 | P3 §4-5 |
| 缓存操作 | P8 §4 |
| 消息队列（Kafka） | P3 §6-7 |
| 错误处理 | P3 §2-3 |
| 安全与认证 | P4 |
| 编写测试 | P5 |
| 容器化部署 | P6 |
| 性能优化 | P8 |

## 检查清单

### 必须通过
```
[ ] Cargo.toml workspace 依赖（版本以 workspace Cargo.toml 为准）
[ ] modules/ 目录结构符合规范
[ ] Server::run 启动，禁止手动初始化
[ ] AppState #[derive(Clone)] + Arc 包装
[ ] Handler → Service → Repo 单向调用
```

### 应该通过
```
[ ] R<T> 包装所有 HTTP 响应
[ ] AppError 工厂方法处理错误
[ ] 错误码按服务分段分配
[ ] sqlxplus 实体宏（ModelMeta + CRUD）
[ ] CacheKeyBuilder 构建缓存键
[ ] tracing 日志（禁止 println!）
[ ] Docker 多阶段构建
[ ] /health 健康检查端点
```

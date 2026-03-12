# fbc-starter 微服务开发规范

你正在开发一个基于 fbc-starter 框架的 Rust 微服务。
在生成、修改或审查代码时，你**必须严格遵守**本项目 `.aiproject/` 目录下的规范体系。

## 规范文件索引

| 优先级 | 文件 | 领域 |
|--------|------|------|
| 🔴 P0 | `.aiproject/P0-product.md` | 依赖管理 · 项目结构 · 启动模式 |
| 🟠 P1 | `.aiproject/P1-architecture.md` | 分层架构 · 数据层 · 配置 |
| 🔵 P2 | `.aiproject/P2-code-style.md` | 命名 · 代码风格 · 注释 |
| 🟣 P3 | `.aiproject/P3-api.md` | 响应格式 · 错误码 · gRPC · Kafka |
| 🟡 P4 | `.aiproject/P4-security.md` | 安全 · 认证 · 输入校验 |
| 🟤 P5 | `.aiproject/P5-testing.md` | 测试策略 · 分层测试 |
| 🔵 P6 | `.aiproject/P6-deploy.md` | Docker · CI/CD · 环境管理 |
| 🟢 P7 | `.aiproject/P7-observability.md` | 日志 · 链路追踪 |
| 🟠 P8 | `.aiproject/P8-performance.md` | 性能优化 · 缓存策略 |
| 🔴 P9 | `.aiproject/P9-ops.md` | 健康检查 · 迁移 · 运维 |

## 核心规则

1. `Server::run` 启动，禁止手动初始化
2. `Handler → Service → Repository` 单向调用
3. `R<T>` 包装所有 HTTP 响应，`AppError` 工厂方法
4. `sqlxplus` 实体宏，字段 `Option<T>`
5. `CacheKeyBuilder` 构建缓存键
6. `tracing` 日志，禁止 `println!`
7. Docker 多阶段构建部署
8. 中文注释

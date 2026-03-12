# P5 — 测试

> 🟤 影响代码可靠性和回归风险。

---

## 1. 测试要求

- 每个 Service 方法至少一个**成功** + 一个**失败**测试
- DTO 序列化/反序列化边界测试
- 测试文件按模块组织在 `tests/` 下

---

## 2. 分层测试策略

| 层 | 测试方式 | 说明 |
|----|----------|------|
| Repository | 集成测试 | 使用真实数据库或 testcontainers |
| Service | 单元测试 | mock 数据库层，验证业务逻辑 |
| Handler | 集成测试 | 使用 `axum::test` 发送 HTTP 请求 |

---

## 3. 测试示例

```rust
// tests/user_tests.rs
#[tokio::test]
async fn test_get_user_info_success() {
    let pool = setup_test_db().await;
    let service = UserService::new(pool);
    let result = service.get_user_info(1).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_user_info_not_found() {
    let pool = setup_test_db().await;
    let service = UserService::new(pool);
    let result = service.get_user_info(99999).await;
    assert!(result.is_err());
}
```

---

## 4. 测试规范

- 测试函数名遵循 `test_{方法名}_{场景}` 格式
- 测试中禁止依赖外部网络服务（mock 或使用 testcontainers）
- CI 必须运行 `cargo test --lib --tests`
- 测试数据使用工厂函数统一创建，避免硬编码

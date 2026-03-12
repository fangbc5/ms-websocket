# P6 — 部署

> 🔵 影响交付效率和环境一致性。使用 Docker 容器化部署。

---

## 1. Docker 多阶段构建

每个微服务根目录提供 `Dockerfile`：

```dockerfile
# ===== 构建阶段 =====
FROM rust:1.83-slim AS builder

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev cmake build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 先复制依赖文件，利用 Docker 缓存
COPY Cargo.toml Cargo.lock ./
COPY fbc-starter/Cargo.toml fbc-starter/Cargo.toml
COPY ms-xxx/Cargo.toml ms-xxx/Cargo.toml

# 创建空的 src 目录用于预编译依赖
RUN mkdir -p fbc-starter/src ms-xxx/src \
    && echo "fn main() {}" > ms-xxx/src/main.rs \
    && touch fbc-starter/src/lib.rs \
    && cargo build --release -p ms-xxx 2>/dev/null || true

# 复制实际源码并构建
COPY . .
RUN touch ms-xxx/src/main.rs && cargo build --release -p ms-xxx

# ===== 运行阶段 =====
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN groupadd -r app && useradd -r -g app app

COPY --from=builder /app/target/release/ms-xxx /usr/local/bin/
COPY ms-xxx/.env.example /app/.env

WORKDIR /app
USER app

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["ms-xxx"]
```

---

## 2. Docker Compose（开发环境）

```yaml
# docker-compose.yml
version: '3.8'
services:
  ms-xxx:
    build:
      context: .
      dockerfile: ms-xxx/Dockerfile
    ports:
      - "3000:3000"
    env_file:
      - ms-xxx/.env
    depends_on:
      - mysql
      - redis
    networks:
      - hula-network

  mysql:
    image: mysql:8.0
    environment:
      MYSQL_ROOT_PASSWORD: root
      MYSQL_DATABASE: hula
    ports:
      - "3306:3306"
    volumes:
      - mysql-data:/var/lib/mysql
    networks:
      - hula-network

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    networks:
      - hula-network

volumes:
  mysql-data:

networks:
  hula-network:
```

---

## 3. CI/CD Pipeline

```bash
# 代码检查
cargo fmt -- --check
cargo clippy -- -D warnings

# 测试
cargo test --lib --tests

# 安全审计
cargo audit

# 构建镜像
docker build -t ms-xxx:${VERSION} -f ms-xxx/Dockerfile .

# 推送镜像
docker push registry.example.com/ms-xxx:${VERSION}
```

---

## 4. 环境管理

| 环境 | 配置方式 | 说明 |
|------|----------|------|
| 开发 | `.env` 文件 | 本地开发，docker-compose |
| 测试 | `.env.test` | CI/CD 自动测试 |
| 生产 | 环境变量 / 配置中心 | Nacos / K8s ConfigMap |

- `.env.example` **必须提交**到 Git（模板，不含真实值）
- `.env` **禁止提交**到 Git（包含真实密钥）
- 生产环境通过 Nacos 或 K8s 注入配置，不使用 `.env` 文件

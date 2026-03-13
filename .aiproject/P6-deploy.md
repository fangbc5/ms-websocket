# P6 — 部署

> 🔵 影响交付效率和环境一致性。使用 Docker 容器化部署。

---

## 1. Docker 镜像构建标准

### 构建基础镜像

所有微服务统一使用 `fbc-builder:latest`（Alpine musl 工具链），产出全静态链接二进制。

```bash
# 首次在 workspace 根目录构建基础镜像
docker build -f docker/Dockerfile.builder -t fbc-builder:latest .
```

### 标准 Dockerfile 模板

每个微服务根目录提供 `Dockerfile`，使用三阶段构建 + `scratch` 零字节运行镜像：

```dockerfile
# {service} Docker 镜像（fbc-builder + scratch）
# 构建: docker build -f {service}/Dockerfile .（workspace 根目录）
# syntax=docker/dockerfile:1

FROM fbc-builder:latest AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM fbc-builder:latest AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json -p {service}
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p {service} && \
    cp target/release/{service} /app/{service}-bin && \
    strip /app/{service}-bin

FROM scratch
COPY --from=builder /app/{service}-bin /app/{service}
WORKDIR /app
EXPOSE {port}
ENTRYPOINT ["/app/{service}"]
```

### 关键规则

| 规则 | 说明 |
|------|------|
| 构建镜像 | `fbc-builder:latest`（Alpine musl），**禁止** `rust:*-slim` |
| 运行镜像 | `scratch`，**禁止** `debian-slim` / `distroless` |
| CA 证书 | **不复制**（内网服务无 TLS 需求） |
| SSL 环境变量 | **不设置** `SSL_CERT_FILE` |
| `APP__SERVER__ADDR` | **不在 Dockerfile 中设置**，由 `docker-compose.yml` 指定 |
| BuildKit cache | **必须启用** `--mount=type=cache` 加速增量编译 |
| `strip` | 构建后 **必须** strip 二进制 |
| Healthcheck | **不在 Dockerfile 中配置**（scratch 无可用工具） |
| `EXPOSE` | 保留，作为文档声明 |

---

## 2. Docker Compose 标准

### 标准模板

所有微服务的 `docker-compose.yml` **必须**使用 `env_file` 引用外部环境变量文件 `.env.docker`，
**禁止**在 `docker-compose.yml` 中内联 `environment:` 配置项。

```yaml
# {service} 一键部署（仅服务本身）
# 使用方式（在 {service} 目录下）:
#   docker compose up -d --build

services:
  {service}:
    build:
      context: ..
      dockerfile: {service}/Dockerfile
    image: {service}:latest
    container_name: {service}
    ports:
      - "{port}:{port}"
    env_file:
      - .env.docker
    restart: unless-stopped
    networks:
      - fbc-network

networks:
  fbc-network:
    external: true
```

### `.env.docker` 文件规范

| 文件 | 是否提交 Git | 说明 |
|------|:---:|------|
| `.env.docker` | ❌ 禁止 | 包含真实密钥和连接信息，**必须**在 `.gitignore` 中排除 |
| `.env.example` | ✅ 必须 | 模板文件，使用占位符替代敏感值，供新开发者参考（开发和 Docker 部署共用） |

`.env.docker` 标准内容（所有微服务必须包含以下基础配置）:

```env
# ===== 服务器配置 =====
APP__SERVER__ADDR=0.0.0.0
APP__SERVER__PORT={port}

# ===== 日志配置 =====
APP__LOG__LEVEL=info

# ===== Nacos =====
APP__NACOS__SERVER_ADDRS=host.docker.internal:8848
APP__NACOS__SERVICE_NAME={service}
APP__NACOS__NAMESPACE={namespace-id}

# ... 其他业务配置（数据库、Redis、Kafka 等）
```

### 关键规则

| 规则 | 说明 |
|------|------|
| 配置方式 | **必须** `env_file: .env.docker`，**禁止** 内联 `environment:` |
| `APP__SERVER__ADDR` | **必须设置** `0.0.0.0`，否则服务仅监听 `127.0.0.1`，Nacos 注册失败 |
| `APP__NACOS__NAMESPACE` | **必须设置**，否则注册到 `public` 命名空间 |
| Healthcheck | **不配置**（Nacos 通过 gRPC 心跳管理健康状态） |
| 网络 | 使用外部 `fbc-network`，所有服务共享同一网络 |
| `.gitignore` | **必须**包含 `.env.docker` 条目 |

---

## 3. 部署脚本（deploy.sh）

每个微服务根目录提供 `deploy.sh`，一键完成停止旧容器、构建镜像、启动服务：

```bash
#!/bin/bash
set -e

SERVICE="{service}"
PORT="{port}"

echo "🚀 开始部署 $SERVICE..."

# 检查 Docker 是否安装
if ! command -v docker &> /dev/null; then
    echo "❌ Docker 未安装，请先安装 Docker"
    exit 1
fi

# 检查 docker compose 是否可用
if ! docker compose version &> /dev/null; then
    echo "❌ docker compose 不可用，请先安装 Docker Compose V2"
    exit 1
fi

# 停止旧容器
echo "📦 停止旧容器..."
docker compose down 2>/dev/null || true

# 构建镜像
echo "🔨 构建 Docker 镜像..."
docker compose build

# 启动容器
echo "▶️  启动容器..."
docker compose up -d

# 等待服务启动
echo "⏳ 等待服务启动..."
sleep 3

# 检查服务状态
if docker compose ps | grep -q "Up\|running"; then
    echo "✅ $SERVICE 部署成功！"
    echo "📍 服务端口: $PORT"
    echo "📊 查看日志: docker compose logs -f"
    echo "🛑 停止服务: docker compose down"
else
    echo "❌ 服务启动失败，查看日志:"
    docker compose logs
    exit 1
fi
```

---

## 4. 服务注册要点

- 微服务经过网关（`ms-gateway`），**不对外暴露**
- Nacos 服务注册/发现使用 gRPC 协议（`set_use_grpc(true)`）
- Nacos 健康检查基于 gRPC 心跳，**不依赖** Docker healthcheck 或 HTTP 探针
- `APP__SERVER__ADDR` 必须为 `0.0.0.0`，否则容器内服务仅监听 loopback，Nacos 无法通信

---

## 5. CI/CD Pipeline

```bash
# 代码检查
cargo fmt -- --check
cargo clippy -- -D warnings

# 测试
cargo test --lib --tests

# 构建镜像（workspace 根目录执行）
docker build -f {service}/Dockerfile -t {service}:${VERSION} .

# 推送镜像
docker push registry.example.com/{service}:${VERSION}
```

---

## 6. 环境管理

| 环境 | 配置方式 | 说明 |
|------|----------|------|
| 开发 | `.env` 文件 | 本地开发，`cargo run` |
| 容器 | `.env.docker` 文件 | Docker 部署，通过 `env_file` 加载 |
| 生产 | 环境变量 / Nacos 配置中心 | K8s ConfigMap / Nacos |

| 文件 | Git | 说明 |
|------|:---:|------|
| `.env.example` | ✅ 提交 | 环境配置模板（不含真实值，开发和 Docker 共用） |
| `.env` | ❌ 忽略 | 本地开发真实配置 |
| `.env.docker` | ❌ 忽略 | Docker 部署真实配置 |

- 生产环境通过 Nacos 或 K8s 注入配置，不使用 `.env` 文件


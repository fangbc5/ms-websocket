# ms-websocket Docker 镜像（多阶段构建）
# 构建时请在 workspace 根目录执行: docker build -f ms-websocket/Dockerfile .

# ---- 构建阶段（使用 slim 镜像） ----
FROM rust:1.83-slim AS builder

WORKDIR /app

# 安装最小构建依赖（根据当前依赖需要 OpenSSL 等）
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 复制整个 workspace（.dockerignore 在 workspace 根目录排除 target/.git 等）
COPY . .

# 仅编译 ms-websocket 及其依赖
RUN cargo build --release -p ms-websocket

# ---- 运行阶段 ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 从构建阶段复制二进制
COPY --from=builder /app/target/release/ms-websocket /app/ms-websocket

# 容器内默认监听所有接口
ENV APP__SERVER__ADDR=0.0.0.0

EXPOSE 30001

ENTRYPOINT ["/app/ms-websocket"]

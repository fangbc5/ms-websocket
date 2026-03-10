# ms-websocket Docker 镜像（cargo-chef 三阶段构建）
# 构建时请在 workspace 根目录执行: docker build -f ms-websocket/Dockerfile .

# ---- 基础构建环境 ----
FROM rust:1.88-slim AS base

# 配置 Cargo 使用国内镜像源
RUN mkdir -p /usr/local/cargo && \
    echo '[source.crates-io]' > /usr/local/cargo/config.toml && \
    echo 'replace-with = "ustc"' >> /usr/local/cargo/config.toml && \
    echo '[source.ustc]' >> /usr/local/cargo/config.toml && \
    echo 'registry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"' >> /usr/local/cargo/config.toml

# 替换 apt 源为清华镜像
RUN sed -i 's|deb.debian.org|mirrors.tuna.tsinghua.edu.cn|g' /etc/apt/sources.list.d/debian.sources

# 安装构建依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates \
    gcc g++ make cmake \
    protobuf-compiler libclang-dev \
    && rm -rf /var/lib/apt/lists/*

# 安装 cargo-chef
RUN cargo install cargo-chef --locked

WORKDIR /app

# ---- 阶段 1: 分析依赖，生成 recipe.json ----
FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---- 阶段 2: 缓存依赖 + 编译业务代码 ----
FROM base AS builder

# 只复制 recipe.json（依赖描述），这一层只要依赖不变就命中缓存
COPY --from=planner /app/recipe.json recipe.json

# 编译所有依赖（不编译业务代码），结果被 Docker 缓存
RUN cargo chef cook --release --recipe-path recipe.json -p ms-websocket

# 复制真实源码，只增量编译业务代码
COPY . .
RUN cargo build --release -p ms-websocket

# ---- 阶段 3: 运行环境 ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/ms-websocket /app/ms-websocket

ENV APP__SERVER__ADDR=0.0.0.0
EXPOSE 30001
ENTRYPOINT ["/app/ms-websocket"]

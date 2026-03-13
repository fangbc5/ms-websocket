# ms-websocket Docker 镜像（fbc-builder + scratch）
# 构建: docker build -f ms-websocket/Dockerfile .（workspace 根目录）
# syntax=docker/dockerfile:1

FROM fbc-builder:latest AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM fbc-builder:latest AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json -p ms-websocket
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p ms-websocket && \
    cp target/release/ms-websocket /app/ms-websocket-bin && \
    strip /app/ms-websocket-bin

FROM scratch
COPY --from=builder /app/ms-websocket-bin /app/ms-websocket
WORKDIR /app
EXPOSE 30201
ENTRYPOINT ["/app/ms-websocket"]

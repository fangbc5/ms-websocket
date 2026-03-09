#!/usr/bin/env bash
# ms-websocket 一键 Docker 部署
# 用法: ./deploy-docker.sh [--with-redis]
#   无参数: 只启动 ms-websocket，需在 .env.docker 中配置 APP__REDIS__URL 指向共享 Redis
#   --with-redis: 同时启动本 compose 内的 Redis 容器

set -e
cd "$(dirname "$0")"

if [[ ! -f .env.docker ]]; then
  echo "首次运行: 已从 .env.docker.example 生成 .env.docker，请按需修改（尤其 Redis 地址）后重新执行。"
  cp -n .env.docker.example .env.docker
  exit 0
fi

PROFILE=""
if [[ "$1" == "--with-redis" ]]; then
  PROFILE="--profile redis"
  echo "正在构建并启动 ms-websocket + Redis..."
else
  echo "正在构建并启动 ms-websocket（使用 .env.docker 中的 Redis 地址，不启动本 compose 的 Redis）..."
fi

docker compose $PROFILE up -d --build

echo ""
echo "部署完成。"
echo "  - WebSocket: ws://localhost:${APP__SERVER__PORT:-30001}/ws"
[[ -n "$PROFILE" ]] && echo "  - Redis: localhost:6379"
echo "查看日志: docker compose logs -f ms-websocket"
echo "停止: docker compose $PROFILE down"

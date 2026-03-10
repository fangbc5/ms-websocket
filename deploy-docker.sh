#!/usr/bin/env bash
set -e

echo "🚀 开始部署 ms-websocket..."

cd "$(dirname "$0")"

# 检查 Docker 是否安装
if ! command -v docker &> /dev/null; then
    echo "❌ Docker 未安装，请先安装 Docker"
    exit 1
fi

# 检查 docker compose 是否可用
if ! docker compose version &> /dev/null; then
    echo "❌ docker compose 不可用，请确认 Docker Compose V2 已安装"
    exit 1
fi

# 检查 .env.docker 配置文件
if [[ ! -f .env.docker ]]; then
    echo "⚠️  首次运行: 已从 .env.docker.example 生成 .env.docker"
    echo "   请按需修改（尤其 APP__REDIS__URL）后重新执行。"
    cp -n .env.docker.example .env.docker
    exit 0
fi

# 确保共享网络存在，便于与 ms-gateway 等服务互通
docker network inspect fbc-network >/dev/null 2>&1 || docker network create fbc-network

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
    echo "✅ ms-websocket 部署成功！"
    echo "📍 WebSocket 地址: ws://localhost:${APP__SERVER__PORT:-30001}/ws"
    echo "📊 查看日志: docker compose logs -f ms-websocket"
    echo "🛑 停止服务: docker compose down"
else
    echo "❌ 服务启动失败，查看日志:"
    docker compose logs
    exit 1
fi

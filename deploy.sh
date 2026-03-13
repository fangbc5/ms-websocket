#!/bin/bash
set -e

echo "🚀 开始部署 ms-websocket..."

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
    echo "✅ ms-websocket 部署成功！"
    echo "📍 服务端口: 30201"
    echo "📊 查看日志: docker compose logs -f"
    echo "🛑 停止服务: docker compose down"
else
    echo "❌ 服务启动失败，查看日志:"
    docker compose logs
    exit 1
fi

# ms-websocket Docker 部署

## Redis 使用方式

- 与其他项目共用 Redis：在 `.env.docker` 中把 `APP__REDIS__URL` 设为共享 Redis 的地址（如 `redis://host.docker.internal:6379` 或已有 Redis 服务名）。
- 在本目录使用 `docker-compose-redis.yml` 单独起一个 Redis（端口默认为 6379），然后在 `.env.docker` 里设置 `APP__REDIS__URL=redis://redis:6379`。

## 一键部署

在 **ms-websocket 目录**下执行：

```bash
# 方式一：使用脚本（推荐）
cd ms-websocket
chmod +x deploy-docker.sh
./deploy-docker.sh
# 首次会生成 .env.docker，按需修改（尤其 Redis 地址）后再次执行

# 方式二：共用 Redis，只起 ms-websocket
cp .env.docker.example .env.docker
# 编辑 .env.docker，把 APP__REDIS__URL 改成共享 Redis 地址
docker compose up -d --build
```

- 仅 `docker compose up -d` 时：只启动 **ms-websocket**，端口 `30001`（可用 `.env.docker` 中 `APP__SERVER__PORT` 修改）。
- 使用 `--profile redis` 时：额外启动 **Redis**，端口 `6379`。

## 仅构建镜像（不启动）

在 **workspace 根目录** 执行：

```bash
docker build -f ms-websocket/Dockerfile -t ms-websocket:latest .
```

## 环境变量

- 容器内已默认设置 `APP__SERVER__ADDR=0.0.0.0`。Redis 地址由 `.env.docker` 中的 `APP__REDIS__URL` 决定（共用 Redis 时填共享地址，使用本目录的 `docker-compose-redis.yml` 时填 `redis://redis:6379`）。
- 其他配置见 `ms-websocket/.env.docker.example`。需要 Nacos/Kafka 时，将地址改为宿主机或对应容器（如 `host.docker.internal`）。

## 常用命令

```bash
cd ms-websocket

# 查看日志
docker compose logs -f ms-websocket

# 停止并删除容器
docker compose down
```

> `depends_on` 对 Redis 使用了 `required: false`，需 Docker Compose v2.20.2+。若版本较旧，共用 Redis 时需自行去掉 compose 里对 `redis` 的 `depends_on`。

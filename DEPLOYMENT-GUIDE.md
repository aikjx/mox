# 璇玑 RelGraph · 算子统一系统（OUS）部署运维手册

> **版本**：v1.0.0
> **日期**：2026-09-04
> **适用环境**：Linux / macOS / Windows (WSL2)

---

## 目录

1. [系统要求](#1-系统要求)
2. [快速开始](#2-快速开始)
3. [配置说明](#3-配置说明)
4. [服务架构](#4-服务架构)
5. [LLM 提供商配置](#5-llm-提供商配置)
6. [数据库初始化](#6-数据库初始化)
7. [监控与日志](#7-监控与日志)
8. [备份与恢复](#8-备份与恢复)
9. [故障排查](#9-故障排查)
10. [性能调优](#10-性能调优)
11. [升级指南](#11-升级指南)

---

## 1. 系统要求

### 最低配置

| 组件 | 最低要求 | 推荐配置 |
|------|----------|----------|
| CPU | 4 核 | 8 核+ |
| 内存 | 8 GB | 16 GB+ |
| 磁盘 | 20 GB 可用空间 | 50 GB+ SSD |
| 操作系统 | Ubuntu 20.04+ / CentOS 8+ / macOS 12+ / Windows 10 (WSL2) | Ubuntu 22.04 LTS |
| Docker | 20.10+ | 24.0+ |
| Docker Compose | 2.0+ | 2.20+ |

### GPU 支持（可选，使用本地 Ollama 时推荐）

- NVIDIA GPU，显存 ≥ 8 GB（7B 模型）/ ≥ 16 GB（14B 模型）
- NVIDIA Driver ≥ 525
- NVIDIA Container Toolkit

---

## 2. 快速开始

### 2.1 克隆代码

```bash
git clone <repository-url> infotopograph
cd infotopograph
```

### 2.2 配置环境变量

```bash
cp .env.example .env
# 编辑 .env，至少修改以下配置：
# - POSTGRES_PASSWORD
# - LLM_PROVIDER / LLM_MODEL
# - 对应 LLM 提供商的 API Key
```

### 2.3 启动服务

```bash
# 基础服务（前端 + 后端 + LLM 服务 + 数据库 + 缓存）
docker-compose up -d

# 包含本地 Ollama（需要 GPU）
docker-compose --profile ollama up -d

# 包含监控（Prometheus + Grafana）
docker-compose --profile monitoring up -d

# 全部服务
docker-compose --profile ollama --profile monitoring up -d
```

### 2.4 验证服务

```bash
# 查看服务状态
docker-compose ps

# 查看日志
docker-compose logs -f

# 健康检查
curl http://localhost:8080/health
curl http://localhost:8081/actuator/health
curl http://localhost:8001/health
```

### 2.5 访问系统

- **前端界面**：http://localhost:8080
- **API 文档**：http://localhost:8081/docs（如果启用了 Swagger）
- **LLM 服务**：http://localhost:8001/docs
- **Prometheus**：http://localhost:9090（监控 profile）
- **Grafana**：http://localhost:3000（监控 profile，默认 admin/admin）

### 2.6 停止服务

```bash
# 停止但保留数据
docker-compose down

# 停止并删除数据卷（谨慎！）
docker-compose down -v
```

---

## 3. 配置说明

### 3.1 环境变量一览

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `NGINX_PORT` | 8080 | Nginx 对外端口 |
| `PLATFORM_PORT` | 8081 | 后端平台对外端口 |
| `LLM_PORT` | 8001 | LLM 服务对外端口 |
| `POSTGRES_USER` | mox | 数据库用户名 |
| `POSTGRES_PASSWORD` | mox | 数据库密码（**生产环境必须修改**） |
| `POSTGRES_DB` | mox | 数据库名 |
| `POSTGRES_PORT` | 5432 | 数据库端口 |
| `REDIS_PORT` | 6379 | Redis 端口 |
| `LLM_PROVIDER` | ollama | LLM 提供商 |
| `LLM_MODEL` | qwen2.5 | 默认模型 |
| `LLM_SERVICE_API_KEY` | （空） | LLM 服务 API Key（留空不鉴权） |
| `RUST_LOG` | info,mox=debug | Rust 日志级别 |
| `LOG_LEVEL` | INFO | Python 日志级别 |

### 3.2 生产环境配置建议

```env
# 安全
POSTGRES_PASSWORD=<强密码>
LLM_SERVICE_API_KEY=<随机字符串>

# 性能
RUST_LOG=info
LOG_LEVEL=WARNING

# LLM（使用云服务）
LLM_PROVIDER=qwen
LLM_MODEL=qwen-max
QWEN_API_KEY=<你的API Key>
```

---

## 4. 服务架构

### 4.1 服务清单

| 服务 | 镜像/构建 | 端口 | 说明 |
|------|-----------|------|------|
| nginx | nginx:1.27-alpine | 8080 | 反向代理 + 前端静态资源 |
| platform-gateway | 本地构建 (Rust) | 8081 | 后端平台网关 + 业务逻辑 |
| llm-inference-svc | 本地构建 (Python) | 8001 | LLM 推理网关（OpenAI 兼容） |
| ollama | ollama/ollama | 11434 | 本地 LLM 运行时（可选） |
| postgres | postgres:16-alpine | 5432 | 关系型数据库 |
| redis | redis:7-alpine | 6379 | 缓存 + 会话 |
| prometheus | prom/prometheus | 9090 | 指标采集（可选） |
| grafana | grafana/grafana | 3000 | 可视化仪表盘（可选） |

### 4.2 网络拓扑

```
客户端 (浏览器)
    │
    ▼ :8080
┌─────────┐
│  Nginx  │  反向代理 + 静态资源 + Gzip + 限流
└────┬────┘
     │
     ├───────────────┐
     ▼               ▼
┌──────────┐   ┌──────────────┐
│ Platform │   │ LLM Inference│
│ Gateway  │   │    Service   │
│ (Rust)   │   │   (Python)   │
└────┬─────┘   └──────┬───────┘
     │                 │
     ├─────────┐       ├────────────┐
     ▼         ▼       ▼            ▼
┌────────┐ ┌──────┐ ┌────────┐ ┌─────────┐
│Postgres│ │Redis │ │ Ollama │ │OpenAI/  │
│        │ │      │ │(本地)  │ │Qwen/豆包│
└────────┘ └──────┘ └────────┘ └─────────┘
```

---

## 5. LLM 提供商配置

### 5.1 Ollama（本地开源模型，推荐开发/隐私场景）

```env
LLM_PROVIDER=ollama
LLM_MODEL=qwen2.5
OLLAMA_BASE_URL=http://ollama:11434
```

启动后拉取模型：

```bash
# 进入容器
docker-compose exec ollama bash

# 拉取模型（根据显存选择）
ollama pull qwen2.5:7b       # 7B 参数，约 4.7GB
ollama pull qwen2.5:14b      # 14B 参数，约 9GB
ollama pull llama3.1:8b       # Llama 3.1 8B
ollama pull deepseek-r1:7b    # DeepSeek R1 推理模型

# 验证
ollama run qwen2.5 "你好"
```

### 5.2 OpenAI

```env
LLM_PROVIDER=openai
LLM_MODEL=gpt-4o-mini
OPENAI_API_BASE=https://api.openai.com/v1
OPENAI_API_KEY=sk-xxxxxxxxxxxxxxxx
```

### 5.3 通义千问（阿里云）

```env
LLM_PROVIDER=qwen
LLM_MODEL=qwen-max
QWEN_API_KEY=sk-xxxxxxxxxxxxxxxx
QWEN_BASE_URL=https://dashscope.aliyuncs.com/compatible-mode/v1
```

### 5.4 豆包（火山引擎）

```env
LLM_PROVIDER=doubao
LLM_MODEL=doubao-pro-32k
DOUBAO_API_KEY=xxxxxxxxxxxxxxxx
DOUBAO_BASE_URL=https://ark.cn-beijing.volces.com/api/v3
```

### 5.5 自定义 OpenAI 兼容端点

任何 OpenAI 兼容的 API 都可以使用 openai 提供商：

```env
LLM_PROVIDER=openai
LLM_MODEL=your-model-name
OPENAI_API_BASE=https://your-custom-endpoint.com/v1
OPENAI_API_KEY=your-api-key
```

---

## 6. 数据库初始化

### 6.1 自动初始化

首次启动时，PostgreSQL 会自动执行 `database/init/` 目录下的 SQL 脚本。

### 6.2 手动初始化

```bash
# 进入 postgres 容器
docker-compose exec postgres bash

# 执行初始化脚本
psql -U mox -d mox -f /docker-entrypoint-initdb.d/01-schema.sql

# 验证表结构
psql -U mox -d mox -c "\dt"
```

### 6.3 数据库迁移

使用 `sqlx` 或 `diesel` 进行版本化迁移（根据后端实现选择）。

```bash
# 示例：sqlx 迁移
docker-compose exec platform-gateway sqlx migrate run
```

---

## 7. 监控与日志

### 7.1 日志查看

```bash
# 查看所有服务日志
docker-compose logs -f

# 查看特定服务日志
docker-compose logs -f platform-gateway
docker-compose logs -f llm-inference-svc

# 查看最近 100 行
docker-compose logs --tail=100 platform-gateway

# 按时间过滤
docker-compose logs --since="10m" platform-gateway
```

### 7.2 指标监控（启用 monitoring profile）

**Prometheus 指标端点**：

| 服务 | 端点 | 说明 |
|------|------|------|
| platform-gateway | `/actuator/metrics` | Rust 服务指标 |
| llm-inference-svc | `/metrics` | Python 服务指标 |
| nginx | （需配置 stub_status） | Nginx 指标 |

**关键指标**：

- `llm_requests_total{provider,model,status}` — LLM 请求总数
- `llm_request_duration_seconds{provider,model}` — LLM 请求延迟
- HTTP 请求 QPS / 延迟 / 错误率
- 数据库连接池使用率
- Redis 内存使用率

### 7.3 健康检查

```bash
# 各服务健康检查
curl http://localhost:8080/health          # Nginx
curl http://localhost:8081/actuator/health # 后端
curl http://localhost:8001/health           # LLM 服务

# Docker 容器健康状态
docker-compose ps
```

---

## 8. 备份与恢复

### 8.1 数据库备份

```bash
# 备份
docker-compose exec postgres pg_dump -U mox mox > backup_$(date +%Y%m%d).sql

# 恢复
cat backup_20260101.sql | docker-compose exec -T postgres psql -U mox mox
```

### 8.2 数据卷备份

```bash
# 备份 postgres 数据卷
docker run --rm -v mox_postgres-data:/data -v $(pwd):/backup alpine \
  tar czf /backup/postgres_backup.tar.gz -C /data .

# 备份 redis 数据卷
docker run --rm -v mox_redis-data:/data -v $(pwd):/backup alpine \
  tar czf /backup/redis_backup.tar.gz -C /data .
```

### 8.3 定时备份（Cron）

```bash
# 编辑 crontab
crontab -e

# 每天凌晨 2 点备份数据库
0 2 * * * cd /path/to/infotopograph && docker-compose exec -T postgres pg_dump -U mox mox > backups/db_$(date +\%Y\%m\%d).sql && find backups -name "db_*.sql" -mtime +7 -delete
```

---

## 9. 故障排查

### 9.1 服务无法启动

```bash
# 查看具体错误
docker-compose logs <service-name>

# 常见问题：
# 1. 端口被占用 → 修改 .env 中的端口
# 2. 权限不足 → 检查数据卷权限
# 3. 内存不足 → 增加内存或减少并发
```

### 9.2 LLM 调用失败

```bash
# 检查 LLM 服务状态
curl http://localhost:8001/health
curl http://localhost:8001/v1/models

# 检查日志
docker-compose logs llm-inference-svc

# 常见原因：
# 1. API Key 无效 → 检查 .env
# 2. 模型不存在 → 执行 ollama list 或检查云服务模型名
# 3. 网络不通 → 检查防火墙 / 代理设置
# 4. 超时 → 增加 REQUEST_TIMEOUT
```

### 9.3 数据库连接失败

```bash
# 检查 postgres 状态
docker-compose ps postgres
docker-compose logs postgres

# 测试连接
docker-compose exec postgres pg_isready -U mox

# 常见原因：
# 1. 密码不匹配 → 检查 POSTGRES_PASSWORD
# 2. 数据卷损坏 → 尝试恢复备份
# 3. 端口冲突 → 修改 POSTGRES_PORT
```

### 9.4 前端无法访问

```bash
# 检查 nginx 状态
docker-compose ps nginx
docker-compose logs nginx

# 测试后端连通性
docker-compose exec nginx wget -qO- http://platform-gateway:8080/actuator/health

# 常见原因：
# 1. 前端未构建 → 检查 frontend-dist 卷
# 2. 后端未启动 → 等待 platform-gateway 健康
# 3. 浏览器缓存 → 强制刷新 (Ctrl+Shift+R)
```

### 9.5 SSE 流式输出不工作

```bash
# 检查 nginx 配置（proxy_buffering off）
docker-compose exec nginx cat /etc/nginx/conf.d/default.conf | grep -A5 "location /api"

# 测试 SSE 端点
curl -N http://localhost:8080/api/alliance/stream?query=test

# 常见原因：
# 1. 代理缓冲未关闭 → 检查 nginx 配置
# 2. 防火墙拦截 → 检查网络策略
# 3. 浏览器不支持 → 使用现代浏览器
```

---

## 10. 性能调优

### 10.1 Nginx 调优

```nginx
# nginx.conf
worker_processes auto;
worker_connections 4096;
keepalive_timeout 65;
gzip_comp_level 6;
```

### 10.2 PostgreSQL 调优

```sql
-- postgresql.conf（根据内存调整）
shared_buffers = 25% of RAM
effective_cache_size = 75% of RAM
work_mem = 4MB
maintenance_work_mem = 64MB
max_connections = 200
```

### 10.3 Redis 调优

```redis
# redis.conf
maxmemory 256mb
maxmemory-policy allkeys-lru
appendonly yes
save 900 1
save 300 10
```

### 10.4 LLM 服务调优

```env
# 增加并发
MAX_CONCURRENT=20
MAX_RPM=120

# 增加超时
REQUEST_TIMEOUT=120

# Ollama 并行
OLLAMA_NUM_PARALLEL=4
OLLAMA_KEEP_ALIVE=24h
```

### 10.5 后端调优

```env
# Rust 日志级别（生产环境降低日志）
RUST_LOG=info

# 数据库连接池
DATABASE_POOL_SIZE=20

# 任务并发
MAX_CONCURRENT_TASKS=50
```

---

## 11. 升级指南

### 11.1 常规升级

```bash
# 1. 拉取最新代码
git pull

# 2. 备份数据
docker-compose exec postgres pg_dump -U mox mox > backup_pre_upgrade.sql

# 3. 重新构建并启动
docker-compose build
docker-compose up -d

# 4. 验证
docker-compose ps
curl http://localhost:8080/health
```

### 11.2 数据库 schema 升级

```bash
# 执行迁移
docker-compose exec platform-gateway sqlx migrate run

# 验证
docker-compose exec postgres psql -U mox -c "\dt"
```

### 11.3 回滚

```bash
# 回滚代码
git checkout <previous-version>

# 恢复数据库
cat backup_pre_upgrade.sql | docker-compose exec -T postgres psql -U mox mox

# 重新启动
docker-compose build
docker-compose up -d
```

---

## 附录

### A. 常用命令速查

```bash
# 启动
docker-compose up -d

# 停止
docker-compose down

# 重启单个服务
docker-compose restart platform-gateway

# 进入容器
docker-compose exec platform-gateway bash

# 查看资源使用
docker stats

# 清理未使用的镜像/卷
docker system prune -a
docker volume prune
```

### B. 端口分配

| 端口 | 服务 | 说明 |
|------|------|------|
| 8080 | Nginx | 前端 + 反向代理 |
| 8081 | platform-gateway | 后端 API |
| 8001 | llm-inference-svc | LLM 推理服务 |
| 5432 | PostgreSQL | 数据库 |
| 6379 | Redis | 缓存 |
| 11434 | Ollama | 本地 LLM（可选） |
| 9090 | Prometheus | 指标（可选） |
| 3000 | Grafana | 可视化（可选） |

### C. 联系与支持

- 项目文档：`docs/` 目录
- 架构文档：`docs/ARCHITECTURE-ENTERPRISE.md`
- API 文档：启动后访问 `/docs`

---

*文档结束*

# 专家联盟生产级部署方案

## 服务拓扑

```
┌─────────────────────────────────────────────────────────┐
│                    负载均衡 (Nginx)                      │
└─────────────────────┬───────────────────────────────────┘
                      │
         ┌────────────┼────────────┐
         │            │            │
    ┌────▼────┐ ┌────▼────┐ ┌────▼────┐
    │ gateway │ │ frontend │ │  docs   │
    │  :3080  │ │  :3020  │ │  :8080  │
    └────┬────┘ └─────────┘ └─────────┘
         │
    ┌────┼────────────┬────────────┐
    │    │            │            │
┌───▼──┐┌───▼──┐┌────▼────┐┌────▼────┐
│ sched││exec  ││registry ││  redis  │
│:3100 ││:3200 ││  :3400  ││  :6379  │
└───┬──┘└───┬──┘└────┬────┘└─────────┘
    │       │         │
    └───────┴─────────┘
              │
        ┌─────▼─────┐
        │ PostgreSQL│
        │   :5432   │
        └───────────┘
```

## docker-compose.yml

```yaml
version: '3.8'

services:
  # PostgreSQL 数据库
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: alliance
      POSTGRES_USER: mox
      POSTGRES_PASSWORD: ${DB_PASSWORD:-changeme}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U mox"]
      interval: 5s
      timeout: 5s
      retries: 5

  # Redis 缓存
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data

  # Registry Svc
  registry-svc:
    build: .
    command: ./mox-alliance-registry-svc
    environment:
      - DATABASE_URL=postgres://mox:${DB_PASSWORD:-changeme}@postgres:5432/alliance
      - RUST_LOG=info
    ports:
      - "3400:3400"
    depends_on:
      postgres:
        condition: service_healthy

  # Scheduler Svc
  scheduler-svc:
    build: .
    command: ./mox-alliance-scheduler-svc
    environment:
      - DATABASE_URL=postgres://mox:${DB_PASSWORD:-changeme}@postgres:5432/alliance
      - REGISTRY_URL=http://registry-svc:3400
      - RUST_LOG=info
    ports:
      - "3100:3100"
    depends_on:
      postgres:
        condition: service_healthy
      registry-svc:
        condition: service_started

  # Executor Svc
  executor-svc:
    build: .
    command: ./mox-alliance-executor-svc
    environment:
      - DATABASE_URL=postgres://mox:${DB_PASSWORD:-changeme}@postgres:5432/alliance
      - RUST_LOG=info
    ports:
      - "3200:3200"
    depends_on:
      postgres:
        condition: service_healthy
      scheduler-svc:
        condition: service_started

  # Gateway
  gateway:
    build: .
    command: ./mox-platform-gateway
    environment:
      - SCHEDULER_URL=http://scheduler-svc:3100
      - EXECUTOR_URL=http://executor-svc:3200
      - REGISTRY_URL=http://registry-svc:3400
      - RUST_LOG=info
    ports:
      - "3080:3080"
    depends_on:
      - registry-svc
      - scheduler-svc
      - executor-svc

  # Prometheus 监控
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml
    ports:
      - "9090:9090"

  # Grafana 可视化
  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    volumes:
      - grafana_data:/var/lib/grafana
    depends_on:
      - prometheus

volumes:
  postgres_data:
  redis_data:
  grafana_data:
```

## 监控指标

### 各服务暴露的 metrics 端点

| 服务 | 端口 | 路径 |
|------|------|------|
| gateway | 3080 | /metrics |
| scheduler-svc | 3100 | /metrics |
| executor-svc | 3200 | /metrics |
| registry-svc | 3400 | /metrics |

### 核心监控指标

- `alliance_task_total`: 任务总数
- `alliance_task_duration_ms`: 任务执行耗时
- `alliance_expert_active`: 活跃专家数
- `alliance_match_latency_ms`: 专家匹配延迟
- `alliance_fusion_duration_ms`: 融合耗时
- `alliance_dynamic_routes_total`: Dynamic 路由决策次数

## 日志方案

- **格式**：JSON 结构化日志
- **级别**：info（生产）/ debug（开发）
- **输出**：stdout + 文件轮转
- **采集**：Promtail → Loki（可选）

## 高可用策略

| 层级 | 方案 |
|------|------|
| 应用层 | 多实例 + 负载均衡 |
| 数据库 | PostgreSQL 主从复制 |
| 缓存 | Redis Sentinel |
| 熔断 | 各服务内置熔断器 |
| 重试 | 指数退避重试 |

## 部署步骤

1. 构建镜像：`docker-compose build`
2. 启动基础设施：`docker-compose up -d postgres redis`
3. 启动微服务：`docker-compose up -d registry-svc scheduler-svc executor-svc gateway`
4. 启动监控：`docker-compose up -d prometheus grafana`
5. 验证：访问 http://localhost:3080/health

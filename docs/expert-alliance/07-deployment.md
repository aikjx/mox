---
doc_id: EA-OPS-001
title: 专家联盟部署与运维架构
version: V1.0
authority: 🟡参考
last_updated: 2026-09-25
---

# 专家联盟部署与运维架构

## 1. 当前部署形态

### 1.1 进程清单

| 进程 | 端口 | 二进制 | 启动方式 |
|------|------|--------|---------|
| platform-gateway-svc | 3080 | Rust | `cargo run -p mox-platform-gateway-svc` |
| alliance-scheduler-svc | 3100 | Rust | `cargo run -p mox-alliance-scheduler-svc` |
| alliance-executor-svc | 3200 | Rust | `cargo run -p mox-alliance-executor-svc` |
| alliance-registry-svc | 3400 | Rust | `cargo run -p mox-alliance-registry-svc` |

### 1.2 依赖组件

| 组件 | 用途 | 当前状态 |
|------|------|---------|
| SQLite | 持久化存储 | ✅ 已落地（本地文件） |
| Nacos | 配置中心/服务发现 | ✅ tools/rnacos（内嵌） |
| PostgreSQL | 生产级数据库 | 📋 规划中 |
| Redis | 缓存/会话 | 📋 规划中 |
| Prometheus | 指标采集 | 📋 observability/ 目录配置 |
| Grafana | 指标可视化 | 📋 observability/ 目录配置 |

---

## 2. 本地开发启动

```bash
# 1. 启动 Nacos（内嵌）
cd platform/domains/alliance/tools/rnacos
./start.sh

# 2. 启动注册中心
cargo run -p mox-alliance-registry-svc

# 3. 启动调度器
cargo run -p mox-alliance-scheduler-svc

# 4. 启动执行器
cargo run -p mox-alliance-executor-svc

# 5. 启动网关
cargo run -p mox-platform-gateway-svc
```

---

## 3. Docker 部署

```yaml
# docker-compose.yml 片段
services:
  gateway:
    build: .
    ports:
      - "3080:3080"
    depends_on:
      - scheduler
      - executor
      - registry

  scheduler:
    build: .
    command: cargo run -p mox-alliance-scheduler-svc
    ports:
      - "3100:3100"

  executor:
    build: .
    command: cargo run -p mox-alliance-executor-svc
    ports:
      - "3200:3200"

  registry:
    build: .
    command: cargo run -p mox-alliance-registry-svc
    ports:
      - "3400:3400"
```

---

## 4. 生产级演进路线

| 阶段 | 目标 | 关键改动 |
|------|------|---------|
| Phase 1（当前） | 单机演示 | SQLite + 内嵌 Nacos + 进程内调用 |
| Phase 2 | 生产级单体 | PostgreSQL + Redis + 独立 Nacos |
| Phase 3 | 微服务化 | gRPC 长连接 + K8s + HPA |
| Phase 4 | 云原生 | 多租户隔离 + 自动扩缩 + 全链路追踪 |

---

## 5. 可观测性

### 5.1 指标端点

| 端点 | 说明 |
|------|------|
| `/metrics` | Prometheus 指标（gateway 侧） |
| `/health` | 健康检查（各服务） |

### 5.2 关键指标

| 指标 | 说明 |
|------|------|
| task_create_total | 任务创建总数 |
| task_duration_ms | 任务执行时长 |
| expert_match_latency_ms | 专家匹配延迟 |
| node_schedule_latency_ms | 节点调度延迟 |
| task_success_rate | 任务成功率 |

---

## 6. 配置参考

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| gateway.port | 3080 | 网关端口 |
| scheduler.port | 3100 | 调度器端口 |
| executor.port | 3200 | 执行器端口 |
| registry.port | 3400 | 注册中心端口 |
| db.path | ./data/alliance.db | SQLite 路径 |
| nacos.addr | 127.0.0.1:8848 | Nacos 地址 |

---
title: 06 - 安全与可观测性
version: V2.0
authority: 🟢权威
doc_id: EA-DOC-016
last_updated: 2026-08-31
source_of_truth: V2.0目标架构安全与可观测性（未落地）
---

# 06 - 安全与可观测性

> 版本：v2.0 | 日期：2026-08-26 | 状态：企业级草案
>
> 前置：[00-需求分析](docs/expert-alliance/v2/00-requirements.md) | [01-架构设计](docs/expert-alliance/v2/01-architecture.md)


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:3100 / executor-svc:3200），10个内置领域专家，任务仓库为内存+文件快照。

---

## 一、安全架构

### 1.1 安全四件套

```
┌─────────────────────────────────────────────────────────────┐
│                    专家联盟安全架构                              │
│                                                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │  认证     │  │  授权     │  │  审计     │  │  加密     │  │
│  │  Auth    │  │  Authz   │  │  Audit   │  │  Encrypt │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  │
│       │              │              │              │         │
│       ▼              ▼              ▼              ▼         │
│  JWT/OIDC/SSO   RBAC+ABAC     不可篡改日志     TLS/mTLS    │
│  MFA/验证码      数据权限       操作追踪        字段加密     │
│  Token黑名单     接口权限       合规报告        密码哈希     │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 认证（Authentication）

| 认证方式 | 适用场景 | 实现 |
|----------|----------|------|
| JWT | API 调用 | RS256 签名，15min access + 7d refresh |
| OIDC/SSO | 企业用户 | OAuth2 + OpenID Connect |
| API Key | 服务间/MCP | 随机字符串 + HMAC 签名 |
| mTLS | 服务间认证 | Istio 自动 mTLS / SPIFFE |
| MFA | 高安全操作 | TOTP / 短信验证码 |

**JWT Claims**：
```json
{
  "sub": "user-123",
  "tenant_id": "tenant-456",
  "roles": ["tenant_admin", "developer"],
  "permissions": ["graph:read", "ai:execute", "task:create"],
  "exp": 1724656800,
  "iat": 1724655900,
  "jti": "token-uuid",
  "iss": "mox-auth-svc",
  "aud": "mox-platform"
}
```

### 1.3 授权（Authorization）

**RBAC 角色**：

| 角色 | 权限 |
|------|------|
| `platform_admin` | 所有权限（平台级） |
| `tenant_admin` | 租户内所有权限 |
| `expert_developer` | 专家注册/更新/测试，任务创建/查看 |
| `analyst` | 任务创建/查看，图谱只读，AI执行 |
| `viewer` | 只读所有 |
| `mcp_client` | 仅 tools/list + tools/call（MCP 专用） |

**ABAC 数据权限**：
- 全部数据：`tenant_admin`
- 本部门数据：`department_id IN (子部门树)`
- 本人数据：`created_at = user_id`
- 自定义：策略引擎

**接口权限矩阵**（部分）：

| 接口 | platform_admin | tenant_admin | expert_dev | analyst | viewer | mcp_client |
|------|---------------|-------------|-----------|---------|--------|-----------|
| POST /tasks | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ |
| GET /tasks | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ |
| POST /experts | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ |
| GET /experts | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ |
| POST /mcp/tools/call | ✓ | ✓ | ✓ | ✓ | ✗ | ✓ |
| DELETE /tasks | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ |

### 1.4 多租户安全

| 隔离级别 | 实现 | 适用 |
|----------|------|------|
| L1 逻辑隔离（默认） | 所有表 `tenant_id` + PostgreSQL RLS + 图存储 VID前缀 | 中小企业 |
| L2 Schema 隔离 | 每租户独立 PostgreSQL Schema + 图存储独立分片 | 对数据隔离有要求 |
| L3 集群隔离 | 每租户独立 K8s 命名空间 + 独立数据库 + 独立图存储 | 大型企业/合规要求 |

**PostgreSQL RLS 示例**：
```sql
ALTER TABLE tasks ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON tasks
  USING (tenant_id = current_setting('app.tenant_id')::uuid);

-- 应用层设置
SET app.tenant_id = 'tenant-456';
```

### 1.5 审计

所有关键操作记录不可篡改审计日志（哈希链 + WORM 存储）：

| 操作类型 | 审计内容 |
|----------|----------|
| 专家注册/更新/注销 | 专家定义变更前后 |
| 任务创建/取消/干预 | 任务配置/操作人/原因 |
| 权限变更 | 角色/权限变更前后 |
| 数据导出 | 导出范围/数据量/操作人 |
| 登录/登出 | 时间/IP/设备 |
| MCP 工具调用 | 工具名/参数摘要/结果状态 |

### 1.6 加密

| 层级 | 加密方式 |
|------|----------|
| 传输 | TLS 1.3（外部）+ mTLS（服务间，Istio） |
| 存储 | 磁盘加密（LUKS）+ 敏感字段加密（AES-256-GCM/国密SM4） |
| 密码 | Argon2id 哈希 |
| Token | RS256 非对称签名 |
| 对象存储 | 服务端加密（SSE-S3/SM4） |
| 密钥管理 | KMS（轮换/审计/备份） |

**敏感字段自动脱敏**：手机号、身份证、邮箱、密码、Token、API Key


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:3100 / executor-svc:3200），10个内置领域专家，任务仓库为内存+文件快照。

---

## 二、可观测性

### 2.1 三大支柱

```
┌─────────────────────────────────────────────────────────────┐
│                    可观测性三位一体                              │
│                                                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                  │
│  │  日志     │  │  指标     │  │  链路     │                  │
│  │  Logs    │  │ Metrics  │  │ Traces   │                  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘                  │
│       │              │              │                          │
│       ▼              ▼              ▼                          │
│  tracing+Loki    Prometheus     OTel+Jaeger                  │
│  结构化JSON       RED+USE+业务   全链路Trace                  │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
                    ┌──────────┐
                    │ Grafana  │  统一仪表盘
                    └────┬─────┘
                         │
                         ▼
                    ┌──────────┐
                    │AlertMgr  │  P0-P3告警
                    └──────────┘
```

### 2.2 日志（Logs）

**技术栈**：`tracing` + `tracing-subscriber`（JSON格式）→ OTel Collector → Loki

**日志字段规范**（所有日志必须包含）：
`timestamp`, `level`, `service`, `tenant_id`, `user_id`, `trace_id`, `span_id`, `request_id`, `message`, `expert_id`(可选), `task_id`(可选), `node_id`(可选), `duration_ms`(可选), `error`(可选)

**日志级别**：
- ERROR：系统错误/任务失败/节点失败
- WARN：重试/降级/缓存未命中率高
- INFO：任务创建/完成/节点完成/专家注册
- DEBUG：详细参数/中间状态（生产默认关闭）
- TRACE：函数级调用（默认关闭）

### 2.3 指标（Metrics）

**技术栈**：`metrics` + `prometheus` → Prometheus → Grafana

**核心指标分类**：

| 类别 | 指标 | 类型 | 说明 |
|------|------|------|------|
| **任务** | `expert_task_total` | Counter | 任务总数（按状态/类型/租户） |
| | `expert_task_duration_seconds` | Histogram | 任务执行时长分布 |
| | `expert_task_success_rate` | Gauge | 任务成功率 |
| | `expert_task_running` | Gauge | 当前运行中任务数 |
| **节点** | `expert_node_total` | Counter | 节点执行总数（按专家/状态） |
| | `expert_node_duration_seconds` | Histogram | 节点执行时长 |
| | `expert_node_retry_total` | Counter | 节点重试次数 |
| **专家** | `expert_call_total` | Counter | 专家调用次数 |
| | `expert_success_rate` | Gauge | 专家成功率 |
| | `expert_avg_latency_ms` | Gauge | 专家平均延迟 |
| | `expert_health_status` | Gauge | 专家健康状态（0/1） |
| **协作** | `expert_collaboration_pair_total` | Counter | 专家协作对次数 |
| | `expert_collaboration_success_rate` | Gauge | 协作成功率 |
| **融合** | `expert_fusion_total` | Counter | 结果融合次数（按策略） |
| | `expert_fusion_duration_seconds` | Histogram | 融合时长 |
| **案例** | `expert_case_total` | Counter | 案例总数 |
| | `expert_case_use_total` | Counter | 案例复用次数 |
| **MCP** | `mcp_tool_call_total` | Counter | MCP工具调用次数 |
| | `mcp_tool_call_duration_seconds` | Histogram | MCP工具调用时长 |
| **RED** | `requests_total` | Counter | 请求总数（按方法/状态） |
| | `request_duration_seconds` | Histogram | 请求延迟 |
| | `request_errors_total` | Counter | 错误请求 |
| **USE** | `cpu_usage_percent` | Gauge | CPU使用率 |
| | `memory_usage_bytes` | Gauge | 内存使用 |
| | `db_pool_connections` | Gauge | 数据库连接池 |

### 2.4 链路追踪（Traces）

**技术栈**：`tracing-opentelemetry` + `opentelemetry-otlp` → OTel Collector → Jaeger

**关键 Span**：
```
gateway.request
  ├── auth.verify
  ├── tenant.check_quota
  ├── alliance.create_task
  │   ├── registry.match_experts
  │   │   └── kg.graph_query
  │   ├── kg.search_cases
  │   └── alliance.generate_plan
  ├── alliance.execute_task
  │   ├── node.execute (专家A)
  │   │   ├── agent.react_loop
  │   │   │   ├── agent.understand
  │   │   │   ├── agent.plan
  │   │   │   ├── agent.act (工具调用)
  │   │   │   │   └── grpc.call (底层服务)
  │   │   │   ├── agent.observe
  │   │   │   └── agent.review
  │   │   └── ai.inference (Python sidecar)
  │   ├── node.execute (专家B)
  │   └── ...
  ├── alliance.fuse_result
  └── alliance.update_memory
      └── kg.update_edge_weights
```

### 2.5 仪表盘（Grafana）

| 仪表盘 | 内容 | 受众 |
|--------|------|------|
| 全局概览 | 任务数/成功率/延迟/运行中/专家健康 | 运维/管理层 |
| 任务详情 | 单任务DAG执行图/节点状态/耗时/错误/专家思考 | 开发/用户 |
| 专家分析 | 专家调用次数/成功率/延迟/协作关系/贡献度 | 开发/产品 |
| 协作分析 | 协作模式分布/融合策略效果/案例复用率 | 产品 |
| MCP监控 | 工具调用次数/延迟/错误/Top工具 | 运维 |
| 基础设施 | K8s节点/Pod/资源/网络 | 运维 |

### 2.6 告警

| 级别 | 规则 | 响应 |
|------|------|------|
| P0 | 服务不可用 / 任务失败率>50% / 图存储主节点故障 | 5min，电话+短信+飞书 |
| P1 | 任务失败率>10% / P99延迟>5s / 专家健康异常>3个 | 15min，短信+飞书 |
| P2 | 单服务错误率>5% / 队列积压 / 磁盘>80% | 1h，飞书+邮件 |
| P3 | 缓存命中率下降 / 慢查询增加 / 案例复用率低 | 4h，邮件 |


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:3100 / executor-svc:3200），10个内置领域专家，任务仓库为内存+文件快照。

---

## 三、弹性容错

### 3.1 弹性七件套

| 机制 | 实现位置 | 说明 |
|------|----------|------|
| 限流 | 网关 + 服务层 | 租户/用户/接口级，令牌桶+滑动窗口 |
| 熔断 | gRPC客户端 | 三态（Closed/Open/HalfOpen），失败率阈值 |
| 降级 | 联盟核心 | 专家匹配降级/计划生成降级/融合降级 |
| 重试 | 节点执行 | 指数退避+抖动，最多3次，幂等保证 |
| 超时 | 多层级 | 网关30s/服务间5s/数据库2s/AI推理120s |
| 舱壁 | Agent运行时 | 专家实例池隔离，防止单专家耗尽资源 |
| 死信队列 | NATS | 消息重试超限转入DLQ，人工处理 |

### 3.2 降级策略

| 场景 | 降级方案 |
|------|----------|
| 专家匹配服务不可用 | 降级为规则匹配（按领域标签过滤） |
| 图谱查询超时 | 降级为缓存结果/默认专家列表 |
| 协作计划生成失败 | 降级为单专家串行（选Top1专家直接执行） |
| 结果融合失败 | 降级为择优选择（取评分最高的专家结果） |
| AI推理服务不可用 | 降级为模板化回答/缓存结果 |
| 图存储不可用 | 降级为PostgreSQL查询（元数据） |
| 高负载 | 关闭非核心功能（案例推荐/协作统计），保留核心任务执行 |

### 3.3 专家故障自动切换

```
节点执行失败
    │
    ├── 可重试错误（网络/5xx/超时）→ 指数退避重试（≤3次）
    │
    ├── 重试耗尽
    │     │
    │     ├── 有替代专家（同领域同能力，评分≥0.5）→ 自动切换替代专家重试
    │     │
    │     ├── 非关键路径节点 → 标记Skipped，继续下游（降级）
    │     │
    │     └── 关键路径节点 → 任务失败，通知用户
    │
    └── 不可恢复错误（专家不存在/权限拒绝/参数错误）→ 立即失败
```


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:3100 / executor-svc:3200），10个内置领域专家，任务仓库为内存+文件快照。

---

## 四、SLA 目标

| 指标 | 目标 |
|------|------|
| 服务可用性 | 99.95%（月停机<22分钟） |
| 任务创建响应 | P99 < 500ms |
| 专家匹配 | P99 < 200ms |
| 节点执行（不含AI） | P99 < 2s |
| AI流式首包 | < 500ms |
| 任务进度推送延迟 | < 1s |
| 并发任务数 | ≥ 100 |
| 数据持久性 | 99.999999999%（11个9） |
| RPO | < 1分钟 |
| RTO | < 15分钟 |

---

*下一篇：[07-实施路线图](docs/expert-alliance/v2/07-roadmap.md)*

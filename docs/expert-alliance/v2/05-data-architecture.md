---
title: 05 - 数据架构
version: V2.0
authority: 🟢权威
doc_id: EA-DOC-015
last_updated: 2026-08-31
source_of_truth: V2.0目标架构数据架构（未落地）
---

# 05 - 数据架构

> 版本：v2.0 | 日期：2026-08-26 | 状态：企业级草案
>
> 前置：[00-需求分析](docs/expert-alliance/v2/00-requirements.md) | [01-架构设计](docs/expert-alliance/v2/01-architecture.md) | [02-领域模型](docs/expert-alliance/v2/02-domain-model.md)


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:3100 / executor-svc:3200），10个内置领域专家，任务仓库为内存+文件快照。

---

## 一、数据架构总览

### 1.1 数据存储选型

| 数据类型 | 存储引擎 | 说明 |
|----------|----------|------|
| **业务关系数据** | PostgreSQL 16 | 任务/专家/注册/案例元数据 |
| **知识图谱** | 自研图存储（RocksDB+Raft） | 专家-能力-领域-工具-数据-案例关联网络 |
| **缓存** | Redis 7 | 会话/工作记忆/专家列表/匹配结果/限流 |
| **事件/消息** | NATS JetStream | 领域事件/任务进度/异步解耦 |
| **对象存储** | MinIO | 任务结果/导出文件/大文本/附件 |
| **向量索引** | pgvector | 案例语义检索/专家描述相似度 |
| **全文搜索** | PostgreSQL tsvector | 任务/专家/案例全文搜索 |
| **时序数据** | TimescaleDB（可选） | 专家调用指标/任务执行统计 |

### 1.2 数据流向

```
用户请求
    │
    ▼
┌─────────────┐     ┌─────────────┐
│  PostgreSQL  │◄────│  联盟核心    │ 任务/节点/结果
│  (业务数据)  │     │  (alliance) │
└─────────────┘     └──────┬──────┘
                            │
                     ┌──────▼──────┐
                     │    Redis     │ 缓存/会话/工作记忆/限流
                     │   (缓存)     │
                     └──────┬──────┘
                            │
                     ┌──────▼──────┐
                     │ NATS JetStream│ 事件总线
                     │  (事件/消息)  │
                     └──────┬──────┘
                            │
          ┌─────────────────┼─────────────────┐
          ▼                 ▼                 ▼
   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
   │ 自研图存储   │  │    MinIO    │  │  订阅服务     │
   │ (知识图谱)   │  │  (对象存储)  │  │ (通知/审计)  │
   └─────────────┘  └─────────────┘  └─────────────┘
```


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:3100 / executor-svc:3200），10个内置领域专家，任务仓库为内存+文件快照。

---

## 二、PostgreSQL 数据模型

### 2.1 任务表

```sql
CREATE TABLE tasks (
    task_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL,
    user_id         UUID NOT NULL,
    title           VARCHAR(256) NOT NULL,
    description     TEXT NOT NULL,
    task_type       VARCHAR(64) NOT NULL DEFAULT 'custom',
    status          VARCHAR(32) NOT NULL DEFAULT 'pending',
    progress        REAL NOT NULL DEFAULT 0,
    current_node_id VARCHAR(128),

    -- 协作配置
    preference      JSONB NOT NULL DEFAULT '{}',   -- CollaborationPreference
    constraints     JSONB NOT NULL DEFAULT '{}',   -- TaskConstraints
    inputs          JSONB NOT NULL DEFAULT '[]',   -- DataReference[]

    -- 计划与结果
    plan            JSONB,                           -- CollaborationPlan
    result          JSONB,                           -- TaskResult
    error           JSONB,                           -- TaskError

    -- 时间
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    duration_ms     BIGINT,

    -- 索引
    CONSTRAINT chk_status CHECK (status IN
        ('pending','planning','running','paused','completed','failed','cancelled'))
);

CREATE INDEX idx_tasks_tenant_status ON tasks(tenant_id, status);
CREATE INDEX idx_tasks_tenant_created ON tasks(tenant_id, created_at DESC);
CREATE INDEX idx_tasks_user ON tasks(tenant_id, user_id);
CREATE INDEX idx_tasks_type ON tasks(tenant_id, task_type);
CREATE INDEX idx_tasks_status ON tasks(status) WHERE status IN ('running','pending');
-- 全文搜索
CREATE INDEX idx_tasks_fts ON tasks USING gin(to_tsvector('english', title || ' ' || description));
```

### 2.2 节点执行表

```sql
CREATE TABLE task_nodes (
    node_id         VARCHAR(128) NOT NULL,
    task_id         UUID NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    expert_id       VARCHAR(128) NOT NULL,
    expert_name     VARCHAR(128) NOT NULL,
    node_type       VARCHAR(32) NOT NULL DEFAULT 'execute',
    status          VARCHAR(32) NOT NULL DEFAULT 'pending',

    -- 配置
    config          JSONB NOT NULL DEFAULT '{}',
    input_refs      JSONB NOT NULL DEFAULT '[]',
    output_defs     JSONB NOT NULL DEFAULT '[]',

    -- 执行结果
    result          JSONB,
    error           TEXT,
    thoughts        JSONB NOT NULL DEFAULT '[]',   -- ExpertThought[]
    metrics         JSONB,                           -- NodeMetrics

    -- 重试
    retry_count     INT NOT NULL DEFAULT 0,
    max_retries     INT NOT NULL DEFAULT 3,

    -- 时间
    scheduled_at    TIMESTAMPTZ,
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,

    PRIMARY KEY (task_id, node_id)
);

CREATE INDEX idx_task_nodes_task ON task_nodes(task_id);
CREATE INDEX idx_task_nodes_status ON task_nodes(status) WHERE status = 'running';
CREATE INDEX idx_task_nodes_expert ON task_nodes(expert_id);
```

### 2.3 专家表

```sql
CREATE TABLE experts (
    expert_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL,              -- 'system' = 内置
    name            VARCHAR(128) NOT NULL,
    description     TEXT,
    role            VARCHAR(64) NOT NULL,
    version         VARCHAR(32) NOT NULL DEFAULT '1.0.0',
    priority        INT NOT NULL DEFAULT 5,
    status          VARCHAR(32) NOT NULL DEFAULT 'active',

    -- 定义
    domains         JSONB NOT NULL DEFAULT '[]',
    capabilities    JSONB NOT NULL DEFAULT '[]',
    tools           JSONB NOT NULL DEFAULT '[]',
    knowledge       JSONB NOT NULL DEFAULT '{}',
    personality     JSONB NOT NULL DEFAULT '{}',
    memory_config   JSONB NOT NULL DEFAULT '{}',
    metadata        JSONB NOT NULL DEFAULT '{}',

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(tenant_id, name),
    CONSTRAINT chk_expert_status CHECK (status IN ('active','inactive','maintenance','deprecated'))
);

CREATE INDEX idx_experts_tenant ON experts(tenant_id);
CREATE INDEX idx_experts_status ON experts(status);
CREATE INDEX idx_experts_role ON experts(role);
CREATE INDEX idx_experts_fts ON experts USING gin(to_tsvector('english', name || ' ' || description));
```

### 2.4 能力/工具/领域表

```sql
-- 能力定义（全局共享）
CREATE TABLE capabilities (
    capability_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(128) NOT NULL UNIQUE,
    description     TEXT,
    category        VARCHAR(64),
    input_types     JSONB NOT NULL DEFAULT '[]',
    output_types    JSONB NOT NULL DEFAULT '[]',
    confidence      REAL NOT NULL DEFAULT 0.8,
    requires_expertise JSONB NOT NULL DEFAULT '[]',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 工具定义（从gRPC反射自动生成）
CREATE TABLE tools (
    tool_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(128) NOT NULL UNIQUE,
    description     TEXT,
    service_name    VARCHAR(128) NOT NULL,
    method          VARCHAR(256) NOT NULL,
    async           BOOLEAN NOT NULL DEFAULT FALSE,
    parameters      JSONB,                           -- JSON Schema
    returns         JSONB,
    category        VARCHAR(64),
    timeout_ms      BIGINT NOT NULL DEFAULT 30000,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(service_name, method)
);

CREATE INDEX idx_tools_service ON tools(service_name);
CREATE INDEX idx_tools_category ON tools(category);

-- 领域树
CREATE TABLE domains (
    domain_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(128) NOT NULL UNIQUE,
    description     TEXT,
    parent_domain_id UUID REFERENCES domains(domain_id),
    level           INT NOT NULL DEFAULT 0,
    path            VARCHAR(512) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_domains_parent ON domains(parent_domain_id);
CREATE INDEX idx_domains_path ON domains(path);
```

### 2.5 案例表

```sql
CREATE TABLE cases (
    case_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL,
    source_task_id  UUID NOT NULL REFERENCES tasks(task_id),
    title           VARCHAR(256) NOT NULL,
    description     TEXT,
    task_type       VARCHAR(64) NOT NULL,
    input_summary   TEXT,
    output_summary  TEXT,

    -- 协作快照
    expert_ids      JSONB NOT NULL DEFAULT '[]',
    mode            VARCHAR(32),
    fusion_strategy VARCHAR(32),
    plan_snapshot   JSONB,

    -- 评分
    rating          REAL NOT NULL DEFAULT 0,        -- 0-5
    success_rate    REAL NOT NULL DEFAULT 0,        -- 复现成功率
    execution_time_ms BIGINT,
    use_count       INT NOT NULL DEFAULT 0,

    -- 向量（pgvector）
    embedding       vector(1536),                   -- 语义向量

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at    TIMESTAMPTZ
);

CREATE INDEX idx_cases_tenant ON cases(tenant_id);
CREATE INDEX idx_cases_type ON cases(tenant_id, task_type);
CREATE INDEX idx_cases_rating ON cases(rating DESC);
CREATE INDEX idx_cases_embedding ON cases USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
CREATE INDEX idx_cases_fts ON cases USING gin(to_tsvector('english', title || ' ' || description));
```

### 2.6 审计日志表

```sql
CREATE TABLE audit_logs (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       UUID NOT NULL,
    user_id         UUID,
    service         VARCHAR(64) NOT NULL,
    action          VARCHAR(64) NOT NULL,
    resource_type   VARCHAR(64),
    resource_id     VARCHAR(128),
    ip_address      INET,
    user_agent      TEXT,
    request_id      VARCHAR(128),
    trace_id        VARCHAR(128),
    result          VARCHAR(16) NOT NULL,           -- success/failure
    reason          TEXT,
    before          JSONB,
    after           JSONB,
    hash            VARCHAR(128) NOT NULL,           -- 哈希链
    prev_hash       VARCHAR(128),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_tenant ON audit_logs(tenant_id, created_at DESC);
CREATE INDEX idx_audit_user ON audit_logs(user_id, created_at DESC);
CREATE INDEX idx_audit_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX idx_audit_action ON audit_logs(action, created_at DESC);

-- 不可篡改：禁止UPDATE/DELETE（通过触发器或应用层保证）
```


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:3100 / executor-svc:3200），10个内置领域专家，任务仓库为内存+文件快照。

---

## 三、知识图谱数据模型

### 3.1 图存储复用

专家联盟知识图谱完全复用自研 `mox-graph-storage-svc`，通过 gRPC 调用。

**多租户隔离**：VID 租户前缀方案
- 系统专家：`vid = "system:expert:{expert_id}"`
- 租户专家：`vid = "{tenant_id}:expert:{expert_id}"`
- 案例：`vid = "{tenant_id}:case:{case_id}"`
- 底层引擎完全不感知租户

### 3.2 顶点类型（7种）

| 顶点类型 | VID 前缀 | 核心属性 | 租户隔离 |
|----------|----------|----------|----------|
| Expert | `expert:` | expert_id, name, role, priority, status | 是（system共享） |
| Capability | `cap:` | capability_id, name, category, input_types, output_types | 否（全局） |
| Domain | `domain:` | domain_id, name, level, path | 否（全局） |
| Tool | `tool:` | tool_id, name, service_name, method, category | 否（全局） |
| Data | `data:` | data_id, name, type, source, sensitivity | 是 |
| Case | `case:` | case_id, title, task_type, rating, success_rate | 是 |
| Task | `task:` | task_id, status, created_at | 是（TTL归档） |

### 3.3 边类型（12种）

| 边类型 | 起点→终点 | 权重属性 | 说明 |
|--------|----------|----------|------|
| has_capability | Expert→Capability | proficiency(0-1), usage_count, success_rate | 专家具备能力 |
| operates_in | Expert→Domain | expertise_level, task_count, avg_rating | 专家活跃领域 |
| requires_tool | Capability→Tool | mandatory, usage_frequency | 能力需要工具 |
| operates_on | Tool→Data | operation(r/w/x) | 工具操作数据 |
| contains_data | Domain→Data | data_category | 领域包含数据 |
| solved_by | Case/Task→Expert | contribution(0-1), role, rating | 案例由专家解决 |
| used_capability | Case/Task→Capability | effectiveness(0-1), usage_count | 案例使用能力 |
| similar_to | Case→Case | similarity(0-1), dimensions | 案例相似 |
| collaborates_with | Expert→Expert | frequency, success_rate, avg_duration | 专家协作历史 |
| depends_on | Capability→Capability | dependency_type | 能力依赖 |
| subdomain_of | Domain→Domain | - | 领域父子 |
| executed_by | Task→Expert | node_id, status | 任务由专家执行（运行时） |

### 3.4 图谱初始化数据

**领域树**（约30个节点）：
```
知识图谱 → 图谱构建/查询/推理/治理
数据分析 → 统计分析/趋势预测/异常检测/可视化
人工智能 → NLP/文本生成/语义理解/多模态
安全合规 → 权限审计/数据脱敏/合规检查/风险评估
工作流 → 流程设计/任务编排/自动化执行
数据治理 → 数据标准/数据质量/元数据/数据目录
```

**能力定义**（约20个）：
- 推理类：逻辑推理/因果推理/类比推理
- 处理类：数据清洗/格式转换/实体抽取/关系抽取
- 分析类：统计分析/趋势分析/异常检测/根因分析
- 生成类：文本生成/摘要生成/报告生成/代码生成
- 检索类：语义检索/图谱检索/向量检索/全文检索

**工具注册**：从所有 gRPC 服务反射自动发现（预计50+工具）

**内置专家**：10个（图谱构建/数据分析/AI推理/安全审计/流程自动化/数据治理/知识融合/搜索推荐/运维监控/联盟协调）


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:3100 / executor-svc:3200），10个内置领域专家，任务仓库为内存+文件快照。

---

## 四、Redis 数据模型

### 4.1 Key 命名规范

```
{tenant_id}:{category}:{identifier}

示例：
  t1:session:user_123              # 用户会话
  t1:task:task_456:working_memory  # 任务工作记忆
  t1:cache:expert_list              # 专家列表缓存
  t1:cache:match:{hash}             # 匹配结果缓存
  t1:ratelimit:{user_id}:{api}     # 限流计数
  t1:lock:task:{task_id}            # 任务分布式锁
  t1:idempotency:{key}              # 幂等键
  t1:blacklist:token:{jti}          # Token黑名单
```

### 4.2 关键数据结构

| Key | 类型 | TTL | 说明 |
|-----|------|-----|------|
| `{t}:session:{user_id}` | Hash | 24h | 用户会话（偏好/历史/常用专家） |
| `{t}:task:{id}:working_memory` | Hash | 任务结束+1h | 工作记忆（上下文/中间结果） |
| `{t}:task:{id}:node_outputs` | Hash | 任务结束+1h | 节点输出（供下游读取） |
| `{t}:cache:expert_list` | String(JSON) | 5m | 专家列表缓存 |
| `{t}:cache:match:{hash}` | String(JSON) | 10m | 专家匹配结果缓存 |
| `{t}:cache:tools` | String(JSON) | 5m | MCP工具列表缓存 |
| `{t}:ratelimit:{user}:{api}` | String(counter) | 1m | 限流计数（滑动窗口） |
| `{t}:lock:task:{id}` | String | 30s | 任务调度分布式锁 |
| `{t}:idempotency:{key}` | String | 24h | 幂等键（防重复） |
| `{t}:progress:{task_id}` | Stream | 1h | 任务进度事件流（WebSocket推送） |


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:3100 / executor-svc:3200），10个内置领域专家，任务仓库为内存+文件快照。

---

## 五、数据一致性

### 5.1 一致性策略

| 场景 | 一致性模型 | 实现 |
|------|-----------|------|
| 任务创建/状态更新 | 强一致 | PostgreSQL 事务 |
| 专家注册/更新 | 强一致 | PostgreSQL 事务 + 图谱同步（事件最终一致） |
| 任务进度推送 | 最终一致 | NATS 事件 + Redis 缓存 |
| 图谱边权重更新 | 最终一致 | 异步批量更新（任务完成后） |
| 案例提升 | 最终一致 | 事件驱动（任务评分≥4→异步创建Case） |
| 缓存更新 | 最终一致 | Cache-Aside + TTL 过期 |

### 5.2 分布式事务（Saga）

专家注册涉及 PostgreSQL + 图存储双写，采用 Saga 模式：

```
1. 写入 PostgreSQL（experts表）
2. 发布 expert.registered 事件（NATS）
3. kg-svc 订阅事件 → 写入图存储（创建Expert节点+关联边）
4. 如果图存储写入失败 → 发布补偿事件 → 回滚PostgreSQL（或标记为不一致，定时修复）
```

### 5.3 幂等性

所有写操作支持 `idempotency_key`：
- 客户端生成唯一 key（UUID）
- 服务端在 Redis 记录 key → 结果映射
- 重复请求直接返回缓存结果
- TTL 24小时


> ⚠️ **文档状态声明**  
> 本文档为 V2.0 **目标架构设计**，描述的"7个核心服务/31个微服务/PostgreSQL+Redis+Kafka/v2 API路径"等架构**尚未落地实现**。  
> 当前实际实现以 `docs/alliance-architecture-fix-report-20260831.html` 为准：11个crate（proto×3/core×4/svc×2/sdk×1/api×1），2个HTTP服务（scheduler-svc:3100 / executor-svc:3200），10个内置领域专家，任务仓库为内存+文件快照。

---

## 六、数据迁移与备份

### 6.1 迁移工具

使用 `sqlx-cli` 管理数据库迁移：

```
migrations/
├── 001_init.sql                    # 初始表结构
├── 002_add_task_indexes.sql        # 添加索引
├── 003_add_cases_embedding.sql     # 案例向量
├── 004_add_audit_logs.sql          # 审计日志
└── ...
```

迁移策略：扩展-迁移-收缩（Expand-Migrate-Contract）

### 6.2 备份策略

| 数据 | 备份方式 | 频率 | 保留 | RPO |
|------|----------|------|------|-----|
| PostgreSQL | pg_dump + WAL归档 | 全量每日 + WAL实时 | 30天 | <1min |
| 图存储 | RocksDB快照 + CDC | 快照每小时 + CDC实时 | 30天 | <1min |
| Redis | RDB + AOF | RDB每小时 + AOF实时 | 7天 | <5min |
| MinIO | 跨区域复制 | 实时 | 90天 | 0 |
| NATS | 流持久化 | 实时 | 7天 | <1min |

---

*下一篇：[06-安全与可观测性](docs/expert-alliance/v2/06-security-observability.md)*

# 03 - 数据架构优化

> 版本：v1.0 | 日期：2026-08-26 | 状态：草案
>
> 前置阅读：[00-核心原则](./00-principles.md) | [01-服务边界优化](./01-service-boundaries.md) | [02-通信架构优化](./02-communication.md)

## 一、现状诊断

### 1.1 当前数据存储

| 数据类型 | 存储引擎 | 位置 | 问题 |
|----------|----------|------|------|
| 关系数据 | PostgreSQL（通过 sqlx） | 各服务共享？ | 无明确 Database per Service |
| 图数据 | ★自研 mox-graph-storage★（RocksDB + Raft） | 独立模块 | 无多租户隔离 |
| 缓存 | Redis | 共享 | 无明确租户隔离 |
| 对象存储 | S3 兼容（mox-cloud-drive-s3） | 独立模块 | 无租户隔离 |
| 配置数据 | 文件/YAML | 各服务 | 无配置中心 |
| 大文件 | graph.json (91MB) 入库 | Git 仓库 | 大文件不应入库 |
| 运行时数据 | .runtime/, *.db | 根目录 | 运行时数据入库 |

### 1.2 核心问题

| 问题 | 影响 | 严重度 |
|------|------|--------|
| **无 Database per Service** | 服务间可能共享数据库，紧耦合，无法独立扩展 | 🔴 高 |
| **无多租户数据隔离** | 租户数据可能互相访问，安全风险，无法做 SaaS | 🔴 高 |
| **无数据一致性策略** | 跨服务操作无 Saga/最终一致性保障 | 🟡 中 |
| **无数据迁移工具** | Schema 变更无版本管理，回滚困难 | 🟡 中 |
| **无备份容灾策略** | 数据丢失风险，RPO/RTO 无保障 | 🟡 中 |
| **缓存策略不统一** | 缓存穿透/击穿/雪崩风险 | 🟡 中 |
| **大文件入库** | Git 性能差，仓库膨胀 | 🟢 低 |
| **运行时数据入库** | 仓库污染，环境不一致 | 🟢 低 |

---

## 二、目标数据架构

### 2.1 Database per Service 模式

每个服务拥有独立的数据库/Schema，其他服务只能通过 API 访问，不能直接访问数据库。

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  auth-svc   │  │  tenant-svc │  │  system-svc │
│  PostgreSQL │  │  PostgreSQL │  │  PostgreSQL │
│  Schema: auth│  │ Schema:tenant│ │ Schema:system│
└─────────────┘  └─────────────┘  └─────────────┘

┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│   ai-svc    │  │  agent-svc  │  │ graph-svc   │
│  PostgreSQL │  │  PostgreSQL │  │  PostgreSQL │
│  Schema: ai │  │Schema:agent │  │Schema:graph │
└─────────────┘  └─────────────┘  └─────────────┘
       │
       ▼
┌─────────────────────────────────────────────┐
│  mox-graph-storage-svc（★自研分布式图存储★）  │
│  RocksDB + Sharded Raft + CDC + LRU Cache    │
│  多租户：租户前缀隔离（VID = tenant_id:raw_vid）│
└─────────────────────────────────────────────┘

┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ storage-svc │  │  flow-svc   │  │metering-svc │
│ PostgreSQL  │  │ PostgreSQL  │  │ PostgreSQL  │
│Schema:storage│ │Schema:flow  │  │Schema:meter │
└─────────────┘  └─────────────┘  └─────────────┘

共享基础设施（非业务数据）：
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│   Redis     │  │NATS JetStream│ │  MinIO/S3   │
│  缓存/会话   │  │  消息队列    │  │  对象存储    │
└─────────────┘  └─────────────┘  └─────────────┘
```

### 2.2 数据存储选型

| 数据类型 | 存储引擎 | 用途 | 多租户隔离 |
|----------|----------|------|-----------|
| **关系数据** | PostgreSQL 16 | 用户/角色/权限/租户/配置/业务元数据 | tenant_id + RLS |
| **图数据** | ★自研 mox-graph-storage★ | 知识图谱（顶点/边/属性） | 租户前缀 + 分片组 |
| **缓存** | Redis 7 | 会话/Token黑名单/热点数据/分布式锁/限流 | key前缀 tenant: |
| **消息队列** | NATS JetStream | 异步事件/CDC/任务队列/通知/死信 | subject前缀 tenant. |
| **对象存储** | MinIO / S3 | 文件/模型/插件包/备份/日志归档 | bucket前缀 tenant- |
| **向量数据** | pgvector（起步）→ Qdrant（规模化） | RAG检索/语义搜索/嵌入向量 | tenant_id 字段过滤 |
| **全文搜索** | PostgreSQL tsvector（起步）→ Tantivy（规模化） | 全文检索 | tenant_id 过滤 |
| **时序数据** | TimescaleDB（可选） | 监控指标/用量统计/审计日志 | tenant_id |

### 2.3 为什么用 PostgreSQL 而不是其他

| 原因 | 说明 |
|------|------|
| **RLS（行级安全）** | PostgreSQL 原生支持行级安全，是多租户数据隔离的基石 |
| **JSONB** | 支持半结构化数据，灵活存储动态属性 |
| **pgvector** | 向量检索扩展，RAG 场景直接用 |
| **全文搜索** | tsvector 全文索引，起步阶段无需额外搜索引擎 |
| **事务** | ACID 事务，复杂业务逻辑可靠 |
| **生态成熟** | sqlx 异步支持好，工具链完善 |
| **成本低** | 开源免费，运维成熟 |

---

## 三、多租户数据隔离

### 3.1 三档隔离策略

| 级别 | 实现 | 隔离强度 | 适用场景 | 性能损耗 |
|------|------|----------|----------|----------|
| **L1 逻辑隔离** | 所有表加 tenant_id，查询自动追加条件，PostgreSQL RLS 兜底 | 逻辑 | 免费/基础版，默认 | <5% |
| **L2 Schema隔离** | 每租户独立 PostgreSQL Schema，`SET search_path`；图存储每租户独立分片组 | 物理Schema | Pro版 | ~10% |
| **L3 集群隔离** | 每租户独立数据库实例 + 独立图存储集群 | 完全物理 | 企业版/金融/政府 | 独立资源 |

### 3.2 L1 逻辑隔离实现（默认）

#### PostgreSQL RLS

```sql
-- 所有业务表必须包含 tenant_id
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    username VARCHAR(255) NOT NULL,
    email VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(tenant_id, username)
);

-- 启用行级安全
ALTER TABLE users ENABLE ROW LEVEL SECURITY;

-- 创建租户隔离策略
CREATE POLICY tenant_isolation ON users
    USING (tenant_id = current_setting('app.tenant_id')::UUID);

-- 创建索引（必须包含 tenant_id）
CREATE INDEX idx_users_tenant_id ON users(tenant_id);
CREATE INDEX idx_users_tenant_username ON users(tenant_id, username);
```

#### Rust 侧租户上下文

```rust
// libs/mox-db/src/tenant.rs
use sqlx::{Postgres, PgPool};
use std::sync::Arc;

#[derive(Clone)]
pub struct TenantContext {
    pub tenant_id: String,
    pub user_id: Option<String>,
    pub trace_id: String,
}

impl TenantContext {
    /// 从 gRPC Request 提取租户上下文
    pub fn from_request<T>(req: &tonic::Request<T>) -> Result<Self, tonic::Status> {
        let meta = req.metadata();
        let tenant_id = meta.get("x-tenant-id")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| tonic::Status::unauthenticated("missing tenant_id"))?
            .to_string();
        let user_id = meta.get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let trace_id = meta.get("x-trace-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        Ok(Self { tenant_id, user_id, trace_id })
    }
}

/// 租户感知的数据库连接
pub struct TenantDb {
    pool: PgPool,
}

impl TenantDb {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 获取设置了租户上下文的连接
    pub async fn acquire(&self, ctx: &TenantContext) -> Result<sqlx::pool::PoolConnection<Postgres>, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        // 设置会话级 tenant_id，RLS 策略自动生效
        sqlx::query("SET app.tenant_id = $1")
            .bind(&ctx.tenant_id)
            .execute(&mut *conn)
            .await?;
        Ok(conn)
    }

    /// 执行租户隔离的查询
    pub async fn fetch_one<'q, T>(&self, ctx: &TenantContext, query: &'q str) -> Result<T, sqlx::Error>
    where T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'q
    {
        let mut conn = self.acquire(ctx).await?;
        let result = sqlx::query_as::<_, T>(query)
            .fetch_one(&mut *conn)
            .await?;
        Ok(result)
    }
}
```

### 3.3 自研图存储多租户隔离

已有 mox-graph-storage 是 VID hash 分片 + 每分片 Raft Group。用 **租户前缀方案**，底层引擎零修改：

```rust
// libs/mox-tenant/src/graph_vid.rs

pub struct TenantVidCodec;

impl TenantVidCodec {
    /// 编码：VID = "{tenant_id}:{raw_vid}"
    pub fn encode(tenant_id: &str, raw_vid: &str) -> String {
        format!("{}:{}", tenant_id, raw_vid)
    }

    /// 解码
    pub fn decode(tenant_vid: &str) -> Option<(&str, &str)> {
        tenant_vid.split_once(':')
    }

    /// 租户前缀（用于范围扫描）
    pub fn tenant_prefix(tenant_id: &str) -> String {
        format!("{}:", tenant_id)
    }

    /// 去除租户前缀
    pub fn strip_tenant(tenant_vid: &str) -> &str {
        tenant_vid.split_once(':').map(|(_, v)| v).unwrap_or(tenant_vid)
    }
}
```

gRPC Server 层在调用 `R2StorageServer` 前自动拼接租户前缀：

```rust
// services/graph-storage/src/grpc_server.rs
use mox_tenant::TenantVidCodec;

async fn add_vertex(&self, req: Request<AddVertexRequest>) -> Result<Response<VertexAck>, Status> {
    let req = req.into_inner();
    let tenant_id = req.meta.as_ref().unwrap().tenant_id.clone();
    // 租户前缀注入，底层引擎完全不知道租户存在
    let vid = TenantVidCodec::encode(&tenant_id, &req.vid);
    let ack = self.inner.add_vertex(&vid, &req.tag, req.props).await
        .map_err(|e| Status::internal(e.to_string()))?;
    // 返回时去除租户前缀
    let mut ack: VertexAck = ack.into();
    ack.vid = TenantVidCodec::strip_tenant(&ack.vid).to_string();
    Ok(Response::new(ack))
}

async fn get_neighbors(&self, req: Request<GetNeighborsRequest>) -> Result<Response<GetNeighborsResponse>, Status> {
    let req = req.into_inner();
    let tenant_id = req.meta.as_ref().unwrap().tenant_id.clone();
    let vid = TenantVidCodec::encode(&tenant_id, &req.vid);
    let neighbors = self.inner.neighbors(&vid, direction, etype, limit, offset)
        .await.map_err(|e| Status::internal(e.to_string()))?;
    // 返回时去除所有邻居 VID 的租户前缀
    let neighbors: Vec<Neighbor> = neighbors.into_iter().map(|mut n| {
        n.neighbor_vid = TenantVidCodec::strip_tenant(&n.neighbor_vid).to_string();
        n
    }).collect();
    Ok(Response::new(GetNeighborsResponse { neighbors }))
}
```

**CDC 事件也按租户过滤**：
```rust
async fn subscribe_cdc(&self, req: Request<CdcSubscription>) -> Result<Response<Self::SubscribeCdcStream>, Status> {
    let req = req.into_inner();
    let tenant_prefix = TenantVidCodec::tenant_prefix(&req.tenant_id);
    let cdc = self.inner.cdc_source().subscribe_with_filter(move |event| {
        // 只推送当前租户的事件
        event.vid.starts_with(&tenant_prefix)
    });
    // ...
}
```

### 3.4 L2/L3 隔离升级路径

| 升级条件 | 操作 |
|----------|------|
| 租户数据量 > 1000万顶点 | 升级到 L2：该租户独立 Schema + 独立图分片组 |
| 租户是金融/政府/医疗客户 | 升级到 L3：独立数据库实例 + 独立图存储集群 |
| 租户要求数据物理隔离 | 升级到 L3 |

升级过程：
1. 创建独立 Schema/数据库
2. 数据迁移（双写过渡期）
3. 切换路由（租户配置标记隔离级别）
4. 验证一致性
5. 删除旧数据

---

## 四、数据一致性

### 4.1 一致性模型

| 场景 | 一致性模型 | 实现 |
|------|-----------|------|
| 单服务内操作 | 强一致性（ACID） | PostgreSQL 事务 |
| 跨服务操作 | 最终一致性 | Saga 模式（事件驱动） |
| 读操作 | 最终一致性 | 缓存 + 定期刷新 |
| 计数/统计 | 最终一致性 | 异步聚合 + 定期校准 |

### 4.2 Saga 模式

对于跨服务的业务操作，用 Saga 模式保证最终一致性：

```
示例：创建 AI Agent（涉及 agent-svc + ai-svc + metering-svc + notification-svc）

Saga 编排：
  1. agent-svc: 创建 Agent（本地事务）
     → 发布事件: agent.created
     ↓
  2. ai-svc: 订阅 agent.created
     → 初始化 AI 配置（本地事务）
     → 发布事件: agent.ai.initialized
     ↓
  3. metering-svc: 订阅 agent.created
     → 创建用量记录（本地事务）
     → 发布事件: agent.metering.initialized
     ↓
  4. notification-svc: 订阅 agent.created
     → 发送创建通知（本地事务）

补偿（任何步骤失败）：
  → 发布事件: agent.creation.failed
  → agent-svc: 标记 Agent 为失败状态
  → ai-svc: 清理 AI 配置
  → metering-svc: 回滚用量记录
```

**Saga 两种实现方式**：

| 方式 | 实现 | 适用 |
|------|------|------|
| **编排式（Choreography）** | 各服务订阅事件，自主执行和补偿 | 简单流程，服务少 |
| **协调式（Orchestration）** | 中心协调器（Saga Coordinator）控制流程 | 复杂流程，服务多 |

**推荐：编排式起步，复杂流程用协调式**（可在 flow-svc 中实现 Saga 协调器）

### 4.3 CQRS 模式

对于读多写少的场景，用 CQRS（命令查询职责分离）：

```
写操作（Command）：
  用户请求 → 网关 → 写服务 → 数据库（主库）
                          → 发布事件（数据变更）

读操作（Query）：
  用户请求 → 网关 → 读服务 → 读模型（从读库/缓存/物化视图读取）

事件同步：
  写服务发布事件 → 读服务订阅 → 更新读模型（物化视图/缓存/搜索引擎）
```

**适用场景**：
- 图谱查询（写少读多，读模型可预计算）
- 报表/统计（异步聚合）
- 搜索索引（异步同步到搜索引擎）

### 4.4 幂等性设计

所有写操作必须幂等（重复调用结果相同）：

| 幂等手段 | 实现 |
|----------|------|
| **请求 ID** | 客户端生成唯一 request_id，服务端记录已处理的 request_id |
| **唯一约束** | 数据库唯一索引（如 tenant_id + username） |
| **乐观锁** | 版本号字段，更新时检查版本 |
| **自然键** | 业务唯一标识（如订单号） |

```sql
-- 幂等记录表
CREATE TABLE idempotency_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    request_id VARCHAR(255) NOT NULL,
    service_name VARCHAR(255) NOT NULL,
    response JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(tenant_id, service_name, request_id)
);
```

---

## 五、数据迁移

### 5.1 迁移工具

**sqlx-cli**（Rust 原生，与 sqlx 无缝集成）：

```bash
# 安装
cargo install sqlx-cli --no-default-features --features rustls,postgres

# 创建迁移
sqlx migrate add create_users_table

# 执行迁移
sqlx migrate run

# 回滚
sqlx migrate revert
```

### 5.2 迁移文件规范

```
migrations/
├── 0001_initial.sql
├── 0002_add_tenant_id.sql
├── 0003_enable_rls.sql
├── 0004_create_indexes.sql
└── ...
```

每个迁移文件必须包含：
```sql
-- 0002_add_tenant_id.sql
-- 描述：为所有业务表添加 tenant_id 字段
-- 作者：xxx
-- 日期：2026-08-26

BEGIN;

ALTER TABLE users ADD COLUMN tenant_id UUID;
UPDATE users SET tenant_id = '00000000-0000-0000-0000-000000000000' WHERE tenant_id IS NULL;
ALTER TABLE users ALTER COLUMN tenant_id SET NOT NULL;
CREATE INDEX idx_users_tenant_id ON users(tenant_id);

-- ... 其他表

COMMIT;
```

### 5.3 迁移策略

| 场景 | 策略 |
|------|------|
| **新服务** | 从第一个迁移文件开始，干净的 Schema |
| **现有服务拆分** | 1. 创建新 Schema 2. 数据迁移（双写） 3. 切换读 4. 切换写 5. 删除旧表 |
| **大表迁移** | 分批迁移 + 进度跟踪 + 回滚方案 |
| **破坏性变更** | 扩展-迁移-收缩（Expand-Migrate-Contract）：先加新字段，双写，切换读，删除旧字段 |

### 5.4 数据迁移验证

每次迁移后必须验证：
1. 行数一致
2. 关键字段值一致（抽样对比）
3. 索引完整
4. 约束生效
5. 应用功能正常

---

## 六、数据备份与容灾

### 6.1 备份策略

| 数据类型 | 备份方式 | 频率 | 保留期 | RPO |
|----------|----------|------|--------|-----|
| PostgreSQL | pg_dump + WAL 归档 | 全量每日 + 增量每小时 | 30天 | <1小时 |
| 图存储（RocksDB） | 快照 + CDC 日志 | 全量每日 + CDC 实时 | 30天 | <1分钟 |
| Redis | RDB + AOF | RDB每小时 + AOF实时 | 7天 | <1分钟 |
| 对象存储 | 版本控制 + 跨区域复制 | 实时 | 90天 | 0 |
| 配置数据 | Git 版本控制 | 实时 | 永久 | 0 |

### 6.2 PostgreSQL 备份

```bash
# 全量备份
pg_dump -h localhost -U postgres -F c -f backup_$(date +%Y%m%d).dump dbname

# WAL 归档（实时增量）
# postgresql.conf
archive_mode = on
archive_command = 'test ! -f /backup/wal/%f && cp %p /backup/wal/%f'

# 恢复到指定时间点（PITR）
pg_basebackup -D /var/lib/postgresql/data -X stream
# 重放 WAL 到指定时间点
```

### 6.3 自研图存储备份

利用已有 CDC（变更数据捕获）实现实时备份：

```
图存储主集群
  → CDC 事件流（NATS JetStream）
    → 备份集群（实时重放 CDC 事件）
    → 对象存储（CDC 日志持久化，可重放）

全量快照：
  → RocksDB Checkpoint（一致性快照）
  → 上传到对象存储
```

### 6.4 容灾架构

```
主可用区（AZ-A）          备可用区（AZ-B）
┌──────────────┐          ┌──────────────┐
│ PostgreSQL主  │──复制──→│ PostgreSQL从  │
│ 图存储主集群  │──CDC──→│ 图存储备集群  │
│ Redis主      │──复制──→│ Redis从      │
│ NATS主集群   │──复制──→│ NATS从集群   │
└──────────────┘          └──────────────┘
       │                        │
       └────────┬───────────────┘
                ▼
         对象存储（跨区域复制）
         备份/快照/WAL/CDC日志
```

**RTO/RPO 目标**：
- RPO（恢复点目标）：< 1 分钟
- RTO（恢复时间目标）：< 15 分钟

---

## 七、缓存策略

### 7.1 多级缓存

```
请求 → L1 本地缓存（内存，LruCache，进程内）
     → L2 分布式缓存（Redis，跨进程共享）
     → L3 数据库（PostgreSQL / 图存储）
```

| 缓存层级 | 技术 | 容量 | 延迟 | 适用 |
|----------|------|------|------|------|
| L1 本地缓存 | moka / lru | 小（MB级） | <1ms | 热点配置/字典/不变数据 |
| L2 分布式缓存 | Redis | 大（GB级） | <5ms | 会话/用户信息/热点查询结果 |
| L3 数据库 | PostgreSQL/图存储 | 极大 | 10-100ms | 持久化数据 |

### 7.2 缓存模式

| 模式 | 适用场景 | 实现 |
|------|----------|------|
| **Cache-Aside** | 通用读多写少 | 读：先查缓存，miss 查库并回填；写：先更库，再删缓存 |
| **Write-Through** | 写多读少，强一致 | 写：同时写缓存和库 |
| **Write-Behind** | 写多，可容忍丢失 | 写：先写缓存，异步批量写库 |
| **Refresh-Ahead** | 热点数据，提前刷新 | 缓存即将过期时异步刷新 |

**推荐：Cache-Aside（旁路缓存）**，简单可靠。

### 7.3 缓存问题防护

| 问题 | 防护手段 |
|------|----------|
| **缓存穿透** | 布隆过滤器（Bloom Filter）+ 空值缓存（短TTL） |
| **缓存击穿** | 互斥锁（分布式锁）+ 热点数据永不过期 |
| **缓存雪崩** | TTL 随机化（基础TTL ± 随机偏移）+ 多级缓存 + 熔断降级 |
| **数据不一致** | 先更库再删缓存 + 延迟双删 + 最终一致性 |

### 7.4 缓存 Key 规范

```
格式：{tenant_id}:{service}:{entity}:{id}:{field}

示例：
  tenant-123:auth:user:user-456:profile
  tenant-123:ai:prompt:prompt-789:content
  tenant-123:graph:vertex:vid-abc:neighbors
  system:config:feature-flags
```

---

## 八、数据安全

### 8.1 数据加密

| 层级 | 加密方式 | 算法 |
|------|----------|------|
| **传输加密** | TLS 1.3 | AES-256-GCM |
| **存储加密** | 磁盘加密（LUKS） + 字段加密 | AES-256 / 国密 SM4 |
| **敏感字段** | 应用层加密 + 密钥管理（KMS） | AES-256-GCM / 国密 SM4 |
| **密码** | 加盐哈希 | bcrypt / Argon2id |
| **Token** | 签名 + 可选加密 | JWT (RS256/ES256) |
| **对象存储** | 服务端加密（SSE） | AES-256 / 国密 SM4 |

### 8.2 敏感数据识别与脱敏

| 数据类型 | 脱敏方式 | 示例 |
|----------|----------|------|
| 手机号 | 中间4位星号 | 138****1234 |
| 邮箱 | 用户名部分星号 | u***@example.com |
| 身份证 | 中间10位星号 | 110***********1234 |
| 银行卡 | 中间卡号星号 | 6222 **** **** 1234 |
| 姓名 | 保留姓氏 | 张* |
| 地址 | 保留省市 | 广东省佛山市*** |

脱敏在以下场景自动执行：
- 日志输出（自动脱敏敏感字段）
- API 响应（根据权限返回脱敏/完整数据）
- 数据分析（匿名化处理）

### 8.3 数据访问审计

所有数据访问操作记录审计日志（不可篡改）：

```
审计日志字段：
  - timestamp: 操作时间
  - tenant_id: 租户ID
  - user_id: 操作者ID
  - service: 服务名
  - action: 操作类型（CREATE/READ/UPDATE/DELETE/EXPORT）
  - resource_type: 资源类型
  - resource_id: 资源ID
  - ip_address: 来源IP
  - user_agent: 客户端
  - request_id: 请求ID
  - trace_id: 链路ID
  - result: 操作结果（SUCCESS/FAIL）
  - reason: 失败原因
```

审计日志存储：
- 实时写入 NATS → 审计服务 → 不可篡改存储（追加写 + 哈希链）
- 保留期：≥ 180 天（合规要求）
- 支持按租户/用户/时间/操作类型查询

---

## 九、数据质量

### 9.1 数据质量维度

| 维度 | 说明 | 检查方式 |
|------|------|----------|
| **完整性** | 必填字段非空，关联数据完整 | 非空约束、外键约束、定时检查 |
| **准确性** | 数据值正确，符合业务规则 | 业务规则校验、范围检查、格式校验 |
| **一致性** | 跨服务/跨系统数据一致 | 定期对账、数据校准 |
| **唯一性** | 无重复数据 | 唯一索引、去重检查 |
| **时效性** | 数据及时更新 | 更新时间检查、延迟监控 |
| **合规性** | 符合数据保护法规 | 敏感字段检查、访问审计 |

### 9.2 数据质量监控

- 定时数据质量检查（每日）
- 数据质量仪表盘（Grafana）
- 异常告警（质量分数低于阈值）
- 数据质量报告（月度）

---

## 十、总结

数据架构优化的核心是**"Database per Service + 多租户三档隔离 + 最终一致性 + 自研图存储零修改"**：

1. **Database per Service**：每个服务独立数据库 Schema，服务间只通过 API 访问
2. **多租户三档隔离**：L1 逻辑隔离（tenant_id + RLS，默认）→ L2 Schema 隔离 → L3 集群隔离
3. **自研图存储零修改**：用租户前缀方案（VID = tenant_id:raw_vid），底层 R2StorageServer 完全不改
4. **最终一致性**：跨服务操作用 Saga 模式（事件驱动），单服务内用 ACID 事务
5. **数据迁移**：sqlx-cli 管理迁移版本，扩展-迁移-收缩策略处理破坏性变更
6. **备份容灾**：PostgreSQL WAL + 图存储 CDC + 对象存储跨区域复制，RPO<1min RTO<15min
7. **多级缓存**：本地缓存 + Redis + 数据库，防护穿透/击穿/雪崩
8. **数据安全**：全链路加密 + 敏感字段脱敏 + 不可篡改审计日志

---

*下一篇：[04-部署架构优化](./04-deployment.md)*

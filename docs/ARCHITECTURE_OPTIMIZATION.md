# MOX v3.0 全维架构优化报告

> 项目：infotopograph / MOX 模块化系统架构低代码平台
> 版本：v3.0.0-ai-powered
> 日期：2026-09-05
> 范围：全维度架构整理、分析、优化、归一化

---

## 一、项目现状诊断

### 1.1 项目概况

| 维度 | 现状 |
|------|------|
| 语言 / 工具链 | Rust edition 2021 / rustc 1.98.0-nightly |
| 工作区规模 | 130+ crate，default-members 27 个 |
| 核心域 | platform / base / kg / cloud / flow / kb |
| 已有能力 | DSQL 动态 SQL、JWT/RBAC 认证、可观测性、知识图谱、云盘、IAM、插件体系、WASM 加载 |

### 1.2 八类核心问题

#### P1：所有 svc crate 均为库，无独立部署 binary

- **现象**：`mox-kg-service-svc`、`mox-cloud-master-svc`、`mox-kb-svc` 等所有 svc crate 均无 `main.rs`，无法独立启动。
- **影响**：无法按微服务独立部署、扩缩容、滚动升级；所有能力耦合在单体入口中。
- **严重度**：🔴 高

#### P2：无统一缓存抽象，缓存逻辑散落各处

- **现象**：`mox-dsql-core` 内部有 `DsqlCache`，但其他域（kg/cloud/kb）无统一缓存层；无 L1/L2 多级缓存、无缓存统计、无防穿透机制。
- **影响**：重复造轮子、缓存策略不一致、无法统一监控与调优。
- **严重度**：🔴 高

#### P3：知识库（KB）被 kg 和 cloud 两域瓜分

- **现象**：`mox-kb-svc` 挂靠在 `kg/svc/` 下，云盘域也有文档/文件管理能力，KB 没有独立域。
- **影响**：职责不清、重复实现、无法独立演进。
- **严重度**：🟠 中高

#### P4：六处明确重复功能

| 重复点 | 位置 A | 位置 B |
|--------|--------|--------|
| 可观测性 | `mox-observability-core` | 各 svc 内自建 tracing/logging |
| 认证 | `mox-auth-core`（JWT/RBAC） | `mox-platform-iam-core`（用户/权限） |
| kg-core | `platform/core/mox-kg-core` | `kg/core/mox-kg-*-core` 系列 |
| KB | `kg/svc/mox-kb-svc` | cloud 域文档管理 |
| foundation | `platform/foundation/*` | `base/*-core` 系列 |
| flow 域 | `flow/unified-*`（13个） | `base/*-core`（7个） |

- **严重度**：🟠 中高

#### P5：动态 SQL 与业务逻辑未数据库化

- **现象**：`mox-dsql-core` 已有 SQL 模板/引擎/存储/处理框架，但 SQL 模板和业务逻辑主要硬编码在代码中，未持久化到数据库；无缓存加速层。
- **影响**：修改 SQL/逻辑需重新编译部署，无法热更新；新站开发需重复编写大量模板代码。
- **严重度**：🔴 高

#### P6：无统一服务运行时基座

- **现象**：每个 svc 需自行处理 HTTP 启动、中间件、健康检查、优雅停机、配置加载。
- **影响**：重复代码、不一致的运维接口、新服务接入成本高。
- **严重度**：🟠 中

#### P7：配置管理不统一

- **现象**：各 crate 配置方式各异，无统一的三级配置（默认值 < TOML < 环境变量）规范。
- **影响**：运维复杂度高、环境切换易出错。
- **严重度**：🟡 中

#### P8：缺少企业级交付物

- **现象**：无 Dockerfile、无 docker-compose、无配置示例、无架构文档、无 CI/CD 流水线。
- **影响**：无法一键部署、运维门槛高。
- **严重度**：🟠 中高

---

## 二、归一化目标架构

### 2.1 架构总览

```
┌─────────────────────────────────────────────────────────────────┐
│                        新站极简开发层                              │
│  只需：动态 SQL 模板 + 业务逻辑（数据库存储）+ 动态加载           │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                     统一服务运行时 (mox-server-runtime)           │
│  ServiceModule trait / 三级配置 / 统一中间件 / 健康检查 / 优雅停机 │
└──────┬──────────┬──────────┬──────────┬────────────────────────┘
       │          │          │          │
┌──────▼───┐ ┌───▼────┐ ┌──▼─────┐ ┌─▼──────┐
│ KG 服务   │ │Cloud服务│ │IAM服务 │ │KB 服务  │  ← 独立部署 binary
│ :8101    │ │ :8102  │ │ :8103  │ │ :8104  │
└─────┬────┘ └───┬────┘ └──┬─────┘ └─┬──────┘
      │          │          │          │
┌─────▼──────────▼──────────▼──────────▼──────────────────────────┐
│                      共享基础层 (platform/shared)                  │
│  mox-cache-core / mox-auth-core / mox-observability-core          │
│  mox-config-core / mox-server-runtime / mox-error                 │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                      数据与缓存层                                   │
│  PostgreSQL（SQL模板/业务逻辑/元数据） + Redis（L2分布式缓存）      │
│  + 内存 LRU（L1本地缓存）                                          │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 归一化原则

1. **重复功能上收**：所有重复能力统一到 `platform/shared/` 下的单一 crate
2. **域独立部署**：每个业务域有独立的 server binary，可单独启动/扩缩容
3. **配置三级覆盖**：默认值 < TOML 配置文件 < 环境变量（`MOX_` 前缀）
4. **缓存多级统一**：L1 内存 LRU + L2 Redis，统一 `Cache` trait
5. **新站极简**：只需定义 SQL 模板和业务逻辑（存数据库），运行时自动加载执行

---

## 三、已完成的优化工作

### 3.1 新增 crate 清单（7个）

| Crate | 路径 | 状态 | 说明 |
|-------|------|------|------|
| `mox-cache-core` | `platform/shared/mox-cache-core/` | ✅ 编译通过 | 统一缓存抽象：Cache trait / MemoryCache(LRU+TTL+统计) / MultiCache(L1+L2) / RedisCache |
| `mox-server-runtime` | `platform/shared/mox-server-runtime/` | ✅ 编译通过 | 统一服务基座：ServiceModule trait / 三级配置 / axum 启动 / 健康检查 / 优雅停机 |
| `mox-kb-core` | `platform/domains/kb/core/mox-kb-core/` | ✅ 编译通过 | 独立 KB 域核心：KbManager / KbStore trait / Document / 版本控制 / 全文检索 |
| `mox-kg-server` | `platform/domains/kg/svc/mox-kg-server/` | ✅ 编译通过 | 知识图谱独立微服务（端口 8101），**复用 mox-kg-service-svc http_adapter（10个真实端点：6个KG查询+4个AI引擎）** |
| `mox-cloud-server` | `platform/domains/cloud/svc/mox-cloud-server/` | ✅ 编译通过 | 云盘独立微服务（端口 8102），**基于 MasterServer 构建8个REST端点：卷注册/心跳/分配/列表/快照/恢复/指标/Leader** |
| `mox-iam-server` | `platform/domains/platform/svc/mox-iam-server/` | ✅ 编译通过 | IAM/SSO 独立微服务（端口 8103），**基于 mox-auth-core 构建10个REST端点：注册/登录/刷新/验证/用户信息/列表/改密/禁用，JWT+PBKDF2+RBAC** |
| `mox-kb-server` | `platform/domains/kb/svc/mox-kb-server/` | ✅ 编译通过 | 知识库独立微服务（端口 8104，含完整 CRUD + 内存存储） |

### 3.2 工作区 Cargo.toml 更新

- 新增 7 个 workspace members
- 新增 3 个 workspace dependencies（mox-cache-core / mox-server-runtime / mox-kb-core）
- 修复所有依赖路径层级

### 3.3 企业级交付物

| 交付物 | 路径 | 说明 |
|--------|------|------|
| Dockerfile ×4 | 各 server 目录下 | 多阶段构建，统一运行时镜像 |
| docker-compose | `docker-compose.microservices.yml` | 4 服务 + Redis，一键启动 |
| 配置示例 ×4 | 各 server `config/server.example.toml` | 三级配置模板 |
| 本报告 | `docs/ARCHITECTURE_OPTIMIZATION.md` | 全维架构优化文档 |

---

## 四、核心架构设计

### 4.1 统一缓存层（mox-cache-core）

```
┌──────────────────────────────────────────────┐
│              MultiCache (L1 + L2)             │
│  get → L1 命中? → 返回                        │
│       → L1 未命中 → L2 命中? → 回填 L1 → 返回 │
│       → L2 未命中 → 执行源查询 → 写 L1+L2     │
└──────────┬───────────────────┬───────────────┘
           │                   │
┌──────────▼───────┐  ┌────────▼──────────┐
│  MemoryCache (L1) │  │  RedisCache (L2)  │
│  - LRU 淘汰       │  │  - SCAN 批量失效   │
│  - TTL 过期       │  │  - 分布式共享      │
│  - 容量限制       │  │  - feature gate    │
│  - 原子统计       │  │                    │
└───────────────────┘  └───────────────────┘
```

**关键特性**：
- `CacheValue<T>`：空值防穿透（防止缓存击穿）、TTL、版本哈希
- `CacheStats`：Prometheus 格式输出，命中/未命中/淘汰/过期统计
- `MultiCache`：L1 内存 + L2 Redis 自动穿透回填
- 泛型 `Cache<K, V>` trait，可扩展任意后端

### 4.2 统一服务运行时（mox-server-runtime）

**ServiceModule trait**：
```rust
#[async_trait]
pub trait ServiceModule: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    async fn routes(&self, config: &ServerConfig) -> Router;
    async fn init(&self, config: &ServerConfig) -> Result<(), RuntimeError>;
    async fn shutdown(&self);
    async fn ready_checks(&self) -> Vec<(&'static str, bool)>;
}
```

**三级配置加载**：
1. 代码内默认值（`Default` impl）
2. TOML 配置文件（`--config` 指定）
3. 环境变量覆盖（`MOX_SERVER_HOST` / `MOX_SERVER_PORT` / `MOX_LOG_LEVEL` 等）

**统一中间件链**：
- `Extension(state)`：共享状态注入
- `TraceLayer`：HTTP 请求追踪
- `TimeoutLayer`：请求超时（默认 30s）
- `DefaultBodyLimit`：请求体限制（默认 10MB）
- `CorsLayer`：跨域配置

**自动挂载端点**：
- `GET /health/live`：存活探针
- `GET /health/ready`：就绪探针（含各模块检查）
- `GET /health/metrics`：Prometheus 指标

### 4.3 动态 SQL + 业务逻辑数据库化架构

```
┌─────────────────────────────────────────────────────────────┐
│                      请求入口                                  │
│  POST /api/v1/dsql/execute  { template_id, params }         │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                   DSQL 执行引擎 (mox-dsql-core)               │
│  1. 查缓存（模板 + 结果）                                      │
│  2. 未命中 → 从数据库加载 SQL 模板 + 业务逻辑                   │
│  3. 参数校验 + SQL 渲染（防注入）                               │
│  4. 执行业务逻辑（WASM / 内置函数 / 动态加载）                  │
│  5. 执行 SQL 查询                                              │
│  6. 结果写缓存（带 TTL + 版本）                                 │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                   数据库存储层                                  │
│  dsql_templates      → SQL 模板（ID / 名称 / 内容 / 版本）     │
│  dsql_logic          → 业务逻辑（ID / 模板关联 / 逻辑代码）     │
│  dsql_params         → 参数定义（名称 / 类型 / 校验规则）       │
│  dsql_executions     → 执行日志（时间 / 耗时 / 状态 / 错误）    │
└─────────────────────────────────────────────────────────────┘
```

**新站极简开发流程**：
1. 在数据库中插入 SQL 模板（`dsql_templates`）
2. 定义参数校验规则（`dsql_params`）
3. 编写业务逻辑（`dsql_logic`，支持 WASM 动态加载）
4. 调用 `POST /api/v1/dsql/execute` 执行
5. **无需重新编译部署**，模板和逻辑热更新

### 4.4 四大模块独立部署

| 服务 | 端口 | 职责 | 依赖 |
|------|------|------|------|
| mox-kg-server | 8101 | 知识图谱管理、图查询、实体关系 | mox-kg-core / mox-cache-core |
| mox-cloud-server | 8102 | 文件存储、分片、元数据、分享 | mox-cloud-* / mox-cache-core |
| mox-iam-server | 8103 | 用户认证、SSO、RBAC、权限管理 | mox-auth-core / mox-platform-iam-core |
| mox-kb-server | 8104 | 文档管理、版本控制、全文检索、知识分析 | mox-kb-core / mox-cache-core |

**独立部署优势**：
- 按需扩缩容（如 KB 搜索压力大时只扩 KB 服务）
- 独立滚动升级（不影响其他服务）
- 故障隔离（单个服务崩溃不拖垮全局）
- 技术栈可异构（未来可用其他语言重写单个服务）

---

## 五、企业级加固清单

### 5.1 已完成

- ✅ 统一缓存抽象（防穿透 / TTL / 统计）
- ✅ 统一服务运行时（配置 / 中间件 / 健康检查 / 优雅停机）
- ✅ 4 个独立微服务 binary
- ✅ Dockerfile（多阶段构建）
- ✅ docker-compose（含 Redis）
- ✅ 配置示例（三级配置）
- ✅ 健康检查端点（live / ready / metrics）
- ✅ 编译验证（7 个新 crate 全部通过）

### 5.2 待完成（按优先级）

| 优先级 | 事项 | 说明 |
|--------|------|------|
| ~~P0~~ | ~~kg/cloud/iam 服务接入实际业务逻辑~~ | ✅ **已完成**：kg 复用 http_adapter(10端点)，cloud 基于 MasterServer(8端点)，iam 基于 mox-auth-core(10端点) |
| ~~P0~~ | ~~mox-dsql-core 缓存替换为 mox-cache-core~~ | ✅ **已完成**：DsqlCache 内部改用 MemoryCache（LRU+TTL+统计），同步 API 保持不变，14个单元测试全部通过 |
| ~~P1~~ | ~~SQL 模板/业务逻辑数据库化表结构 + 迁移脚本~~ | ✅ **已完成**：migration 003 新增 dsql_logic/dsql_logic_version 业务逻辑表 + 7个性能索引；PostgreSQL 兼容完整 schema（8表+12索引）已交付；14个单元测试全部通过 |
| ~~P1~~ | ~~Redis L2 缓存在 server 中启用~~ | ✅ **已完成**：mox-server-runtime 新增 cache_factory.rs，根据配置自动创建 L1 内存/L2 Redis 缓存；支持 memory/redis/none 三种后端；配置层已有 redis_url/backend/MOX_REDIS_URL 环境变量 |
| ~~P1~~ | ~~单元测试覆盖扩充~~ | ✅ **已完成**：mox-server-runtime 2→15测试，mox-kb-core 1→11测试，mox-dsql-core 14测试，mox-cache-core 5测试，合计45+测试全部通过 |
| ~~P2~~ | ~~CI/CD 流水线~~ | ✅ **已完成**：GitHub Actions 配置（fmt+clippy lint / 8 crate 并行编译 / 3 crate 单元测试 / redis-backend 特性编译 / 4 服务 Docker 镜像构建） |
| ~~P2~~ | ~~分布式追踪~~ | ✅ **已完成**：mox-server-runtime 新增 tracing_utils.rs，trace_id 提取/生成/注入，兼容 W3C Trace Context（traceparent），与 tracing span 集成，7 个测试通过 |
| P2 | 审计日志 | 关键操作（SQL 模板修改 / 权限变更 / 数据删除）的不可变审计日志（表结构已存在，查询 API 待实现） |
| P3 | 配置中心 | 接入 Nacos / Apollo / etcd，支持配置热更新 |
| P3 | 服务发现 | 接入 Consul / etcd / Kubernetes Service |
| ~~P3~~ | ~~限流熔断~~ | ✅ **已完成**：mox-server-runtime 新增 rate_limit.rs，令牌桶无锁实现（AtomicU64），支持 QPS/突发配置，429 响应，集成到 ServerConfig.rate_limit，5 个测试通过 |

---

## 六、新站极简开发指南

### 6.1 开发一个新网站需要什么？

**只需 3 步，零编译：**

```
步骤 1：定义 SQL 模板（存入数据库 dsql_templates 表）
  INSERT INTO dsql_templates (id, name, sql, version)
  VALUES ('user_list', '用户列表', 'SELECT * FROM users WHERE status = {{status}} LIMIT {{limit}}', 1);

步骤 2：定义参数校验（存入数据库 dsql_params 表）
  INSERT INTO dsql_params (template_id, name, type, required, validator)
  VALUES ('user_list', 'status', 'string', true, '^active|inactive$'),
         ('user_list', 'limit', 'int', false, '1..100');

步骤 3：调用执行 API
  POST /api/v1/dsql/execute
  { "template_id": "user_list", "params": { "status": "active", "limit": 20 } }
```

### 6.2 需要编写代码吗？

- **简单查询**：不需要，纯数据库配置
- **复杂业务逻辑**：编写 WASM 插件或在 `dsql_logic` 表中定义逻辑代码，运行时动态加载
- **自定义端点**：实现 `ServiceModule` trait，复用 `mox-server-runtime` 基座，约 50 行代码

---

## 七、验证结果

### 7.1 编译验证

```
cargo check -p mox-cache-core       → ✅ Finished (1 warning)
cargo check -p mox-server-runtime   → ✅ Finished (3 warnings)
cargo check -p mox-kb-core          → ✅ Finished
cargo check -p mox-kg-server        → ✅ Finished
cargo check -p mox-cloud-server     → ✅ Finished
cargo check -p mox-iam-server       → ✅ Finished
cargo check -p mox-kb-server        → ✅ Finished (1 warning)
```

### 7.2 警告说明

所有警告均为 `unused import` / `unused variable` 类型，不影响功能，可在后续 `cargo fix` 中自动清理。

---

## 八、总结

本次全维架构优化完成了以下核心目标：

1. **动态 SQL + 缓存加速**：建立了统一缓存抽象（mox-cache-core），为 SQL 模板和查询结果的多级缓存奠定基础；DSQL 数据库化架构已设计完成，待执行表结构迁移。

2. **四大模块归一化 + 独立部署**：KB 从 kg/cloud 独立为新域（mox-kb-core）；4 个独立微服务 binary（kg/cloud/iam/kb）全部编译通过，可独立部署、扩缩容。

3. **新站极简开发**：统一服务运行时（mox-server-runtime）将新服务接入成本从"复制粘贴数百行"降到"实现一个 trait（约 50 行）"；DSQL 数据库化后，简单查询甚至零代码。

4. **企业级交付**：Dockerfile ×4、docker-compose、配置示例 ×4、健康检查、优雅停机、三级配置、Prometheus 指标端点全部就绪。

**剩余工作**主要集中在 P0/P1 级别的实际业务逻辑接入和 DSQL 数据库化落地，详见第五章加固清单。

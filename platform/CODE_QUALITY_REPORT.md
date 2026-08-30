# MOX 平台基础层 & 网关层 & 架构测试 代码质量评测报告

> 评测范围：`platform/foundation/`、`platform/gateway/`、`platform/arch-test/`
> 评测日期：2026-08-30
> 指标维度：实现状态 / 代码量 / 测试覆盖 / 文档 / 质量评级

---

## 一、Crate 总览（共 7 个）

| # | Crate 名称 | 所属层 | 源文件数 | 总行数 | 代码行数（约） | 实现状态 | 单元/集成测试数 | README | 质量评级 |
|---|-----------|--------|---------|--------|--------------|---------|---------------|--------|---------|
| 1 | `mox-error` | L0 Foundation | 1 | 534 | ~336 | **完整实现** | 6（inline） | 有 | A |
| 2 | `mox-platform-paths` | L0 Foundation | 1 | 424 | ~286 | **完整实现** | 8（inline） | 无 | A- |
| 3 | `mox-platform-observability` | L0 Foundation | 5 | 634 | ~467 | **实质实现** | 0 | 无 | B+ |
| 4 | `mox-cloud-foundation` | L0 Foundation | 13 | 3,670 | ~3,325 | **实质实现** | 51（50 集成 + 1 单元） | 有 | A- |
| 5 | `mox-platform-foundation` | L0 Foundation | 1 | 164 | ~153 | **骨架/数据定义** | 2（集成） | 有 | B |
| 6 | `mox-platform-gateway-svc` | L1 Gateway | 8 | 1,174 | ~842 | **实质实现（含占位）** | 0 | 无 | B- |
| 7 | `mox-arch-test` | Arch Test | 1 | 428 | ~340 | **完整实现** | 8（inline） | 无 | A- |

**代码量总计**：约 5,749 行代码 / 7,028 总行
**测试总数**：约 75 个测试用例

---

## 二、各 Crate 详细评测

### 2.1 mox-error — 全局错误码系统

**路径**：`platform/foundation/mox-error/`

**实现状态**：完整实现

**核心能力**：
- 4 级错误严重等级：Info / Warning / Error / Critical
- 10 个业务域代码：KG / AI / FL / OP / PJ / RS / US / PL / DT / CL
- `MoxError` 结构体：错误码、消息、详情、等级、HTTP 状态码、trace_id、时间戳、错误链
- `define_domain_errors!` 宏：快速定义各域错误码常量
- 预置 4 个域的错误码：KG（存储/算法/元数据 3 模块）、AI（对话/LLM/Agent 3 模块）、User（认证/权限 2 模块）、Platform（通用/配置/验证 3 模块）
- Axum `IntoResponse` 集成（feature-gated，默认启用）：统一 JSON 错误响应 + 自动日志

**测试**：6 个 inline 单元测试（错误码格式、宏、Display、detail、域前缀唯一性等）

**质量亮点**：
- 设计规范，注释详尽，中文文档清晰
- 错误链支持（`source`）符合 Rust 最佳实践
- 宏设计优雅，便于扩展新域

---

### 2.2 mox-platform-paths — 统一路径管理

**路径**：`platform/foundation/mox-platform-paths/`

**实现状态**：完整实现

**核心能力**：
- `ProjectRoot::detect()` 自动检测项目根目录（向上查找 `platform/` + `Cargo.toml`）
- 支持 `MOX_ROOT` / `MOX_DATA_DIR` / `MOX_CONFIG_DIR` / `MOX_PLUGINS_DIR` / `MOX_THIRD_PARTY_DIR` / `MOX_RUNTIME_DIR` 环境变量覆盖
- 5 大类路径：架构代码（platform/config/shared/docs/frontend）、运行时数据（storage/cache/logs/uploads/exports）、插件（wasm/scripts/extensions）、第三方（models）、运行时状态（pid/socket/lock）
- `PathConfig` 结构体：支持从环境变量加载、应用到环境变量
- `verify_separation()` 架构-数据分离不变量验证
- `ensure_all_dirs()` 启动时目录自动创建

**测试**：8 个 inline 单元测试（根目录检测、各类路径验证、分离不变量、配置默认值等）

**质量亮点**：
- 严格的架构-数据分离原则贯彻到底
- 路径命名规范，环境变量覆盖机制完善
- 缺少 README 文档

---

### 2.3 mox-platform-observability — 可观测性基础

**路径**：`platform/foundation/mox-platform-observability/`

**实现状态**：实质实现

**模块组成**（5 个文件）：
- `logging.rs`：结构化日志（JSON / Pretty / Compact 三种格式），基于 `tracing-subscriber`
- `metrics.rs`：Prometheus 指标注册中心，含标准 HTTP 指标（请求数/延迟直方图/错误数/在途请求数）和可扩展 ServiceMetrics
- `tracing_ctx.rs`：分布式追踪上下文（trace_id / span_id 传播）
- `middleware.rs`：Axum 可观测性中间件层
- `lib.rs`：统一 `init()` 入口 + 全局指标单例

**测试**：0 个测试（无 inline 单元测试，无 tests/ 目录）

**质量问题**：
- **零测试覆盖** — 作为基础层 crate 缺乏测试保障
- 缺少 README
- middleware 和 tracing_ctx 的实现深度需进一步验证

---

### 2.4 mox-cloud-foundation — L5 领域抽象层

**路径**：`platform/foundation/mox-cloud-foundation/`

**实现状态**：实质实现（体量最大的基础层 crate）

**模块组成**（12 个模块，10 个核心 trait）：

**Cloud Drive 5 个 trait**：
| 模块 | Trait | Mock 实现 |
|------|-------|-----------|
| `object_storage.rs` | `ObjectStorageProvider` | BTreeMap 内存实现（put/get/delete/list/head/multipart） |
| `meta_storage.rs` | `MetaStorageProvider` | 内存文件系统（mkdir/rmdir/rename/symlink/stat/xattr/chmod/chown/statfs） |
| `chunk_manager.rs` | `ChunkManagerProvider` | 内存分块管理（alloc/write/read/delete/rebuild/stats/gc） |
| `iam.rs` | `IamProvider` | 内存 IAM（用户 CRUD/认证/策略授权/角色/STS） |
| `quota.rs` | `QuotaProvider` | 内存配额（用户配额/目录配额/写入校验/列表） |

**Graph 5 个 trait**：
| 模块 | Trait | Mock 实现 |
|------|-------|-----------|
| `graph_query.rs` | `GraphQueryProvider` | 空实现（vertex/edge CRUD/neighbors/k-hop/subgraph/cypher/ngql） |
| `graph_meta.rs` | `GraphMetaProvider` | 内存元数据（space/tag/edge-type CRUD + hosts） |
| `graph_algo_single.rs` | `GraphAlgoSingleProvider` | 空结果实现（PPR/CNM/Betweenness/Harmonic/Density/BDE） |
| `partition_router.rs` | `PartitionRouterProvider` | 8 分片固定路由（vid→shard/host 映射/rebalance） |
| `cdc_publisher.rs` | `CdcPublisherProvider` | 内存事件总线（vertex/edge 事件 + subscribe + offset + lag） |

**附加能力**：
- `iam_standard_policies.rs`：10 条标准 IAM Policy（P1-P10）+ Deny 优先 evaluate 引擎
- `sts_ttl900.rs`：STS 临时凭证服务（TTL 900 秒，HMAC 签名）

**测试**：
- 集成测试：`tests/t1_t2_t3_red_green.rs` — 50 个测试用例（Cloud Drive 25 + Graph 25），TDD RED-GREEN 模式
- 单元测试：lib.rs 中 1 个编译测试
- **测试质量高**：所有 Mock 均通过对应测试验证

**质量亮点**：
- TDD 开发模式，测试覆盖率高
- Mock 实现完整，可用于上层单元测试
- IAM 策略引擎设计专业（Deny 优先、条件键支持）
- 架构清晰：trait 定义 + Mock 实现 + 集成测试

**不足**：
- Graph Query / Graph Algo 的 Mock 基本返回空结果，行为模拟较浅
- 部分 error 类型使用 `Box<dyn Error>`，不够类型安全

---

### 2.5 mox-platform-foundation — Crate 元数据

**路径**：`platform/foundation/mox-platform-foundation/`

**实现状态**：骨架 / 数据定义

**核心内容**：
- `AisLayer` 枚举：L2Gateway ~ L7Infrastructure 共 7 层
- `CrateMeta` 结构体：id / name / version / layer / owner
- `all_crate_metas()`：16 个 crate 的元数据清单（含 UUID v5 标识符）

**测试**：
- `tests/crate_id_unique.rs`：2 个测试（ID 唯一性 + UUID 格式校验）
- `tests/lookup.rs`：lookup 相关测试

**质量评价**：
- 纯数据定义型 crate，无业务逻辑
- 作为"crate 注册表"用途明确
- README 存在但内容需确认

---

### 2.6 mox-platform-gateway-svc — L1 企业级网关

**路径**：`platform/gateway/mox-platform-gateway-svc/`

**实现状态**：实质实现（含大量占位模块）

**模块分析**（8 个文件）：

| 模块 | 状态 | 说明 |
|------|------|------|
| `main.rs` | 完整 | CLI 参数解析 + tokio runtime + serve_forever 调用 |
| `lib.rs` | 实质 | `build_gateway_router()` 构建 12 端点 + `serve_forever()` 优雅退出 |
| `routes.rs` | 骨架/未集成 | 31 域路由描述符矩阵，但未在主 router 中使用 |
| `routing.rs` | 骨架/未集成 | 自研 Router 结构（与 axum Router 重复），未集成 |
| `auth.rs` | 实质/未集成 | JWT + API Key 中间件实现完整，但未接入主 router |
| `rate_limit.rs` | 实质/未集成 | Token Bucket 限流实现完整，但未接入主 router |
| `config.rs` | 完整 | AuthConfig / RateLimitConfig / RoutingConfig 配置结构 |
| `o11y.rs` | **纯占位** | 明确标注"占位符"，MetricsCollector 方法为空实现 |

**实际运行端点**（12 个）：
- L0 通用：`/health`、`/api/v1/status`
- L2 KG：`/kg/v1/neighborhood`、`/kg/v1/path`、`/kg/v1/shortest-path`、`/kg/v1/centrality`、`/kg/v1/communities`、`/kg/v1/stats`
- L3 AI：`/ai/engine/process`、`/ai/engine/analyze`、`/ai/engine/capabilities`、`/ai/engine/metrics`

**测试**：0 个测试（无 tests/ 目录，无 inline 测试）

**质量问题**：
1. **模块碎片化严重** — 8 个源文件中，有 5 个（routes/routing/auth/rate_limit/o11y）未实际集成到主路由
2. **o11y.rs 明确为占位实现** — 与 `mox-platform-observability` 功能重叠，应统一
3. **零测试覆盖** — 网关作为核心入口，缺乏测试保障
4. **路由架构不清晰** — 同时存在 axum Router（主路径）和自研 Router（routing.rs）两套体系
5. **缺少 README**
6. **28 个域为 stub 状态** — 迁移进度约 12/31 ≈ 39%

---

### 2.7 mox-arch-test — 架构合规测试

**路径**：`platform/arch-test/`

**实现状态**：完整实现

**8 项架构约束测试**：
| 测试函数 | 验证内容 |
|---------|---------|
| `test_layering_rules` | L0~L5 分层依赖规则（如 L0 不能依赖任何域、L1 只能依赖 L0/L2 等） |
| `test_cross_domain_dependencies_go_through_api` | 跨域依赖必须经过 L2 api 层 |
| `test_no_circular_dependencies` | DFS 环检测，禁止循环依赖 |
| `test_api_crates_are_pure` | L2 api crate 只能依赖 L0 foundation |
| `test_architecture_data_separation` | platform/ 目录下无运行时数据文件（.db/.log 等） |
| `test_no_hardcoded_data_paths` | 代码中无硬编码 `./data/`、`./config/` 等相对路径 |
| `test_plugins_outside_platform` | 插件文件（.wasm/.so 等）不在 platform/ 内 |
| `test_third_party_outside_platform` | third_party/vendor 目录不在 platform/ 内 |

**技术实现**：
- 自动扫描 workspace 中所有 `mox-*` crate 及其依赖
- 通过路径分类判定层级（foundation→L0, gateway→L1, api→L2, core→L3, svc→L4, sdk→L5）
- TOML 解析提取依赖关系
- 支持排除列表（过渡期豁免 crate）

**质量亮点**：
- 设计精良的架构守护工具
- 覆盖分层、跨域、循环、纯度、数据分离 5 大维度
- 与 `mox-platform-paths` 的分离原则形成"代码 + 测试"双重保障

**不足**：
- 缺少 README 说明如何运行和扩展
- crate 层级判定基于路径约定，缺乏显式元数据校验

---

## 三、总体问题清单

### P0 — 关键问题

1. **网关层模块碎片化严重**：`mox-platform-gateway-svc` 有 8 个源文件，但实际运行只使用了 lib.rs + main.rs 中的最小功能。auth、rate_limit、routes、routing、o11y 共 5 个模块处于"写了但未接入"状态，形成技术债。

2. **网关层零测试覆盖**：作为系统入口的网关没有任何测试，健康检查、路由、错误处理等均无自动化验证。

3. **可观测性 crate 零测试**：`mox-platform-observability` 作为所有 svc 层依赖的基础库，没有任何测试。

### P1 — 重要问题

4. **o11y 模块重复与占位**：网关层的 `o11y.rs` 是明确的占位实现，而基础层已有功能更完整的 `mox-platform-observability`。应统一使用基础层实现，删除网关层占位模块。

5. **网关路由体系不统一**：同时存在 axum Router（实际运行）和自研 `routing::Router`（未使用）两套路由体系，架构意图不清晰。

6. **Graph Mock 实现较浅**：`mox-cloud-foundation` 中 Graph Query 和 Graph Algo 的 Mock 基本返回空结果，无法支撑复杂的上层单元测试场景。

7. **README 覆盖率低**：7 个 crate 中仅 3 个有 README（mox-error、mox-cloud-foundation、mox-platform-foundation），缺 4 个。

### P2 — 改进建议

8. **错误类型可更类型安全**：`mox-cloud-foundation` 中 trait 方法返回 `Box<dyn Error>`，建议定义统一的错误枚举或使用 `thiserror`。

9. **架构测试可扩展**：`mox-arch-test` 目前依赖路径分类，可结合 `mox-platform-foundation` 的 `CrateMeta` 做更精确的层级判定。

10. **mox-platform-foundation 职责单一**：目前仅存 crate 元数据，可考虑更名为 `mox-crate-meta` 或扩展更多平台级公共类型。

11. **测试组织待规范化**：部分 crate 用 inline tests（mox-error、mox-platform-paths），部分用 tests/ 目录（mox-cloud-foundation、mox-platform-foundation），风格不统一。

---

## 四、质量总评

| 维度 | 评分（10 分制） | 说明 |
|------|----------------|------|
| 架构设计 | 8.5 | 分层清晰，trait 抽象规范，架构测试守护到位 |
| 代码质量 | 8.0 | 命名规范、注释详尽、Rust 惯用写法；但部分模块有技术债 |
| 测试覆盖 | 6.0 | mox-cloud-foundation 测试充分（50 个），但网关层和可观测性为零 |
| 文档完备度 | 5.5 | README 覆盖率不足一半，代码内注释质量较高 |
| 功能完备度 | 6.5 | 基础层较完整，网关层仅 ~39% 迁移完成 |
| **综合评分** | **6.9** | 基础扎实，架构清晰，但网关层和测试覆盖需加强 |

---

## 五、优先行动建议

1. **清理网关层死代码**：评估 auth / rate_limit / routes / routing / o11y 五个模块的迁移计划，要么接入主路由，要么标记为 deprecated 或移入独立分支
2. **补充网关测试**：至少添加健康检查、路由注册、CORS 等基础集成测试
3. **统一可观测性实现**：网关层使用 `mox-platform-observability`，删除占位的 `o11y.rs`
4. **补齐 README**：为 mox-platform-paths、mox-platform-observability、mox-platform-gateway-svc、mox-arch-test 添加 README
5. **加强 observability 测试**：为日志初始化、指标注册、中间件行为添加单元测试

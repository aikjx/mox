# 规格：璇玑 mox v4 · 需求-架构-业务处理 全维度企业级闭环架构与全功能开发

> 规格语言：中文
> 治理中枢：璇玑归一化知识图谱（需求 → 架构 → 业务流程 → 模块 → 文档 → 代码 六层归一化双向绑定）
> 技术底座约束：后端 Rust 全维自研（Rust Gateway + 16 crate Services + Node EAF 编排），图谱唯一真相源，AIS DIP 依赖反转分层
> 前置交付基线（已通过独立 Review A+）：SPEC-1 Storage 双写回源 + SPEC-2 FileStore EC/MPU + SPEC-3 Nebula L1/CDC + SPEC-4 CNM/RAW/精度/LPA禁用 + SPEC-5 PR转置/激活扩散 + SPEC-6 Rust Gateway ai_engine + SPEC-7 internal+rerank+CEM + SPEC-8 协议兼容 + SPEC-10 三流程 Trace E2E + SPEC-13 SLO/容量/TCO + SPEC-14 故障注入 HA + SPEC-15 129 GREEN 全回归 + 产品手册 v3

---

## 1. 问题与目标（Specify）

### 1.1 客观问题（来自前序 SPEC 验收 + 图谱 6 层缺口扫描）

| 类别 | ID | 事实陈述 | 量化证据 |
|---|---|---|---|
| **Rust AIS 架构缺口** | R-01 | 16 crate 0 在璇玑图谱注册，三注册表 business/engine/algorithm-registry.js 全部指向 backend-node `src/*.js`，Rust 端完全断链 | 0/16 Rust crate 绑定（见 SPEC-Rust A-01~B-05） |
| Rust AIS 依赖污染 | R-02 | L6 `operator-core/types.rs` 直接依赖 7 个外部 crate（serde/nalgebra/ndarray/thiserror/anyhow/tracing/uuid）违反 L6 std-only | 7 外部依赖越界 |
| Rust AIS 同级硬耦合 | R-03 | `hermes-flow-bridge` / `business-catalog` 直接 `use mox_expert::*` 具体 struct；`mox-system/orchestrator.rs` 反向依赖 services 实现 | 3 处 DIP 违反 |
| 核心算法单源失效 | R-04 | tech-registry.js 6 条算法声明 `singleSource=true`，但 Rust `graph-algorithms/src/lib.rs` 存在 6 条完全相同独立实现，registry 未登记 Rust | 双实现漂移 6/6 |
| 框架依赖扩散 | R-05 | `rusqlite` 本应仅 Infra（mox-system），实际扩散至 L3 `ai-agent` 和 L4 `primiflow-core` Cargo.toml | 2 crate 违规扩散 |
| **业务流程缺口** | F-01 | 三流程（graph_bulk / file_upload / ai_rag）端点已 E2E GREEN，但缺少「业务全链路编排」统一入口：用户请求 → 图谱意图→流程选择→多步串行/并行→产物回谱→审计 | 0 条 `/ai/engine/workflow/*` 端点 |
| 业务流程缺口 | F-02 | 无企业级合规端点：`GET /atlas/verify`（对 6 层实体跨域对账）、`POST /atlas/governance/audit`（审计导出）、`GET /atlas/health/enterprise`（SLO/RPO/RTO 实时指标） | 0 条合规端点 |
| 业务流程缺口 | F-03 | 无「业务流程节点-图谱实体-代码文件」显式边：三流程运行的 step 在图谱中无独立 workflow_step 节点；无法追溯「哪个业务流程步调用哪个 Rust crate 哪个函数」 | workflow_step 节点数=0 |
| **性能/企业级缺口** | E-01 | `boundary_ultra_deep_chain_with_data_deps` 500 深链 P99=10,594ms 超预算 ≤10,000ms | 1 测试失败（SPEC-Rust G-01 / P-01） |
| 文档-代码不一致 | E-02 | 架构文档 `02-architecture.md` 仅描述 mox-system 单 crate 5 层；16 crate README 覆盖率=2/16=12.5% | 14 crate README 缺失 |
| 版本治理漂移 | E-03 | primiflow-core dev-deps reqwest=0.11（workspace=0.12）；mox-expert/hermes/business-catalog 3 crate 未 workspace=true 继承 8+ 依赖 | 3 类漂移 |

### 1.2 目标用户 & 关注价值

| 用户层 | 关键价值 |
|---|---|
| **企业决策者（CTO/CIO/采购）** | 一张图=全公司 IT 资产；RPO=0 / RTO<60s；TCO 降 42%；等保合规审计一键导出 |
| **架构/工程委员会** | AIS DIP 全合规；L6 kernel std-only；16 crate 可插拔可替换；三流程 100% 可追溯 |
| **开发专家联盟** | 16 crate 独立小项目 + README + 统一注册常量 + 图谱双向绑定；每次 PR 自动图谱反向同步 |
| **算法联盟** | 7 核心算法 singleSource=true；Rust/Node 对账 Δ≤1e-6；CEM 多目标 4 加权；500 深链≤10s |
| **SRE/运维联盟** | `/atlas/health/enterprise` 一屏 SLO；16 crate 独立版本；故障注入 HA 一键重演；灰度蓝绿自动路由 |
| **普通开源开发者** | 架构清晰 + 依赖统一 + 精准测试指引；不改动 L6 kernel/精度护栏/路由语义即可稳定二次开发 |

### 1.3 非目标（严格不做）

- ❌ **不新建 JavaScript 业务逻辑模块**：所有 Node 新增代码仅限端点壳/编排层/路由映射，业务算法全进 Rust crate
- ❌ **不引入新数据库/新中间件**：存储仍为 Postgres+Citus / NebulaGraph / MinIO / Redis 四件套（SPEC-1 基线）
- ❌ **不修改算法精度护栏**：CNM / PageRank d=0.85 maxIter=30 / Brandes / harmonic / RAW 双向展开 / 禁用 toFixed —— 红线锁死
- ❌ **不破坏 Router AC-10 语义**：静态优先 / 参数少优先 / 同参数长路径优先 —— 红线锁死
- ❌ **不新增第 17+ Rust crate**：在现有 16 crate + gateway 内落地；复用 crate 不另立项目
- ❌ **不新增前端 UI 组件**：端点 / README / 架构文档 / 测试为交付形态；UI 走后续独立 Spec

### 1.4 目标（Must / Should / Nice-to-have）

**Must（验收 gate，不通过 = fail）**：
1. 6 层归一化图谱：Rust 16 crate = 16 节点，绑定 demand/architecture/business_process/module/document/code 六层边，GET /atlas/verify 全绿
2. AIS DIP 合规：L6 operator-core kernel 100% std-only；3 处同级硬耦合全部 trait 反转；rusqlite 仅存在于 mox-system
3. 核心算法 singleSource=true：tech-registry.js 6 条显式声明 Rust 为主实现；Rust/Node 对账 Δ≤1e-6；LPA 对外出口完全禁用
4. 业务流程三端统一：`/ai/engine/workflow/execute` 统一入口，graph_bulk/file_upload/ai_rag 三流程 = 3 个 workflow 模板；每个 step 生成图谱 workflow_step 节点 + traceId 关联
5. 500 深链 ≤ 10,000 ms（拓扑剪枝 + memo 缓存）
6. 企业级 3 端点：`GET /atlas/verify`、`GET /atlas/health/enterprise`、`POST /atlas/governance/audit`
7. 全量回归 GREEN：cargo build/test/clippy（16 crate）+ Node 12 套 + Rust gateway 3 套 = 全部 GREEN，总数 ≥ 129（SPEC-15 基线）

**Should（质量提升）**：
8. 16 crate README 覆盖率 = 16/16；统一 CRATE_ID / CRATE_META / ENGINE_NAME 常量
9. 依赖治理 100% workspace=true 继承；无写死版本号；无 dev-deps 漂移
10. 架构文档三方对账：`02-architecture.md` + project-atlas.md + 三注册表 + 16 crate 四源一致

**Nice-to-have**：
11. R-5 指标 CEM 加权分（Q=0.55/S=0.2/T=0.1/Stability=0.15）≥ 0.82

---

## 2. 需求-架构-业务处理流程 全维设计（§二 核心交付）

### 2.1 架构分层（Rust AIS L6 分层对齐 DIP）

```
┌───────────────────────────────────────────────────────────────┐
│ L1 EDGE / CLIENT  SDK（企业控制台/第三方系统）                │
├───────────────────────────────────────────────────────────────┤
│ L2 GATEWAY（platform/gateway/runtime）                        │
│   Axum 路由  ·  ai_router AC-10  ·  Sidecar（Node 降级）       │
│   /ai/engine/*  ·  /atlas/*  ·  /graph/*  ·  /files/*         │
├───────────────────────────────────────────────────────────────┤
│ L3 ORCHESTRATION（platform/backend-node EAF）                 │
│   workflow 模板引擎 · 意图识别 rerank · 多目标 CEM 寻优         │
│   internal/intent · internal/graph-algo · DualWriteStorage    │
├───────────────────────────────────────────────────────────────┤
│ L4 APPLICATION SERVICES（platform/services/ 16 crate）        │
│   L4.1 mox-expert     治理专家引擎（GovernContext trait）   │
│   L4.2 primiflow-core    流程内核（数据流/控制流）             │
│   L4.3 business-catalog  业务目录（抽象 CatalogProvider）      │
│   L4.4 hermes-flow-bridge 跨引擎桥接（抽象 HermesBridge trait）│
│   L4.5 ai-agent          智能体（Agent trait, 禁止 rusqlite）  │
│   L4.6 graph-algorithms  7 核心算法（主实现 singleSource=true）│
│   L4.7 flow-ai / primiflow-fusion / telemetry 等 10 个         │
├───────────────────────────────────────────────────────────────┤
│ L5 DOMAIN ABSTRACTIONS（每个 crate src/domain/ pub trait）     │
│   MemberProvider / TaskProvider / PermissionProvider          │
│   CatalogProvider / HermesBridge / PersistenceProvider        │
│   AgentProvider / AlgorithmProvider / WorkflowStepProvider     │
├───────────────────────────────────────────────────────────────┤
│ L6 KERNEL（operator-core/src/kernel/ mod.rs ONLY std）         │
│   kernel_ext/（wrapper impl serde / nalgebra 等）              │
│   0 外部 crate —— AIS 红线                                    │
├───────────────────────────────────────────────────────────────┤
│ L7 INFRASTRUCTURE（mox-system ONLY impl rusqlite / remote ）│
│   PostgresProvider  ·  NebulaAdapter  ·  S3ChunkBackend       │
│   RemoteGraphDriver · CDC event bus · L1 cache                │
└───────────────────────────────────────────────────────────────┘
```

**架构硬约束（违反 = AC fail）**：
A. L6 kernel 仅 `std`、`core`、`alloc`；外部 crate 全部在 `kernel_ext/` wrapper 层
B. 所有跨 crate 依赖走 L5 trait（DIP）；禁止同级 concrete struct 直接 use
C. `rusqlite`、`tokio-postgres`、`rusoto_s3` 等 IO crate 只出现在 L7 mox-system impl（禁止 4/5/6 层出现）
D. 7 核心算法主实现 = graph-algorithms crate；Node `GraphFormulas` 仅作 FFI 调用 + 薄兼容层（不得另写算法核心循环）

### 2.2 六层归一化知识图谱数据模型（唯一真相源）

```
Entity 类型（已存在 + 新增 3 类）
  demand           需求节点（PRD/SPEC/Issue）
  architecture     架构节点（分层/模块边界）
  business_process 业务流程节点（graph_bulk / file_upload / ai_rag）
  module           模块节点（16 Rust crate + backend-node 模块）
  document         文档节点（.md / README / 02-architecture.md）
  code             代码节点（.rs / .js 函数 + 文件）
  workflow_step   *新增* 三流程运行 step
  audit_event     *新增* 合规审计事件
  slo_snapshot    *新增* SLO 实时快照

Edge 类型（核心）
  implements       module → code          模块由代码实现
  documents        document → module/code 文档描述
  satisfies        code → demand          代码满足需求
  orchestrates     business_process → module/workflow_step 编排
  runs_on          workflow_step → code   step 执行代码函数
  audits           audit_event → 6 层任意 审计关联
  snapshots        slo_snapshot → code/module/workflow_step
```

**图谱反向同步铁律**：
> 任何需求变更 → 任何代码提交 → 任何 bug 修复 → 必须写回 6 层节点与边；
> `POST /atlas/verify` 对账 `6 层边密度 > 0.12` 才判绿。

### 2.3 业务处理流程（企业级三流程统一编排）

```
  ┌──────────────────────────────────────────────────────────┐
  │  Client / POST /ai/engine/workflow/execute               │
  │   { workflow_id, inputs, project_domain, options }       │
  └──────────────┬───────────────────────────────────────────┘
                 ▼
  ┌──────────────────────────────────────────────────────────┐
  │ GATEWAY (Rust)                                           │
  │ 1. AC-10 路由匹配                                        │
  │ 2. 意图识别 intent → workflow_id 映射（若未给）           │
  │ 3. 生成 traceId, spanId                                  │
  └──────────────┬───────────────────────────────────────────┘
                 ▼
  ┌──────────────────────────────────────────────────────────┐
  │ ORCHESTRATOR (Node EAF)                                   │
  │ 4. 拉取 workflow 模板（内置 3 + 自定义）                  │
  │ 5. 按 steps 串行/并行/DAG 调度执行                         │
  │ 6. 每 step：                                              │
  │    ├─ 创建图谱 workflow_step 节点                         │
  │    ├─ 写入 slo_snapshot 起始                              │
  │    ├─ 调用 Rust crate trait 实现（L5→L7）                 │
  │    ├─ 产物写入 Postgres / Nebula / MinIO                  │
  │    ├─ CDC 事件推送 → L1 缓存失效                          │
  │    └─ slo_snapshot 结束（dur_ms / ok / retcode）          │
  │ 7. 所有 step 完成 → 生成 workflow_result 汇总             │
  └──────────────┬───────────────────────────────────────────┘
                 ▼
  ┌──────────────────────────────────────────────────────────┐
  │ KNOWLEDGE GRAPH (Nebula)                                  │
  │ 8. business_process 节点 ← runs_on → workflow_step       │
  │    workflow_step ← runs_on → code (Rust/JS 函数)         │
  │    code ← implements → module (16 Rust crate)            │
  │    module ← documents → README/架构文档                   │
  │    code ← satisfies → demand/SPEC/Issue                  │
  │ 9. slo_snapshot ← snapshots → 6 层任意（追溯质量归因）     │
  └──────────────┬───────────────────────────────────────────┘
                 ▼
  ┌──────────────────────────────────────────────────────────┐
  │ 返回统一 shape（SPEC-8 shape 等价）                        │
  │   { ok, data, traceId, workflow_id, steps:[{id,retcode,dur_ms,artifacts}], graph:[nodes,edges] }  │
  └──────────────────────────────────────────────────────────┘
```

**内置 3 个 Workflow 模板（SPEC-10 E2E 通过）**：

| workflow_id | 步骤 | 关键产物 | 失败回滚策略 |
|---|---|---|---|
| `wf-graph-bulk-v1` | S1 解析文档 → S2 实体抽取（AI/规则双引擎） → S3 边构建（RAW 双向展开） → S4 写入 Nebula（分片路由 project_domain hash） → S5 生成 audit_event | 图谱 (V,E) + CDC 增量日志 | S4 前 = 全回滚；S4 后 = Raft 读回校验自动幂等重试 |
| `wf-file-upload-v1` | S1 分块 SHA-256 → S2 MPU 上传 MinIO EC → S3 元数据写入 Postgres+Citus → S4 生成 file_code 节点 + document 边 → S5 audit_event | chunk_manifest, object_url, metadata_id | S2 前 = 清临时分块；S2 后 = 引用计数 GC（30 天 LRU） |
| `wf-ai-rag-v1` | S1 意图识别（关键词 + activation spread PPR d=0.85 30 轮） → S2 语义向量检索 pgvector HNSW → S3 图谱子图 CNM 社区 rerank → S4 RRF 融合（reciprocal rank） → S5 LLM 生成回答 → S6 结果写入 ai_answer_code 节点 → S7 audit_event | answer, citations {doc,chunk,subgraph}, rerank_scores | 任何步失败 = 返回 top-K 纯检索兜底（可用性 ≥ 99.9%） |

**扩展机制**：自定义 workflow 用 `POST /atlas/workflows` 注册，schema 走 JSON Schema v7，步骤复用内置 step 原子算子；算子 = L5 `WorkflowStepProvider` trait 新实现，严格遵循 DIP。

### 2.4 企业级合规与治理

**GET /atlas/verify（6 层对账端点）**：
```
Response 关键：
  ok: bool
  checks: [
    { id: "rust_crates_registered", ok: true, detail: "16/16 in 3 registries + atlas_auto_registry.json" }
    { id: "ais_l6_std_only",      ok: true, detail: "operator-core kernel 0 extern crates" }
    { id: "dip_traits_bound",     ok: true, detail: "hermes/bridage/business-catalog/mox-system 全部走 trait" }
    { id: "frame_dep_not_spread", ok: true, detail: "rusqlite 仅在 mox-system" }
    { id: "algo_single_source",   ok: true, detail: "7 algo main=Rust; Node Δ≤1e-6" }
    { id: "six_layer_edge_density", ok: true, detail: "global=0.142 ≥ threshold 0.12" }
    { id: "readme_coverage",      ok: true, detail: "16/16 crate README" }
    { id: "workflow_3_complete",  ok: true, detail: "3 workflow GREEN (last 24h 各 ≥ 1 run)" }
  ]
```

**POST /atlas/governance/audit**：
- 输入：`{ time_range, project_domain?, entities?: [...ids] }`
- 输出：JSON-LD 审计导出（可导 CSV），含 6 层变更流水、执行 traceId、业务流程 step 结果、算法 Δ 记录；符合等保三级审计留痕要求（180 天，不可篡改——企业增强版）

**GET /atlas/health/enterprise**：
- 返回 SPEC-13/T14 指标：SLA（99.9% P99 / 99.95%）、RPO=0、RTO<60s、TCO 节省 42%、MinIO EC parity ok、Nebula Raft leader ok、Gateway HPA replicas ok
- 企业增强版：Prometheus exporter，Grafana dashboard JSON，PagerDuty webhook

### 2.5 非功能需求（NFR）

| 维度 | 指标 | 验收方法 |
|---|---|---|
| **性能** | 500 深链 P99 ≤ 10,000 ms；graph/search rerank P99 ≤ 200 ms；ai/engine/workflow/execute P99 ≤ 3 s（空转） | cargo test + Node E2E 取 100 次 P99 |
| **稳定性** | 14 项故障注入全绿（SPEC-14 基线）；RPO=0；RTO<60s；Gateway sidecar 3 s 降级 | fault-injection tests + readiness probe |
| **可扩展** | 新增 crate = 新增 README + CRATE_ID + 三注册表登记 + atlas_auto 自同步，≤ 30 min；新增 workflow step = 实现 L5 trait + 单测 | 手测 + test-new-crate-smoke |
| **可部署** | 开源版：`helm install mox --set tier=oss` ≤ 10 min；企业版：`--set tier=enterprise` + 3 节点 MinIO + 3 节点 Nebula + Gateway HPA ≤ 30 min | CI deploy smoke |
| **可二次开发** | 每个 crate README = 独立小项目（如何跑单测/如何改 trait/如何跑 129 全回归）；TDD RED→GREEN 文档化；精度护栏 CI 强制 | README review + CI 阻断 |
| **合规（企业增强）** | 审计不可篡改日志；Rust 二进制 SBOM 生成；输入 sanitize（长度/类型/JSON Schema v7）；OWASP Top10 覆盖（Rust 输入验证） | test-input-sanitize + SBOM 生成 |

---

## 3. 功能需求（Functional Requirements）

| 编号 | 需求 | 对应 §2 位置 | 类型 |
|---|---|---|---|
| FR-01 | 16 Rust crate 注册入璇玑图谱三注册表 + 动态层 self-sync 扫描 | 2.2 六层模型 | rule |
| FR-02 | 16 crate 显式 `pub const CRATE_ID / CRATE_META / ENGINE_NAME` 常量 | 2.2 代码绑定 | rule |
| FR-03 | 7 核心算法 singleSource=true（Rust 主实现）；Node 端对账 Δ≤1e-6 | 2.1 D 约束 | rule |
| FR-04 | operator-core L6 kernel/kernel_ext 双层拆分，kernel 0 外部依赖 | 2.1 A 约束 | rule |
| FR-05 | mox-system orchestrator 依赖 L5 Member/Task/Permission trait（非 concrete） | 2.1 B + T6 | rule |
| FR-06 | hermes-flow-bridge / business-catalog 改依赖 mox-expert trait（非 struct） | 2.1 B + T8 | rule |
| FR-07 | rusqlite 仅在 mox-system（L7）；ai-agent / primiflow-core 移除 rusqlite | 2.1 C 约束 | rule |
| FR-08 | 依赖 100% workspace=true 继承；消除版本漂移 | T4 | rule |
| FR-09 | `/ai/engine/workflow/execute` 端点 + 3 内置 workflow | 2.3 | rule |
| FR-10 | 每个 workflow step 创建图谱节点，runs_on 绑定 code 函数 | 2.2 + 2.3 S6 | rule |
| FR-11 | `GET /atlas/verify` 8 项对账端点 | 2.4 | rule |
| FR-12 | `GET /atlas/health/enterprise` SLO/SLA/RPO/RTO 端点 | 2.4 | rule |
| FR-13 | `POST /atlas/governance/audit` 审计导出端点（企业增强：不可篡改） | 2.4 | rule |
| FR-14 | 500 深链 mox_optimize P99 ≤ 10,000 ms | 2.5 性能 | rule |
| FR-15 | 14 crate README 补全（16/16 覆盖率） | 2.5 可二次开发 | rule |
| FR-16 | 架构文档三方对账（02-architecture.md / project-atlas.md / 三注册表 / 16 crate） | T10 | rubric |
| FR-17 | TCO 4 加权分（CEM）≥ 0.82 | 目标 11 | rubric |
| FR-18 | 图谱反向同步证据：SPEC-3 的每一次改动 = 6 层实体新增/更新 | 2.2 铁律 | rubric |

---

## 4. 约束与假设

**技术底座约束（红线）**：
1. 后端 Rust 全维自研（Gateway + 16 crate）；Node 仅做编排层（EAF）
2. 图谱唯一真相源：6 层归一化绑定
3. 算法护栏：CNM / PPR d=0.85-30 / Brandes / harmonic / RAW 双向 / 无 toFixed / LPA 禁用出口
4. 路由护栏：AC-10 静态优先 / 参数少优先 / 同参数长路径优先
5. 部署 4 件套：Postgres+Citus（元数据）/ NebulaGraph（图谱）/ MinIO（对象）/ Redis（向量+缓存）
6. TDD RED→GREEN：先写失败测试，再最小实现

**假设**：
- 16 Rust crate 代码骨架已存在（G-01 事实：63,783 LOC / 551 tests）
- SPEC-1 Storage 双写 PostgresProvider 已绿
- SPEC-6 Rust Gateway ai_engine 4 端点已绿
- 用户接受「企业增强版」有 3 个付费能力（审计不可篡改 / Prometheus 导出 / PagerDuty）且不进开源主干

---

## 5. 验收标准（Acceptance Criteria）——**仅 rule / rubric**

### Rule（二值可观察）
| 编号 | 内容 | 证据源 |
|---|---|---|
| AC-01 | `atlas/business-registry.js` Rust crate 登记数 = 16 | 读 registries + diff |
| AC-02 | `atlas/engine-registry.js` Rust crate 登记数 = 16；ENGINE_NAME = 常量值 | grep 常量 + 读 registry |
| AC-03 | `atlas/algorithm-registry.js` 7 条均显式 main=rust | 读 registry |
| AC-04 | `operator-core/src/kernel/mod.rs` `grep -E '^use (serde|nalgebra|ndarray|thiserror|anyhow|tracing|uuid)'` 0 条匹配 | cargo test + 脚本扫描 |
| AC-05 | Rust/Node 7 算法对账 `max|v_rust - v_node| ≤ 1e-6` | test-algo-rust-node-diff |
| AC-06 | orchestrator.rs 无 `use crate::services::Member/Task/Permission struct`；仅依赖 trait | cargo + grep |
| AC-07 | hermes-flow-bridge / business-catalog Cargo.toml `mox-expert` 下 **无** concrete struct 依赖路径；lib.rs 仅 use trait | grep |
| AC-08 | ai-agent / primiflow-core Cargo.toml `rusqlite` 行不存在；mox-system 存在 | grep Cargo.toml |
| AC-09 | 16 crate Cargo.toml 所有依赖为 `workspace = true`（例外 ≤ 1 且文档化） | grep Cargo.toml |
| AC-10 | primiflow-core dev-deps reqwest 版本 == workspace reqwest 版本 | cargo tree |
| AC-11 | `/ai/engine/workflow/execute` 3  workflow_id 各跑 10 次：`ok=true` ≥ 9/10；shape = 统一等价 | test-workflow-3-green.js |
| AC-12 | 每 workflow 运行后 Nebula `count(workflow_step)` ≥ steps.count × 10 runs | nGQL count + assert |
| AC-13 | `GET /atlas/verify` 8 个 checks 全 `ok=true` | curl + JSON assert |
| AC-14 | `GET /atlas/health/enterprise` `slo.availability.p99 ≥ 99.9` + `rpo_ms = 0` + `rto_ms < 60000` | curl + assert |
| AC-15 | `POST /atlas/governance/audit` 返回 `audit_entries.length ≥ 1`（跑过至少 1 次 workflow 后） | curl + assert |
| AC-16 | 500 深链 `boundary_ultra_deep_chain_with_data_deps` P99 100 次运行 ≤ 10,000 ms | cargo test 100× P99 |
| AC-17 | `find platform/services -name README.md | wc -l` = 16 | 脚本 |
| AC-18 | cargo build（workspace --all-targets）0 error；cargo test 0 fail；cargo clippy --all-targets -- -D warnings 0 warning | CI 日志 |
| AC-19 | Node 12 test suites 全 GREEN + Rust gateway 3 suites 全 GREEN；合计 ≥ 129 GREEN | Mocha + cargo test 输出 |
| AC-20 | 全量回归无精度护栏违规（无 toFixed、无 LPA 公开 API 返回、RAW 未双向展开） | test-precision-guardrail.js + grep |
| AC-21 | 全量回归无路由语义违规（AC-10 测试套件） | router_semantics.rs GREEN |

### Rubric（量化打分）
| 编号 | 维度 | 刻度 / 通过阈值 | 证据源 |
|---|---|---|---|
| AC-22 | 架构三方对账一致度（文档/注册表/代码/图谱） | 0=≥3 处不一致 · 1=1~2 处 · 2=0 处；阈值 ≥ 2 | diff report |
| AC-23 | 图谱 6 层边密度（衡量归一化质量） | 0=<0.08 · 1=0.08~0.12 · 2=>0.12；阈值 ≥ 2 | /atlas/verify detail |
| AC-24 | 开发者体验 DX（README 可用性/TDD 指引/CI 门禁） | 0=<8 crate 有 DX 段落 · 1=8~14 · 2=16；阈值 ≥ 2 | README 评审 |
| AC-25 | CEM 多目标加权分 0.55Q+0.2S+0.1T+0.15Stability | 0=<0.7 · 1=0.7~0.82 · 2=>0.82；阈值 ≥ 1 | CEM 测试报告 |
| AC-26 | 企业合规审计完备度（字段/追溯/不可篡改分级） | 0=字段缺 >3 · 1=可追溯但不可篡改仅企业版 · 2=开源可追溯+企业不可篡改；阈值 ≥ 1 | audit response schema |

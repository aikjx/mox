# MOX / 璇玑 归一化全系统架构设计（Normalized Full-System Architecture）

> 版本：v2.0（归一化重写版）　·　数据基准日：**2026-09-16**　·　权威等级：🟢 本层归一化架构唯一权威
> 本稿以**当前代码**为准重写，取代旧 v1.0 中"48 crate / 8 域"的迁移态基线。
> 硬事实（全部经代码核对，禁止再沿用旧值）：
> - workspace **143 个 crate**，**12 个业务域**；
> - 唯一 HTTP 入口网关 **mox-server :3080**（crate `mox-platform-gateway-svc`）；
> - 企业默认**四进程**：gateway:3080 / operator-server:3001 / alliance-scheduler:3100 / alliance-executor:3200；
> - 六层单向依赖：`foundation → api → proto → core → svc → gateway:3080`。
>
> 权威引用：结构治理 `docs/ARCHITECTURE-OF-DOCS.md`（DOC-GOV-ARC-V1.0）；端口唯一权威 `docs/api/PORT-REGISTRY.md`；接口唯一权威 `docs/API-REGISTRY.md`（223 路由，由 `scripts/gen-api-registry.py` 从 `actuator.rs ROUTES` 生成）。

---

## 0. 一句话定位

MOX 是 **Rust 原生的企业级 AI 服务平台**：把知识图谱、专家联盟多智能体编排、知识库、AI 引擎、云存储、语音、权限治理统一收敛到**单网关入口 :3080**下的模块化单体，并保留把任一业务域拆成独立进程（3411–3414）的演进能力。

设计三原则：**企业级、先设计后开发、归一化**——一个主题一个目录、一个事实一个权威源、路径即契约。

---

## 1. 全局规模与组成（事实基准）

### 1.1 总量

| 项 | 数值 | 来源 |
|---|---|---|
| workspace crate 总数 | **143** | 根 `Cargo.toml` `[workspace].members` |
| 业务域 | **12** | `platform/domains/*` |
| 企业默认进程 | **4** | `scripts/start-mox-enterprise.ps1` |
| 已登记 API 路由 | **223** | `docs/API-REGISTRY.md`（ROUTES 生成） |

### 1.2 构成分解（143 去向）

| 分组 | crate 数 | 说明 |
|---|---:|---|
| 12 业务域合计 | **121** | 见 §2 域矩阵 |
| `platform/foundation/` 横切基座 | 8 | foundation / cloud-foundation / observability / paths / error / audit / api-protocol / framework |
| `platform/domains/foundation/` | 2 | rbac-engine / pipeline-framework |
| `platform/shared/` 统一契约与运行时 | 10 | unified-contract / unified-algo-core / config / auth / observability / cache / server-runtime / resilience / event / lock |
| `platform/gateway/` | 1 | `mox-platform-gateway-svc`（唯一入口） |
| `platform/arch-test/` | 1 | 架构约束测试 |
| **合计** | **143** | |

> 业务域与"基座层"严格区分：`foundation / shared / gateway` 是横切基础设施，不参与业务域计数，也不被算作第 13 个业务域。

---

## 2. 十二域划分与职责矩阵

### 2.1 域总表（crate 数按 api/proto/core/svc/sdk 分层）

| 域 | crate 数 | 层构成 | 职责定位 | 完成度 |
|---|---:|---|---|:--:|
| **platform** | 22 | api1 / core15 / svc4 / sdk2 | 平台内核：IAM、DSQL、元数据、编排、模块、插件、连接器、集成、企业治理、算子运行时宿主 | 🟢 |
| **flow** | 18 | api1 / core12 / svc5 | 工作流平台：12 个 unified-* 内核 + WASM 算子 / PrimiFlow / 融合 / 桥接 / EA 工作区 | 🟢 |
| **alliance** | 13 | api1 / proto3 / core5 / svc2 / sdk2 | 专家联盟：10 领域专家 + 6 融合策略 + 调度/执行 | 🟢 |
| **cloud** | 13 | api1 / core4 / svc6 / sdk2 | 云存储：master/volume/s3/filer/rebalance + 纠删码内核 | 🟢 |
| **ai** | 12 | api1 / proto1 / core5 / svc4 / sdk1 | AI 能力：意图识别 / 专家调度 / Flow 编排 / Agent 运行时 | 🟢 |
| **kg** | 12 | api1 / core2 / svc8 / sdk1 | 知识图谱：算法/元数据/SDK + 8 svc（storage/service/streams/spark/hub/fusion/kb） | 🟢 |
| **data** | 10 | api1 / core3 / svc4 / sdk2 | 数据治理：公式/归一化/标准 + ETL/合规/目录/数据面 | 🟢 |
| **voice** | 8 | api1 / core1 / svc5 / sdk1 | 语音：ASR/意图/核心/算子 + DSP + Python 绑定 + 桌面端 | 🟢 |
| **base** | 7 | core7 | 统一基座纯内核抽象：model/store/index/graph/query/perm/lifecycle | 🟢 |
| **kb** | 2 | core1 / svc1 | 知识库内核 + server（默认内嵌网关） | 🟢 |
| **project** | 2 | core1 / svc1 | 项目图内核 + 服务 | 🟢 |
| **market** | 2 | api1 / svc1 | 系统模板市场（发布/浏览/加载/fork/反馈） | 🟢 |
| **合计** | **121** | | | **全 ready** |

> 完成度判定依据：逐 crate 统计 `src/` 行数、`tests/` 目录、生产 `src/` 中 `todo!()/unimplemented!()` 计数。12 域生产代码中 `todo!()/unimplemented!()` 基本为零；仅约 11 处 `unreachable!()`，全部是 match 防御兜底（如 `ParseError(_) => unreachable!()`），非功能占位。详见 L7 过程证据 `docs/working-reports/_norm_research/domain-matrix.md`。

### 2.2 域间职责边界（不重叠原则）

| 能力 | 负责域/核心 crate | 不负责 |
|---|---|---|
| 图存储/查询/算法 | kg（`mox-kg-storage-svc` / `mox-kg-service-svc` / `mox-kg-algo-core`） | 云对象存储、业务编排 |
| 知识库 | kb（`mox-kb-core`）+ kg 域 `mox-kb-svc` | 图算法、LLM 调度 |
| 多专家协作/融合 | alliance（`mox-alliance-core/fusion`） | 单 Agent 运行时（归 ai） |
| 单 Agent 运行时 / 意图 | ai（`mox-ai-agent-svc` / `mox-ai-intent-core`） | 多专家组队调度 |
| 工作流/算子执行 | flow（`mox-flow-*-core` / `mox-flow-operator-wasm-svc`） | 知识图谱算法 |
| 公式/归一化/ETL/合规 | data | 业务展示 |
| 对象/块/文件存储 | cloud（master/volume/s3/filer + Reed-Solomon） | 图存储 |
| 语音 ASR/TTS/桌面 | voice | 音乐转谱（外部 8012 桥接） |
| IAM/RBAC/元数据/编排宿主 | platform（`mox-platform-iam-core` / `mox-rbac-engine` / `mox-platform-orchestrator-svc`） | 业务算法 |
| 模板上架/分发 | market | 模板内容生成 |
| 纯内核抽象 | base | 任何 IO / 对外服务 |

---

## 3. 六层单向依赖（清洁架构变体）

```
L0 foundation  横切基座（platform/foundation + platform/shared + domains/foundation）
                 错误 / 审计 / 鉴权内核 / 配置 / 可观测 / 缓存 / 韧性 / 事件 / 锁 / 路径
   ↓
L1 api          各域对外契约 DTO（mox-<域>-api），零内部业务依赖
   ↓
L2 proto        gRPC / 服务间契约（mox-<域>-<subj>-proto），仅依赖 api
   ↓
L3 core         纯计算 / 算法引擎，无 IO（mox-<域>-<能力>-core），仅依赖 foundation
   ↓
L4 svc          服务实现（mox-<域>-<能力>-svc），依赖 api + proto + core
   ↓
L5 gateway      唯一入口 mox-server :3080（mox-platform-gateway-svc），装配路由 + 鉴权 + 反代
```

依赖规则：
1. **严格单向**，禁止底层反向依赖顶层；核心层（core）保持无 IO、可独立单测。
2. **跨域不直连**：业务域之间不互相 `use`，通过 platform 层编排、SDK 层或事件（`mox-event-core`）中转。
3. **扩展点闭环**：实现 Trait → 实现 Factory → 注册 Registry → 加配置 → 自动组装，核心代码零改动。
4. 命名公式：`mox-<domain>-<layer>-<role>`（layer ∈ api / proto / core / svc / sdk）。

> 与旧稿差异：旧稿"L6/L5/L4/L3/L2/L1/L0 七层 + 8 域"为迁移态描述；现归一为上述 **6 层、12 域**。

---

## 4. 部署进程拓扑

### 4.1 企业默认四进程（`scripts/start-mox-enterprise.ps1` 实测接线）

| 进程名 | 二进制 | crate | 端口 | 职责 |
|---|---|---|---:|---|
| **mox-server** | `mox-server.exe` | `platform/gateway/mox-platform-gateway-svc` | **3080** | 唯一 HTTP 入口；进程内内嵌 KG/KB/Cloud/IAM/RBAC/联盟任务域；中间件链（可观测→CORS→限流→鉴权 HS256）；反代业务域、后连联盟 |
| **operator-server** | `operator-server.exe` | `…/platform/svc/mox-platform-orchestrator-svc` | **3001** | 算子运行时 / 业务域宿主：聚合 primiflow / fusion / ai-agent / data-catalog / kg-algo，承载 `/api/graph/*` `/api/ai/*` `/api/market/*` 等未被网关原生命中的业务域 |
| **mox-alliance-scheduler** | `mox-alliance-scheduler.exe` | `…/alliance/svc/mox-alliance-scheduler-svc` | **3100** | 任务调度 + 专家匹配（RuleBasedExpertMatcher）+ 计划生成，HTTP 桥接执行器 |
| **mox-alliance-executor** | `mox-alliance-executor.exe` | `…/alliance/svc/mox-alliance-executor-svc` | **3200** | DAG 执行 + 节点调度 + 状态；进程内调 `mox-ai-expert-svc` 跑专家节点 |

启动接线：网关通过环境变量 `MOX_ALLIANCE_SCHEDULER_URL=http://127.0.0.1:3100`、`MOX_ALLIANCE_EXECUTOR_URL=http://127.0.0.1:3200` 后连联盟；`operator-server` 注入 `OUS_API_TOKEN`。

### 4.2 网关进程内内嵌 vs 后连（按 `modules.rs` / `proxy.rs` 源码核对）

- **进程内内嵌（全部跑在 3080 单进程）**：KG（`/kg/v1/*`，`mox-kg-service-svc`）、KB（`/api/kb/*`）、Cloud（`/cloud/v1/*`）、IAM（`/api/system|security/*`）、RBAC（`/rbac/v1/*`）、专家联盟任务域（`/api/alliance/*`）及专家广场，共 7 模块由 `build_module_routers` 统一 merge + 统一鉴权层。
- **HTTP 后连外部进程**：
  - operator-server **3001**：网关 `proxy.rs` 对未命中的 `/api/*` 做 catch-all 反向代理（注入 `OUS_API_TOKEN`）；
  - PrimiFlow **8000**：仅 `/api/projects/*`；
  - alliance scheduler **3100** / executor **3200**：`mox-alliance-http-sdk` 用 reqwest 远程调用，可用环境变量开关切本地/远程。

### 4.3 可选独立扩展（默认不开）

同一 `mox-server` 二进制 + `MOX_HOST_ROLE`，`deployment.rs::domain_router` 只装载该域：

| 宿主角色 | 容器端口 | 独立二进制 | 启用场景 |
|---|---:|---|---|
| `MOX_HOST_ROLE=kg` | **3411** | `mox-kg-server` | 图谱域需独立扩缩/隔离时 |
| `MOX_HOST_ROLE=cloud` | **3412** | `mox-cloud-server` | 云存储域独立部署 |
| `MOX_HOST_ROLE=iam` | **3413** | `mox-iam-server` | 统一 IAM 独立面 |
| `MOX_HOST_ROLE=kb` | **3414** | `mox-kb-server` | 知识库独立进程 |

默认 fused = `MOX_HOST_ROLE=all`（全内嵌、单二进制、SQLite 单副本 PVC），故不另起进程；split 形态见 `docker-compose.domains.yml`，nginx 统一入口，K8s Ingress 只转发至网关，避免绕过统一鉴权。

> 注意二进制同名歧义：遗留 Python `mox-server` 在 :8600（LEGACY，勿用），与本设计的 Rust 网关 `mox-server` :3080 同名不同物。

### 4.4 cloud 域：数据面 / 控制面边界声明（2026-09-17 新增）

> 对标 RustFS `storage-control-data-plane.md` 落地；证据：`docs/working-reports/_norm_research/cloud-split-assessment.md`。

| 面 | crate | 职责 | 边界规则 |
|---|---|---|---|
| **控制面** | `mox-cloud-master-svc`（raft/scheduler/volume_allocator/volume_replica） | 集群元数据、卷分配、副本布局、存储池拓扑 | 只读快照起步；不承载对象读写热路径；元数据变更必须显式经 master |
| **数据面** | `mox-cloud-s3-svc`（S3 语义）+ `mox-cloud-volume-svc`（卷/EC 落盘）+ `mox-cloud-filer-svc`（POSIX）+ `mox-cloud-rebalance-svc`（数据搬迁） | 对象读写、纠删码、文件服务、再均衡 | 热路径行为红线：**对象-卷放置、EC 配置、写 quorum 不得漂移** |
| **契约面** | `mox-cloud-domain-traits` + `mox-cloud-api` | StorageBackend/ChunkId/BackendCapabilities 等 trait 与 DTO | 契约不得 import 实现模块；svc 访问底层存储只能经 trait/边界 |

**热路径不漂移红线**（新增功能/重构时禁止触碰）：
1. 对象到卷/集的映射（放置）语义；
2. EC 纠删码分块与重建规则（`mox-cloud-kernel` 为唯一内核实现）；
3. 读写 quorum 与一致性模型（`ConsistencyModel::Strong`）；
4. 数据面变更必须带聚焦测试（kernel/rebalance 公共 API 集成测试已补 2026-09-17：内联 222/62 + 集成 28/25 全绿，见 cloud-split-assessment C1/C2）。

**可选独立进程**：`MOX_HOST_ROLE=cloud`（:3412，`mox-cloud-server`）即控制面+数据面合并独立部署形态；默认 fused 内嵌网关 3080。

---

## 5. 跨域关联流程（请求闭环）

### 5.1 专家联盟编排闭环（主链路）

> 关键事实（源码核对）：联盟任务闭环**不经过** operator-server:3001，网关 :3080 直接后连 3100→3200。

```
外部
 │  POST /api/alliance/tasks
 ▼
[网关 :3080] mox-platform-gateway-svc
 │  中间件：可观测 → CORS → 限流 → 鉴权（auth.rs，HS256 真验签；IAM 数据 mox-platform-iam-core / SQLite）
 │  命中联盟路由 → mox-alliance-http-sdk
 ▼  (1) HTTP reqwest 出站
[scheduler :3100] mox-alliance-scheduler-core
 │  落库 + RuleBasedExpertMatcher 专家匹配 + 计划生成
 ▼  (2) HTTP 桥接 with_executor_url
[executor :3200] mox-alliance-executor-core
 │  按 DAG 逐节点执行，进程内调 mox-ai-expert-svc（Expert 节点）
 │  融合节点汇总（§6 六大策略之一）
 ▼  (3) /tasks/:id/result、/tasks/:id/status
[网关 :3080] remote_status_poll / remote_fusion_result 取数并归一化
 ▼
外部（归一化结果原路返回）
```

### 5.2 普通业务请求闭环（对照链路）

```
外部 → 网关 :3080 → 同一鉴权
   ├─ 命中原生路由（如 /kg/v1/*、/api/kb/*、/cloud/v1/*、/rbac/v1/*）：
   │     进程内由对应内嵌 svc 直接计算，不离开 3080。
   └─ 未命中原生路由（catch-all /api/*）：
         proxy.rs 注入 OUS_API_TOKEN，HTTP 反代 → operator-server :3001
              → 业务域计算（如 mox-kg-algo-core / primiflow / ai-agent）→ 透传返回
         /api/projects/* 单独反代 → PrimiFlow :8000
```

### 5.3 闭环要点

- **鉴权只在网关做一次**（HS256），下游进程靠 `OUS_API_TOKEN` 信任网关，K8s 层禁止绕过网关直连。
- **编排分层**：平台算子编排走 operator-server:3001；专家联盟多智能体编排走 scheduler:3100→executor:3200。两条编排链并行，不混用。
- **可替换/可拆分**：上游 URL 可用环境变量覆盖（VOICE/PRIMIFLOW/ORCHESTRATOR/ALLIANCE），任务仓储可插拔（file 快照默认 / memory），远程开关即"模块化单体 → 微服务"的拆分预演。

---

## 6. 核心子系统：专家联盟（alliance）

### 6.1 六大融合策略（全部真实现，零占位）

权威位置：`platform/domains/alliance/core/mox-alliance-core/src/fusion/strategies/`

| 策略 | 文件 | 规模 | 单测 | 导出类型 |
|---|---|---|---:|---|
| weighted_voting | `weighted_voting.rs` | 238 行 | 文档断言 | `WeightedVotingFusion` |
| confidence_weighting | `confidence_weighting.rs` | 281 行 | 27 | `ConfidenceWeightingFusion` |
| stacking | `stacking.rs` | 451 行 | 23 | `StackingFusion` |
| debate | `debate.rs` | 383 行 | 25 | `DebateFusion` |
| map_reduce | `map_reduce.rs` | 389 行 | 27 | `MapReduceFusion` |
| iterative_refinement | `iterative_refinement.rs` | 357 行 | 31 | `IterativeRefinementFusion` |

`strategies/mod.rs` 显式导出全部 6 个 struct，被 executor-core/scheduler-core 引用，并有 `tests/bench_alliance.rs` 基准。

### 6.2 十大领域专家（全部真实存在并已接线）

权威定义：`…/alliance/core/mox-alliance-config-core/src/examples/domain_experts.rs` 的 `build_domain_experts()`（虽在 `examples/`，但被生产加载，非示例）。`mox-alliance-scheduler-svc/src/server.rs:238` 启动时 `let builtin_modules = build_domain_experts();`；`boot-config` 提供 yml overlay 覆盖。

| # | module_id | 定位 | 主模型 |
|---|---|---|---|
| 1 | `expert-code` | 代码编程 | deepseek-coder-v2 |
| 2 | `expert-math` | 数学推理 | openai-o1 |
| 3 | `expert-medical` | 医学咨询 | claude-3-opus |
| 4 | `expert-law` | 法律咨询 | qwen-law-72b |
| 5 | `expert-finance` | 金融分析 | claude-3-5-sonnet |
| 6 | `expert-creative` | 创意写作 | claude-3-opus |
| 7 | `expert-vision` | 图像理解 | gpt-4o |
| 8 | `expert-translation` | 多语翻译 | deepseek-chat |
| 9 | `expert-research` | 学术研究 | gpt-4o |
| 10 | `expert-arch` | 架构设计 | gpt-4o |

---

## 7. 功能完成度矩阵（2026-09-16 代码核对）

| 域 | 完成度 | 关键证据（src 规模 / 测试） |
|---|:--:|---|
| platform | 🟢 ready | `mox-dsql-core` 704、`orchestrator-core` 435、`operator-core` 1724、`graph-core` 1696 行；均带 tests |
| flow | 🟢 ready | `lowcode-core` 4112、`unified-perm-core` 3659、`unified-process-core` 2308、`unified-meta-core` 2113 行 |
| alliance | 🟢 ready | 6 策略合计 2099 行、157 单测；10 专家配置完整且已接线 |
| cloud | 🟢 ready | `rebalance-svc` 2345 行；`cloud-kernel` Reed-Solomon 纠删码 |
| ai | 🟢 ready | `agent-svc` lib 945、`intent-core` 455、`intent-svc` 704 行 |
| kg | 🟢 ready | `kg-sdk` 663、`storage-svc` 492、`fusion-svc` 306 行 |
| data | 🟢 ready | `compliance-svc` 361、`etl-svc` 383、`formula-native` 306 行 |
| voice | 🟢 ready | `desktop-app` 1117、`intent-svc` 471、`dsp-core` 429 行；各 svc 带 tests |
| base | 🟢 ready | 7 个 `mox-base-*-core` 各 176–311 行，纯内核零 todo |
| kb | 🟢 ready | `mox-kb-core` 231 行；`mox-kb-server` main 166 行 |
| project | 🟢 ready | `project-graph-svc` 总 src 1163 行 |
| market | 🟢 ready | `template-svc` 539 行，`TemplateMarket/SystemTemplate/Domain` 类型齐全 |

**结论**：12 业务域整体 ready，无 stub/空壳域。剩余"缺口"不在功能实现，而在**文档-代码漂移收敛**（见 §8）与**跨域直连治理**（旧稿统计 24 处跨域直连，持续收敛到仅平台层编排）。

---

## 8. 归一化遗留项与收敛计划（本次设计的后续动作）

### 8.1 文档-代码漂移（已盘点 49 处，详见 L7 证据 `doc-landscape.md`）

| 类别 | 待修 | 处置 |
|---|---:|---|
| 网关 `:8080` → `:3080` | 10 处 | 全改 3080（历史对照表里的除外） |
| `五进程` → `四进程` | 1 处（SYSTEM-OVERVIEW.md:77） | 改四进程表 |
| `48/60+/73 crate` → `143` | 13 处 | 改 143 |
| `8 域/八大域` → `12 域` | 25 处 | 改 12 域 |
| KB 独立进程 `:8104` | SYSTEM-OVERVIEW.md:67/83 | 改为"默认内嵌网关，可选 3414" |
| 旧 Python 栈 `:8600/:8601` | 4 篇整篇过期 | 头部标注 LEGACY |

### 8.2 权威收敛

- 本稿 `NORMALIZED_ARCHITECTURE.md`（v2.0）为 **L2 归一化架构唯一权威**；`SYSTEM-OVERVIEW.md` 作快速顶层总览（数值已与本稿对齐）。
- 多版并行稿（`ARCHITECTURE_DESIGN_v3.1.md` / `OPTIMAL_ARCHITECTURE.md` / `MOX-UNIFIED-ENTERPRISE-BASELINE-v1.0.md` / `architecture.md` 旧 Python 栈等）应在头部标注"已被本稿取代/历史参考"，不删除（保留 git 历史）。
- 变更后须跑门禁：`python scripts/check-doc-links.py`、`python scripts/verify-ports.py`、`python scripts/gen-api-registry.py` 对比。

### 8.3 演进方向（企业级加固，持续）

1. 跨域直连持续收敛：业务域间一律走 SDK / 事件（`mox-event-core`）/ 平台编排，禁止直连。
2. core 层纯计算率维持 100%（无 IO）。
3. 模块化单体 → 微服务按 `MOX_HOST_ROLE` 渐进拆分，网关始终为唯一鉴权入口。
4. 可观测/韧性/限流/审计已在 `platform/shared`（observability/resilience/audit）就绪，接入所有 svc。

---

## 9. 关联文档与证据

- 文档治理权威：`docs/ARCHITECTURE-OF-DOCS.md`（DOC-GOV-ARC-V1.0）
- 端口唯一权威：`docs/api/PORT-REGISTRY.md`（3080 / 3001 / 3100 / 3200 / 3411–3414）
- 接口唯一权威：`docs/API-REGISTRY.md`（223 路由）
- 顶层总览：`docs/architecture/SYSTEM-OVERVIEW.md`；业务流程：`docs/architecture/BUSINESS-FLOWS.md`
- 本次归一化 L7 过程证据：
  - `docs/working-reports/_norm_research/domain-matrix.md`（12 域逐 crate 完成度）
  - `docs/working-reports/_norm_research/topology-flow.md`（部署拓扑与请求闭环源码实证）
  - `docs/working-reports/_norm_research/doc-landscape.md`（文档权威分级与 49 处漂移清单）
  - `docs/working-reports/_norm_research/runtime-e2e-acceptance.md`（四进程真跑端到端验收：健康/鉴权/专家匹配/任务拆 DAG 闭环/三路指标实测增长）

---

*v2.0 · 2026-09-16 · 以当前代码为唯一事实基准：143 crate / 12 域 / 六层 / 四进程 / 网关 :3080。*

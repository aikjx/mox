# 信息关联关系图（关图）骨架定义 · 企业级基准

> 编号：**GR-STD-V1.0**
> 定位：项目启动第一步骤「搭建骨架」的归一化落地产物，是后续 P1~P5 的统一基准。
> 作用：在现有代码级图 `graph.json` 之上注入 **REQ 需求根节点** 与 **六维绑定骨架**，并约定偏离检测规则。

---

## 1. 基线事实（实测）

| 项 | 值 |
| --- | --- |
| 代码级图 `graph.json` | 352 节点 / 730 边（CodeFile 178 / Dependency 86 / Doc 34 / Config 33 / ScheduleTask 11 / Data 6 / Script 4） |
| 企业版图 `graph.enterprise.json` | 372 节点 / 751 边（新增 Requirement 20 / Bind 21） |
| 需求对齐覆盖率（首版骨架） | **88.3%** |
| 偏离节点（GR-E6） | 22（测试/CI 脚本/前端配置/孤立 ddl.sql 及其 6 张表） |

---

## 2. 模型对齐（GR-STD ↔ info-graph）

关图规范定义 12 类节点 / 7 类边；参考实现 `tools/info-graph` 已覆盖，P0 扩展如下：

### 节点类型（InfoKind）

| GR-STD 类别 | info-graph kind | 首版骨架状态 |
| --- | --- | --- |
| 业务信息 | Business | 待 P3 业务建模注入 |
| 数据信息 | Data | 已含 6 个 SQL 表 |
| 功能信息 | Function | 待 P3 注入 |
| 接口信息 | Interface | 待 P3 注入 |
| 代码文件信息 | CodeFile | 178（自动扫描） |
| 脚本 | Script | 4 |
| 定时任务 | ScheduleTask | 11 |
| 配置 | Config | 33 |
| 依赖库 | Dependency | 86 |
| 第三方服务 | ThirdParty | 经 Dependency 节点表达 |
| 文档 | Doc | 34 |
| 运行时 | Runtime | 待 P2 注入 |
| **需求根（扩展）** | **Requirement** | **20（本骨架注入）** |

### 边类型（RelationKind）

`Call / ReadWrite / Reference / Dependency / Inheritance / ConfigRef / Deploy` + **`Bind`（六维绑定边，本骨架扩展）**。

---

## 3. REQ 需求根节点清单

来源：`OUS-业务功能规划与架构数据关系分析.md` §1.1 能力域 + §1.3 功能完整度矩阵。

### 3.1 能力域需求根（D01~D13）

| ID | 名称 | 主责 crate | 状态 |
| --- | --- | --- | --- |
| D01 | 算子内核与执行 | operator-core / operator-wasm | done |
| D02 | 知识图谱 | operator-graph / ai-agent | done |
| D03 | 流程图优化 AI | flow-ai | done |
| D04 | 全维治理/璇玑 | xuanji-expert | partial |
| D05 | 业务全景目录 | business-catalog | done |
| D06 | AI 智能体 | ai-agent | done |
| D07 | 算子商城 | runtime(market) | partial |
| D08 | AI 自动化中枢 | runtime(automation) | done |
| D09 | 外部流系统桥接 | hermes-flow-bridge | partial |
| D10 | 璇玑系统 | xuanji-system | done |
| D11 | 模板市场 | template-market | done |
| D12 | 可视化拓扑前端 | frontend / primiflow | done |
| D13 | 哼唱旋律转歌谱应用 | melody2score | done |

> D13 为本次新增：把「哼唱/演奏音频 → 音高检测 → 音符解析 → 简谱/musicxml」的端到端应用作为独立能力域挂入关图，并经 `guantu.req.json` 的 Bind 边绑定到 `melody2score/` 各代码/脚本节点。其领域子图见 `melody2score/graph/melody_infograph.json`（可被 `tools/info-graph` 直接加载）。

### 3.2 跨域缺口需求根（R01~R08，来自 §1.3）

| ID | 缺口 | 状态 |
| --- | --- | --- |
| R01 | 治理台 HTTP API | gap |
| R02 | 商城死代码路由 | gap |
| R03 | WASM 真热加载 | gap |
| R04 | Hermes 真实对接 | gap |
| R05 | 浏览器真实无头 | gap |
| R06 | 六维绑定 Registry | done（P3 认证：crates/primiflow-fusion/src/sixdim.rs + platform.rs） |
| R07 | 守恒残差全局闸门 | done（P3 认证：unified.rs full_gate + conservation_report，非平凡校验 C²=κ²+τ²） |
| R08 | 文档自生成 PT-DOC | done（P3 认证：ptdoc.rs 生成 PT-DOC 01~10，含 TraceMatrix） |

---

## 4. 六维绑定骨架（REQ→FUN→BIZ→ALG→TSK→COD）

以 **Bind 边** 将 REQ 根绑定到其主责 crate 入口代码节点（`crates/*/src/lib.rs` 或 `src/main.rs`），作为「需求→代码」首层可追溯链。后续 P3 在 `flow-ai::primitive` 之上建立完整 Registry，把 Bind 细化为 **REQ→FUN→BIZ→ALG→TSK→COD** 六维一一绑定，并导出 TraceMatrix。

- **REQ（需求根）**：`Requirement:Dxx` 节点。
- **FUN（功能）**：主责 crate 入口函数（如 `normalize_requirement` / `programming_pipeline` / `xuanji_optimize`）。
- **BIZ（业务）**：业务七维专家对流程图并行分析。
- **ALG（算法）**：flow-ai 求解（CPM+RCPSP+Dijkstra+冲突修复）+ reconcile 约束物化 + verify 守恒残差。
- **TSK（任务）**：双璇玑十四维并行派发 + 回退点 Checkpoint。
- **COD（代码）**：emit/codegen 产物 + AuditChain 哈希链落库。

首版绑定锚点示例：`Requirement:D01 --Bind--> CodeFile:crates/operator-core/src/lib.rs`

> D04 实例与 TraceMatrix 主表见《璇玑全维分析需求业务处理基准》（`xuanji-requirement-baseline.md`）及其附表《TraceMatrix 六维绑定追溯》（`xuanji-trace-matrix.md`）。

---

## 5. 偏离检测规则（关图规范「需求锚定与偏离治理」）

`info-graph deviate` 以 REQUIREMENT 节点为根，做**无向可达性 BFS**：

- **GR-E6 偏离/隐性依赖**：核心实现节点（CodeFile/Script/Data/Interface/Function/Business/Runtime，非外部）不可达任何 REQ 根 → 视为无需求溯源，即偏离信号。
- **GR-E7 需求未分解**：REQ 根无任何出边（未绑定任何实现）→ 需求悬空。

报告输出：需求根数、核心节点数、已对齐数、偏离数、覆盖率(%)。

### 首版偏离清单（22 项，供 P1/P2 清零）

- 测试文件：`runtime/tests/runtime_integration.rs`、`xuanji-expert/tests/debug_opt.rs`、`xuanji-expert/tests/expert_unit_tests.rs`
- CI/校验脚本：`scripts/ci.py`、`start.sh`、`verify_axioms.py`、`verify_tests.sh`、`verify_tests.ps1`、`snake.py`
- 前端孤立：`frontend/vite.config.js`、`frontend/src/types.js`、`frontend/src/router/index.js`
- 独立后端：`primiflow/backend/engine.py`、`primiflow/backend/main.py`、`docs/ai-architecture/agentic_loop_minimal.py`
- 数据孤岛：`ddl.sql` 未挂接任何 crate，连带其 6 张表（PROJECTS/CONVERSATIONS/TOPOLOGYS/ASSETS/ARTIFACTS/TRACE_LINKS）成孤岛

---

## 6. 工具链（唯一入口）

```bash
# 1) 重建代码级图（可选）
tools/info-graph/target/release/info-graph build --root . --out graph.json

# 2) 注入 REQ 根 + 绑定骨架 → 企业级关图
tools/info-graph/target/release/info-graph skeleton \
  --graph graph.json --spec guantu.req.json --out graph.enterprise.json

# 3) 校验（孤儿/悬空/缺证据/孤岛）
tools/info-graph/target/release/info-graph validate --graph graph.enterprise.json

# 4) 偏离检测（需求对齐覆盖率）
tools/info-graph/target/release/info-graph deviate  --graph graph.enterprise.json

# 5) 导出可视化
tools/info-graph/target/release/info-graph export  --graph graph.enterprise.json --format mermaid
```

---

## 7. 后续扩展路径与 CI 门禁

| 阶段 | 动作 | 出口闸 |
| --- | --- | --- |
| P1 | 编译测试全绿；清理 R02 死代码、挂载 R01 治理台 | `cargo build/test --workspace` 0 错 0 败 |
| P2 | 注入 Runtime 节点；真接 R03/R04/R05；清理前端/后端孤立 | 偏离清单中前端/后端项清零 |
| P3 | 六维绑定 Registry（R06）+ TraceMatrix；数据表挂接 crate（R07/R08） | 六维零孤儿、连通 REQ ✅（primiflow-fusion 24 测 + 全 workspace 510 测 0 错 0 败 0 panic；`platform::p3_exit_gate_zero_orphan_connected_to_req` 编码出口闸） |
| P4 | benches 基线 + 覆盖率门禁（tarpaulin.toml + enterprise-ci coverage job） | tarpaulin ≥70% ✅（核心 12 crate lib 行覆盖率 **70.30%**；benches 基线 fuse_all 43.5µs / synthesize 57ms / full_gate 122µs / registry 236µs） |
| P5 | CI 关图校验常态化（孤儿/偏离自动告警） | 每次合并零漂移 |

**CI 门禁（P5 · `tools/guantu_gate.py`）：** 以信息关联关系图为唯一基准，在 `.github/workflows/graph-gate.yml`（push/PR 触发）强制「变更必须同步图」。

1. `build` → `graph.json`；2. `skeleton` → `graph.enterprise.json`；3. `validate` 采 GR-E1/E2/E3 + `deviate` 采 GR-E6；4. 首次运行固化基线 `.guantu_baseline.json`（已知问题签名 + 当前覆盖率）放行，避免对存量孤儿/偏离打地鼠；5. 后续运行相对基线门禁——漂移（新增孤儿/孤岛即 `exit 1`）、覆盖率回归（不低于基线，容差 0.05）、绝对下限 `COVERAGE_FLOOR=90.0`。

自动豁免（派生代码不强制溯源）：路径含 `target/`、`node_modules/`、`frontend/dist/`、`.workbuddy/`、`examples/out/`；诚实保留白名单 `ALLOWED_DEVIATIONS`（如 `snake.py` 游戏 demo）。

> 当前基线（实测）：8 项已知问题 + 覆盖率 **96.6%**（含 melody2score/core 两处孤儿、DATA_R1S~R4S 四张数据表未挂接需求根，均为已知债务，仅阻断「新增」）。本地复跑：`python3 tools/guantu_gate.py`。

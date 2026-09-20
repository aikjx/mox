# AI 处理引擎 · 算法架构模块化与归一化方案

> 状态：分析定稿，未实施
> 日期：2026-09-20
> 范围：`platform/domains/ai/**`、`platform/domains/alliance/**`、`platform/shared/mox-unified-contract`
> 上游：CAVM 融合优化方案（同目录 CAVM 融合优化方案.md）

---

## 一、诊断：当前 AI 处理引擎的八个真实问题

证据均来自本轮实际检索，行号为当前工作区状态。

### P0 · 层违规与 SSOT 空挂

| # | 问题 | 证据 |
|---|---|---|
| 1 | **统一归一化契约从未被使用**。`mox-unified-contract` 在 `platform/domains/ai` 与 `platform/domains/alliance` 下的引用计数均为 **0** | grep `mox_unified_contract` 两域命中 0 |
| 2 | 但 `mox-ai-alliance-engine/Cargo.toml:52` 已声明 `mox-unified-contract = { workspace = true }` | 空挂依赖，SSOT 形同虚设 |
| 3 | core 层含 IO：`mox-ai-alliance-engine`（位于 `domains/ai/core/`）依赖 `sqlx`(Cargo.toml:49)、`reqwest`(:46)、SSE `futures-util`(:43)，`persistence.rs` 直接建 Postgres 连接 | 违反 6 层架构「core = 纯计算无 IO」 |

### P0 · 同义定义多重化

| # | 类型 | 定义处 | 数量 |
|---|---|---|---|
| 4 | `TaskStatus` | `alliance/proto/.../types.rs:19`、`ai/core/mox-ai-alliance-engine/src/orchestration.rs:76`、`.../persistence.rs:29`（**同 crate 内两份**）、`ai/svc/mox-ai-expert-svc/src/alliance/orchestration.rs:80`、`platform/api/src/lib.rs:93`、`gateway/scheduler/mod.rs:14`、`platform-system-core/model.rs:107`、`project-graph-core/schema.rs:132`、`flow-algo-alliance-core/compute_engine.rs:37`、`flow-ai-assistant-core/types.rs:81` | **10 处** |
| 5 | `GateGrade` / `QualityGrade` | `ai/core/.../gate.rs:35`、`ai/svc/.../alliance/gate.rs:35`、`shared/mox-unified-contract/src/quality.rs:16`（名为 `QualityGrade`） | **3 处** |
| 6 | `GateScore` | `ai/core/.../gate.rs:47`、`ai/svc/.../alliance/gate.rs:45` | **2 处** |
| 7 | 门禁阈值 | `ai/core/.../constants.rs:24-28` = 0.90/0.80/0.70；`ai/core/.../learning.rs:268-271` 硬编码 0.90/0.80/0.70（**第三处**）；`shared/.../quality.rs:86` = 0.85/0.70/0.50 | **3 套，互相矛盾** |
| 8 | 门禁评分公式 | A：`constants.rs:36` `0.55Q+0.20S+0.10T+0.15St`（HC-8 锁定）；B：`mox-unified-contract/src/normalize.rs:190` `0.55Q+0.25Cov+0.20Time` | **2 套，维度完全不同** |

**后果**：同一综合分 0.80，在 alliance-engine 判 B 级，在 unified-contract 判 A 级；维度名 Speed/TokenEfficiency/Stability 与 Coverage/Timeliness 无法对齐，跨服务报表不可比。

### P1 · 术语污染（历史批量替换事故）

- 字符串 `mox 模块化系统架构` 在 `platform/` 下命中 **96+**（检索结果被截断，实际更多）。
- 已污染**功能数据**，不只是注释：
  - `Cargo.toml:7` description 写作「专家联盟mox 模块化系统架构分析引擎」
  - `router.rs` 的 `complex_keywords` 关键词表含该串 → 复杂度判定会误命中
  - `mox-unified-contract/src/quality.rs:281` 测试断言文案「安mox 模块化系统架构维度不达标」

### P1 · 两套联盟实现并行

- `domains/ai/core/mox-ai-alliance-engine`：6 阶段管线（intent→team→debate→synthesize→gate→learn→done），`RoutePath` 3 档（Fast/Standard/Deep）
- `domains/alliance/core/mox-alliance-core`：DAG 拓扑 + `fusion/strategies/` 7 个策略文件；`domains/alliance/proto/.../types.rs:66` `AllianceMode` 7 种（含 `Dynamic`，**网关侧 6 种未覆盖**）；`FusionStrategy` 9 种
- 两套体系名称近似、职责重叠，网关 `experts_orchestration.rs` 用展示串匹配融合策略（历史已踩过漂移 bug：旧串 `majority_vote` 与新串 `rrf` 不匹配导致全落默认分支）

### P1 · 正交维度混用

协作拓扑（mode，谁和谁合作）与计算深度（route，想多久）在概念上正交，但当前只有 AI 引擎有 `RoutePath`，联盟链路没有；CAVM 的「按需推理」缺少落点。

---

## 二、目标架构：AI 处理引擎模块化

### 2.1 依赖方向（严格单向，arch-test 可门禁）

```
L5 proto   mox-alliance-common-proto / mox-ai-expert-proto   （枚举与契约唯一源）
              ↑
L4 shared  mox-unified-contract      （归一化 SSOT：clamp/权重/等级/常量/命名映射）
              ↑
L3 core    mox-ai-reason-core        （纯算法：调度/评分/合成/保形/学习更新，零 IO）
              ↑
L2 engine  mox-ai-alliance-engine    （编排：阶段流/SSE/辩论/组队，异步但无 DB）
              ↑
L1 store   mox-ai-expert-svc         （IO：sqlx 持久化、事件落库、审计）
              ↑
L0 gateway mox-platform-gateway-svc  （3080 唯一入口）
```

### 2.2 模块拆分（两档）

**方案 A（低风险，建议先做）** — 不新建 crate，在 `mox-ai-alliance-engine` 内做边界收敛：

| 动作 | 内容 |
|---|---|
| A1 | `persistence.rs` 整体下沉到 `mox-ai-expert-svc`，core 移除 `sqlx` 依赖 |
| A2 | `orchestration.rs` / `persistence.rs` 的双份 `TaskStatus` 删除，统一 `use mox_alliance_common_proto::TaskStatus` |
| A3 | 新增 `src/normalize.rs`：收纳全部 clamp/权重/共识/合成/评分纯函数，只调用 `mox_unified_contract::normalize` |
| A4 | `constants.rs` 改为「re-export + 锁值测试」：常量本体迁到 unified-contract，本地保留 `pub use` 与防漂移断言（HC 硬约束语义不变） |

**方案 B（目标态）** — 拆 3 个 crate：

| crate | 层 | 职责 | 允许依赖 |
|---|---|---|---|
| `mox-ai-reason-core` | core | router（RoutePath + VOI 调度）、gate（评分分级）、synthesis、algorithm（5 维分析）、conformal（保形校准）、learning 纯计算 | unified-contract、proto |
| `mox-ai-alliance-engine` | engine | orchestration（6 阶段 + SSE）、debate、team、intent、kg、llm_consultant（HTTP 适配器） | reason-core、pipeline-framework、audit |
| `mox-ai-reason-store`（并入 svc） | store | persistence、events、session 仓储 | engine、sqlx |

### 2.3 模块清单与单一职责（`mox-ai-alliance-engine` 现状 → 目标）

| 模块 | 现状 | 目标职责 | 归一化动作 |
|---|---|---|---|
| `router.rs` | RoutePath 3 档 + 复杂度/置信度阈值 | **计算调度器**：输入状态 → 是否深度推理 | 升级为 VOI 规则（见 §4） |
| `gate.rs` | GateScore/GateGrade/GateResult | **验证与风险控制层**：评分 + 分级 + 自动执行/升级/拒绝三态 | 阈值与公式统一引用 SSOT |
| `synthesis.rs` | 归一合成 | 纯函数：多专家输出 → 单一结果 + 置信度 | 融合策略引用联盟 9 种枚举 |
| `algorithm.rs` | 5 维分析 `AnalysisDimension` | 纯计算维度分析 | 维度名 SSOT |
| `learning.rs` | EWMA 在线学习 + 硬编码 0.90/0.80/0.70 | 在线校准与漂移监测 | 删除硬编码阈值 |
| `debate.rs` / `team.rs` | 辩论 / 组队 | 编排（engine 层） | 常量引用 SSOT |
| `intent.rs` | 意图分类 | INTENT_CLASSES 7 类（HC-9） | 常量 SSOT |
| `persistence.rs` | sqlx + 自有 TaskStatus | → 下沉 svc | 删除重复枚举 |
| `events.rs` | 事件 | 事件名用 `AUDIT_EVENTS_7` | 事件名 SSOT |
| `constants.rs` | 全部 HC 常量 | → re-export SSOT | 保留锁值测试 |

---

## 三、归一化方案：六张 SSOT 表

统一落点为 `platform/shared/mox-unified-contract`，AI/alliance 两域**必须**改为引用（当前引用数为 0，这是本次归一化的核心动作）。

### SSOT-1 · 枚举

| 概念 | 权威定义 | 收敛动作 |
|---|---|---|
| `TaskStatus` | `alliance/proto/.../types.rs:19`（Pending/Planning/Running/Paused/Completed/Failed/Cancelled） | 其余 9 处删除，改 `pub use` |
| `QualityGrade`(A/B/C/D) | `unified-contract/src/quality.rs` | 两处 `GateGrade` 改为 type alias 或删除 |
| `AllianceMode` | proto 7 种 | 补齐网关 `Dynamic` 分支，6→7 |
| `FusionStrategy` | proto 9 种 | 展示串映射集中到一处（见 SSOT-4） |

### SSOT-2 · 常量（HC 硬约束）

迁移到 unified-contract，AI 引擎 `pub use` 并保留锁值测试（HC-2/5/8/9 语义不得变）：

```
SPREAD_METHOD="spread", SPREAD_DAMPING=0.85, SPREAD_ROUNDS=30      (HC-2)
RRF_K=60, SPREAD_WEIGHT=0.7                                        (HC-8 家族)
GATE_THRESHOLD_A/B/C = 0.90 / 0.80 / 0.70                          (HC-8)
DEBATE_MAX_TOKENS_PER_ROUND=900, EXPERT_TIMEOUT_SECS=60            (EAF-STD 4.3)
QUALITY_FORMULA = "0.55×Quality + 0.20×Speed + 0.10×TokenEfficiency + 0.15×Stability"
INTENT_CLASSES 7 类, PHASE_NAMES 7 段, AUDIT_EVENTS_7 7 事件
```

### SSOT-3 · 评分口径（需 ADR 决策，不可静默双轨）

当前两套公式互斥，必须由 ADR 明确二选一：

- **推荐（对齐既有硬约束）**：以 `QUALITY_FORMULA`（0.55Q+0.20S+0.10T+0.15St）与阈值 0.90/0.80/0.70 为准。理由：已被 `mox-platform-orchestrator-svc/src/routes/ai_engine.rs:150-153` 作为对外契约暴露，且有 `hard_constants_locked_no_drift` 防漂移测试与 `alliance/mod.rs:228` 的 Done 事件文案断言。
- **被废弃方**：`unified-contract/src/normalize.rs:190` 的 `gate_score`（0.55Q+0.25Cov+0.20Time）与 `quality.rs:86` 的 0.85/0.70/0.50。
- 若业务确需保留交付门禁（coverage/timeliness），**必须重命名**为 `delivery_gate_score` / `DeliveryGrade` 并显式声明与质量门禁不同轨，禁止复用 `QualityGrade` 名字。

### SSOT-4 · 命名映射（枚举 ↔ serde ↔ 展示串）

三层命名必须收敛为单一双向映射表 + 往返测试。已知非 1:1 映射（这是历史漂移 bug 的根因）：

| 枚举变体 | 网关展示串 |
|---|---|
| `BestOf` | `first_wins` |
| `Weighted` | `weighted_voting` |
| `Voting` | `rrf` |
| `ConfidenceWeighted` | `llm_judge` |
| `Concatenation` | `consensus` |
| `Stacking` / `Debate` / `MapReduce` / `Iterative` | 同名 |

要求：`to_display()` / `from_display()` 双向，未知值回退有显式日志，**不允许**在各调用点手写 match（当前 `experts_orchestration.rs::parse_fusion_strategy` 兼容三层命名即为此债的补丁）。

### SSOT-5 · 事件与审计

- 事件名：只认 `AUDIT_EVENTS_7`（7 个），阶段名只认 `PHASE_NAMES`（7 个）
- 时间戳：统一秒级 RFC3339（沿用 `alliance_remote.rs` 既有约定）
- 审计 trace 字段口径统一（沿用 `mox-audit` 哈希链）

### SSOT-6 · 术语

- **优先修功能面**：`router.rs::complex_keywords`、`Cargo.toml` description、错误/断言文案
- **其次修注释与文档**：`mox 模块化系统架构` 全量回退为正确术语
- **加 CI 禁词门禁**：`scripts/` 新增检查，禁止该串再次入库（防止批量替换复发）

---

## 四、与 CAVM 的融合落点（承接 CAVM 融合优化方案）

把论文的七个模块映射到本仓库既有资产，避免新建七个大模型，只做**接口层增补**：

| CAVM 模块 | 仓库落点 | 现状 | 增补 |
|---|---|---|---|
| ① 状态编码器 | `mox-ai-intent-core`、`experts_session` | 部分 | — |
| ② 概率决策头 | `algorithm.rs` 打分、`router.rs`、`synthesis` 置信度 | 有，非校准概率 | 输出改为校准概率，记录 Brier/ECE |
| ③ 计算调度器 | **`router.rs::RoutePath`** | Fast/Standard/Deep 阈值式 | 升级为 VOI 规则：`Δ_VOI = R_fast − R_deep > c` 才升档 |
| ④ 推理模型 | 联盟 6+1 模式 DAG、`debate.rs` | 已有 | 补齐 `Dynamic` 模式 |
| ⑤ 世界模型 | **无** | 缺失 | 缺，day-1 用 `mox-ai-flow-core` 确定性流程仿真顶替并标注受限 |
| ⑥ 验证与风险控制层 | **`gate.rs` GateScore/GateGrade** | 有形式门禁，无统计校准 | 新增 `conformal.rs`（split conformal + 覆盖计数）附于 gate 之后 |
| ⑦ 行动与反馈 | `mox-ai-flow-core`、`events.rs`、`persistence.rs` 审计 | 有执行与事件，无回滚/沙盒 | 缺 |

**新增文件建议**（全部纯计算、可单测、无 IO）：

```
mox-ai-reason-core/src/voi.rs        Δ_VOI 计算与调度决策（式 14/16），含 Regret ≤ |Δ̂−Δ| 界
mox-ai-reason-core/src/conformal.rs  split conformal：分位数、预测集合、覆盖计数（式 21-23）
mox-ai-reason-core/src/risk.rs       风险预算分配 α_t（式 27）+ 拒绝率/覆盖率统计
tests/exact_scheduling.rs             有理数精确实验（断言 450/250/250/1000/18）
```

`router.rs` 的 `RoutePath` 直接复用作调度结果；`gate.rs` 的 A/B/C/D 映射为：A/B → 自动执行，C → 升级复核，D → 拒绝（对应 CAVM 式 25 的三态协议）。

---

## 五、实施路线

| 阶段 | 内容 | 风险 | 验收 |
|---|---|---|---|
| **P0** | ① 术语污染修功能面 + 加 CI 禁词门禁；② `learning.rs` 硬编码阈值删除；③ 同 crate 内双份 `TaskStatus` 合并 | 低 | `cargo test -p mox-ai-alliance-engine` 全绿 + 禁词门禁通过 |
| **P1** | ④ SSOT-3 评分口径 ADR 决策后统一；⑤ `GateGrade`→`QualityGrade` 收敛；⑥ 门禁常量迁 unified-contract 并 re-export | 中（对外表含义变化） | 锁值测试 + `ai_engine.rs` 契约测试 |
| **P2** | ⑦ 方案 A1：`persistence.rs` 下沉 svc，core 去 `sqlx`；⑧ 新增 `normalize.rs` | 中（跨 crate 移动） | arch-test 加「core 不得依赖 sqlx/reqwest」门禁 |
| **P3** | ⑨ 方案 B 拆 `mox-ai-reason-core`；⑩ CAVM 增补（voi/conformal/risk + 精确实验测试）；⑪ `AllianceMode` 补齐 `Dynamic`；⑫ 命名映射 SSOT-4 | 高 | 全 workspace `cargo test` + `verify-ports.py` + 文档链接检查 |

### 门禁建议（防复发）

1. `arch-test`：core 层 crate 禁止依赖 `sqlx`/`reqwest`/`tokio-net`
2. 禁词检查脚本：禁止 `mox 模块化系统架构` 入库
3. 枚举唯一性检查：同名 `enum` 在 workspace 内出现次数 > 1 时告警
4. 每次改 `constants.rs` / `quality.rs` 必须同步锁值测试

---

## 六、明确不做的事（诚实边界）

- 不新建 7 个 CAVM 模块对应的大模型；只做接口层增补与纯算法模块。
- 不在本轮改动 HC-2/HC-5/HC-8/HC-9 的数值（受硬约束保护，改需 ADR）。
- 不声称归一化后性能提升；本方案是结构治理，收益需 P3 后的消融实验验证。
- 世界模型、回滚/沙盒属真实缺口，day-1 不以流程仿真冒充。

---

## 七、P0 实施记录（2026-09-20）

### 7.1 已完成

| # | 动作 | 落点 |
|---|---|---|
| 1 | 术语污染回退（可执行面） | **112 个文件 / 248 处**，`mox 模块化系统架构` → `架构`，按字节读写保留 BOM 与 CRLF |
| 2 | 路由关键词表修复 | `router.rs:227` 关键词表剔除污染串（原会导致复杂度判定误命中），补 `迁移` |
| 3 | Cargo.toml description 修复 | `mox-ai-alliance-engine/Cargo.toml:7` → 「璇玑 · 专家联盟架构分析引擎」 |
| 4 | 契约测试文案修复 | `mox-unified-contract/src/quality.rs` 阻断原因回退为「安全维度不达标」 |
| 5 | 新增术语门禁 | `scripts/check-forbidden-terms.py` |
| 6 | 同 crate 双份 `TaskStatus` 合并 | `persistence.rs` 删除自有枚举，统一引用 `orchestration::TaskStatus`；`as_str()` 委托 `label()` 保持单一真源，补齐 `from_str` 与 `Default` |
| 7 | 门禁阈值第三处硬编码消除 | `learning.rs` 测试改用 `gate::grade_from_total`（该函数由私有提升为 `pub`） |

### 7.2 术语门禁设计（`scripts/check-forbidden-terms.py`）

- **只扫可执行面**：跳过 `//` `///` `//!` `#` `*` `/*` `<!--` 开头的注释行。注释层存量另行逐步清理，避免一次性批量回退制造新错误术语。
- **扫描范围**：`platform`、`frontend-ui/src`、`scripts`、`config`、`deploy`，扩展名 12 种；跳过 `target`/`node_modules`/`third_party`/`ais`/`90_历史归档` 等。
- **跳过自身**：门禁脚本持有禁用术语定义，必须排除，否则必然自命中。
- **UTF-8 输出**：Windows GBK 控制台打印 emoji 会抛 `UnicodeEncodeError` 导致门禁自身崩溃，已 `reconfigure(encoding="utf-8", errors="replace")`。
- **`--stats`**：打印污染串上下文模式频次，用于制定批量回退方案。

### 7.3 事故与教训（务必记住）

一次性回退脚本扫描 `scripts/` 时**改写了自己的术语定义**（`FORBIDDEN_TERMS` 与 `TERM` 常量被替换为回退值），导致门禁把全仓库正常的「架构」判定为污染，一次性报出 **331 处误报**。

- 修复：恢复术语定义为原污染串；门禁脚本增加自指跳过；一次性脚本用后即删。
- 教训：**批量替换脚本必须排除自身与门禁定义文件**；更稳的做法是把术语表外置到不受扫描的目录（如 `config/forbidden-terms.txt`），或让替换脚本只在明确传入的文件清单上工作。
- 另一个坑：`git status` 显示大量文件同时存在 staged 与 unstaged 改动（并行会话所致），本轮 unstaged 改动无法用 git 单独回滚，回滚需按文件核对。

### 7.4 验证结果

| 项 | 结果 |
|---|---|
| 术语门禁 | **[OK] 扫描 1901 个文件，0 处命中** |
| `mox-ai-alliance-engine` | 95 + 6 + 61 通过，0 失败 |
| `mox-ai-expert-core` / `mox-ai-intent-core` / `mox-ai-expert-proto` / `mox-ai-flow-core` | 109 + 66 + 83 + 60 + 1 通过，0 失败 |
| `cargo check --all-targets` | 通过，无新增 warning |
| `cargo clippy --all-targets` | **本轮新增 0 条**；剩余告警均为预存在（见 7.5） |

### 7.5 新发现的预存在债务（本轮未修）

`cargo clippy -p mox-ai-alliance-engine --all-targets` 报 **15 个 error + 2 个 warning**，经 `git diff` 逐项核对均非本轮改动引入：

| 类别 | 位置 | 数量 |
|---|---|---|
| `ignoring a result with .ok() is misleading` | `mox-pipeline-framework/src/{context.rs:254,result.rs:58}`、`mox-ai-expert-core/src/{experts/architecture.rs,engine/mod.rs,normalize.rs}`、`mox-ai-alliance-engine/src/engine.rs`（263/274/277/290/...） | 15 |
| `method from_str can be confused for ... FromStr::from_str` | `algorithm.rs:42`（`AnalysisDimension`）、`orchestration.rs:41`（`OrchestrationStrategy`） | 2 |

核对方法：对 `engine.rs` 取 unstaged diff，改动行仅为 4 行中文串替换，未触及任何 `.ok()` 代码行。

这 15 处 `.ok()` 会让启用 clippy 的 CI 直接失败，属**独立于本方案的既有债务**，建议单独立项处理（涉及 3 个 crate，且 `mox-ai-alliance-engine` 文件存在并行会话改动，不宜在本轮一并修改）。

### 7.5 待办（P1/P2 未启动）

- ~~P1 阻塞项：SSOT-3 评分口径 ADR 未决策~~ → **已决策并实施，见第八节**。
- P1 其余：`GateGrade` / `QualityGrade` **不做类型合并**（见 8.2，label 语义冲突），仅统一阈值。
- P2：persistence 下沉 svc + arch-test 层依赖门禁。
- 注释层术语存量（未清，不影响门禁）：分布在 `docs/` 与代码注释中，建议按目录逐步清理。

---

## 八、P1 实施记录：ADR-SSOT-3 评分口径归一（2026-09-20）

### 8.1 决策

**采用方案一：统一到 HC-8**（阈值 0.90 / 0.80 / 0.70），废弃 unified-contract 侧的 0.85 / 0.70 / 0.50。

判定依据：
1. HC-8 已被 `mox-platform-orchestrator-svc/src/routes/ai_engine.rs:150` 作为对外契约暴露，且有 `hard_constants_locked_no_drift` 防漂移测试守护；
2. AI 联盟引擎是等级的唯一生产方，unified-contract 生产侧引用数为 **0**（仅 `mox-cloud-kb-core` 两个测试文件 `use mox_unified_contract::*`）；
3. 前端 `alliance.store.js` 硬编码的 0.85/0.70/0.50 属跨端口径漂移，须随 SSOT 同步。

### 8.2 关键修正：`GateGrade` 与 `QualityGrade` 不做类型合并

原计划把两处 `GateGrade` 收敛为 `QualityGrade`，实施前核对发现**两者不是同一类型**：

| 类型 | `label()` 返回 | 使用方 |
|---|---|---|
| `mox-ai-alliance-engine::GateGrade` | `"A"` / `"B"` / `"C"` / `"D"` | `orchestration.rs:317` 事件文案「质量门禁（{}级）」、`learning.rs` |
| `mox-unified-contract::QualityGrade` | `"优秀"` / `"良好"` / `"合格"` / `"不合格"` | 前端 `GRADE_META` 中文标签 |

若强行别名化，会改变对外 SSE 事件文案与既有契约断言。因此改为：**类型各自保留，阈值共用单一源**，并在代码注释中显式声明二者关系。

### 8.3 落地动作

| # | 动作 | 落点 |
|---|---|---|
| 1 | `GATE_THRESHOLDS` 改为 0.90 / 0.80 / 0.70 | `mox-unified-contract/src/quality.rs` |
| 2 | 锁值断言同步 + 新增守护测试 `gate_thresholds_match_ai_engine_hc8` | 同上 |
| 3 | AI 引擎阈值常量改为**引用契约**（非字面量） | `constants.rs`：`GATE_THRESHOLD_A/B/C = mox_unified_contract::GATE_THRESHOLDS.a/b/c`，HC 数值不变 |
| 4 | 前端 `GRADE_META.min` 同步为 0.90 / 0.80 / 0.70 | `frontend-ui/src/stores/alliance.store.js` |
| 5 | 异轨公式改名 | `normalize.rs::gate_score` → `delivery_gate_score`（维度为 覆盖度/时效，与 HC-8 的质量/速度/token 效率/稳定性 非同一对象），`lib.rs` 补充重导出 |
| 6 | 连带修复 | `persistence.rs` 的 `TaskStatus` 由私有导入改为 **`pub use` 再导出** |

### 8.4 连带事故与修复

删除 `persistence.rs` 自有枚举后，`mox-cloud-kb-core/tests/e2e_integration_test.rs:340` 因 `use mox_ai_alliance_engine::persistence::*` 取不到 `TaskStatus` 而编译失败（4 处 E0433）。

- 修复：改为 `pub use crate::orchestration::TaskStatus;` —— 既消除重复定义（类型是同一个，非副本），又保持 `persistence::TaskStatus` 路径对既有 glob 导入可用。
- 教训：**删除同 crate 内的重复类型时，必须同时检查跨 crate 的 glob 导入**（`use xxx::mod::*` 会隐式依赖被删项）。re-export 是比直接删除更安全的收敛方式。

### 8.5 验证结果

| 项 | 结果 |
|---|---|
| `mox-ai-alliance-engine` | 95 + 6 通过，0 失败 |
| `mox-unified-contract` | 62 通过（含新增锁值测试），0 失败 |
| `mox-cloud-kb-core` 测试编译 | 通过（re-export 修复后） |
| `cargo clippy` | 仍为 15 个预存在 error，**本轮新增 0** |
| 术语门禁 | 1901 文件，0 命中 |

### 8.6 剩余

- P2：`persistence.rs` 下沉 svc、core 去 `sqlx`、arch-test 加层依赖门禁。
- P3：拆 `mox-ai-reason-core`；CAVM 三模块（`voi.rs` / `conformal.rs` / `risk.rs`）；`AllianceMode` 补齐 `Dynamic`；命名映射 SSOT-4。
- 独立债务：15 处预存在 `.ok()` clippy error（见 7.5）。

---

## 九、专家联盟：Dynamic 协作模式开发（2026-09-20）

### 9.1 缺口

协议层 `AllianceMode` 有 7 种，但 Dynamic 长期是空实现：

| 层 | 改动前 | 状态 |
|---|---|---|
| `scheduler-core/planner.rs:71` | `Dynamic => generate_parallel_plan`，注释「暂退化为并行，运行时决策在执行器侧实现」 | **真缺口** |
| `http-sdk/alliance.rs` `build_dag_for_task` | 有 Dynamic 分支（需求分析→动态路由→深度执行→融合输出） | 展示态占位，注释自述「暂用并行拓扑占位」（描述不准确） |
| 测试 | `mode_str` / `build_dag_for_task` 用例只覆盖 6 种 | Dynamic 未测 |

### 9.2 设计决策

**不新增第七套拓扑** —— Dynamic 在既有六种拓扑中按特征选型，避免「多一种模式多一套不可测分支」。

决策在**规划期**一次性完成（非执行期实时改拓扑），因此：
- 不需要改 proto 结构体（`CollaborationPlan` 保持不变，`mode` 仍为 `Dynamic`，契约与前端展示不变）；
- 决策可单测、可复现。

规则（确定性，自上而下短路）：

| 优先级 | 信号 | 选定 |
|---|---|---|
| 1 | 任务描述含 迭代/优化/反复/改进/refine/iterate | Iterative |
| 2 | 任务描述含 评审/审核/复核/review | Sequential（先产出后复核） |
| 3 | 专家数 ≤ 1 | Sequential |
| 4 | 领域去重数 ≥ 3 | Hierarchical |
| 5 | 匹配分极差 ≥ 0.25 | Hierarchical（强弱分明，高分领衔） |
| 6 | 极差 < 0.10 且专家数 ≥ 3 | Voting（实力接近，投票裁决） |
| 7 | 其余（≥2 位专家） | Debate（存在分歧，辩论仲裁） |

阈值提为显式关联常量（`DYNAMIC_SPREAD_HIERARCHICAL` / `DYNAMIC_SPREAD_VOTING` / `DYNAMIC_DOMAIN_DIVERSITY_MIN`），便于单测锁定与后续调优。

### 9.3 可观测性

- 决策以 `tracing::info!` 落日志（task_id / selected / reason）；
- 决策理由写入**首节点 description**，形如 `[动态路由→分层] 覆盖 3 个领域，采用分层协同。...` —— 前端 DAG 与审计可见选型依据，无需改契约。

### 9.4 诚实边界

- 规则是**确定性启发式**，非学习得到，也不保证最优；
- 执行器侧「按中间结果实时改写拓扑」**仍未实现**，本实现只做规划期选型；
- 网关 `build_dag_for_task` 的 Dynamic 仍是**展示态**拓扑（受前端契约：首节点需求分析恒 Completed、末节点恒融合输出），其注释已修正为说明与调度器规划期选型的关系，避免被误读为"未实现"。

### 9.5 验证

| 项 | 结果 |
|---|---|
| `mox-alliance-scheduler-core` | 93 通过，0 失败（新增 8 个 Dynamic 用例） |
| `mox-alliance-http-sdk` | 8 通过，0 失败（`mode_str` 7 种全覆盖 + `build_dag_for_task` 7 种 DAG 有效性 + Dynamic 路由节点断言） |
| `mox-alliance-scheduler-svc` | 9 通过，0 失败 |
| clippy | 无新增 |

新增用例：单专家→串行、跨域→分层、分数悬殊→分层、分数接近→投票、默认→辩论、迭代关键词、评审关键词、决策理由可审计。

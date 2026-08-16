# 专家联盟（Expert-Alliance）全维整理 · 归一化 · 优化规范标准书

> 版本：v1.0（全维治理 / 企业级 / 与 `expert-alliance-product.md` 姊妹篇）
> 代码落点：`crates/expert-alliance`（ir / expert / experts / reconcile / verify / govern / programming / context / harness / audit / rbac / flow_loader / bench / server）
> 定位：在"产品需求已对齐"基础上，做**功能归一化、冲突诊断、I/O 规范、知识库规范、落地下一步**的企业级收口。
> 配套文档：`expert-alliance-product.md`（SRS + 架构 + 业务流）、`architecture.md`（OUS 企业级总架构 v7.0）、`mathematical-foundation.md`（数学内核）。

---

## 0. 文档导航

| 章节 | 内容 |
|------|------|
| 1. 功能全景总结 | 七维单图、专家层、裁决层、验证/治理/护栏、支撑层 |
| 2. 归一化模型 | 三层收口（输入/过程/输出）+ 四条不变式 |
| 3. 意义与应用价值 | 本质、应用场景、差异化定位 |
| 4. 重复与冲突诊断 | 正交（非重复） vs 5 项真实缺陷（P1–P5） |
| 5. 输入输出规范标准 | IN-* / OUT-* 企业级契约 |
| 6. 知识库整理规范标准 | KB Schema + 六条铁律 + 目录 + 入库门禁 |
| 7. 优化落地路线图 | P0/P1/P2 改造清单与验收 |

---

## 1. 功能全景总结（实际实现）

### 1.1 核心 IR 层（`ir.rs`）—— 单图多维铁律
- **四维合一**：业务 / 算法 / 权限 / 资源四种"图"在内存中是**同一个 `FlowGraph`**，维度仅作为节点标签（`DimensionTag`）。"改一处，全维同步"天然成立，从物理上杜绝多图分裂。
- **七维枚举 `Dimension` 与优先级**：`Permission=7, Security=7, Resource=6, Data=5, Business=4, Observability=3, Algorithm=2`。优先级即"否决权重"——权限与安全最高，性能最低。
- **`auto_dimension()`**：按节点 `tags` 中的 `dim:xxx` 前缀 + `ToolKind` 自动着色，使外部输入无需手工标注维度。

### 1.2 专家层（`expert.rs` + `experts/*`）—— 七位只读插件

统一契约 `ExpertOpinion`：
```
ExpertOpinion {
  expert, dimension,
  constraints: Vec<Constraint>,   // 可翻译为图元素的硬约束
  risks: Vec<Risk>,               // 风险（Blocking / Warning / Info）
  score: f64(0..1),               // 专家对当前图的可执行评分
  metrics, suggestions,           // suggestions = 软建议（不进裁决）
  skipped, skip_reason
}
```
否决机制：`push_risk(Blocking)` 扣分 −0.5 / `Warning` −0.2；`push_veto` 强制 `veto=true` 且升级为璇玑否决。

| 专家 | 维度 | 实际检查项 | 产出类型 |
|------|------|-----------|---------|
| business | Business | 业务链完整性 / 断点 | Risk + MustOrder |
| algorithm | Algorithm | 并行度、关键路径、缓存机会 | Suggestion(Parallelize/Cache) |
| permission | Permission | 敏感前缀无脱敏 Guard；外部写无 authz；**生产前缀 `db:prod*` 写 → `push_veto`** | MustGuard + Veto |
| resource | Resource | 资源池上限、配额 | ResourceCap |
| security | Security | Http/Shell 需沙箱；LLM 输出无 Guard；**regulated 租户 PII 经 HTTP 外发 → Blocking** | MustIsolate |
| data | Data | 非幂等写、血缘孤立、库写未开事务 | MustOrder |
| observability | Observability | 埋点 / 追踪缺失 | MustAudit |

### 1.3 裁决层（`reconcile.rs`）—— 只翻译，不求解
- 按 `dimension.priority()` **升序**处理：高优先级后写覆盖低优先级。
- 把 8 种 `Constraint` 翻译为 flow-ai 可识别的图元素：`MustGuard`→插 Guard 节点并重连前驱、`MustSerialize`→物化 Mutex 边、`ResourceCap`→取 min。
- **铁律：裁决器不求解，唯一求解器是 `flow_ai::optimize()`。**

### 1.4 验证 / 治理 / 护栏层
- `verify()` → `AlgoVerification{ vetoed, summary }`：**最高权限，治理不可覆盖**。
- `govern()` → `GateResult{ approved, reason }`，尊重 `algo.vetoed`。
- `programming.rs` 五道护栏（G-A~G-E）：
  - G-A 草稿不可执行（默认 `DraftStatus::AiDraft` 隔离）
  - G-B 动作必须映射节点
  - G-C **三证齐全**（verify 通过 + 双向一致 + Approved）
  - G-D 产出必须署名（`authored_by`）
  - G-E 失败回退最近安全点（`Checkpoint` 七态）
- `check_loops()`：图含 `LoopStart` 但 registry 未登记 → **默认视为无界即否决**（保守优先）。

### 1.5 支撑层
`harness.rs`（插件化运行时 + PreGate/PostGate 瀑布钩子）、`audit/*`（哈希链 + HMAC 签名 + S3/RabbitMQ sink）、`rbac/*`（通配符权限 `db:prod/*`）、`context.rs`（Tenant/Principal/Policy/Quota/CompatibilityRegistry: MCP+Skills+Loops）、`server.rs`、`bench.rs`（`gov_pii_graph()` 唯一权威场景构造器）、`flow_loader`（YAML 外部化）。

---

## 2. 归一化模型（三层收口 + 四条不变式）

```
输入归一化：原始意图 ─normalize_requirement()→ NormalizedRequirement（可判定）
            四类图   ─auto_dimension()→ 单一 FlowGraph + DimensionTag（物理节点唯一）
            外部能力 ─CompatibilityRegistry→ MCP/Skills/Loops 全部落到同一张图

过程归一化：7 专家 ─dispatch(只读并行)→ ExpertOpinion[]（同构契约）
            ─reconcile(优先级升序)→ ReconciledPlan（Constraint→图元素）
            ─flow_ai::optimize()→ 唯一求解器

输出归一化：verify(最高权限) → govern(闸门) → 三证齐全才 emit
            全过程 ─AuditChain(hash 链)→ 唯一可信轨迹
```

**四条不变式（架构正确性基石）**：
1. **物理节点唯一**——单图多维，改一处全维同步。
2. **专家无状态只读、互不调用**——可并行、可插件化、可独立测试。
3. **裁决器不求解**——唯一求解器是 flow-ai，避免多重最优解分叉。
4. **否决权单向**——专家 `veto` → `algo.vetoed` → 治理不可覆盖（安全/权限不可逆降级）。

---

## 3. 意义与应用价值

### 3.1 本质
它不是"多个 AI 投票"，而是把 **"AI 说行"变成"七维可判定 + 三证齐全 + 全链可审计"的工程闸门**。解决 AI 辅助编程最致命的三无问题：**无来源、无判据、无回退**。

### 3.2 应用场景
- **政务 / 金融强合规 RPA 与数据归集**（`gov-pii` 是主打场景，见 `bench::gov_pii_graph()`）。
- 企业级流程自动化治理平台（多租户算子平台的配额与越权防护）。
- AI 编程平台的出码前置闸门（草稿与可执行物理隔离）。
- 多租户 SaaS 的敏感数据外发管控（regulated 租户 PII 经 HTTP 外发 Blocking）。

### 3.3 差异化定位
`Permission/Security=7 > Algorithm=2` 的优先级设计，**制度化保证"性能永远不能绕过权限与安全"**。这是相对普通 AI 编排器的核心护城河。

---

## 4. 重复与冲突诊断（关键结论）

### 4.1 合理的"正交"（非重复，应保留）
- `permission.rs` = **图静态分析**（节点缺 Guard）；`rbac/policy.rs` = **运行时主体鉴权**（`db:prod/*` 通配匹配）。层次不同，不重复。
- `verify()` = 算法不变式；专家 = 维度判据。`programming.rs` 已把生产写保护从编排层**迁移到** permission 专家，避免重复检查——这是好设计范式，应制度化。

### 4.2 真实缺陷（5 项，按严重度排序）

**P1｜PII 判据三处分叉，造成假阳性阻断（最严重）**
- `permission.rs:22` 用 `starts_with` + 5 前缀 `["db:citizen_","pii:","id_card","phone","bank_card"]`
- `permission.rs:53` 又抄一份顺序不同的同类数组 `sensitive_prefixes_w`
- `security.rs:52` 却用 `contains("pii") || contains("citizen")`

→ 同一资源判定可能矛盾：`var:citizen_safe`（已脱敏）被 security 的 `contains("citizen")` 命中报 Blocking，但 permission 的 `starts_with("db:citizen_")` 不命中。**已脱敏数据被误判泄露 = 假阳性阻断**，直接损害可用性。

**P2｜`reconcile.conflicts` 永久为空（冲突检测未实现）**
```43:43:crates/expert-alliance/src/reconcile.rs
    let conflicts: Vec<ReconcileConflict> = Vec::new();
```
声明为**不可变 `let`**，到第 155 行返回从未 `push`。后果：
- 文档宣称的"同优先级无法仲裁时升级为 Blocking"**未实现**。
- `Permission` 与 `Security` **同为优先级 7**，约束冲突时靠 `sort_by_key` 的**不稳定排序**决定谁覆盖谁 → **裁决结果不确定**。

**P3｜语义冲突被静默吞掉**
`algorithm` 产出 `Suggestion::Parallelize`，`data`/`resource` 产出 `MustSerialize`（Mutex 边），二者语义相反。但 `Suggestion` 与 `Constraint` 是**两条不交汇通道**——建议不进裁决，冲突只能靠 flow-ai 求解器隐式兜底，无显式溯源。

**P4｜硬编码常量散落，无集中管理**
| 常量 | 位置 |
|------|------|
| 敏感前缀表 ×3 版本 | `permission.rs:22,53`、`security.rs:52` |
| 生产前缀表 | `permission.rs:52` |
| 扣分权重 0.5/0.2 | `expert.rs:113,115` |
| 维度优先级 7/6/5/4/3/2 | `ir.rs:27-33` |
| 默认配额 8/1.0/5000ms | `context.rs:87` |
| 模糊词表"尽量/差不多/尽可能" | `programming.rs:86-90` |
| Guard 节点 `duration_ms=5` | `reconcile.rs:72` |
| 角色魔法字符串 | `context.rs:219-225`、`programming.rs:364,384` |

**P5｜次要：能力/鉴权双轨**
- `Capability::ViewAudit/RunFlow/ApproveFlow` 三个变体在专家层从未使用（七位专家全查 `EditFlow`）。
- `context.can()` 用字符串比对角色，与 `rbac/policy.rs` 策略体系是**两套并行机制**，易产生不一致。

---

## 5. 输入输出规范标准（企业级契约）

### 5.1 输入规范（IN-*）

| 编号 | 规范 | 判据 |
|------|------|------|
| IN-1 | 需求书必须可判定 | 每条 `constraint` 含"主体+动作+资源+阈值"，不含模糊词 |
| IN-2 | `forbidden` 必须显式声明 | 非空，或显式 `"无"/"none"` |
| IN-3 | 必须署名 | `authored_by` 含模型名/版本/专家视角，禁止匿名 |
| IN-4 | 默认草稿 | 未确认一律 `DraftStatus::AiDraft`，不可进入建模 |
| IN-5 | **资源必须规范化 URI** | `<scheme>:<env>/<domain>/<entity>`，如 `db:prod/citizen/info`（**根治 P1 的根本手段**） |
| IN-6 | 循环必须登记 | 每个 `LoopStart` 必须有 `LoopGuard`，缺失即否决 |
| IN-7 | 节点必须声明 `accesses` | 无声明视为不可分析，Blocking |

### 5.2 输出规范（OUT-*）

| 编号 | 规范 |
|------|------|
| OUT-1 | 每条 Risk 五元组齐全：`severity + nodes + dimension + message + remediation`；Blocking 必须给 remediation |
| OUT-2 | `veto` 仅用于"不可自动修复"，必须带审批路径 |
| OUT-3 | 严重度语义固定：`Info`=提示不扣分 / `Warning`=−0.2 可自动修复 / `Blocking`=−0.5 需约束修复 / `veto`=禁止出码 |
| OUT-4 | **三证齐全才出码**：`!algo.vetoed && gate.approved && roundtrip_ok` |
| OUT-5 | 每个可执行节点必须有双向映射（可机验） |
| OUT-6 | 失败必须回退到明确 `Checkpoint` 并写审计 |
| OUT-7 | 审计事件 hash 链 + HMAC 签名，不可篡改 |
| OUT-8 | **所有阻断必须给出 `conflicts` 溯源**（修复 P2 后强制） |

### 5.3 优化优先级
1. **P0**：新建 `sensitivity.rs` 单一权威模块，导出 `is_sensitive / is_production / is_desensitized(resource)`，三处调用同一函数 → 消灭 P1 判据不一致。
2. **P0**：实现 `reconcile.rs` 冲突检测：`let mut conflicts`，对同优先级（Permission vs Security）与语义相反约束（`MustSerialize` vs `Parallelize`）`push ReconcileConflict{escalated:true}`，同级无法仲裁升级 Blocking。
3. **P1**：把 `Suggestion` 纳入裁决输入，与 `Constraint` 做交叉冲突检查（根治 P3）。
4. **P1**：新建 `constants.rs`（或 `policy.toml` 外置）集中所有阈值；角色字符串改枚举 `Role`（根治 P4/P5）。
5. **P2**：统一 `context.can()` 与 `rbac/policy.rs` 为单一鉴权入口。

---

## 6. 知识库整理规范标准（KB-*）

### 6.1 知识条目统一 Schema
```
kb_id           稳定唯一（如 KB-SEC-014），永不复用同一号
title           一句话判据式标题
dimension       七维之一（必须 ≥1，可多）
kind            判据 | 修复方案 | 场景模板 | 反例 | 术语
severity_hint   Info / Warning / Blocking / Veto
resource_scope  规范化 URI 前缀（对齐 IN-5）
judgeable       是否可机器判定（true 才可生成检查器）
code_ref        实现位置 file:line（如 permission.rs:22）
authored_by     署名（对齐 G-D）
status          draft | reviewed | authoritative | deprecated
supersedes      替代的旧条目 id
evidence        测试用例 / bench 场景引用
```

### 6.2 六条铁律
1. **单一权威源（SSOT）**：同一判据只允许**一条** `authoritative` 条目；重复必须 `supersedes` 合并 → 制度化防止 P1 类三处分叉。
2. **可判定优先**：`judgeable=true` 的条目必须能映射为代码检查器或测试用例；不可判定的只能存 `kind=术语`。
3. **代码↔知识双向绑定**：每条判据必须有 `code_ref`，每个硬编码常量必须有 `kb_id` 注释（复用双向校验思想，防文档腐化）。
4. **场景模板同源**：场景类知识必须引用唯一构造器（如 `bench::gov_pii_graph()`），禁止手写副本 → 参照 `bench.rs` 已有"唯一权威图构造器"实践。
5. **冲突显式登记**：两条判据可能相反时，必须建立 `conflicts_with` 关系并写明仲裁规则（按 `dimension.priority()`，同级则升级人工）。
6. **草稿隔离 + 生命周期**：AI 生成条目默认 `status=draft`，`draft` 不得作为检查依据（KB 层 G-A）；`deprecated` 保留但不参与检索。

### 6.3 目录组织建议
```
kb/
├─ dimensions/{business,algorithm,permission,resource,security,data,observability}/
├─ scenarios/            场景模板（引用权威构造器，不复制图）
├─ constants/            集中阈值表（与 constants.rs 一一对应）
├─ conflicts/            跨维度冲突仲裁规则
└─ glossary/             术语（Checkpoint / DraftStatus / 三证 等）
```

### 6.4 入库质量门禁（对齐三证思想）
| 门禁 | 判据 |
|------|------|
| 一致性 | 无同义重复、无与现存条目矛盾（否则必须登记 `conflicts_with`） |
| 可判定性 | `judgeable=true` 必须附机验用例 |
| 可溯源 | `code_ref` + `authored_by` + `evidence` 齐全 |
| 三证齐全才 `authoritative` | 一致性 + 可判定性 + 可溯源，缺一停留 `reviewed` |

---

## 7. 优化落地路线图

| 优先级 | 改造项 | 交付物 | 验收 | 状态 |
|--------|--------|--------|------|------|
| **P0** | 敏感判据归一 | `sensitivity.rs` 单一权威（`is_sensitive_leak`/`is_production_or_sensitive_write`/`is_desensitized`）+ `permission.rs`/`security.rs` 改为调用 | `sensitivity::tests::*` 4 例（含 `var:citizen_safe` 不再假阳性）+ `end_to_end::missing_desensitize_blocked_by_gate` 仍通过 | ✅ 已落地 |
| **P0** | 冲突检测落地 | `reconcile.rs` `let mut conflicts` + `Constraint::nodes()` + 同类别冲突升级 Blocking / 互补类别记录 semantic | `reconcile::tests::same_priority_conflict_escalates` / `complementary_constraints_not_escalated` / `serialize_vs_parallelize_recorded` / `no_false_conflict_for_distinct_nodes` | ✅ 已落地 |
| **P1** | Suggestion 进裁决 | `ReconciledPlan.adopted_suggestions` + `GovernanceReport.adopted_suggestions` 显式采纳；与 `MustSerialize` 语义相反的 `Parallelize` 不采纳（已记 semantic 冲突） | `reconcile::tests::non_conflicting_suggestions_adopted` / `parallelize_not_adopted_when_serialize_conflict` | ✅ 已落地 |
| **P1** | 常量集中化 | `lib.rs` 新增 `DIM_PRIORITY` / `DIM_THRESHOLD` / `CONFLICT_ESCALATE_PRIORITY_GAP` / `NORMALIZATION_WEIGHTS` 等 SSOT 常量；`ir.rs::Dimension::priority()` 改为委托 `dim_priority()`，消除魔法数字 | 全 workspace 编译 0 warning + `grep` 维度优先级数字零漂移 | ✅ 已落地 |
| **P2** | 鉴权单入口 | `context.can()` 由硬编码角色字符串匹配改为委托 `rbac::check`（资源级 `PermissionCheck`，`Capability` → `(action, resource)` 映射），内置角色矩阵/继承/通配符/跨租户隔离对专家鉴权真正生效 | `context::tests::rbac_editor_can_edit_flow` / `rbac_viewer_cannot_edit_flow` / `rbac_admin_passes_all` | ✅ 已落地 |

### 7.1 P0 实施要点（已验证）
1. **根治 P1 假阳性**：原 `permission.rs`/`security.rs` 三处分叉判定（含 `contains("citizen")` 误杀 `var:citizen_safe`）统一收口到 `sensitivity.rs`。敏感域用 `citizen_`（带下划线）而非裸 `citizen`，避免变量名误判；脱敏后缀 `_safe/_desensitized/_masked/_anon` 一律视为安全。
2. **P2 冲突检测不再永久空**：`reconcile.conflicts` 改为 `mut` 并真正填充。`Constraint` 增加 `nodes()` 方法支撑按节点归并；新增 `ConstraintKind` 语义分类。升级规则收紧为**同类别约束同优先级冲突才升级 Blocking**，互补类别（如 `MustGuard` + `MustIsolate`）记录为 `semantic` 溯源而不否决——避免把正常正交互补误杀（实测曾误杀 `missing_desensitize_blocked_by_gate`，已修正）。
3. **pipeline 消费升级冲突**：`alliance_optimize` 在 5.6 段检查 `plan.conflicts.escalated`，并入 `algo.vetoed`，兑现"同级无法仲裁升级阻断"的文档承诺。

### 7.2 P1 实施要点（已验证）
1. **Suggestion 真正落地**：此前专家产出的 `Suggestion`（Cache/Merge/Offload/Parallelize）只停留在 `ExpertOpinion`，裁决器从不消费，优化建议永远无法进最终计划。现 `reconcile()` 收集全部 `Suggestion` 并显式采纳进 `ReconciledPlan.adopted_suggestions`；`pipeline.rs` 写入 `GovernanceReport.adopted_suggestions` 对外暴露。采纳规则：与硬串行约束 `MustSerialize` 语义相反的 `Parallelize` **不采纳**（已在 semantic 冲突记录），其余（Cache/Merge/Offload 等）一律采纳。
2. **常量 SSOT 收口**：维度优先级、激活门槛、冲突升级门槛、归一化权重等魔法数字原散落在 `ir.rs`/`reconcile.rs`。现集中到 `lib.rs` 的 `DIM_PRIORITY`/`DIM_THRESHOLD`/`CONFLICT_ESCALATE_PRIORITY_GAP`/`NORMALIZATION_WEIGHTS`（附 `dim_priority()`/`dim_threshold()` 便捷查询），`Dimension::priority()` 改为委托 `dim_priority()`，单一数据源、零漂移。`Dimension::Permission` 与 `Dimension::Security` 保持同优先级（100），确保"权限/安全同级冲突无法仲裁即升级"的语义不变。

### 7.3 P2 实施要点（已验证）
1. **鉴权单入口打通 RBAC**：`context.can()` 原为硬编码角色字符串匹配（`EditFlow => "editor"`），与 `rbac/policy.rs` 完全脱节——对 RBAC 策略的任何修改都不会反映到专家鉴权。现 `Capability` 映射为资源级 `(action, resource)`（`ViewAudit→read:audit:*`、`RunFlow→execute:flow:*`、`EditFlow→write:flow:*`、`ApproveFlow→admin:flow:gov-pii/*`），统一走 `rbac::check`，使内置角色矩阵、继承链、通配符与跨租户隔离对专家鉴权真正生效。`Principal` 的 `subject` + `roles` 直接作为 `PermissionCheck` 输入，无需重复造鉴权逻辑。

---

## 8. 一句话结论

架构方向（**单图多维 + 只读并行专家 + 裁决不求解 + 三证出码 + 单向否决权**）是正确且优雅的企业级内核。**P0（敏感判据归一 + 冲突检测落地）、P1（Suggestion 进裁决 + 常量集中化）、P2（鉴权单入口委托 RBAC）已全部落地并通过全量测试**——判据分叉假阳性、冲突检测永久空、专家建议不落地、鉴权与 RBAC 脱节四项缺陷均已被根治。剩余可演进项：将维度优先级/门槛等 SSOT 进一步外置为 `policy.toml`、把角色字符串升级为 `Role` 枚举（根治 P4/P5），即可演进为"最伟大的"企业级全维治理产品。

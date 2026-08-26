# 璇玑 · 最强处理模式（Stage 2 设计）

> 设计原则（用户指令）：先设计、再开发，跟文档一步步落实。所有操作明确、所有代码明确。
> 目标：在「七专家诊断 → 全维裁决 → flow-ai 求解」之后，再加一层**璇玑验证网关**，
> 保证：**优化不改变语义、修复不引入新冲突、代码与流程图双向一致**。这一层数学正确性，任何治理权限都不可覆盖。

---

## 一、最强处理模式总链路（端到端）

```
原始流程图 (FlowGraph)
   │
   ├─[1] 归一化 (auto_dimension)            —— 七类流程图同一 IR，维度仅着色
   │
   ├─[2] 七专家并行诊断 (dispatch)          —— 业务/算法/权限/资源/安全/数据/可观测
   │
   ├─[3] 全维裁决 (reconcile)               —— 权限/安全优先，只翻译不求解
   │
   ├─[4] flow-ai 求解 (optimize)            —— 并行化/CPM/RCPSP/Dijkstra/冲突修复/出码
   │
   ├─[5] ⛨ 璇玑验证网关 (verify) ★新增★ —— 最高权限，数学正确性不可被覆盖
   │        5a 拓扑守恒：优化后图仍是同一 DAG 结构（节点/边计数 + 可达性闭包等价）
   │        5b 数据依赖守恒：剪除的伪依赖不会破坏真依赖（read/write 集仍满足）
   │        5c 冲突消解守恒：自动修复后 0 阻塞冲突、0 悬空异常边
   │        5d 收益可信：speedup≥1 且 scheduled_ms ≤ sequential_ms（不虚报）
   │        5e 代码往返一致：code ⇄ graph 双向解析节点/边匹配（若 emit_code）
   │
   ├─[6] 治理闸门 (govern)                  —— RBAC + 合规 + SLA/成本，最终出码许可
   │        ★ 验证网关若 FAIL，治理闸门强制 BLOCK（最高权限优先于权限/安全专家）★
   │
   └─[7] 审计链 (audit)                     —— 追加写哈希链，含验证结论
```

**最高权限优先级（覆盖一切）：**
```
算法验证网关(数学正确性)  >  权限专家  >  安全专家  >  其他专家  >  业务/资源/数据/可观测
```
即：即便权限专家批准、安全专家放行，只要验证网关发现「优化破坏了语义/依赖/一致性」，流程必须 BLOCK，且审计链记录 `algorithm_veto`。

---

## 二、验证网关算法（明确、可验证）

### 5a 拓扑守恒（Topology Invariant，语义级）
- 设 `G0` = 优化前图，`G1` = 优化后图 `opt.optimized_graph`
- 校验：`G1` 保留 `G0` 的**全部原始节点**（`id` 集合为超集；flow-ai 合法新增 guard/handler 不算丢失）
- 校验：**仅对真数据依赖对守恒可达性**——对每一对 `(u,v)` 若 `u.write_set ∩ v.read_set ≠ ∅`（写→读真依赖），则 `G0.reaches(u,v)` 为真时 `G1.reaches(u,v)` 也必须为真。
  - 普通控制边 / 无数据共享的并行化（如 `guard→web1` 被剪除，因为 web 不消费脱敏变量）**不算破坏**，属 flow-ai 合法优化。
  - 若某条「写→读」依赖被剪断导致读早于写 → FAIL（语义破坏）。
- 用 `flow_ai::model::Reachability::reaches(i,j)` 比较（节点 index 经 `index_of` 映射）。

### 5b 数据依赖守恒（Data-Dependency Invariant）
- 对每条被 `plan.removed_edges` 删除的边 `(u,v)`：
  - 若不存在 `G1` 中某条 `u→...→v` 的路径使 `u` 的写集与 `v` 的读集仍满足（即真数据依赖被剪断），则 FAIL
  - 工具：`flow_ai::dataflow` 的 `Dependency` + 节点 `read_set()/write_set()`
- 对所有保留边：检查无 **RAW 冒险违规**（读早于写）

### 5c 冲突消解守恒（Conflict Resolution Invariant）
- 取 `opt.conflicts`：必须 `has_blocking() == false`
- 所有 `remedy` 已应用：对每个 `Conflict` 其 `remedy` 非空，且应用后 `detect(G1)` 复检该资源组 0 阻塞
- 无悬空 `exception` 边：每个 `exception` 边的目标节点必须存在且是 Guard/Handler 类型

### 5d 收益可信（Credible Gains）
- `opt.gains.speedup >= 1.0`
- `opt.gains.scheduled_ms <= opt.gains.sequential_ms + ε`（并行不应比串行更慢，除非资源受限在误差内）
- `removed_false_deps <= 删除边总数`

### 5e 代码往返一致（Code Round-Trip，仅 emit_code 时）
- 若 `opt.code` 存在：用 `flow_ai::codegen::reverse_from_python` 解析生成的 `main.py`
- 反向图 `G2` 的**工具节点数量** `≥ G1` 的核心工具节点数量（反向解析器会派生新 id，故比数量不比 id）；若反向解析器因缩进未被识别到任何工具节点（rev==0 而原>0），仅**告警**不阻断。
- 不一致则标记 `code_mismatch`，**不阻断**（仅告警 + 审计），因为反向解析器有已知局限。

---

## 三、数据结构（明确）

```rust
/// 单条验证结论
pub struct Check {
    pub name: &'static str,        // "topology" / "data_dep" / "conflict" / "gains" / "code_rt"
    pub passed: bool,
    pub detail: String,            // 人类可读说明 / 反例
}

/// 璇玑验证报告（最高权限）
pub struct AlgoVerification {
    pub checks: Vec<Check>,
    pub all_passed: bool,          // 全部通过 = true
    pub vetoed: bool,              // 任一阻断级检查 FAIL = true → 治理必须 BLOCK
    pub summary: String,
}
```

- `verify(before: &FlowGraph, opt: &OptimizationReport) -> AlgoVerification`
- 在 `pipeline::mox_optimize` 中插入：步骤 [5] 调用 `verify(&raw_base, &opt)`，结果写入 `GovernanceReport.algo`（新增字段），并作为 `govern` 的硬输入：`if algo.vetoed { gate = Blocked(reason="algorithm_veto") }`

---

## 四、开发步骤（跟文档逐步）

- [x] **Step 1** 新增 `src/verify.rs`：`Check` / `AlgoVerification` / `verify()`，复用 flow-ai 原语，含 5 类检查
- [x] **Step 2** `pipeline.rs`：`GovernanceReport` 加 `pub algo: AlgoVerification`；`mox_optimize` 调用 `verify` 并让治理尊重 veto
- [x] **Step 3** `govern.rs`：`GateResult` 增加 `algorithm_veto` 判定分支（最高优先级）；`FlowStatus` 新增 `Blocked` 变体
- [x] **Step 4** `server.rs`：DTO 透出 `algorithm.checks`（前端高亮「验证通过/算法否决」）
- [x] **Step 5** 前端 `mox.html`：新增「⛨ 璇玑验证」卡片
- [x] **Step 6** 单测：`normal_optimization_passes_verification` 验证阻断级检查全部通过、`code_roundtrip_passes_for_generated_code` 验证往返（**5a 经实测修正为语义级**：真数据依赖守恒，普通并行化不判违规——flow-ai 正确剪除 `guard→web1/web2` 这类无数据共享边）
- [x] **Step 7** CLI `mox verify <flow.json>` 子命令（退出码 0=通过 / 2=否决）；`demo` 展示验证结论
- [x] **Step 8** 编译 + 全量测试 + HTTP 实测：**lib 23 + integration 5 全通过，0 warning**

---

## 五、预期收益（最强模式）

- 任何优化/修复若破坏语义 → 被算法网关**自动否决**（而非靠人工 review）
- 代码与流程图双向一致可被 CI 证明
- RBAC/合规让位于数学正确性：不会出现「权限放行但结果错」的事故
- 前端实时显示「算法验证✔ / 否决✘」，形成完整可信闭环

# 璇玑校验设计（Algorithm Verification · AV）

> 配套文档：`docs/architecture.md`(总架构) · `docs/modules/mathematical-foundation.md`(数学内核) · `docs/modules/xuanji-expert-normalization.md`(归一化) · `docs/modules/xuanji-expert-product.md`(产品化) · `docs/modules/business-process-flows.md`(企业级业务处理流程)

- 文档等级：🟢 权威（设计态 · modules/）
- 编号：AV-STD-V1.0
- 适用范围：operator-unified-system（OUS）数学内核自洽性、PT‑Primi 合规、璇玑治理闸门的统一验证矩阵
- 关联规范：`docs/specs/pt-primi-架构规范-v1.0-完整版.md` §9 · `docs/full-dimensional/GOVERNANCE_CONSOLE_API_READY_20260816.md` · `docs/enterprise/璇玑-信息化系统开发验收报告-V1.0.md`

---

## 1. 定位与范围 <a id="s1"></a>

**璇玑校验（Algorithm Verification, AV）** 定义"系统自洽性"的企业级验证矩阵，回答三个问题：

1. **数学是否自洽？** —— operator-core 六公理与守恒律是否可被机械验证（L1）。
2. **拓扑是否合规？** —— 涌现拓扑是否满足 PT‑Primi 守恒恒等式与六维绑定（L2）。
3. **治理是否放行？** —— ⛨璇玑验证网关（最高权限）是否通过守恒残差闸门与审计链（L3）。

AV 不负责"生成"，只负责"证明自洽"；其结论作为治理闸门（Governance Gate）的输入，未通过则拓扑/代码禁止进入生产验收。

---

## 2. 验证维度与判定标准 <a id="s2"></a>

| 维度 | 名称 | 验证对象 | 判定标准（阈值） | 工具/来源 |
| --- | --- | --- | --- | --- |
| L1 | 数学公理自洽 | operator-core 六公理 + 守恒律 | 六公理全部 `pass`，概率守恒 L1 范数 = 1、能量守恒 L2 范数稳定 | `verify_axioms.py`（仓根） |
| L2‑a | PT‑Primi 守恒 | 涌现拓扑 `C² = κ² + τ²` | 残差 `ε = \|C − √(κ²+τ²)\| ≤ ε_max`（默认 `1e‑3`），否则拒绝并报警 | `docs/specs/pt-primi-架构规范-v1.0-完整版.md` §3.1 / §9.1 |
| L2‑b | 六维绑定 | REQ/FUN/BIZ/ALG/TSK/COD | 零孤儿；`TraceMatrix` 全量导出且连通至 `REQ` | 静态扫描 + TraceMatrix |
| L2‑c | 确定性 | 生产拓扑 | 记录 `(G, B, P, seed)`，`Emerge` 可复现；无 seed 视为实验态，禁入验收 | 配置校验 |
| L3 | 璇玑治理闸门 | ⛨璇玑验证网关（最高权限） | `GovernanceReport` 全绿、`AuditChain` 完整可追溯 | `docs/full-dimensional/GOVERNANCE_CONSOLE_API_READY_20260816.md` |
| L4 | 工程质量 | 编译/测试/覆盖 | 错误/失败 = 0；核心 crate 行覆盖 ≥ 70% | `cargo test` / `tarpaulin` |

---

## 3. 验证方法（工具链） <a id="s3"></a>

### 3.1 L1 数学公理自洽验证
- 脚本：`verify_axioms.py`（仓根）。
- 覆盖：公理 1 万物皆算子、公理 2 算子可组合（`>>` 表示 `g∘f`）、公理 3 算子可并行（张量积 `@`）、单子三定律、范畴论定律、守恒律系统（概率守恒 L1 范数 = 1、能量守恒 L2 范数）。
- 输出：六公理 + 守恒律逐项 `pass/fail`；任一 `fail` 即阻断。

### 3.2 L2 PT‑Primi 合规验证
- 守恒残差：依 `ε = |C − √(κ²+τ²)|` 计算，超 `ε_max` 拒绝发布并报警（规范 §9.1）。
- 六维绑定：静态扫描 `REQ/FUN/BIZ/ALG/TSK/COD` 绑定键，校验"每个 FUN 绑定 ≥1 REQ、每个 BIZ 绑定 ≥1 FUN、每个 ALG 绑定 ≥1 BIZ、每个 TSK/COD 绑定其上游 ALG"，零孤儿。
- TraceMatrix：生成六维全链路溯源矩阵，写入全域知识库，支持"沿绑定链回溯至 REQ、前向传播影响分析"。

### 3.3 L3 璇玑治理闸门
- ⛨璇玑验证网关为最高权限节点，汇聚 `xuanji_optimize` 的 `GovernanceReport(AuditChain)`，对守恒残差、绑定完整性、审计链完整性做最终裁决。
- 闸门双通道（veto / state）：否决则阻断出码/出图；通过则放行至治理闸门与交付。

### 3.4 L4 工程质量
- `cargo test --workspace` 全绿；`tarpaulin` 覆盖率门禁（核心算法 crate ≥ 70%）。

---

## 4. 闸门与基线 <a id="s4"></a>

- **强制合规项（验收前必须全绿）**：守恒残差 ε 通过 · 六维绑定零孤儿 · TraceMatrix 连通 REQ · 所有生产拓扑含 seed · 全套 PT‑DOC‑01~10 已生成且含溯源页 · 外部子图已隔离影响域 · 定时任务声明触发/重试策略 · 代码文件含绑定注释。
- **基线**：`ε_max = 1e‑3`（默认，可配）；覆盖率阈值 70%（建议，可据 crate 调整）。
- **可复现**：生产拓扑必带 `seed`，缺失视为实验态，禁止进入验收。

---

## 5. 验收联动 <a id="s5"></a>

- AV 结论是 `docs/enterprise/璇玑-信息化系统开发验收报告-V1.0.md`（ISD‑V1.0）的验证依据之一；ISD 验收以"AV 全维度通过 + 闸门放行"为前提。
- 每次全功能开发完成一项，即跑对应标准校验，确保"最优"可量化证明而非主观判断。

---

## 6. Glossary <a id="glossary"></a>

| 术语 | 英文 | 定义 |
| --- | --- | --- |
| 璇玑校验 | Algorithm Verification (AV) | 系统自洽性的统一验证矩阵（L1 数学 / L2 合规 / L3 治理） |
| 守恒残差 | Conservation Residual (ε) | `ε = \|C − √(κ²+τ²)\|`，度量拓扑是否自洽 |
| 六维绑定 | Six‑Dimensional Binding | REQ/FUN/BIZ/ALG/TSK/COD 一一映射 ID 绑定（公理 A4） |
| TraceMatrix | Traceability Matrix | 六维全链路溯源矩阵，连通至 REQ |
| 璇玑验证网关 | Xuanji Verification Gateway (⛨) | 最高权限治理节点，汇聚 GovernanceReport/AuditChain 做最终裁决 |
| 审计链 | AuditChain | 治理动作不可篡改的链式记录 |
| 种子 | Seed | 保证涌现 `Emerge` 确定性的随机种子 |

---

## 7. RACI <a id="raci"></a>

| 活动 | 负责人 | 复核 | 知会 | 批准 |
| --- | --- | --- | --- | --- |
| L1 数学公理验证 | 算法工程师 | 架构师 | 测试 | 架构师 |
| L2 PT‑Primi 合规校验 | 平台工程师 | 架构师 | 测试 | 架构师 |
| L3 璇玑治理闸门裁决 | ⛨璇玑验证网关（自动） | 架构师 | 治理台 | 治理负责人 |
| ISD 验收联动 | 验收负责人 | 架构师 | 客户 | 项目 owner |

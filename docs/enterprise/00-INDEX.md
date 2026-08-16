# 专家联盟系统 · 企业级文档总目录（Documentation Master Index）

> **文档定位**：本目录是「专家联盟系统（Expert Alliance System）」企业级文档的**唯一入口与治理中心**。
> 它将散落在 `docs/` 各处的需求、架构、设计、业务处理文档统一收敛为一套**可迭代、可追踪、可追溯**的标准化文档集，
> 并定义文档本身的版本管理、角色职责（RACI）与评审节奏，使「文档即产品」成为工程纪律。
>
> **适用系统**：
> - `crates/alliance-system`（协作治理域：成员 / 任务 / 权限 / 通信）
> - `crates/expert-alliance`（璇玑融合引擎域：双联盟十四维治理 / 归一化 / 裁决）
> - `frontend/`（融合工作台 AllianceFusionView、监控台 MonitorView）
>
> 最后更新：**2026-08-16** · 版本：**v1.0 (ENT)**

---

## 1. 文档集总览（四文档 + 治理 + 路线图）

| 编号 | 文档 | 类型 | 说明 | 状态 |
|------|------|------|------|------|
| `00-INDEX.md` | 文档总目录与治理 | **治理** | 本文：导航、版本注册、RACI、评审节奏 | ✅ v1.0 |
| `01-requirements.md` | 需求规格（SRS） | **需求文档** | 干系人、范围、功能/非功能需求、追踪矩阵、验收 | ✅ v1.0 |
| `02-architecture.md` | 企业级架构（多视图） | **架构文档** | 业务/信息/应用/技术/安全/集成/部署七视图 + ADR | ✅ v1.0 |
| `03-design.md` | 详细设计 | **设计文档** | 四大子系统模块设计、领域模型、FSM、API 契约 | ✅ v1.0 |
| `04-business-processing.md` | 业务处理 | **业务处理文档** | 8 大业务流程、生命周期状态机、BR 规则目录 | ✅ v1.0 |
| `05-iteration-roadmap.md` | 迭代与优化路线图 | **治理/路线图** | 持续改进机制、优先级待办、企业级 DoD、KPI、风险 | ✅ v1.0 |

**配套参考文档（既有，纳入本索引统一管理，不重复造轮子）**：

| 文档 | 归属视图 | 与本文关系 |
|------|----------|------------|
| `docs/architecture.md`（v7.0） | 总体技术架构 | 父系统 OUS 总架构，本文是其「联盟子系统」切面 |
| `docs/enterprise-architecture-analysis.md` | 架构/能力矩阵 | 双联盟十四维、能力覆盖矩阵、持续优化清单 |
| `docs/expert-alliance-business-requirements.md` | 需求/业务规则 | 联盟融合业务规则（BR-01…BR-21、GAP 清单），是 `01/04` 的权威来源 |
| `docs/business-process-flows.md` | 业务处理（执行引擎） | WorkflowEngine / 6 企业模板落地 |
| `docs/business-process-flowcharts.md` | 业务处理（可视化） | Mermaid 流程图 / 时序图全集 |
| `docs/expert-alliance-alliance-fusion-flows.md` | 业务处理（融合） | BP-6 联盟融合优化链路 |
| `docs/expert-alliance-normalization.md` | 设计（融合引擎） | 归一化 IR 设计 |
| `docs/expert-alliance-product.md` | 产品/架构 | 产品化视角 |
| `crates/expert-alliance/DESIGN.md` / `DESIGN_STAGE2.md` | 设计（融合引擎） | 璇玑引擎实现设计 |
| `docs/alliance-system-business-architecture.html` | 架构（可视化） | 全维度分层架构交互图 |

---

## 2. 文档标准（Enterprise Documentation Standard）

### 2.1 编号与版本

- **文档版本**：采用 `主版本.次版本` + 阶段标签（`DRAFT` / `REVIEW` / `ENT` 企业级发布 / `SUP`  superseded）。
- **需求编号**：功能需求 `FR-<域>-<n>`（如 `FR-MEM-01`）；非功能 `NFR-<n>`；业务规则 `BR-<n>`；架构决策 `ADR-<n>`。
- **变更驱动**：任何代码合并（尤其是 `crates/alliance-system`、`crates/expert-alliance`）若影响行为/接口/权限，必须在合并前同步更新对应文档章节与「变更记录」。

### 2.2 角色职责（RACI）

| 活动 | 架构师 | 模块 Owner | 安全/合规 | 文档维护者 | QA |
|------|:--:|:--:|:--:|:--:|:--:|
| 需求变更评审 | A | R | C | C | I |
| 架构决策（ADR） | R | C | C | I | — |
| 设计评审 | A | R | C | C | I |
| 业务规则变更 | C | R | A | C | I |
| 文档发布（ENT） | A | C | C | R | C |

> R=负责执行，A=最终问责，C=被咨询，I=被知会。

### 2.3 评审与迭代节奏

- **文档即代码**：与源码同仓库、同 PR 评审。
- **季度架构评审（QAR）**：每季度核对 `02-architecture.md` 与代码事实偏差（参考 `enterprise-architecture-analysis.md` 的「代码事实校验」机制）。
- **增量迭代**：每个迭代（建议双周）产出 `05-iteration-roadmap.md` 的 backlog 进展，并 bump 相关文档次版本。
- **GAP 闭环**：发现的缺陷/缺口登记为 `BR-/GAP-` 并进入路线图，修复后回填「追踪矩阵」与「变更记录」。

---

## 3. 企业级「文档完备性」自查清单

本套文档声称达到「最高标准企业级架构」，以如下清单自证（✅ 已具备，📋 路线图中）：

| 维度 | 必备产物 | 落点 | 状态 |
|------|----------|------|------|
| 需求可验证 | 每条需求映射代码/测试，GAP 有验收断言 | `01-requirements.md` §追踪矩阵 | ✅ |
| 架构多视图 | 业务/信息/应用/技术/安全/集成/部署 | `02-architecture.md` | ✅ |
| 设计可追溯 | 模块→代码文件→接口契约 | `03-design.md` | ✅ |
| 流程可闭环 | 生命周期 FSM + 业务规则目录 | `04-business-processing.md` | ✅ |
| 安全可审计 | RBAC/作用域/审计链/威胁模型 | `02/03/04` | ✅ |
| 迭代可持续 | 路线图 + DoD + KPI + 风险 | `05-iteration-roadmap.md` | ✅ |
| 可观测性设计 | 指标/追踪/告警定义 | `05` 待补 → `02` §技术视图 | 📋 |
| 多租户隔离 | 数据/策略按租户分层 | `01/02` 已声明，`05` 强化 | 📋 |
| 灾备与持久化 | WAL/快照/重放 | `05` 路线图 | 📋 |

---

## 4. 阅读路径建议

- **新成员 onboarding**：`00-INDEX` → `01-requirements` → `04-business-processing` → `03-design`。
- **架构师评审**：`02-architecture`（七视图 + ADR）→ `05-iteration-roadmap`。
- **安全合规审计**：`01-requirements`（NFR/安全）→ `03-design`（RBAC/审计）→ `04-business-processing`（BR 规则）。
- **开发新功能**：对应 `03-design` 模块 → 改代码 → 同步 `01/04` 追踪矩阵 → 提 PR。

---

*本目录为活文档，随系统演进持续迭代。任何目录结构变更须在「变更记录」留痕。*

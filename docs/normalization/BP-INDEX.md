# BP-INDEX · 业务流程归一化（BP-xx）

> 编号：**DOC-NORM-BP-V1.0** · 归属：[README.md](README.md)（SSoT 枢纽）
> 内容：mox 全行业统一的业务处理流程（5 阶段 SOP）+ 领域包流程 + 核心子流程。

---

## 1. 统一业务处理流程（5 阶段 SOP）

所有行业的业务系统后端到前端，处理流程统一为 5 阶段，每阶段绑归一化产出与规范编号。

| 阶段 | 动作 | 归一化产出 | 规范 | 现有落点 |
|------|------|-----------|------|----------|
| 需求 | AI 对话 → 需求编译 → 需求知识图谱 → 需求文档 | BP 业务流程图 + 需求 IR | `BP-01` | `CaomeiView` / `aiCaomeiParse` / 功能图谱 §2.3 |
| 架构 | 架构设计 → 系统架构图 → ER 图 → 技术选型 | ARC 架构规范 + API 契约 | `BP-02`→`ARC/API` | `aiGenerateFlowDiagram` / `aiGenerateErd` |
| 实现 | 开发测试 → 代码生成 → 制品管理 | 代码/DDL/图谱 + TPL 清单 | `BP-03`→`TPL` | `aiFullComplete` / `primiflow` |
| 测试 | 测试策略 → 单测/集成/E2E → 缺陷修复 | 验证矩阵 | `BP-04`→`VAL` | `aiDevTestFix` / 功能图谱 §10 |
| 验收 | 验收标准 → 发布闸门 → 发布 KB → mox 模块化系统架构完成 | 发布回执 + evidence 入图 | `BP-05`→`VAL` | `aiPublishArtifactsToKb` / `aiGenerateProjectGraph` |

**事实来源**：`docs/对话开发系统-端到端流水线.mmd` · `璇玑-mox 模块化系统架构需求业务处理流程图-归一化企业级.md` · 功能图谱 §9。

---

## 2. 核心子流程（归一化 SOP）

| 流程 | 步骤（归一化） | 现有落点 |
|------|---------------|----------|
| 项目联动上下文 | 选项目 → 自动注入 project_id → 会话/任务/资源/图谱/制品归属 | `ProjectPicker` / `ProjectChip` / `projectContext.js` |
| 专家联盟调度 | 提问 → 意图识别 → 路由(内容感知) → 单/多/辩论/智能咨询 → 熔断器 → 图谱沉淀 | `ExpertCenterView` / `expertDebate` / `multiExpertConsult` |
| 知识沉淀 | AI 对话 → 自动入图(开关) → 节点/边 → 导出/项目图谱/发布 KB | `GraphView` / `aiPublishArtifactsToKb` |
| 对话开发端到端 | 需求→架构→实现→测试→验收（5 阶段展开） | `docs/对话开发系统-端到端流水线.mmd` |

---

## 3. 行业领域包流程（BP-1x 系列，待落地样例）

> 行业差异数据化/配置化/模板化，流程本身不变。以下为归一化流程骨架，落地时填领域参数。

| 领域包 | 代表流程 | 复用模板 | 复用底座 |
|--------|----------|----------|----------|
| 金融 `finance` | 信贷申请→风控→审批→放款→对账 | `TPL-03` 主子表 + `TPL-05` 工作流 | mox_sys + iam + audit |
| 医疗 `medical` | 问诊→病历→排班→医保结算 | `TPL-04` 图谱 + `TPL-01` 单表 | mox_sys + kg + ea |
| 政务 `gov` | 事项申报→受理→审批→出证→监管 | `TPL-05` 工作流 + `TPL-02` 树表 | mox_sys + iam(sso) + audit |

> **首个落地样例**：[BP-GOV-政务审批.md](business/BP-GOV-政务审批.md)（DOC-NORM-BP-GOV-V1.0）—— 含领域包装配、5 阶段 SOP、流程明细、零重复造轮子论证；可作为其余行业（金融/医疗/零售）复制蓝本。
| 零售 `retail` | 进销存→会员→营销→订单履约 | `TPL-03` 主子表 + `TPL-06` AI 域 | mox_sys + ea + ai |

> 状态：🔴 尚无领域包样例。首个样例建议 `gov`（政务审批），复用现有 `WorkflowView` + `iam(sso)`。

---

## 4. 登记规则

- 新业务流程文档命名 `BP-{两位序号}-{中文短名}.md`，放 `docs/normalization/business/`。
- 必须含编号章节 + `#anchor`，头部声明权威等级。
- 跨流程引用统一 `docs/normalization/BP-INDEX.md#章节`。

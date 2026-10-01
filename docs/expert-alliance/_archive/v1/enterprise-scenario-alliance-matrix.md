---
title: 政企场景 × 联盟能力覆盖矩阵
version: v1.0
doc_id: EA-DOC-SCENARIO-MATRIX-001
last_updated: 2026-09-24
source_of_truth: mox-alliance 代码已实现能力（scheduler / executor / fusion / registry）
---

# 政企场景 × 联盟能力覆盖矩阵

> 📌 **诚实声明**
>
> 本矩阵**仅基于代码已实现能力**编制，能力清单以 `mox-alliance-config-core/src/examples/domain_experts.rs`、调度器（:3100）、执行器（:3200）、融合引擎、注册中心（:3400）的现有实现为准。
>
> 当前环境**未配置真实 LLM 模型**（`MOX_LLM_*` 环境变量未设置），执行器在无模型时返回 `MODEL_NOT_CONFIGURED` / `ExecutorUnavailable (4002)`，**此为预期行为**。因此本文档中所有"端到端场景效果"一律标注为：**待配置 `MOX_LLM_*` 后验证**，不主张"已完成端到端验证"。

---

## 一、已实现能力基线（编制依据）

| 类别 | 已实现项 |
|------|----------|
| 协作模式（6） | `sequential` 串行、`parallel` 并行、`debate` 辩论、`hierarchical` 层级、`iterative` 迭代、`voting` 投票 |
| 融合策略（6 + 6 兼容） | `weighted_voting`、`confidence_weighting`、`stacking`、`debate`、`map_reduce`、`iterative_refinement`；兼容基础函数 `rrf` / `weighted` / `voting` / `best_of` / `concatenate` / `merge_json` |
| 内置专家（10） | `expert-code`、`expert-math`、`expert-medical`、`expert-law`、`expert-finance`、`expert-creative`、`expert-vision`、`expert-translation`、`expert-research`、`expert-arch`（见 `domain_experts.rs`） |
| 调用链路 | 网关 `:3080 /api/alliance/*` → http-sdk `RemoteAllianceClient`（注入 `X-Tenant-Id` / `X-User-Id` / `x-request-id`）→ 调度 `:3100`（专家匹配 + DAG 规划 + 幂等提交 + 持久化）→ `ExecutorBridge` → 执行 `:3200`（DAG 执行 + 融合 + `state_sink` 持久化）→ `read_back` 回读；注册中心 `:3400` 负责专家实例注册 / 发现 / 心跳 |
| 运行时能力 | 任务提交（`idempotency_key`）、状态机仲裁 `can_transition_to`、任务 CRUD、`cancel` / `pause` / `resume` / `retry` / `skip_node`、执行状态回读、融合输出持久化（`persist_fusion_output` / `read_fusion_output`）、`search_experts` 按能力/领域过滤、租户身份全程贯穿 |
| HA | 多活租约选主（默认关闭，`MOX_ALLIANCE_HA_MODE=on` 开启） |

---

## 二、政企场景覆盖矩阵

| # | 场景 | 适用协作模式 | 融合策略 | 参与专家 | 调用链路 | 已实现 / 待验证状态 |
|---|------|--------------|----------|----------|----------|---------------------|
| 1 | 政企公文起草与合规审查 | `hierarchical`（起草→核稿→签发层级）+ `iterative`（起草-审核-修改循环） | `iterative_refinement`（多轮打磨成稿）+ `confidence_weighting`（合规意见高权重） | `expert-creative`（起草）、`expert-law`（合规审查）、`expert-research`（政策依据核对） | 网关:3080 → SDK 注入租户头 → 调度:3100（DAG：起草→法律核稿→修订循环）→ 执行:3200（迭代精炼）→ `read_fusion_output` 回读成稿 | 编排/状态机/迭代融合代码已实现；真实 LLM 成稿与合规审查效果**待配置 `MOX_LLM_*` 后验证** |
| 2 | 合同智能审查与风险提示 | `parallel`（法律/财务/行业惯例三维并行）+ `debate`（争议条款正反方）+ `voting`（风险定级） | `weighted_voting`（高风险条款权重上浮）+ `debate`（融合双面观点） | `expert-law`（条款合法性）、`expert-finance`（支付/履约财务风险）、`expert-research`（行业惯例与判例） | 网关:3080 → 调度:3100（匹配三专家 + DAG 并行/辩论节点）→ 执行:3200（并行执行 + 辩论融合）→ `persist_fusion_output` 持久化风险清单 | 并行/辩论/加权投票框架已实现；风险识别准确率与召回率**待配置 `MOX_LLM_*` 后验证** |
| 3 | 舆情监测与研判分析 | `parallel`（多平台/多媒体分片并行）+ `map_reduce`（分片归因聚合）+ `hierarchical`（初筛→研判→定级） | `map_reduce`（分片归并）+ `confidence_weighting`（按信源置信度加权） | `expert-vision`（图片/视频舆情识别）、`expert-translation`（涉外多语种信源）、`expert-research`（事件背景研判） | 网关:3080 → 调度:3100（任务分片规划）→ 执行:3200（Map 节点并行 + Reduce 节点聚合）→ `read_back` 回读研判结论 | MapReduce 融合与多专家并行调度已实现；多媒体理解与涉外研判质量**待配置 `MOX_LLM_*` 后验证** |
| 4 | 财务分析与风险报告 | `sequential`（指标计算→解读→对标→成稿流水线）+ `hierarchical`（分析师→复核→签发） | `stacking`（计算层/解释层/结论层分层堆叠）+ `confidence_weighting` | `expert-math`（量化指标计算）、`expert-finance`（财务解读）、`expert-research`（行业对标）、`expert-creative`（报告成稿） | 网关:3080 → 调度:3100（串行 DAG 规划）→ 执行:3200（逐层堆叠融合）→ `state_sink` 持久化各层中间结果 → `read_fusion_output` 取最终报告 | 串行 DAG 与 stacking 融合代码已实现；财务结论的专业准确性**待配置 `MOX_LLM_*` 后验证** |
| 5 | 知识库智能问答（RAG） | `parallel`（多路召回并行）+ `map_reduce`（召回片段归并）+ `voting`（答案一致性校验） | `rrf`（多路召回融合）+ `confidence_weighting`（按证据置信度加权）+ `voting` | `expert-research`（检索与综合）、`expert-code`（知识库查询/工具调用）、`expert-translation`（跨语种问答） | 网关:3080 → 调度:3100（匹配检索类专家）→ 执行:3200（并行召回 + RRF 融合 + 投票校验）→ `read_fusion_output` 回读答案与引用 | RRF/加权/投票融合函数已实现；幻觉率与引用准确性**待配置 `MOX_LLM_*` 后验证** |
| 6 | 工单智能分诊与客服 | `sequential`（意图识别→分诊→应答→回写）+ `hierarchical`（一线→专家→主管升级） | `confidence_weighting`（低置信自动升级人工）+ `best_of`（多应答候选选优） | `expert-research`（客服知识库检索）、`expert-code`（工单系统查询/回写）、`expert-translation`（多语种客服） | 网关:3080 → 调度:3100（提交 + 幂等键防重）→ 执行:3200（分诊置信度判定 + 升级分支）→ `persist_fusion_output` 持久化工单处理记录 | 幂等提交、状态机升级分支、best_of 融合已实现；分诊准确率与应答质量**待配置 `MOX_LLM_*` 后验证** |
| 7 | 政策解读与研究报告 | `parallel`（多政策文本并行解读）+ `iterative`（解读→征询→修订）+ `debate`（政策影响正反方） | `stacking`（条文解读层/影响分析层/建议层分层）+ `iterative_refinement` | `expert-research`（政策文本梳理）、`expert-law`（合法性审查）、`expert-finance`（财政/经济影响）、`expert-creative`（研究报告撰写） | 网关:3080 → 调度:3100（多专家 DAG + 迭代节点规划）→ 执行:3200（堆叠融合 + 多轮精炼）→ `read_back` 回读各轮修订稿 | 迭代精炼与堆叠融合已实现；政策解读的专业深度与中立性**待配置 `MOX_LLM_*` 后验证** |

---

## 三、逐场景选型说明

### 1. 政企公文起草与合规审查
公文生产天然是"起草→核稿→签发"的层级流水线，且需要多轮打磨，因此选 `hierarchical` + `iterative`。`iterative_refinement` 对应反复核改的实际工作流，`confidence_weighting` 让法律合规意见在融合时权重高于初稿，避免"写作自由"压过合规底线。

### 2. 合同智能审查与风险提示
合同风险可从合法性、财务、行业惯例三个维度**并行**切入以提升召回；对争议条款用 `debate` 让正反意见同时暴露，最后 `weighted_voting` 汇总风险等级——高风险条款（如付款、违约）权重上浮，避免被多数低风险条款稀释。

### 3. 舆情监测与研判分析
舆情数据天然按平台/语种/媒体类型分片，`map_reduce` 与 `parallel` 直接对应"分片抓取—并行分析—归并研判"。视觉专家负责图片视频、翻译专家负责涉外信源，结论按信源置信度加权，降低低质信源噪声。

### 4. 财务分析与风险报告
财务报告是典型串行流水线：数学计算 → 金融解读 → 行业对标 → 成稿，层次清晰故用 `sequential` + `hierarchical`。`stacking` 融合适合把"计算层结果"与"解释层结果"分层汇总，保证最终报告既含数字又含结论。

### 5. 知识库智能问答（RAG）
RAG 的工程本质是多路召回 + 片段归并，`rrf`（Reciprocal Rank Fusion）与 `map_reduce` 是行业标准做法；答案生成后用 `voting` 做一致性校验，可在不增加检索成本的前提下压低幻觉率。

### 6. 工单智能分诊与客服
工单流程是标准的层级升级链路（一线坐席→专家→主管），分诊置信度低时必须自动升级，故 `hierarchical` + `confidence_weighting`。`best_of` 用于从多个应答候选中挑出最贴合工单上下文的回复，提升首次解决率。

### 7. 政策解读与研究报告
政策解读需跨领域分层输出：条文解读、影响分析、对策建议，适合 `stacking`；对存在争议的政策影响用 `debate` 呈现正反两面，避免单一视角；最后 `iterative_refinement` 把散点观点打磨成结构完整的研究报告。

---

## 四、能力就绪度总览

### 🟢 已实现框架（不依赖真实模型即可走通编排链路）
- 6 种协作模式的 DAG 编排（sequential / parallel / debate / hierarchical / iterative / voting）
- 6 种融合策略 + 6 个兼容融合基础函数（rrf / weighted / voting / best_of / concatenate / merge_json）
- 10 个内置专家定义（`mox-alliance-config-core/src/examples/domain_experts.rs`）
- 任务提交（`idempotency_key` 幂等）、状态机仲裁（`can_transition_to`）
- 任务 CRUD、`cancel` / `pause` / `resume` / `retry` / `skip_node` 干预
- 执行状态回读（`read_back`）、融合输出持久化与回读（`persist_fusion_output` / `read_fusion_output`）
- 专家搜索（`search_experts`，按能力/领域过滤）
- 网关 `:3080` → SDK → 调度 `:3100` → 执行 `:3200` 全链路与租户身份头贯穿
- 注册中心 `:3400` 专家注册 / 发现 / 心跳
- 多活租约选主（HA，默认关闭）

### 🟡 待真实模型验证（代码已写，端到端效果待 `MOX_LLM_*` 配置后验证）
- 上述 7 个政企场景的端到端推理质量与产出可用性
- 10 个内置专家在真实 LLM 下的领域专业度
- `debate` / `voting` 模式下多观点碰撞与收敛的真实表现
- `iterative_refinement` 多轮打磨的收敛速度与终稿质量
- 无模型时执行器返回 `MODEL_NOT_CONFIGURED` / `ExecutorUnavailable (4002)` —— **预期行为**，非缺陷

### 🔴 未在本矩阵中主张（超出本文档编制依据，需以代码为准另行核实）
- 实时 WebSocket 进度推送、报告 PDF/Excel 导出、专家版本灰度发布、工具自动注册（gRPC 反射）、工作记忆/长期记忆等——上述能力**未出现在本文档采信的代码事实清单中**，本文档不将其作为"已就绪"能力主张，后续如需纳入须先核实代码实现再更新本矩阵。

---

> 维护说明：本矩阵随代码实现演进更新。新增协作模式 / 融合策略 / 内置专家或场景落地时，须同步修订本表与"就绪度总览"，并保持"待 `MOX_LLM_*` 配置后验证"的诚实标注。

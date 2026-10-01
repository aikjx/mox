# 专家联盟 · 政企场景应用矩阵

> **标题**：专家联盟政企场景应用矩阵
> **版本**：V1.0 ｜ **权威等级**：🟡参考 ｜ **编号**：EA-SCEN-001
> **最后更新日期**：2026-09-24 ｜ **主责联盟**：开发联盟 R
> **单源声明**：本文档映射"真实政企场景 → 联盟能力"，能力事实以 `platform/domains/alliance/` 代码为准；流程拓扑见 `docs/expert-alliance/EA-DIAG-001-专家联盟架构与业务处理流程图-V1.0.html`。

---

## 一、场景 × 能力矩阵

**图例**：协作模式 = 6 种（sequential/parallel/debate/hierarchical/iterative/voting）；融合策略 = 6 种 trait（weighted_voting/confidence_weighting/stacking/debate/map_reduce/iterative_refinement）；专家 = 11 位内置（expert-code/code-engine/math/medical/law/finance/creative/vision/translation/research/arch）。

| 序号 | 政企场景 | 目标产出 | 推荐协作模式 | 推荐融合策略 | 参与专家 | 调用链路 | 实现状态 |
|:--:|------|------|------|------|------|------|------|
| S1 | 政企公文起草与合规审查 | 公文初稿 + 合规要点清单 + 修改建议 | hierarchical + debate | debate / weighted_voting | expert-law、expert-creative、expert-research | 网关 /api/alliance/tasks → 调度3100（匹配+规划）→ 执行3200（DAG+融合）→ 结果回读 | ✅ 链路已实现；🔶 真实模型端到端待配置 `MOX_LLM_*` 后验证 |
| S2 | 合同智能审查 | 风险条款定位、合规分析、修订建议 | sequential + debate | debate / stacking | expert-law、expert-finance | 同上 | ✅ 链路已实现；🔶 待真实模型验证 |
| S3 | 舆情监测与研判 | 事件脉络、态度分布、研判结论 | parallel + voting | weighted_voting / confidence_weighting | expert-research、expert-finance、expert-arch | 同上 | ✅ 链路已实现；🔶 待真实模型验证 |
| S4 | 财务分析与风险报告 | 指标解读、异常归因、风险报告 | hierarchical + iterative | confidence_weighting / iterative_refinement | expert-finance、expert-math、expert-research | 同上 | ✅ 链路已实现；🔶 待真实模型验证 |
| S5 | 政企知识库智能问答 | 基于私有知识库的溯源问答 | parallel | weighted_voting / rrf | expert-research、expert-law、expert-translation | 同上（可接入知识检索） | ✅ 链路已实现；🔶 知识检索与真实模型待验证 |
| S6 | 工单智能分诊与客服 | 工单分类、优先级建议、答复草稿 | sequential | weighted_voting / best_of | expert-research、expert-law、expert-creative | 同上 | ✅ 链路已实现；🔶 待真实模型验证 |
| S7 | 政策解读与研究报告 | 政策要点拆解、影响分析、研报 | hierarchical + iterative | iterative_refinement / map_reduce | expert-law、expert-research、expert-translation | 同上 | ✅ 链路已实现；🔶 待真实模型验证 |
| S8 | 政企 IT 项目代码审查与架构评估 | 代码缺陷清单、架构风险、整改建议 | debate + voting | debate / weighted_voting | expert-code、expert-code-engine、expert-arch | 同上（code-engine 本地推理可离线降级） | ✅ 链路已实现（含 code-engine 本地推理降级链）；🔶 云端模型待验证 |

> 状态图例：✅ = 联盟域能力与链路已落地并有测试证据；🔶 = 依赖真实 LLM 的运行结果，需配置 `MOX_LLM_API_KEY/BASE_URL/MODEL`（或既有提供者）后验证，未配置时如实返回 `MODEL_NOT_CONFIGURED`，不伪造空报告（仓库诚实边界铁律）。

## 二、场景到能力的支撑证据

| 场景依赖的能力 | 代码落点 | 验证证据 |
|------|------|------|
| 任务提交/幂等回放/状态机仲裁 | scheduler-core（submit_task、can_transition_to、bind_idempotency_key） | 单元测试（scheduler-core 99 项）+ 集成 e2e |
| 专家匹配 Top-N | ModularWeightMatcher（唯一权威路径） | 匹配器测试归一化 |
| 6 模式 DAG 规划 + 计划验证 | scheduler-core dag 规划 + 无环/可达验证 | 模式差异化 DAG 测试 |
| DAG 执行 + 节点容错（超时/重试/替代/降级） | executor-core | 执行器集成测试（含未配置模型拒绝执行） |
| 6 策略融合 + 兼容函数 | mox-alliance-core/fusion | 融合引擎测试 |
| 执行状态/融合输出持久化回读 | state_sink + 融合输出持久化 | 跨重启读回测试 |
| 专家注册/发现/心跳/摘除 | registry-svc :3400（三存储后端） | 15 项测试（8 单元 + 7 集成） |
| 租户身份贯穿 | http-sdk RequestContext + 出站头注入 | 网关 19 路由处理器 + http-sdk 测试 |
| 多活租约选主/故障接管 | scheduler-core 同库租约 | 选主/接管测试 |
| 配置中心/命名注册 | boot-config（nacos,naming feature） | 真实 rnacos e2e（--ignored） |

## 三、一个端到端场景示例（S2 合同智能审查）

```
用户提交："审查这份采购合同的风险条款"
  → 网关 :3080 提取 X-Tenant-Id → http-sdk 注入请求上下文
  → 调度 :3100：幂等键检查（无重复）→ ModularWeightMatcher 匹配 expert-law + expert-finance
  → 规划 DAG（sequential + debate：先各自审查、再辩论收敛）
  → 执行 :3200：节点并行调用两位专家 → debate 融合 → QA
  → state_sink 持久化 → 结果回读：风险条款清单 + 置信度 + 各专家原始结果
  → 前端工作台展示（专家正文 + 自评分）
```

---

**变更记录**：V1.0（2026-09-24）首发：8 个政企场景 × 能力矩阵，能力全部对齐代码事实。

*版权所有 © 2026 璇玑 RelGraph · 三联盟｜开源协议 MIT*

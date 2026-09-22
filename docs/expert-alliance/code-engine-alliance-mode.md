# 自研 AI 代码引擎 · 开发专家联盟处理模式（CODEENGINE-ALLIANCE-V1.0）

> 权威源：本文件描述 `mox-codeengine-core` / `mox-codeengine-svc` 两 crate 的设计与契约。
> 联盟总体架构见 [CURRENT-ARCHITECTURE.md](./CURRENT-ARCHITECTURE.md)；处理模式定义见 [README.md](./README.md)。

## 1. 定位

**全自研**（zero external AI dependency）的端到端代码引擎：自然语言需求 → 可交付代码工程。
全部推理为确定性自研算法——规则 IR 解析器、四角色专家联盟、加权裁决、三证守恒闸门；
不依赖任何外部 LLM API，离线可跑、结果可复现、链路可审计。

| crate | 层 | 路径 | 职责 |
|---|---|---|---|
| `mox-codeengine-core` | L5 core | `platform/domains/ai/core/mox-codeengine-core` | 引擎本体（纯计算，复用 `mox-ai-flow-core` 求解与出码） |
| `mox-codeengine-svc` | L4 svc | `platform/domains/ai/svc/mox-codeengine-svc` | HTTP 服务 `:3210`（`MOX_CODEENGINE_PORT` 覆盖） |

## 2. 十阶段处理模式（对齐联盟语义）

```
Intent → Build → Solve → Team → Diagnose → Verdict → Generate → Gate → Deliver → Learn
  IR归一   建图    求解   组队    并行诊断    加权裁决    裁决后出码  三证闸门   交付     案例沉淀
```

每阶段产出一条 `StageEvent{stage, ok, note}`，全轨迹随 `EngineResult` 返回（可审计）。

- **Intent**（`ir.rs`）：中文/英文需求句法切分 → 步骤 + 工具类别推断 + 资源读写抽取 + 敏感资源打标。
- **Build**（`graph.rs`）：IR → `FlowGraph`；敏感步骤前自动注入 `desensitize` 护栏、Shell 步骤注入 `path_check` 护栏。
- **Solve**（复用 `mox-ai-flow-core::pipeline::optimize`）：数据流并行化 → 冲突检测 → 自动修复 → CPM/RCPSP 排程 → 算力分级路由。
- **Team/Diagnose**（`experts.rs`）：四角色专家独立出具意见（`Finding{code,severity,suggestion,confidence}`）：
  analyst（结构完整性）/ builder（可优化性）/ auditor（安全合规）/ coordinator（协作质量）。
- **Verdict**（`verdict.rs`）：加权投票融合（analyst .25 / builder .20 / auditor .35 / coordinator .20，threshold .75）；
  任一专家 Blocking 意见禁止出码，auditor 的 Blocking 另记一票否决（vetoed）。
- **Generate**：仅在裁决通过后调用 `codegen::generate`（C1 红线：未审不过不得生成）。
- **Gate**（`gate.rs`）三证守恒：
  1. 拓扑守恒——优化图保留全部可执行节点；
  2. 数据守恒——读写资源集合前后不变；
  3. 往返守恒——生成代码覆盖每个节点且可被 `reverse_from_python` 重建。
- **Deliver**：`delivered = approved && gates.passed`。
- **Learn**（`case.rs`）：`CodeCase` 入 `CaseBook`，导出 JSONL，交付率作为后续调权学习信号。

## 3. HTTP 契约

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/api/codeengine/run` | body `{text, id?, name?, auto_guard?, threshold?, with_bundle?}` → 全量裁决/闸门/事件/代码包 |
| GET | `/api/codeengine/health` | 引擎名、处理模式、十阶段与四专家阵容 |
| GET | `/api/codeengine/cases` | Learn 案例库与交付率 |

## 4. 联盟接入

引擎以专家 **`expert-code-engine`**（`code-engine-001` · 自研AI代码引擎）注册于
`mox-alliance-config-core::examples::domain_experts::build_domain_experts()`，
tags `code / codegen / self-hosted / programming`，provider `mox-selfhosted` 指向
`http://127.0.0.1:3210/api/codeengine`，零 token 成本、回退链 deepseek → openai-code。
调度器（`:3100`）启动时经 `build_domain_experts()` 种子自动纳入专家匹配。

## 5. 验证

- 单测：core 18 项 + svc 冒烟 1 项 + doctest 1 项全绿；
- 端口门禁：`3210` 已登记 `docs/api/PORT-REGISTRY.md` §3.2 与 `scripts/verify-ports.py` CANONICAL（ERROR=0）；
- 否决路径：敏感资源 + `auto_guard=false` → auditor 一票否决、不出码、不交付（测试覆盖）。

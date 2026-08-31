# 企业级业务处理流程（Business Process Flows）

>
> **标题**：企业级业务处理流程
> **版本**：V1.0
> **权威等级**：🟢权威（流程规范主文档）
> **编号**：EA-DOC-058
> **文档层级**：L4流程标准层
> **最后更新日期**：2026-08-31
> **主责联盟**：开发联盟 R
> **单源声明**：本文档是企业级业务处理流程的唯一权威承载。冲突时以 `docs/enterprise/18-全域顶层总设计-三联盟模式-V1.0.md` 为准。
>
本文档描述算子统一系统（OUS）中**已落地**的企业级业务处理流程能力，以代码为准（`crates/ai-agent/src/workflow_engine.rs`、`crates/ai-agent/src/flow_engine.rs`、`crates/runtime/src/main.rs`、`frontend/`）。
>
> 本文与 `docs/architecture.md` §9「13 条业务处理流程卡」（按端点划分的**设计规范**）互补：§9 是"系统对外提供哪些业务流程端点"，本文是"企业级业务流程如何被编排、执行与复用"。两者不冲突。

---

## 1. 概述

企业级业务处理流程由两套互补引擎承载：

| 引擎 | 模块 | 入口端点 | 定位 |
|------|------|----------|------|
| **业务流程引擎** `WorkflowEngine` | `crates/ai-agent/src/workflow_engine.rs` | `POST /api/ai/workflows/execute` | BPMN 风格、节点编排、条件分支、企业流程模板 |
| **可视化流程图引擎** `FlowEngine` | `crates/ai-agent/src/flow_engine.rs` | `POST /api/ai/flows`（`create_flow`/`validate_flow`/`execute_flow`） | Three.js 画布、DAG、真实 LLM/Browser/HTTP 节点 |

两者都基于有向图 + BFS/拓扑排序执行，差异在于：

- `WorkflowEngine` 面向**业务语义节点**（`AiTask`/`Operator`/`Condition`/`PluginCall`），强调"业务流程化"，适合企业审批/核验/会签类场景，内置企业级流程模板。
- `FlowEngine` 面向**技术执行节点**（`LLM`/`Browser`/`HttpRequest`），强调"可视化编排与实时渲染"，适合 RPA/自动化执行类场景。

---

## 2. 业务流程引擎核心技术架构

### 2.1 执行模型

`WorkflowEngine::execute_business_workflow` 的执行流程：

1. 加载 `BusinessWorkflow`（来自模板 `create_from_template` 或前端提交的 `saveWorkflow`）。
2. 以 `start` 节点为根，按拓扑关系做 **BFS 逐层执行**（支持顺序、条件分支、并行合并、子流程、用户任务）。
3. 每个节点调用 `execute_node` 产生 `NodeExecutionRecord`（`status`/`output`/`error`）。
4. 汇总为 `WorkflowResult`：`completed_nodes`/`failed_nodes`/`total_nodes` 计数 + `operators_called`/`plugins_called`/`parallel_branches` 指标。

### 2.2 节点类型与真实执行能力

| 节点类型 | 配置变体 | 真实执行方式 | 备注 |
|----------|----------|--------------|------|
| `Start` | `WorkflowNodeConfig::Start` | 标记流程开始，注入时间戳 | — |
| `End` | `WorkflowNodeConfig::End` | 输出最终变量快照 | — |
| `Script` | `Script { language, code }` | **预留未接入**：返回 `status:"pending"`，不假装执行 | 沙箱（WASM/进程隔离）为后续路线图 |
| `AiTask` | `AiTask { task_type, prompt }` | 若 `LLMClient` 已注入且 `is_enabled()`：对 `prompt` 做 `${var}` 模板替换 → 调用 `LLMClient::chat` → 返回真实 LLM 输出，**输出若为 JSON 对象则自动展开为流程变量**（AI→变量闭环）；否则降级 `status:"simulated"` + `simulated:true` 标记 | LLM 需配置可用 API Key 才真实执行 |
| `Operator` | `Operator { operator_id, parameters }` | 通过 HTTP `POST {OPERATOR_API_BASE:-http://127.0.0.1:3998}/operators/{operator_id}` 调用**真实已注册算子端点**，返回 HTTP 状态 + body；**30s 超时**，失败区分「超时/连接失败」并附 URL 便于排障 | 与 runtime 服务同源 |
| `Condition` | `Condition { expression, true_path, false_path }` | **真实表达式求值**：支持 `${var}` 引用、`==/!=/>/</>=/<=`、`&&`/`||`、顶层括号；执行循环按 `result` **只路由 `true_path`/`false_path` 之一**（通过/拒绝互斥）；变量未定义时 fail-closed 按 `false`（走拒绝路径），语法错误仍报错；**输出补充 `matched_branch` 与 `referenced_variables`**（命中分支名 + 被引用变量实际取值，便于审计） | 不再硬编码 `true`，不再两条分支同时执行 |
| `PluginCall` | `PluginCall { plugin_id, method, parameters }` | 通过 HTTP `GET {PLUGIN_API_BASE:-http://127.0.0.1:3998}/plugins/{plugin_id}/{method}` 调用**真实插件总线端点**；**30s 超时**，失败区分「超时/连接失败」；失败返回明确错误 | — |
| `Parallel` | `Parallel { branches, merge_strategy }` | 并行分支占位 + 合并策略枚举（`AllComplete`/`AnyComplete`/`FirstSuccess`/`VoteMajority`） | 分支树为后续增强 |
| `SubWorkflow` | `SubWorkflow { workflow_id }` | 子流程调用占位 | — |
| `UserTask` | `UserTask { assignee, form }` | 挂起待人工处理（返回 `user_task_pending`） | 人工审批接入点 |
| `Delay` | `Delay { duration_ms }` | `tokio::time::sleep` 真实延时 | — |

> 关键修正（2026-08 补完）：此前 `AiTask`/`Operator`/`Condition`/`PluginCall` 全部返回模拟结果。现已接入真实 LLM 句柄（`AIAgent::new` 通过 `WorkflowEngine::new_with_llm` 注入）与真实 HTTP 调用；并补全 **AI→变量→条件分支** 闭环（见 §2.3），企业流程可真实跑通"审查 → 判定 → 通过/拒绝"业务逻辑。

### 2.3 变量与模板

- 节点间数据通过 `HashMap<String, serde_json::Value>` 的 `variables` 传递。
- `AiTask.prompt` 支持 `${var}` 占位符，执行前经 `apply_template` 替换为当前变量值。
- `Condition.expression` 支持 `${var}` 引用，经 `eval_condition` 求值。
- **AI→变量闭环**：`AiTask` 真实执行后，若 LLM 返回 JSON 对象（如 `{"verify_pass": true, "reason": "…"}`），其键自动展开到节点输出，并经「合并输出到变量」写入 `instance.variables`——因此 `Condition` 的 `${verify_pass}` 可由真实 AI 判定驱动。
- **条件路由**：`Condition` 执行后按 `result` 只入队 `true_path`/`false_path` 指向的节点（`finance-invoice-verify` 中 `fi-ok`/`fi-risk` 互斥，不会同时执行）。
- **fail-closed**：条件引用的变量未定义时（典型场景：LLM 未配置导致 `AiTask` 降级模拟），`resolve_value` 返回 `Null` 哨兵——等值比较为 `false`，排序比较（`>`/`<` 等）也一律 `false`（不会退化为字符串比较导致 `${loss} < 0.001` 误判），流程走拒绝/风险路径继续执行，而非整体失败；表达式语法错误（无比较符、非法操作符）仍会显式报错，避免掩盖配置错误。
- **跨类型比较**：`==/!=` 支持数值协调（`1 == "1"`、`1 == true` 均等价），纯文本则按字符串严格相等；排序比较（`>/</>=/<=`）优先数值、退化为字典序。避免「数字写成字符串」导致的判定偏差。

---

## 3. 接口清单（真实端点）

来自 `crates/runtime/src/main.rs`：

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/ai/workflows/execute` | 执行业务工作流（`execute_business_workflow`） |
| GET | `/api/ai/workflows/templates` | 列出内置 + 已注册流程模板（含 6 个企业模板） |
| GET | `/api/ai/workflows` | 列出工作流定义 |
| GET | `/api/ai/workflows/instances` | 列出运行中的工作流实例 |

> 前端通过 `frontend/src/api/index.js` 的 `getWorkflowTemplates` / `executeWorkflowDef` / `saveWorkflow` 对接。

---

## 4. 内置流程模板

通过 `WorkflowEngine::register_builtin_templates` 注册。当前共 **11 个**模板（技术类 5 + 企业类 6）：

### 4.1 技术类（`category` 非 enterprise）

| id | name | 说明 |
|----|------|------|
| `data-pipeline` | 数据处理管道 | 加载→清洗→归一化→输出 |
| `nn-training` | 神经网络训练 | 数据准备→建模→训练→评估 |
| `algorithm-analysis` | 算法分析 | 模式识别→流程图生成→算子映射→并行优化 |
| `chat-response` | 对话响应 | 意图识别→对话生成 |
| `plugin-collaboration` | 多插件协作 | 任务分发→并行/单插件调用→合并 |

### 4.2 企业级业务处理流程（`category: "enterprise"`）

编排范式统一为：**开始 → AI 审查/核验（或算子执行）→ 条件分支 → 结束（合规/风险）**。

| id | name | 节点编排 | 业务含义 |
|----|------|----------|----------|
| `finance-invoice-verify` | 财务发票核验 | Start → AiTask(发票核验) → Condition(`${verify_pass}==true`) → End(合规通过)/End(标记风险) | AI 核验发票要素与税务风险 |
| `hr-onboarding` | 人事入职审批 | Start → Operator(创建账号权限) → AiTask(资料完整性审查) → Condition(`${profile_complete}==true`) → End(入职完成)/End(退回补充) | 入职账号开通 + 资料审查 |
| `procurement-apply` | 采购申请审批 | Start → AiTask(预算合规检查) → Condition(`${over_budget}==true`) → End(转人工审批)/End(自动通过) | 预算合规与超预算分流 |
| `expense-reimburse` | 报销审批 | Start → AiTask(票据合规审查) → Condition(`${compliant}==true`) → End(批准报销)/End(驳回) | 票据真实性与合规审查 |
| `contract-countersign` | 合同会签 | Start → Operator(发起会签) → AiTask(条款风险审查) → Condition(`${risk_low}==true`) → End(签署生效)/End(退回修改) | 会签发起 + 条款风险审查 |
| `legal-compliance-review` | 法务合规审查 | Start → AiTask(合规风险审查) → Condition(`${compliant}==true`) → End(合规通过)/End(标记风险) | 合规风险审查 |

> 说明：企业模板的 `Condition` 分支变量（如 `${verify_pass}`/`${profile_complete}`）由上游 `AiTask` 的执行结果驱动——LLM 真实执行时返回 JSON 对象即自动展开为 `variables`（AI→变量闭环，见 §2.3）；未配置 LLM 时 `AiTask` 降级模拟、分支变量缺失，`Condition` 按 fail-closed 走拒绝路径，流程仍可完成（安全默认拒绝）。模板 `plugin-collaboration` 的分发条件已修正为 `${needs_parallel} == true`（原裸标识符 `needs_parallel` 无法解析，导致模板恒失败）。

---

## 5. 企业级流程场景规划（目标场景）

以下场景建议在现有引擎上以"新增 `WorkflowTemplate`"方式扩展（参见 §7）。它们是**目标场景**，标注当前落地状态：

| 场景 | 建议节点编排 | 状态 |
|------|--------------|------|
| 财务发票核验 | 见 §4.2 模板（已内置） | ✅ 模板已落地 |
| 人事入职审批 | 见 §4.2 模板（已内置） | ✅ 模板已落地 |
| 采购申请审批 | 见 §4.2 模板（已内置） | ✅ 模板已落地 |
| 报销审批 | 见 §4.2 模板（已内置） | ✅ 模板已落地 |
| 合同会签 | 见 §4.2 模板（已内置） | ✅ 模板已落地 |
| 法务合规审查 | 见 §4.2 模板（已内置） | ✅ 模板已落地 |
| 采购订单履约跟踪 | Start → Operator(查询订单) → AiTask(履约风险预测) → Condition → End | 📋 待扩展模板 |
| 客户 onboarding KYC | Start → Operator(身份核验) → AiTask(风险评级) → Condition → End | 📋 待扩展模板 |
| 工单自动分派 | Start → AiTask(意图分类) → Condition(优先级) → Operator(分派) → End | 📋 待扩展模板 |

---

## 6. 前端对接

`frontend/src/api/index.js` 提供：

- `getWorkflowTemplates()` → `GET /api/ai/workflows/templates`
- `saveWorkflow(def)` → 提交/保存 `BusinessWorkflow` 定义
- `executeWorkflowDef(def)` → `POST /api/ai/workflows/execute`
- 可视化设计器位于 `frontend/src/views/`，画布节点体系与 `WorkflowNodeType` 对齐。

---

## 7. 扩展指南：新增一个企业流程模板

在 `crates/ai-agent/src/workflow_engine.rs` 的 `register_builtin_templates` 中追加：

```rust
self.templates.register(WorkflowTemplate {
    id: "your-biz-flow".to_string(),
    name: "你的业务流".to_string(),
    description: "一句话描述".to_string(),
    category: "enterprise".to_string(),
    nodes: vec![
        WorkflowNode { id: "start".to_string(), node_type: WorkflowNodeType::Start,
            name: "开始".to_string(), config: WorkflowNodeConfig::Start,
            position: Some(NodePosition { x: 30.0, y: 200.0 }) },
        WorkflowNode { id: "ai".to_string(), node_type: WorkflowNodeType::AiTask,
            name: "AI审查".to_string(),
            config: WorkflowNodeConfig::AiTask { task_type: "your_review".to_string(),
                prompt: "审查：${input}".to_string() },
            position: Some(NodePosition { x: 200.0, y: 200.0 }) },
        WorkflowNode { id: "cond".to_string(), node_type: WorkflowNodeType::Condition,
            name: "判定".to_string(),
            config: WorkflowNodeConfig::Condition { expression: "${pass} == true".to_string(),
                true_path: "ok".to_string(), false_path: "flag".to_string() },
            position: Some(NodePosition { x: 400.0, y: 200.0 }) },
        WorkflowNode { id: "ok".to_string(), node_type: WorkflowNodeType::End,
            name: "通过".to_string(), config: WorkflowNodeConfig::End,
            position: Some(NodePosition { x: 600.0, y: 120.0 }) },
        WorkflowNode { id: "flag".to_string(), node_type: WorkflowNodeType::End,
            name: "标记".to_string(), config: WorkflowNodeConfig::End,
            position: Some(NodePosition { x: 600.0, y: 280.0 }) },
    ],
    connections: vec![
        WorkflowConnection { from: "start".to_string(), to: "ai".to_string(), label: None },
        WorkflowConnection { from: "ai".to_string(), to: "cond".to_string(), label: None },
        WorkflowConnection { from: "cond".to_string(), to: "ok".to_string(), label: Some("通过".to_string()) },
        WorkflowConnection { from: "cond".to_string(), to: "flag".to_string(), label: Some("风险".to_string()) },
    ],
    variables: HashMap::new(),
});
```

运行时通过 `AIAgent::new()` 自动注入 LLM 句柄，无需额外配置即可让 `AiTask` 真实执行（前提是 LLM API Key 可用）。

---

## 8. 已知限制与路线图

| 项 | 现状 | 路线图 |
|----|------|--------|
| `Script` 节点 | 返回 `pending`，未接入沙箱 | 接入 WASM/进程隔离沙箱 |
| `Parallel` 分支 | 枚举与占位就绪，分支树执行待增强 | 实现真并行子图执行 |
| `SubWorkflow` | 占位 | 支持嵌套工作流调用 |
| `Operator`/`PluginCall` | **已实现（2026-08-16）**：真实 HTTP 调用，30s 超时，失败区分「超时/连接失败」并附 URL 便于排障 | 与算子注册表强校验、增加重试 |
| ~~`AiTask` 变量回写~~ | **已实现（2026-08）**：LLM 输出 JSON 对象自动展开为 `variables`，驱动 `Condition` 分支（AI→变量→条件分支闭环） | 支持任意深度嵌套 JSON 展开、输出 schema 约束 |
| `Condition` 分支 | **已实现**：按 `result` 只路由 `true_path`/`false_path` 之一；变量未定义 fail-closed 走拒绝路径；输出含 `matched_branch`+`referenced_variables` 审计字段；比较支持跨类型数值等价 | 多路分支（switch）、分支计数统计 |
| LLM 真实执行 | 需配置可用 API Key；未配置时降级 `simulated` | 配置注入与失败可观测 |
| 持久化 | 工作流实例为内存态（`WorkflowInstance` 在 `running_instances`） | 持久化到 `$OUS_HOME/workflows`（见 `docs/architecture.md` §8） |
| 人工审批 `UserTask` | 挂起占位 | 接入工单/审批中心回调 |

---

## 9. 与架构文档的关系

- `docs/architecture.md` §9「13 条业务处理流程卡」：按**端点**划分的系统级业务流程设计规范（算子注册、执行、对话、工作流编排、浏览器自动化等），是"系统对外提供哪些业务流程"。
- `docs/architecture.md` §28「业务流程设计模块」：设计态模块（Three.js 画布、DSL、校验、版本化、模板市场）。
- 本文：聚焦于**企业级业务处理流程的编排与执行能力**，即 `WorkflowEngine` + 6 个企业模板的真实落地情况。

三者共同构成 OUS 的"业务流程化"能力栈：设计（§28）→ 规范（§9）→ 执行（本文）。

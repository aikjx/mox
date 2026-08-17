# 企业级业务处理流程图（Business Process Flowcharts）

> 配套文档：`architecture.md`（§9 业务处理流程卡、§28 业务流程设计模块）、`business-process-flows.md`（企业级流程执行引擎）。
> 本文用 **Mermaid 流程图/时序图** 把"企业级处理业务流程"可视化，便于评审、与代码对齐、以及对客演示。
> 所有端点以 `crates/runtime/src/main.rs` 真实路由为准，所有流程以 `crates/ai-agent/src/workflow_engine.rs` / `flow_engine.rs` 真实实现为准。

---

## 0. 图例（统一符号）

| 图形 | 含义 |
|------|------|
| `([开始])` / `([结束])` | 流程起止 |
| `[处理节点]` | 算子执行 / AI 审查 / 算子调用 |
| `{条件分支}` | `Condition` 节点，按 `${var}` 求值路由 |
| `{{专家/治理}}` | 璇玑 / 璇玑 / 治理复核 |
| `>人工任务]` | `UserTask` 挂起待审批 |
| `[[状态向量/图谱]]` | 守恒校验 / 知识沉淀 |
| `---` / `===` | 成功路径 / 异常/回滚路径 |

---

## 1. 统一业务处理流水线（所有请求的通用状态机）

任何业务请求都流经标准阶段，并遵循统一状态机（与 `architecture.md` §5/§9 对齐）。

```mermaid
stateDiagram-v2
    [*] --> Pending: 接入 /api/* (鉴权+租户路由)
    Pending --> Running: 意图归一化 → 状态向量投影(公理2)
    Running --> Completed: 编排决策(单算子/FLOW/SUPER_EXPERT)\n+ 执行 + 守恒校验(公理6) + 出参\n+ 沉淀 session/event + 图谱加权边
    Running --> WaitingUser: 人工审批节点(UserTask)
    WaitingUser --> Running: 审批回调
    Running --> Failed: 异常(超时/类型不兼容/守恒残差超阈)
    Failed --> Retry: 指数退避 ≤3
    Retry --> Running: 重试
    Failed --> Rollback: 守恒律回滚(状态向量回到 Turn 前)
    Rollback --> [*]: 记录审计
    Completed --> [*]
```

**阶段映射（端点级）：**

```mermaid
flowchart TD
    A([客户端/门户/CLI]) --> B[API 网关\n鉴权 Authorization: Bearer\n限流 + trace_id 注入]
    B --> C{编排决策}
    C -->|单算子| D[POST /api/execute\noperator-wasm 沙箱执行]
    C -->|可视化流| E[POST /api/ai/flows/execute\nFlowEngine DAG 驱动]
    C -->|业务工作流| F[POST /api/ai/workflows/execute\nWorkflowEngine 节点编排]
    C -->|全维求解| G[SUPER_EXPERT 调度中枢 §19]
    D --> H[[守恒校验 conservation::check_all]]
    E --> H
    F --> H
    G --> H
    H -->|通过| I[session/event 追加\n图谱加权边沉淀]
    H -->|残差超阈| J[回滚至 Turn 前 + 告警]
    I --> K([结构化响应 / SSE 流 / 工作流实例])
    J --> K
```

---

## 2. 企业级业务处理流程模板（6 个已落地模板）

编排范式统一为：**开始 → AI 审查/算子执行 → 条件分支 → 结束（合规/风险）**。
`AiTask` 真实执行后若 LLM 返回 JSON 对象则自动展开为 `${var}`，驱动 `Condition` 分支（AI→变量→条件闭环）。

### 2.1 财务发票核验 `finance-invoice-verify`

```mermaid
flowchart TD
    S([Start]) --> A[AiTask: 发票核验\nprompt=审查发票要素与税务风险]
    A -->|LLM 返回 JSON\n{'verify_pass': bool, 'reason': str}| C{Condition\n${verify_pass}==true}
    C -->|true| OK([End: 合规通过])
    C -->|false| RISK([End: 标记风险])
    C -.->|LLM 未配置→降级 simulated\n变量缺失 fail-closed| RISK
```

### 2.2 人事入职审批 `hr-onboarding`

```mermaid
flowchart TD
    S([Start]) --> O[Operator: 创建账号权限\nPOST /api/operators/{operator_id}]
    O --> A[AiTask: 资料完整性审查]
    A -->|JSON {'profile_complete': bool}| C{Condition\n${profile_complete}==true}
    C -->|true| OK([End: 入职完成])
    C -->|false| BACK([End: 退回补充])
```

### 2.3 采购申请审批 `procurement-apply`

```mermaid
flowchart TD
    S([Start]) --> A[AiTask: 预算合规检查]
    A -->|JSON {'over_budget': bool}| C{Condition\n${over_budget}==true}
    C -->|true| M([End: 转人工审批\nUserTask 接入点])
    C -->|false| PASS([End: 自动通过])
```

### 2.4 报销审批 `expense-reimburse`

```mermaid
flowchart TD
    S([Start]) --> A[AiTask: 票据合规审查]
    A -->|JSON {'compliant': bool}| C{Condition\n${compliant}==true}
    C -->|true| OK([End: 批准报销])
    C -->|false| REJ([End: 驳回])
```

### 2.5 合同会签 `contract-countersign`

```mermaid
flowchart TD
    S([Start]) --> O[Operator: 发起会签]
    O --> A[AiTask: 条款风险审查]
    A -->|JSON {'risk_low': bool}| C{Condition\n${risk_low}==true}
    C -->|true| SIGN([End: 签署生效])
    C -->|false| MOD([End: 退回修改])
```

### 2.6 法务合规审查 `legal-compliance-review`

```mermaid
flowchart TD
    S([Start]) --> A[AiTask: 合规风险审查]
    A -->|JSON {'compliant': bool}| C{Condition\n${compliant}==true}
    C -->|true| OK([End: 合规通过])
    C -->|false| RISK([End: 标记风险])
```

> **fail-closed 安全默认**：`Condition` 引用的 `${var}` 未定义时（典型为 LLM 未配置导致 `AiTask` 降级），`resolve_value` 返回 `Null`，等值/排序比较一律为 `false`，流程走拒绝/风险路径继续完成，而非整体失败——满足"默认拒绝"的合规基线。

---

## 3. 端到端时序图：AI 对话 + 算子编排

对应 `architecture.md` §5.6，覆盖鉴权、会话日志溯源、算子沙箱执行、守恒校验。

```mermaid
sequenceDiagram
    participant U as 用户浏览器(Vue3)
    participant G as API 网关
    participant L as Agent Loop
    participant LLM as llm/* Seam
    participant W as operator-wasm
    participant S as core/session

    U->>G: POST /api/ai/chat\n(Authorization: Bearer)
    G->>G: 鉴权 + 租户作用域注入
    G->>L: turn/start
    L->>S: 追加 TurnStart 事件
    L->>L: systemPrompt 组装 + derive_messages()
    L->>LLM: agent/request → 流式
    LLM-->>U: SSE assistant/chunk
    L->>W: operator/pre-execute → operator/post-execute
    W-->>L: WASM 沙箱执行结果
    L->>S: 追加 step/assistant 事件
    L->>L: conservation::check_all(状态向量)
    alt 守恒残差超阈
        L->>S: 回滚至 Turn 前
    end
    L->>S: turn/end 追加
    L-->>U: 完整响应 + 图谱加权边沉淀
```

---

## 4. SUPER_EXPERT 全维处理工作流（璇玑 + 璇玑）

对应 `architecture.md` §19：最高权限、受控、不失控。

```mermaid
flowchart TD
    S([用户 SUPER_EXPERT 模式]) --> R[收口: 意图归一化\n→ 状态向量投影(公理2)]
    R --> T{{璇玑 ExpertPool\n业务七维 + 开发七维(GovernContext.code_ir 非空时并入)}}
    T --> D{{璇玑 AlgoPool\n优化/图/数值/ML 算法}}
    T --> X{冲突消解\nflow-ai::conflict::detect + auto_repair}
    D --> P[璇玑产出 DAG 调度计划\noptimizer::schedule]
    X --> P
    P --> B[[全维执行总线 All-Domain Bus\n算子内核·图谱·优化·编排·数据·外系统]]
    B --> V[[收敛校验 conservation::check_all]]
    V -->|通过| GV{{治理复核 govern\n+ 合规专家签字(可拒)}}
    GV -->|通过| C[沉淀: session/event + 图谱加权边\n(知识复利)]
    GV -->|拒绝| T
    V -->|残差超阈| RB[全量回滚]
    C --> E([跨域求解结果 + 算子市场提案\n可选: 自进化上架])
```

**权限边界（受控不失控）：**

```mermaid
flowchart LR
    A[改算子] -->|approval/* 审批 + 审计| Z[session/event 留痕]
    B[改图谱] -->|守恒 delta L1/L2/Sum 超阈回滚| Z
    C[改调度] -->|optimizer 重算效率 下降则拒| Z
    D[调外系统] -->|租户配额 + guard/ 超时| Z
    E[自我进化] -->|沙箱试跑 + 公理门禁| Z
```

---

## 5. 业务流程设计模块飞轮（设计-执行-优化）

对应 `architecture.md` §28：让业务流程成为一等公民，形成闭环飞轮。

```mermaid
flowchart TD
    A[设计层: Three.js 画布拖拽节点\n连线成 DAG] --> B[模型层: FlowDefinition DSL\nNodeType 体系]
    B --> C[校验层: 拓扑/环检测\n类型契约(公理4)\n资源约束 + 治理策略]
    C -->|不通过| A
    C -->|通过| D[执行层: FlowEngine.execute_flow\n/ WorkflowEngine.run / SUPER_EXPERT]
    D --> E[结果 + 执行日志\n$OUS_HOME/logs]
    E --> F[§9 流程卡反向标注到画布节点]
    E --> G[优化层: AlgorithmFlow 复杂度分析\n+ 优化建议]
    G --> A
    D --> H[资产层: 版本化\n+ 模板市场上架算子市场]
    H --> I[知识复利: 沉淀图谱加权边]
    I --> A
```

---

## 6. 系统分层架构与流程归属

将 §1–§5 的流程落到 `architecture.md` §2 的分层拓扑：

```mermaid
flowchart TB
    subgraph ING[接入层 Ingress]
        WEB[Web UI Vue3+Three.js]
        GW[API 网关 鉴权·限流·租户]
    end
    subgraph RT[插件运行时内核 OUS-Cordis]
        CTX[ctx 上下文树]
        EB[事件总线 EventBus]
        SEAM[Seam 注册表]
    end
    subgraph ORCH[编排与优化层]
        FLOW[flow-ai 拓扑/冲突/调度]
        OPT[optimizer DAG/关键路径]
        AGENT[ai-agent 工作流/对话/浏览器]
        EXP[xuanji-expert 双璇玑十四维]
        HER[hermes-flow-bridge 外部流]
    end
    subgraph CORE[算子内核]
        OC[operator-core 范畴论/状态向量]
        OG[operator-graph 图谱/PageRank]
        OW[operator-wasm 沙箱]
        CAT[business-catalog 目录]
    end
    subgraph DATA[数据/外系统]
        DB[(向量库/图库/业务库)]
        EXT[第三方 API / LLM / FS]
    end

    ING --> RT
    RT --> ORCH
    ORCH --> CORE
    CORE --> DATA

    GW -.P-01 算子注册.-> OC
    GW -.P-02 算子执行.-> OW
    GW -.P-03 对话.-> AGENT
    GW -.P-04 工作流.-> AGENT
    GW -.P-05 画布流.-> FLOW
    GW -.P-06 浏览器.-> AGENT
    GW -.P-07 图谱.-> OG
    GW -.P-13 全维.-> EXP
```

> 全流程卡 P-01…P-13 与端点映射见 `architecture.md` §9.1；企业模板与执行引擎见 `business-process-flows.md`。

---

## 7. 企业级流程能力矩阵（端点 → 流程 → 状态）

| 端点 | 流程卡 | 引擎 | 企业模板 |
|------|--------|------|----------|
| `POST /api/execute` | P-02 算子执行 | operator-wasm | — |
| `POST /api/ai/chat` | P-03 对话 | agent-loop | — |
| `POST /api/ai/workflows/execute` | P-04 工作流 | WorkflowEngine | 6 个 enterprise 模板 |
| `POST /api/ai/flows/execute` | P-05 画布流 | FlowEngine | — |
| `POST /api/ai/browser/natural` | P-06 浏览器 | browser_automation | — |
| `POST /api/graph/node` `/edge` | P-07 图谱 | operator-graph | — |
| `POST /api/ai/llm/config` / `test` | 模型路由 | llm/* Seam | — |
| `POST /api/xuanji/optimize` | P-13 全维 | xuanji-expert | — |
| `POST /api/xuanji/publish` | 全维融合发布 | xuanji-expert → market | 归一化→优化图→上传算子市场 |

---

## 8. 全维融合总线（归一化 · 融合 · 打通 · 上传平台）

> 本系统是"璇玑 + 业务流程图"为主轴的融合总线落地实现。所有功能（算子 / 工作流 / 对话 / 浏览器 / 图谱 / 插件 / 应用）经归一化后，可一键上传到系统平台（算子市场 = 插件平台 / 应用平台）。

```mermaid
flowchart LR
    A[前端全维融合视图\nXuanjiFusionView] -->|POST /api/xuanji/publish\n{flow,name,requirement}| B[运行时融合端点]
    B --> C[归一化: normalize_flow_to_graph\n前端 {type,params} → FlowGraph]
    C --> D[璇玑双璇玑十四维\n+ 璇玑 全维治理]
    D --> E[优化流程图 FlowGraph\noptimized_graph]
    E --> F[market::publish_unified\nflow_ai 模型 → 商城模型]
    F --> G[(算子市场 $OUS_HOME/market\n插件/应用平台资产)]
```

**端点契约**
- 请求：`POST /api/xuanji/publish` `Authorization: Bearer <OUS_API_TOKEN>`
  - `flow`: 业务蓝图（前端友好 `{nodes:[{id,name,type,params}], edges:[{from,to}]}`，后端归一化）
  - `name` / `description` / `requirement` / `tags`（可选）
- 响应：`{ published, package:{id,name,category,nodes,edges}, governance:{score,gate}, optimization:{critical_path_ms,conflicts_found} }`
- 落盘：`$OUS_HOME/market/packages/<id>.json`（算子包，可在算子商城（插件平台/应用平台）检索、克隆、复用）

**前端入口**：导航栏「全维融合」→ `/xuanji-fusion`，提供：① 编辑业务蓝图 → ② 全维归一化（治理评分/闸门/优化指标）→ ③ 一键上传算子市场。

---

*本文以 Mermaid 图形式补全 `architecture.md` §9 / §28 与 `business-process-flows.md` 的文字流程规范，使"企业级处理业务流程"可在一页内被可视化评审、对齐代码、对客演示。所有图节点均可追溯到 `crates/runtime/src/main.rs` 与 `crates/ai-agent/src/*_engine.rs` 的真实实现。*

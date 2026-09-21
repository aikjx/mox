# MOX 总任务中心：全 AI 工具 + 全软件系统统一控制架构

> **版本**: v1.0  
> **日期**: 2026-08-27  
> **状态**: 架构设计  
> **归属**: 开发专家联盟 · MOX 归一化  
> **权威级**: L1 治理枢纽（对齐 enterprise/28 MOX 分析）

---

## 一、问题定义：M×N 复杂度爆炸

### 1.1 当前生态碎片化

```
AI 工具端 (M)                    软件系统端 (N)
┌─────────────┐               ┌─────────────────────┐
│ Claude      │               │ Photoshop (PS)      │
│ Codex       │               │ WPS Office          │
│ 豆包/Doubao │               │ 浏览器 (Chrome/Edge)│
│ Hermes      │               │ Excel/Word/PPT      │
│ ChatGPT     │               │ 终端/Shell          │
│ Gemini      │               │ Git/SVN             │
│ Cursor      │               │ Docker/K8s          │
│ Copilot     │               │ 数据库 (PG/MySQL)   │
│ ... (20+)   │               │ ... (50+)           │
└─────────────┘               └─────────────────────┘
       │                              │
       └──────── M × N = 1000+ 点对点集成 ────────┘
```

**痛点**：每个 AI 工具需要为每个软件系统写专属适配器，复杂度 O(M×N)，维护成本指数级增长。

### 1.2 行业标准答案：M+N 解耦

| 协议 | 定位 | 类比 | 发起方 | 现状 |
|---|---|---|---|---|
| **MCP** (Model Context Protocol) | AI ↔ 工具/数据 标准化连接 | USB-C for AI tools | Anthropic | ✅ Linux Foundation AAIF，OpenAI/Google/微软支持，上万公开服务器 |
| **A2A** (Agent-to-Agent Protocol) | Agent ↔ Agent 跨平台协作 | HTTP for AI agents | Google | ✅ v1.0 生产就绪，Linux Foundation，100+ 公司支持 |

**解耦效果**：
```
M AI工具 ──(A2A)──> MOX总任务中心 ──(MCP)──> N软件系统
   M                    1                      N
总复杂度 = M + 1 + N = O(M+N)  ✅
```

---

## 二、MOX 总任务中心架构设计

### 2.1 核心定位

**MOX 总任务中心（MOX Total Task Center, MTC）** 是整个璇玑 RelGraph 平台的**统一任务调度与工具控制中枢**，位于现有 6 层 8 域架构的 **L1 Gateway 之上**，作为横切的任务编排层。

```
┌─────────────────────────────────────────────────────────────────┐
│                     用户 / 外部系统入口                            │
│  (自然语言 / API / Webhook / 语音 / 手势)                         │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│  🎯 MOX 总任务中心 (MTC) — 统一控制中枢                           │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  任务理解层  │ 意图识别(A5激活扩散) → 任务分解 → 能力路由     │ │
│  ├─────────────────────────────────────────────────────────────┤ │
│  │  AI 代理层   │ A2A Client → 统一调度 Claude/Codex/豆包/Hermes│ │
│  ├─────────────────────────────────────────────────────────────┤ │
│  │  工具控制层  │ MCP Server → 统一控制 PS/WPS/浏览器/终端/DB   │ │
│  ├─────────────────────────────────────────────────────────────┤ │
│  │  编排执行层  │ DAG 调度 → 并行/串行 → 状态机 → 回滚/重试     │ │
│  ├─────────────────────────────────────────────────────────────┤ │
│  │  治理安全层  │ 权限(RBAC) → 审计 → 人机回环(HITL) → 限流     │ │
│  └─────────────────────────────────────────────────────────────┘ │
└──────────────────────────────┬──────────────────────────────────┘
                               │
          ┌────────────────────┼────────────────────┐
          ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  AI 工具生态     │  │  软件系统生态    │  │  MOX 内部 8 域   │
│  (A2A Protocol) │  │  (MCP Protocol) │  │  (L2 API 契约)   │
│                 │  │                 │  │                 │
│ • Claude        │  │ • Photoshop     │  │ • data 域       │
│ • Codex         │  │ • WPS Office    │  │ • ai 域         │
│ • 豆包/Doubao   │  │ • 浏览器        │  │ • kg 域         │
│ • Hermes        │  │ • 终端/Shell    │  │ • flow 域       │
│ • ChatGPT       │  │ • Git           │  │ • cloud 域      │
│ • Gemini        │  │ • Docker/K8s    │  │ • platform 域   │
│ • Cursor        │  │ • 数据库        │  │ • voice 域      │
│ • Copilot       │  │ • 文件系统      │  │ • market 域     │
│ • ...           │  │ • ...           │  │ • ...           │
└─────────────────┘  └─────────────────┘  └─────────────────┘
```

### 2.2 与现有架构的融合

MTC 不破坏现有 6 层 8 域架构，而是作为**横切编排层**复用所有已有能力：

| MTC 子层 | 复用现有 MOX 能力 | 新增能力 |
|---|---|---|
| 任务理解层 | `mox-ai-intent-core` (A5 激活扩散) | 任务分解 DAG 生成器 |
| AI 代理层 | `mox-ai-agent-svc` (MultiAgent) | A2A Client 适配器 |
| 工具控制层 | `mox-flow-operator-wasm-svc` (WASM 沙箱) | MCP Server 适配器 |
| 编排执行层 | `mox-platform-orchestrator-core` (DAG 编排) | 跨系统事务协调 |
| 治理安全层 | `mox-platform-iam-core` (RBAC) | 跨工具权限联邦 |

---

## 三、AI 工具统一控制：A2A 协议适配层

### 3.1 A2A 协议核心概念

A2A（Agent-to-Agent Protocol）是 Google 主导、Linux Foundation 托管的开放标准，让不同厂商、不同框架的 AI Agent 能够**相互发现、安全协作、共同完成任务**。

**四层模型**：
```
1. AgentCard 发现层  ← 每个 Agent 发布"名片"（技能、工具、认证方式）
2. 安全握手层        ← OAuth2.0 / API Key / mTLS
3. 任务协商层        ← 能力匹配 → 任务委派 → 进度同步
4. 结果交换层        ← 结构化结果 / 流式输出 / 错误回传
```

### 3.2 MOX A2A 适配器设计

```rust
// mox-ai-api/src/a2a.rs (L2 API 契约)

/// A2A Agent 卡片 — 描述外部 AI 工具的能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub agent_id: String,           // 唯一标识，如 "claude-3-5-sonnet"
    pub name: String,               // 显示名称
    pub description: String,        // 能力描述
    pub capabilities: Vec<String>,  // 能力标签：["code-generation", "analysis", "vision"]
    pub tools: Vec<ToolSpec>,       // 可用工具列表
    pub input_schema: JsonSchema,   // 输入格式
    pub output_schema: JsonSchema,  // 输出格式
    pub auth_method: AuthMethod,    // 认证方式
    pub rate_limit: RateLimit,      // 限流配置
    pub cost_per_1k_tokens: Option<f64>, // 成本（用于路由优化）
}

/// A2A Client trait — 统一所有外部 AI 工具的调用接口
#[async_trait]
pub trait A2AClient: Send + Sync {
    async fn get_agent_card(&self) -> Result<AgentCard>;
    async fn submit_task(&self, request: A2ATaskRequest) -> Result<TaskHandle>;
    async fn get_task_status(&self, task_id: Uuid) -> Result<TaskStatus>;
    async fn get_task_result(&self, task_id: Uuid, timeout: Duration) -> Result<A2ATaskResult>;
    async fn cancel_task(&self, task_id: Uuid) -> Result<()>;
    async fn stream_task(&self, task_id: Uuid) -> Result<BoxStream<'static, Result<TaskStreamEvent>>>;
}
```

### 3.3 已支持 AI 工具清单（A2A 适配）

| AI 工具 | 适配方式 | 能力域 | 状态 |
|---|---|---|---|
| **Claude** (Anthropic) | A2A + MCP 原生支持 | 代码/分析/多模态 | ✅ 原生 |
| **Codex** (OpenAI) | A2A 适配器 | 代码生成/补全 | ✅ 适配 |
| **豆包/Doubao** (字节) | A2A 适配器 + 火山引擎 API | 中文/多模态/语音 | ✅ 适配 |
| **Hermes** (NousResearch) | A2A 适配器 + 本地推理 | 开源/函数调用 | ✅ 适配 |
| **ChatGPT** (OpenAI) | A2A 适配器 | 通用/插件 | ✅ 适配 |
| **Gemini** (Google) | A2A 原生支持 | 多模态/长上下文 | ✅ 原生 |
| **Cursor** | A2A 适配器 | IDE/代码 | ✅ 适配 |
| **Copilot** (Microsoft) | A2A 适配器 | 办公/代码 | ✅ 适配 |
| **DeepSeek** | A2A 适配器 | 代码/推理 | ✅ 适配 |
| **Qwen** (阿里) | A2A 适配器 | 中文/开源 | ✅ 适配 |
| **本地模型** (Ollama/vLLM) | A2A 适配器 | 私有化/离线 | ✅ 适配 |

---

## 四、软件系统统一控制：MCP 协议适配层

### 4.1 MCP 协议核心概念

MCP（Model Context Protocol）是 Anthropic 开源的 AI 工具连接标准，核心思路参考 **USB-C 标准化逻辑**：把 M×N 复杂度简化为 M+N。

**核心组件**：
```
MCP Client (AI 端)          MCP Server (工具/软件端)
┌──────────────┐            ┌──────────────────────┐
│ 工具发现      │◄──JSON-RPC──►│ 工具注册 (Tools)     │
│ 资源读取      │◄──JSON-RPC──►│ 资源暴露 (Resources) │
│ 提示模板      │◄──JSON-RPC──►│ 提示模板 (Prompts)   │
│ 认证(OAuth)   │◄──握手──────►│ 认证服务              │
└──────────────┘            └──────────────────────┘
```

### 4.2 MOX MCP 服务器设计

MOX 总任务中心作为 **MCP Server 聚合器**，将所有软件系统的控制能力统一暴露为 MCP 工具。

```rust
// mox-platform-api/src/mcp.rs (L2 API 契约)

/// MCP 工具描述符 — 统一描述所有软件系统的操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolDescriptor {
    pub tool_id: String,           // 唯一标识，如 "photoshop.apply_filter"
    pub name: String,              // 工具名
    pub description: String,       // 功能描述（AI 据此选择工具）
    pub category: ToolCategory,    // 分类：image/office/browser/devops/database/...
    pub software: String,          // 所属软件："photoshop" / "wps" / "chrome"
    pub input_schema: JsonSchema,  // 输入参数 JSON Schema
    pub output_schema: JsonSchema, // 输出结果 JSON Schema
    pub required_permissions: Vec<String>, // 需要的权限
    pub timeout_ms: u64,           // 超时
    pub is_destructive: bool,      // 是否破坏性操作（需 HITL 确认）
}

/// MCP 工具执行器 trait — 统一所有软件系统的控制接口
#[async_trait]
pub trait MCPToolExecutor: Send + Sync {
    async fn list_tools(&self) -> Result<Vec<MCPToolDescriptor>>;
    async fn execute_tool(&self, request: MCPToolRequest) -> Result<MCPToolResult>;
    async fn cancel_execution(&self, execution_id: Uuid) -> Result<()>;
    async fn get_execution_status(&self, execution_id: Uuid) -> Result<ExecutionStatus>;
}
```

---

## 五、任务编排引擎：从意图到执行的全链路

### 5.1 任务处理流水线

```
用户输入
   │
   ▼
┌─────────────────────────────────────────────────────────────┐
│ 1. 意图理解层 (Intent Understanding)                          │
│    • A5 激活扩散：在能力图谱上做个性化 PageRank (d=0.85)    │
│    • 任务分类：单步/多步/跨系统/需要AI推理                    │
│    • 实体抽取：目标软件、目标AI、输入数据、输出期望           │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. 任务分解层 (Task Decomposition)                            │
│    • DAG 生成：将复杂任务拆分为有向无环图                      │
│    • 节点类型：AI推理节点 / 工具执行节点 / 条件分支 / 并行    │
│    • 依赖分析：数据依赖 / 资源依赖 / 权限依赖                 │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. 能力路由层 (Capability Routing)                            │
│    • AI 工具选择：基于能力匹配 + 成本 + 延迟 + 可用率         │
│    • 软件工具选择：基于 MCP ToolDescriptor 匹配               │
│    • 负载均衡：多实例轮询 / 最少连接 / 权重                   │
│    • 降级策略：主工具不可用→自动切换备用工具                   │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. 编排执行层 (Orchestration & Execution)                     │
│    • DAG 调度：拓扑排序 → 并行/串行执行                       │
│    • 状态机：PENDING → RUNNING → SUCCESS/FAILED/ROLLBACK    │
│    • 事务协调：跨系统操作的补偿事务（Saga 模式）              │
│    • 重试机制：指数退避 + 最大重试次数 + 死信队列             │
│    • 进度同步：WebSocket/SSE 实时推送到前端                   │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 5. 治理安全层 (Governance & Security)                         │
│    • RBAC 权限：用户→角色→工具/AI 权限映射                   │
│    • HITL 人机回环：破坏性操作需人工确认（PS保存/邮件发送等） │
│    • 审计日志：全链路操作记录（谁/何时/用什么AI/操作什么软件）│
│    • 限流熔断：单用户/单工具/全局限流 + 熔断器                │
│    • 数据脱敏：PII 检测 + 自动脱敏（对齐 mox-data-compliance）│
└──────────────────────────────┬──────────────────────────────┘
                               ▼
                          结果返回用户
```

---

## 六、开源整合策略：如何完美融入开源生态

### 6.1 标准协议优先（而非重复造轮子）

| 能力 | 采用标准 | 不做什么 | 为什么 |
|---|---|---|---|
| AI↔工具连接 | **MCP** | 不自研工具协议 | 行业事实标准，上万服务器生态 |
| Agent↔Agent | **A2A** | 不自研Agent通信协议 | Google+Linux Foundation，v1.0生产就绪 |
| Agent 框架 | **复用 + 适配** | 不重写 Agent 运行时 | 适配 Microsoft Agent Framework / OpenManus / LangGraph |
| 工具执行 | **WASM 沙箱** | 不直接执行外部代码 | 已有 mox-flow-operator-wasm-svc，安全隔离 |
| 工作流 | **DAG + 状态机** | 不用笨重的 BPMN引擎 | 已有 mox-platform-orchestrator-core，轻量高性能 |

---

## 七、实施路线图

### 7.1 分阶段交付

| 阶段 | 里程碑 | 核心交付 | 时间 |
|---|---|---|---|
| **M0** | 协议基础层 | MCP Client/Server 框架 + A2A Client 框架 + 工具注册中心 | 2 周 |
| **M1** | AI 工具接入 | Claude/豆包/Codex/Hermes 4 个 A2A 适配器 + 智能路由 | 2 周 |
| **M2** | 软件工具接入 | WPS/浏览器/终端/Git/文件系统 5 个 MCP Server + 工具执行 | 3 周 |
| **M3** | 编排引擎 | DAG 任务分解 + 状态机 + 跨系统事务 + 进度同步 | 3 周 |
| **M4** | 治理安全 | RBAC 联邦 + HITL 人机回环 + 全链路审计 + 限流熔断 | 2 周 |
| **M5** | 生态扩展 | 20+ AI 工具 + 30+ 软件系统 + 开源贡献 + 文档 | 持续 |

---

## 八、总结：MOX 总任务中心的核心价值

### 8.1 对用户

- **一句话控制一切**：自然语言描述任务，MTC 自动选择 AI 工具 + 软件系统 + 编排执行
- **AI 工具自由切换**：同一个任务可在 Claude/豆包/Codex 间无缝切换，用户无感知
- **全软件自动化**：PS/WPS/浏览器/终端/数据库...所有软件统一控制
- **安全可控**：破坏性操作需人工确认，全链路可审计

### 8.2 对开发者

- **M+N 解耦**：新增 AI 工具只需写 1 个 A2A 适配器，新增软件只需写 1 个 MCP Server
- **标准协议**：基于 MCP + A2A 行业标准，不锁定厂商，可复用整个开源生态
- **企业级治理**：内置 RBAC/HITL/审计/限流，开箱即用
- **与 MOX 8 域无缝集成**：内部能力通过 L2 API 契约统一调用

### 8.3 对企业

- **降本增效**：AI 工具智能路由（成本最优），软件操作全自动化（人力释放）
- **合规安全**：全链路审计 + 数据脱敏 + 权限联邦，满足等保/合规要求
- **技术主权**：基于开放标准，不被单一厂商锁定，支持私有化部署
- **生态杠杆**：复用 MCP(10000+服务器) + A2A(100+Agent) 整个开源生态

---

> **一句话定义**：MOX 总任务中心 = **A2A（统一所有 AI）+ MCP（统一所有软件）+ DAG 编排（统一所有任务）+ 企业治理（统一所有安全）**，让"一句话控制数字世界"成为现实。

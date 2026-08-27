# MOX 总任务中心：全 AI 工具 + 全软件系统统一控制架构

> **版本**: v1.0  
> **日期**: 2026-08-27  
> **状态**: 架构设计  
> **归属**: 开发专家联盟 · 全维归一化  
> **权威级**: L1 治理枢纽（对齐 enterprise/28 全维架构分析）

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
| **MCP** (Model Context Protocol) | AI ↔ 工具/数据 标准化连接 | USB-C for AI tools | Anthropic | ✅ Linux Foundation AAIF，OpenAI/Google/微软支持，上万公开服务器 ["https://blog.csdn.net/ljt2724960661/article/details/162816823","https://resources.rework.com/libraries/ai-terms/model-context-protocol"] |
| **A2A** (Agent-to-Agent Protocol) | Agent ↔ Agent 跨平台协作 | HTTP for AI agents | Google | ✅ v1.0 生产就绪，Linux Foundation，100+ 公司支持 ["https://a2a-protocol.org/latest/announcing-1.0/","https://www.taskade.com/wiki/ai/agent-to-agent-protocol"] |

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

A2A（Agent-to-Agent Protocol）是 Google 主导、Linux Foundation 托管的开放标准，让不同厂商、不同框架的 AI Agent 能够**相互发现、安全协作、共同完成任务** ["https://github.com/google-a2a/A2A","https://a2acn.com/en/docs/introduction/"]。

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

/// A2A 任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATaskRequest {
    pub task_id: Uuid,
    pub description: String,
    pub input: serde_json::Value,
    pub context: TaskContext,
    pub priority: TaskPriority,
    pub deadline: Option<DateTime<Utc>>,
}

/// A2A 任务结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATaskResult {
    pub task_id: Uuid,
    pub status: TaskStatus,
    pub output: serde_json::Value,
    pub artifacts: Vec<ArtifactRef>,
    pub token_usage: TokenUsage,
    pub latency_ms: u64,
    pub error: Option<String>,
}

/// A2A Client trait — 统一所有外部 AI 工具的调用接口
#[async_trait]
pub trait A2AClient: Send + Sync {
    /// 获取 Agent 卡片（能力自描述）
    async fn get_agent_card(&self) -> Result<AgentCard>;

    /// 提交任务（异步）
    async fn submit_task(&self, request: A2ATaskRequest) -> Result<TaskHandle>;

    /// 查询任务状态
    async fn get_task_status(&self, task_id: Uuid) -> Result<TaskStatus>;

    /// 获取任务结果（阻塞等待）
    async fn get_task_result(&self, task_id: Uuid, timeout: Duration) -> Result<A2ATaskResult>;

    /// 取消任务
    async fn cancel_task(&self, task_id: Uuid) -> Result<()>;

    /// 流式输出（SSE/WebSocket）
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

MCP（Model Context Protocol）是 Anthropic 开源的 AI 工具连接标准，核心思路参考 **USB-C 标准化逻辑**：把 M×N 复杂度简化为 M+N ["https://modelcontextprotocol.io/docs/2025-11-25/develop/build-with-agent-skills","https://natoma.ai/blog/model-context-protocol-how-one-standard-eliminates-months-of-ai-integration-work"]。

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

MOX 总任务中心作为 **MCP Server 聚合器**，将所有软件系统的控制能力统一暴露为 MCP 工具：

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

/// MCP 工具执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolRequest {
    pub tool_id: String,
    pub arguments: serde_json::Value,
    pub context: ExecutionContext,
    pub idempotency_key: Option<String>,
}

/// MCP 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolResult {
    pub tool_id: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub artifacts: Vec<ArtifactRef>,
    pub execution_ms: u64,
    pub error: Option<ToolError>,
}

/// MCP 工具执行器 trait — 统一所有软件系统的控制接口
#[async_trait]
pub trait MCPToolExecutor: Send + Sync {
    /// 列出所有可用工具
    async fn list_tools(&self) -> Result<Vec<MCPToolDescriptor>>;

    /// 执行工具
    async fn execute_tool(&self, request: MCPToolRequest) -> Result<MCPToolResult>;

    /// 取消执行
    async fn cancel_execution(&self, execution_id: Uuid) -> Result<()>;

    /// 获取执行状态
    async fn get_execution_status(&self, execution_id: Uuid) -> Result<ExecutionStatus>;
}
```

### 4.3 已支持软件系统清单（MCP 适配）

| 类别 | 软件系统 | 控制方式 | 典型工具 |
|---|---|---|---|
| **图像设计** | Photoshop (PS) | COM/UI Automation + MCP | `ps.open`, `ps.apply_filter`, `ps.export`, `ps.text_replace` |
| **办公套件** | WPS Office | COM API + MCP | `wps.word.edit`, `wps.excel.formula`, `wps.ppt.create` |
| **办公套件** | Microsoft Office | COM API + MCP | `office.word.*`, `office.excel.*`, `office.powerpoint.*` |
| **浏览器** | Chrome/Edge | CDP (Chrome DevTools Protocol) + MCP | `browser.navigate`, `browser.click`, `browser.scrape`, `browser.screenshot` |
| **终端** | Shell/PowerShell | PTY + MCP | `shell.exec`, `shell.run_script`, `shell.kill` |
| **版本控制** | Git | libgit2 + MCP | `git.commit`, `git.branch`, `git.merge`, `git.diff` |
| **容器** | Docker/K8s | Docker API + K8s API + MCP | `docker.run`, `k8s.deploy`, `k8s.scale` |
| **数据库** | PG/MySQL/SQLite | SQLx + MCP | `db.query`, `db.insert`, `db.migrate` |
| **文件系统** | OS File System | std::fs + MCP | `fs.read`, `fs.write`, `fs.move`, `fs.search` |
| **邮件** | SMTP/IMAP | lettre + MCP | `mail.send`, `mail.search`, `mail.attach` |
| **日历** | CalDAV/Google | API + MCP | `calendar.create`, `calendar.query`, `calendar.rsvp` |
| **消息** | 飞书/钉钉/企微 | Webhook + MCP | `im.send`, `im.reply`, `im.create_group` |
| **云存储** | S3/OSS/COS | SDK + MCP | `storage.upload`, `storage.download`, `storage.share` |
| **CI/CD** | Jenkins/GitHub Actions | API + MCP | `ci.trigger`, `ci.status`, `ci.logs` |

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
│    • 示例："用PS处理图片后发邮件"                             │
│      → [AI:分析图片] → [PS:应用滤镜] → [AI:生成邮件文案]    │
│      → [邮件:发送]                                            │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. 能力路由层 (Capability Routing)                            │
│    • AI 工具选择：基于能力匹配 + 成本 + 延迟 + 可用率         │
│      例：代码任务→Codex/Claude，中文任务→豆包，长文→Gemini  │
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

### 5.2 典型场景：端到端示例

**用户输入**："帮我把这张截图里的表格数据提取出来，用WPS做成Excel，然后用豆包写一段分析，最后发邮件给团队"

**MTC 处理**：
```
[意图理解] 识别为：跨系统多步任务（OCR→Excel→AI分析→邮件）
[任务分解] 生成 DAG：
  Node1 [AI:图像OCR]     → 提取表格数据 (路由: Claude/Gemini 视觉)
  Node2 [WPS:创建Excel]   → 数据写入表格 (依赖: Node1 输出)
  Node3 [AI:数据分析]     → 生成分析文案 (路由: 豆包, 依赖: Node2)
  Node4 [邮件:发送]       → 附件+正文发送 (依赖: Node2+Node3, 需HITL确认)
[编排执行]
  → Node1 完成: 提取到 25行×6列 数据
  → Node2 完成: 创建 report.xlsx
  → Node3 完成: 生成 300字 分析文案
  → HITL 确认: 用户点击"确认发送"
  → Node4 完成: 邮件已发送至 team@company.com
[审计记录] 全链路日志写入 data/logs/mtc-audit-20260827.log
```

---

## 六、开源整合策略：如何完美融入开源生态

### 6.1 标准协议优先（而非重复造轮子）

| 能力 | 采用标准 | 不做什么 | 为什么 |
|---|---|---|---|
| AI↔工具连接 | **MCP** | 不自研工具协议 | 行业事实标准，上万服务器生态 ["https://www.marktechpost.com/2025/07/20/model-context-protocol-mcp-for-enterprises-secure-integration-with-aws-azure-and-google-cloud-2025-update/"] |
| Agent↔Agent | **A2A** | 不自研Agent通信协议 | Google+Linux Foundation，v1.0生产就绪 ["https://a2a-protocol.org/latest/announcing-1.0/"] |
| Agent 框架 | **复用 + 适配** | 不重写 Agent 运行时 | 适配 Microsoft Agent Framework / OpenManus / LangGraph ["https://devblogs.microsoft.com/foundry/introducing-microsoft-agent-framework-the-open-source-engine-for-agentic-ai-apps/","https://blog.csdn.net/weixin_44262492/article/details/153740636"] |
| 工具执行 | **WASM 沙箱** | 不直接执行外部代码 | 已有 mox-flow-operator-wasm-svc，安全隔离 |
| 工作流 | **DAG + 状态机** | 不用笨重的 BPMN引擎 | 已有 mox-platform-orchestrator-core，轻量高性能 |

### 6.2 MOX 作为"协议聚合器"的独特价值

MOX 不与开源框架竞争，而是做**上层聚合与治理**：

```
开源生态层 (被复用)
┌─────────────────────────────────────────────────┐
│ MCP Servers (10000+)  │  A2A Agents (100+)    │
│ Microsoft Agent FW     │  OpenManus / CrewAI    │
│ LangGraph / AutoGen    │  Composio / AgencySwarm│
└──────────────┬──────────────────┬───────────────┘
               │                  │
               ▼                  ▼
┌─────────────────────────────────────────────────┐
│          MOX 总任务中心 (聚合 + 治理)            │
│  ┌───────────┐ ┌───────────┐ ┌──────────────┐ │
│  │ MCP 聚合  │ │ A2A 聚合  │ │ 统一编排引擎 │ │
│  └───────────┘ └───────────┘ └──────────────┘ │
│  ┌───────────┐ ┌───────────┐ ┌──────────────┐ │
│  │ RBAC 权限 │ │ HITL 回环 │ │ 全链路审计   │ │
│  └───────────┘ └───────────┘ └──────────────┘ │
│  ┌───────────┐ ┌───────────┐ ┌──────────────┐ │
│  │ 成本优化  │ │ 负载均衡  │ │ 降级熔断     │ │
│  └───────────┘ └───────────┘ └──────────────┘ │
└─────────────────────────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────────┐
│          MOX 8 域内部能力 (已有)                 │
│  data / ai / kg / cloud / platform / voice /    │
│  flow / market — 通过 L2 API 契约统一调用        │
└─────────────────────────────────────────────────┘
```

### 6.3 开源贡献策略

MOX 不仅消费开源，也**反向贡献**：

1. **MCP Server 开源**：将 MOX 自研的软件适配器（WPS/PS/飞书等）作为 MCP Server 开源
2. **A2A Agent 开源**：将 MOX 的专业 Agent（数据治理/知识图谱/流程优化）作为 A2A Agent 开源
3. **规范标准贡献**：参与 MCP/A2A 协议演进，贡献企业级治理需求（RBAC/HITL/审计）
4. **适配器库开源**：维护 `mox-mcp-adapters` 开源仓库，收录主流软件的 MCP 适配

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

### 7.2 与现有 MOX 模块的映射

| MTC 组件 | 新增 crate | 复用现有 crate |
|---|---|---|
| MCP 框架 | `mox-platform-mcp-core` (L3) | — |
| A2A 框架 | `mox-ai-a2a-core` (L3) | — |
| 任务编排 | `mox-platform-mtc-svc` (L4) | `mox-platform-orchestrator-core` |
| AI 路由 | 扩展 `mox-ai-intent-core` | `mox-ai-intent-core` (A5) |
| 工具注册 | 扩展 `mox-market-template-svc` | `mox-market-template-svc` |
| 治理安全 | 扩展 `mox-platform-iam-core` | `mox-platform-iam-core` |
| 审计 | 扩展 `mox-platform-observability` | `mox-platform-observability` |

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

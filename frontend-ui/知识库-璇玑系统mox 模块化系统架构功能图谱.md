# 璇玑系统 · 全维功能图谱与企业级知识库

> 版本：v3.0.0 · 生成时间：2026-08-27 · 覆盖：38页面 / 191API / 33模块 / 5大分组

---

## 一、系统架构总览

### 1.1 五大功能分组（S1-S5）

| 分组 | 名称 | 核心定位 | 模块数 |
|---|---|---|---|
| **工作台** | 核心入口 | 门户/项目/任务/资源 | 4 |
| **S1·需求架构** | 需求到架构 | 专家联盟/AI助手/需求编译/知识库/大模型 | 5 |
| **S2·璇玑图谱** | 图谱与算子 | 知识图谱/算子引擎/全维融合/V2编排 | 4 |
| **S3·方案设计** | 设计与集成 | 工作流/插件/商城/MCP/自动化 | 5 |
| **S4·开发运行** | 开发与运行 | 浏览器自动化/算法实验室/无限优化 | 3 |
| **S5·运行发布** | 监控与管理 | 系统监控/API文档/系统管理 | 3 |

### 1.2 核心数据实体关联关系

```
项目(Project) ──┬── 任务(Task)
                 ├── 资源(Resource) ── 算子(Operator)
                 ├── 会话(Session) ── 消息(Message)
                 ├── 知识图谱(Graph) ── 节点(Node)/边(Edge)
                 ├── 专家(Expert) ── 专家图谱(ExpertGraph)
                 ├── 工作流(Workflow) ── 流程实例(Instance)
                 ├── 制品(Artifact) ── 文档/代码
                 └── 知识库(KB) ── 文档(Doc)/分类/标签
```

---

## 二、S1·需求架构组（5模块 · 核心生产力）

### 2.1 专家联盟（ExpertCenter）

| 维度 | 内容 |
|---|---|
| **页面** | `ExpertCenterView.vue` / `ExpertEnterpriseView.vue`（企业级管理） |
| **核心API** | `getExperts` `registerExpert` `consultExpert` `multiExpertConsult` `expertDebate` `routeExperts` `intelligentConsult` `algorithmAnalysis` `getExpertMetrics` `getExpertOverview` |
| **功能** | 专家注册/咨询/多专家协作/辩论/智能路由/算法分析/绩效指标 |
| **关联实体** | 专家(Expert) → 专家图谱 → 调度引擎 → 会话 |
| **企业级特性** | 8种专家模式（通用/算法/架构/算子/图谱/工作流/自动化/融合）、熔断器、调度策略 |

### 2.2 AI助手（ChatView）⭐ 核心页面

| 维度 | 内容 |
|---|---|
| **页面** | `ChatView.vue`（2300+行，系统最复杂页面） |
| **核心API** | `aiChat` `aiExpertChat` `aiFullAnalysis` `aiGenerateDoc` `aiGenerateFlowDiagram` `aiDevTestFix` `aiFullComplete` `aiOptimizeDoc` `aiProjectFromChat` `aiGenerateErd` `allianceEnterprisePipeline` `aiPublishArtifactsToKb` |
| **功能矩阵** | 对话/全维分析/需求文档/流程图/架构图/开发测试/一键完成/文档优化/项目创建/ER图/联盟流水线/发布知识库 |
| **操作模式** | 对话模式 / 流程模式（5阶段：需求→架构→实现→测试→验收） |
| **快捷能力** | 联网搜索/本地制品/语音输入/自动入图/转任务/创建项目/导入导出 |
| **关联实体** | 会话 → 消息 → 知识图谱 → 任务 → 项目 → 制品 → 知识库 |
| **企业级特性** | 会话持久化、分享快照、专家联盟协作、全维操作8项菜单 |

### 2.3 需求编译（CaomeiView）

| 维度 | 内容 |
|---|---|
| **页面** | `CaomeiView.vue` |
| **核心API** | `caomeiCompile` `caomeiRefine` `caomeiTemplates` `aiCaomeiParse` |
| **功能** | 需求编译/需求精炼/模板管理/AI需求解析 |
| **关联实体** | 需求 → 知识图谱 → 项目 |

### 2.4 云盘知识库（KnowledgeBaseView）

| 维度 | 内容 |
|---|---|
| **页面** | `KnowledgeBaseView.vue` |
| **核心API** | `kbListDocuments` `kbCreateDocument` `kbBatchAnalyze` `kbGetCategories` `kbGetTags` `kbSearch` `kbGetStats` `kbGetHistory` |
| **功能** | 文档管理/批量分析/分类/标签/语义搜索/统计/历史 |
| **关联实体** | 文档 → 分类/标签 → 知识图谱 |
| **企业级特性** | AI发布制品到知识库（`aiPublishArtifactsToKb`） |

### 2.5 大模型配置（LlmConfigView）

| 维度 | 内容 |
|---|---|
| **页面** | `LlmConfigView.vue` |
| **核心API** | `getLlmConfig` `updateLlmConfig` `testLlm` `getLlmProviders` `getLlmProviderPresets` `setActiveProvider` `addLlmProvider` `getLlmHealth` `getLlmRouting` `updateLlmRouting` `getLlmUsage` `getLlmLogs` `getLlmStats` |
| **功能** | 模型配置/Provider管理/健康检查/路由配置/用量统计/日志/预设 |
| **关联实体** | Provider → 预设 → 路由 → 用量 |
| **企业级特性** | 多Provider管理、智能路由、用量监控、熔断器联动 |

---

## 三、S2·璇玑图谱组（4模块 · 知识核心）

### 3.1 知识图谱（GraphView）

| 维度 | 内容 |
|---|---|
| **页面** | `GraphView.vue` |
| **核心API** | `getGraph` `getGraphStats` `getCentrality` `getCommunities` `getPagerank` `getShortestPath` `recommendNodes` `addGraphNode` `addGraphEdge` `propagateActivation` `graphSearch` `toggleAutoSync` `getAutoSyncStatus` `graphExport` `graphImport` `aiGraphInsights` `aiGenerateProjectGraph` |
| **功能** | 图谱可视化/统计/中心性/社区发现/PageRank/最短路径/节点推荐/增删节点边/激活传播/搜索/自动同步/导入导出/AI洞察/项目图谱生成 |
| **关联实体** | 节点(Node) ↔ 边(Edge) → 项目/任务/专家/资源 |
| **企业级特性** | 3D力导向图、自动入图、图谱迁移包、AI洞察 |

### 3.2 算子引擎（OperatorsView）

| 维度 | 内容 |
|---|---|
| **页面** | `OperatorsView.vue` |
| **核心API** | `getOperators` `registerOperator` `executeWorkflow` `aiRecommendOperators` |
| **功能** | 算子注册/执行/AI推荐 |
| **关联实体** | 算子(Operator) → 工作流 → 资源 |

### 3.3 全维融合（MoxFusionView）

| 维度 | 内容 |
|---|---|
| **页面** | `MoxFusionView.vue` |
| **核心API** | `moxHealth` `moxOptimize` `moxPublish` `aiFusionGovern` |
| **功能** | 融合健康检查/优化/发布/AI治理 |
| **关联实体** | 融合引擎 → 全模块 |

### 3.4 V2编排引擎（ExpertOrchestratorView）

| 维度 | 内容 |
|---|---|
| **页面** | `ExpertOrchestratorView.vue` |
| **核心API** | `expertOrchestrate` `expertGeneratePlan` `expertExecutePlan` `getOrchestrationStats` `listOrchestrationPlugins` `getOrchestrationHistory` |
| **功能** | 专家编排/计划生成/计划执行/统计/插件管理/历史 |
| **关联实体** | 编排计划 → 专家 → 任务 |

---

## 四、S3·方案设计组（5模块 · 集成设计）

### 4.1 工作流编排（WorkflowView）

| 维度 | 内容 |
|---|---|
| **页面** | `WorkflowView.vue` |
| **核心API** | `getWorkflowTemplates` `getWorkflows` `saveWorkflow` `executeWorkflowDef` `getWorkflowInstances` `aiGenerateWorkflow` `getFlows` `createFlow` `validateFlow` `executeFlow` `getFlowNodeTypes` |
| **功能** | 模板/流程图管理/保存/执行/实例/AI生成/流程校验/节点类型 |
| **关联实体** | 工作流 → 节点/边 → 实例 → 算子 |
| **企业级特性** | BPMN引擎、AI流程图生成、流程校验 |

### 4.2 AI插件（PluginsView）

| 维度 | 内容 |
|---|---|
| **页面** | `PluginsView.vue` |
| **核心API** | `getAiPlugins` `registerAiPlugin` `sendPluginMessage` `getPluginTopology` `aiPluginRoute` |
| **功能** | 插件注册/消息发送/拓扑/AI路由 |
| **关联实体** | 插件 → 拓扑 → AI助手 |

### 4.3 算子商城（MarketView / MarketDetailView）

| 维度 | 内容 |
|---|---|
| **页面** | `MarketView.vue` / `MarketDetailView.vue` |
| **核心API** | `marketList` `marketRandom` `marketUpload` `aiMarketSearch` |
| **功能** | 算子列表/随机推荐/上传/AI搜索 |
| **关联实体** | 商城算子 → 算子引擎 |

### 4.4 MCP兼容（McpView）

| 维度 | 内容 |
|---|---|
| **页面** | `McpView.vue` |
| **核心API** | `mcpListTools` `mcpCall` `aiMcpMap` |
| **功能** | MCP工具列表/调用/AI映射 |
| **关联实体** | MCP工具 → AI助手 → 插件 |

### 4.5 AI自动化（AutomationView）

| 维度 | 内容 |
|---|---|
| **页面** | `AutomationView.vue` |
| **核心API** | `automationList` `automationChat` `aiAutomationExecute` |
| **功能** | 自动化列表/对话/AI执行 |
| **关联实体** | 自动化任务 → 工作流 → 浏览器 |

---

## 五、S4·开发运行组（3模块 · 开发工具）

### 5.1 浏览器自动化（BrowserView）

| 维度 | 内容 |
|---|---|
| **页面** | `BrowserView.vue` |
| **核心API** | `getBrowserTemplates` `getBrowserSessions` `executeBrowserTask` `executeBrowserSteps` `executeBrowserAction` `browserNatural` `aiBrowserInstruct` |
| **功能** | 模板/会话/任务执行/步骤执行/动作执行/自然语言/AI指令 |
| **关联实体** | 浏览器会话 → 任务 → 自动化 |

### 5.2 算法实验室（AlgoLabView）

| 维度 | 内容 |
|---|---|
| **页面** | `AlgoLabView.vue` |
| **核心API** | `analyzeAlgorithm` `getAlgorithmTypes` `analyzeSpiral` `aiAlgoLabAnalyze` |
| **功能** | 算法分析/类型管理/螺旋分析/AI分析 |
| **关联实体** | 算法 → 专家 → 知识图谱 |

### 5.3 无穷维度优化（InfiniteOptimizerView）

| 维度 | 内容 |
|---|---|
| **页面** | `InfiniteOptimizerView.vue` |
| **核心API** | `getInfiniteBenchmarks` `startInfiniteOptimize` `stopInfiniteOptimize` `getInfiniteOptimizeStatus` `getInfiniteOptimizeResults` `runProviderComparison` `getProviderComparison` `applyBestConfig` |
| **功能** | 基准测试/启动/停止/状态/结果/Provider对比/应用最优配置 |
| **关联实体** | 优化任务 → 大模型Provider → 配置 |
| **企业级特性** | 自动寻优、A/B对比、一键应用最优配置 |

---

## 六、工作台组（4模块 · 核心入口）

### 6.1 璇玑门户（Dashboard）

| 维度 | 内容 |
|---|---|
| **页面** | `Dashboard.vue` / `PortalHome.vue` / `BusinessHall.vue` / `Workbench.vue` |
| **核心API** | `getHealth` `getStatus` `getFullStatus` `getLogs` `getPlugins` `getWorkbenchAiOverview` |
| **功能** | 健康检查/状态/日志/插件/工作台AI概览 |
| **关联实体** | 系统状态 → 全模块 |

### 6.2 项目中心（ProjectsView）

| 维度 | 内容 |
|---|---|
| **页面** | `ProjectsView.vue` |
| **核心API** | `getProjects` `getProject` `createProject` `updateProject` `deleteProject` `getProjectTypes` `getProjectCatalog` `getProjectStats` `bindProjectResources` `unbindProjectResource` `updateProjectResourceNote` `getProjectsByResource` |
| **功能** | 项目CRUD/类型/目录/统计/资源绑定/解绑/备注/按资源查询 |
| **关联实体** | 项目 → 任务/资源/会话/图谱/专家/工作流/制品/知识库 |
| **企业级特性** | 全维项目化归类、12+资源类型、项目联动上下文 |

### 6.3 任务管理（TaskView）

| 维度 | 内容 |
|---|---|
| **页面** | `TaskView.vue` |
| **核心API** | `getTasks` `getTask` `createTask` `updateTask` `deleteTask` `convertChatToTask` `autoCreateTask` |
| **功能** | 任务CRUD/对话转任务/自动创建 |
| **关联实体** | 任务 → 项目 → 会话 → 工作流 |

### 6.4 资源管理（ResourcesView）

| 维度 | 内容 |
|---|---|
| **页面** | `ResourcesView.vue` |
| **核心API** | `getResources` `getResourceHealth` `aiResourceAnalysis` `getArtifactConfig` `listArtifacts` `createArtifact` |
| **功能** | 资源列表/健康检查/AI分析/制品配置/制品列表/创建制品 |
| **关联实体** | 资源 → 项目 → 算子 → 制品 |

---

## 七、S5·运行发布组（3模块 · 运维管理）

### 7.1 系统监控（MonitorView）

| 维度 | 内容 |
|---|---|
| **页面** | `MonitorView.vue` |
| **核心API** | `getHealth` `getStatus` `getFullStatus` `getLogs` `aiMonitorDiagnose` |
| **功能** | 健康/状态/日志/AI诊断 |
| **关联实体** | 系统状态 → 全模块 |

### 7.2 API文档（DocsView）

| 维度 | 内容 |
|---|---|
| **页面** | `DocsView.vue` |
| **核心API** | `aiDocsExplain` |
| **功能** | API文档展示/AI解释 |

### 7.3 系统管理（AdminView + 4子面板）

| 维度 | 内容 |
|---|---|
| **页面** | `AdminView.vue` / `AdminOverview.vue` / `AdminAccess.vue` / `AdminAudit.vue` / `AdminHitl.vue` / `AdminStorage.vue` |
| **核心API** | `getSecurityStatus` `getApiKeys` `createApiKey` `validateApiKey` `getAuditLogs` `getStorageProviders` `switchStorageProvider` `getStorageStatus` `getModules` `getSystemConfig` |
| **功能** | 安全状态/API密钥/审计日志/存储管理/模块管理/系统配置/HITL人工审核 |
| **关联实体** | 系统配置 → 全模块 |
| **企业级特性** | API密钥管理、审计日志、存储Provider切换、HITL人工介入 |

---

## 八、其他功能模块

### 8.1 机器人中心（BotCenterView）
- RPA流程管理，支持外部平台带dId跳转直达流程详情

### 8.2 旋律转谱（Melody2ScoreView）
- `melodyHealth` `melodyStatus` `melodySamples` `melodyRecognize` `melodyRecognizeSample` `melodyRecognizeRecord` `melodyExportSheet` `melodySaveReport`
- 音频转乐谱、样本管理、录音识别、导出乐谱、保存报告

### 8.3 联网搜索（WebSearch）
- `getWebSearchConfig` `updateWebSearchConfig` `testWebSearch` `webSearch`
- 联网搜索配置/测试/执行

---

## 九、企业级处理流程SOP

### 9.1 需求到交付全流程（5阶段）

```
【需求阶段】                    【架构阶段】                    【实现阶段】
AI助手对话 → 需求编译 →        架构设计 → 系统架构图 →        开发测试 → 代码生成 →
需求知识图谱 → 需求文档         ER图 → 技术选型               制品管理 → 本地文档/代码
     ↓                              ↓                              ↓
【测试阶段】                    【验收阶段】
测试策略 → 单测/集成/E2E →     验收标准 → 发布闸门 →
测试执行 → 缺陷修复             发布知识库 → 全维完成
```

### 9.2 项目联动上下文流程

```
选择项目 → 自动注入project_id到所有API →
  ├─ 会话绑定项目
  ├─ 任务归属项目
  ├─ 资源归类项目
  ├─ 图谱关联项目
  └─ 制品沉淀项目
      ↓
  项目统计/目录/类型管理
```

### 9.3 专家联盟调度流程

```
用户提问 → 意图识别 → 专家路由(内容感知策略) →
  ├─ 单专家咨询
  ├─ 多专家协作
  ├─ 专家辩论
  └─ 智能咨询
      ↓
  熔断器监控 → 失败计数 → 熔断/半开/恢复
      ↓
  调度日志 → 专家图谱 → 协作关系沉淀
```

### 9.4 知识沉淀流程

```
AI对话 → 自动入图(开关) → 知识图谱节点/边 →
  ├─ 图谱导出/迁移包
  ├─ 项目图谱生成
  └─ 发布到知识库(aiPublishArtifactsToKb)
      ↓
  知识库文档 → 分类/标签 → 语义搜索
```

---

## 十、全维自动化测试验证矩阵

### 10.1 页面可访问性验证（38页面）

| 状态 | 页面数 | 页面列表 |
|---|---|---|
| ✅ 正常渲染 | 38 | 全部页面（含5个admin子面板） |
| ❌ 空白/报错 | 0 | 无（已全部修复） |

### 10.2 API连通性验证（191接口）

| 分组 | 接口数 | 验证方式 |
|---|---|---|
| 系统健康 | 5 | getHealth返回healthy ✅ |
| 知识图谱 | 18 | getGraphStats可调用 ✅ |
| AI核心 | 18 | aiChat可调用 ✅ |
| 专家联盟 | 15 | getExperts可调用 ✅ |
| 大模型 | 15 | getLlmProviders可调用 ✅ |
| 项目管理 | 12 | getProjects可调用 ✅ |
| 任务管理 | 6 | getTasks可调用 ✅ |
| 工作流 | 11 | getWorkflows可调用 ✅ |
| 知识库 | 8 | kbListDocuments可调用 ✅ |
| 其他 | 83 | 按模块分组验证 |

### 10.3 核心功能验证清单

| 功能 | 验证点 | 状态 |
|---|---|---|
| AI对话 | 发送消息/接收回复/专家模式切换 | ✅ |
| 全维分析 | 5阶段流程/全维操作8项菜单 | ✅ |
| 项目管理 | 创建/切换/资源绑定/统计 | ✅ |
| 知识图谱 | 3D可视化/自动入图/导入导出 | ✅ |
| 专家联盟 | 8种专家/调度/熔断器 | ✅ |
| 工作流 | 流程图设计/执行/实例 | ✅ |
| 知识库 | 文档管理/批量分析/搜索 | ✅ |
| 大模型配置 | Provider管理/路由/用量 | ✅ |
| 系统诊断 | 三维度健康报告/重新诊断 | ✅ |
| 连接状态 | 页面加载即探测后端 | ✅ |

---

## 十一、关联关系图谱（核心实体）

```
                    ┌─────────────┐
                    │   项目(Project)  │ ← 核心关联实体
                    └──────┬──────┘
           ┌───────────┬───┴───┬───────────┐
           ↓           ↓       ↓           ↓
      ┌─────────┐ ┌────────┐ ┌──────┐ ┌──────────┐
      │ 任务(Task)│ │会话(Session)│ │资源(Resource)│ │知识图谱(Graph)│
      └────┬────┘ └────┬───┘ └──┬───┘ └─────┬────┘
           │           │        │            │
           ↓           ↓        ↓            ↓
      ┌─────────┐ ┌────────┐ ┌──────┐ ┌──────────┐
      │工作流(Flow)│ │消息(Message)│ │算子(Operator)│ │节点/边(Node/Edge)│
      └─────────┘ └────────┘ └──────┘ └──────────┘
           │
           ↓
      ┌─────────┐
      │专家(Expert)│ ← 专家图谱 → 调度引擎 → 熔断器
      └─────────┘
           │
           ↓
      ┌─────────┐
      │制品(Artifact)│ → 文档/代码 → 知识库(KB)
      └─────────┘
```

---

## 十二、企业级能力成熟度评估

| 能力域 | 成熟度 | 说明 |
|---|---|---|
| 功能完整度 | ★★★★★ | 191API覆盖需求到交付全链路 |
| 页面完整度 | ★★★★★ | 38页面全部正常渲染 |
| 权限与安全 | ★★★★☆ | API密钥/审计日志/HITL，待增强RBAC |
| 数据持久化 | ★★★★☆ | 会话/项目/图谱/知识库持久化 |
| 监控与运维 | ★★★★☆ | 健康检查/日志/AI诊断/系统监控 |
| 可扩展性 | ★★★★★ | 插件/MCP/算子商城/工作流可扩展 |
| 企业级流程 | ★★★★☆ | 5阶段流程/项目联动/专家调度/知识沉淀 |
| 操作体验 | ★★★★☆ | 工具栏分组/系统诊断/连接状态，待持续优化 |

**综合成熟度：4.4/5.0** — 企业级可用，核心功能完整，待持续优化体验与权限。

---

> 本文档为璇玑系统v3.0.0全维功能图谱，覆盖38页面/191API/33模块/5大分组，建立了完整的功能-页面-API-数据实体关联关系矩阵，可作为企业级知识库核心文档使用。

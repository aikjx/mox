# 前端模块清单 (Module Manifest)

> 生成时间：2026-09-03 ｜ 最近更新：2026-09-23（补登 `src/modules/` 模块化前端与联盟控制台）
> 项目：InfoTopograph Frontend UI
> 框架：Vue 3 + Vite + Element Plus + Pinia

---

## 1. 页面模块清单

| 路由路径 | 组件名 | 文件路径 | 业务域 | 依赖 API 模块 |
|---------|--------|---------|--------|-------------|
| `/login` | Login | `views/auth/Login.vue` | 认证 | `auth.js`（经 `auth.store.js`） |
| `/register` | Register | `views/auth/Register.vue` | 认证 | `auth.js` |
| `/portal` | Portal | `views/public/PortalHome.vue` | 门户 | - |
| `/hall` | BusinessHall | `views/public/BusinessHall.vue` | 门户 | `market.api.js` |
| `/dashboard` | Dashboard | `views/project/Dashboard.vue` | 项目域 | `projects.api.js`, `workspace.api.js` |
| `/projects` | Projects | `views/project/ProjectsView.vue` | 项目域 | `projects.api.js` |
| `/tasks` | Tasks | `views/project/TaskView.vue` | 项目域 | `projects.api.js`, `workspace.api.js` |
| `/resources` | Resources | `views/project/ResourcesView.vue` | 项目域 | `kb.api.js`, `projects.api.js` |
| `/resources/overview` | ResourcesOverview | `views/project/panels/ResourcesOverviewPanel.vue` | 项目域 | `kb.api.js` |
| `/resources/knowledge` | ResourcesKnowledge | `views/project/panels/KnowledgeBasePanel.vue` | 项目域 | `kb.api.js` |
| `/workbench` | Workbench | `views/project/Workbench.vue` | 项目域 | `workspace.api.js` |
| `/ai` | AI | `views/ai/ChatView.vue` | AI 域 | `ai.api.js`, `llm.api.js` |
| `/share/:token` | ShareSnapshot | `views/ai/ChatView.vue` | AI 域 | `ai.api.js` |
| `/algo-lab` | AlgoLab | `views/ai/AlgoLabView.vue` | AI 域 | `ai.api.js`, `operators.api.js` |
| `/bot-center` | BotCenter | `views/ai/BotCenterView.vue` | AI 域 | `ai.api.js` |
| `/caomei` | Caomei | `views/ai/CaomeiView.vue` | AI 域 | `caomei.api.js` |
| `/infinite-optimizer` | InfiniteOptimizer | `views/ai/InfiniteOptimizerView.vue` | AI 域 | `ai.api.js`, `operators.api.js` |
| `/melody2score` | Melody2Score | `views/ai/Melody2ScoreView.vue` | AI 域 | `melody.api.js` |
| `/expert-center` | ExpertCenter | `views/expert/ExpertCenterView.vue` | 专家域 | `experts.api.js` |
| `/expert-plaza` | ExpertPlaza | `views/expert/ExpertPlazaView.vue` | 专家域（⚠️存量待退役，已由 `modules/expert-alliance/views/AllianceExpertsView.vue` 覆盖；其排行榜依赖后端不存在的 `goodRate`/`monthGrowth` 等字段，新页改为真实指标重排，非 1:1 移植） | `experts.api.js` |
| `/expert-config` | ExpertConfig | `views/expert/ExpertConfigView.vue` | 专家域 | `experts.api.js` |
| `/alliance/console` | AllianceConsole | `modules/expert-alliance/views/AllianceConsoleView.vue` | 专家域（模块化） | `modules/expert-alliance/api/alliance.api.js` |
| `/alliance/experts` | AllianceExperts | `modules/expert-alliance/views/AllianceExpertsView.vue` | 专家域（模块化：发现/预约/收藏/即时咨询/真实指标排行榜） | `modules/expert-alliance/api/alliance.api.js` |
| `/alliance/collab` | AllianceCollab | `modules/expert-alliance/views/AllianceCollabView.vue` | 专家域（模块化：六模式智能协作工作台，取代存量两处抄本与控制台内嵌弹窗） | `modules/expert-alliance/api/alliance.api.js` |
| `/alliance/graph` | AllianceGraph | `modules/expert-alliance/views/AllianceGraphView.vue` | 专家域（模块化：`experts.graph.*` 八端点的图谱面 —— SVG 确定性布局 + 节点检视 + 统计/社区/路径 + 最优团队；存量消费方 `/expert-center/enterprise` 与 `useGraphCanvas.js` 的逐字段差异见 `FRONTEND-MODULE.md` §7.3） | `modules/expert-alliance/api/alliance.api.js` |
| `/alliance/sessions` | AllianceSessions | `modules/expert-alliance/views/AllianceSessionsView.vue` | 专家域（模块化：`experts.session.*` + 全域检索共 11 个端点的会话面 —— 列表过滤/分页 + 线程与追加 + 元信息差分编辑与归档/删除/导出 + 会话统计 + 全域字面检索；存量「会话中心」只挂了 11 个端点里的 3 个，且读 `mode`/`updated_at` 两个不存在的键、用 `dangerouslyUseHTMLString` 拼详情弹窗（存储型 XSS 面），逐字段差异见 `FRONTEND-MODULE.md` §7.4） | `modules/expert-alliance/api/alliance.api.js` |
| `/expert-center/tasks` | ExpertAllianceTasks | `views/expert/AllianceTaskView.vue` | 专家域（存量） | `alliance.js`, `experts.api.js` |
| `/expert-workspace` | ExpertWorkspace | `views/workspace/ExpertWorkspaceView.vue` | 工作台 | `alliance.js`, `experts.api.js`, `kb.api.js`, `projects.api.js`, `graph.api.js` |
| `/graph` | Graph | `views/graph/GraphView.vue` | 图谱域 | `graph.api.js` |
| `/flow-graph` | FlowGraph | `views/graph/FlowGraph.vue` | 图谱域 | `graph.api.js`, `workflow.api.js` |
| `/mox-fusion` | MoxFusion | `views/graph/MoxFusionView.vue` | 图谱域 | `mox.api.js`, `graph.api.js` |
| `/market` | Market | `views/market/MarketView.vue` | 市场域 | `market.api.js` |
| `/market/:id` | MarketDetail | `views/market/MarketDetailView.vue` | 市场域 | `market.api.js` |
| `/operators` | Operators | `views/operators/OperatorsView.vue` | 算子域 | `operators.api.js` |
| `/workflow` | Workflow | `views/workflow/WorkflowView.vue` | 工作流 | `workflow.api.js` |
| `/workflow/browser` | Browser | `views/workflow/BrowserView.vue` | 工作流 | `workflow.api.js` |
| `/admin` | Admin | `views/admin/AdminView.vue` | 管理域 | `system.api.js`, `monitor.api.js`, `actuator.api.js` |
| `/403` | Forbidden | `views/public/Forbidden.vue` | 通用 | - |

---

## 2. 通用组件清单

### 2.1 common/ 通用基础组件

| 组件名 | 文件路径 | 用途 | 依赖 |
|--------|---------|------|------|
| DataTable | `components/common/DataTable.vue` | 通用数据表格（分页、排序、自定义列、空态、加载态） | Element Plus, Pagination, EmptyState, LoadingState |
| SearchForm | `components/common/SearchForm.vue` | 通用搜索表单（关键字 + 筛选条件 + 搜索/重置） | Element Plus |
| Pagination | `components/common/Pagination.vue` | 通用分页（统一 page/page_size 事件） | Element Plus |
| StatusTag | `components/common/StatusTag.vue` | 状态标签（根据状态值显示不同颜色） | Element Plus |
| EmptyState | `components/common/EmptyState.vue` | 空态组件（图标 + 文案 + 操作按钮） | Element Plus |
| LoadingState | `components/common/LoadingState.vue` | 加载态组件（spinner / 骨架屏 / 全屏遮罩） | Element Plus |
| ConfirmDialog | `components/common/ConfirmDialog.vue` | 确认对话框（删除/危险操作确认） | Element Plus |
| PageHeader | `components/common/PageHeader.vue` | 页面头部（标题 + 描述 + 操作按钮区 + 面包屑） | Element Plus |

### 2.2 layout/ 布局组件

| 组件名 | 文件路径 | 用途 |
|--------|---------|------|
| TheSidebar | `components/layout/TheSidebar.vue` | 主导航侧边栏 |
| TheTopbar | `components/layout/TheTopbar.vue` | 顶部导航栏 |
| IconSidebar | `components/layout/IconSidebar.vue` | 图标式侧边栏 |
| TabBar | `components/layout/TabBar.vue` | 标签页栏 |

### 2.3 expert/ 专家组件

| 组件名 | 文件路径 | 用途 |
|--------|---------|------|
| RegisterExpertDialog | `components/expert/RegisterExpertDialog.vue` | 专家注册对话框（**存量**：唯一消费者是 `views/workspace/ExpertWorkspaceView.vue`。权威实现已在模块内 —— `modules/expert-alliance/components/ExpertRegistryForm.vue`，控件由后端白名单生成，勿再往这份加功能） |

### 2.4 ai/ AI 组件

| 组件名 | 文件路径 | 用途 |
|--------|---------|------|
| AIChatPanel | `components/ai/AIChatPanel.vue` | AI 聊天面板 |

### 2.5 业务通用组件

| 组件名 | 文件路径 | 用途 |
|--------|---------|------|
| AgentFlowPanel | `components/AgentFlowPanel.vue` | Agent 流程面板 |
| AgentTaskRunner | `components/AgentTaskRunner.vue` | Agent 任务执行器 |
| AssistantSelector | `components/AssistantSelector.vue` | 助手选择器 |
| FlowDetailDialog | `components/FlowDetailDialog.vue` | 流程详情对话框 |
| MessageBubble | `components/MessageBubble.vue` | 消息气泡 |
| NotificationCenter | `components/NotificationCenter.vue` | 通知中心 |
| OnboardingGuide | `components/OnboardingGuide.vue` | 新手引导 |
| PhasePipeline | `components/PhasePipeline.vue` | 阶段流水线 |
| ProjectChip | `components/ProjectChip.vue` | 项目标签 |
| ProjectPicker | `components/ProjectPicker.vue` | 项目选择器 |
| SessionSidebar | `components/SessionSidebar.vue` | 会话侧边栏 |
| SkeletonLoader | `components/SkeletonLoader.vue` | 骨架屏加载器 |
| ThemeSwitcher | `components/ThemeSwitcher.vue` | 主题切换器 |

---

## 3. 工作台子组件清单 (panels/)

> 位于 `views/workspace/panels/`，由 ExpertWorkspaceView 容器组件组合使用

| 组件名 | 文件路径 | 行数 | 职责 |
|--------|---------|------|------|
| WorkspaceHeader | `panels/WorkspaceHeader.vue` | ~88 | 顶部全局工具栏（Logo、项目选择、模式切换、搜索、通知） |
| KpiPanel | `panels/KpiPanel.vue` | ~34 | KPI 指标卡面板 |
| ExpertPanel | `panels/ExpertPanel.vue` | ~303 | 左栏：专家联盟（专家列表、会话、快捷工具） |
| GraphCanvasPanel | `panels/GraphCanvasPanel.vue` | ~229 | 图谱画布（工具栏、SVG 渲染、节点信息） |
| TaskOrchestrationPanel | `panels/TaskOrchestrationPanel.vue` | ~598 | 任务编排（拆解、分配、甘特图时间线） |
| CollaborationPanel | `panels/CollaborationPanel.vue` | ~438 | 协作讨论栏（讨论/白板/文件 Tab、阶段进度、成员） |
| WhiteboardPanel | `panels/WhiteboardPanel.vue` | ~153 | 白板子组件（便签、连线、画笔） |
| FilePanel | `panels/FilePanel.vue` | ~127 | 共享文件管理（上传、预览、下载） |
| HistoryPanel | `panels/HistoryPanel.vue` | ~60 | 历史记录侧边栏 |
| KnowledgeBasePanel | `panels/KnowledgeBasePanel.vue` | ~239 | 右栏：知识库云盘（文档、标签、版本） |
| AIAssistantPanel | `panels/AIAssistantPanel.vue` | ~57 | AI 助手浮窗 |
| DebateDialog | `panels/DebateDialog.vue` | ~153 | 专家辩论对话框 |
| MultiConsultDialog | `panels/MultiConsultDialog.vue` | ~194 | 多专家咨询对话框 |
| SmartRouteDialog | `panels/SmartRouteDialog.vue` | ~144 | 智能匹配专家对话框 |

---

## 4. API 模块清单

| 文件名 | 导出函数（主要） | 后端端点域 |
|--------|----------------|-----------|
| `http.js` | http 实例、请求/响应拦截器 | 基础 HTTP 封装 |
| `index.js` | API 统一导出入口 | - |
| `alliance.api.js` | 29 个导出（`allianceGetExperts` / `getAllianceTaskLogs` / `cancelAllianceTask` …）| ⚠️存量待迁移：任务域（`/api/alliance/tasks/*`）仍由此文件取数；**三条被禁端点已随 2026-09-27 归一化删除**（`runAllianceFullSSE`→`/api/ai/engine/alliance/full`、`getAllianceCapabilities`→`/ai/engine/alliance/capabilities` 均不在网关 :3080 契约内且零消费者，`getAllianceStats`→`/api/alliance/stats` 为后端恒零桩）；联盟协作/专家/KPI 取数一律改用 `modules/expert-alliance/api/alliance.api.js` |
| `experts.api.js` | `getExperts`, `getExpertGraph`, `getExpertSessions`, `expertDebate`, `multiExpertConsult`, `routeExperts`, `registerExpert` | `/api/experts` 专家管理。⚠️存量：`:20-34` 有 11 个会话端点导出（含 `listExpertSessions` 这个 `@deprecated` 别名），但全库只有 `getExpertSessions` / `createExpertSession` 被调用过 —— detail / stats / update / delete / messages / similar-search / semantic-search / export / archive 九个导出**零消费者**；会话取数改用 `modules/expert-alliance/api/alliance.api.js`，差异见 `FRONTEND-MODULE.md` §7.4 |
| `kb.api.js` | `kbListDocuments`, `kbGetCategories`, `kbGetTags`, `kbSearch`, `kbGetVersions` | `/api/kb` 知识库 |
| `projects.api.js` | `getProjects` | `/api/projects` 项目管理 |
| `graph.api.js` | 图谱数据操作 | `/api/graph` 知识图谱 |
| `ai.api.js` | AI 对话、会话管理 | `/api/ai` AI 助手。⚠️存量：`aiExpertChat`（`/api/ai/expert-chat`）已随 2026-09-27 归一化删除——该分支读 `results`/`summary` 两个子 handler 从不产出的键，`content` 恒退化为兜底串；联盟协作一律走 `modules/expert-alliance/api` 的六模式契约（`/api/experts/*`） |
| `llm.api.js` | LLM 模型管理 | `/api/llm` 大模型 |
| `workflow.api.js` | 工作流定义/执行 | `/api/workflow` 工作流 |
| `operators.api.js` | 算子注册/管理 | `/api/operators` 算子库 |
| `market.api.js` | 市场商品/订单 | `/api/market` 应用市场 |
| `mox.api.js` | MOX 融合平台 | `/api/mox` 融合平台 |
| `system.api.js` | 系统配置、用户、角色、菜单 | `/api/system` 系统管理 |
| `monitor.api.js` | 监控指标、日志 | `/api/monitor` 监控 |
| `actuator.api.js` | 健康检查、指标 | `/actuator` 运维端点 |
| `caomei.api.js` | 草莓 AI 相关 | `/api/caomei` |
| `melody.api.js` | 旋律转谱 | `/api/melody` |
| `workspace.api.js` | 工作台数据 | `/api/workspace` |

---

## 5. Composables 清单

| 文件名 | 路径 | 用途 |
|--------|------|------|
| `useWhiteboard.js` | `composables/workspace/useWhiteboard.js` | 白板状态与交互逻辑 |
| `useGraphCanvas.js` | `composables/workspace/useGraphCanvas.js` | 图谱画布数据与视口控制 |
| `useTaskOrchestration.js` | `composables/workspace/useTaskOrchestration.js` | 任务编排（拆解、分配、执行） |
| `useAlliance.js` | `composables/workspace/useAlliance.js` | 联盟协作 SSE 与消息管理 |
| `useKnowledgeBase.js` | `composables/useKnowledgeBase.js` | 知识库通用逻辑 |
| `useMessageActions.js` | `composables/useMessageActions.js` | 消息操作通用逻辑 |
| `useTheme.js` | `composables/useTheme.js` | 主题切换 |
| `projectContext.js` | `composables/projectContext.js` | 项目上下文 |

---

## 6. 目录结构说明

```
frontend-ui/
├── public/                    # 静态资源
├── src/
│   ├── api/                   # API 接口层（按业务域拆分）
│   │   ├── http.js            # HTTP 实例与拦截器
│   │   ├── index.js           # API 统一导出
│   │   ├── alliance.js        # 专家联盟 SSE
│   │   ├── experts.api.js     # 专家管理
│   │   ├── kb.api.js          # 知识库
│   │   ├── projects.api.js    # 项目管理
│   │   ├── graph.api.js       # 知识图谱
│   │   ├── ai.api.js          # AI 助手
│   │   ├── llm.api.js         # 大模型
│   │   ├── workflow.api.js    # 工作流
│   │   ├── operators.api.js   # 算子库
│   │   ├── market.api.js      # 应用市场
│   │   ├── system.api.js      # 系统管理
│   │   └── ...                # 其他业务 API
│   ├── assets/                # 静态资源（图片、字体）
│   ├── components/            # 全局可复用组件
│   │   ├── common/            # 通用基础组件（8 个）
│   │   ├── layout/            # 布局组件
│   │   ├── expert/            # 专家相关组件
│   │   ├── ai/                # AI 相关组件
│   │   └── ...                # 业务通用组件
│   ├── composables/           # 组合式函数（逻辑复用）
│   │   └── workspace/         # 工作台专用 composables
│   ├── constants/             # 常量定义
│   ├── modules/               # 模块化前端（自包含业务模块，见 §9）
│   │   ├── index.js           # 模块登记入口（路由与导航由此派生）
│   │   ├── _kernel/           # envelope.js 信封归一 + module-registry.js 模块内核
│   │   └── expert-alliance/   # 专家联盟模块：contract / model / api / store / components / views + 测试
│   ├── router/                # 路由配置
│   ├── stores/                # Pinia 状态管理
│   ├── styles/                # 全局样式与设计 token
│   │   └── workspace.css      # 工作台样式（从大组件提取）
│   ├── utils/                 # 工具函数
│   ├── views/                 # 页面视图（按业务域组织）
│   │   ├── admin/             # 管理域
│   │   ├── ai/                # AI 域
│   │   ├── expert/            # 专家域
│   │   ├── graph/             # 图谱域
│   │   ├── market/            # 市场域
│   │   ├── public/            # 游客/裸页面（门户、业务大厅、403）
│   │   ├── operators/         # 算子域
│   │   ├── project/           # 项目域
│   │   ├── workflow/          # 工作流域
│   │   └── workspace/         # 工作台
│   │       ├── ExpertWorkspaceView.vue   # 工作台容器（< 1000 行）
│   │       ├── mockData.js                # 工作台 Mock 数据
│   │       └── panels/                    # 工作台子组件（14 个）
│   ├── App.vue                # 根组件
│   └── main.js                # 应用入口
├── index.html
├── package.json
├── vite.config.js
└── ...
```

### 目录职责说明

| 目录 | 职责 |
|------|------|
| `api/` | 后端接口封装，按业务域拆分，统一通过 `http.js` 发起请求 |
| `components/` | 全局可复用组件，`common/` 为无业务逻辑的基础组件 |
| `composables/` | 组合式函数，封装可复用的状态逻辑，`workspace/` 为工作台专用 |
| `views/` | 页面级组件，按业务域组织子目录；复杂页面使用 `panels/` 子目录拆分 |
| `modules/` | 自包含业务模块（契约+模型+API+store+视图+测试同目录），路由与导航由模块登记派生 |
| `stores/` | Pinia 全局状态管理 |
| `router/` | 路由配置与导航守卫 |
| `styles/` | 全局样式、CSS 变量、设计 token |
| `utils/` | 纯函数工具库 |
| `constants/` | 业务常量、枚举定义 |
| `assets/` | 静态资源（图片、字体、图标） |

---

## 7. 依赖关系图

### 7.1 核心依赖链

```
页面 (views/)
  ├── 调用 → API 模块 (api/*.api.js)
  │     └── 调用 → http.js (axios 实例 + 拦截器)
  │           └── 调用 → 后端 REST / SSE 端点
  ├── 组合 → 通用组件 (components/common/*)
  ├── 组合 → 业务组件 (components/*)
  ├── 使用 → Composables (composables/*)
  │     └── 调用 → API 模块
  └── 依赖 → Stores (stores/*)
        └── 调用 → API 模块
```

### 7.2 工作台依赖链

```
ExpertWorkspaceView (容器, < 1000 行)
  ├── 组合 → WorkspaceHeader / KpiPanel / ExpertPanel
  ├── 组合 → GraphCanvasPanel / TaskOrchestrationPanel
  ├── 组合 → CollaborationPanel
  │     ├── 组合 → WhiteboardPanel / FilePanel / HistoryPanel
  ├── 组合 → KnowledgeBasePanel / AIAssistantPanel
  ├── 组合 → DebateDialog / MultiConsultDialog / SmartRouteDialog
  ├── 组合 → RegisterExpertDialog (components/expert/)
  ├── 使用 → useWhiteboard / useGraphCanvas / useTaskOrchestration / useAlliance
  ├── 调用 → alliance.js (SSE)
  ├── 调用 → experts.api.js
  ├── 调用 → kb.api.js
  ├── 调用 → projects.api.js
  ├── 调用 → graph.api.js
  └── 导入 → workspace.css (样式)
```

### 7.3 通用组件依赖

```
DataTable
  ├── 依赖 → Pagination
  ├── 依赖 → EmptyState
  └── 依赖 → LoadingState

SearchForm → 无内部组件依赖
Pagination → 无内部组件依赖
StatusTag → 无内部组件依赖
EmptyState → 无内部组件依赖
LoadingState → 无内部组件依赖
ConfirmDialog → 无内部组件依赖
PageHeader → 无内部组件依赖
```

---

## 8. 架构设计原则

1. **容器-展示分离**：复杂页面使用容器组件（Container）+ 子面板组件（Panels）模式，容器负责状态管理和数据调度，子组件负责纯展示和交互
2. **逻辑复用**：跨组件共享的状态逻辑提取为 Composables，避免重复代码
3. **API 分层**：所有后端调用通过 `api/` 模块统一封装，页面不直接使用 axios
4. **组件无业务逻辑**：`components/common/` 下的组件为纯展示/交互组件，通过 props 配置，不包含业务逻辑
5. **样式模块化**：大组件的 scoped 样式提取为独立 CSS 文件，便于子组件共享
6. **Mock 数据隔离**：降级用的 Mock 数据独立存放，不与业务逻辑混编
7. **契约归一化**：跨语言契约（阶段/枚举/门限/端点）以 Rust 源码与 `docs/API-REGISTRY.md` 为权威，前端侧由 `src/modules/*/contract/*.test.js` 强制对齐，禁止在页面里散落字面量

---

## 9. 模块化前端（`src/modules/`）

> 权威说明：`docs/expert-alliance/FRONTEND-MODULE.md`（EA-DOC-FE-MODULE，🟢）

| 模块 | 版本 | 页面 | 契约测试 | 状态 |
|------|------|------|---------|------|
| `expert-alliance` | 1.0.0 | `/alliance/console` 联盟控制台（含调度配置表单、调度状态与负载重置、DAG 页签） · `/alliance/experts` 联盟专家广场 · `/alliance/collab` 智能协作工作台 · `/alliance/graph` 专家协作图谱 · `/alliance/sessions` 专家会话中心 · `/alliance/orchestration` 专家编排台（逐字段带来源角标） | 契约、用例与变异电池规模**以 `FRONTEND-MODULE.md` §8 为唯一权威，本行不复述数字**（此处曾写死「跨语言契约 94 例 / 模块 432 例 15 文件 / 全量 515 例 / 电池 78 个」，两批落地后即与实现态不符——同一坑已在索引 V3.1 用「不复述以免腐烂」的口径避掉） | 🟢 已上线；§7 第 1 批（专家广场 + 平台 KPI + 真实指标排行榜）与第 2 批（六模式智能协作）均已落地并与 :3080 真实联调；第 3 批把 `experts.graph.*` 八个已登记端点做成图谱面（含调度配置：网关此前只有 api、无界面），第 4 批把 `experts.session.*` 十一个端点做成会话面 —— 两批落地期间 :3080 停机（`curl` 均 `000` 连接被拒）→ **四面均无实跑证据，会话页从未对真数据渲染过**，核对清单登记在 `FRONTEND-MODULE.md` §8；本批另由契约测试暴露两处后端治理缺口：`GET/PUT /api/experts/dispatcher/config` 与 `POST /api/experts` 三条路由早已挂载却漏登记 `actuator.rs` `ROUTES`，现已补三行（225 → 228）并重生成 `docs/API-REGISTRY.md`；两域 74 条 ready 路由的挂载情况由 `contract/endpoints.js` 的 `UNMOUNTED_ROUTES` 台账锁死（三档计数与覆盖率以 `FRONTEND-MODULE.md` §3.2 为唯一权威，本行不复算；编排台挂载后台账 backlog 已归零，且“backlog 长出一条即判红”是门禁而不是排期，逐条理由给后端行号）；退役核对见 §7.1（广场）/ §7.2（协作，6 处请求/响应缺陷逐条）/ §7.3（图谱，含存量 `findOptimalTeam({question,size})` 被 serde 全丢、`type_distribution` 与 `Math.random()` 坐标等 9 条）/ §7.4（会话，11 条：三处消费方只挂 2/11 端点、响应形态各猜一遍致协作会话面板恒空、`mode`/`type`/`project_id` 假字段、列表投影当详情、HTML 拼接弹窗、本地假搜索与假统计、伪造本地会话顶上），存量 `ExpertPlazaView.vue` / `ExpertCenterView.vue` / `ExpertWorkspaceView.vue` 与 `ExpertEnterprisePanel.vue` 的会话块待用户确认后删除；§7.5 已把「存量联盟面独有能力」逐条分档（A 档联盟域内缺失 / B 档非联盟域须搬家 / C 档前端自造），其中 A 档第一行——专家注册 + 编辑 + 停用面——已于 2026-09-24 落地（17 项控件全部由 `merge_expert_from_value` 的白名单生成，含 `as u32` 截断、`proficiency` 越界整条丢弃、软删单向不可逆三处静默陷阱的界面留痕），故 `/expert-config` 的退役前提已成立，仍只等用户点名 |

### 9.1 新增一个模块页面的步骤

1. `modules/<域>/contract/` 补齐契约（若涉及新枚举/端点，同步写断言到 `contract.test.js`，让 Rust/`API-REGISTRY` 成为裁判）。
2. `model/normalize.js` 增一个 `normXxx`，`api/*.api.js` 增一个方法（不发 `ElMessage`，错误冒泡）。
3. 在 `modules/<域>/index.js` 的 `routes`/`nav` 追加一条（`meta.{title,module,layout}` 必填，`layout ∈ {default, blank}`；nav 图标用 Element Plus 组件名，不得写 emoji、不得带 `count`/`badge`）。
4. 在 `src/modules/index.js` 追加 `import './<域>/index.js'` —— 仅新模块需要。
5. `npx vitest run src/modules && npx vite build`。路由与侧栏无需手改。

### 9.2 内核提供的保证

| 位置 | 保证 |
|------|------|
| `_kernel/envelope.js` | `{code,msg,data}` 与 alliance 族双层壳的幂等剥离；`code!==0` 一律 `ApiError` |
| `_kernel/module-registry.js` | `defineModule` 校验描述完整性与模块重名；`collectRoutes`/`collectNav` 派生路由与导航 |
| `composables/useActiveModule.js` | 模块归属单源（`meta.module` 优先，路径前缀回退） |
| `modules/wiring.test.js` | 装配守卫：模块路由先于通配兜底、nav 已挂载、侧栏 `path` 均有真实路由、无编造计数 |

---

*文档结束*

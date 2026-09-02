# frontend-ui 生产化差距审计报告

> 审计对象：`D:\a10\aikjx\gitcode\infotopograph\frontend-ui`（Vue3 + Vite5 + Element Plus SPA）
> 审计日期：2026-09-01
> 审计方式：只读源码审计，未修改任何文件
> 严重度定义：P0 = 阻断生产上线；P1 = 显著影响企业级可用性；P2 = 体验/规范类改进

---

## 一、路由层审计

### 1.1 路由总览

- 路由文件：`src/router/index.js`（554 行）
- 历史模式：`createWebHashHistory`（hash 路由，生产环境无需服务端 rewrite）
- 全部路由组件均使用 `() => import()` 懒加载
- `scrollBehavior` 回顶已实现
- 404 兜底：`/:pathMatch(.*)*` → redirect `/dashboard`（**无独立 404 页面**）

### 1.2 路由守卫

全局守卫 `router.beforeEach`（L447-522）已实现：
- 白名单 `WHITE_LIST = ['/login','/portal','/hall','/share','/s/','/403']`
- 无 token 跳转 `/login?redirect=`
- 登录后首次 `permissionStore.loadPermissions()`
- `meta.requiresPermission` / `meta.requiresRole` 校验逻辑存在

### 1.3 路由层缺口

- **[P1] `src/router/index.js` L480-519** → 权限校验框架存在，但**几乎所有路由 meta 仅声明 title，未声明 requiresPermission/requiresRole** → 权限校验实际未启用，任何登录用户可访问全部页面。建议：为每个业务路由补充 `meta.requiresPermission` 或 `meta.requiresRole`，并与后端权限点对齐。
- **[P2] `src/router/index.js` L530** → 404 兜底直接 redirect `/dashboard`，无独立 404 页面 → 用户无法感知"路径不存在"。建议：新增 `NotFound.vue` 视图并指向独立路由。
- **[P2] `src/router/index.js` L475** → `permissionStore.loadPermissions()` 失败时仅 `console.warn` 后继续放行 → 权限加载失败时用户仍可进入系统。建议：失败时跳转登录或展示权限错误页。

---

## 二、视图域逐个审计

### 2.1 admin 域（系统管理）

**页面清单**：AdminView（tab 壳）+ panels 下 14 个面板（Overview/User/Role/Department/Access/Audit/Config/Dict/Hitl/Llm/Menu/Monitor/Storage/Docs）

**路由引用**：AdminView 的 TABS 仅 11 项（L35-47），**AdminUser / AdminRole / AdminDepartment 三个面板已实现但未挂 tab，也未被任何路由引用** → 孤儿组件。

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| 孤儿面板 | P1 | `admin/panels/AdminUser.vue` 全文 | 用户管理面板功能完整（CRUD/搜索/分页/角色分配/数据权限树），但 AdminView TABS 无对应项，用户无法从 UI 进入 | 在 AdminView TABS 中补充 user/role/dept 三项，或通过嵌套路由独立挂载 |
| 孤儿面板 | P1 | `admin/panels/AdminRole.vue` 全文 | 角色管理（CRUD/菜单权限树/数据权限/复制角色/角色用户列表），同上不可达 | 同上 |
| 孤儿面板 | P1 | `admin/panels/AdminDepartment.vue` 全文 | 部门树/岗位 CRUD/人员列表，同上不可达 | 同上 |
| 头像 Mock | P1 | `admin/panels/AdminUser.vue` L530-537 | `handleAvatarUpload` 使用 FileReader 本地 base64 预览，注释明确 `// Mock: 使用本地预览`，未调真实上传接口 | 接入文件上传 API（如 `/api/system/upload`），保存返回的 URL |
| 死链按钮 | P1 | `admin/panels/AdminDepartment.vue` L632-636 | `goToUserManage` 仅 `dispatchEvent(CustomEvent('admin:navigate-user'))` + ElMessage.info，无任何监听者和路由跳转 → 点击无实际效果 | 改为 `router.push('/admin?tab=user')` 或直接路由到用户管理页 |
| 无加载态 | P1 | `admin/panels/AdminOverview.vue` L127-136 | onMounted 内 `Promise.all` 并发 5 个 API，**无 v-loading**，5 个 `.catch(() => {})` 全静默 → 加载期间 KPI 显示 `-`，失败无任何提示 | 添加 `v-loading` 指令，catch 中用 ElMessage.error 提示 |
| 错误静默 | P2 | `admin/panels/AdminUser.vue` L374 等多处 | API 错误使用 `console.warn` 无用户提示 | 统一替换为 ElMessage.error |
| 错误静默 | P2 | `admin/panels/AdminRole.vue` 多处 | 同上 | 同上 |
| 错误静默 | P2 | `admin/panels/AdminDepartment.vue` 多处 | 同上 | 同上 |

**admin 域正面项**：AdminAccess/AdminAudit/AdminConfig/AdminDict/AdminHitl/AdminLlm/AdminMenu/AdminMonitor/AdminStorage/AdminDocs 均接真实 API，有 v-loading/el-empty/分页/搜索/表单校验/删除确认，质量较高。

---

### 2.2 ai 域（AI 对话与工具）

**页面清单**：ChatView、ShareSnapshot（复用 ChatView）、CaomeiView、AlgoLabView、InfiniteOptimizerView、BotCenterView、Melody2ScoreView

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| 绕过 API 层 | P1 | `ai/Melody2ScoreView.vue` L290/304/438/450/607/627 | **直接使用 axios 裸调** `/api/melody2score/*`（health/samples/recognize/export-sheet/save-report/download），未走 `@/api` 统一封装和 http 拦截器（token 注入/错误统一处理） | 新建 `src/api/melody.js` 封装所有接口，视图层改为 import 封装函数 |
| 建议文案写死 | P2 | `ai/ChatView.vue` L119-159 | 快捷建议 suggestions 为写死对象数组 | 属正常产品行为，可接受；如需动态化可从后端获取 |
| 对话调用位置 | P2 | `ai/ChatView.vue` 全文 | ChatView 本身无直接 API import，实际对话调用在 `components/ai/AIChatPanel.vue` → 需确认该组件是否接真实接口 | 审计 AIChatPanel 组件确认 API 接入 |

---

### 2.3 expert 域（专家联盟）

**页面清单**：ExpertCenterView（tab 壳）+ panels（ExpertOverviewPanel/ExpertEnterprisePanel/ExpertOrchestratorPanel）+ AllianceTaskView + ExpertConfigView + ExpertPlazaView

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| 大面积硬编码 Mock | **P0** | `expert/panels/ExpertOverviewPanel.vue` L265-290 | **8 位专家全部写死**（林算法/陈架构/王数据/张AI/李工作流/赵图谱/孙安全/周性能），含 `metrics.total_consults`/`success_rate` 等假数据，零 API 调用 | 接入 `getExperts()` 真实接口，移除硬编码数组 |
| 硬编码 Mock | P0 | `expert/panels/ExpertOverviewPanel.vue` L371-376 | 项目进度 `phaseProgress` 写死 75/60/35/15 | 从项目 API 获取真实进度 |
| 硬编码 Mock | P0 | `expert/panels/ExpertOverviewPanel.vue` L385-400 | 知识图谱 canvas 数据写死（7 节点 8 边） | 接入 `getExpertGraph()` 真实接口 |
| 空 watch 实现 | P1 | `expert/panels/ExpertOverviewPanel.vue` L480-482 | `currentPhase` 切换 watch 为空实现，注释"切换阶段时更新智能匹配" → 切换阶段无任何效果 | 实现阶段切换逻辑，调用对应 API 或更新视图 |
| 无加载/错误态 | P1 | `expert/panels/ExpertOverviewPanel.vue` 全文 | 无 v-loading、无 el-empty、无错误提示 | 添加加载态和空态 |
| 错误静默 | P2 | `expert/panels/ExpertOrchestratorPanel.vue` L310/318/328 | API 错误使用 `console.error` 无用户提示 | 替换为 ElMessage.error |

**expert 域正面项**：ExpertOrchestratorPanel 接真实 API（expertOrchestrate/expertGeneratePlan/getOrchestrationStats 等），功能完整。

---

### 2.4 graph 域（知识图谱）

**页面清单**：GraphView、MoxFusionView、FlowGraph

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| 快捷分析跳 AI | P2 | `graph/GraphView.vue` L407-414 | `runQuickAnalysis` 对 centrality/community/path 仅跳转 `/ai` 带 prompt 参数，未直接调用已 import 的 `getCentrality/getCommunities/getShortestPath` 等 API → 图谱内分析能力未闭环 | 在图谱页内直接调用分析 API 并展示结果抽屉（analysisResult 机制已存在） |
| 搜索结果无 UI | P2 | `graph/GraphView.vue` L873-890 | `doSearch` 调用 `graphSearch` 后结果存入 `searchResult`，但 template 中无搜索结果展示区域 → 用户搜索后看不到结果 | 在画布侧或下方添加搜索结果面板 |

**graph 域正面项**：GraphView 接 11 个真实 API（getGraph/getGraphStats/getShortestPath/getNeighbors/recommendNodes/graphSearch/getCentrality/getCommunities/getPagerank/propagateActivation），有 5 段式渐进加载骨架（skeleton→fetch→module→render→physics），ForceGraph3D 动态 import 按需拆 chunk，布局算法 6 种，质量较高。

---

### 2.5 market 域（算子包市场）

**页面清单**：MarketView、MarketDetailView

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| 表单无校验 | P2 | `market/MarketDetailView.vue` L29-34 | 需求描述 textarea 无 `:rules` 校验，仅在 saveAll 时手动检查非空（L252） | 添加 el-form rules 校验规则 |
| 功能点无校验 | P2 | `market/MarketDetailView.vue` L42-56 | 功能点清单的 title/description 输入框无校验，可保存空标题 | 添加必填校验 |

**market 域正面项**：MarketDetailView 接 5 个真实 API（marketGet/marketUpdate/marketClone/marketExport/marketDelete），有 v-loading、el-empty、删除确认、导出回退机制，业务流程图可拖拽编辑，质量较好。

---

### 2.6 misc 域（杂项页面）

**页面清单**：Login、PortalHome、BusinessHall、Forbidden

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| 占位内容 | P2 | `misc/PortalHome.vue` | 门户首页可能含 placeholder/待实现内容（Grep 命中 TODO/placeholder） | 核查并替换为真实内容或移除 |
| 占位内容 | P2 | `misc/BusinessHall.vue` | 业务大厅可能含 placeholder 内容（Grep 命中 TODO/placeholder） | 同上 |

---

### 2.7 operators 域（算子执行）

**页面清单**：OperatorsView

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| 占位/待实现 | P2 | `operators/OperatorsView.vue` | Grep 命中 TODO/placeholder，可能含未实现功能 | 核查具体行并补全或标注 |

---

### 2.8 project 域（项目管理）

**页面清单**：Dashboard、ProjectsView、TaskView、ResourcesView（嵌套 ResourcesOverviewPanel/KnowledgeBasePanel）、Workbench

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| Mock 执行日志 | P1 | `project/Dashboard.vue` L331-348 | `generateMockLogs()` 生成 15 条假执行日志（5 种假工作流、随机成功率/耗时/维度），当真实 API 返回空时自动填充 → 用户看到假数据 | API 返回空时应显示 el-empty"暂无执行记录"，不应自动填充 mock |
| KPI 兜底假值 | P1 | `project/Dashboard.vue` L312-316 | KPI 卡片在 API 失败/无数据时兜底为硬编码值（算子 8/节点 23/执行 15/成功率 98.5%）→ 仪表盘可能展示假数据 | 无数据时显示 `—` 或 0，不应兜底为看起来合理的假值 |
| 阶段进度模拟 | P2 | `project/Dashboard.vue` L214-224 | 项目阶段进度根据当前阶段**模拟**（已完成阶段 100%、当前阶段 65%、未开始 0%），非真实数据 | 从项目 API 获取真实阶段进度 |
| Mock 任务/动态/文档 | **P0** | `project/ProjectsView.vue` L559-629 | `mockTasks`（7 条假任务）、`mockActivities`（6 条假动态）、`mockDocs`（6 条假文档）、`codeLines`（30 行假 Python 代码）全部硬编码，概览/任务/文档/动态 Tab 和深视图 IDE 均展示这些假数据 | 接入任务 API/动态 API/文档 API，深视图 IDE 接入真实代码仓库或移除该功能 |
| Mock 成员 | P1 | `project/ProjectsView.vue` L588-595 | `memberRoles`/`memberSkillSets` 写死，成员头像用项目名首字符生成（L689-694）→ 成员 Tab 展示假成员 | 接入项目成员 API |
| 深视图全假 | P1 | `project/ProjectsView.vue` L316-411 | "进入项目"深视图是完整的假 IDE（假文件树/假代码编辑器/假协作专家/假今日任务/假 AI 建议/假实时预览 metrics）→ 用户点击"进入项目"看到一个完全虚构的开发环境 | 要么接入真实代码仓库/终端/预览，要么移除该入口避免误导 |
| 按钮无实际功能 | P2 | `project/ProjectsView.vue` L733-747 | `toggleFavorite`/`shareProject`/`downloadDoc`/`applySuggestion`/`ignoreSuggestion` 均仅 ElMessage 提示，无实际逻辑 | 接入真实功能或移除按钮 |
| 任务切换无持久化 | P2 | `project/ProjectsView.vue` L725-731 | `toggleTask` 仅修改本地 mockTasks 状态，不调 API | 接入任务状态更新 API |
| Mock 知识库 | P1 | `project/panels/KnowledgeBasePanel.vue` | Grep 命中 mock/TODO，知识库面板可能含假数据 | 核查并接入真实 KB API |
| Mock 资源概览 | P2 | `project/panels/ResourcesOverviewPanel.vue` | Grep 命中 mock/TODO | 核查并接入真实资源 API |

---

### 2.9 workflow 域（工作流编排）

**页面清单**：WorkflowView（tab 壳）+ panels（WorkflowFlowsPanel/PluginsPanel/McpPanel/AutomationPanel）+ BrowserView

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| 占位空组件 | **P0** | `workflow/panels/WorkflowFlowsPanel.vue` L1-9 | template 是空 div + `display:none`，注释自述"内容由父组件 WorkflowView 渲染" → 嵌套路由 `/workflow/flows` 指向一个完全空白的组件，实际内容渲染在父组件 WorkflowView 的 `v-show="outerTab==='flows'"` 里 → 路由与渲染脱节，直接访问 `/workflow/flows` 时父组件 outerTab 可能不是 flows 导致空白 | 将流程编排内容迁移到 WorkflowFlowsPanel，或改为非嵌套路由结构 |
| JSON 手填无校验 | P1 | `workflow/WorkflowView.vue` L27 附近 | "保存/执行" tab 使用手填 JSON 文本（`buildWorkflowPayload` 手动解析），无表单校验仅非空检查 → 用户易输入非法 JSON 导致后端报错 | 改为可视化表单或添加 JSON schema 校验 |
| 插件面板占位 | P2 | `workflow/panels/PluginsPanel.vue` | Grep 命中 TODO/placeholder | 核查并补全 |
| MCP 面板占位 | P2 | `workflow/panels/McpPanel.vue` | Grep 命中 TODO/placeholder | 核查并补全 |
| 浏览器自动化 | P2 | `workflow/BrowserView.vue` | Grep 命中 TODO/placeholder | 核查并补全 |

**workflow 域正面项**：WorkflowView 接 12 个真实 API，AutomationPanel 接真实 API 且有 loading/el-empty/错误提示。

---

### 2.10 workspace 域（专家工作台）

**页面清单**：ExpertWorkspaceView（单文件巨型组件，约 2500+ 行）

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| Mock 专家兜底 | P1 | `workspace/ExpertWorkspaceView.vue` L2182-2220 | `getMockExperts()` 返回 8 位硬编码假专家，API 失败或返回格式不匹配时自动使用 mock → 用户可能看到假专家列表 | API 失败时显示 el-empty + 错误提示，不应静默降级到假数据 |
| Mock 会话兜底 | P1 | `workspace/ExpertWorkspaceView.vue` L2246-2255 | `getMockSessions()` 返回 3 条硬编码假会话，同上 | 同上 |
| KPI 硬编码 | P1 | `workspace/ExpertWorkspaceView.vue` L1955-1960 | 4 个 KPI 卡片值全部写死（在线专家 12/协作会话 28/知识文档 156/进行中任务 7），无 API 调用 | 接入统计 API 动态获取 |
| 项目选择器写死 | P1 | `workspace/ExpertWorkspaceView.vue` L25-29 | 项目下拉选项硬编码 3 个（璇玑知识工程/MOX 平台架构/AI 算法实验室），未从项目列表 API 获取 | 接入 `getProjects()` 动态渲染选项 |
| 共享文件全假 | P1 | `workspace/ExpertWorkspaceView.vue` L2348-2353 | `sharedFiles` 4 条硬编码假文件，上传仅本地 unshift 不调 API（L2363-2377） | 接入文件上传/列表 API |
| 协作成员全假 | P1 | `workspace/ExpertWorkspaceView.vue` L2308-2314 | `collabMembers` 5 个硬编码假成员 | 接入真实协作成员 API |
| 会话消息未接 API | P1 | `workspace/ExpertWorkspaceView.vue` L2257-2271 | `selectSession` 仅插入一条系统消息，未加载历史消息；`sendCollabMsg` 未审计是否接真实 SSE/API | 接入会话消息历史 API 和发送 API |
| 文件上传仅本地 | P1 | `workspace/ExpertWorkspaceView.vue` L2363-2377 | `handleBeforeFileUpload` 仅本地 unshift 到 sharedFiles，`return false` 阻止自动上传 → 文件实际未上传到服务器 | 接入真实文件上传 API |
| 预览/下载假功能 | P2 | `workspace/ExpertWorkspaceView.vue` L2411-2421 | `previewFile`/`downloadFile` 仅 ElMessage 提示，无实际逻辑 | 接入真实预览/下载 |
| 白板无持久化 | P2 | `workspace/ExpertWorkspaceView.vue` L2432-2500+ | 白板（便签/文本/连线/画笔）全部纯前端内存操作，`saveWhiteboard` 未审计是否接 API，刷新即丢失 | 接入白板保存 API 或 localStorage 持久化 |
| 项目阶段写死 | P2 | `workspace/ExpertWorkspaceView.vue` L2331-2338 | `projectPhases` 5 阶段写死，`currentProjectPhase` 初始为 1 | 从项目 API 获取真实阶段 |
| 单文件过大 | P2 | `workspace/ExpertWorkspaceView.vue` 全文 | 单文件 2500+ 行，template/script/style 全在一个文件，可维护性差 | 拆分为子组件（左栏/中栏/右栏/协作栏/对话框等） |

---

## 三、组件层审计（src/components/）

**组件清单**（19 个）：
- 根级 13 个：AgentFlowPanel、AgentTaskRunner、AssistantSelector、FlowDetailDialog、MessageBubble、NotificationCenter、OnboardingGuide、PhasePipeline、ProjectChip、ProjectPicker、SessionSidebar、SkeletonLoader、ThemeSwitcher
- ai/：AIChatPanel（核心对话组件）
- expert/：RegisterExpertDialog
- layout/：IconSidebar、TabBar、TheSidebar、TheTopbar

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| 组件复用率待核查 | P2 | `src/components/` 全文 | 部分通用组件（AgentFlowPanel/AgentTaskRunner/PhasePipeline/OnboardingGuide）可能仅在 1 处使用或未使用 | 全局 Grep 组件名确认复用情况，未使用的移除或标注 |
| 占位组件待核查 | P2 | `src/components/OnboardingGuide.vue` | 新手引导组件可能含 placeholder 步骤 | 核查内容真实性 |

**组件层正面项**：AIChatPanel 是核心对话组件（ChatView 依赖），RegisterExpertDialog 被 ExpertWorkspaceView 引用，ThemeSwitcher 负责主题切换，layout 4 组件构成主框架。

---

## 四、状态层审计（src/stores/）

**Store 清单**（8 个）：ai、app、auth、index、permission、project、ui、user

| Store | 接真实 API | 持久化 | 备注 |
|-------|-----------|--------|------|
| ai.store.js | 是 | 是（localStorage） | 会话/助手本地管理 |
| auth.store.js | 是 | 是（localStorage） | token/用户信息 |
| permission.store.js | 是 | 是（localStorage） | 权限点/角色 |
| project.store.js | 是 | 是（localStorage） | 当前项目上下文 |
| app.store.js | 否 | 是（localStorage） | 主题/侧边栏状态等 UI 偏好 |
| ui.store.js | 否 | 否 | 纯内存 UI 状态 |
| user.store.js | 否 | 否 | 纯内存用户状态（可能与 auth 重复） |
| index.js | — | — | store 入口/聚合 |

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| user store 纯内存 | P2 | `stores/user.store.js` | 无持久化、无 API 调用，可能与 auth.store 功能重叠 | 合并到 auth.store 或补充持久化/API |
| ui store 纯内存 | P2 | `stores/ui.store.js` | 无持久化，刷新后 UI 状态丢失 | 关键状态（如侧边栏折叠）持久化到 localStorage |

---

## 五、全局特性审计

### 5.1 暗色主题

- 主题文件：`src/styles/` 下 4 个 CSS（index.css、cyberpunk.css、dark.css、sky.css）
- 主题切换：`components/ThemeSwitcher.vue` + `composables/useTheme.js`
- `app.store.js` 持久化主题选择到 localStorage
- **结论**：暗色主题切换机制完整，4 套主题可用。

### 5.2 响应式断点

- ChatView 有 768px 断点（侧栏抽屉化，L461-470）
- GraphView 有 900px/1100px 断点
- Dashboard 有 grid 响应式布局
- **缺口**：**[P2] 全局无统一断点体系**（无 CSS 变量定义 sm/md/lg/xl 断点），各页面自行硬编码断点值 → 一致性差。建议：在 global.css 定义统一断点变量或引入响应式工具类。

### 5.3 i18n / 多语言

- **Grep 结果：`i18n|useI18n|\$t\(` 在 src 下 0 文件命中**
- **结论**：**完全无国际化框架**，所有文案硬编码中文。企业级产品如需多语言支持，需引入 vue-i18n 并提取全部文案。
- **[P2] 全局** → 无 i18n 基础设施 → 如需出海/多语言，需从零搭建。

### 5.4 无障碍（a11y）

- Grep `aria-|role=|tabindex=` 仅 4 个文件命中（directives/permission.js、components/MessageBubble.vue、styles/global.css、components/ProjectChip.vue）
- **结论**：**无障碍基础几乎为零**——无 aria-label、无 role 语义、无键盘导航规划、无 focus 管理。
- **[P1] 全局** → 关键交互元素（按钮/表单/对话框）缺少 aria 标注和键盘可达性 → 企业级合规（如 WCAG）不达标。建议：优先为表单控件、导航菜单、对话框添加 aria-label 和键盘操作支持。

### 5.5 权限指令

- `src/directives/permission.js` 定义了 `v-permission` 指令
- **Grep 结果：`v-permission` 在 src 下仅 1 个文件命中（即指令定义文件本身）**
- **结论**：**权限指令定义了但零使用**——没有任何视图/组件用 `v-permission` 控制按钮/菜单显隐。
- **[P1] `src/directives/permission.js` + 全局** → 权限指令未落地 → 按钮级权限控制缺失。建议：在关键操作按钮（删除/导出/配置等）上添加 `v-permission="'system:user:delete'"` 等指令。

---

## 六、构建配置审计

### 6.1 package.json

| 项目 | 值 |
|------|-----|
| name | operator-unified-system-frontend |
| 端口 | 3020（dev） |
| scripts | dev / build / preview / lint |

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| lint 脚本兜底 | P2 | `package.json` L11 | lint 脚本为 `eslint --ext .js,.vue src --fix 2>/dev/null \|\| echo eslint_not_installed`，eslint 不在 devDependencies → 实际运行走兜底分支，lint 形同虚设 | 将 eslint + eslint-plugin-vue 加入 devDependencies，移除兜底 |
| pinia 依赖位置 | P2 | `package.json` L39 | pinia 在 devDependencies 而非 dependencies → 生产构建时可能被 tree-shaking 或误判 | 移至 dependencies |

### 6.2 vite.config.js

| 缺口 | 严重度 | 位置 | 问题描述 | 建议方向 |
|------|--------|------|----------|----------|
| API 代理写死 | P1 | `vite.config.js` L108-111 | `/api` 代理目标写死 `localhost:8080` → 生产环境需通过网关或环境变量配置 | 使用 `process.env.VITE_API_BASE_URL` 环境变量，生产构建时注入真实网关地址 |

---

## 七、汇总表

| 视图域 | 页面数 | 已接真实接口数 | 含 mock 数 | 缺状态数 | 死链数 |
|--------|--------|----------------|-----------|---------|--------|
| admin | 15 | 12 | 1（头像） | 1（Overview） | 1（部门→用户） |
| ai | 7 | 5 | 0 | 0 | 0 |
| expert | 7 | 2 | 1（Overview 全假） | 1（Overview） | 0 |
| graph | 3 | 1 | 0 | 0 | 0 |
| market | 2 | 2 | 0 | 0 | 0 |
| misc | 4 | 1 | 2 | 1 | 0 |
| operators | 1 | 1 | 1 | 0 | 0 |
| project | 7 | 3 | 3（Dashboard/ProjectsView/KnowledgeBase） | 1 | 0 |
| workflow | 7 | 3 | 2（Plugins/MCP） | 1 | 1（FlowsPanel 空） |
| workspace | 1 | 0 | 1（全页大面积 mock） | 1 | 0 |
| **合计** | **64** | **30** | **11** | **6** | **2** |

> 注：页面数含 panels 子组件；"已接真实接口数"指该域内至少调用了一个 `@/api` 函数的页面数；"含 mock 数"指存在硬编码假数据且在生产路径上可能展示的页面数。

---

## 八、P0 级阻断项汇总（必须修复后方可生产上线）

1. **`expert/panels/ExpertOverviewPanel.vue`** — 专家概览全页硬编码 8 位假专家 + 假进度 + 假图谱，零 API 调用。
2. **`project/ProjectsView.vue`** — 任务/动态/文档/代码编辑器全部 mock，"进入项目"深视图是完全虚构的 IDE 环境。
3. **`workflow/panels/WorkflowFlowsPanel.vue`** — 嵌套路由指向空 div + display:none 的占位组件，路由与渲染脱节。

---

## 九、P1 级高优先级项汇总

1. 路由权限校验框架存在但全路由 meta 未声明权限点 → 实际未启用。
2. admin 域 3 个孤儿面板（User/Role/Department）功能完整但 UI 不可达。
3. AdminDepartment "跳转用户管理"死链按钮。
4. AdminOverview 无加载态 + 5 个 API 错误全静默。
5. Melody2ScoreView 绕过 @/api 层直接 axios 裸调。
6. ExpertOverviewPanel 无加载/错误/空态 + 空 watch 实现。
7. Dashboard KPI 兜底假值 + mock 执行日志自动填充。
8. ProjectsView mock 成员 + 按钮无实际功能 + 任务切换无持久化。
9. WorkflowView JSON 手填无校验。
10. ExpertWorkspaceView 大面积 mock（专家/会话/KPI/项目/文件/成员）+ 文件上传仅本地 + 会话消息未接 API。
11. v-permission 指令定义但零使用。
12. 无障碍基础几乎为零。
13. vite.config.js API 代理写死 localhost:8080。

---

*审计完成。本报告仅做差距识别，未包含代码修复方案。*

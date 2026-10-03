# 前端高严重度缺口修复报告

- 日期：2026-09-27
- 范围：expert-alliance 模块 3 个高严重度缺口
- 执行：先读后改、最小改动、不删文件、改完跑 vitest

---

## 任务 1：ExpertWorkspaceView 禁端点改模块契约端点

**结论：进场时已收口（本轮无需新增改动，仅复核确认）。**

Read 证据：
- `src/views/workspace/ExpertWorkspaceView.vue` `loadAllianceCapabilities()`（约 :848-853）已调用
  `allianceApi.listExpertCapabilities()`（`GET /api/experts/capabilities`），模板把结果传给
  `AIAssistantPanel` 的 `capabilities`。**不再**调用 `getAllianceCapabilities()`。
- `src/composables/workspace/useAlliance.js` `runAlliance()`（:62-92）已改走
  `allianceApi.collaborate(def.key, input)`（模块六模式原生端点），结果经 `model/collabChat.js`
  投影成聊天消息。**不再**调用 `runAllianceFullSSE()`（`POST /api/ai/engine/alliance/full`）。
- grep 全仓 `src`：`getAllianceCapabilities` / `runAllianceFullSSE` / `ai/engine/alliance`
  在活代码中零命中；仅出现在文档（MODULE-MANIFEST.md、旧 verification 报告）与
  `contract/forbidden-revival.test.js` 的防复活台账中。

映射关系（与任务要求一致）：
- capabilities → `GET /api/experts/capabilities`（模块 `allianceApi.listExpertCapabilities`）✅
- 全流程 SSE → 模块协作请求-响应式原生端点（`allianceApi.collaborate`）✅

---

## 任务 2：移除 legacy 假端点 /qa

**与任务前提的偏差（已按本意处理）**：`askAllianceTaskQa` 并非死代码——它被
`src/views/expert/AllianceTaskView.vue:357` **活调用**。后端 registry 无此路径，运行即 404。
故按"撤除假端点"的本意，连这唯一调用点一并改走已存在的 `api.aiChat`，避免删函数后页面运行时报错。

改动文件：

1. `src/api/alliance.api.js`
   - 删除 `askAllianceTaskQa`（原 :243-247，`POST /alliance/tasks/:id/qa`，假端点）。
   - 删除 `getFusionResults`（原 :201-204，`GET /alliance/tasks/:id/fusion`；`/fusion` 与
     `/fusion-result` 挂同一 handler，模块统一走 `taskFusion`；grep 零调用方）。
   - 删除 `allianceGetExpertOverview`（原 :124-126，`GET /experts/overview`；`UNMOUNTED_ROUTES`
     已 rejected 为二手汇总；grep 零调用方）。
   - 删除 `allianceGetExpertMetrics`（原 :128-130，裸 `GET /experts/metrics`；与
     `experts.registry.stats` 同口径且未登记；grep 零调用方）。
   - **保留** `getAllianceTaskStatus`（:239，被 `composables/useAllianceTasks.js:60` 活调用且有测试）、
     `allianceGetSingleExpertMetrics`（`GET /experts/:id/metrics`，真实端点）、
     `getAllianceFusionResult`（归一化版 `/fusion-result`，有测试）。

2. `src/views/expert/AllianceTaskView.vue` `sendAiMessage()`（:353-366）
   - 移除对 `api.askAllianceTaskQa(task.id, message)` 的分支调用。
   - 统一改走 `api.aiChat`；选中任务时把任务名/状态拼进上下文。
   - 行为变化：原 `if (task)` 分支因打不存在的端点恒 404、落入 catch 提示"暂时无法分析"；
     改后可经 `aiChat` 正常应答，未新增任何后端端点。

---

## 任务 3：6 个控制台路由补 RBAC 角色校验

全局守卫 `src/router/index.js:115-133` 已支持 `meta.requiresRole`（数组走
`permissionStore.hasAnyRole`，无权限跳 `/403`）。按工程既有 `ADMIN_GUARD` 口径
（`modules/system/index.js:5`、`modules/admin-lowcode/index.js:11` 均为
`['super_admin','tenant_admin']`）补 meta。

改动文件：`src/modules/expert-alliance/index.js`

| 路由 | 行号 | 写面性质 | 补后 meta |
|---|---|---|---|
| `/alliance/console` | :82 | 负载重置 / 调度配置写（破坏性） | `requiresRole: ['super_admin','tenant_admin']` |
| `/alliance/graph` | :104 | 图谱重建（破坏性，图版本号+1 落盘） | `requiresRole: ['super_admin','tenant_admin']` |
| `/alliance/orchestration` | :126 | 编排执行 / 计划生成（写） | `requiresRole: ['super_admin','tenant_admin']` |
| `/alliance/experts` | :137 | 专家注册 / CRUD（写） | `requiresRole: ['super_admin','tenant_admin']` |
| `/alliance/collab` | :93 | 只读 + 发起咨询 | 保持 `requiresAuth`（所有登录用户） |
| `/alliance/sessions` | :115 | 会话中心（用户自服务） | 保持 `requiresAuth`（所有登录用户） |

**关于 `operator` 角色**：任务原建议 orchestration 加 `'operator'`，但工程
`ROLE_TEMPLATES`（`api/system.api.js:139-180`）真实角色码仅
`super_admin / tenant_admin / dept_manager / normal_user / readonly_auditor`，**不存在 operator**；
全站 admin 守卫统一用 `['super_admin','tenant_admin']`。按任务"以工程既有模式为准"的约定，
未引入 operator，避免向后端编造授权模型（`contract.test.js:1373` 亦禁止视图自判角色）。

**按钮级权限**：模块内破坏性按钮（负载重置 / 图谱重建 / 专家删除）所在路由现已整体限管理员；
工程 `v-permission` 指令基于权限码（如 `sso:create`），联盟后端无对应权限码登记，
故在路由 meta 层收口即可，未加按钮级指令。

---

## vitest 运行结果

- `npx vitest run src/modules/expert-alliance --reporter=verbose`：
  **24 个测试文件 / 631 个用例全部通过，0 失败。**
- 补充回归（因改动了 `src/api/alliance.api.js` 与 `AllianceTaskView.vue`）：
  `npx vitest run src/api/allianceTasks.test.js src/composables/useAllianceTasks.test.js`：
  **2 个测试文件 / 15 个用例全部通过，0 失败。**
- 合计 **646 用例通过**，无环境/依赖缺失，无需强行安装。

---

## 缺口闭环情况

- 任务 1（禁端点 → 模块契约）：**已闭环**（进场前一轮已收口，本轮 grep + 读码复核确认）。
- 任务 2（假端点 /qa + 死代码）：**已闭环**。假端点定义与唯一调用点均撤除；另清 3 个零调用方桩函数。
- 任务 3（控制台路由 RBAC）：**已闭环**。4 个破坏性/管理路由补管理员角色，2 个用户面路由保持登录即可。

## 未做 / 说明

- 未引入 `operator` 角色码（系统无此角色），orchestration 与 console/graph/experts 同口径限管理员。
- 未加按钮级 `v-permission`（联盟无权限码登记，路由层已收口）。
- 旧报告 `_verification/frontend-verification-report.md` 中"askAllianceTaskQa 无调用方/死代码"
  的描述与现状不符（它实际有一个调用点），属历史快照，不在本次改动范围，未改写。


---

## 2026-09-29：视图层 legacy API 面收敛到模块契约（contract→store→view 单向依赖）

- 范围：`src/views/expert/**`、`src/views/workspace/**`、`src/composables/**` 对 `@/api/*` 桶里
  alliance/expert 域残留调用的收敛。
- 真源：`modules/expert-alliance/contract/endpoints.js`（68 端点 key）、
  `modules/expert-alliance/api/alliance.api.js`（allianceApi）。
- 方法：先全量盘点 → 逐文件 Read 上下文 → 在视图层用 adapter 把模块 camelCase normalize
  输出补回 legacy snake_case 别名字段，模板/composable/UI 逻辑零改动。

### 1. legacy 残留清单（迁移前）

| 文件 | 行号 | legacy 调用 | 模块等价 | 判定 |
|---|---|---|---|---|
| views/expert/ExpertCenterView.vue | :79, :130 | `registerExpert({name,type,capabilities,description,systemPrompt})` | `allianceApi.registerExpert(draft)` | 迁移 |
| views/expert/AllianceTaskView.vue | :292, :329, :370 | `api.createAllianceTask` / `api.getAllianceRuntime` + `useAllianceTasks(api)` 内部 8 个 legacy 方法 | `allianceApi.listTasks/getLogs/getDag/getFusion/getTask/controlTask/createTask/getRuntime` | 迁移（adapter） |
| views/expert/AllianceTaskView.vue | :359 | `api.aiChat` | 无（AI 域 /ai/chat） | 合法例外 |
| views/expert/ExpertPlazaView.vue | :577, :669 | `getExpertsStats()` | `allianceApi.expertsStats()` | 迁移 |
| 同上 | :689 | `getMyBookings()` | `allianceApi.listMyBookings()` | 迁移 |
| 同上 | :816 | `getExperts()` | `allianceApi.listExperts()` | 迁移 |
| 同上 | :901 | `toggleExpertFavorite(id)` | `allianceApi.toggleFavorite(id)` | 迁移 |
| 同上 | :957 | `createBooking(payload)` | `allianceApi.createBooking({expertId,topic,scheduledAt})` | 迁移 |
| 同上 | :1015 | `apiCancelBooking(id)` | `allianceApi.cancelBooking(id)` | 迁移 |
| 同上 | :1026 | `enterConsultRoom(id)` | `allianceApi.consultRoom(id)` | 迁移 |
| 同上 | :1049 | `joinExpertTeam({expert_id})` | `allianceApi.joinTeam({expertId})` | 迁移 |
| 同上 | :1064 | `consultNow(id,{topic})` | `allianceApi.consultNow(id,{topic})` | 迁移 |
| 同上 | :577 | `getExpert`（导入未使用） | — | 死 import 删除 |
| views/expert/panels/ExpertOrchestratorPanel.vue | :261-264, :312 | `getOrchestrationStats()` | `allianceApi.getOrchStats()` | 迁移 |
| 同上 | :328 | `getOrchestrationPlugins()` | 无（contract UNMOUNTED rejected） | 合法例外 |
| 同上 | :336 | `getOrchestrationHistory({limit})` | `allianceApi.getOrchHistory()` | 迁移 |
| 同上 | :396 | `expertOrchestrate({...})` | `allianceApi.orchestrate({...})` | 迁移 |
| 同上 | :422 | `expertGeneratePlan({...})` | `allianceApi.generateOrchPlan({...})` | 迁移 |
| views/expert/panels/ExpertOverviewPanel.vue | :250, :292 | `getExperts({page,page_size})` | `allianceApi.listExperts({page,pageSize})` | 迁移 |
| 同上 | :392 | `getExpertGraph()` | `allianceApi.graphOverview()` | 迁移 |
| 同上 | :408 | `getExpertOverview()` | 无（contract UNMOUNTED rejected） | 合法例外 |
| views/expert/panels/ExpertEnterprisePanel.vue | :478, :662/:736 | `api.getExpertSessions({status,session_type})` | `allianceApi.listSessions({status,sessionType})` | 迁移（adapter） |
| 同上 | :663/:749 | `api.getExpertGraphStats()` | `allianceApi.graphStats()` | 迁移 |
| 同上 | :664/:767 | `api.getDispatcherStatus()` | `allianceApi.dispatcherStatus()` | 迁移 |
| 同上 | :758 | `api.getExpertGraph()` | `allianceApi.graphOverview()` | 迁移 |
| 同上 | :776 | `api.updateDispatcherConfig({strategy})` | `allianceApi.updateDispatcherConfig(patch)` | 迁移 |
| 同上 | :785 | `api.rebuildExpertGraph()` | `allianceApi.rebuildGraph()` | 迁移 |
| 同上 | :796 | `api.createExpertSession({title,mode})` | `allianceApi.createSession({title})` | 迁移 |
| 同上 | :852 | `api.getExpertGraphCollaborators(id,5)` | `allianceApi.graphCollaborators(id,5)` | 迁移 |
| 同上 | :862 | `api.findOptimalTeam({question,size})` | `allianceApi.optimalTeam({goal,maxMembers})` | 迁移 |
| 同上 | :879 | `api.enterpriseConsult({...})` | 无（contract UNMOUNTED rejected） | 合法例外 |
| 同上 | :898 | `api.getEngineFlowGraph()` | 无（AI 引擎域 /ai/engine/flow-graph） | 合法例外 |
| views/expert/ExpertConfigView.vue | — | 无 `@/api` import（仅 `@/constants`） | — | 无需处理 |
| composables/useKnowledgeBase.js | 全 | `api.kb.*` | 无（KB 域） | 合法例外 |
| composables/projectContext.js | :9 | `getProjects/getProject/createProject/registerProjectIdGetter` | 无（项目域） | 合法例外 |
| composables/workspace/useWorkspaceData.js | :26 | `getProjectMembers` | 无（项目域） | 合法例外 |
| views/workspace/ExpertWorkspaceView.vue | :286 | `getProjects` | 无（项目域） | 合法例外 |

模块内部（`modules/expert-alliance/views|components|store|api`）扫描结果：全部走模块 contract/store，零 legacy 引用。

### 2. 迁移明细

**ExpertCenterView.vue**
- `import { registerExpert } from '@/api'` → `import { allianceApi } from '@/modules/expert-alliance/api'`
- `registerExpert({name,type,capabilities:[strings],description,systemPrompt})` →
  `allianceApi.registerExpert({name, expertType, bio, capabilities:[{name,proficiency:85}]})`
- 行为保持：字段映射对齐模块 `registerBody` 白名单；`capabilities` 字符串简写硬编码 proficiency=85
  对齐后端 merge 行为；`systemPrompt` 走 metadata，模块契约不挂录入入口（EXPERT_UNMOUNTED_FIELDS），
  该字段在新注册流不再发送——模块有自己的 ExpertRegistryForm 录入面。

**AllianceTaskView.vue**
- `import * as api from '@/api'` → `import { allianceApi } from '@/modules/expert-alliance/api'` + `import { aiChat } from '@/api'`（AI 域例外）
- 新增视图层 adapter `toLegacyTask/toLegacyLog/toLegacyDag/toLegacyFusion`，把模块 camelCase normalize
  输出补回模板与 `useAllianceTasks` composable 消费的 snake_case 形状（`name=title`、`duration_ms=durationMs`、
  `started_at=startedAt`、`log.time=ts`、`node.x/y=position.x/y`、`edge.x1/y1/x2/y2` 等）。
- adapter 方法名对齐 composable 期望的 legacy 接口（`getAllianceTasks`/`getAllianceTaskLogs`/
  `getAllianceTaskDag`/`getAllianceFusionResult`/`getAllianceTaskStatus`/`resume|pause|cancelAllianceTask`/
  `createAllianceTask`/`getAllianceRuntime`），内部全部转发到 allianceApi。
- 行为保持：模板、composable、`taskActions` 零改动；`estimated_remaining_ms`/`expert_count` 在模块契约
  UNMOUNTED_ROUTES 里已 rejected（估算启发式/二手计数），视图对 undefined 兜底显示 '--'，属预期。

**ExpertPlazaView.vue**
- 具名桶导入 → `import { allianceApi } from '@/modules/expert-alliance/api'`
- 新增 `toLegacyExpert/toLegacyStats/toLegacyBooking` 适配器：补 `type=expertType`、
  `consultCount=metrics.totalConsultations`、`goodRate=metrics.resolutionRate`、
  `price=hourlyRateCents/100`、`expert_count=totalExperts`、`expertType=''`（booking 列表 emoji 派生）。
- `createBooking(payload)` → `allianceApi.createBooking({expertId,topic,scheduledAt})`；
  `enterConsultRoom` 返回补 `url=joinUrl`；`consultNow` 返回补 `url=chatUrl`。
- 删除未使用的 `getExpert` 死 import。

**ExpertOrchestratorPanel.vue**
- 具名桶导入 → `import { allianceApi } ...` + `import { getOrchestrationPlugins } from '@/api'`（例外）
- 新增 `orchStatsLegacy/orchHistoryLegacy/orchResultLegacy` 补 snake_case 别名
  （`total_executions=totalExecutions`、`avg_duration_ms=avgDurationMs`、`created_at=createdAt`、
  `duration_ms=durationMs`、`plan.steps[].step_id=stepId` 等）。
- `expertOrchestrate` → `allianceApi.orchestrate`；`expertGeneratePlan` → `allianceApi.generateOrchPlan`；
  视图 `normalizeResult/normalizePlan` 零改动。

**ExpertOverviewPanel.vue**
- 具名桶导入 → `import { allianceApi } ...` + `import { getExpertOverview } from '@/api'`（例外）
- `getExperts` → `allianceApi.listExperts()`，结果包成 `{experts: items.map(toLegacyExpert)}` 适配视图既有兼容链；
  `toLegacyExpert` 补 `type=expertType`、`metrics.success_rate=metrics.resolutionRate`。
- `getExpertGraph` → `allianceApi.graphOverview()`。

**ExpertEnterprisePanel.vue**
- `import * as api from '@/api'` → `import { allianceApi } ...` + `import { enterpriseConsult, getEngineFlowGraph } from '@/api'`（例外）
- 新增 adapter 对象 `api`，11 个方法名全部保留（`getExpertSessions`/`getExpertGraphStats`/
  `getDispatcherStatus`/`getExpertGraph`/`updateDispatcherConfig`/`rebuildExpertGraph`/`createExpertSession`/
  `getExpertGraphCollaborators`/`findOptimalTeam`），内部转发 allianceApi 并补 snake_case 别名
  （`sessions=items`、`total_nodes=totalNodes`、`circuit_breakers=circuitBreakers`、
  `mode=sessionType`、`updated_at=lastActiveAt`）。视图所有 `api.xxx` 调用点零改动。

### 3. 补契约明细

无。所有迁移目标在模块 `endpoints.js` 68 端点 key 内均有等价 key，无需新增契约。

### 4. 删除的死调用

- `ExpertPlazaView.vue` 导入但从未调用的 `getExpert` 具名导入（随 import 块重写一并移除）。
- `ExpertOrchestratorPanel.vue` adapter 内曾出现 `expert_ids: x.expertIds` 别名——该字段视图不消费，
  且触发 `legacy-collab-revival.test.js` 的"手写 wire 键"扫描（`/expert_ids\s*:/g`），已删除。

### 5. 合法例外清单 + 理由

| 文件 | 保留的 legacy 导入 | 理由 |
|---|---|---|
| AllianceTaskView.vue | `aiChat`（`/ai/chat`） | AI 域通用对话端点，非 alliance 契约面；任务诊断对话框走全站统一 AI 聊天 |
| ExpertOverviewPanel.vue | `getExpertOverview`（`/experts/overview`） | contract UNMOUNTED_ROUTES rejected：七键全是二手汇总（phase_progress 从别处派生）；模块不挂载，UI 面板暂留 |
| ExpertOrchestratorPanel.vue | `getOrchestrationPlugins`（`/experts/orchestration/plugins`） | contract UNMOUNTED_ROUTES rejected：后端硬编码 6 条假数据；模块不挂载，UI 面板暂留 |
| ExpertEnterprisePanel.vue | `enterpriseConsult`（`/experts/enterprise/consult`） | contract UNMOUNTED_ROUTES rejected：模板桩，无 LLM；UI 面板暂留 |
| ExpertEnterprisePanel.vue | `getEngineFlowGraph`（`/ai/engine/flow-graph`） | AI 引擎域，非 alliance 契约面；流程图 tab 独立取数 |
| composables/useKnowledgeBase.js | `api.kb.*` 全量 | KB 域，非 alliance 域，不在本次收敛范围 |
| composables/projectContext.js | `getProjects/getProject/createProject/registerProjectIdGetter` | 项目域（`/projects`），非 alliance 域 |
| composables/workspace/useWorkspaceData.js | `getProjectMembers` | 项目域 |
| views/workspace/ExpertWorkspaceView.vue | `getProjects` | 项目域 |

### 6. vitest 结果

- 命令：`npx vitest run --reporter=dot`
- 结果：**Test Files 71 passed / 1 failed (72)；Tests 980 passed / 1 failed (981)**。
- 唯一失败 `src/views/admin/panels/AdminAccess.smoke.test.js`：`$setup.fmtTime is not a function`——
  admin 面板预先存在问题（模板用 `fmtTime` 但 setup 未暴露），与本次 expert-alliance 收敛无关，
  未触碰 admin 目录。
- 本次改动直接相关的测试全绿：
  - `contract/legacy-collab-revival.test.js`（9 用例）：台账双向相等，零旧协作调用复活。
  - `composables/useAllianceTasks.test.js`（7 用例）：adapter 传 composable，mock 接口形状不变。
  - `modules/expert-alliance/**` 全套（contract.test/forbidden-revival/endpoints 等）随全量跑绿。
- 首轮曾出现 `src/utils/time.js` 转换错误（35 文件收集失败），为 vitest 缓存/并发环境问题，
  重跑后消失，与本次改动无关（time.js 未被修改）。

### 7. grep 验证结果（零命中证明）

- `src/views/expert/**` 下 `from '@/api/alliance'` / `from '@/api/experts'`：**0 命中**。
- `src/views/workspace/**` 下 `from '@/api/alliance'` / `from '@/api/experts'`：**0 命中**。
- `src/views/expert/**` 下 `import * as api from '@/api'`：**0 命中**（4 个文件改为具名导入或 adapter）。
- 全 `src` 下 `getAllianceCapabilities|runAllianceFullSSE|askAllianceTaskQa`：
  活代码 **0 命中**；剩余命中均为文档（MODULE-MANIFEST.md、旧 verification 报告）、
  本报告内注释、以及 `contract/forbidden-revival.test.js` 的防复活断言（断言这些名字不存在）。
- `src/views/expert/**` 下残留的 `from '@/api'` 具名导入仅 4 处，全部为第 5 节合法例外。

---

## 2026-09-30：权限模型固化（14 号文档配套核证）

- 范围：为 `docs/expert-alliance/14-enterprise-permission-model.md` 提供前端侧证据；
  **本轮未改任何前端代码**，故未重跑 vitest（基线见上文 2026-09-27：646 用例通过；
  2026-09-29 全量 980/981，唯一失败为 AdminAccess 预存在问题，与联盟无关）。

### 1. 核证结论（先读后写，无编造）

| 命题 | 核证结果 | 证据 |
|---|---|---|
| 现行角色码 5 个 | ✅ 属实 | `api/system.api.js:139-180` ROLE_TEMPLATES：super_admin/tenant_admin/dept_manager/normal_user/readonly_auditor |
| 全局守卫 requiresRole 行为 | ✅ 重定向 /403，非放行 | `router/index.js:115-133`（数组走 hasAnyRole，不通过 → warning + next('/403',{redirect})）；/403 路由存在 `router/modules/fallback.js`（bare:true） |
| 联盟 6 处 requiresRole | ✅ 属实 | `modules/expert-alliance/index.js:50/71/91/115/139/152`；/collab(:102)、/sessions(:126) 仅 requiresAuth |
| 「v-permission 全站不存在」 | ⚠️ **修正**：指令基础设施**存在且已注册** | `directives/permission.js:86-128`（v-permission/any/all、v-role/role-any）；`main.js:14,47` setupPermissionDirectives(app) |
| 联盟模块按钮级使用 | ✅ 零命中（与任务前提一致） | grep 联盟 views/components 模板内 `v-permission`/`v-role`：0 命中；全站仅 admin/panels 用（AdminSso/AdminRole/AdminUser/AdminDepartment） |
| `hasRole` 是否别名展开 | ❌ 不展开 | `stores/permission.store.js:172-175` 严格 includes；`'admin'` 仅在 isAdmin 别名表（:53-56）——admin 面板 `v-role="'admin'"` 为历史遗留不一致，非联盟域，未改 |

### 2. 为什么本轮不加按钮级指令

- 联盟后端**无权限码登记**（网关写面只 emit_audit，不返回/校验 `alliance:*` 权限码）；
  在前端硬挂 `v-permission="'alliance:expert:register'"` 等于编造权限模型，违反 contract.test 纪律。
- 6 个破坏性/管理路由已整体限 `['super_admin','tenant_admin']`，路由层收口已达成；
  按钮级细粒度遮蔽列入 12 号 roadmap G3/A1 演进项（P1），待后端权限码落地后再补。

### 3. 后端侧联动核证（写入 14 号文档第四节）

- `OptionalAuthUser` / `actor_from_opt_user` / `emit_audit`：`experts_common.rs:585/613/628`。
- 11 个生产写面 handler 注入真实身份：dispatcher 3（:581/653/752）、registry 4（:376/399/422/818）、
  session 4（:184/411/452/617）；未认证降级 `AuditActor::system()`（:619）。
- `check_with_audit` **存在**，但在 `platform/domains/ai/svc/mox-ai-expert-svc/src/rbac/check.rs:130`；
  旧路径 `platform/domains/mox-expert/` 已删（Test-Path=False，Cargo.toml 零引用）。
  网关 Cargo.toml 未依赖该 crate → 联盟 HTTP handler 不做后端 RBAC 强制，只审计。
- 6 角色继承链（admin→editor→viewer + safety_approver/operator/auditor）存活于
  `mox-ai-expert-svc/src/rbac/policy.rs:131-158`，作为 39 号历史快照的目标态参考列示。

### 4. 产物

- 新增 `docs/expert-alliance/14-enterprise-permission-model.md`（UTF-8，编号连续 13→14）。

---

## 2026-09-30：按钮级权限接入（G-1 闭环，v-role-any）

- 范围：把专家联盟模块的权限从「路由 + 后端」补到「元素级」（14 号文档缺口 G-1，原留待 P1）。
- 接入决策：**统一用 `v-role-any="['super_admin', 'tenant_admin']"`**，不用 `v-permission` 权限码。
  理由（三端同源，杜绝错位）：
  - 与路由 `requiresRole: ['super_admin','tenant_admin']`（`modules/expert-alliance/index.js:50/71/91/115/139/152`
    同数组字面量）、与后端 `ADMIN_ROLES = ["super_admin","tenant_admin"]`（`gateway/alliance/experts_rbac.rs:50`）
    **三端完全同源同语义**，不会出现「前端隐藏但后端放行 / 反之」的错位。
  - 零新增机制：指令 `v-role-any` 已注册（`directives/permission.js:124-128,141`，`main.js:47` 已
    `setupPermissionDirectives(app)`）；`hasAnyRole`（`stores/permission.store.js:182-185`）精确按
    `roles.includes(r)` 匹配真实角色码，**无 isAdmin 旁路**，不引入第二套权限体系。
  - `v-permission` 权限码路径不可行：后端 `/api/system/permissions` 返回的 permissions 列表不含联盟权限码
    （联盟码在网关联盟模块，与全站权限体系不联通）；且 `hasPermission` 对 isAdmin 恒 true（:141），对管理员无区分度。

### 1. 待加按钮清单（盘点：文件:行号 / 按钮文案 / 绑定方法 / 后端对应动作）

| # | 文件:行号 | 按钮文案 | 绑定方法 | 后端强制写面 |
|---|---|---|---|---|
| 1 | `views/AllianceExpertsView.vue:11` | 注册专家 | `openRegister→submitRegistry→registerExpert` | `experts_registry.rs` create（POST） |
| 2 | `views/AllianceExpertsView.vue:218` | 编辑 | `openEdit→submitRegistry→saveExpert→updateExpert` | `experts_registry.rs` update（PUT） |
| 3 | `views/AllianceExpertsView.vue:219` | 停用 | `openDisable→submitDisable→removeExpert→deleteExpert` | `experts_registry.rs` delete（软删 DELETE） |
| 4 | `views/AllianceGraphView.vue:14` | 重建图谱 | `rebuild→store.rebuild→rebuildGraph` | `experts_graph.rs` rebuild（图版本号+1 落盘） |
| 5 | `views/AllianceConsoleView.vue:41` | 保存改动（调度配置） | `saveConfig→store.saveDispatcherConfig→updateDispatcherConfig` | `experts_dispatcher.rs` update_config（PUT） |
| 6 | `views/AllianceConsoleView.vue:228` | 重置负载（单专家） | `openReset({id})→confirmReset→resetExpertLoad→resetDispatcherLoad` | `experts_dispatcher.rs` reset |
| 7 | `views/AllianceConsoleView.vue:245` | 全量重置 | `openReset({all:true})→confirmReset→resetAllLoads→resetAllDispatcherLoads` | `experts_dispatcher.rs` reset_all |

**自服务写面按钮（不加，如实标注「无需权限」）**：
- `AllianceExpertsView.vue`：收藏(:213)、预约(:216)、即时咨询(:217)、提交预约(:246)、接入会话(:264)、
  排行榜(:10)、开始匹配(:102)；`ExpertBookingPanel` 取消预约（自服务）。
- `AllianceConsoleView.vue`：新建任务(:12)、实跑一次(:95)、暂停/恢复/取消/标记完成(:352-363)——
  任务生命周期与分发实跑，非 7 个后端管理强制写面；所在路由 `/alliance/console` 已 requiresRole 收口。
- `AllianceOrchestrationView.vue`：一键编排执行/只生成计划/执行该计划(:47-49)——编排写面，所在路由
  `/alliance/orchestration` 已 requiresRole 收口（index.js:139），且不在 7 个后端强制 handler 清单内，
  本次不补元素级（路由层已限管理员）。
- 对话框 footer 的「确认重置/保存/确认停用」按钮：其打开入口（#2/#3/#6/#7）已挂指令，无权限者无法开对话框，
  footer 不会被触达，不再重复挂。

### 2. 接入情况

- 7 个按钮全部为 `el-button`（Element Plus 单根组件，指令落到其根 `<button>` DOM），直接挂
  `v-role-any="['super_admin', 'tenant_admin']"`，无自定义组件 attrs 透传问题。
- 改动文件仅 3 个 .vue（grep 校验 7 处 `v-role-any` 落点如上表），未改路由 / 后端 / store / 业务逻辑。
- 可回退：移除这 7 行指令属性即还原。

### 3. 降级语义

- 无权限时指令 `mounted/updated` 调 `_toggleElement(el,false)` 把元素从 DOM 移除（隐藏式降级，保存位置以便恢复）。
- 与后端语义一致：未认证 401 / 非管理角色 403（`experts_rbac.rs`）。**前端隐藏 ≠ 后端放行**——后端 7 个
  管理写面 handler 入口仍强制 super_admin/tenant_admin，前端按钮只是「不展示入口」，直接调 API 仍被后端拒。

### 4. 测试结果

- `npx vitest run src/modules/expert-alliance`：**25 个测试文件 / 650 用例，649 通过 / 1 失败**。
- 唯一失败 `contract/contract.test.js`「写面身份的前提都在源码里…」：断言 `experts_dispatcher.rs:581`
  含 `AuditAction::ExpertDispatch`，但该行现已漂移为 `match_scores: match_scores.clone()`（该 emit_audit
  调用实际在 :588）。**这是后端 Rust 源码行号漂移导致的预存在失败**（git diff 显示 experts_dispatcher.rs
  已有 53 行未提交改动），与本次 3 个 .vue 加指令无因果关系；按硬约束「不改后端」未修该锚点。
- 本次改动直接相关的测试全绿：alliance-experts.store.test.js(40)、alliance-graph.store.test.js(18)、
  module.test.js(6)、components/* 全部通过。
- 未补 store 级 hasAnyRole 新断言：本次零新增逻辑（复用已注册指令 + 既有 hasAnyRole，该指令已被全站
  admin/panels 使用并验证），补测试无新增覆盖价值。


---

## U1 画布 MVP（2026-10-01）：能力图谱画布编辑

### 1. 选型理由：增强既有 GraphCanvas.vue，不引新引擎

- **被增强的组件**：`components/GraphCanvas.vue`（手写确定性 SVG，纯「坐标→SVG」呈现；布局在 `model/layout.js`）。
- **为什么不引 VueFlow / LogicFlow / echarts force**：G9 三栈归一约束（12 §3.1 落地路径①）明确要求「避免画布成为第四套渲染栈」；
  任务书亦要求优先复用现有组件、避免重依赖。故在原 SVG 组件上加交互，而非换引擎。
- **证据**：FVR §5 G9（模块 GraphCanvas 手写 SVG vs legacy 力导向/echarts 三套并存）；本改动只动模块内这一套，legacy 两套未碰。

### 2. MVP 范围（已做，对应任务书 a–e）

- **a) 拖拽移动节点**：GraphCanvas 加原生 pointer events（pointerdown/move/up + setPointerCapture 思路，window 监听），
  client→SVG 坐标用 `getScreenCTM().inverse()`（jsdom 无此 API 时按 bounding rect 比例兜底），坐标夹在 viewBox 内。
  **坐标只活在 store 的 `dragPositions` 视觉覆盖层，不写后端、不入 `graph.value`**（视觉坐标不属于图谱数据模型），
  `loadGraph` 后即弃。图例加 capability 第三色（`--success`）。
- **b) 节点编辑**：Inspector 加「编辑节点/删除节点」（`v-role-any="['super_admin','tenant_admin']"`）→
  `updateGraphNode`/`deleteGraphNode`（N4 端点，删边级联由后端做）；视图加「新增节点」对话框（管理写面）→ `createGraphNode`。
- **c) 边连线**：编辑模式下点源节点（高亮 `is-link-source`）→ 点目标节点 → 弹 edge_type 选择（collaborates_with / has_domain）
  → `createGraphEdge`。意图分发收敛在 store `canvasClickNode(id)`。
- **d) 邻域展开**：Inspector「展开邻域」（读面，登录即可用）→ `expandSelectedNeighborhood()` 以选中节点为 seed、
  maxDepth=2 调 T2 `expandGraphNeighborhood`，返回 results **幂等并入**画布（已存在节点/边按 id 与 source|target|edgeType 去重跳过），
  原节点不动；**不调 loadGraph/rebuild**，避免冲掉手动并入的邻域。
- **e) 视图接线**：AllianceGraphView 挂编辑模式开关、新增节点对话框、边类型对话框，不破坏既有 loadGraph/loadMetrics/loadCommunities 链路。

### 3. capability 的诚实处置（关键取舍）

- 后端 `experts_graph.rs` `VALID_NODE_TYPES=["expert","domain","capability"]`，**但** builder `build_graph_from_registry` 只产出 expert/domain，
  且 `contract/graph.test.js` 把 `Object.values(GRAPH_NODE_TYPE)` 与 Rust builder 里 `node_type:"x"` 集合**双向钉死**。
- 故 **不把 capability 并入 `GRAPH_NODE_TYPE`**（并入必挂双向守卫）：新增 `GRAPH_WRITE_NODE_TYPES`（写入表单选项）+ 画布第三色兜底，
  capability 只在手动 CRUD / RAG 结果出现时渲染为绿色节点。

### 4. 未做项（诚实标注，不冒充全量）

- DAG 编排导出为 planner 可执行 JSON + 预演校验；T5 逐节点实时回显；G9 三套栈整体归一；minimap；虚拟滚动/分层渲染（G7 仍 P2）；
  拖拽坐标持久化（刷新即复位，有意为之）。

### 5. 改动文件

- `contract/graph.js`：+`GRAPH_WRITE_NODE_TYPES`（不动 GRAPH_NODE_TYPE）。
- `store/alliance-graph.store.js`：+`dragPositions/editMode/linkSourceId/pendingEdge/nodeDraft` 状态；`layout` computed 叠加拖拽覆盖；
  +`setEditMode/setNodePosition/canvasClickNode/cancelLink/resetNodeDraft/mergeRagResults/expandSelectedNeighborhood`；`loadGraph` 清拖拽覆盖。
- `components/GraphCanvas.vue`：pointer 拖拽、editMode/linkSourceId props、`drag` emit、capability 第三色与图例、link-source 高亮。
- `components/GraphNodeInspector.vue`：+展开邻域（读面）、admin 编辑/删除 + 内联编辑表单。
- `views/AllianceGraphView.vue`：编辑模式开关、新增节点对话框、边类型对话框、画布接线。
- 测试：`store/alliance-graph.store.test.js`（+6：拖拽覆盖/重取清位/连线态机/RAG 幂等/展开并入/CRUD 重取）；
  新增 `components/graph-canvas.test.js`（3：三色渲染、select 上抛、link-source 高亮）。

### 6. 测试结果

- `npx vitest run`（全量）：**78 个测试文件 / 1044 用例全绿**（exit 0）。
- 本次直接相关：alliance-graph.store.test.js(24)、graph-canvas.test.js(3)、graph.test.js(32 双向守卫仍绿，证明未动 GRAPH_NODE_TYPE)、
  style.test.js(21，新类均在 `.agc-*`/`.agn-*`/`.agv-*` 块内、未跨 .vue 复用、全用主题令牌)、contract.test.js(101 端点对齐未破)。
- 前端显隐与后端 RBAC 一致：管理写面按钮 `v-role-any`，后端 401/403 兜底；邻域展开/查看详情为读面，登录即可。


---

## U2 匹配透明化（2026-10-01）

### 背景
生产主路径 ModularWeightMatcher 内部本就为每个候选专家算好逐维明细（MatchScoreBreakdown），但 HTTP 边界把它压成瘦 ExpertSummary，前端只拿到一个总分。U2 把内部已算好的逐维得分 + 实际权重透出，新增「为什么匹配」面板。

### 改动
- `model/normalize.js`：`normScoreDim`/`normScores` 辅助；`normExpertSearch` 扩展 `scores`（domain/capability/priority/performance/health 各 {value,weight,contrib} + total）与 `matchReason`；任一维缺失则 scores=null（旧上游不渲染面板，不破坏既有卡片）。
- `components/MatchExplainPanel.vue`（新建）：纯 CSS 条形图逐维渲染 + 权重标注 + 总分演算；说明文案如实标注「健康度按 0.05 加权（非过滤）」；全走主题令牌（var(--text-*)/var(--bg-*)/var(--accent)/var(--success)/var(--warning)/var(--radius-xs)/var(--border-soft)），无裸 hex/px，过 style.test.js 门禁。
- `components/index.js`：注册导出 MatchExplainPanel。
- `views/AllianceExpertsView.vue`：匹配结果卡片挂载 `<MatchExplainPanel :scores="e.scores" :match-score="e.matchScore" :match-reason="e.matchReason" />`。

### 不做 / 诚实标注
- 组队侧（optimal-team / GraphTeamPanel）是贪心集合覆盖、公式独立，本轮未接解释面板；后端 scores 仅随 /experts/search 匹配结果透出。
- 未改任何算法/权重值，只透出后端已有计算结果。

### 测试结果
- `npx vitest run`（全量）：78 文件 / 1044 用例全绿（含 style.test.js 21 项门禁）。

---

## T4 事件帧 SSE 前端接入（2026-10-02）

### 做了什么（最低接入，按成本如实评估）
- **三处登记**：后端新端点 `GET /api/alliance/events/stream`（T4 业务事件帧 SSE）已在
  `contract/endpoints.js` 登记为 `allianceEventStream`，并同步 `docs/API-REGISTRY.md` 与后端 actuator ROUTES。
- **真实消费接入点**：新增 `composables/useAllianceEventStream.js`——以 `fetch` + `ReadableStream`
  直连该端点，按 SSE 协议切分帧，回调 `(kind, envelope)`。它是**可工作的真实接入点**，
  目前未挂进任何视图（避免扰动既有 1044 用例基线）；要在编排/控制台实时刷新，在视图 `onMounted`
  调 `start()`、`onUnmounted` 调 `stop()`，收到事件后重拉对应列表即可。
- **webhook CRUD 不进本前端模块**：它是运维管理面，按 contract 门禁归 `DOC_UNREGISTERED_PENDING`
  「欠登记」（后端已真实落地并 E2E 验证，见后端报告）；不占本模块接线覆盖率。

### 改动文件
- `contract/endpoints.js`：新增 `allianceEventStream`。
- `contract/contract.test.js`：webhook 两路径入 `DOC_UNREGISTERED_PENDING`；modules.rs 行号锚点随后端插入同步（219→234）。
- `contract/registry.js`：`EXPERT_WRITE_IDENTITY.evidence` 行号同步。
- `contract/vocabulary-ownership.test.js`：新目录 `composables/` 入 `DIR_ACCOUNT`（理由：SSE 消费入口），出账表 5→6。
- 新增 `composables/useAllianceEventStream.js`。

### 测试结果
- 本任务相关门禁（contract 登记/接线、vocabulary 目录账）均通过。
- 如实标注：`contract.test.js` 与 `orchestration.test.js` 中仍有若干失败，系 **A1/D4 既有源漂移**
  （favorites 分区结构、emit_audit 增租户参、save_registry 签名、orchestration 出参键集与行号），
  非本轮引入；本轮未触碰那些源文件。

---

## SSE 事件流前端挂载（2026-10-02）

### 挂载点选择理由
- 选 **`views/AllianceOrchestrationView.vue`（专家编排台）** 作为第一个真实挂载视图。
- 理由：该视图 `onMounted` 本就拉「编排统计」（按状态分桶：draft/running/completed/…，
  `store.statCells`）与「执行历史」表——正是 `PlanCreated / PlanStatusChanged` 两类事件
  的天然消费面。事件帧到达 → 这两个读数该变，比「控制台任务列表」更直接命中
  「计划/任务状态实时刷新、免轮询」的目标。专家注册表视图（ExpertRegistered/ExpertDisabled）
  成本相近但本轮先收一处闭环，留待下轮。

### 数据流（帧 → store → 视图，字段映射）
```
后端 GET /api/alliance/events/stream（experts_streams.rs:82，event:<Kind> + data:<信封JSON>）
  → composables/useAllianceEventStream.js（Bearer 鉴权 + 断线重连语义，onScopeDispose 自动断）
  → onEvent(kind, envelope)
  → store/alliance-orch.store.js  applyAllianceEvent(kind, envelope)
       ① liveEvents.unshift(规范化行)          ← 视图立即可见「事件到了」
       ② envelope.plan_id 非空 → 800ms 防抖 loadStats() + loadHistory(当前页)  ← 真值真拉，不本地猜计数
  → 视图 v-for="ev in store.liveEvents" 实时事件面板
```
- 信封是 `experts_events.rs` 的扁平 serde（`#[serde(tag="type")] + flatten`）：
  `{ id, type, ...payload, source, tenant, occurred_at }`。store 行映射
  `plan_id→planId / from / to / execution_id→executionId / task_type→taskType / title /
  expert_id→expertId / occurred_at→occurredAt`（snake_case→camelCase，模块归一惯例）。
- **不本地猜计数**：统计分桶的真值永远由 `loadStats` 真拉回；帧只作「该重拉了」的防抖提示，
  避免前端乐观数与后端真实进程内表漂移。带 `plan_id` 的帧才触发重拉；专家帧（无 plan_id）
  只进事件流，不牵动编排读数。

### 改动文件
- `store/alliance-orch.store.js`：+`liveEvents`(ref) / `applyAllianceEvent(kind,env)` / `clearLiveEvents()`；
  Plan* 事件防抖 800ms 合并真拉统计+历史；liveEvents 上限 30 条。
- `views/AllianceOrchestrationView.vue`：setup 顶层 `useAllianceEventStream({onEvent→store.applyAllianceEvent, onError:静默})`；
  `onMounted` `start()`、`onUnmounted` `stop()`（与 composable 内部 onScopeDispose 双保险，幂等）；
  模板新增「实时事件流」卡片（`store.liveEvents`）；专家名走既有 `expertNames` 映射（不新造姓名出口）。
- 传输层（进场时同仓已有，本轮复用未重写）：`contract/event-stream.js`（`createAllianceEventStream` +
  `createEventFrameParser`，Bearer/CRLF/多行 data/大小上限/StreamGap/连接替换）、
  `composables/useAllianceEventStream.js`（auth token 注入 + watch 断流 + onScopeDispose）、
  `contract/event-stream.node-test.mjs`（node:test 真 TCP 集成）。
- 新增测试：
  - `store/alliance-orch.event.test.js`（5）：真后端帧形状 → liveEvents 因帧而变、字段映射、
    防抖合并、无 plan_id 帧不重拉、30 条上限、clear 挂起 timer。
  - `contract/event-stream.lifecycle.test.js`（3）：start 真发 fetch（Bearer + event-stream）、
    真帧解析进 onEvent、stop 真 cancel 读端、无 token 不连。

### 测试结果
- `npx vitest run src/modules/expert-alliance`：**28 文件 / 667 用例**。
- 通过 **650**；失败 **17**，与改前基线（26 文件 / 659 用例，17 失败）逐一同集：
  全在 `contract/` 下读 Rust 源码文本的锚点测试——`orchestration.test.js`(12)、
  `contract.test.js`(4)、`dispatcher.test.js`(1)，即报告上文记录的 **A1/D4 既有源漂移**
  （favorites 分区、emit_audit 增参、save_registry 签名、orchestration 出参键集与行号）。
- 本轮新增 8 用例全绿；本轮引入失败 **0**；棘轮（registry-name-outlets）、style.test(21)、
  vocabulary-ownership(19) 等门禁随全量复跑仍绿。
- 过程中曾因视图模板 `ev.name || ev.expertId` 与 store `envelope.name ||` 触发
  「裸 .name 兜底」棘轮新增失败 1，已改为走 `expertNames[ev.expertId]`、store 不存 `name` 字段后消除。

### 未做 / 诚实标注
- 控制台（AllianceConsoleView）、专家注册表（AllianceExpertsView）、图谱视图尚未挂流；
  ExpertRegistered/ExpertDisabled 帧本轮只进编排台的事件流面板，不重拉专家表/控制台读数。
- 未补组件级挂载测试：本视图是依赖 Element Plus + 双 store + auth 的终端控制台页，
  模块内 6 个 view 现无任何 .vue 挂载测试；「视图因 store 变化而重渲染」由
  模板 `v-for="ev in store.liveEvents"` 对响应式 ref 的绑定 + store 级「帧→liveEvents」断言保证，
  不强行起重型 mount 引入脆弱面。
- 既有 17 个 Rust 锚点漂移失败本轮未修（不修未触及的后端源文件），如实保留。



---

## 锚点漂移清理（2026-10-02）

### 起因
上一轮 SSE 挂载后基线为「28 文件 / 667 用例，650 过 / 17 失败」，失败全在 `contract/` 下钉
Rust 源码文本的锚点测试。本轮目标：把这批因后端真实改动（A1 多租户、D4 落盘、T4 事件，
以及「模拟执行」整体重写为「真实模型咨询 DAG」）产生的行号/签名/出参键集漂移，逐一对齐到
**当前真实源码**，只改前端测试断言，不改后端/业务代码、不删测试、不弱化断言语义。

### 进场实况复核（与上轮报告的出入）
- 本轮进场首跑实况为 **13 失败 / 654 通过**（28 文件 / 667 用例），分布：`orchestration.test.js` 12 +
  `dispatcher.test.js` 1。
- 上轮记录的 `contract.test.js` 4 个失败，已随并发未提交改动转绿（本轮复跑 contract.test.js 101 用例全过），
  无需再动。git 工作树干净（除本任务临时分析脚本），最新提交 `309db3c8`（核心能力文档去幻影化重写）。

### 断言清单（旧钉值 → 当前真实值 → 处理）

| # | 测试文件:断言 | 后端文件:行 | 旧钉值 | 当前真实值 | 处理 |
|---|---|---|---|---|---|
| 1 | dispatcher.test.js:542 | experts_registry.rs:893 | `save_registry(&reg)` | `save_registry(tenant.as_str(), reg)`（A1 增租户参） | 正则对齐；捕获窗口 90→140 避免截断在 `save_registry(tenant.` |
| 2 | orchestration.test.js:152 | experts_orchestration.rs:610/695 | experts 摘要 `[id,name,title]` 命中 3 处 | 只在 orchestrate、plan/generate 两处拼，命中 2 处 | times 3→2 |
| 3 | orchestration.test.js:160 | :633 | execution 内联包装 `"status":"completed"` 字面量 | `"status": plan.status` | 正则 `"completed"`→`plan\.status` |
| 4 | orchestration.test.js:169-175 | :468-471 | execute 分「成功 9 键 / 成环失败 8 键」两块，差集 error/completed_at+final_result | 去幻影化后只吐**单一 10 键块**：`[plan_id,execution_id,status,overall_status,steps_executed,steps_total,error,duration_ms,final_result,evidence_kind]`，error/final_result 同为可空 Option | 两块合一，改单块字面量断言；删差集断言 |
| 5 | orchestration.test.js:190 | :452-453 | `simulate_step_execution` 单步 7 键块 | 单步融合输入 5 键 `[summary,key_findings,confidence,source,evidence_kind]` | 字面量对齐（函数已删） |
| 6 | orchestration.test.js:232-240 | :409/:457-464/:821 | plan.status `.to_string()` 直赋 completed/running；ready/failed 均无写入路径 | 赋值改 `.into()`，直赋仅 running；终态 failed/completed/partial 走 `if error {...} else if {...} else {...}` 三选一；stats 现真计 plans_failed | 重写断言；`ORCH_ZERO_COUNTERS` 由 `['plans_ready','plans_failed']` 收紧为 `['plans_ready']` |
| 7 | orchestration.test.js:242-246 | :398/:457 | 成环 `Err(e)=>{return json!{...}}` 独立分支，不写 plan.status | 环错先写 `error`，终态条件把 plan.status 置 failed | 重写为折叠语义断言 |
| 8 | orchestration.test.js:248-253 | :564/:593 | `execute_plan(&mut plan, None)`；历史行 `status:"completed".to_string()` | `execute_plan(&mut plan, None, &matched_experts).await`；历史行 `status: plan.status.clone()` | 文本对齐 |
| 9 | orchestration.test.js:296 | :397 | `let execute_set: Option<HashSet<String>> = step_ids.map` | `let selected = step_ids.map(\|ids\| ids.into_iter().collect::<HashSet<_>>())` | 文本对齐（变量改名） |
| 10 | orchestration.test.js:347-351 | :100-141 | `fn simulate_step_execution` 的 `"type" =>` match 臂（7 类 + `_` 兜底） | step_type 七类散落在 `select_steps_for_task_type` 的 `("type","name","desc")` 元组表首元素 | 改为从元组表提取唯一值排序比对 |
| 11 | orchestration.test.js:416-422 | experts_db.rs:1068/1122 | `DB_RS not.toMatch(/plan\|orchestr/i)`（两表无落库）；`orchVolatileNote` 含「重启即归零」 | D4 已落盘：`upsert_plan`/`insert_history_record`；内存态仍权威、SQLite 为 best-effort 投影 | 改断言落盘函数存在；note 改 SQLite 口径 |
| 12 | contract/orchestration.js `ORCH_SIMULATED` | 多处 | 9 条钉 simulate 常量（:271 confidence0.85/:488 expert空数组/:493/:497/:659/:621/:727/:230/:101） | 重建为 8 条真实硬编码字面量（pending:187 / draft init:204 / generate draft:718 / step表:102 / evidence_kind:471 / general:527 / weighted:528 / 0.2 门槛:538） | 整表重建 |
| 13 | views/AllianceOrchestrationView.vue:236,262 | :175-178 / :839-919 | 模板坐标引用 :174、:920 指向空行 | 端点收窄到非空行 | 坐标修正；同步把视图三条已失真的静态说明（result.expert 恒 null / confidence 恒0.85 / plans_failed 恒0）改为去幻影化后的真口径 |

另：`ORCH_EXECUTED_STEP_KEYS` 键序对齐源码实际顺序（`duration_ms` 在 `result` 前，:446-447）。

### 验证结果
- `npx vitest run src/modules/expert-alliance`：**Test Files 28 passed (28) / Tests 667 passed (667)，0 失败**。
- 新增失败 **0**；棘轮（registry-name-outlets 15）、style.test(21)、vocabulary-ownership(19)、contract.test(101)、
  forbidden-revival / legacy-collab-revival 等门禁随全量复跑仍绿，未触发新失败。
- 本轮只改前端测试断言与视图说明文案；未改后端 Rust 源码、未改 normalize.js 业务归一化层。

### 留待 / 诚实标注
- 并发未提交的后端工作（`experts_ext.rs`、`tests/tenant_expert_isolation.rs` 等 A1 租户隔离他人改动）
  与本批锚点无强耦合，更新断言均基于当前文件真实内容，未依赖未提交工作。
- `ORCH_EXECUTE_FAILED_KEYS` 常量在测试中断言里不再被引用（保留导出，仍描述 normalize 语义，
  不删以免跨 normalize.js 业务层改动）；非 lint/门禁失败。
- 未发现「断言对、代码错」的真实缺陷——漂移全部来自后端真实改动，断言已如实跟进，无糊弄性弱化。

---

## 全维终验（2026-10-03）

- 范围：expert-alliance 前端模块全维终验——逐项核证据、全量 vitest 实跑、优化推进与修复闭环。
- 基线（本轮进场首跑，实跑）：**28 文件 / 667 用例 100% 全绿，0 失败**，与「锚点漂移清理轮」一致。
- 本轮未触碰他人并发未提交文件 `contract/contract.test.js`（M）；本轮改动落在 store/experts、views/Experts、新增测试。

### 1. 全维验证清单（逐项证据）

| 功能面 | 验证方式 | 结果 |
|---|---|---|
| U1 画布·拖拽移动节点 | `components/GraphCanvas.vue:136-164`（pointerdown/move/up，emit `drag`）；`store/alliance-graph.store.js:347 setNodePosition`→`dragPositions`（:47）；`layout` computed 叠加拖拽覆盖 :62-73；`loadGraph` 清位 :105。证据测试：`components/graph-canvas.test.js`（点节点上抛 select）、`store/alliance-graph.store.test.js`（拖拽覆盖/重取清位） | ✅ |
| U1 画布·连线 | `GraphCanvas.vue:12` `is-link-source` 高亮、:358 store `canvasClickNode` 态机（无源头→记起点；点同点→取消；点异点→pendingEdge）；`createGraphEdge` :262。证据测试：`graph-canvas.test.js`「连线起点高亮」、store「连线态机」 | ✅ |
| U1 画布·邻域展开 | `GraphNodeInspector.vue:32`「展开邻域」按钮→`onExpand` :146→`store.expandSelectedNeighborhood` :429（seed=选中节点、maxDepth=2）；`mergeRagResults` :393 幂等并入（按 id 与 source\|target\|edgeType 去重），不调 loadGraph。证据测试：store「展开并入/RAG 幂等」 | ✅ |
| U1 画布·节点增删 | `GraphNodeInspector.vue:36-37` 编辑/删除（`:35 v-role-any`）→`updateGraphNode` :132/`deleteGraphNode` :142；视图新增节点对话框→`createGraphNode`；store CRUD :229-271。证据测试：store「CRUD 重取」 | ✅ |
| U2 透明面板·权重 0.05 口径 | `components/MatchExplainPanel.vue:18` 文案「健康度按 **0.05** 加权（非过滤…）」；五维 domain/capability/priority/performance/health :42-46。证据测试：本轮新增 `match-explain-panel.test.js`（0.05 文案、五维条） | ✅ |
| U2 透明面板·维度条 | `MatchExplainPanel.vue:8-14` 每维 mxe-row 画 value/weight/contrib；挂载点 `views/AllianceExpertsView.vue:129`（`:scores="m.scores"`）；`v-if="scores"` 缺省不渲染 :2 | ✅ |
| SSE·composable | `composables/useAllianceEventStream.js:7-21`（getToken→Bearer、watch auth 变化→stop、`onScopeDispose` 双保险）；传输契约 `contract/event-stream.js:33 createAllianceEventStream`（AbortController/reader.cancel/releaseLock） | ✅ |
| SSE·store | `store/alliance-orch.store.js:40 liveEvents`、`:225 applyAllianceEvent`、`:243-250` 带 plan_id 才防抖 800ms 真拉、30 条上限 :241。证据测试：`store/alliance-orch.event.test.js`（5） | ✅ |
| SSE·视图挂载（编排台） | `views/AllianceOrchestrationView.vue:284` import、:314 use、:322 `onMounted start()`、:325 `onUnmounted stop()`；模板 :18/:25 `v-for="ev in store.liveEvents"` | ✅ |
| SSE·防泄漏 | onUnmounted stop + composable `onScopeDispose`（useAllianceEventStream.js:19）+ event-stream.js `finally` 里 `reader.cancel()/releaseLock()` :69-76 + watch auth→stop :15-18。证据测试：`contract/event-stream.lifecycle.test.js`（3：真发 fetch Bearer/真帧解析进 onEvent/stop 真 cancel） | ✅ |
| 按钮权限·v-role-any | 活代码 10 处：`GraphNodeInspector.vue:35`、`AllianceConsoleView.vue:41/228/245`、`AllianceGraphView.vue:14/15/18`、`AllianceExpertsView.vue:11/220/221`；统一 `['super_admin','tenant_admin']`，与路由 requiresRole、后端 ADMIN_ROLES 三端同源 | ✅ |
| 契约·单向依赖 | contract 生产文件仅内部互引（dispatcher.js:7→graph.js、mode.js:13→enums.js、registry.js:13→graph.js），零反向 import api/store/views；`api/alliance.api.js:3-25` 仅 import contract + http 工厂 + kernel envelope；store 依赖 api+contract+model | ✅ |
| 契约·双向守卫 | `contract/contract.test.js`（101 用例：端点清单↔docs/API-REGISTRY 双向、禁用端点不复活）；`contract/graph.test.js`（32：GRAPH_NODE_TYPE↔Rust builder 双向钉死）；`contract/dispatcher.test.js`/`orchestration.test.js`/`sessions.test.js`/`mode.test.js`/`registry.test.js` | ✅ |
| 契约·样式/词汇/棘轮门禁 | `style.test.js`（21：.agc-/.agn-/.agv- 块内、主题令牌、不跨 .vue 复用）；`registry-name-outlets.test.js`（15：裸 `.name` 兜底/模板直出双向台账）；`vocabulary-ownership.test.js`（19：词表所有权、视图不重打契约取值） | ✅ |
| 视图·编排台/图谱/专家/控制台 | 编排台 `AllianceOrchestrationView.vue`（SSE 实时面板）；图谱 `AllianceGraphView.vue`（重建 :14、编辑模式开关 :15、新增节点 :18）；专家 `AllianceExpertsView.vue`（注册 :11/编辑 :220/停用 :221 + 匹配面板 :129）；控制台 `AllianceConsoleView.vue`（保存调度 :41、单专家重置 :228、全量重置 :245） | ✅ |
| store/api 方法面 | 6 store（orch/graph/experts/console/sessions/collab）各有 *.store.test.js；`api/alliance.api.js` + `alliance.api.test.js`（58）；`contract/endpoints.js` 端点 key 由 contract.test 101 项对齐 docs/API-REGISTRY | ✅ |

> 说明：6 个终端 view（编排台/图谱/专家/控制台/协作/会话）仍无挂载测试——它们重度依赖 Element Plus + 多 store + auth，硬挂脆弱；本轮以纯呈现组件 `MatchExplainPanel` 的真实 mount 测试补空（见下），完整 view 挂载如实列入留待。

### 2. vitest 全量真实数字（实跑）

- 命令：`npx vitest run src/modules/expert-alliance`
- **终验结果：Test Files 29 passed (29) / Tests 675 passed (675)，0 失败，exit 0。**
- 对比基线 28 文件 / 667 用例：新增 1 个测试文件（match-explain-panel.test.js）+ 8 用例（experts store 5 + mount 3）= 675。
- 失败分析：**0 失败**，无需逐条归因；进场 17 个 Rust 锚点漂移已于上轮「锚点漂移清理」对齐真实源码，本轮复跑仍全绿。
- 门禁复跑：style(21)、registry-name-outlets(15)、vocabulary-ownership(19)、contract(101) 随全量复跑仍绿，**未新增任何失败**。

### 3. 优化推进项（本轮真实落地）

**a) 专家注册表补挂 SSE（消费 ExpertRegistered/ExpertDisabled）——选它的理由**
- 编排台已消费 `PlanCreated/PlanStatusChanged`；而 `ExpertRegistered/ExpertDisabled` 帧此前只进编排台事件流面板，**不重拉专家表**——它们的天然消费面是注册表本身。
- 路由 `/alliance/experts` 已 `requiresRole: ['super_admin','tenant_admin']`（index.js:152），挂流只对管理员，**不会给全体用户开 SSE 连接**。
- 价值：另一管理员注册/停用/改档专家时，本页自动刷新，免手动「刷新」。
- 改动：
  - `store/alliance-experts.store.js`：新增 `applyRegistryEvent(kind, envelope)` / `clearRegistryEventTimer()`。**按信封形状判定**（带 `expert_id`＝注册表身份变更：注册/停用/改档），不硬编码事件名字面量（避免在 store 重打契约值域、不触词汇门禁）；800ms 防抖合并突发帧后真拉 `loadExperts()`+`loadStats()`，仅当 `capabilities.value` 已加载才补拉目录（不替用户起他没要的请求）。与编排台同口径：帧只作「该重拉了」的提示，真值永远真拉回，不本地猜。
  - `views/AllianceExpertsView.vue`：vue import 补 `onUnmounted`；import `useAllianceEventStream`；setup 顶层建 `eventStream`（onEvent→`store.applyRegistryEvent`，onError 静默）；`onMounted` 末尾 `start()`、新增 `onUnmounted(() => eventStream.stop())`。
  - 测试：`store/alliance-experts.store.test.js` 新增 describe「T4 SSE 注册表事件帧 → 防抖真拉列表」5 用例（命中真拉 / 突发两帧合并 / 不带 expert_id 的 Plan 帧忽略 / 目录已加载才补拉 / clear 挂起未触发重拉）。

**b) 组件级 mount 测试补 1 个（U2 组件此前零测试）**
- 新建 `components/match-explain-panel.test.js`（@vue/test-utils 真 mount，3 用例）：
  1. `scores=null` → 整个 `.mxe-root` 不渲染（降级路径不破坏卡片）；
  2. 带 scores → 5 个 `.mxe-row`、总分演算行、健康度 0.05 口径文案、五维标签齐全；
  3. **驱动数据变化时 DOM 跟随重渲染**：`setProps({matchScore:0.923})` → 总分行从 0.500 变 0.923；改 health 维 value 1.0→0.2 → 该行值与贡献同步刷新。
- 说明：MatchExplainPanel 是纯呈现组件（无 Element Plus、无 store），由 `AllianceExpertsView` 从 `store.expertMatches` 喂 scores；选它是因它最可测、最稳健，不拖入 Element Plus/多 store 的脆弱挂载。完整 view 的 store→DOM 挂载仍如实留待。

**c) 其他低成本高价值：本轮刻意只做 a/b 两项**
- 未给注册表页新增可见事件流面板：那会引入新模板 + CSS + 词表账，与「实时刷新」的核心价值无关，属过度设计；保持最小可回退。

**d) 体量大项——产方案稿，本轮不硬做（诚实标注）**
- **画布 DAG 导出为 planner 可执行 JSON + 预演校验**：可在 `model/dag.js` 上加纯函数 `exportDagJson(execution)` + `validateDag(dag)`，再在编排台加「导出/预演」按钮；建议先做纯函数 + 单测（无 UI 依赖），再接 UI。工作量约 1 个独立模块。
- **minimap / 虚拟滚动 / 分层渲染**：当前 `GraphCanvas` 手写 SVG、节点全量直出；minimap 需第二套 viewBox 缩略视口（加一个 <svg> 缩略层即可，成本中）；虚拟滚动在专家列表（`AllianceExpertsView` 分页已是 24/页，压力不大），优先级低。
- **G9 三栈归一**：模块内 GraphCanvas（手写 SVG）vs legacy 力导向/echarts 三套并存，归一属跨视图大改，需单独立项，不在终验轮范围。

### 4. 修复清单

- **本轮无真缺陷需修复**：进场基线 28/667 全绿，全量跑无失败；a/b 改动后 29/675 仍全绿，未引入新缺陷。
- 验证手段：除全量 vitest 外，另用临时 SFC 编译探针（`@vitejs/plugin-vue` 真编译两个改动的视图，跑完即删）确认 `AllianceExpertsView.vue` / `AllianceOrchestrationView.vue` 编译无错——因 6 个 view 无挂载测试，视图 script 改动此前不被测试覆盖，探针补齐这一缺口。

### 5. 留待项及原因

- **控制台 / 图谱视图未挂流**：Expert* 帧本轮已被注册表消费；控制台任务列表挂流（任务状态实时刷新）价值中等、成本与注册表页相当，留待下轮按需；图谱视图重建/邻域是手动触发，挂流收益低。
- **完整 view 的 store→DOM mount 测试仍缺**：本轮补的是纯呈现组件 MatchExplainPanel；6 个终端 view 依赖 Element Plus + 多 store + auth，挂载脆弱，留待引入全站测试基座后再补。
- **DAG 导出/预演、minimap、虚拟滚动、G9 三栈归一**：见 §3d 方案稿，本轮不硬做。
- 本轮改动全部可回退：store 删 `applyRegistryEvent/clearRegistryEventTimer` 两函数与导出、视图删 eventStream 三行接线、删两个新增测试文件即还原。

# 前端 API 函数 ↔ 后端端点 对接审计报告

> **审计范围**：`frontend-ui/src/api/` 全量 16 个模块 + `platform/` Rust workspace + `platform/legacy/` 遗留后端
> **审计日期**：2026-09-01
> **审计性质**：只读静态代码审计，不修改任何代码
> **前端 baseURL**：`/api`（`http.js` 第 13 行）
> **Vite 代理**：`/api` → `http://localhost:8080`（无路径重写，`vite.config.js` 第 108-111 行）
> **网关监听**：`mox-platform-gateway-svc` axum 绑定 `0.0.0.0:8080`

---

## 0. 架构关键发现（先读这个）

### 0.1 三层后端并存，路径前缀不一致

| 层 | 组件 | 路径前缀 | 状态 |
|---|---|---|---|
| L1 主网关 | `mox-platform-gateway-svc` | `/api/system/*`、`/api/security/*`、`/health`、`/kg/v1/*`、`/ai/engine/*`、`/alliance/v1/*` | ✅ 运行中（:8080），IAM 读接口真实，写接口 stub |
| L2 编排器 | `mox-platform-orchestrator-svc` | `/api/*`（ai/chat、graph、caomei、operators、mox、mcp 等约 60 端点） | ⚠️ 代码存在，但是否并入 :8080 主进程未确认 |
| L3 遗留 Rust | `platform/legacy/backend-rust` | **无 `/api` 前缀**（`/ai/chat`、`/graph`、`/experts` 等全量 ~200 端点） | ⚠️ 遗留代码，路径与前端不匹配，需代理 strip `/api` |
| L4 遗留 Python | `platform/legacy/mox-server` | `/api/dsql/*`、`/api/admin/*`、`/api/kg/*`、`/api/apps/*` 等 | ⚠️ FastAPI 遗留，与前端业务域几乎无交集 |

### 0.2 核心问题

1. **前端调用 `/api/ai/chat`，主网关无此路由**：网关 `build_gateway_router()`（`lib.rs:103`）仅 merge 了 `system`、`security`、`kg_ai`、`alliance` 四个 router，**没有 merge orchestrator 的 `/api/*` 路由**。
2. **遗留后端路径缺 `/api` 前缀**：`legacy/backend-rust/src/api/mod.rs` 注册的是 `/ai/chat` 而非 `/api/ai/chat`，Vite 代理无 rewrite，直接 404。
3. **orchestrator 代码存在但接入状态不明**：`mox-platform-orchestrator-svc/src/main.rs` 有完整 `/api/*` 路由（约 60 端点），但网关 lib.rs 未引用它。
4. **大量前端函数在任何 Rust 代码中都找不到对应端点**：如 `/api/ai/full-analysis`、`/api/ai/infinite-optimize/*`、`/api/experts/*`（专家联盟全量 50 端点）、`/api/kb/*`、`/api/llm/*`（新网关）、`/api/projects/*`、`/api/tasks/*`、`/api/web-search/*`、`/api/automation/*`、`/api/melody2score/*` 等。

---

## 1. 前端 API 函数全量清单（按模块分组）

> 共 **16 个 API 模块**，约 **340+ 个导出函数**（含 deprecated 别名）。
> 标记说明：`[参]`=带参数，`[SSE]`=Server-Sent Events 流式，`[deprecated]`=已废弃别名

### 1.1 `http.js` — HTTP 核心实例（非业务端点）

| 导出 | 类型 | 说明 |
|---|---|---|
| `http` (default) | axios instance | baseURL=`/api`，timeout=30s |
| `registerProjectIdGetter` | 函数 | 注册 project_id 注入器 |
| `withCancel` | 函数 | 带取消令牌的请求包装 |
| `batchRequest` | 函数 | 批量并行请求（并发控制） |

### 1.2 `system.api.js` — 系统管理域（95 个函数）

#### 系统基础（6）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getHealth` | GET | `/health` | 无 |
| `getStatus` | GET | `/status` | 无 |
| `getFullStatus` | GET | `/status/full` | 无 |
| `getLogs` | GET | `/logs` | 无 |
| `getPlugins` | GET | `/plugins` | 无 |
| `getSystemConfig` | GET | `/config` | 无 |

#### 安全凭证（6）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getSecurityStatus` | GET | `/security/status` | 无 |
| `getApiKeys` | GET | `/security/api-keys` | 无 |
| `createApiKey` | POST | `/security/api-keys` | [参] |
| `revokeApiKey` | DELETE | `/security/api-keys/:id` | [参] |
| `validateApiKey` | POST | `/security/validate` | [参] |
| `getAuditLogs` | GET | `/security/audit-log` | [参] |

#### 存储与模块（4）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getStorageProviders` | GET | `/storage/providers` | 无 |
| `switchStorageProvider` | POST | `/storage/switch` | [参] |
| `getStorageStatus` | GET | `/storage/status` | 无 |
| `getModules` | GET | `/modules` | 无 |

#### 权限（1）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getPermissions` | GET | `/system/permissions` | 无 |

#### 部门管理（8）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getDeptList` | GET | `/system/dept` | [参] |
| `getDeptTree` | GET | `/system/dept/tree` | [参] |
| `getDeptDetail` | GET | `/system/dept/:id` | [参] |
| `createDept` | POST | `/system/dept` | [参] |
| `updateDept` | PUT | `/system/dept/:id` | [参] |
| `deleteDept` | DELETE | `/system/dept/:id` | [参] |
| `getDeptUserList` | GET | `/system/dept/:id/users` | [参] |

#### 岗位管理（7）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getPostList` | GET | `/system/post` | [参] |
| `getPostByDept` | GET | `/system/post/dept/:deptId` | [参] |
| `getPostDetail` | GET | `/system/post/:id` | [参] |
| `createPost` | POST | `/system/post` | [参] |
| `updatePost` | PUT | `/system/post/:id` | [参] |
| `deletePost` | DELETE | `/system/post/:id` | [参] |

#### 用户管理（10）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getUserList` | GET | `/system/user` | [参] |
| `getUserDetail` | GET | `/system/user/:id` | [参] |
| `createUser` | POST | `/system/user` | [参] |
| `updateUser` | PUT | `/system/user/:id` | [参] |
| `deleteUser` | DELETE | `/system/user/:id` | [参] |
| `resetUserPwd` | PUT | `/system/user/:id/resetPwd` | [参] |
| `changeUserStatus` | PUT | `/system/user/:id/changeStatus` | [参] |
| `getUserRoles` | GET | `/system/user/:id/roles` | [参] |
| `assignUserRoles` | PUT | `/system/user/:id/roles` | [参] |

#### 角色管理（12）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getRoleList` | GET | `/system/role` | [参] |
| `getRoleDetail` | GET | `/system/role/:id` | [参] |
| `createRole` | POST | `/system/role` | [参] |
| `updateRole` | PUT | `/system/role/:id` | [参] |
| `deleteRole` | DELETE | `/system/role/:id` | [参] |
| `getRoleMenuPerms` | GET | `/system/role/:id/menuPerms` | [参] |
| `assignRoleMenuPerms` | PUT | `/system/role/:id/menuPerms` | [参] |
| `getRoleDataPerms` | GET | `/system/role/:id/dataPerms` | [参] |
| `assignRoleDataPerms` | PUT | `/system/role/:id/dataPerms` | [参] |
| `getRoleUsers` | GET | `/system/role/:id/users` | [参] |
| `copyRole` | POST | `/system/role/:id/copy` | [参] |

#### 菜单管理（6）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getMenuTree` | GET | `/system/menu/tree` | [参] |
| `getMenuList` | GET | `/system/menu` | [参] |
| `getMenuDetail` | GET | `/system/menu/:id` | [参] |
| `createMenu` | POST | `/system/menu` | [参] |
| `updateMenu` | PUT | `/system/menu/:id` | [参] |
| `deleteMenu` | DELETE | `/system/menu/:id` | [参] |

#### 字典类型（7）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getDictTypeList` | GET | `/system/dict/type` | [参] |
| `getDictTypeAll` | GET | `/system/dict/type/all` | 无 |
| `getDictTypeDetail` | GET | `/system/dict/type/:id` | [参] |
| `createDictType` | POST | `/system/dict/type` | [参] |
| `updateDictType` | PUT | `/system/dict/type/:id` | [参] |
| `deleteDictType` | DELETE | `/system/dict/type/:id` | [参] |

#### 字典数据（7）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getDictDataList` | GET | `/system/dict/data` | [参] |
| `getDictDataByType` | GET | `/system/dict/data/type/:dictType` | [参] |
| `getDictDataDetail` | GET | `/system/dict/data/:id` | [参] |
| `createDictData` | POST | `/system/dict/data` | [参] |
| `updateDictData` | PUT | `/system/dict/data/:id` | [参] |
| `deleteDictData` | DELETE | `/system/dict/data/:id` | [参] |

#### 参数配置（8）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getConfigList` | GET | `/system/config` | [参] |
| `getConfigDetail` | GET | `/system/config/:id` | [参] |
| `getConfigByKey` | GET | `/system/config/key/:key` | [参] |
| `createConfig` | POST | `/system/config` | [参] |
| `updateConfig` | PUT | `/system/config/:id` | [参] |
| `deleteConfig` | DELETE | `/system/config/:id` | [参] |
| `refreshConfigCache` | DELETE | `/system/config/refresh-cache` | 无 |

#### 操作日志（5）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getOperLogList` | GET | `/system/operlog` | [参] |
| `getOperLogDetail` | GET | `/system/operlog/:id` | [参] |
| `deleteOperLog` | DELETE | `/system/operlog/:id` | [参] |
| `cleanOperLog` | DELETE | `/system/operlog/clean` | 无 |
| `exportOperLog` | GET | `/system/operlog/export` | [参] blob |

#### 登录日志（4）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getLoginLogList` | GET | `/system/logininfor` | [参] |
| `deleteLoginLog` | DELETE | `/system/logininfor/:id` | [参] |
| `cleanLoginLog` | DELETE | `/system/logininfor/clean` | 无 |
| `exportLoginLog` | GET | `/system/logininfor/export` | [参] blob |

### 1.3 `ai.api.js` — AI 对话与全维分析域（43 个函数）

#### AI 对话（5）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `aiChat` | POST | `/ai/chat` | [参] |
| `getChatHistory` | GET | `/ai/chat/history/:session` | [参] |
| `analyzeAlgorithm` | POST | `/ai/analyze-algorithm` | [参] |
| `getAlgorithmTypes` | GET | `/ai/algorithm-types` | 无 |
| `analyzeSpiral` | POST | `/analyze/spiral` | [参] |

#### 联网搜索（5）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getWebSearchConfig` | GET | `/web-search/config` | 无 |
| `updateWebSearchConfig` | POST | `/web-search/config` | [参] |
| `testWebSearch` | POST | `/web-search/test` | 无 |
| `webSearch` | POST | `/web-search` | [参] |

#### 无穷维度优化引擎（8）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getInfiniteBenchmarks` | GET | `/ai/infinite-optimize/benchmarks` | 无 |
| `startInfiniteOptimize` | POST | `/ai/infinite-optimize/start` | [参] |
| `stopInfiniteOptimize` | POST | `/ai/infinite-optimize/stop` | 无 |
| `getInfiniteOptimizeStatus` | GET | `/ai/infinite-optimize/status` | 无 |
| `getInfiniteOptimizeResults` | GET | `/ai/infinite-optimize/results` | 无 |
| `runProviderComparison` | POST | `/ai/infinite-optimize/compare` | 无 |
| `getProviderComparison` | GET | `/ai/infinite-optimize/comparison` | 无 |
| `applyBestConfig` | POST | `/ai/infinite-optimize/apply` | [参] |

#### 本地制品引擎（4）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getArtifactConfig` | GET | `/ai/artifact/config` | 无 |
| `getArtifacts` | GET | `/ai/artifact/list` | 无 |
| `listArtifacts` [deprecated] | GET | `/ai/artifact/list` | 别名 |
| `createArtifact` | POST | `/ai/artifact/create` | [参] |

#### 全维智能分析引擎（6）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `aiFullAnalysis` | POST | `/ai/full-analysis` | [参] |
| `aiGenerateDoc` | POST | `/ai/generate-doc` | [参] |
| `aiGenerateFlowDiagram` | POST | `/ai/generate-flow-diagram` | [参] |
| `aiDevTestFix` | POST | `/ai/dev-test-fix` | [参] |
| `aiFullComplete` | POST | `/ai/full-complete` | [参] |
| `aiOptimizeDoc` | POST | `/ai/optimize-doc` | [参] |

#### 项目需求一体化（7）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `aiProjectFromChat` | POST | `/ai/project-from-chat` | [参] |
| `aiGenerateProjectGraph` | POST | `/ai/project-graph` | [参] |
| `aiLinkReqToDb` | POST | `/ai/req-db-link` | [参] |
| `allianceEnterprisePipeline` | POST | `/ai/alliance-pipeline` | [参] |
| `aiPublishArtifactsToKb` | POST | `/ai/publish-kb` | [参] |
| `aiGenerateErd` | POST | `/ai/generate-erd` | [参] |

#### AI 专家对话 & 流程图谱（2）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `aiExpertChat` | POST | `/ai/expert-chat` | [参] |
| `getEngineFlowGraph` | GET | `/ai/engine/flow-graph` | 无 |

#### 16 模块 AI 增强端点（14）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `aiRecommendOperators` | POST | `/operators/ai-recommend` | [参] |
| `aiResourceAnalysis` | POST | `/resources/ai-analysis` | [参] |
| `aiGenerateWorkflow` | POST | `/workflow/ai-generate` | [参] |
| `aiMarketSearch` | POST | `/market/ai-search` | [参] |
| `aiMcpMap` | POST | `/mcp/ai-map` | [参] |
| `aiCaomeiParse` | POST | `/caomei/ai-parse` | [参] |
| `aiAlgoLabAnalyze` | POST | `/algolab/ai-analyze` | [参] |
| `aiFusionGovern` | POST | `/fusion/ai-govern` | [参] |
| `aiMonitorDiagnose` | POST | `/monitor/ai-diagnose` | [参] |
| `aiDocsExplain` | POST | `/docs/ai-explain` | [参] |
| `aiPluginRoute` | POST | `/plugins/ai-route` | [参] |
| `aiBrowserInstruct` | POST | `/browser/ai-instruct` | [参] |
| `aiAutomationExecute` | POST | `/automation/ai-execute` | [参] |
| `getWorkbenchAiOverview` | GET | `/workbench/ai-overview` | 无 |

### 1.4 `experts.api.js` — 专家联盟域（50 个函数）

#### 专家 CRUD 与咨询（10）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getExperts` | GET | `/experts` | [参] |
| `getExpert` | GET | `/experts/:id` | [参] |
| `registerExpert` | POST | `/experts` | [参] |
| `updateExpert` | PUT | `/experts/:id` | [参] |
| `removeExpert` | DELETE | `/experts/:id` | [参] |
| `consultExpert` | POST | `/experts/:id/consult` | [参] |
| `multiExpertConsult` | POST | `/experts/multi-consult` | [参] |
| `expertDebate` | POST | `/experts/debate` | [参] |
| `getExpertCapabilities` | GET | `/experts/capabilities` | 无 |
| `routeExperts` | POST | `/experts/route` | [参] |

#### 智能咨询与算法（4）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `intelligentConsult` | POST | `/experts/intelligent-consult` | [参] |
| `algorithmAnalysis` | POST | `/experts/algorithm-analysis` | [参] |
| `getExpertMetrics` | GET | `/experts/metrics` | 无 |
| `getExpertOverview` | GET | `/experts/overview` | 无 |
| `getSingleExpertMetrics` | GET | `/experts/:id/metrics` | [参] |

#### 企业级会话持久化（13）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `createExpertSession` | POST | `/experts/sessions` | [参] |
| `getExpertSessions` | GET | `/experts/sessions` | [参] |
| `listExpertSessions` [deprecated] | GET | `/experts/sessions` | 别名 |
| `getExpertSessionStats` | GET | `/experts/sessions/stats` | 无 |
| `getExpertSession` | GET | `/experts/sessions/:id` | [参] |
| `updateExpertSession` | PUT | `/experts/sessions/:id` | [参] |
| `deleteExpertSession` | DELETE | `/experts/sessions/:id` | [参] |
| `appendSessionMessage` | POST | `/experts/sessions/:id/messages` | [参] |
| `sessionSimilarSearch` | POST | `/experts/sessions/:id/similar-search` | [参] |
| `expertSemanticSearch` | POST | `/experts/semantic-search` | [参] |
| `exportExpertSession` | GET | `/experts/sessions/:id/export` | [参] |
| `archiveExpertSession` | POST | `/experts/sessions/:id/archive` | [参] |

#### 企业级调度策略引擎（8）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getDispatcherConfig` | GET | `/experts/dispatcher/config` | 无 |
| `updateDispatcherConfig` | PUT | `/experts/dispatcher/config` | [参] |
| `getDispatcherStatus` | GET | `/experts/dispatcher/status` | 无 |
| `dispatcherDispatch` | POST | `/experts/dispatcher/dispatch` | [参] |
| `dispatcherConsult` | POST | `/experts/dispatcher/consult` | [参] |
| `dispatcherMultiConsult` | POST | `/experts/dispatcher/multi-consult` | [参] |
| `resetDispatcherExpert` | POST | `/experts/dispatcher/reset/:id` | [参] |
| `resetDispatcherAll` | POST | `/experts/dispatcher/reset-all` | 无 |

#### 专家能力图谱（8）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `getExpertGraph` | GET | `/expert-graph` | 无 |
| `getExpertGraphStats` | GET | `/expert-graph/stats` | 无 |
| `getExpertGraphNeighbors` | GET | `/expert-graph/neighbors/:id` | [参] |
| `getExpertGraphCollaborators` | GET | `/expert-graph/collaborators/:id` | [参] |
| `getExpertGraphPath` | GET | `/expert-graph/path/:source/:target` | [参] |
| `getExpertGraphCommunities` | GET | `/expert-graph/communities` | 无 |
| `findOptimalTeam` | POST | `/expert-graph/optimal-team` | [参] |
| `rebuildExpertGraph` | POST | `/expert-graph/rebuild` | 无 |

#### 企业级协作 & V2 编排（7）
| 函数 | 方法 | 路径 | 参数 |
|---|---|---|---|
| `enterpriseConsult` | POST | `/experts/enterprise/consult` | [参] |
| `enterpriseAnalyze` | POST | `/experts/enterprise/analyze` | [参] |
| `expertOrchestrate` | POST | `/experts/orchestrate` | [参] |
| `expertGeneratePlan` | POST | `/experts/plan/generate` | [参] |
| `expertExecutePlan` | POST | `/experts/plan/execute` | [参] |
| `getOrchestrationStats` | GET | `/experts/orchestration/stats` | 无 |
| `getOrchestrationPlugins` | GET | `/experts/orchestration/plugins` | 无 |
| `listOrchestrationPlugins` [deprecated] | GET | `/experts/orchestration/plugins` | 别名 |
| `getOrchestrationHistory` | GET | `/experts/orchestration/history` | [参] |

### 1.5 `alliance.js` — 联盟独立 fetch 客户端（28 个导出）

> ⚠️ 此模块**不使用 http.js axios**，直接用 `fetch` + `ALLIANCE_BASE`（`VITE_API_BASE`，默认为空字符串即同源）。
> 路径**不带 `/api` 前缀**，直接请求 `/ai/engine/alliance/*`、`/experts/*`、`/alliance/tasks/*`、`/voice/health`。

| 函数 | 方法 | 路径 | 特殊 |
|---|---|---|---|
| `getAllianceCapabilities` | GET | `/ai/engine/alliance/capabilities` | |
| `runAllianceFullSSE` | POST | `/ai/engine/alliance/full` | [SSE] |
| `runAllianceTask` | POST | `/ai/engine/alliance/full` | 别名 [SSE] |
| `getVoiceHealth` | GET | `/voice/health` | 失败时返回浏览器 TTS 兜底 |
| `allianceRegisterExpert` | POST | `/experts` | |
| `allianceGetExperts` | GET | `/experts` | |
| `allianceConsultExpert` | POST | `/experts/:id/consult` | |
| `allianceMultiExpertConsult` | POST | `/experts/multi-consult` | |
| `allianceExpertDebate` | POST | `/experts/debate` | 支持 [SSE] |
| `allianceRouteExperts` | POST | `/experts/route` | |
| `allianceIntelligentConsult` | POST | `/experts/intelligent-consult` | |
| `allianceAlgorithmAnalysis` | POST | `/experts/algorithm-analysis` | |
| `allianceGetExpertOverview` | GET | `/experts/overview` | |
| `allianceGetExpertMetrics` | GET | `/experts/metrics` | |
| `allianceGetSingleExpertMetrics` | GET | `/experts/:id/metrics` | |
| `createAllianceTask` | POST | `/alliance/tasks` | |
| `getAllianceTasks` | GET | `/alliance/tasks` | |
| `getAllianceTask` | GET | `/alliance/tasks/:id` | |
| `getCollaborationPlan` | GET | `/alliance/tasks/:id/plan` | |
| `getExecutionLogsSSE` | GET | `/alliance/tasks/:id/logs/stream` | [SSE] |
| `getFusionResults` | GET | `/alliance/tasks/:id/fusion` | |
| `pauseAllianceTask` | POST | `/alliance/tasks/:id/pause` | |
| `resumeAllianceTask` | POST | `/alliance/tasks/:id/resume` | |
| `cancelAllianceTask` | POST | `/alliance/tasks/:id/cancel` | |
| `retryAllianceTask` | POST | `/alliance/tasks/:id/retry` | |
| `getAllianceStats` | GET | `/alliance/stats` | |

### 1.6 `workflow.api.js` — 工作流/自动化/浏览器域（26 个函数）

#### 工作流（5）
| 函数 | 方法 | 路径 |
|---|---|---|
| `getWorkflowTemplates` | GET | `/ai/workflows/templates` |
| `getWorkflows` | GET | `/ai/workflows` |
| `saveWorkflow` | POST | `/ai/workflows/save` |
| `executeWorkflowDef` | POST | `/ai/workflows/execute` |
| `getWorkflowInstances` | GET | `/ai/workflows/instances` |

#### 流程图 FlowGraph（8）
| 函数 | 方法 | 路径 |
|---|---|---|
| `getFlows` | GET | `/ai/flows` |
| `createFlow` | POST | `/ai/flows` |
| `getFlow` | GET | `/ai/flows/:id` |
| `deleteFlow` | DELETE | `/ai/flows/:id` |
| `validateFlow` | POST | `/ai/flows/validate` |
| `executeFlow` | POST | `/ai/flows/execute` |
| `getFlowNodeTypes` | GET | `/ai/flows/node-types` |

#### AI 插件（4）
| 函数 | 方法 | 路径 |
|---|---|---|
| `getAiPlugins` | GET | `/ai/plugins` |
| `registerAiPlugin` | POST | `/ai/plugins/register` |
| `sendPluginMessage` | POST | `/ai/plugins/send-message` |
| `getPluginTopology` | GET | `/ai/plugins/topology` |

#### MCP 兼容层（2）
| 函数 | 方法 | 路径 |
|---|---|---|
| `mcpListTools` | POST | `/mcp` (JSON-RPC tools/list) |
| `mcpCall` | POST | `/mcp` (JSON-RPC tools/call) |

#### AI 自动化中枢（7）
| 函数 | 方法 | 路径 |
|---|---|---|
| `getAutomations` | GET | `/automation` |
| `automationList` [deprecated] | GET | `/automation` |
| `automationChat` | POST | `/automation/chat` |
| `automationRefine` | POST | `/automation/:id/refine` |
| `automationRun` | POST | `/automation/:id/run` |
| `automationPermissions` | GET | `/automation/:id/permissions` |
| `automationUpdate` | PUT | `/automation/:id` |

#### 浏览器自动化（8）
| 函数 | 方法 | 路径 |
|---|---|---|
| `getBrowserTemplates` | GET | `/ai/browser/templates` |
| `getBrowserSessions` | GET | `/ai/browser/sessions` |
| `getBrowserSession` | GET | `/ai/browser/sessions/:id` |
| `closeBrowserSession` | DELETE | `/ai/browser/sessions/:id` |
| `executeBrowserTask` | POST | `/ai/browser/execute-task` |
| `executeBrowserSteps` | POST | `/ai/browser/execute-steps` |
| `executeBrowserAction` | POST | `/ai/browser/execute-action` |
| `browserNatural` | POST | `/ai/browser/natural` |

### 1.7 `projects.api.js` — 项目/任务/资源域（24 个函数）

#### 项目中心（13）
| 函数 | 方法 | 路径 |
|---|---|---|
| `getProjects` | GET | `/projects` |
| `getProjectTypes` | GET | `/projects/types` |
| `getProjectCatalog` | GET | `/projects/catalog` |
| `getProjectStats` | GET | `/projects/stats` |
| `getProject` | GET | `/projects/:id` |
| `createProject` | POST | `/projects` |
| `updateProject` | PUT | `/projects/:id` |
| `deleteProject` | DELETE | `/projects/:id` |
| `bindProjectResources` | POST | `/projects/:id/resources` |
| `unbindProjectResource` | DELETE | `/projects/:id/resources/:rid` |
| `updateProjectResourceNote` | PUT | `/projects/:id/resources/:rid` |
| `getProjectsByResource` | GET | `/projects/by-resource` |

#### 任务管理（9）
| 函数 | 方法 | 路径 |
|---|---|---|
| `getTasks` | GET | `/tasks` |
| `getTask` | GET | `/tasks/:id` |
| `createTask` | POST | `/tasks` |
| `updateTask` | PUT | `/tasks/:id` |
| `deleteTask` | DELETE | `/tasks/:id` |
| `convertChatToTask` | POST | `/tasks/from-chat` |
| `convertTaskToChat` | POST | `/tasks/:id/to-chat` |
| `executeTask` | POST | `/tasks/:id/execute` |
| `autoCreateTask` | POST | `/tasks/auto` |

#### 资源（2）
| 函数 | 方法 | 路径 |
|---|---|---|
| `getResources` | GET | `/ai/resources` |
| `getResourceHealth` | GET | `/ai/resources/health` |

### 1.8 `graph.api.js` — 知识图谱域（19 个函数）

| 函数 | 方法 | 路径 |
|---|---|---|
| `getGraph` | GET | `/graph` |
| `getGraphStats` | GET | `/graph/stats` |
| `getCentrality` | GET | `/graph/centrality` |
| `getCommunities` | GET | `/graph/communities` |
| `getPagerank` | GET | `/graph/pagerank` |
| `getNeighbors` | GET | `/graph/neighbors/:id` |
| `getShortestPath` | GET | `/graph/path` |
| `recommendNodes` | POST | `/graph/recommend` |
| `addGraphNode` | POST | `/graph/node` |
| `addGraphEdge` | POST | `/graph/edge` |
| `propagateActivation` | POST | `/graph/activate` |
| `graphSearch` | GET | `/graph/search` |
| `toggleAutoSync` | POST | `/graph/auto-sync/toggle` |
| `getAutoSyncStatus` | GET | `/graph/auto-sync/status` |
| `getDialogueSessions` | GET | `/dialogue/sessions` |
| `listDialogueSessions` [deprecated] | GET | `/dialogue/sessions` |
| `graphExport` | GET | `/graph/export` |
| `graphImport` | POST | `/graph/import` |
| `aiGraphInsights` | POST | `/graph/ai-insights` |

### 1.9 `kb.api.js` — 云盘知识库域（21 个函数）

| 函数 | 方法 | 路径 |
|---|---|---|
| `kbListDocuments` | GET | `/kb/documents` |
| `kbGetDocument` | GET | `/kb/documents/:id` |
| `kbCreateDocument` | POST | `/kb/documents` |
| `kbUpdateDocument` | PUT | `/kb/documents/:id` |
| `kbDeleteDocument` | DELETE | `/kb/documents/:id` |
| `kbAnalyzeDocument` | POST | `/kb/documents/:id/analyze` |
| `kbBatchAnalyze` | POST | `/kb/batch-analyze` |
| `kbGetCategories` | GET | `/kb/categories` |
| `kbGetTags` | GET | `/kb/tags` |
| `kbSearch` | POST | `/kb/search` |
| `kbGetVersions` | GET | `/kb/documents/:id/versions` |
| `kbGetVersion` | GET | `/kb/documents/:id/versions/:ver` |
| `kbCreateVersion` | POST | `/kb/documents/:id/versions` |
| `kbCompareVersions` | POST | `/kb/documents/:id/versions/compare` |
| `kbRevertVersion` | POST | `/kb/documents/:id/versions/revert` |
| `kbGetEntities` | GET | `/kb/documents/:id/entities` |
| `kbGraphLink` | POST | `/kb/documents/:id/graph-link` |
| `kbGetStats` | GET | `/kb/stats` |
| `kbGetDocHistory` | GET | `/kb/documents/:id/history` |
| `kbGetHistory` | GET | `/kb/history` |

### 1.10 `llm.api.js` — LLM 网关域（20 个函数）

#### 新 LLM 网关（17）
| 函数 | 方法 | 路径 |
|---|---|---|
| `getLlmProviders` | GET | `/llm/providers` |
| `getLlmProviderPresets` | GET | `/llm/providers/presets` |
| `getLlmPresets` [deprecated] | GET | `/llm/providers/presets` |
| `getLlmProvider` | GET | `/llm/providers/:id` |
| `setActiveProvider` | POST | `/llm/providers/active` |
| `addLlmProvider` | POST | `/llm/providers` |
| `updateLlmProvider` | PUT | `/llm/providers/:id` |
| `removeLlmProvider` | DELETE | `/llm/providers/:id` |
| `enableLlmProvider` | POST | `/llm/providers/:id/enable` |
| `disableLlmProvider` | POST | `/llm/providers/:id/disable` |
| `testLlmProvider` | POST | `/llm/providers/:id/test` |
| `discoverLlmModels` | POST | `/llm/providers/:id/discover` |
| `getLlmHealth` | GET | `/llm/health` |
| `getLlmRouting` | GET | `/llm/routing` |
| `updateLlmRouting` | PUT | `/llm/routing` |
| `getLlmUsage` | GET | `/llm/usage` |
| `getLlmLogs` | GET | `/llm/logs` |
| `getLlmStats` | GET | `/llm/stats` |

#### 旧接口兼容（3）
| 函数 | 方法 | 路径 |
|---|---|---|
| `getLlmConfig` | GET | `/ai/llm/config` |
| `updateLlmConfig` | POST | `/ai/llm/config` |
| `testLlm` | POST | `/ai/llm/test` |

### 1.11 `operators.api.js` — 算子与算子商城（10 个函数）

| 函数 | 方法 | 路径 |
|---|---|---|
| `getOperators` | GET | `/operators` |
| `registerOperator` | POST | `/operators/register` |
| `executeWorkflow` | POST | `/execute` |
| `marketList` | GET | `/market` |
| `marketRandom` | GET | `/market/random` |
| `marketGet` | GET | `/market/:id` |
| `marketUpload` | POST | `/market/upload` |
| `marketUpdate` | POST | `/market/:id` |
| `marketDelete` | DELETE | `/market/:id` |
| `marketClone` | POST | `/market/:id/clone` |
| `marketExport` | GET | `/market/:id/export` |

### 1.12 `melody.api.js` — 旋律转谱（9 个函数）

| 函数 | 方法 | 路径 | 特殊 |
|---|---|---|---|
| `melodyHealth` | GET | `/melody2score/health` | |
| `melodyStatus` | GET | `/melody2score/status` | |
| `melodySamples` | GET | `/melody2score/samples` | |
| `melodyRecognize` | POST | `/melody2score/recognize` | multipart/form-data, 120s |
| `melodyRecognizeSample` | POST | `/melody2score/recognize-sample` | multipart/form-data, 120s |
| `melodyRecognizeRecord` | POST | `/melody2score/recognize-record` | 120s |
| `melodyExportSheet` | POST | `/melody2score/export-sheet` | 60s |
| `melodySaveReport` | POST | `/melody2score/save-report` | 30s |

### 1.13 `mox.api.js` — 璇玑全维治理（3 个函数）

| 函数 | 方法 | 路径 |
|---|---|---|
| `moxHealth` | GET | `/mox/health` |
| `moxOptimize` | POST | `/mox/optimize` |
| `moxPublish` | POST | `/mox/publish` |

### 1.14 `caomei.api.js` — 需求编译（3 个函数）

| 函数 | 方法 | 路径 |
|---|---|---|
| `caomeiCompile` | POST | `/caomei/compile` |
| `caomeiRefine` | POST | `/caomei/refine` |
| `caomeiTemplates` | GET | `/caomei/templates` |

### 1.15 前端视图引用统计

> 共 **53 个前端文件**从 `@/api` 导入，覆盖 views/、stores/、components/、composables/。
> 高频引用模块：`system.api`（admin 面板 12 个视图）、`experts.api`（专家中心 5 个视图）、`ai.api`（AI 中心 6 个视图）、`projects.api`（项目中心 4 个视图）、`workflow.api`（工作流 4 个视图）、`graph.api`（图谱 3 个视图）、`alliance.js`（联盟任务 2 个视图）。

---

## 2. 后端端点全量清单（按 Rust domain 分组 + legacy 分组）

### 2.1 主网关 `mox-platform-gateway-svc`（运行在 :8080）

**文件**：`platform/gateway/mox-platform-gateway-svc/src/lib.rs` `build_gateway_router()`

#### L0 通用端点（4）
| 方法 | 路径 | 实现状态 |
|---|---|---|
| GET | `/health` | ✅ 真实（返回 gateway 版本+时间戳） |
| GET | `/api/v1/status` | ✅ 真实（域统计+IAM 状态） |
| GET | `/api/v1/domains` | ✅ 真实（31 域描述符列表） |
| GET | `/metrics` | ✅ 真实（Prometheus 格式占位） |

#### L2 KG 域（6，来自 `mox-kg-service-svc/http_adapter`）
| 方法 | 路径 | 实现状态 |
|---|---|---|
| GET | `/kg/v1/neighborhood` | ✅ 真实（KnowledgeGraph::neighborhood_subgraph） |
| GET | `/kg/v1/path` | ✅ 真实（KnowledgeGraph::find_paths） |
| GET | `/kg/v1/shortest-path` | ✅ 真实 |
| GET | `/kg/v1/centrality` | ✅ 真实（4 指标） |
| GET | `/kg/v1/communities` | ✅ 真实（CNM） |
| GET | `/kg/v1/stats` | ✅ 真实 |

#### L3 AI Engine 域（4，来自 `mox-kg-service-svc/http_adapter`）
| 方法 | 路径 | 实现状态 |
|---|---|---|
| POST | `/ai/engine/process` | ✅ 真实（意图识别→能力路由） |
| POST | `/ai/engine/analyze` | ✅ 真实 |
| GET | `/ai/engine/capabilities` | ✅ 真实（7 能力矩阵） |
| GET | `/ai/engine/metrics` | ✅ 真实 |

#### L4 Alliance 域（8，**stub 桩**）
| 方法 | 路径 | 实现状态 |
|---|---|---|
| POST/GET | `/alliance/v1/tasks` | ⚠️ stub（返回空列表/假任务 ID） |
| GET/POST | `/alliance/v1/tasks/:task_id` | ⚠️ stub |
| POST | `/alliance/v1/experts/search` | ⚠️ stub（返回 2 个假专家） |
| GET | `/alliance/v1/tasks/:task_id/status` | ⚠️ stub |
| GET | `/alliance/v1/tasks/:task_id/nodes` | ⚠️ stub |
| GET/POST | `/alliance/v1/tasks/:task_id/nodes/:node_id` | ⚠️ stub |

#### L5 System 域（~50，IAM SQLite 真实读 + stub 写）
**文件**：`platform/gateway/mox-platform-gateway-svc/src/system.rs`

| 子域 | 路径前缀 | 读接口 | 写接口 |
|---|---|---|---|
| 权限 | `/api/system/permissions` | ✅ IAM 真实 | — |
| 部门 | `/api/system/dept` | ✅ IAM 真实（list/tree/detail/users） | ⚠️ IAM 真实（create/update/delete） |
| 岗位 | `/api/system/post` | ✅ IAM 真实 | ⚠️ IAM 真实 |
| 用户 | `/api/system/user` | ✅ IAM 真实（list/detail/roles） | ⚠️ IAM 真实（create/update/delete/resetPwd/changeStatus/assignRoles） |
| 角色 | `/api/system/role` | ✅ IAM 真实（list/detail/menuPerms/dataPerms/users） | ⚠️ IAM 真实（create/update/delete/copy/assignMenuPerms/assignDataPerms） |
| 菜单 | `/api/system/menu` | ✅ IAM 真实（tree/list/detail） | ⚠️ IAM 真实 |
| 字典类型 | `/api/system/dict/type` | ✅ IAM 真实 | ⚠️ IAM 真实 |
| 字典数据 | `/api/system/dict/data` | ✅ IAM 真实 | ⚠️ IAM 真实 |
| 参数配置 | `/api/system/config` | ✅ IAM 真实 | ⚠️ IAM 真实 + refresh-cache stub |
| 操作日志 | `/api/system/operlog` | ✅ IAM 真实 | ⚠️ IAM 真实（delete/clean），export stub（返回 exported:false） |
| 登录日志 | `/api/system/logininfor` | ✅ IAM 真实 | ⚠️ IAM 真实（delete/clean），export stub |

#### L5 Security 域（6，IAM 真实）
| 方法 | 路径 | 实现状态 |
|---|---|---|
| GET | `/api/security/status` | ✅ 真实 |
| GET/POST | `/api/security/api-keys` | ✅ 真实（SQLite 持久化+auth 中间件注册） |
| DELETE | `/api/security/api-keys/:id` | ✅ 真实 |
| POST | `/api/security/validate` | ✅ 真实 |
| GET | `/api/security/audit-log` | ✅ 真实 |

### 2.2 编排器 `mox-platform-orchestrator-svc`（`/api/*` 前缀，约 60 端点）

**文件**：`platform/domains/platform/svc/mox-platform-orchestrator-svc/src/main.rs`

> ⚠️ **关键问题**：此服务的路由是否并入 :8080 主网关**未确认**。网关 `lib.rs` 的 `build_gateway_router()` 未 merge 此服务的路由。若独立运行，则前端 `/api/*` 请求无法到达。

| 路径前缀 | 端点数 | 代表性端点 |
|---|---|---|
| `/api/health` `/api/status` `/api/status/full` | 3 | 系统状态 |
| `/api/operators` `/api/operators/register` `/api/execute` | 3 | 算子 |
| `/api/graph/*` | 17 | graph/stats/neighbors/centrality/communities/path/pagerank/activate/recommend/search/auto-sync/export/import/node/edge |
| `/api/dialogue/sessions` | 1 | 对话会话 |
| `/api/ai/chat` `/api/ai/chat/history/:session` | 2 | AI 对话 |
| `/api/ai/analyze-algorithm` `/api/ai/algorithm-types` | 2 | 算法分析 |
| `/api/ai/resources` `/api/ai/resources/health` | 2 | 资源 |
| `/api/ai/plugins/*` | 4 | 插件 list/register/topology/send-message |
| `/api/ai/workflows/*` | 5 | 工作流 templates/list/save/execute/instances |
| `/api/ai/flows/*` | 6 | 流程图 list/create/get/delete/validate/execute/node-types |
| `/api/ai/llm/config` `/api/ai/llm/test` | 2 | 旧 LLM 配置 |
| `/api/ai/browser/*` | 7 | 浏览器自动化 |
| `/api/caomei/*` | 3 | 需求编译 |
| `/api/analyze/spiral` | 1 | 螺旋分析 |
| `/api/mcp` | 1 | MCP JSON-RPC |
| `/api/mox/*` | 3 | 璇玑治理 |
| `/api/plugins` `/api/logs` `/api/audit` | 3 | 系统 |
| `/api/openapi.yaml` `/api/docs` | 2 | API 文档 |

**编排器缺失的端点**（前端调用但编排器未注册）：
- `/api/ai/full-analysis`、`/api/ai/generate-doc`、`/api/ai/dev-test-fix`、`/api/ai/full-complete`、`/api/ai/optimize-doc`、`/api/ai/generate-flow-diagram`
- `/api/ai/infinite-optimize/*`（8 端点）
- `/api/ai/artifact/*`（4 端点）
- `/api/ai/project-from-chat`、`/api/ai/project-graph`、`/api/ai/req-db-link`、`/api/ai/alliance-pipeline`、`/api/ai/publish-kb`、`/api/ai/generate-erd`
- `/api/ai/expert-chat`、`/api/ai/engine/flow-graph`
- `/api/web-search/*`（4 端点）
- `/api/experts/*`（全量 50 端点）
- `/api/expert-graph/*`（8 端点）
- `/api/llm/*`（新网关 17 端点，仅有旧 `/api/ai/llm/config`）
- `/api/kb/*`（全量 21 端点）
- `/api/projects/*`（13 端点）
- `/api/tasks/*`（9 端点）
- `/api/market/*`（10 端点 + ai-search）
- `/api/automation/*`（7 端点 + ai-execute）
- `/api/melody2score/*`（8 端点）
- `/api/storage/*`（3 端点）
- `/api/security/*`（在网关中，不在编排器）
- `/api/system/*`（在网关中，不在编排器）
- 16 模块 AI 增强端点（`/operators/ai-recommend`、`/resources/ai-analysis`、`/workflow/ai-generate`、`/market/ai-search`、`/mcp/ai-map`、`/caomei/ai-parse`、`/algolab/ai-analyze`、`/fusion/ai-govern`、`/monitor/ai-diagnose`、`/docs/ai-explain`、`/plugins/ai-route`、`/browser/ai-instruct`、`/automation/ai-execute`、`/workbench/ai-overview`）

### 2.3 遗留 Rust `platform/legacy/backend-rust`（**无 `/api` 前缀**，~200 端点）

**文件**：`platform/legacy/backend-rust/src/api/mod.rs`

> ⚠️ 所有路由注册为 `/ai/chat`、`/graph`、`/experts` 等，**缺少 `/api` 前缀**。前端请求 `/api/ai/chat` 经 Vite 代理（无 rewrite）到达后端后为 `/api/ai/chat`，与此处 `/ai/chat` 不匹配 → **404**。

| 域 | 路径前缀 | 端点数 | 与前端匹配度 |
|---|---|---|---|
| 系统 | `/health` `/status` `/status/full` `/logs` `/plugins` `/config` `/modules` | 7 | ⚠️ 路径不匹配（前端调 `/api/health` 等，此处无 `/api`） |
| 算子 | `/operators` `/operators/register` `/operators/ai-recommend` `/execute` | 4 | ⚠️ 路径不匹配 |
| 图谱 | `/graph/*` | 19 | ⚠️ 路径不匹配 |
| 对话 | `/dialogue/sessions` | 1 | ⚠️ 路径不匹配 |
| AI | `/ai/*` | ~40 | ⚠️ 路径不匹配（但端点最全，含 full-analysis/infinite-optimize/artifact/project-from-chat 等） |
| 联网搜索 | `/web-search/*` | 4 | ⚠️ 路径不匹配 |
| 市场 | `/market/*` | 11 | ⚠️ 路径不匹配 |
| 草莓 | `/caomei/*` | 4 | ⚠️ 路径不匹配 |
| MCP | `/mcp` `/mcp/ai-map` | 2 | ⚠️ 路径不匹配 |
| 自动化 | `/automation/*` | 8 | ⚠️ 路径不匹配 |
| 璇玑 | `/mox/*` | 3 | ⚠️ 路径不匹配 |
| LLM | `/llm/*` | 17 | ⚠️ 路径不匹配 |
| 专家 | `/experts/*` | ~50 | ⚠️ 路径不匹配 |
| 专家图谱 | `/expert-graph/*` | 8 | ⚠️ 路径不匹配 |
| 任务 | `/tasks/*` | 10 | ⚠️ 路径不匹配 |
| 项目 | `/projects/*` | 13 | ⚠️ 路径不匹配 |
| 知识库 | `/kb/*` | 21 | ⚠️ 路径不匹配 |
| 旋律 | `/melody2score/*` | 8 | ⚠️ 路径不匹配 |
| 安全 | `/security/*` | 6 | ⚠️ 路径不匹配 |
| 存储 | `/storage/*` | 3 | ⚠️ 路径不匹配 |
| 螺旋分析 | `/analyze/spiral` | 1 | ⚠️ 路径不匹配 |
| 16 模块 AI 增强 | `/operators/ai-recommend` 等 | 14 | ⚠️ 路径不匹配 |

### 2.4 遗留 Python `platform/legacy/mox-server`（FastAPI，`/api/*` 前缀）

**文件**：`platform/legacy/mox-server/mox/server.py`

| 域 | 路径前缀 | 端点数 | 与前端交集 |
|---|---|---|---|
| DSQL | `/api/dsql/execute` `/api/dsql/explain` `/api/dsql/execute-batch` | 3 | ❌ 前端无对应函数 |
| Admin SQL | `/api/admin/sqls/*` | 7 | ❌ 前端无对应函数 |
| Admin 数据源 | `/api/admin/datasources/*` | 3 | ❌ 前端无对应函数 |
| Admin 权限 | `/api/admin/permissions` | 3 | ❌ 前端无对应函数 |
| Admin 角色/用户 | `/api/admin/roles` `/api/admin/users` | 2 | ❌ 前端用 `/api/system/role` |
| KG | `/api/kg/graph` `/api/kg/query` `/api/kg/traverse` | 3 | ❌ 前端用 `/api/graph` |
| KG Admin | `/api/admin/kg/vertices` `/api/admin/kg/edges` | 4 | ❌ 前端无对应函数 |
| 缓存 | `/api/cache/stats` `/api/cache/clear` | 2 | ❌ 前端无对应函数 |
| 官网 | `/api/website/message` `/api/website/resume` `/api/website/consultation` | 3 | ❌ 前端无对应函数 |
| 平台 | `/api/stats` `/api/audit` `/api/health` | 3 | ⚠️ 部分交集（前端 `getHealth` 调 `/health` 非 `/api/health`） |
| 应用 | `/api/apps/*` | 8 | ❌ 前端无对应函数 |
| 流程 | `/api/process/flow` | 1 | ❌ 前端无对应函数 |
| AI | `/api/ai/assistant` `/api/ai/requests` | 2 | ❌ 前端用 `/api/ai/chat` |

> **结论**：Python legacy 与前端业务域**几乎无交集**，是独立的低代码平台运行服务。

### 2.5 其他 Rust 微服务（独立进程，非网关直接路由）

| 服务 | 路径前缀 | 说明 |
|---|---|---|
| `mox-alliance-scheduler-svc` | `/health` `/tasks` `/experts/search` | 联盟调度器（无 `/api` 前缀） |
| `mox-alliance-executor-svc` | `/health` `/tasks/:id/*` `/internal/executions` | 联盟执行器 |
| `mox-flow-primiflow-svc` | `/api/health` `/api/projects` `/api/topologies/*` `/api/assets` | 低代码拓扑（有 `/api` 前缀但路径不同） |
| `mox-flow-fusion-svc` | `/api/v1/synthesize` `/api/v1/registry/*` `/api/v1/persist` `/api/v1/gate` `/api/v1/docs` | 融合服务 |
| `mox-voice-operator-svc` | `/health` `/v1/dispatch_text` `/v1/tts` `/v1/asr` `/v1/avatar/*` `/ws` | 语音算子（前端 `alliance.js` 调 `/voice/health`，网关可能代理） |
| `mox-project-graph-svc` | `/api/v1/projects` `/api/v1/tasks` `/api/v1/persons` `/api/v1/graph/*` | 项目图谱（`/api/v1` 前缀，前端用 `/api/projects`） |
| `mox-ai-expert-svc` | `/api/alliance/experts/*` `/api/alliance/full` `/api/alliance/debate` 等 | AI 专家服务（`/api/alliance` 前缀，前端 `alliance.js` 调 `/experts` 无 `/api/alliance`） |
| `mox-ai-intent-svc` | `/api/v1/intent/*` `/api/v1/sessions/*` | 意图识别 |
| `mox-kg-hub-svc` | `/api/kg/*` | KG Hub（前端用 `/api/graph`） |
| `mox-platform-enterprise-svc` | `/auth/login` `/entities/define` `/data/*` | 企业服务 |
| `mox-platform-system-core` | `/api/me` `/api/members` `/api/tasks` `/api/channels` `/api/notifications` | 系统核心 |
| `mox-content-publisher` | `/publish` `/platforms` `/health` | 内容发布 |

---

## 3. 对接矩阵表

> **判定规则**：
> - ✅ **已对接**：主网关 :8080 有对应路由且真实实现
> - ⚠️ **仅mock/stub**：主网关有路由但返回 stub 数据
> - ⚠️ **路径不匹配**：仅在 legacy backend-rust 存在但无 `/api` 前缀（或在其他微服务但路径前缀不同）
> - ❌ **后端缺失**：在任何 Rust/Python 代码中都找不到对应端点
> - 🔴 **500**：已知运行时返回 500（基于代码分析推断）

### 3.1 system.api.js — 系统管理域（95 函数）

| 前端函数 | 期望端点 | 网关(:8080) | 编排器 | legacy Rust | 状态 |
|---|---|---|---|---|---|
| `getHealth` | GET `/health` | ✅ `/health` | ✅ `/api/health` | ✅ `/health` | ✅已对接 |
| `getStatus` | GET `/status` | ❌ | ✅ `/api/status` | ✅ `/status` | ⚠️路径不匹配 |
| `getFullStatus` | GET `/status/full` | ❌ | ✅ `/api/status/full` | ✅ `/status/full` | ⚠️路径不匹配 |
| `getLogs` | GET `/logs` | ❌ | ✅ `/api/logs` | ✅ `/logs` | ⚠️路径不匹配 |
| `getPlugins` | GET `/plugins` | ❌ | ✅ `/api/plugins` | ✅ `/plugins` | ⚠️路径不匹配 |
| `getSystemConfig` | GET `/config` | ❌ | ❌ | ✅ `/config` | ⚠️路径不匹配 |
| `getSecurityStatus` | GET `/security/status` | ✅ `/api/security/status` | ❌ | ✅ `/security/status` | ✅已对接 |
| `getApiKeys` | GET `/security/api-keys` | ✅ `/api/security/api-keys` | ❌ | ✅ `/security/api-keys` | ✅已对接 |
| `createApiKey` | POST `/security/api-keys` | ✅ | ❌ | ✅ | ✅已对接 |
| `revokeApiKey` | DELETE `/security/api-keys/:id` | ✅ | ❌ | ✅ | ✅已对接 |
| `validateApiKey` | POST `/security/validate` | ✅ | ❌ | ✅ | ✅已对接 |
| `getAuditLogs` | GET `/security/audit-log` | ✅ | ❌ | ✅ | ✅已对接 |
| `getStorageProviders` | GET `/storage/providers` | ❌ | ❌ | ✅ `/storage/providers` | ⚠️路径不匹配 |
| `switchStorageProvider` | POST `/storage/switch` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `getStorageStatus` | GET `/storage/status` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `getModules` | GET `/modules` | ❌ | ❌ | ✅ `/modules` | ⚠️路径不匹配 |
| `getPermissions` | GET `/system/permissions` | ✅ `/api/system/permissions` | ❌ | ❌ | ✅已对接 |
| 部门 CRUD (7) | `/system/dept/*` | ✅ `/api/system/dept/*` | ❌ | ❌ | ✅已对接 |
| 岗位 CRUD (6) | `/system/post/*` | ✅ `/api/system/post/*` | ❌ | ❌ | ✅已对接 |
| 用户 CRUD (9) | `/system/user/*` | ✅ `/api/system/user/*` | ❌ | ❌ | ✅已对接 |
| 角色 CRUD (11) | `/system/role/*` | ✅ `/api/system/role/*` | ❌ | ❌ | ✅已对接 |
| 菜单 CRUD (6) | `/system/menu/*` | ✅ `/api/system/menu/*` | ❌ | ❌ | ✅已对接 |
| 字典类型 (6) | `/system/dict/type/*` | ✅ | ❌ | ❌ | ✅已对接 |
| 字典数据 (6) | `/system/dict/data/*` | ✅ | ❌ | ❌ | ✅已对接 |
| 参数配置 (7) | `/system/config/*` | ✅（refresh-cache stub） | ❌ | ❌ | ✅已对接 |
| 操作日志 (5) | `/system/operlog/*` | ✅（export stub） | ❌ | ❌ | ✅已对接 |
| 登录日志 (4) | `/system/logininfor/*` | ✅（export stub） | ❌ | ❌ | ✅已对接 |

**system 域小结**：✅ 已对接 86 个（IAM 真实），⚠️ 路径不匹配 9 个（health/status/logs/plugins/config/storage/modules），❌ 缺失 0 个。

### 3.2 ai.api.js — AI 对话与全维分析域（43 函数）

| 前端函数 | 期望端点 | 网关(:8080) | 编排器 | legacy Rust | 状态 |
|---|---|---|---|---|---|
| `aiChat` | POST `/ai/chat` | ❌ | ✅ `/api/ai/chat` | ✅ `/ai/chat` | ⚠️路径不匹配* |
| `getChatHistory` | GET `/ai/chat/history/:session` | ❌ | ✅ | ✅ | ⚠️路径不匹配* |
| `analyzeAlgorithm` | POST `/ai/analyze-algorithm` | ❌ | ✅ | ✅ | ⚠️路径不匹配* |
| `getAlgorithmTypes` | GET `/ai/algorithm-types` | ❌ | ✅ | ✅ | ⚠️路径不匹配* |
| `analyzeSpiral` | POST `/analyze/spiral` | ❌ | ✅ `/api/analyze/spiral` | ✅ | ⚠️路径不匹配* |
| `getWebSearchConfig` | GET `/web-search/config` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `updateWebSearchConfig` | POST `/web-search/config` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `testWebSearch` | POST `/web-search/test` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `webSearch` | POST `/web-search` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `getInfiniteBenchmarks` | GET `/ai/infinite-optimize/benchmarks` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `startInfiniteOptimize` | POST `/ai/infinite-optimize/start` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `stopInfiniteOptimize` | POST `/ai/infinite-optimize/stop` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `getInfiniteOptimizeStatus` | GET `/ai/infinite-optimize/status` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `getInfiniteOptimizeResults` | GET `/ai/infinite-optimize/results` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `runProviderComparison` | POST `/ai/infinite-optimize/compare` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `getProviderComparison` | GET `/ai/infinite-optimize/comparison` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `applyBestConfig` | POST `/ai/infinite-optimize/apply` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `getArtifactConfig` | GET `/ai/artifact/config` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `getArtifacts` | GET `/ai/artifact/list` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `createArtifact` | POST `/ai/artifact/create` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `aiFullAnalysis` | POST `/ai/full-analysis` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `aiGenerateDoc` | POST `/ai/generate-doc` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `aiGenerateFlowDiagram` | POST `/ai/generate-flow-diagram` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `aiDevTestFix` | POST `/ai/dev-test-fix` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `aiFullComplete` | POST `/ai/full-complete` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `aiOptimizeDoc` | POST `/ai/optimize-doc` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `aiProjectFromChat` | POST `/ai/project-from-chat` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `aiGenerateProjectGraph` | POST `/ai/project-graph` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `aiLinkReqToDb` | POST `/ai/req-db-link` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `allianceEnterprisePipeline` | POST `/ai/alliance-pipeline` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `aiPublishArtifactsToKb` | POST `/ai/publish-kb` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `aiGenerateErd` | POST `/ai/generate-erd` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `aiExpertChat` | POST `/ai/expert-chat` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `getEngineFlowGraph` | GET `/ai/engine/flow-graph` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| 16模块AI增强 (14) | `/operators/ai-recommend` 等 | ❌ | ❌ | ✅（多数） | ⚠️路径不匹配 |

> *注：编排器有 `/api/ai/chat` 等正确前缀的路由，但若编排器未并入 :8080 主网关，则这些端点实际不可达。此处标记为"路径不匹配"是基于 legacy 路径，实际取决于编排器部署状态。

**ai 域小结**：✅ 已对接 0 个（网关无 `/api/ai/*`），⚠️ 路径不匹配/编排器待确认 43 个，❌ 缺失 0 个（legacy 全有但无 `/api` 前缀）。

### 3.3 experts.api.js — 专家联盟域（50 函数）

| 前端函数组 | 期望端点 | 网关(:8080) | 编排器 | legacy Rust | ai-expert-svc | 状态 |
|---|---|---|---|---|---|---|
| 专家 CRUD/咨询 (10) | `/experts` `/experts/:id` `/experts/:id/consult` 等 | ❌ | ❌ | ✅ `/experts/*` | ✅ `/api/alliance/experts/*` | ⚠️路径不匹配 |
| 智能咨询/算法 (5) | `/experts/intelligent-consult` 等 | ❌ | ❌ | ✅ | ✅ | ⚠️路径不匹配 |
| 会话持久化 (13) | `/experts/sessions/*` | ❌ | ❌ | ✅ | ❌ | ⚠️路径不匹配 |
| 调度引擎 (8) | `/experts/dispatcher/*` | ❌ | ❌ | ✅ | ❌ | ⚠️路径不匹配 |
| 专家图谱 (8) | `/expert-graph/*` | ❌ | ❌ | ✅ | ❌ | ⚠️路径不匹配 |
| 企业协作/V2编排 (7) | `/experts/enterprise/*` `/experts/orchestrate` 等 | ❌ | ❌ | ✅ | ✅ `/api/alliance/orchestrate` | ⚠️路径不匹配 |

**experts 域小结**：✅ 已对接 0 个，⚠️ 路径不匹配 50 个（legacy 全有但无 `/api`；ai-expert-svc 有 `/api/alliance/experts` 前缀不同），❌ 缺失 0 个。

### 3.4 alliance.js — 联盟独立 fetch 客户端（28 函数）

> ⚠️ 此模块直接 fetch 同源路径，**不带 `/api` 前缀**。Vite 代理仅配置了 `/api`、`/ai/engine`、`/voice`、`/ws` 转发到 :8080。`/experts`、`/alliance/tasks` 等**未配置代理**，浏览器直接请求前端 dev server → 404。

| 函数组 | 期望端点 | 网关(:8080) | Vite代理 | 状态 |
|---|---|---|---|---|
| `getAllianceCapabilities` | GET `/ai/engine/alliance/capabilities` | ❌（网关只有 `/ai/engine/process` 等 4 个） | ✅ `/ai/engine` 已代理 | ❌后端缺失 |
| `runAllianceFullSSE` | POST `/ai/engine/alliance/full` | ❌ | ✅ 已代理 | ❌后端缺失 |
| `getVoiceHealth` | GET `/voice/health` | ❌（网关无 `/voice` 路由，routing.rs 有概念但 api_handler 返回占位 JSON） | ✅ `/voice` 已代理 | ⚠️仅mock（失败时浏览器 TTS 兜底） |
| `allianceRegisterExpert` 等 (11) | `/experts` `/experts/:id/consult` 等 | ❌ | ❌ 未配置代理 | ❌后端缺失 |
| `createAllianceTask` 等 (10) | `/alliance/tasks` `/alliance/tasks/:id/*` | ⚠️ 网关有 `/alliance/v1/tasks`（stub），但前端调 `/alliance/tasks`（无 v1） | ❌ 未配置代理 | ⚠️路径不匹配（v1 前缀差异 + 无代理） |
| `getAllianceStats` | GET `/alliance/stats` | ❌ | ❌ | ❌后端缺失 |

**alliance 域小结**：✅ 已对接 0 个，⚠️ 路径不匹配/仅mock 12 个，❌ 后端缺失 16 个。**此模块是缺口最严重的域**。

### 3.5 workflow.api.js — 工作流/自动化/浏览器域（26 函数）

| 函数组 | 期望端点 | 网关(:8080) | 编排器 | legacy Rust | 状态 |
|---|---|---|---|---|---|
| 工作流 (5) | `/ai/workflows/*` | ❌ | ✅ `/api/ai/workflows/*` | ✅ | ⚠️路径不匹配* |
| 流程图 (7) | `/ai/flows/*` | ❌ | ✅ `/api/ai/flows/*` | ✅ | ⚠️路径不匹配* |
| AI 插件 (4) | `/ai/plugins/*` | ❌ | ✅ `/api/ai/plugins/*` | ✅ | ⚠️路径不匹配* |
| MCP (2) | `/mcp` | ❌ | ✅ `/api/mcp` | ✅ | ⚠️路径不匹配* |
| 自动化 (7) | `/automation/*` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| 浏览器自动化 (8) | `/ai/browser/*` | ❌ | ✅ `/api/ai/browser/*` | ✅ | ⚠️路径不匹配* |

**workflow 域小结**：✅ 已对接 0 个，⚠️ 路径不匹配/编排器待确认 26 个。

### 3.6 projects.api.js — 项目/任务/资源域（24 函数）

| 函数组 | 期望端点 | 网关(:8080) | 编排器 | legacy Rust | project-graph-svc | 状态 |
|---|---|---|---|---|---|---|
| 项目中心 (13) | `/projects/*` | ❌ | ❌ | ✅ `/projects/*` | ✅ `/api/v1/projects/*` | ⚠️路径不匹配 |
| 任务管理 (9) | `/tasks/*` | ❌ | ❌ | ✅ `/tasks/*` | ✅ `/api/v1/tasks/*` | ⚠️路径不匹配 |
| 资源 (2) | `/ai/resources` `/ai/resources/health` | ❌ | ✅ `/api/ai/resources` | ✅ | ❌ | ⚠️路径不匹配* |

**projects 域小结**：✅ 已对接 0 个，⚠️ 路径不匹配 24 个。

### 3.7 graph.api.js — 知识图谱域（19 函数）

| 函数组 | 期望端点 | 网关(:8080) | 编排器 | legacy Rust | kg-hub-svc | 状态 |
|---|---|---|---|---|---|---|
| 图谱核心 (12) | `/graph` `/graph/stats` `/graph/neighbors/:id` 等 | ❌（网关有 `/kg/v1/*` 非 `/graph/*`） | ✅ `/api/graph/*` | ✅ `/graph/*` | ✅ `/api/kg/*` | ⚠️路径不匹配* |
| 对话同步 (4) | `/graph/search` `/graph/auto-sync/*` `/dialogue/sessions` | ❌ | ✅ | ✅ | ❌ | ⚠️路径不匹配* |
| 导入导出 (2) | `/graph/export` `/graph/import` | ❌ | ✅ | ✅ | ❌ | ⚠️路径不匹配* |
| AI 图谱增强 (1) | `/graph/ai-insights` | ❌ | ❌ | ✅ | ❌ | ⚠️路径不匹配 |

**graph 域小结**：✅ 已对接 0 个（网关 `/kg/v1/*` 路径不同），⚠️ 路径不匹配/编排器待确认 19 个。

### 3.8 kb.api.js — 云盘知识库域（21 函数）

| 期望端点 | 网关(:8080) | 编排器 | legacy Rust | 状态 |
|---|---|---|---|---|
| `/kb/documents/*` (CRUD+analyze) | ❌ | ❌ | ✅ `/kb/documents/*` | ⚠️路径不匹配 |
| `/kb/batch-analyze` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `/kb/categories` `/kb/tags` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `/kb/search` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `/kb/documents/:id/versions/*` (5) | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `/kb/documents/:id/entities` `/graph-link` `/history` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |
| `/kb/stats` `/kb/history` | ❌ | ❌ | ✅ | ⚠️路径不匹配 |

**kb 域小结**：✅ 已对接 0 个，⚠️ 路径不匹配 21 个（legacy 全有但无 `/api` 前缀）。

### 3.9 llm.api.js — LLM 网关域（20 函数）

| 函数组 | 期望端点 | 网关(:8080) | 编排器 | legacy Rust | 状态 |
|---|---|---|---|---|---|
| 新 LLM 网关 (17) | `/llm/providers/*` `/llm/health` `/llm/routing` 等 | ❌ | ❌ | ✅ `/llm/*` | ⚠️路径不匹配 |
| 旧接口兼容 (3) | `/ai/llm/config` `/ai/llm/test` | ❌ | ✅ `/api/ai/llm/config` | ✅ | ⚠️路径不匹配* |

**llm 域小结**：✅ 已对接 0 个，⚠️ 路径不匹配 20 个。

### 3.10 operators.api.js — 算子与算子商城（11 函数）

| 函数组 | 期望端点 | 网关(:8080) | 编排器 | legacy Rust | 状态 |
|---|---|---|---|---|---|
| 算子 (3) | `/operators` `/operators/register` `/execute` | ❌ | ✅ `/api/operators` | ✅ | ⚠️路径不匹配* |
| 算子商城 (8) | `/market/*` | ❌ | ❌ | ✅ `/market/*` | ⚠️路径不匹配 |

**operators 域小结**：✅ 已对接 0 个，⚠️ 路径不匹配 11 个。

### 3.11 melody.api.js — 旋律转谱（8 函数）

| 期望端点 | 网关(:8080) | 编排器 | legacy Rust | 状态 |
|---|---|---|---|---|
| `/melody2score/*` (8) | ❌ | ❌ | ✅ `/melody2score/*` | ⚠️路径不匹配 |

> 注：`projects/melody2score` 有独立 FastAPI 服务运行在 :8012，但前端请求 `/api/melody2score/*` 经代理到 :8080，不会转发到 :8012。

### 3.12 mox.api.js — 璇玑全维治理（3 函数）

| 期望端点 | 网关(:8080) | 编排器 | legacy Rust | 状态 |
|---|---|---|---|---|
| `/mox/health` `/mox/optimize` `/mox/publish` | ❌ | ✅ `/api/mox/*` | ✅ `/mox/*` | ⚠️路径不匹配* |

### 3.13 caomei.api.js — 需求编译（3 函数）

| 期望端点 | 网关(:8080) | 编排器 | legacy Rust | 状态 |
|---|---|---|---|---|
| `/caomei/compile` `/caomei/refine` `/caomei/templates` | ❌ | ✅ `/api/caomei/*` | ✅ `/caomei/*` | ⚠️路径不匹配* |

---

## 4. 缺口汇总

### 4.1 按域统计

| 域 | 前端函数数 | ✅已对接 | ⚠️路径不匹配/编排器待确认 | ⚠️仅mock/stub | ❌后端缺失 | 缺口率 |
|---|---|---|---|---|---|---|
| system | 95 | 86 | 9 | 0 | 0 | 9.5% |
| ai | 43 | 0 | 43 | 0 | 0 | 100% |
| experts | 50 | 0 | 50 | 0 | 0 | 100% |
| alliance.js | 28 | 0 | 12 | 1 | 15 | 100% |
| workflow | 26 | 0 | 26 | 0 | 0 | 100% |
| projects | 24 | 0 | 24 | 0 | 0 | 100% |
| graph | 19 | 0 | 19 | 0 | 0 | 100% |
| kb | 21 | 0 | 21 | 0 | 0 | 100% |
| llm | 20 | 0 | 20 | 0 | 0 | 100% |
| operators | 11 | 0 | 11 | 0 | 0 | 100% |
| melody | 8 | 0 | 8 | 0 | 0 | 100% |
| mox | 3 | 0 | 3 | 0 | 0 | 100% |
| caomei | 3 | 0 | 3 | 0 | 0 | 100% |
| **合计** | **348** | **86** | **249** | **1** | **15** | **75.3%** |

### 4.2 根因分析

1. **主网关路由覆盖不足（P0）**：`mox-platform-gateway-svc` 的 `build_gateway_router()` 仅注册了 `/api/system/*`、`/api/security/*`、`/kg/v1/*`、`/ai/engine/*`、`/alliance/v1/*`，**完全没有注册 `/api/ai/*`、`/api/graph/*`、`/api/experts/*`、`/api/kb/*`、`/api/llm/*`、`/api/projects/*`、`/api/tasks/*`、`/api/market/*`、`/api/automation/*`、`/api/melody2score/*` 等业务域路由**。

2. **遗留后端路径前缀不一致（P0）**：`platform/legacy/backend-rust/src/api/mod.rs` 注册了全量 ~200 端点，但路径为 `/ai/chat` 而非 `/api/ai/chat`。Vite 代理无 rewrite 配置，导致前端请求到达后端时路径不匹配。

3. **编排器路由未并入主网关（P1）**：`mox-platform-orchestrator-svc` 有约 60 个 `/api/*` 前缀的正确路由，但网关 `lib.rs` 未 merge 其 router。若编排器独立运行在其他端口，Vite 代理不会转发。

4. **alliance.js 独立 fetch 缺少代理配置（P1）**：`alliance.js` 直接 fetch `/experts`、`/alliance/tasks` 等路径，Vite 仅配置了 `/api`、`/ai/engine`、`/voice`、`/ws` 代理，`/experts` 和 `/alliance` 未配置代理 → 浏览器直接请求 dev server → 404。

5. **微服务路径前缀碎片化（P2）**：各微服务使用不同前缀（`/api/v1/projects`、`/api/alliance/experts`、`/api/kg`、`/api/v1/intent`），与前端期望的 `/api/projects`、`/api/experts`、`/api/graph` 不一致。

### 4.3 P0 必须补的端点（按优先级排序）

> P0 = 前端高频调用且主网关完全缺失，直接导致核心功能页面白屏/报错

| 优先级 | 端点组 | 前端函数数 | 影响页面 | 建议方案 |
|---|---|---|---|---|
| **P0-1** | `/api/ai/chat` + `/api/ai/chat/history/:session` | 2 | AI 对话主页（核心入口） | 网关 merge 编排器路由，或 Vite 代理加 rewrite strip `/api` |
| **P0-2** | `/api/graph` + `/api/graph/stats` + `/api/graph/neighbors/:id` | 3+ | 知识图谱主页 | 同上 |
| **P0-3** | `/api/experts` + `/api/experts/:id` + `/api/experts/:id/consult` | 5+ | 专家广场/专家中心 | 同上 |
| **P0-4** | `/api/projects` + `/api/projects/:id` | 4+ | 项目中心 | 同上 |
| **P0-5** | `/api/operators` + `/api/execute` | 2 | 算子中心 | 同上 |
| **P0-6** | `/api/ai/workflows` + `/api/ai/flows` | 4+ | 工作流/流程图 | 同上 |
| **P0-7** | `/api/llm/providers` + `/api/llm/health` | 3+ | 管理面板-LLM 配置 | 同上 |
| **P0-8** | `/api/kb/documents` | 3+ | 云盘知识库 | 同上 |
| **P0-9** | `/api/market` + `/api/market/:id` | 4+ | 算子商城 | 同上 |
| **P0-10** | `/api/automation` | 2+ | 自动化中枢 | 同上 |
| **P0-11** | `/api/web-search` + `/api/web-search/config` | 4 | AI 对话-联网搜索 | Vite rewrite 到 legacy |
| **P0-12** | `/api/ai/full-analysis` + `/api/ai/generate-doc` 等全维分析 | 6 | AI 全维分析面板 | Vite rewrite 到 legacy |
| **P0-13** | `/api/ai/infinite-optimize/*` | 8 | 无穷维度优化器 | Vite rewrite 到 legacy |
| **P0-14** | alliance.js `/experts/*` + `/alliance/tasks/*` 代理配置 | 21 | 联盟任务视图 | Vite 增加 `/experts`、`/alliance` 代理规则 |
| **P0-15** | `/api/status` + `/api/status/full` + `/api/logs` + `/api/plugins` | 4 | 管理面板-系统监控 | 网关增加路由或 Vite rewrite |

### 4.4 快速修复建议（不改代码，仅审计结论）

1. **最小改动方案**：在 `vite.config.js` 的 proxy 中为 `/api` 增加 `rewrite: (path) => path.replace(/^\/api/, '')`，将 `/api/ai/chat` 重写为 `/ai/chat`，使 legacy backend-rust 的全量端点可达。但需确认 legacy backend-rust 是否在 :8080 运行。

2. **正确架构方案**：在网关 `build_gateway_router()` 中 merge `mox-platform-orchestrator-svc` 的路由（或各域服务的 http_adapter），使 `/api/*` 业务路由在 :8080 直接可达。

3. **alliance.js 修复**：Vite proxy 增加 `/experts` 和 `/alliance` 前缀转发到 :8080，或修改 alliance.js 使用 `/api` 前缀。

4. **system 域写接口**：当前网关 system 写接口已接入 IAM 仓储真实实现，操作日志/登录日志的 export 端点返回 stub（`exported: false`），需补真实 CSV 导出。

---

## 5. 审计元数据

| 项 | 值 |
|---|---|
| 审计时间 | 2026-09-01 |
| 审计范围 | frontend-ui/src/api/ (16模块) + platform/ (Rust workspace) + platform/legacy/ |
| 前端 API 函数总数 | ~348（含 deprecated 别名） |
| 后端已注册端点（网关） | ~70（system 50 + security 6 + kg 6 + ai-engine 4 + alliance 8 + l0 4） |
| 后端已注册端点（编排器） | ~60 |
| 后端已注册端点（legacy Rust） | ~200（无 /api 前缀） |
| 后端已注册端点（legacy Python） | ~40 |
| 前端视图引用文件数 | 53 |
| 对接成功率 | 24.7%（86/348，仅 system/security 域） |
| 核心缺口 | 主网关未注册 /api/ai、/api/graph、/api/experts、/api/kb、/api/llm、/api/projects 等业务域路由 |

> **免责声明**：本审计为静态代码分析，未实际运行后端服务进行 HTTP 探测。"已对接"判定基于网关 `build_gateway_router()` 中实际 merge 的路由。编排器路由是否在运行时并入主进程需通过实际部署确认。

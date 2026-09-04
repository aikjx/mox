# 前端 API 接口后端实现完整性核验报告 — 分片1：AI/LLM/Graph 域

> **核验日期**：2026-09-03  
> **核验范围**：`frontend-ui/src/api/ai.api.js`、`llm.api.js`、`graph.api.js`  
> **架构基线**：网关（Rust/axum :8080）→ 代理转发 `/api/*` → 编排器（:3001）；`/api/projects/*` → PrimiFlow（:8000）  
> **响应信封**：统一协议 `{code, msg, data}`，`code===0` 成功并解包 data；编排器与网关 kg_ai 路由均使用 `mox_api_protocol::ApiResponse`，与前端拦截器一致。

---

## 架构追踪结论（前置）

### 路由分发链路
1. **网关原生路由**（`lib.rs::build_gateway_router` 合并）：actuator、l0、kg_ai（`/kg/v1/*` + `/ai/engine/*`，**无 `/api` 前缀**）、kb、alliance、system、security、monitor、workspace、projects_ext、experts_ext、misc、kb_ext、notification。
2. **业务代理**（`proxy.rs::build_proxy_router`）：`/api/projects/*` → PrimiFlow；其余 `/api/{*path}` → 编排器 `:3001`，**完整路径（含 `/api` 前缀）原样转发**。
3. **编排器路由**（`main.rs`）：直接注册 `/api/*` 路由 + `.nest("/api/market")`、`.nest("/api/automation")`、`.nest("/api/agent")`、`.nest("/api/governance")`；另有 `.nest("/ai/engine")`（**无 `/api` 前缀**，与前端 `/api/ai/engine/*` 不匹配）。

### 关键发现：`/ai/engine/*` 路径前缀错位
- 网关 kg_ai 路由（`http_adapter.rs:770-773`）注册在 `/ai/engine/{process,analyze,capabilities,metrics}`，**无 `/api` 前缀**。
- 编排器（`main.rs:566`）`.nest("/ai/engine", ...)` 同样**无 `/api` 前缀**。
- 前端 `baseURL='/api'`，所有调用均为 `/api/ai/engine/*`。
- 结论：`/api/ai/engine/*` 请求经代理转发到编排器后，既不匹配编排器 `/ai/engine` nest（前缀差 `/api`），也不匹配任何 `/api/*` 直连路由，**必然 404**。网关 kg_ai 的 4 个引擎端点对前端不可达。

### Legacy 后端定位
大量端点仅存在于 `platform/legacy/backend-rust/src/api/mod.rs`（已废弃的 Node→Rust 迁移期后端），**不在当前网关代理路径中**。以下标注「legacy-only」的端点均指此情况，前端调用将 404。

---

## 一、ai.api.js 核验明细

> 文件：`frontend-ui/src/api/ai.api.js`  
> 非 deprecated 导出函数：**49 个**（`listArtifacts` 为 `getArtifacts` 别名，跳过）

| # | 前端函数 | 前端调用(file:line) | 方法+路径 | 后端实现(file:line) | 判定 | 证据/差异说明 |
|---|---------|---------------------|----------|---------------------|------|--------------|
| 1 | aiChat | ai.api.js:5 | POST /api/ai/chat | 编排器 main.rs:492（路由），handler main.rs:2034 | ✅ 已实现（代理→编排器） | 方法一致；路由 `/api/ai/chat` 精确匹配；响应 `ApiResponse{code,msg,data}` 与前端解包一致 |
| 2 | getChatHistory | ai.api.js:6 | GET /api/ai/chat/history/:session | 编排器 main.rs:493（路由），handler main.rs:2070 | ✅ 已实现（代理→编排器） | 路径参数 `:session` 匹配；前端 `encodeURIComponent(session)` 与 axum path extractor 兼容 |
| 3 | analyzeAlgorithm | ai.api.js:7 | POST /api/ai/analyze-algorithm | 编排器 main.rs:499（路由），handler main.rs:2191 | ✅ 已实现（代理→编排器） | 方法一致；handler 调用 `state.ai_agent.analyze_algorithm()` 真实实现 |
| 4 | getAlgorithmTypes | ai.api.js:8 | GET /api/ai/algorithm-types | 编排器 main.rs:500（路由），handler `list_algorithm_types` | ✅ 已实现（代理→编排器） | 方法一致；路由精确匹配 |
| 5 | analyzeSpiral | ai.api.js:9 | POST /api/analyze/spiral | 编排器 main.rs:541（路由），handler main.rs:2176 | ✅ 已实现（代理→编排器） | 方法一致；handler 调用 `mox_data_catalog_svc::spiral::analyze_spiral()` 真实算法 |
| 6 | getWebSearchConfig | ai.api.js:12 | GET /api/web-search/config | 无（legacy-only: mod.rs:641） | ❌ 未实现 | 编排器无 `/api/web-search/*` 路由；仅 legacy 后端有；代理转发后 404 |
| 7 | updateWebSearchConfig | ai.api.js:13 | POST /api/web-search/config | 无（legacy-only: mod.rs:642） | ❌ 未实现 | 同上 |
| 8 | testWebSearch | ai.api.js:14 | POST /api/web-search/test | 无（legacy-only: mod.rs:643） | ❌ 未实现 | 同上 |
| 9 | webSearch | ai.api.js:15 | POST /api/web-search | 无（legacy-only: mod.rs:644） | ❌ 未实现 | 同上 |
| 10 | getInfiniteBenchmarks | ai.api.js:18 | GET /api/ai/infinite-optimize/benchmarks | 无（legacy-only: mod.rs:589） | ❌ 未实现 | 编排器无 `/api/ai/infinite-optimize/*` 路由；仅 legacy 后端有 |
| 11 | startInfiniteOptimize | ai.api.js:19 | POST /api/ai/infinite-optimize/start | 无（legacy-only: mod.rs:590） | ❌ 未实现 | 同上 |
| 12 | stopInfiniteOptimize | ai.api.js:20 | POST /api/ai/infinite-optimize/stop | 无（legacy-only: mod.rs:591） | ❌ 未实现 | 同上 |
| 13 | getInfiniteOptimizeStatus | ai.api.js:21 | GET /api/ai/infinite-optimize/status | 无（legacy-only: mod.rs:592） | ❌ 未实现 | 同上 |
| 14 | getInfiniteOptimizeResults | ai.api.js:22 | GET /api/ai/infinite-optimize/results | 无（legacy-only: mod.rs:593） | ❌ 未实现 | 同上 |
| 15 | runProviderComparison | ai.api.js:23 | POST /api/ai/infinite-optimize/compare | 无（legacy-only: mod.rs:594） | ❌ 未实现 | 同上 |
| 16 | getProviderComparison | ai.api.js:24 | GET /api/ai/infinite-optimize/comparison | 无（legacy-only: mod.rs:595） | ❌ 未实现 | 同上 |
| 17 | applyBestConfig | ai.api.js:25 | POST /api/ai/infinite-optimize/apply | 无（legacy-only: mod.rs:596） | ❌ 未实现 | 同上 |
| 18 | getArtifactConfig | ai.api.js:28 | GET /api/ai/artifact/config | 无（legacy-only: mod.rs:599） | ❌ 未实现 | 编排器无 `/api/ai/artifact/*` 路由 |
| 19 | getArtifacts | ai.api.js:29 | GET /api/ai/artifact/list | 无（legacy-only: mod.rs:600） | ❌ 未实现 | 同上 |
| 20 | createArtifact | ai.api.js:32 | POST /api/ai/artifact/create | 无（legacy-only: mod.rs:601） | ❌ 未实现 | 同上 |
| 21 | aiFullAnalysis | ai.api.js:35 | POST /api/ai/full-analysis | 无（legacy-only: mod.rs:574） | ❌ 未实现 | 编排器无此路由 |
| 22 | aiGenerateDoc | ai.api.js:36 | POST /api/ai/generate-doc | 无（legacy-only: mod.rs:575） | ❌ 未实现 | 同上 |
| 23 | aiGenerateFlowDiagram | ai.api.js:37 | POST /api/ai/generate-flow-diagram | 无（legacy-only: mod.rs:576） | ❌ 未实现 | 同上 |
| 24 | aiDevTestFix | ai.api.js:38 | POST /api/ai/dev-test-fix | 无（legacy-only: mod.rs:577） | ❌ 未实现 | 同上 |
| 25 | aiFullComplete | ai.api.js:39 | POST /api/ai/full-complete | 无（legacy-only: mod.rs:578） | ❌ 未实现 | 同上 |
| 26 | aiOptimizeDoc | ai.api.js:40 | POST /api/ai/optimize-doc | 无（legacy-only: mod.rs:579） | ❌ 未实现 | 同上 |
| 27 | aiProjectFromChat | ai.api.js:44 | POST /api/ai/project-from-chat | 无（legacy-only: mod.rs:580） | ❌ 未实现 | 编排器无此路由 |
| 28 | aiGenerateProjectGraph | ai.api.js:46 | POST /api/ai/project-graph | 无（legacy-only: mod.rs:581） | ❌ 未实现 | 同上 |
| 29 | aiLinkReqToDb | ai.api.js:48 | POST /api/ai/req-db-link | 无（legacy-only: mod.rs:582） | ❌ 未实现 | 同上 |
| 30 | allianceEnterprisePipeline | ai.api.js:50 | POST /api/ai/alliance-pipeline | 无（legacy-only: mod.rs:583） | ❌ 未实现 | 注意：网关有 `/api/alliance/*` 原生路由（alliance.rs），但路径不同（`/api/alliance/tasks` 等），非 `/api/ai/alliance-pipeline` |
| 31 | aiPublishArtifactsToKb | ai.api.js:52 | POST /api/ai/publish-kb | 无（legacy-only: mod.rs:584） | ❌ 未实现 | 编排器无此路由；kb 路由由 `mox_kb_svc::handlers::build_kb_router()` 承载，但无 `/api/ai/publish-kb` |
| 32 | aiGenerateErd | ai.api.js:54 | POST /api/ai/generate-erd | 无（legacy-only: mod.rs:585） | ❌ 未实现 | 同上 |
| 33 | aiExpertChat | ai.api.js:57 | POST /api/ai/expert-chat | 无（legacy-only: mod.rs:569） | ❌ 未实现 | 编排器无此路由；网关 alliance 域有 `/api/alliance/*` 但路径不匹配 |
| 34 | getEngineFlowGraph | ai.api.js:60 | GET /api/ai/engine/flow-graph | 无 | ❌ 未实现 | **双重问题**：(1) 网关 kg_ai 路由（http_adapter.rs:770-773）和编排器 nest（main.rs:566）均注册在 `/ai/engine/*`（无 `/api` 前缀），前端 `/api/ai/engine/flow-graph` 前缀不匹配；(2) 即使前缀匹配，两端均无 `/flow-graph` 子路由（仅有 process/analyze/capabilities/metrics/workflow/execute/workflow/templates/alliance/*） |
| 35 | aiRecommendOperators | ai.api.js:63 | POST /api/operators/ai-recommend | 无（legacy-only: mod.rs:539） | ❌ 未实现 | 编排器有 `/api/operators`（GET list，main.rs:469）和 `/api/operators/register`（POST，main.rs:470），但无 `/api/operators/ai-recommend` |
| 36 | aiResourceAnalysis | ai.api.js:64 | POST /api/resources/ai-analysis | 无（legacy-only: mod.rs:785） | ❌ 未实现 | 编排器有 `/api/ai/resources`（GET，main.rs:502）和 `/api/ai/resources/health`（GET，main.rs:503），但无 `/api/resources/ai-analysis`（路径前缀不同：`/api/resources` vs `/api/ai/resources`） |
| 37 | aiGenerateWorkflow | ai.api.js:65 | POST /api/workflow/ai-generate | 无（legacy-only: mod.rs:786） | ❌ 未实现 | 编排器有 `/api/ai/workflows/*`（main.rs:510-514），但无 `/api/workflow/ai-generate`（路径前缀不同） |
| 38 | aiMarketSearch | ai.api.js:66 | POST /api/market/ai-search | 无（legacy-only: mod.rs:655） | ❌ 未实现 | 编排器有 `.nest("/api/market", market::market_routes())`（main.rs:546），包含 CRUD/上传/导出等，但无 `/api/market/ai-search` |
| 39 | aiMcpMap | ai.api.js:67 | POST /api/mcp/ai-map | 无（legacy-only: mod.rs:665） | ❌ 未实现 | 编排器有 `/api/mcp`（POST handle_mcp_rpc，main.rs:581），但无 `/api/mcp/ai-map` |
| 40 | aiCaomeiParse | ai.api.js:68 | POST /api/caomei/ai-parse | 无（legacy-only: mod.rs:661） | ❌ 未实现 | 编排器有 `/api/caomei/compile`（POST，main.rs:495）、`/api/caomei/refine`（POST，main.rs:496）、`/api/caomei/templates`（GET，main.rs:497），但无 `/api/caomei/ai-parse` |
| 41 | aiAlgoLabAnalyze | ai.api.js:69 | POST /api/algolab/ai-analyze | 无（legacy-only: mod.rs:791） | ❌ 未实现 | 编排器无 `/api/algolab/*` 路由 |
| 42 | aiFusionGovern | ai.api.js:70 | POST /api/fusion/ai-govern | 无（legacy-only: mod.rs:792） | ❌ 未实现 | 编排器子服务有 `.nest("/fusion", ...)`（subservers.rs:31，无 `/api` 前缀），前端 `/api/fusion/ai-govern` 前缀不匹配且无 `ai-govern` 子路由 |
| 43 | aiMonitorDiagnose | ai.api.js:71 | POST /api/monitor/ai-diagnose | 无（legacy-only: mod.rs:789） | ❌ 未实现 | 网关 monitor 原生路由（monitor.rs:438）有 `/api/monitor/metrics/detail` 等，但无 `/api/monitor/ai-diagnose` |
| 44 | aiDocsExplain | ai.api.js:72 | POST /api/docs/ai-explain | 无（legacy-only: mod.rs:790） | ❌ 未实现 | 编排器无 `/api/docs/*` 路由（`/api/docs` 为 Swagger UI，main.rs:585，GET 仅） |
| 45 | aiPluginRoute | ai.api.js:73 | POST /api/plugins/ai-route | 无（legacy-only: mod.rs:787） | ❌ 未实现 | 编排器有 `/api/plugins`（GET list_plugins，main.rs:586）和 `/api/ai/plugins/*`（main.rs:505-508），但无 `/api/plugins/ai-route` |
| 46 | aiBrowserInstruct | ai.api.js:74 | POST /api/browser/ai-instruct | 无（legacy-only: mod.rs:788） | ❌ 未实现 | 编排器有 `/api/ai/browser/*`（main.rs:520-529），但无 `/api/browser/ai-instruct`（路径前缀不同：`/api/browser` vs `/api/ai/browser`） |
| 47 | aiAutomationExecute | ai.api.js:75 | POST /api/automation/ai-execute | 无（legacy-only: mod.rs:674） | ❌ 未实现 | 编排器有 `.nest("/api/automation", automation::router())`（main.rs:556），包含 `/`（GET/POST chat）、`/:id`（PUT）、`/:id/refine`、`/:id/run`、`/:id/permissions`，但无 `/api/automation/ai-execute` |
| 48 | getWorkbenchAiOverview | ai.api.js:76 | GET /api/workbench/ai-overview | 无（legacy-only: mod.rs:784） | ❌ 未实现 | 编排器无 `/api/workbench/*` 路由 |

**ai.api.js 小计**：已查证 49 条 → ✅ 已实现 5 条（10.2%），❌ 未实现 44 条（89.8%），⚠️ 不一致 0 条。

---

## 二、llm.api.js 核验明细

> 文件：`frontend-ui/src/api/llm.api.js`  
> 非 deprecated 导出函数：**20 个**（`getLlmPresets` 为 `getLlmProviderPresets` 别名，跳过）

| # | 前端函数 | 前端调用(file:line) | 方法+路径 | 后端实现(file:line) | 判定 | 证据/差异说明 |
|---|---------|---------------------|----------|---------------------|------|--------------|
| 1 | getLlmProviders | llm.api.js:4 | GET /api/llm/providers | 无（legacy-only: mod.rs:682） | ❌ 未实现 | 编排器无 `/api/llm/*` 路由；仅 legacy 后端有完整 LLM Provider CRUD |
| 2 | getLlmProviderPresets | llm.api.js:5 | GET /api/llm/providers/presets | 无（legacy-only: mod.rs:683） | ❌ 未实现 | 同上 |
| 3 | getLlmProvider | llm.api.js:8 | GET /api/llm/providers/:id | 无（legacy-only: mod.rs:684） | ❌ 未实现 | 同上 |
| 4 | setActiveProvider | llm.api.js:9 | POST /api/llm/providers/active | 无（legacy-only: mod.rs:685） | ❌ 未实现 | 同上 |
| 5 | addLlmProvider | llm.api.js:10 | POST /api/llm/providers | 无（legacy-only: mod.rs:686） | ❌ 未实现 | 同上 |
| 6 | updateLlmProvider | llm.api.js:11 | PUT /api/llm/providers/:id | 无（legacy-only: mod.rs:687） | ❌ 未实现 | 同上 |
| 7 | removeLlmProvider | llm.api.js:12 | DELETE /api/llm/providers/:id | 无（legacy-only: mod.rs:688） | ❌ 未实现 | 同上 |
| 8 | enableLlmProvider | llm.api.js:13 | POST /api/llm/providers/:id/enable | 无（legacy-only: mod.rs:689） | ❌ 未实现 | 同上 |
| 9 | disableLlmProvider | llm.api.js:14 | POST /api/llm/providers/:id/disable | 无（legacy-only: mod.rs:690） | ❌ 未实现 | 同上 |
| 10 | testLlmProvider | llm.api.js:15 | POST /api/llm/providers/:id/test | 无（legacy-only: mod.rs:691） | ❌ 未实现 | 同上 |
| 11 | discoverLlmModels | llm.api.js:16 | POST /api/llm/providers/:id/discover | 无（legacy-only: mod.rs:692） | ❌ 未实现 | 同上 |
| 12 | getLlmHealth | llm.api.js:17 | GET /api/llm/health | 无（legacy-only: mod.rs:693） | ❌ 未实现 | 编排器无 `/api/llm/health` 路由 |
| 13 | getLlmRouting | llm.api.js:18 | GET /api/llm/routing | 无（legacy-only: mod.rs:694） | ❌ 未实现 | 同上 |
| 14 | updateLlmRouting | llm.api.js:19 | PUT /api/llm/routing | 无（legacy-only: mod.rs:695） | ❌ 未实现 | 同上 |
| 15 | getLlmUsage | llm.api.js:20 | GET /api/llm/usage | 无（legacy-only: mod.rs:696） | ❌ 未实现 | 同上 |
| 16 | getLlmLogs | llm.api.js:21 | GET /api/llm/logs | 无（legacy-only: mod.rs:697） | ❌ 未实现 | 同上；前端传 `?limit=50` 查询参数 |
| 17 | getLlmStats | llm.api.js:22 | GET /api/llm/stats | 无（legacy-only: mod.rs:698） | ❌ 未实现 | 同上 |
| 18 | getLlmConfig | llm.api.js:25 | GET /api/ai/llm/config | 编排器 main.rs:516（路由），handler main.rs:2536 | ✅ 已实现（代理→编排器） | 方法一致；handler 返回 `{api_base, model, temperature, max_tokens, enabled, has_api_key}`；响应信封一致 |
| 19 | updateLlmConfig | llm.api.js:26 | POST /api/ai/llm/config | 编排器 main.rs:517（路由），handler main.rs:2550 | ✅ 已实现（代理→编排器） | 方法一致；handler 接受 `LLMConfigRequest`（api_base/api_key/model/temperature/max_tokens/enabled），更新 `state.ai_agent.llm_client()` 配置 |
| 20 | testLlm | llm.api.js:27 | POST /api/ai/llm/test | 编排器 main.rs:518（路由），handler main.rs:2585 | ✅ 已实现（代理→编排器） | 方法一致；handler 调用 `state.ai_agent.test_llm_connection()` 真实连通性测试 |

**llm.api.js 小计**：已查证 20 条 → ✅ 已实现 3 条（15.0%），❌ 未实现 17 条（85.0%），⚠️ 不一致 0 条。

---

## 三、graph.api.js 核验明细

> 文件：`frontend-ui/src/api/graph.api.js`  
> 非 deprecated 导出函数：**18 个直接 API + 1 个复合函数**（`listDialogueSessions` 为 `getDialogueSessions` 别名，跳过）

| # | 前端函数 | 前端调用(file:line) | 方法+路径 | 后端实现(file:line) | 判定 | 证据/差异说明 |
|---|---------|---------------------|----------|---------------------|------|--------------|
| 1 | getGraph | graph.api.js:4 | GET /api/graph | 编排器 main.rs:473（路由），handler main.rs:1760 | ✅ 已实现（代理→编排器） | 方法一致；handler 返回 `GraphData{nodes, edges}`；响应信封一致 |
| 2 | getGraphStats | graph.api.js:5 | GET /api/graph/stats | 编排器 main.rs:474（路由），handler main.rs:1828 | ✅ 已实现（代理→编排器） | 方法一致；handler 返回 `GraphStats` |
| 3 | getCentrality | graph.api.js:6 | GET /api/graph/centrality | 编排器 main.rs:478（路由），handler `get_centrality` | ✅ 已实现（代理→编排器） | 方法一致；路由精确匹配 |
| 4 | getCommunities | graph.api.js:7 | GET /api/graph/communities | 编排器 main.rs:479（路由），handler `get_communities` | ✅ 已实现（代理→编排器） | 方法一致 |
| 5 | getPagerank | graph.api.js:8 | GET /api/graph/pagerank | 编排器 main.rs:481（路由），handler `get_pagerank` | ✅ 已实现（代理→编排器） | 方法一致 |
| 6 | getNeighbors | graph.api.js:9 | GET /api/graph/neighbors/:id | 编排器 main.rs:477（路由），handler `get_neighbors` | ✅ 已实现（代理→编排器） | 路径参数 `:id` 匹配；前端 `encodeURIComponent(id)` 兼容 |
| 7 | getShortestPath | graph.api.js:10-11 | GET /api/graph/path?source=&target= | 编排器 main.rs:480（路由），handler `get_shortest_path` | ✅ 已实现（代理→编排器） | 查询参数 `source, target` 匹配 |
| 8 | recommendNodes | graph.api.js:12 | POST /api/graph/recommend | 编排器 main.rs:483（路由），handler `recommend_nodes` | ✅ 已实现（代理→编排器） | 方法一致 |
| 9 | addGraphNode | graph.api.js:13 | POST /api/graph/node | 编排器 main.rs:475（路由），handler `add_node` | ✅ 已实现（代理→编排器） | 方法一致 |
| 10 | addGraphEdge | graph.api.js:14 | POST /api/graph/edge | 编排器 main.rs:476（路由），handler `add_edge` | ✅ 已实现（代理→编排器） | 方法一致 |
| 11 | propagateActivation | graph.api.js:16-17 | POST /api/graph/activate | 编排器 main.rs:482（路由），handler `propagate_activation` | ✅ 已实现（代理→编排器） | 请求体 `{start_nodes, iterations}` 与前端一致（iterations 默认 10） |
| 12 | graphSearch | graph.api.js:21-22 | GET /api/graph/search?q=&limit= | 编排器 main.rs:485（路由），handler main.rs:1920 | ✅ 已实现（代理→编排器） | 查询参数 `q, limit`（默认 20）匹配 |
| 13 | toggleAutoSync | graph.api.js:24-25 | POST /api/graph/auto-sync/toggle | 编排器 main.rs:486（路由），handler main.rs:1941 | ✅ 已实现（代理→编排器） | 请求体 `{enabled}` 匹配 |
| 14 | getAutoSyncStatus | graph.api.js:27 | GET /api/graph/auto-sync/status | 编排器 main.rs:487（路由），handler main.rs:1953 | ✅ 已实现（代理→编排器） | 方法一致 |
| 15 | getDialogueSessions | graph.api.js:29 | GET /api/dialogue/sessions | 编排器 main.rs:488（路由），handler main.rs:1960 | ✅ 已实现（代理→编排器） | 方法一致；注意路径为 `/api/dialogue/sessions`（非 `/api/graph/dialogue`），精确匹配 |
| 16 | graphExport | graph.api.js:33 | GET /api/graph/export | 编排器 main.rs:489（路由），handler main.rs:1970 | ✅ 已实现（代理→编排器） | 方法一致；返回 JSON 迁移包 |
| 17 | graphImport | graph.api.js:35 | POST /api/graph/import | 编排器 main.rs:490（路由），handler main.rs:1978 | ✅ 已实现（代理→编排器） | 方法一致；幂等合并 |
| 18 | aiGraphInsights | graph.api.js:38 | POST /api/graph/ai-insights | 无（legacy-only: mod.rs:559） | ❌ 未实现 | 编排器无 `/api/graph/ai-insights` 路由；仅 legacy 后端有 |
| 19 | getAggregatedGraph | graph.api.js:42-267 | 复合函数（见下方说明） | 部分实现 | ⚠️ 不一致（复合） | 见下方复合函数拆解 |

### getAggregatedGraph 复合函数拆解（graph.api.js:42-267）

该函数不直接导出单一 HTTP 调用，而是并行调用 8 个端点后在前端聚合：

| 子调用 | 方法+路径 | 后端实现 | 判定 |
|--------|----------|---------|------|
| getGraph() | GET /api/graph | 编排器 main.rs:473 | ✅ |
| http.get('/projects') | GET /api/projects | 网关 misc.rs:371（`list_projects_paginated`，原生）；同时代理 `/api/projects/*` → PrimiFlow，存在路由冲突（axum 具体度优先，网关原生 `/api/projects` 优先于代理 nest） | ⚠️ 冲突 |
| http.get('/experts') | GET /api/experts | 无：网关 experts_ext.rs 仅有 `/api/experts/{stats,bookings/*,:id/*,team}` 子路由，无根列表；编排器亦无；代理转发后 404（但 `silent:true` + `allSettled` 降级为空数组） | ❌ |
| http.get('/operators') | GET /api/operators | 编排器 main.rs:469（`list_operators`） | ✅ |
| http.get('/tasks') | GET /api/tasks | 网关 misc.rs:370（`list_tasks_paginated`，原生） | ✅ |
| http.get('/kb/documents') | GET /api/kb/documents | 网关 kb 路由（`mox_kb_svc::handlers::build_kb_router()`，lib.rs:136） | ✅（推断：kb router 承载文档列表） |
| http.get('/ai/workflows') | GET /api/ai/workflows | 编排器 main.rs:511（`list_workflows`） | ✅ |
| http.get('/automation') | GET /api/automation | 编排器 automation.rs:921（`list_handler`，nest 于 `/api/automation`） | ✅ |

**graph.api.js 小计**：直接 API 18 条 → ✅ 已实现 17 条（94.4%），❌ 未实现 1 条（5.6%）；复合函数 1 个 → 7/8 子调用可解析，1 个缺失（`/api/experts`），1 个路由冲突（`/api/projects`）。

---

## 四、本片统计汇总

### 总量统计

| 前端文件 | 核验接口数 | ✅ 已实现 | ❌ 未实现 | ⚠️ 不一致 | 实现率 |
|----------|-----------|----------|----------|----------|--------|
| ai.api.js | 49 | 5 | 44 | 0 | 10.2% |
| llm.api.js | 20 | 3 | 17 | 0 | 15.0% |
| graph.api.js（直接API） | 18 | 17 | 1 | 0 | 94.4% |
| **合计** | **87** | **25** | **62** | **0** | **28.7%** |

> 另有 graph.api.js 复合函数 `getAggregatedGraph` 1 个（含 8 个子调用，其中 1 个缺失、1 个路由冲突），未计入直接 API 统计。

### 按实现路径分类

| 实现路径 | 数量 | 占比 |
|---------|------|------|
| ✅ 已实现（代理→编排器） | 25 | 28.7% |
| ✅ 已实现（网关原生） | 0 | 0% |
| ❌ 未实现（仅 legacy 后端） | 61 | 70.1% |
| ❌ 未实现（全后端无，含路径前缀错位） | 1 | 1.1% |
| ⚠️ 不一致 | 0 | 0% |

### 问题清单（不一致/缺失明细）

#### A. 高优先级：大面积缺失（62 个接口仅存在于 legacy 后端）

以下接口在当前架构（网关→编排器）中无对应 handler，前端调用将 404。所有这些接口仅在 `platform/legacy/backend-rust/src/api/mod.rs` 中有定义，但 legacy 后端不在当前代理路径中。

**A1. ai.api.js — mox 模块化系统架构智能分析引擎（6 个）**
- `aiFullAnalysis` (POST /api/ai/full-analysis) — legacy mod.rs:574
- `aiGenerateDoc` (POST /api/ai/generate-doc) — legacy mod.rs:575
- `aiGenerateFlowDiagram` (POST /api/ai/generate-flow-diagram) — legacy mod.rs:576
- `aiDevTestFix` (POST /api/ai/dev-test-fix) — legacy mod.rs:577
- `aiFullComplete` (POST /api/ai/full-complete) — legacy mod.rs:578
- `aiOptimizeDoc` (POST /api/ai/optimize-doc) — legacy mod.rs:579

**A2. ai.api.js — 项目需求一体化（6 个）**
- `aiProjectFromChat` (POST /api/ai/project-from-chat) — legacy mod.rs:580
- `aiGenerateProjectGraph` (POST /api/ai/project-graph) — legacy mod.rs:581
- `aiLinkReqToDb` (POST /api/ai/req-db-link) — legacy mod.rs:582
- `allianceEnterprisePipeline` (POST /api/ai/alliance-pipeline) — legacy mod.rs:583
- `aiPublishArtifactsToKb` (POST /api/ai/publish-kb) — legacy mod.rs:584
- `aiGenerateErd` (POST /api/ai/generate-erd) — legacy mod.rs:585

**A3. ai.api.js — 无穷维度优化引擎（9 个）**
- `getInfiniteBenchmarks` / `startInfiniteOptimize` / `stopInfiniteOptimize` / `getInfiniteOptimizeStatus` / `getInfiniteOptimizeResults` / `runProviderComparison` / `getProviderComparison` / `applyBestConfig` — legacy mod.rs:589-596

**A4. ai.api.js — 本地制品引擎（3 个）**
- `getArtifactConfig` / `getArtifacts` / `createArtifact` — legacy mod.rs:599-601

**A5. ai.api.js — 联网搜索（4 个）**
- `getWebSearchConfig` / `updateWebSearchConfig` / `testWebSearch` / `webSearch` — legacy mod.rs:641-644

**A6. ai.api.js — 16 模块 AI 增强端点（14 个）**
- `aiRecommendOperators` (POST /api/operators/ai-recommend) — legacy mod.rs:539
- `aiResourceAnalysis` (POST /api/resources/ai-analysis) — legacy mod.rs:785
- `aiGenerateWorkflow` (POST /api/workflow/ai-generate) — legacy mod.rs:786
- `aiMarketSearch` (POST /api/market/ai-search) — legacy mod.rs:655
- `aiMcpMap` (POST /api/mcp/ai-map) — legacy mod.rs:665
- `aiCaomeiParse` (POST /api/caomei/ai-parse) — legacy mod.rs:661
- `aiAlgoLabAnalyze` (POST /api/algolab/ai-analyze) — legacy mod.rs:791
- `aiFusionGovern` (POST /api/fusion/ai-govern) — legacy mod.rs:792
- `aiMonitorDiagnose` (POST /api/monitor/ai-diagnose) — legacy mod.rs:789
- `aiDocsExplain` (POST /api/docs/ai-explain) — legacy mod.rs:790
- `aiPluginRoute` (POST /api/plugins/ai-route) — legacy mod.rs:787
- `aiBrowserInstruct` (POST /api/browser/ai-instruct) — legacy mod.rs:788
- `aiAutomationExecute` (POST /api/automation/ai-execute) — legacy mod.rs:674
- `getWorkbenchAiOverview` (GET /api/workbench/ai-overview) — legacy mod.rs:784

**A7. ai.api.js — 其他（2 个）**
- `aiExpertChat` (POST /api/ai/expert-chat) — legacy mod.rs:569
- `aiGraphInsights` 属 graph.api.js — legacy mod.rs:559

**A8. llm.api.js — LLM 网关全量缺失（17 个）**
- Provider CRUD 12 个：`getLlmProviders` / `getLlmProviderPresets` / `getLlmProvider` / `setActiveProvider` / `addLlmProvider` / `updateLlmProvider` / `removeLlmProvider` / `enableLlmProvider` / `disableLlmProvider` / `testLlmProvider` / `discoverLlmModels` — legacy mod.rs:682-692
- 运维 5 个：`getLlmHealth` / `getLlmRouting` / `updateLlmRouting` / `getLlmUsage` / `getLlmLogs` / `getLlmStats` — legacy mod.rs:693-698

#### B. 路径前缀错位（架构级问题）

**B1. `/api/ai/engine/*` 全路径不可达**
- 前端 `getEngineFlowGraph` 调用 `GET /api/ai/engine/flow-graph`
- 网关 kg_ai 路由（http_adapter.rs:770-773）注册在 `/ai/engine/{process,analyze,capabilities,metrics}`（**无 `/api` 前缀**）
- 编排器（main.rs:566）`.nest("/ai/engine", ...)` 同样**无 `/api` 前缀**
- 前端 `baseURL='/api'`，请求为 `/api/ai/engine/*`，经代理转发到编排器后前缀不匹配，**必然 404**
- 影响：不仅 `getEngineFlowGraph` 缺失，网关 kg_ai 的 4 个标准引擎端点（process/analyze/capabilities/metrics）和编排器 ai_engine 路由的 9 个子端点对前端均不可达
- **建议**：将网关 kg_ai 路由和编排器 `/ai/engine` nest 统一迁移到 `/api/ai/engine/*` 前缀，或在代理层做路径重写

#### C. 复合函数子调用问题（graph.api.js getAggregatedGraph）

**C1. `GET /api/experts` 缺失**
- 网关 experts_ext.rs 仅有子路由（`/api/experts/stats`、`/api/experts/bookings/*`、`/api/experts/:id/*`、`/api/experts/team`），无根列表 `/api/experts`
- 编排器亦无此路由
- 前端用 `silent:true` + `allSettled` 降级为空数组，不阻断整体，但专家节点永远为空

**C2. `GET /api/projects` 路由冲突**
- 网关 misc.rs:371 原生注册 `/api/projects`（GET `list_projects_paginated`）
- 网关代理 proxy.rs:86-89 同时将 `/api/projects` nest → PrimiFlow（:8000）
- axum 按具体度匹配：直接 `.route("/api/projects", ...)` 优先于 `.nest("/api/projects", fallback)`，实际由网关 misc 处理
- 但若 PrimiFlow 有更具体的 `/api/projects/:id` 等路由，可能产生不一致行为
- 建议：明确 `/api/projects` 列表端点的归属（网关 misc vs PrimiFlow），避免双源

---

## 五、已查证 vs 推断标注

| 类别 | 数量 | 说明 |
|------|------|------|
| 已查证（有 file:line 证据） | 87 | 所有 87 个直接 API 均有明确的路由注册 file:line 或明确的全后端搜索无匹配证据 |
| 推断（无直接证据） | 1 | `GET /api/kb/documents`（getAggregatedGraph 子调用）：推断由 `mox_kb_svc::handlers::build_kb_router()` 承载，未深入 kb router 内部逐行验证 |

---

## 六、四维比对总结

| 维度 | 已实现接口（25个） | 未实现接口（62个） |
|------|-------------------|-------------------|
| **方法** | 全部一致（GET/POST/PUT/DELETE 与路由注册匹配） | 不适用（无路由） |
| **路径** | 全部精确匹配（含路径参数 `:id`/`:session`） | 61 个仅 legacy 有；1 个（getEngineFlowGraph）路径前缀错位 + 子路由缺失 |
| **请求参数/体** | 已抽查关键 handler：ai_chat、analyze_algorithm、graph_import、propagate_activation 等字段匹配；LLM config 的 `LLMConfigRequest` 字段与前端 payload 兼容 | 不适用 |
| **响应信封** | 编排器统一使用 `mox_api_protocol::ApiResponse{code,msg,data}`，`api_ok()` 返回 `code=0`；前端拦截器 `code===0` 解包 data，完全一致 | legacy 后端使用 `{success,data}` 旧格式，但不在当前路径中 |

---

*报告生成时间：2026-09-03 | 核验工具：源码静态检索（Grep/Read）| 仅只读操作，未修改任何源码*

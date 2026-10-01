# 前端 api 字面路径 → 网关路由 幻影普查读数（2026-09-27）— 供 FRONTEND-MODULE-GOVERNANCE-v1.0.md §5.18 引用

普查对象＝`frontend-ui/src/api/*.js` 里所有**无插值**的字面路径（含插值的 141 条不测：插值路径打过去拿到的 404 是"资源没有"，与"路由没有"混成一类）。
判据：一律 GET（源里写 `http.post` 的路径若真存在，用 POST 打就是在写状态）；`404 且响应体为空`＝网关及其兜底反代之后没有任何路由；`405`＝路径在、方法不对（正常）；`502`＝路径在、上游不可达。

## A. 仪器通电证明（控制组，本脚本现场跑的 curl）

```
$ curl -s -H "Authorization: Bearer <JWT>" http://127.0.0.1:3080/api/tasks?page_size=1
HTTP 200  响应体前 90 字符='{"code":0,"msg":"ok","data":{"filters":{"keyword":null,"sort_by":"created_at","sort_order"'   # 已知真路由，必须 200

$ curl -s -H "Authorization: Bearer <JWT>" http://127.0.0.1:3080/api/kb/documents
HTTP 200  响应体前 90 字符='{"code":0,"msg":"ok","data":{"items":[{"category":"cat-dialogue","id":"kb-58f91170","statu'   # 已知真路由（KB 域），必须 200

$ curl -s -H "Authorization: Bearer <JWT>" http://127.0.0.1:3080/api/zzz-not-a-route-xyz
HTTP 404  响应体前 90 字符=''   # 已知不存在，必须 404 且空体

$ curl -s -H "Authorization: Bearer <JWT>" http://127.0.0.1:3080/melody/v1/health
HTTP 502  响应体前 90 字符='{"code":502,"msg":"上游 melody2score 不可达（http://127.0.0.1:8012/api/samples）：error sending re'   # 网关 Melody 域真实前缀，带令牌应为上游判决而非 404

$ curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8000/api/health   # PrimiFlow 上游
HTTP 000（000＝连接被拒 ⇒ 本轮任何 `404 空体` 都不可能来自 :8000 的反代分支，那类路径只会报 502）
$ curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8012/api/health   # melody2score 上游
HTTP 000（000＝未启动）
```

## B. 首轮普查（202 条静态路径连续打，触发网关限流 ⇒ 尾部 64 条判为 429）

原件＝`%TEMP%\mox-endpoint-census\census3.out`（17805 字节，驱动 `census.py`，仓库外＝没落库⇒没牙、别接 CI）。这里抄桶名与条数：

```
预检 GET /api/tasks?page_size=1 -> 200（必须 200，否则本普查全体 401＝仪器没通电）
静态字面路径 202 条；含插值的模板路径 141 条
=== 404-empty(幻影)：138 条
=== 429：64 条
=== 含插值（未探测，只按首段归类）：
```

## C. 限流复测（64 条，逐条间隔 1.2 s ⇒ 全部出判决，0 条仍被限流）

原件＝`%TEMP%\mox-endpoint-census\reprobe.out`（13950 字节）。这一节是最终台账的来源。

```
census3：幻影 138 条 / 限流未判 64 条 / 全部静态路径 202 条

=== 复测限流桶（一律 GET，1.2 s 间隔）
  /market/random                             operators.api.js:11  200
  /market/upload                             operators.api.js:13  405
  /mcp/ai-map                                ai.api.js:73         404
  /melody2score/export-sheet                 melody.api.js:16     404
  /melody2score/health                       melody.api.js:4      404
  /melody2score/recognize                    melody.api.js:7      404
  /melody2score/recognize-record             melody.api.js:15     404
  /melody2score/recognize-sample             melody.api.js:11     404
  /melody2score/save-report                  melody.api.js:17     404
  /melody2score/status                       melody.api.js:5      404
  /monitor/ai-diagnose                       ai.api.js:77         404
  /monitor/alert-rules                       monitor.api.js:28    200
  /monitor/alerts/summary                    monitor.api.js:14    200
  /monitor/business                          monitor.api.js:11    200
  /monitor/business/timeseries               monitor.api.js:39    200
  /monitor/metrics/detail                    monitor.api.js:5     200
  /monitor/nodes                             monitor.api.js:17    200
  /monitor/quality                           monitor.api.js:8     200
  /mox/health                                mox.api.js:6         200
  /mox/optimize                              mox.api.js:9         405
  /mox/publish                               mox.api.js:11        405
  /notifications/read-all                    notification.api.js:15 405
  /notifications/unread-count                notification.api.js:8 200
  /operators/ai-recommend                    ai.api.js:69         404
  /operators/register                        operators.api.js:6   405
  /plugins/ai-route                          ai.api.js:79         404
  /projects/ai-recommend                     projects.api.js:84   405
  /projects/catalog                          projects.api.js:10   502
  /projects/stats                            projects.api.js:11   502
  /projects/types                            projects.api.js:9    502
  /resources/ai-analysis                     ai.api.js:70         404
  /security/api-keys                         system.api.js:24     403
  /security/audit-log                        system.api.js:29     403
  /security/status                           system.api.js:23     200
  /security/validate                         system.api.js:27     405
  /status/full                               system.api.js:17     200
  /storage/providers                         system.api.js:31     200
  /storage/switch                            system.api.js:32     405
  /system/config                             system.api.js:109    200
  /system/config/refresh-cache               system.api.js:115    405
  /system/dept                               system.api.js:44     200
  /system/dept/tree                          system.api.js:45     200
  /system/dict/data                          system.api.js:101    200
  /system/dict/type                          system.api.js:93     200
  /system/dict/type/all                      system.api.js:94     200
  /system/logininfor                         system.api.js:125    200
  /system/logininfor/export                  system.api.js:128    200
  /system/menu                               system.api.js:86     200
  /system/menu/tree                          system.api.js:85     200
  /system/operlog                            system.api.js:118    200
  /system/operlog/clean                      system.api.js:121    405
  /system/operlog/export                     system.api.js:122    200
  /system/permissions                        system.api.js:40     200
  /system/post                               system.api.js:53     200
  /system/role                               system.api.js:72     200
  /tasks/auto                                projects.api.js:34   404
  /tasks/decompose                           workspace.api.js:46  405
  /tasks/from-chat                           projects.api.js:31   404
  /voice/health                              alliance.api.js:10   200
  /web-search/config                         ai.api.js:12         404
  /web-search/test                           ai.api.js:14         404
  /workbench/ai-overview                     ai.api.js:82         404
  /workflow/ai-generate                      ai.api.js:71         404
  /workspace/history                         workspace.api.js:43  200

复测分桶：200×30, 403×2, 404×18, 405×11, 502×3
仍在限流：0 条

=== 最终幻影台账：唯一路径 156 条 / 调用点 156 处
  ai.api.js              调用点 45 处 / 唯一路径 45 条
  alliance.api.js        调用点  7 处 / 唯一路径  7 条
  caomei.api.js          调用点  3 处 / 唯一路径  3 条
  experts.api.js         调用点 28 处 / 唯一路径 28 条
  graph.api.js           调用点 18 处 / 唯一路径 18 条
  http.test.js           调用点  1 处 / 唯一路径  1 条
  kb.api.js              调用点  7 处 / 唯一路径  7 条
  llm.api.js             调用点 10 处 / 唯一路径 10 条
  melody.api.js          调用点  8 处 / 唯一路径  8 条
  monitor.api.js         调用点  1 处 / 唯一路径  1 条
  projects.api.js        调用点  5 处 / 唯一路径  5 条
  system.api.js          调用点  3 处 / 唯一路径  3 条
  workflow.api.js        调用点 19 处 / 唯一路径 19 条
  workspace.api.js       调用点  1 处 / 唯一路径  1 条

唯一幻影路径清单：
  /api/ai/algorithm-types  <- ai.api.js:8(get)
  /api/ai/alliance-pipeline  <- ai.api.js:50(post)
  /api/ai/analyze-algorithm  <- ai.api.js:7(post)
  /api/ai/artifact/config  <- ai.api.js:28(get)
  /api/ai/artifact/create  <- ai.api.js:32(post)
  /api/ai/artifact/list  <- ai.api.js:29(get)
  /api/ai/browser/execute-action  <- workflow.api.js:52(post)
  /api/ai/browser/execute-steps  <- workflow.api.js:51(post)
  /api/ai/browser/execute-task  <- workflow.api.js:50(post)
  /api/ai/browser/natural  <- workflow.api.js:53(post)
  /api/ai/browser/sessions  <- workflow.api.js:46(get)
  /api/ai/browser/templates  <- workflow.api.js:45(get)
  /api/ai/chat  <- ai.api.js:5(post)
  /api/ai/dev-test-fix  <- ai.api.js:38(post)
  /api/ai/engine/flow-graph  <- ai.api.js:66(get)
  /api/ai/flows  <- workflow.api.js:12(get)
  /api/ai/flows/execute  <- workflow.api.js:18(post)
  /api/ai/flows/node-types  <- workflow.api.js:19(get)
  /api/ai/flows/validate  <- workflow.api.js:17(post)
  /api/ai/full-analysis  <- ai.api.js:35(post)
  /api/ai/full-complete  <- ai.api.js:39(post)
  /api/ai/generate-doc  <- ai.api.js:36(post)
  /api/ai/generate-erd  <- ai.api.js:54(post)
  /api/ai/generate-flow-diagram  <- ai.api.js:37(post)
  /api/ai/infinite-optimize/apply  <- ai.api.js:25(post)
  /api/ai/infinite-optimize/benchmarks  <- ai.api.js:18(get)
  /api/ai/infinite-optimize/compare  <- ai.api.js:23(post)
  /api/ai/infinite-optimize/comparison  <- ai.api.js:24(get)
  /api/ai/infinite-optimize/results  <- ai.api.js:22(get)
  /api/ai/infinite-optimize/start  <- ai.api.js:19(post)
  /api/ai/infinite-optimize/status  <- ai.api.js:21(get)
  /api/ai/infinite-optimize/stop  <- ai.api.js:20(post)
  /api/ai/llm/config  <- llm.api.js:25(get)
  /api/ai/llm/test  <- llm.api.js:27(post)
  /api/ai/optimize-doc  <- ai.api.js:40(post)
  /api/ai/plugins  <- workflow.api.js:22(get)
  /api/ai/plugins/register  <- workflow.api.js:23(post)
  /api/ai/plugins/send-message  <- workflow.api.js:24(post)
  /api/ai/plugins/topology  <- workflow.api.js:26(get)
  /api/ai/project-from-chat  <- ai.api.js:44(post)
  /api/ai/project-graph  <- ai.api.js:46(post)
  /api/ai/publish-kb  <- ai.api.js:52(post)
  /api/ai/req-db-link  <- ai.api.js:48(post)
  /api/ai/resources  <- projects.api.js:37(get)
  /api/ai/resources/health  <- projects.api.js:38(get)
  /api/ai/workflows  <- graph.api.js:90(get)
  /api/ai/workflows/execute  <- workflow.api.js:8(post)
  /api/ai/workflows/instances  <- workflow.api.js:9(get)
  /api/ai/workflows/save  <- workflow.api.js:7(post)
  /api/ai/workflows/templates  <- workflow.api.js:5(get)
  /api/algolab/ai-analyze  <- ai.api.js:75(post)
  /api/alliance/runtime  <- alliance.api.js:232(get)
  /api/alliance/sediment  <- ai.api.js:63(post)
  /api/alliance/tasks  <- alliance.api.js:130(post)
  /api/analyze/spiral  <- ai.api.js:9(post)
  /api/auth/me  <- http.test.js:9(get)
  /api/automation/ai-execute  <- ai.api.js:81(post)
  /api/automation/chat  <- workflow.api.js:38(post)
  /api/browser/ai-instruct  <- ai.api.js:80(post)
  /api/caomei/ai-parse  <- ai.api.js:74(post)
  /api/caomei/compile  <- caomei.api.js:4(post)
  /api/caomei/refine  <- caomei.api.js:5(post)
  /api/caomei/templates  <- caomei.api.js:6(get)
  /api/dialogue/sessions  <- graph.api.js:29(get)
  /api/docs/ai-explain  <- ai.api.js:78(post)
  /api/expert-graph/communities  <- experts.api.js:52(get)
  /api/expert-graph/optimal-team  <- experts.api.js:53(post)
  /api/expert-graph/rebuild  <- experts.api.js:54(post)
  /api/expert-graph/stats  <- experts.api.js:48(get)
  /api/experts/algorithm-analysis  <- alliance.api.js:120(post)
  /api/experts/bookings  <- experts.api.js:82(post)
  /api/experts/bookings/mine  <- experts.api.js:75(get)
  /api/experts/capabilities  <- experts.api.js:13(get)
  /api/experts/debate  <- alliance.api.js:48(post)
  /api/experts/dispatcher/config  <- experts.api.js:37(get)
  /api/experts/dispatcher/consult  <- experts.api.js:41(post)
  /api/experts/dispatcher/dispatch  <- experts.api.js:40(post)
  /api/experts/dispatcher/multi-consult  <- experts.api.js:42(post)
  /api/experts/dispatcher/reset-all  <- experts.api.js:44(post)
  /api/experts/dispatcher/status  <- experts.api.js:39(get)
  /api/experts/enterprise/analyze  <- experts.api.js:58(post)
  /api/experts/enterprise/consult  <- experts.api.js:57(post)
  /api/experts/intelligent-consult  <- alliance.api.js:116(post)
  /api/experts/metrics  <- experts.api.js:17(get)
  /api/experts/multi-consult  <- alliance.api.js:42(post)
  /api/experts/orchestrate  <- experts.api.js:61(post)
  /api/experts/orchestration/history  <- experts.api.js:68(get)
  /api/experts/orchestration/plugins  <- experts.api.js:65(get)
  /api/experts/orchestration/stats  <- experts.api.js:64(get)
  /api/experts/overview  <- experts.api.js:18(get)
  /api/experts/plan/execute  <- experts.api.js:63(post)
  /api/experts/plan/generate  <- experts.api.js:62(post)
  /api/experts/route  <- alliance.api.js:112(post)
  /api/experts/semantic-search  <- experts.api.js:32(post)
  /api/experts/sessions  <- experts.api.js:22(post)
  /api/experts/sessions/stats  <- experts.api.js:26(get)
  /api/experts/stats  <- experts.api.js:72(get)
  /api/experts/team  <- experts.api.js:93(post)
  /api/fusion/ai-govern  <- ai.api.js:76(post)
  /api/graph/activate  <- graph.api.js:17(post)
  /api/graph/ai-insights  <- graph.api.js:38(post)
  /api/graph/auto-sync/status  <- graph.api.js:27(get)
  /api/graph/auto-sync/toggle  <- graph.api.js:25(post)
  /api/graph/centrality  <- graph.api.js:6(get)
  /api/graph/communities  <- graph.api.js:7(get)
  /api/graph/edge  <- graph.api.js:14(post)
  /api/graph/export  <- graph.api.js:33(get)
  /api/graph/import  <- graph.api.js:35(post)
  /api/graph/node  <- graph.api.js:13(post)
  /api/graph/pagerank  <- graph.api.js:8(get)
  /api/graph/path  <- graph.api.js:11(get)
  /api/graph/recommend  <- graph.api.js:12(post)
  /api/graph/search  <- graph.api.js:22(get)
  /api/graph/stats  <- graph.api.js:5(get)
  /api/kb/batch-analyze  <- kb.api.js:10(post)
  /api/kb/categories  <- kb.api.js:11(get)
  /api/kb/documents  <- graph.api.js:89(get)
  /api/kb/entities/search  <- kb.api.js:20(get)
  /api/kb/history  <- kb.api.js:30(get)
  /api/kb/search  <- kb.api.js:13(post)
  /api/kb/stats  <- kb.api.js:28(get)
  /api/kb/tags  <- kb.api.js:12(get)
  /api/llm/health  <- llm.api.js:17(get)
  /api/llm/logs  <- llm.api.js:21(get)
  /api/llm/providers  <- llm.api.js:4(get)
  /api/llm/providers/active  <- llm.api.js:9(post)
  /api/llm/providers/presets  <- llm.api.js:5(get)
  /api/llm/routing  <- llm.api.js:18(get)
  /api/llm/stats  <- llm.api.js:22(get)
  /api/llm/usage  <- llm.api.js:20(get)
  /api/market/ai-search  <- ai.api.js:72(post)
  /api/mcp/ai-map  <- ai.api.js:73(post)
  /api/melody2score/export-sheet  <- melody.api.js:16(post)
  /api/melody2score/health  <- melody.api.js:4(get)
  /api/melody2score/recognize  <- melody.api.js:7(post)
  /api/melody2score/recognize-record  <- melody.api.js:15(post)
  /api/melody2score/recognize-sample  <- melody.api.js:11(post)
  /api/melody2score/samples  <- melody.api.js:6(get)
  /api/melody2score/save-report  <- melody.api.js:17(post)
  /api/melody2score/status  <- melody.api.js:5(get)
  /api/monitor/ai-diagnose  <- ai.api.js:77(post)
  /api/monitor/timeseries  <- monitor.api.js:36(get)
  /api/operators/ai-recommend  <- ai.api.js:69(post)
  /api/plugins/ai-route  <- ai.api.js:79(post)
  /api/projects/by-resource  <- projects.api.js:23(get)
  /api/resources/ai-analysis  <- ai.api.js:70(post)
  /api/storage/status  <- system.api.js:33(get)
  /api/system/logininfor/clean  <- system.api.js:127(delete)
  /api/system/user  <- system.api.js:61(get)
  /api/tasks/auto  <- projects.api.js:34(post)
  /api/tasks/from-chat  <- projects.api.js:31(post)
  /api/web-search/config  <- ai.api.js:12(get)
  /api/web-search/test  <- ai.api.js:14(post)
  /api/workbench/ai-overview  <- ai.api.js:82(get)
  /api/workflow/ai-generate  <- ai.api.js:71(post)
  /api/workspace/kpi  <- workspace.api.js:8(get)
```

## D. 第二把尺子的负结果（静态路由目录不可当权威）

试图只读 `actuator.rs` 的 `r("name","METHOD","/path",…)` 目录行来离线判定"首段是否存在"（驱动 `static-catalog.py`）：

```

```

⇒ 该目录登记的是 `/voice/v1`·`/melody/v1`·`/kg/…` 这类**根前缀**域，对 `/api/*` 面基本是盲的（差集里连 `/tasks`·`/projects` 都"查无此段"而它们在 A 节实测 200）。
结论：**任何常驻门禁都不许拿 actuator 目录当路由真值**，只能拿现场判决或 Rust 挂载代码（`modules.rs` 的 `.merge(...)` 与 `deployment.rs` 的 `nest`）当输入。

## E. 案例：Melody 域的双重错位（源码级事实，行号现场 grep）

```
$ grep -n "melody2score" frontend-ui/src/api/melody.api.js | head -4
4:export const melodyHealth = () => http.get('/melody2score/health')
5:export const melodyStatus = () => http.get('/melody2score/status')
6:export const melodySamples = () => http.get('/melody2score/samples')
7:export const melodyRecognize = (formData) => http.post('/melody2score/recognize', formData, {

$ grep -n "pub fn build_melody_router" platform/gateway/mox-platform-gateway-svc/src/melody.rs
135:pub fn build_melody_router() -> Router<()> {

$ grep -n "build_melody_router" platform/gateway/mox-platform-gateway-svc/src/modules.rs
149:        .merge(upgrade(crate::melody::build_melody_router()))

$ grep -n "nest(\"/api\"" platform/gateway/mox-platform-gateway-svc/src/deployment.rs
25:        HostRole::Kb => upgrade(Router::new().nest("/api", mox_kb_svc::handlers::build_kb_router())),

$ grep -n "const GW" frontend-ui/vite.config.js
95:        const GW = process.env.GATEWAY_URL || 'http://localhost:3080'

$ grep -n "^ *'" frontend-ui/vite.config.js | sed -n '1,14p'
67:        'import.meta.env.VITE_OUS_API_TOKEN': JSON.stringify(_TOKEN),
68:        'import.meta.env.OUS_API_TOKEN': JSON.stringify(_TOKEN),
71:      'import.meta.env.VITE_APP_VERSION': JSON.stringify(
74:      'import.meta.env.VITE_BUILD_TIME': JSON.stringify(new Date().toISOString()),
78:        '@': fileURLToPath(new URL('./src', import.meta.url))
90:        'X-Content-Type-Options': 'nosniff',
91:        'X-Frame-Options': 'SAMEORIGIN',
92:        'Referrer-Policy': 'strict-origin-when-cross-origin'
131:          '/ai/engine': {
137:          '/voice': {
143:          '/ws': {
150:          '/api': {
157:          '/actuator': {
165:          '/kg': {

$ grep -n "'/melody'" frontend-ui/vite.config.js
(无命中)

$ grep -c "melody" frontend-ui/vite.config.js
0

$ grep -rn "Melody2ScoreView" frontend-ui/src/modules/ai/index.js
48:      component: () => import('@/views/ai/Melody2ScoreView.vue'),

```

## F. 交叉账：幻影调用点 × 真实消费者（驱动 `cross-ledger.py`，原件 `cross3.out`）

口径：导出名＝从调用点向上找最近一条 `export const/function`；"有消费者"＝该文件按名 import 了它，**或**该文件 `import * as X from '@/api'` 且出现 `X.<名>` 成员取值（只认前一种会把 29 处真消费者错判成死函数——实测两形之间 65 → 94）。只撞名而拿不到导出的，单列不算任何一类。

```
正对照（已知活函数必须判"有消费者"，判否＝尺子坏了）：
  getTasks             in views/project/TaskView.vue         -> True
  getProjects          in views/project/ProjectsView.vue     -> True
  getAlgorithmTypes    in views/ai/AlgoLabView.vue           -> True

幻影调用点总数=156（唯一路径 156 条）；解出导出名 155 处
有按名 import 的真实调用者（＝活路径打到 404，真故障）：94 处
零调用者（＝api 死函数，可退役候选）：61 处
  其中"只撞名、没 import"的：2 处 ['getDispatcherConfig←/experts/dispatcher/config', 'getExpertMetrics←/experts/metrics']
解不出导出名（写法未覆盖，不算任何一类）：1 处

=== 真故障清单（幻影路径 / 导出名 / 调用者文件×次数）
  /api/ai/algorithm-types                        ai.api.js:getAlgorithmTypes         3 处  views/admin/panels/AdminDocs.vue×1, views/ai/AlgoLabView.vue×2
  /api/ai/analyze-algorithm                      ai.api.js:analyzeAlgorithm          3 处  views/admin/panels/AdminDocs.vue×1, views/ai/AlgoLabView.vue×2
  /api/ai/browser/execute-task                   workflow.api.js:executeBrowserTask  2 处  views/workflow/BrowserView.vue×2
  /api/ai/browser/natural                        workflow.api.js:browserNatural      2 处  views/workflow/BrowserView.vue×2
  /api/ai/browser/sessions                       workflow.api.js:getBrowserSessions  3 处  views/admin/panels/AdminDocs.vue×1, views/workflow/BrowserView.vue×2
  /api/ai/browser/templates                      workflow.api.js:getBrowserTemplates  3 处  views/admin/panels/AdminDocs.vue×1, views/workflow/BrowserView.vue×2
  /api/ai/chat                                   ai.api.js:aiChat                    7 处  stores/ai.store.js×2, views/admin/panels/AdminDocs.vue×1, views/expert/AllianceTaskView.vue×2
  /api/ai/engine/flow-graph                      ai.api.js:getEngineFlowGraph        1 处  views/expert/panels/ExpertEnterprisePanel.vue×1
  /api/ai/flows                                  workflow.api.js:getFlows           11 处  views/admin/panels/AdminDocs.vue×1, views/ai/BotCenterView.vue×2, views/project/Workbench.vue×2
  /api/ai/flows/execute                          workflow.api.js:executeFlow         9 处  views/project/Workbench.vue×2, views/public/BusinessHall.vue×2, views/workflow/WorkflowView.vue×2
  /api/ai/flows/node-types                       workflow.api.js:getFlowNodeTypes    2 处  views/workflow/WorkflowView.vue×2
  /api/ai/flows/validate                         workflow.api.js:validateFlow        2 处  views/workflow/WorkflowView.vue×2
  /api/ai/infinite-optimize/apply                ai.api.js:applyBestConfig           1 处  views/ai/InfiniteOptimizerView.vue×1
  /api/ai/infinite-optimize/benchmarks           ai.api.js:getInfiniteBenchmarks     1 处  views/ai/InfiniteOptimizerView.vue×1
  /api/ai/infinite-optimize/compare              ai.api.js:runProviderComparison     1 处  views/ai/InfiniteOptimizerView.vue×1
  /api/ai/infinite-optimize/comparison           ai.api.js:getProviderComparison     1 处  views/ai/InfiniteOptimizerView.vue×1
  /api/ai/infinite-optimize/results              ai.api.js:getInfiniteOptimizeResults  1 处  views/ai/InfiniteOptimizerView.vue×1
  /api/ai/infinite-optimize/start                ai.api.js:startInfiniteOptimize     1 处  views/ai/InfiniteOptimizerView.vue×1
  /api/ai/infinite-optimize/status               ai.api.js:getInfiniteOptimizeStatus  1 处  views/ai/InfiniteOptimizerView.vue×1
  /api/ai/infinite-optimize/stop                 ai.api.js:stopInfiniteOptimize      1 处  views/ai/InfiniteOptimizerView.vue×1
  /api/ai/llm/config                             llm.api.js:getLlmConfig             1 处  views/admin/panels/AdminDocs.vue×1
  /api/ai/llm/test                               llm.api.js:testLlm                  1 处  views/admin/panels/AdminDocs.vue×1
  /api/ai/plugins                                workflow.api.js:getAiPlugins        3 处  views/admin/panels/AdminDocs.vue×1, views/workflow/panels/PluginsPanel.vue×2
  /api/ai/plugins/register                       workflow.api.js:registerAiPlugin    2 处  views/workflow/panels/PluginsPanel.vue×2
  /api/ai/plugins/send-message                   workflow.api.js:sendPluginMessage   2 处  views/workflow/panels/PluginsPanel.vue×2
  /api/ai/plugins/topology                       workflow.api.js:getPluginTopology   3 处  views/admin/panels/AdminDocs.vue×1, views/workflow/panels/PluginsPanel.vue×2
  /api/ai/project-graph                          ai.api.js:aiGenerateProjectGraph    2 处  views/project/ProjectsView.vue×2
  /api/ai/resources                              projects.api.js:getResources        3 处  views/admin/panels/AdminDocs.vue×1, views/project/ResourcesView.vue×2
  /api/ai/resources/health                       projects.api.js:getResourceHealth   2 处  views/project/ResourcesView.vue×2
  /api/ai/workflows                              graph.api.js:getAggregatedGraph     2 处  views/graph/GraphView.vue×2
  /api/ai/workflows/execute                      workflow.api.js:executeWorkflowDef  2 处  views/workflow/WorkflowView.vue×2
  /api/ai/workflows/instances                    workflow.api.js:getWorkflowInstances  2 处  views/workflow/WorkflowView.vue×2
  /api/ai/workflows/save                         workflow.api.js:saveWorkflow        2 处  views/workflow/WorkflowView.vue×2
  /api/ai/workflows/templates                    workflow.api.js:getWorkflowTemplates  3 处  views/admin/panels/AdminDocs.vue×1, views/workflow/WorkflowView.vue×2
  /api/alliance/runtime                          alliance.api.js:getAllianceRuntime  1 处  views/expert/AllianceTaskView.vue×1
  /api/alliance/sediment                         ai.api.js:sedimentDialogue          2 处  views/ai/ChatView.vue×2
  /api/alliance/tasks                            alliance.api.js:createAllianceTask  3 处  api/allianceTasks.test.js×2, views/expert/AllianceTaskView.vue×1
  /api/analyze/spiral                            ai.api.js:analyzeSpiral             3 处  views/admin/panels/AdminDocs.vue×1, views/ai/AlgoLabView.vue×2
  /api/automation/chat                           workflow.api.js:automationChat      2 处  views/workflow/panels/AutomationPanel.vue×2
  /api/caomei/compile                            caomei.api.js:caomeiCompile         3 处  views/admin/panels/AdminDocs.vue×1, views/ai/CaomeiView.vue×2
  /api/caomei/refine                             caomei.api.js:caomeiRefine          3 处  views/admin/panels/AdminDocs.vue×1, views/ai/CaomeiView.vue×2
  /api/caomei/templates                          caomei.api.js:caomeiTemplates       3 处  views/admin/panels/AdminDocs.vue×1, views/ai/CaomeiView.vue×2
  /api/dialogue/sessions                         graph.api.js:getDialogueSessions    1 处  views/admin/panels/AdminDocs.vue×1
  /api/expert-graph/optimal-team                 experts.api.js:findOptimalTeam      1 处  views/expert/panels/ExpertEnterprisePanel.vue×1
  /api/expert-graph/rebuild                      experts.api.js:rebuildExpertGraph   1 处  views/expert/panels/ExpertEnterprisePanel.vue×1
  /api/expert-graph/stats                        experts.api.js:getExpertGraphStats  2 处  views/expert/panels/ExpertEnterprisePanel.vue×2
  /api/experts/bookings                          experts.api.js:createBooking        2 处  views/expert/ExpertPlazaView.vue×2
  /api/experts/bookings/mine                     experts.api.js:getMyBookings        2 处  views/expert/ExpertPlazaView.vue×2
  /api/experts/dispatcher/status                 experts.api.js:getDispatcherStatus  2 处  views/expert/panels/ExpertEnterprisePanel.vue×2
  /api/experts/enterprise/consult                experts.api.js:enterpriseConsult    1 处  views/expert/panels/ExpertEnterprisePanel.vue×1
  /api/experts/orchestrate                       experts.api.js:expertOrchestrate    2 处  views/expert/panels/ExpertOrchestratorPanel.vue×2
  /api/experts/orchestration/history             experts.api.js:getOrchestrationHistory  2 处  views/expert/panels/ExpertOrchestratorPanel.vue×2
  /api/experts/orchestration/plugins             experts.api.js:getOrchestrationPlugins  2 处  views/expert/panels/ExpertOrchestratorPanel.vue×2
  /api/experts/orchestration/stats               experts.api.js:getOrchestrationStats  2 处  views/expert/panels/ExpertOrchestratorPanel.vue×2
  /api/experts/overview                          experts.api.js:getExpertOverview    2 处  views/expert/panels/ExpertOverviewPanel.vue×2
  /api/experts/plan/generate                     experts.api.js:expertGeneratePlan   2 处  views/expert/panels/ExpertOrchestratorPanel.vue×2
  /api/experts/sessions                          experts.api.js:createExpertSession  3 处  views/expert/panels/ExpertEnterprisePanel.vue×1, views/project/Workbench.vue×2
  /api/experts/stats                             experts.api.js:getExpertsStats      2 处  views/expert/ExpertPlazaView.vue×2
  /api/experts/team                              experts.api.js:joinExpertTeam       2 处  views/expert/ExpertPlazaView.vue×2
  /api/graph/activate                            graph.api.js:propagateActivation    4 处  views/admin/panels/AdminDocs.vue×1, views/graph/GraphView.vue×3
  /api/graph/centrality                          graph.api.js:getCentrality          5 处  views/admin/panels/AdminDocs.vue×1, views/graph/GraphView.vue×4
  /api/graph/communities                         graph.api.js:getCommunities         4 处  views/admin/panels/AdminDocs.vue×1, views/graph/GraphView.vue×3
  /api/graph/pagerank                            graph.api.js:getPagerank            4 处  views/admin/panels/AdminDocs.vue×1, views/graph/GraphView.vue×3
  /api/graph/path                                graph.api.js:getShortestPath        3 处  views/graph/GraphView.vue×3
  /api/graph/recommend                           graph.api.js:recommendNodes         2 处  views/graph/GraphView.vue×2
  /api/graph/search                              graph.api.js:graphSearch            2 处  views/graph/GraphView.vue×2
  /api/graph/stats                               graph.api.js:getGraphStats          3 处  views/admin/panels/AdminDocs.vue×1, views/graph/GraphView.vue×2
  /api/kb/batch-analyze                          kb.api.js:kbBatchAnalyze            1 处  views/project/panels/KnowledgeBasePanel.vue×1
  /api/kb/categories                             kb.api.js:kbGetCategories           5 处  composables/useKnowledgeBase.js×1, views/project/panels/KnowledgeBasePanel.vue×2, views/workspace/ExpertWorkspaceView.vue×2
  /api/kb/documents                              graph.api.js:getAggregatedGraph     2 处  views/graph/GraphView.vue×2
  /api/kb/entities/search                        kb.api.js:kbSearchEntities          1 处  composables/useKnowledgeBase.js×1
  /api/kb/history                                kb.api.js:kbGetHistory              1 处  composables/useKnowledgeBase.js×1
  /api/kb/search                                 kb.api.js:kbSearch                  2 处  views/workspace/ExpertWorkspaceView.vue×2
  /api/kb/stats                                  kb.api.js:kbGetStats                3 处  composables/useKnowledgeBase.js×1, views/project/panels/KnowledgeBasePanel.vue×2
  /api/kb/tags                                   kb.api.js:kbGetTags                 5 处  composables/useKnowledgeBase.js×1, views/project/panels/KnowledgeBasePanel.vue×2, views/workspace/ExpertWorkspaceView.vue×2
  /api/llm/health                                llm.api.js:getLlmHealth             1 处  views/admin/panels/AdminLlm.vue×1
  /api/llm/logs                                  llm.api.js:getLlmLogs               1 处  views/admin/panels/AdminLlm.vue×1
  /api/llm/providers                             llm.api.js:getLlmProviders          1 处  views/admin/panels/AdminLlm.vue×1
  /api/llm/providers/active                      llm.api.js:setActiveProvider        1 处  views/admin/panels/AdminLlm.vue×1
  /api/llm/stats                                 llm.api.js:getLlmStats              3 处  views/admin/panels/AdminLlm.vue×1, views/admin/panels/AdminOverview.vue×2
  /api/llm/usage                                 llm.api.js:getLlmUsage              1 处  views/admin/panels/AdminLlm.vue×1
  /api/melody2score/export-sheet                 melody.api.js:melodyExportSheet     2 处  views/ai/Melody2ScoreView.vue×2
  /api/melody2score/health                       melody.api.js:melodyHealth          2 处  views/ai/Melody2ScoreView.vue×2
  /api/melody2score/recognize                    melody.api.js:melodyRecognize       2 处  views/ai/Melody2ScoreView.vue×2
  /api/melody2score/recognize-sample             melody.api.js:melodyRecognizeSample  2 处  views/ai/Melody2ScoreView.vue×2
  /api/melody2score/samples                      melody.api.js:melodySamples         2 处  views/ai/Melody2ScoreView.vue×2
  /api/melody2score/save-report                  melody.api.js:melodySaveReport      2 处  views/ai/Melody2ScoreView.vue×2
  /api/monitor/timeseries                        monitor.api.js:getTimeseries        2 处  views/admin/panels/AdminMonitor.vue×2
  /api/storage/status                            system.api.js:getStorageStatus      4 处  views/admin/panels/AdminOverview.vue×2, views/admin/panels/AdminStorage.vue×2
  /api/system/logininfor/clean                   system.api.js:cleanLoginLog         2 处  views/admin/panels/AdminAudit.vue×2
  /api/system/user                               system.api.js:getUserList           4 处  views/admin/panels/AdminDepartment.vue×2, views/admin/panels/AdminUser.vue×2
  /api/web-search/config                         ai.api.js:getWebSearchConfig        1 处  views/admin/panels/AdminLlm.vue×1
  /api/web-search/test                           ai.api.js:testWebSearch             1 处  views/admin/panels/AdminLlm.vue×1
  /api/workspace/kpi                             workspace.api.js:getWorkspaceKpi    2 处  composables/workspace/useWorkspaceData.js×2

=== 零调用者清单（按 api 文件聚合）
  ai.api.js              28 个：aiAlgoLabAnalyze, aiAutomationExecute, aiBrowserInstruct, aiCaomeiParse, aiDevTestFix, aiDocsExplain, aiFullAnalysis, aiFullComplete, aiFusionGovern, aiGenerateDoc, aiGenerateErd, aiGenerateFlowDiagram, aiGenerateWorkflow, aiLinkReqToDb, aiMarketSearch, aiMcpMap, aiMonitorDiagnose, aiOptimizeDoc, aiPluginRoute, aiProjectFromChat, aiPublishArtifactsToKb, aiRecommendOperators, aiResourceAnalysis, allianceEnterprisePipeline, createArtifact, getArtifactConfig, getArtifacts, getWorkbenchAiOverview
  alliance.api.js        5 个：allianceAlgorithmAnalysis, allianceExpertDebate, allianceIntelligentConsult, allianceMultiExpertConsult, allianceRouteExperts
  experts.api.js         12 个：dispatcherConsult, dispatcherDispatch, dispatcherMultiConsult, enterpriseAnalyze, expertExecutePlan, expertSemanticSearch, getDispatcherConfig, getExpertCapabilities, getExpertGraphCommunities, getExpertMetrics, getExpertSessionStats, resetDispatcherAll
  graph.api.js           7 个：addGraphEdge, addGraphNode, aiGraphInsights, getAutoSyncStatus, graphExport, graphImport, toggleAutoSync
  llm.api.js             2 个：getLlmProviderPresets, getLlmRouting
  melody.api.js          2 个：melodyRecognizeRecord, melodyStatus
  projects.api.js        3 个：autoCreateTask, convertChatToTask, getProjectsByResource
  workflow.api.js        2 个：executeBrowserAction, executeBrowserSteps

=== 解不出导出名（逐条列，别塞进任一类）
  /api/auth/me                                   http.test.js:9
```

```
$ grep -rn "import \* as .* from '@/api" --include=*.vue --include=*.js frontend-ui/src | wc -l
7
$ grep -rln "import \* as .* from '@/api'" frontend-ui/src  # 命名空间形的文件
frontend-ui/src/composables/useKnowledgeBase.js
frontend-ui/src/views/admin/panels/AdminDocs.vue
frontend-ui/src/views/admin/panels/AdminLlm.vue
frontend-ui/src/views/ai/InfiniteOptimizerView.vue
frontend-ui/src/views/expert/AllianceTaskView.vue
frontend-ui/src/views/expert/panels/ExpertEnterprisePanel.vue
frontend-ui/src/views/project/panels/KnowledgeBasePanel.vue
```

## G. 第二次独立测量：156 条"幻影"里 104 条是仪器假账（2026-09-28 02:27–02:31）

驱动 `%s\rejudge.py`（仓库外，未落库⇒没牙）。判据与首轮同语义（一律 GET、带令牌、404 且空体记幻影），差别只有一处：**探测 URL 用台账里印出的那条 wire 路径本身**（`http://127.0.0.1:3080` + `/api/...`），而首轮探的是 `http://127.0.0.1:3080` + **源里的字面量**（`/ai/flows` 这类不含 baseURL 前缀的写法）。`src/api/http.js:11-12` 是 `axios.create({ baseURL: '/api' })` ⇒ 首轮对 156 条里绝大多数"前缀-free 字面量"打的是缺 `/api` 的地址，判决却是按加了 `/api` 的显示路径印出来的——**显示路径与探测路径不同源**。
### G.1 冒烟证据（两把尺子的源码逐字，读数取自磁盘上的驱动本体）
```def probe(p):
    r = subprocess.run(['curl', '-s', '-w', '\n%{http_code}', '--max-time', '8',
                        '-H', 'Authorization: Bearer ' + TOK,
                        'http://127.0.0.1:3080' + p],
                       capture_output=True, text=True, encoding='utf-8', errors='replace')
    out = (r.stdout or '').rstrip('\n')
    head, _, tail = out.rpartition('\n')
    if re.fullmatch(r'\d{3}', tail.st```首轮探测式＝`'http://127.0.0.1:3080' + p`，p 是**源字面量**；而 final 台账打印式（`reprobe.py` 末段）：```````'/api%s'` 只作用于**打印**，不作用于探测 ⇒ 同一行里"路径"与"码"来自两个不同地址。
### G.2 本轮读数（现场切自 `rejudge.out`）
```[预检] live=('200', 208) fake=('404', 0)：200×52, 400×2, 404×52, 405×48, 500×1, 502×1仍判 404：52 条 / 翻转（非 404）：104 条```### G.3 翻转清单前 12 条（首轮判 404 空体，本轮带 `/api` 前缀重打＝非 404）
```  /api/ai/algorithm-types                        200 bytes=786    application/json | {"code":0,"data":{"types":[{"algorithms":["快速排序","归并排序","堆排�  /api/ai/analyze-algorithm                      405 bytes=0       |   /api/ai/browser/execute-action                 405 bytes=0       |   /api/ai/browser/execute-steps                  405 bytes=0       |   /api/ai/browser/execute-task                   405 bytes=0       |   /api/ai/browser/natural                        405 bytes=0       |   /api/ai/browser/sessions                       200 bytes=44     application/json | {"code":0,"data":{"sessions":[]},"msg":"ok"}  /api/ai/browser/templates                      200 bytes=854    application/json | {"code":0,"data":{"templates":[{"description":"在搜索引擎中搜索关键词  /api/ai/chat                                   405 bytes=0       |   /api/ai/engine/flow-graph                      200 bytes=2903   application/json | {"code":0,"msg":"ok","data":{"edges":[{"source":"intent","ta  /api/ai/flows                                  200 bytes=2593   application/json | {"code":0,"data":{"flows":[{"created_at":"2026-09-27T16:26:2  /api/ai/flows/execute                          405 bytes=0       | ```405 是"路由在、方法不是 GET"（本轮一律 GET 是硬约束，不能为了拿 200 去写状态）；400/500/502 各条都说明该路径**被某个宿主接住了**，与"无任何路由"矛盾。
### G.4 结论口径
- 首轮台账的 156 条"幻影"＝**52 条真 404 空体 ＋ 104 条仪器假账**；156 这个数不得再被引用。
- 202 条静态字面量的正确分桶＝150 条有路由（含 405 那批"路径在、方法不对"）＋52 条无路由。
- 前缀-free 是 `src/api/*.js` 的**主流写法**（baseURL 补 `/api`），本轮 52 条真幻影**全部**仍是前缀-free 写法，  所以"写法的形态"与"路由的存在性"无关，第二轮测量把这两件事分开了。

## H. 52 条真幻影的三口径结案账（2026-09-28）

驱动 `C:\Users\mo\AppData\Local\Temp\mox-endpoint-census\route-catalog.py`＋`C:\Users\mo\AppData\Local\Temp\mox-endpoint-census\final-52.py`（均在仓库外，未落库⇒没牙）。
### H.1 静态路由目录的装配力（`catalog.out` 前若干行）
```入口展开的函数体根：133 个 / 扫描 .rs 149 个静态目录：377 条唯一 (host,file,line,method,path)按宿主：gateway=255（:3080 原生）, orchestrator=122（:3001（网关 /api 反代））装配对照：绝对 3/3 命中，前缀 12/13 命中，反向误收 0 条  缺：['/api/enterprise/admin/'] / 误收：[] ⇒ 目录不完整，ABSENT 判决需人工复核```目录构造规则＝axum 0.7 的 `route/nest/merge` 语义：`nest` 给子路由加前缀、`merge` 不加（源里多写绝对路径）；只从入口文件（网关 `modules.rs`/`deployment.rs`/`lib.rs`/`main.rs`、编排器 `main.rs`、各 svc `main.rs`）展开，其余函数体只能经 merge/nest 抵达 ⇒ 相对路径必带前缀。第一版把 `Router::new().nest(...)` 的容器取成了 `new()` 自己的括号区，5 个 nest 前缀（`/melody/v1`·`/voice/v1`·`/cloud/v1`·`/api/kb`·`/api/alliance/sediment`）整个丢失；修法是取 `Router::new()` 之后的**整段链**，目录从 360 条增至 377 条，前缀对照由 9/13 升到 12/13。残留盲区（要点名，它使 ABSENT 偏严）：`/api/enterprise/admin/` 仍未装配出来。
### H.2 三口径逐条账（`final52.out` 全文）
```口径一：第二次测量仍 404 空体 52 条 / 翻出假账 104 条（156＝52＋104）

消费者正对照：
  getTasks       in views/project/TaskView.vue       -> True
  getProjects    in views/project/ProjectsView.vue   -> True
目录正对照：/api/ai/chat 在目录=True，/api/__zzz__ 在目录=False

====================================================================================================
真幻影路径                                      静态判决      消费者     证据（目录里的最近路由 / 调用者）
====================================================================================================
/api/ai/alliance-pipeline                  ABSENT    0(零调用者) —
/api/ai/artifact/config                    ABSENT    0(零调用者) —
/api/ai/artifact/create                    ABSENT    0(零调用者) —
/api/ai/artifact/list                      ABSENT    0(零调用者) —
/api/ai/dev-test-fix                       ABSENT    0(零调用者) —
/api/ai/full-analysis                      ABSENT    0(零调用者) —
/api/ai/full-complete                      ABSENT    0(零调用者) —
/api/ai/generate-doc                       ABSENT    0(零调用者) —
/api/ai/generate-erd                       ABSENT    0(零调用者) —
/api/ai/generate-flow-diagram              ABSENT    0(零调用者) —
/api/ai/infinite-optimize/apply            ABSENT    1(真故障)  —
/api/ai/infinite-optimize/benchmarks       ABSENT    1(真故障)  —
/api/ai/infinite-optimize/compare          ABSENT    1(真故障)  —
/api/ai/infinite-optimize/comparison       ABSENT    1(真故障)  —
/api/ai/infinite-optimize/results          ABSENT    1(真故障)  —
/api/ai/infinite-optimize/start            ABSENT    1(真故障)  —
/api/ai/infinite-optimize/status           ABSENT    1(真故障)  —
/api/ai/infinite-optimize/stop             ABSENT    1(真故障)  —
/api/ai/optimize-doc                       ABSENT    0(零调用者) —
/api/ai/project-from-chat                  ABSENT    0(零调用者) —
/api/ai/project-graph                      ABSENT    2(真故障)  —
/api/ai/publish-kb                         ABSENT    0(零调用者) —
/api/ai/req-db-link                        ABSENT    0(零调用者) —
/api/algolab/ai-analyze                    ABSENT    0(零调用者) —
/api/browser/ai-instruct                   ABSENT    0(零调用者) —
/api/caomei/ai-parse                       ABSENT    0(零调用者) —
/api/docs/ai-explain                       ABSENT    0(零调用者) —
/api/fusion/ai-govern                      ABSENT    0(零调用者) —
/api/graph/ai-insights                     ABSENT    0(零调用者) —
/api/llm/logs                              ABSENT    1(真故障)  —
/api/llm/providers/active                  ABSENT    1(真故障)  —
/api/llm/stats                             ABSENT    3(真故障)  —
/api/llm/usage                             ABSENT    1(真故障)  —
/api/mcp/ai-map                            ABSENT    0(零调用者) —
/api/melody2score/export-sheet             ABSENT    2(真故障)  —
/api/melody2score/health                   ABSENT    2(真故障)  —
/api/melody2score/recognize                ABSENT    2(真故障)  —
/api/melody2score/recognize-record         ABSENT    0(零调用者) —
/api/melody2score/recognize-sample         ABSENT    2(真故障)  —
/api/melody2score/samples                  ABSENT    2(真故障)  —
/api/melody2score/save-report              ABSENT    2(真故障)  —
/api/melody2score/status                   ABSENT    0(零调用者) —
/api/monitor/ai-diagnose                   ABSENT    0(零调用者) —
/api/operators/ai-recommend                ABSENT    0(零调用者) —
/api/plugins/ai-route                      ABSENT    0(零调用者) —
/api/resources/ai-analysis                 ABSENT    0(零调用者) —
/api/tasks/auto                            ABSENT    0(零调用者) —
/api/tasks/from-chat                       ABSENT    0(零调用者) —
/api/web-search/config                     ABSENT    1(真故障)  —
/api/web-search/test                       ABSENT    1(真故障)  —
/api/workbench/ai-overview                 ABSENT    0(零调用者) —
/api/workflow/ai-generate                  ABSENT    0(零调用者) —

=== 汇总
  ABSENT      真故障        21 条
  ABSENT      零调用者       31 条

=== 按族（首两段前缀 × 静态判决）：条数 / 活调用点数
  api/melody2score         ABSENT       8 条 / 活调用点  12 处
  api/ai                   ABSENT      23 条 / 活调用点  10 处
  api/llm                  ABSENT       4 条 / 活调用点   6 处
  api/web-search           ABSENT       2 条 / 活调用点   2 处
  api/algolab              ABSENT       1 条 / 活调用点   0 处
  api/browser              ABSENT       1 条 / 活调用点   0 处
  api/caomei               ABSENT       1 条 / 活调用点   0 处
  api/docs                 ABSENT       1 条 / 活调用点   0 处
  api/fusion               ABSENT       1 条 / 活调用点   0 处
  api/graph                ABSENT       1 条 / 活调用点   0 处
  api/mcp                  ABSENT       1 条 / 活调用点   0 处
  api/monitor              ABSENT       1 条 / 活调用点   0 处
  api/operators            ABSENT       1 条 / 活调用点   0 处
  api/plugins              ABSENT       1 条 / 活调用点   0 处
  api/resources            ABSENT       1 条 / 活调用点   0 处
  api/tasks                ABSENT       2 条 / 活调用点   0 处
  api/workbench            ABSENT       1 条 / 活调用点   0 处
  api/workflow             ABSENT       1 条 / 活调用点   0 处

写出 final52.json```### H.3 Melody 族的双向对账（目录里 `/melody/v1` 全部 7 条 vs 前端 8 条字面量）
```# 网关侧（静态目录，宿主 gateway）
/melody/v1/health            get    melody.rs:2
/melody/v1/recognize         post   melody.rs:3
/melody/v1/recognize-sample  post   melody.rs:4
/melody/v1/recognize-record  post   melody.rs:5
/melody/v1/save-md           post   melody.rs:6
/melody/v1/export-sheet      post   melody.rs:7
/melody/v1/download/:fname   get    melody.rs:8
/voice/v1/health             get    voice.rs:2
/voice/v1/samples            get    voice.rs:3
/voice/v1/recognize          post   voice.rs:4
# 前端侧（src/api/melody.api.js 全部字面量，经 baseURL=/api）
melodyHealth             /api/melody2score/health   melody.api.js:4
melodyStatus             /api/melody2score/status   melody.api.js:5
melodySamples            /api/melody2score/samples  melody.api.js:6
melodyRecognize          /api/melody2score/recognize melody.api.js:7
melodyRecognizeSample    /api/melody2score/recognize-sample melody.api.js:11
melodyRecognizeRecord    /api/melody2score/recognize-record melody.api.js:15
melodyExportSheet        /api/melody2score/export-sheet melody.api.js:16
melodySaveReport         /api/melody2score/save-report melody.api.js:17```逐条判决：`health`/`recognize`/`recognize-sample`/`recognize-record`/`export-sheet` **5 条尾段在 `/melody/v1` 下逐字存在**⇒ 改前缀可救（含 `createHttpInstance('/melody/v1')` 与 vite 代理键两件事，见 §5.18 E 节）；`samples` 的孪生条在**隔壁 voice 域**（`/voice/v1/samples`）⇒ 要么指过去要么后端补；`status` 无孪生条⇒要后端补；`save-report` 只有语义相近的 `save-md`⇒改名可救但需后端确认语义等价。网关侧 `/melody/v1/download/:fname` 前端**零字面量**（反方向缺口，本轮不计入幻影）。
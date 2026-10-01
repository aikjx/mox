# 联盟治理台真机联调记录（2026-09-28）

本文件是 §5.20 的原始读数档案：下面每一段都是本轮现场印出的输出，文档里的数字一律指向本文件。

- 后端：`mox-platform-orchestrator-svc`（:3001）经模块化网关（:3080）反代，进程为用户所有，本轮未重启、未改。
- 前端：Vite dev（:3020，我起的后台实例）；页面 `http://localhost:3020/#/alliance/governance`。
- 通道：`curl` 直连网关；DOM 读数走 browser-use `evaluate_script`（该工具无视口，`document.visibilityState` 恒为 `hidden`）。

## 1. 五个读通道（curl 直连网关，本轮现场输出）

```
GET /api/governance/dashboard -> HTTP 200, 2350 bytes
GET /api/governance/experts/status -> HTTP 200, 2289 bytes
GET /api/governance/veto/events -> HTTP 200, 92 bytes
GET /api/governance/audit/logs -> HTTP 200, 91 bytes
GET /api/governance/config/experts -> HTTP 200, 411 bytes
-- double prefix control:
GET /api/api/governance/dashboard -> HTTP 404
```

前缀账：`api/http.js:11-12` 的 `baseURL` 是 `/api`，而模块契约 `contract/endpoints.js:110-118` 的 `requestPath()` 结尾 `.replace(/^\/api/, '')` 剥掉台账里的 `/api` 前缀 ⇒ 实际出站是 `/api/governance/...`（200），双重前缀 `/api/api/governance/...` 是 404。治理台不存在"双前缀"缺陷。

## 2. `/experts/status` 的 wire 形状（同一条响应两套命名口径）

```
['business_league', 'dev_league', 'mox', 'timestamp']
['average_health', 'dimensions', 'experts']
{'dimension': 'business', 'enabled': True, 'expertId': 'business', 'healthScore': 1.0, 'lastUpdated': 1790526384, 'totalChecks': 0, 'vetoCount': 0}
experts 条数 business/dev = 7 7
```

外层 `business_league` / `dev_league` / `average_health` 是 snake_case，而 `experts[]` 元素是 camelCase（`healthScore` / `lastUpdated` / `totalChecks` / `expertId`）。`model/normalize.js:122-145` 已经把这件事收口（`business_league → business`、`average_health → averageHealth`，注释里明文警告过键的口径），所以这不是缺陷。**十四维全部有回**，`absentDims` 实测为空。

## 3. 本轮修掉的缺陷：把"还没读到"渲染成"后端未回该维"

现象（改动前，页面首帧）：状态端点在飞、概览端点也还没回 ⇒ `leagues` 里 `byDim` 是空表 ⇒ 十四维全部落进 `{ absent: true }`，页面上同时挂出 14 枚「后端未回该维」；`store.absentDims` 同源地把"没答案"算成"缺 14 维"（它只读 `expertsStatus`，为 null 时 `present` 就是空集）。这是把"无判决"当成"负判决"，与本页其它地方"配置未取到，显示缺省值"的诚实口径不一致。

改动（三处，全在 `frontend-ui/src/modules/governance/`）：

| 文件:行 | 改动 |
|---|---|
| `store/governance.store.js:34-44` | `absentDims` 先取 `expertsStatus.value`，为 null 直接 `return []`（无答案＝无从判断缺谁），并把可选链改成显式解引用 |
| `views/GovernanceConsoleView.vue:229-241` | `leagues` 增 `const pending = !src && store.loading.experts`，缺行对象带 `{ absent: !pending, pending }` |
| `views/GovernanceConsoleView.vue:59-65`、`71-84` | 警告段落拆成两条互斥（无答案＝「状态端点 读数中／未回答，下面退回概览端点里的 expertStates」；有答案才允许「缺 N 维（…）」）；标签链改成 `absent → 后端未回该维` / `pending → 读数中` / `否则分数`，进度条与 `否决/检查/更新` 两处的门从 `!row.absent` 收紧为 `!row.absent && !row.pending` |

## 4. 双向见证（真机；合成状态那三段**不是后端数据**）

改动后重新加载，早采样（loading 窗口内，原样输出）：

```
{"phase":"early","scores":14,"absent":14,"pending":14,"warns":[]}
```

`absent`/`pending`/`scores` 同时为 14 不是"页面同时说三种话"，而是这台仪器的读数被 CSS transition 污染：见 §5。去掉正在离场的节点后，稳态是：

```
{"phase":"settled","vis":"hidden","scores":14,"absent":0,"pending":0,"otherTags":[],"warns":[]}
```

反方向（证明"真缺维"仍然判得出来，不是被我的守卫顺手关掉）——把 pinia 里的 `expertsStatus` 合成裁成 business 3 条 + dev 0 条：

```
{"synthetic_dropped10":{"absent":10,"pending":0,"scores":4,
  "warns":["缺 10 维（security、data、observability、api_compat、performance、maintainabil"]},
 "afterRestore":{"absent":0,"pending":0,"scores":14,"warns":[]}}
```

另一方向（状态端点无答案 + 概览已回，合成）：

```
{"synthetic_loadingNoStatus":{"absent":0,"pending":0,
  "warns":["状态端点 读数中，下面退回概览端点里的 expertStates（两份内存锁同源但可能不同步）。"]}}
```

这一条里 `pending` 为 0 是对的：概览端点已经给出十四维，`leagues` 走的是真实回退链而不是空表，所以每一维都有行可渲染；只有两个端点都没回时才出现「读数中」。三段合成读数跑完即还原，`afterRestore` 即还原见证。

## 5. 仪器教训：`visibilityState: hidden` 让离场节点永不消失

同一枚「后端未回该维」标签的 class 实测长这样：

```
el-tag el-tag--info el-tag--small el-tag--plain el-zoom-in-center-enter-from el-zoom-in-center-leave-from el-zoom-in-center-leave-active
```

即它**已经被判定要移除**，但 `document.visibilityState === 'hidden'`（browser-use 无视口）使 transition 永不结束、节点不 detach。于是"读 `innerText` 找缺陷"会把正在消失的旧状态读成当前状态——本轮就差点把这条 transition 残留当成"页面同时说两种话"的缺陷写进结论。判据：**DOM 文本审计必须过滤 `classList` 含 `leave` 的节点**，或改用状态层（pinia state）复核。

## 6. 回归

- `npx vitest run src/modules/governance` → `Test Files 1 passed (1) / Tests 26 passed (26)`（`governance-contract.test.js` 现场解析编排器 Rust 路由表，26 例）。
- `python scripts/gate/check-frontend-module.py` → `前端模块化门禁：ERROR=0`。
- `python frontend-ui/scripts/gate/check-api-binding-kinds.py --check` → `verdict=PASS（零容忍：函数型导出被当对象用 0 处）`。
- `python frontend-ui/scripts/gate/check-ep-feedback-imports.py --check` → `verdict=PASS (探针：债务多少不改 rc)`。
- SFC 模板改动的编译证据不是 build 日志，而是 §4：页面按新分支重新渲染出了「读数中」/「缺 10 维」两种新文案，模板确实被编译并在跑。

## 7. 只报不改（本轮发现，未动）

1. `store/governance.store.js:43-49` 的 `dimensionRows` **全库零消费者**（`grep -rn dimensionRows frontend-ui/src` 只命中定义与 store 自身的导出行），且它是 `views/…vue:229 leagues` 同一逻辑的第二份实现，两者对"缺维"的口径即将分叉；它还写成 `expertsStatus.value.business.experts`（无可选链），与 `absentDims` 的写法不一致。退役＝删代码，要点名文件。
2. `PUT /api/governance/config/rbac`、`PUT /api/governance/config/experts`、`GET /api/governance/ws`、`POST /api/governance/assess`（含 `assess-single`/`assess-batch`）在本页没有出口；十条路由里页面只用五条 GET。已在 §5.13 的清单里，本轮只复核未变。
3. `npx prettier --check` 在本仓库**不可用**：`package.json` 带 `"type": "module"`，而 prettier 配置文件是 `.js` 里用 `module.exports` ⇒ `Invalid configuration for file ... module is not defined in ES module scope`（本轮原文输出）。所以前端格式没有任何门禁，改完不会被 prettier 兜底，也别把 `prettier --check` 当成验收命令。

## 8. 联盟域可达性账与一条 HMR 假阳的结案（2026-09-28 收束轮，文档见 §5.22）

同一台页面、同一个 XHR 记录器（patch `XMLHttpRequest.prototype.open/send`）扩到整个联盟域；网关 `:3080` 在跑，全部真后端数据，手写值 0 处。

### 8.1 路由总账（`router.getRoutes()` 现场读）

- 记录数 **86**；重名 **0**。
- `meta.module || meta.moduleName` 直方图：`(none) 51 / expert 11 / ai 7 / project 5 / admin 4 / graph 3 / market 2 / workflow 2 / operators 1`。
- `(none) 51` 的成因：`defineModule()` 的 children 只带 `meta.title`，不继承父模块名 ⇒ 归属要靠 `modules/*/index.js` 树形读，不能靠 `meta`。

### 8.2 十条联盟域路由的 GET 侧（去 `/api/health` 后的业务请求数，全部 200）

| 路由 | 业务 GET | 关键读数 |
|---|---:|---|
| `/expert-workspace` | 15 | `experts` 13279B · `experts/sessions` 7500B · `expert-graph` 9865B · `workspace/kpi` 297B |
| `/expert-plaza` | 3 | `experts/stats` 648B · `experts/bookings/mine` 372B |
| `/alliance/console` | 4 | `dispatcher/status` 1216B · `alliance/tasks` 99B · `alliance/runtime` 148B |
| `/alliance/collab` | 0 | 按设计：本页只发 `POST /api/experts/<mode>`，挂载无读通道，实测文案为"请填写问题描述"的表单初始态 |
| `/alliance/graph` | 3 | `expert-graph` 9865B · `stats` 1478B · `communities` 3371B |
| `/alliance/sessions` | 2 | `sessions?page=1&page_size=20` 7500B · `sessions/stats` 667B，渲染"30 个会话 / 47 条消息，第 1 / 2 页" |
| `/alliance/orchestration` | 2 | `orchestration/stats` 332B · `history` 77B（空表，页面已声明进程内不累计） |
| `/alliance/experts` | 3 | `experts?page=1&page_size=24` 13279B · `stats` 648B |
| `/alliance/governance` | 5 | `dashboard` 2350B · `experts/status` 2289B · `config/experts` 411B · `veto/events` 92B · `audit/logs` 91B——与 §1 记录的字节数逐条相同，即 §5.20 修复后的回归 |
| `/expert-config` | 0 | 候选缺陷：文案称"实时预览 · 一键发布 / 配置版本 v1.0.0"，挂载零 GET，读通道契约待裁决 |

每条路由都另伴随 `GET /api/health 200 68B`（`/alliance/governance` 一次挂载捕到 2 次），做差额对照时先摘掉探针。

### 8.3 `—` 的结案：冷加载前后唯一自变量是组件实例

现场读取（重载前）：六个 `.ax-kpi` 全 `—`，同一次脚本里 pinia `allianceExperts.stats = {totalExperts:11, onlineExperts:11, busyExperts:0, offlineExperts:0, totalConsultations:3561, todayConsultations:1, avgRating:4.363636363636363, avgResponseMinutes:5, satisfactionRate:0.8363636363636364, domains:[31 项], ts:"2026-09-27T23:32:50Z"}`、`loading.stats === false`。

排除项（各自有读数）：
- 离开节点残留：`/leave/` 祖先链命中 0，6 格 `visible:true`；
- 纯函数：动态 `import('/src/modules/expert-alliance/contract/enums.js')` 后 `expertStatsCells(store.stats)` → `11 位 / 11 / 0 / 0 / 3561 次 / 1 次 / 4.4 / 5 分钟`，且 `sameRef:true`（喂的就是 store 里那份对象）；
- store 取错：视图 `useAllianceExpertsStore()` 的 id 即 `allianceExperts`；
- 键名错配：`expertStatsCells` 读的 8 个键在实测对象里全部存在。

冷加载同一 URL（`#/alliance/experts`）后的读数：

```
kpiCount=6
11 位 平台专家 | 11 / 0 / 0 在线 / 忙碌 / 离线 | 3561 次 累计咨询 | 1 次 今日咨询 | 4.4 平均评分 | 5 分钟 平均响应
stats.ts=2026-09-27T23:35:00Z  loadingStats=false  noteShown=true
```

⇒ 判为**长命 dev 会话的 HMR 残留实例**（本轮在该页面做过多次 HMR 编辑），`computed` 挂在旧实例上没重算；**前端代码零改动**。

纪律（比 §5 更靠前一条）：在反复 HMR 过的页面上断言"UI 显示 A 而状态是 B"之前，**先冷加载该 URL**。这是唯一能分开"探针会话脏了"与"应用真有病"的动作；本轮若跳过，就会去修一个不存在的渲染缺陷。

## 9. 遗留专家广场的假数据显示现场（文档 §5.23，2026-09-28 同一轮）

`GET /api/experts` 的首条元素键集（页内 `await import('/src/api/index.js')` → `getExperts()`，返回非数组、长度 11；**列表键本身未点明**——探针用的是 `items‖experts‖data` 兜底链，与被审对象同病，故此处只登记元素键，登记列表键需另做一次点明）：

```
availability avatar bio capabilities created_at domains enabled expert_type
hourly_rate_cents id languages metadata metrics name organization
pricing_model skills tags timezone title type updated_at verification_status
```

`processExperts()`（`src/views/expert/ExpertPlazaView.vue:831-850`）16 个读取键的存活对照：

```
present = {consultCount:false, goodRate:false, responseTime:false, avgRating:false,
           price:false, online:false, recommended:false, favorited:false, hot:false,
           isNew:false, skills:true, capabilities:true, description:false, bio:true,
           avatarEmoji:false, type:true}
```

冷加载 `#/expert-plaza` 后的渲染读数（`/leave/` 祖先链过滤后）：

```
cardCount=11
firstCard="测 测试专家·云帆 自定义专家 · 专家联盟 ⭐ 0.0 (0 次咨询) K8s 微服务
           0 参与项目 0.0 用户评分 0.0% 好评率"
expertStats=["0 参与项目","0.0 用户评分","0.0% 好评率", … 共 6 条同形]
mentions0=11
heroLabelsPresent=[false,false,false]   // 入驻专家 / 累计咨询 / 平均响应 三个 label 均不在页面文本里
```

同题对照（同一台浏览器、同一批后端数据）：`#/alliance/experts` 显示 `11 位 平台专家 / 11 · 0 · 0 / 3561 次 累计咨询 / 1 次 今日咨询 / 4.4 平均评分 / 5 分钟 平均响应`（§8.3）。

死读通道证据：`git show HEAD:frontend-ui/src/views/expert/ExpertPlazaView.vue | grep -n heroStats` → 5 行（定义 `:673` ＋ 四条自赋值 `:684-687`），**无模板命中**；而 `loadStats()` 在 `:1096` 的挂载 `Promise.all([loadExperts(), loadStats(), loadMyBookings()])` 里确实发出 `GET /api/experts/stats 200 648B`。原始信封：`{"code":0,"msg":"ok","data":{"avg_rating":4.363636363636363,"avg_response_minutes":5.0,"busy_experts":0,"domains":{…}}}` ⇒ `expert_count / consult_count / good_rate / avg_response` 四键均不存在（`avg_response` 仅作为 `avg_response_minutes` 的前缀出现在全文子串里，子串探针会假报命中）。

本轮零代码改动。裁决项（退役遗留广场 vs 逐出口迁移到 `normExpert` 口径，及 `好评率/平均响应` 缺每专家源、卡片格标题"参与项目"与 consultCount 不同题）见文档 §5.23.3。

## 10. `/expert-center` 四面板补账与两轮 walk 对照（文档 §5.24，2026-09-28）

第一轮 walk（每路由等待 2.5 s，`window.__log`，总耗时 16.1 s）：

```
/expert-center/overview      reqs=[ai/chat/history/sess_… 31B, expert-graph 9865B, experts?page_size=100 13280B, experts/overview 1418B]
/expert-center/enterprise    reqs=[]  text=与总览逐字相同   ← 假象
/expert-center/orchestrator  reqs=[]  text=与总览逐字相同   ← 假象
/expert-center/tasks         reqs=[]  text="联盟任务 描述目标…全部 (0)…正在检查任务服务…"
```

第二轮 walk（每路由等待 6.5 s，`window.__log2`，改用面板唯一标记＋`.page-container` 计数＋`is-active` Tab 三个结构信号）：

```
/expert-center/enterprise    markerPresent=true ("企业级专家管理控制台")  pageContainers=1  activeTab="[企业管理]"
                             reqs=[总览那 4 条]（自身三条挂在面板内层子标签，未下钻）
/expert-center/orchestrator  markerPresent=true ("V2 编排引擎控制台")      pageContainers=1  activeTab="[编排引擎]"
                             reqs=[orchestration/stats 332B, orchestration/plugins 2369B, orchestration/history?limit=20 77B] + 总览 4 条
```

⇒ 四个 Tab 全部真实换景，第一轮读到的"不换景/零请求"是**等待窗口不足**（Vite dev 现场拉模块图，`ExpertEnterprisePanel:473` 引入 echarts）。控制台在本轮 reload 后会话内 **0 条 error/warn**（17 条消息，全是 `[Pinia:*] direct` 与 vite 性能组），也排除了异步块加载失败这一解释。

新账（唯一实收）：总览面板被 `ExpertCenterView.vue:34` 以 `v-show` 常驻，切到企业管理后 6.5 s 窗口内仍发 4 条总览请求（≈24.6 KB）⇒ 看不见的 Tab 在持续付费。修法属遗留外壳生命周期改动，与 §5.23.3 同族裁决，本轮未改。

探针写法记录：`evaluate_script` 有 15 s 上限，"导航＋等待＋读 DOM"串行脚本会被截断（**返回超时但脚本自己会跑完**）⇒ 改成一次调用只启动并把结果写进 `window.__logN`，下一次调用再读，本次两轮 walk 都是靠这个写法拿全的。

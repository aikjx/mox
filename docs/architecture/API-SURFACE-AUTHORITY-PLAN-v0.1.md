# API 表面单源化与企业级收口计划 v0.1（API-SURFACE-PLAN-V0.1）

> 层定位：L2 架构层 · 状态：**待评审**（本文件只提出计划与验收判据，未改任何生产码）
> 上层入口：[文档中心](../README.md) · 结构规范：[ARCHITECTURE-OF-DOCS.md](../ARCHITECTURE-OF-DOCS.md)
> 本层索引：[README.md](./README.md) · 端口权威：[PORT-REGISTRY.md](../api/PORT-REGISTRY.md)
> 仪器：`scripts/gate/check-api-surface.py`（P1a 普查器）· `scripts/gate/classify-unregistered-endpoints.py`（§1.3 调用者分类器，已落库含 `--selftest`，故意未接 CI）· 账：`reports/data/api-surface-census-2026-09-30.json`（立项图像）、`reports/data/api-surface-census-2026-10-01.json`（宽化后复跑）与 `reports/data/api-surface-census-2026-10-02.json`（并发作者再动装配后的第三次复跑）
> 调用者侧账（§1.3）：`reports/data/api-unregistered-callersite-2026-10-01.json`（rev4，含词频第二口径）、`reports/data/api-unregistered-callersite-2026-10-02.json`（rev6，判决口径，驱动未落库）与 `reports/data/api-unregistered-callersite-2026-10-02-landed.json`（rev7，落库门禁复算，判决与 rev6 逐条相等）

---

## 一、为什么要做这件事（本轮实测，非引用）

网关的对外 API 清单今天有**两个互不相同的源**：

1. **声明侧**：`platform/gateway/mox-platform-gateway-svc/src/actuator.rs:423` 的
   `pub static ROUTES: [ApiRoute; 243]`（这一档长度本轮就动过一次：`236 → 242 → 243`，
   所以引用它请引普查工件键 `routes_declared_len` 与它的 `generated_at`，不要引本文这句话——
   本文下面 §1.1 那张表就是这个数的逐次读数）；
   `docs/API-REGISTRY.md` 由 `scripts/doc/gen-api-registry.py` 从这张表生成，CI 有 `gen + git diff --exit-code` 门禁
   （`.github/workflows/ci.yml` 的 "Verify API registry is up to date" 步骤）。
2. **装配侧**：`build_gateway_router` → `build_host_router` → `modules::build_module_routers` → 各域 builder，
   逐层 `.route()` / `.nest()` / `.merge()`，其中域 builder 有一部分住在别的 crate（如 `mox-kb-svc`）。

CI 门禁只保证 **1 == 文档**，从来不检验 **1 == 2**。生成脚本还在文档正文里写死了
"声明即实现：表中每一条都有对应源码注册与真实 handler，不存在纯占位条目"
（`scripts/doc/gen-api-registry.py` 输出行），这句话没有任何尺子验过。

### 1.1 本轮量出来的差额

下表左列由 `scripts/gate/check-api-surface.py --census` 于 2026-09-30 23:13 印出，
逐条读数存于 `reports/data/api-surface-census-2026-09-30.json`；当时解析器自检 9 例全绿（`--selftest`，rc=0）。

同一轮内更早还有一次读数（约 22:50），两次之间 `actuator.rs` 被并发作者改动
（`git status` 为 `MM`、mtime 23:12:03），`ROUTES` 表长 236 → 242；
而"挂载未在册"两次都是 138。**声明侧与装配侧同时在动，而没有任何尺子看这两者的差**——这正是要做 P1 的理由。
本文件按 23:13 那次读数写数，引用任何这些数字前请重跑 `--census`。

23:24 复跑并覆盖写了那份 JSON（`--census --json`），逐字段比对旧图像：9 个 measurement 键**零变动**
（`scanned_files` 1370 / `mounted_normalized` 370 / `routes_parsed` 242 / 差额 138 全部仍在），只多出两键。
因为那次覆盖写暴露了一件事——`--json` 是覆写的，而旧工件**不带运行时刻**，重跑一次就再也说不清它是
哪一刻的分母。所以自 23:24 起工件自己署名：新增 `generated_at`（含时区偏移）与 `scan_roots`（点名扫描集，避免"分母凭手感"）。
跨了午夜之后账名也要对上时刻：10-01 的复跑另存一份 `reports/data/api-surface-census-2026-10-01.json`
（`generated_at` = `2026-10-01 13:57:08+0800`），而 09-30 那份**还原成 23:13 的图像**（9 键、无 `generated_at`——
那个键当时还不存在），免得文件名说一个日期、内容说另一个日期。上表的出处因此挂在印出它的那两份文件内部，而不是文件系统的 mtime。

| 量 | 09-30 23:13 | 10-01 13:57（宽化后复跑） | 10-02 00:01（第三次复跑） | 出处键 |
|----|----|----|----|----|
| 扫描 .rs 文件数（扫描集：gateway/domains/foundation/shared/projects，不含 tests/） | 1370 | 1372 | 1375 | `scanned_files` |
| 在册装配节点（按名字 build_* ＋ 按签名 `-> Router` 新增） | 181（只按名字） | 71 ＝ 名字 60 ＋ 签名独有 11 | 72 ＝ 名字 60 ＋ 签名独有 12 | `assembly_nodes_indexed` / `indexed_by_name` / `indexed_by_signature` |
| 真走进去过的装配节点 | 未记账 | 54 次 / 47 个，其中只靠签名才在册的 1 个（`domain_router`） | 55 次 / 48 个，只靠签名在册的 2 个（`domain_router`、`protected_kb_router`） | `entered_total` / `entered_unique` / `entered_signature_only` |
| 从装配根解析出的挂载路径（字面量 / 参数归一后） | 未记字面量 / **370** | 377 / **373**（含 2 条兜底 `/**`） | 379 / **375**（含同一 2 条兜底） | `mounted_literal` / `mounted_normalized` |
| ROUTES 表声明长度 = 实际解析条目 | 242 = 242 | 243 = 243 | 243 = 243 | `routes_declared_len` / `routes_parsed` |
| 在册但解析不到挂载 | 2 | **0**（那 2 条按保守口径与兜底前缀配上对） | **0**（配对仍 2/2） | `in_table_not_mounted` / `catch_all_paired_with_table` |
| **挂载但未在册** | **138** | **138** | **140** | `mounted_not_in_table` |
| 解析不了而点名的表达式（UNRESOLVED） | 2 | **0** | **0** | `unresolved` |
| 走进去发现是状态升级壳（不挂路径，也不许记成盲区） | 未记账 | 26 次 | 26 次 | `passthrough_shells` |

第三列的"签名独有"是我这轮重算出来的：工件里 `indexed_by_signature` 印的是**按签名匹配的总数**（10-01 为 30、10-02 为 31），
它与按名字在册的 60 个有重叠，所以分解必须写成 `72 ＝ 60 ＋ 12` 才能自证闭合。
本文件此前那一格印的是"71 ＝ 60 ＋ 30"——那个等式不闭合（60 加 30 是 90），属于分解不复算的账，已按三份工件的同一组键改正。

三份工件各自署名：`reports/data/api-surface-census-2026-09-30.json` 是 23:13 那次图像（9 键，无 `generated_at`——
那个键是 23:24 才加的，此份按 23:24 复跑逐字段验过零变动后原样还原）；
`reports/data/api-surface-census-2026-10-01.json` 是宽化后的复跑（20 键，`generated_at` = `2026-10-01 13:57:08+0800`）；
`reports/data/api-surface-census-2026-10-02.json` 是第三次复跑（同一 20 键，`generated_at` = `2026-10-02 00:01:19+0800`）。
**跨前两次运行，声明侧从 242 涨到 243、装配侧从 370 涨到 373，而差额钉在 138 不动**——
并发作者一直在补登录与企业级管理面的字面量，但补的都是已在册的那批。
**14:05:47 又独立复跑了一次**（只把 JSON 写到仓库外的临时路径，没有覆盖 10-01 那份在册工件）：
两份除 `generated_at` 外 **20 个键逐项相同**（含 `mounted_not_in_table` 那 138 条的逐项序列、
`in_table_not_mounted=[]`、`unresolved={}`）⇒ 138 不是"这一轮恰好读到"的量，而是同一图像上的可复现读数；
`--selftest` 同轮 18 例 FAIL 0，第 8 枚变异体（撤壳豁免）仍把壳读成盲区。

**"钉在 138 不动"这句话在 10-02 00:01:19 被现量证否了，这里如实记下**：那份工件的 20 个键与 10-01 逐键比，
变的是 6 个（`scanned_files` 1372→1375、`mounted_literal` 377→379、`mounted_normalized` 373→375、
`assembly_nodes_indexed` 71→72、`entered_total` 54→55 与 `entered_unique` 47→48、`indexed_by_signature` 30→31，
其中后两键是同一件事的两面），不变的是声明侧 243 = 243、`in_table_not_mounted` 空、`unresolved` 空、壳计数 26。
**差额因此从 138 涨到 140**，新增的两条是 `/api/kb/documents/{P}/shares` 与 `/api/kb/documents/{P}/shares/{P}`，
且反向为空（10-01 那 138 条一条都没消失）。归因是可直接指认的：`entered_signature_only` 从 `[domain_router]`
变成 `[domain_router, protected_kb_router]`，即并发作者新加了一个"签名返回 Router 但名字不像 builder"的 KB 受保护路由装配节点，
普查器靠签名口径把它走进了账本。**所以 138 从来不是规律，只是那两个小时窗口的读数**；
引用本节任何一个差额前都要重跑 `--census`，并把 `generated_at` 一起抄下来。

**同一窗口内的第四次复跑（时刻 `2026-10-02 00:57:48+0800`，JSON 只写仓库外临时路径，没动在册工件）**：
20 个键除 `generated_at` 全等，`mounted_not_in_table` 那 140 条**逐条相同**（对称差为空集），
`in_table_not_mounted` 仍空、`unresolved` 仍空、壳计数仍 26。
这条见证的意义不是"140 是规律"（上一段刚说过它两小时前还是 138），而是：
§1.3 那台新落库的分类器读的挂载侧权威（00:01:19 那份工件）在 §1.3 写完的当刻**没有过期**，
两份账读的确实是同一张装配图像——比较口径＝`mounted_not_in_table` 集合的对称差，逐条比而不是比长度。
按 10-02 图像重算下面结论句的比例：140 / 375 ≈ **37.3%**（漏账比例对窗口不敏感，138/370 与 138/373 都是 37% 量级）。

140 条的构成按前三段路径由 `reports/data/api-unregistered-callersite-2026-10-02.json` 的 `by_domain` 键复算，
**11 个域逐域相加恰为 140**（这条分解第一次写下来时漏了 `/actuator/api/*` 那 2 条，128 加 10 并不等于 140，
现量复算才发现并改正——改正后为 128 ＋ 12 ＝ 140）：
`/api/enterprise/*` 109、`/api/auth/*` 8、`/api/system/*` 8、`/api/tenant/*` 3，
零散 12 条＝`/actuator/api/*` 2、`/api/admin/*` 2、`/api/ai/*` 2、`/api/alliance/*` 2、`/api/kb/*` 2、
`/api/audit/export` 1、`/api/scheduler/status` 1。

138 条（10-01 及更早窗口）的构成同上，只是零散里没有 `/api/kb/*` 那 2 条；
那条口径此前把 `/actuator/api/{id}/enable|disable` 写成"等"里的一笔零散，按域复算它其实是 2 条独立端点。

抽样已用第二把尺子复核（不是只信解析器）：

- `/api/auth/login` 真挂载于 `platform/gateway/mox-platform-gateway-svc/src/system/mod.rs:361`
  （`.route("/api/auth/login", post(auth_session::login_handler))`，全路径字面量，无 nest 前缀），
  而 `actuator.rs` 全文搜 `auth/login` **命中 0 条**——登录端点不在"唯一权威清单"里。
- `/api/enterprise/admin/audit-logs` 由 `src/enterprise/admin_api.rs:404` 的 `.route("/audit-logs", ...)`
  经 `enterprise_features.rs:134` 的 `.nest("/admin", admin_router)` 与 `modules.rs:201-202` 的
  `.nest("/api/enterprise", ...)` 拼出；`actuator.rs` 全文搜 `audit-logs` **命中 0 条**。

**宽化那一刀差点把账做坏，这里如实记下**。为了把"条件装配的另一支"读出来，10-01 这轮把装配节点判据
从"函数名像 `build_*router*`"换成"签名返回 `Router`"，并让一支表达式里的**每个**装配调用都被走到（此前只跟第一个），
同时把 `.fallback()` 认成兜底挂载。第一次现量：371 → **429**（+58），看着像"终于读出了 48 条 `/api/alliance/*`"。
逐条查出处才发现这 58 条**全是仪器造假**：`fn upgrade<S>(router: Router<()>) -> Router<S>` 的形参叫 `router`，
而别的 crate 里确有 `pub fn router(...) -> Router`，于是走 upgrade 的壳时把外来的树整个挂进了网关表面——
那些 alliance 端点属于独立 svc（3100/3200），网关只通过 `/api` 兜底代理转过去，并不自己挂载它们。
补上"调用位点"判据（末段标识符后面必须是 `(` 或 `::<…>(`，且前面不能是 `.`）后落回 **373**，
比 09-30 只多那 2 条兜底与 1 条并发新增；`domain_router` 仍在"真被走到"的账上（`entered_signature_only`），
**它带来 0 条新路径**——"角色分桶的表面对 All 桶是子集"这句从此是量出来的，不是假设的。

这五件事各配了一枚代码级变异体（不是改夹具、是把判据本身换成弱化版再 exec 一份）：
只跟第一支 ⇒ 漏 else 支；只按名字在册 ⇒ 漏 `domain_router`；撤兜底识别 ⇒ 落 UNRESOLVED 而不是静默少一条；
撤调用位点判据 ⇒ 外来的 `/foreign-not-mounted` 挂进来；撤壳豁免 ⇒ 把壳读成盲区。
自检 09-30 为 9 例，10-01 复跑 **18 例全绿 rc=0**，10-02 00:15 同轮再跑仍是 **18 例 FAIL 0 rc=0**
（末例＝变异体 8 撤壳豁免仍把壳读成盲区）。

**结论**：所谓"全部 API 的唯一权威清单"今天漏掉了约 **37%**（09-30 为 138/370，10-01 为 138/373，10-02 为 140/375 ≈ 37.3%）的对外表面，
其中包含登录、企业级管理面（租户/部门/角色/权限/审计）与调度器状态。上一轮维护者口头报的"236 vs 309"差额
本身是仪器假账——309 只数了 gateway 一个 crate 的字面量，既没解析 nest 前缀，也没看域 svc crate 的挂载。


### 1.2 这不是"代码没写完"

同一轮里的其他读数：workspace 149 members 全部解析、生产源码里 `todo!()`/`unimplemented!()` 实质为 0
（45 处命中里 42 处在测试文件内）、Rust 测试函数约 6.6K。**缺口在治理链，不在实现量。**

### 1.3 挂载未在册那 140 条的调用者账（裁决点 2 的决策辅助）

裁决点 2 问的是"这 140 条里有没有本就不该对外的"。装配树答不了——它只能说"挂着"，不能说"有人在用"。
所以补了第三侧证据：**前端今天到底调不调这些路径**。账落
`reports/data/api-unregistered-callersite-2026-10-02.json`（rev6，`generated_at` = `2026-10-02 00:07:30+0800`），
它的挂载侧权威由工件键 `authority_for_mounted_side` 指向 `reports/data/api-surface-census-2026-10-02.json`
（同一窗口，不会出现"今天的差额配昨天的调用者"这种错配）。
同一份挂载权威在 `2026-10-02 00:52:16+0800` 由**已落库的门禁**复算了一遍，落在
`reports/data/api-unregistered-callersite-2026-10-02-landed.json`（工件键 `instrument` 点名驱动脚本、`revision` 7）；
两份的判决集合逐条相等，本节表格因此可以按"两台仪器同一读数"引，而不是按我的手感引。

量法写全，是为了让评审能复算，而不是信我的绿：

| 环节 | 判据 | 为什么必须这样（都是本轮真实撞过的坑） |
|---|---|---|
| 语料分档 | `frontend-ui/src` 下 `.js/.ts/.vue/.jsx/.tsx`，**测试档与非测试档分开**（实测 271 / 82 个文件，工件键 `corpus_src_files` / `corpus_test_files`） | 禁令台账里有一枚合成样例：`frontend-ui/src/modules/expert-alliance/contract/forbidden-revival.test.js:129` 的 planted 数组含 `http.post('/api/ai/expert-chat', body)`。它证明的是"这条端点被明令禁止复活"，不是"有人还在调它"。不分档就把它读成活调用者（rev1 犯过，登记在工件 `revisions` 键） |
| `baseURL` 双形态 | 业务实例 `frontend-ui/src/api/http.js:13` 写作 `baseURL: '/api'`；Actuator 独立实例是 `frontend-ui/src/api/http.js:324` 的 `createHttpInstance('/actuator')` | 前端字面量是 `/auth/login`，挂载侧是 `/api/auth/login`：只按全路径探会把绝大多数端点判成"无人调用"，这条规则登记在 rev4 工件 `reports/data/api-unregistered-callersite-2026-10-01.json` 的键 `baseurl_relative_rule`（原文："两种形态都探"；rev6 没带这个键，属于工件自述的一处退步，登记在此）。反过来 `/actuator/api/{P}/enable`（同族 `disable`）两条在前端写作 `/api/${id}/enable`（`frontend-ui/src/api/actuator.api.js:40`），恰是靠这档才命中 |
| 调用位点四类 | 工件键 `判据正则` 存着四条正则原文：`url: '…'` 声明式、"接收者名以 http／axios／request／service／api／instance 收尾"的动词调用、任意 `.getX(`、裸 `get('…')` | 判决取四类中除 `any` 之外的三类（`decl ∪ httpish ∪ bare`），**`any` 只报数不判活**——它会把 `map.get('/some/key')` 这种 Map 取值算成调用（该形状有对照样例盯着）。裸调用这一档在 rev6 上是**坏的**：正则字符类多一枚字面右方括号，于是只有落在文件 0 号位的调用才开火，而真实代码里 `fetch(` 前面永远是空白或箭头符号 ⇒ rev6 的 `bare` 读数恒 0（判决未受影响，逐条对读见下文"牙"一节），落库版已修并配夹具针 |
| 常量展开 | 同文件 `const NAME = '/x/y'` 之后把 `` `${NAME}/z` `` 补成 `/x/y/z` 再探（src 档展开 10 处＝工件键 `constant_expansions_src`，test 档 0） | 这一刀是**判决性**的：载体 `frontend-ui/src/api/sso.api.js:13` 的 `const BASE = '/enterprise/sso'` 让 6 条 `/api/enterprise/sso/*` 端点从不命中全部转 LIVE（例证 `frontend-ui/src/api/sso.api.js:16` 的 ``http.get(`${BASE}/protocols`)``） |
| 参数化端点不许用父前缀判 | 整条路径翻成参数化正则（工件键 `endpoint_pattern_样例` 存了一条实样），末段锚在引号或行尾 | 截到第一个 `{P}` 去探＝把**父资源**的调用者算到子端点头上。rev5 判 LIVE 的 `/api/kb/documents/{P}/shares`、`/api/system/user/{P}/depts`、`/api/alliance/tasks/{P}/qa` 三条正是这种假阳。rev6 把两档并报送（`live_endpoint_count` 21 对 `live_parent_count` 25），多出的 4 条逐条点名在 `parent_only_paths`，不静默。其中 qa 那条有第二口径反证：`frontend-ui/src/views/expert/AllianceTaskView.vue:422` 的注释明写"后端 registry 无 POST /alliance/tasks/:id/qa（原 askAllianceTaskQa 已随假端点撤除）" |

判决（全部读数出自 rev6 那份工件，时刻 `2026-10-02 00:07:30+0800`）：

| 判 | 条数 | 构成与证据 |
|---|---|---|
| 有端点级调用者 ⇒ **该入册并对外** | 21 | `/api/auth/*` 8 条（`frontend-ui/src/api/auth.api.js:20/33/64/76/103/104/105/106`）、`/api/enterprise/sso/*` 6 条（`frontend-ui/src/api/sso.api.js`）、`/api/tenant*` 3 条（`frontend-ui/src/api/system.api.js:131/132/136`）、`/actuator/api/{P}/enable` 与同族 `disable` 共 2 条、`/api/ai/engine/flow-graph` 与 `/api/alliance/sediment` 各 1 条（`frontend-ui/src/api/ai.api.js:63/66`）。每条在工件 `rows[].endpoint_evidence` 里带 `[类] 文件:行号 原文` |
| 只有父前缀命中 ⇒ **判假阳，不算调用者** | 4 | 即上表末行那 4 条 |
| 只出现在禁令台账的合成样例里 ⇒ **网关挂着、前端被明令禁止调用** | 1 | `/api/ai/expert-chat`（工件键 `test_only_paths`）。这一条不是"要不要入册"的问题，而是"网关该不该继续挂一个被联盟契约禁掉的端点"——归 P1 的判决范围之外，需单独立项 |
| 零调用者证据 | 119 | 工件键 `zero_live_endpoint`。含 `/api/enterprise/*` 109 条里的 103 条、`/api/system/approval*` 7 条、`/api/admin/users/{P}/sessions*` 2 条、`/api/scheduler/status`、`/api/audit/export` |

**第二口径（不按端点、按词频）**在 rev4 那份工件里（`reports/data/api-unregistered-callersite-2026-10-01.json`，
时刻 `2026-10-01 23:56:08+0800`，键 `domain_word_frequency_second_channel`）：整个前端语料里 `enterprise` 一词只出现 **11** 次、
`scheduler` **0** 次、`audit` **3** 次，而网关给 enterprise 面挂了 109 条端点。
两把尺子（按端点配对、按词频计数）口径不同但方向一致 ⇒ "企业级管理面 109 条基本没有消费者"这句不是单尺子的话。
rev6 没有复算这一键，所以引用它请连"它是上一版工件的读数"一起引。

**这台仪器的牙（如实登记）**：
- **rev6 的驱动没有落库**（仓库外临时目录的 `unreg_classify_rev6.py`），当时**没牙、不许接 CI**；
  它当时的复算入口是 `python <该文件> reports/data/api-surface-census-2026-10-02.json reports/data/api-unregistered-callersite-2026-10-02.json`（cwd 必须是仓根，工件路径是相对路径；语料遍历用的是绝对根，这是 rev3.5 崩过一次之后改的）。
- 装机器件自带三道装载期断言（非空判据正则集、`frontend-ui/src/api/auth.api.js` 必须在语料里、`/auth/login` 正对照必须开火），
  任一不满足即 rc=1 且不写工件；rev3.6 那次还撞出"分母硬钉 300"的手感断言，已改成结构断言。

**已落库为常驻门禁（本节写完的同一轮，取代上一段"转成常驻门禁是本轮之后的活"）**：
`scripts/gate/classify-unregistered-endpoints.py`。三种模式——`--selftest`（夹具＋变异体）、
`--census PATH --check`（只验仪器自身不变量，债务按咨询口径打印、rc 不由债务决定）、
`--census PATH --json PATH`（产账，产账前先跑八条不变量，任一红即拒绝写出）。
复算入口（cwd＝仓根）：

```
python scripts/gate/classify-unregistered-endpoints.py --selftest
python scripts/gate/classify-unregistered-endpoints.py --census reports/data/api-surface-census-2026-10-02.json --check --show-lists
python scripts/gate/classify-unregistered-endpoints.py --census reports/data/api-surface-census-2026-10-02.json --json reports/data/api-unregistered-callersite-2026-10-02-landed.json
```

落库这一刀**当场抓出一枚 rev6 的针洞**，这就是"没落库的仪器不算证据"的实证：

- 缺陷：`bare` 那类正则的字符类写成了 `[^\w.]]`——类本身在第一个 `]` 就闭合，多出来的那枚成了**字面 `]`**，
  于是整条判据实际要求 `]fetch(` 这种形状；唯一还能开火的路径是 `(?:^|…)` 的 `^` 支，即**只有位于文件 0 号的裸调用才被数到**。
  真实前端代码里 `fetch(` 前面永远是空白或 `=>`，所以 rev6 的 `bare` 档在真语料上恒为 0。
- 影响面量出来了，不是猜的（两份工件对读：rev6 时刻 `2026-10-02 00:07:30+0800`，
  落库版时刻 `2026-10-02 00:52:16+0800`，同向对比脚本读数只在 stdout）：
  **判决一条没变**——`mounted_not_in_table_len` 140、`live_endpoint_count` 21、`live_parent_count` 25、
  `zero_live_endpoint` 119、`test_only_paths` 1 条、语料 271／82、展开 10／0 全等，
  端点档 LIVE **集合**逐条相等（`parent_only_paths` 四条同）；变的是宽档的 `bare` 计数
  （`/actuator/api/{P}/enable` 与同族 `disable` 各 0→2，`/api/alliance/tasks/{P}/qa` 的 parent 档 0→1）。
  ⇒ 上一节那张判决表**不需要改写**，但 rev6 报出的 `bare` 读数不可引用。
- 顺带量出**第四种调用形态**（此前任何档都没真数过它）：两处直接用 `fetch()` 打绝对 `/api/…` 字面量、绕过 `http` 实例的位点——
  `frontend-ui/src/api/alliance.api.js:58`（`/api/experts/debate`）与 `:151`（`/api/alliance/tasks/${id}/logs/stream`，流式）。
  这两条路径都不在本次那 140 条里（现量：按 `/api/experts`、`/api/alliance/tasks` 前缀在未在册清单里各查得 0 条与 1 条，
  那 1 条即 qa 那条父前缀假阳），所以它们不是"漏判的活调用者"，而是一条**登记**：
  以后凡是绕过 `http` 实例的调用，只可能靠 `bare` 档进账，`bare` 坏掉就等于这一族整体隐身。
- 夹具里钉死了这一枚：`--selftest` 的 T1 现含 9 例形状，其中
  `const f = () => fetch('/misc/bare-call')`（`fetch` 不在 0 号位）期望 `bare = 1`——把正则改回 `[^\w.]]` 这格必红。

`--selftest` 的 15 例（9 正对照／不变量＋6 变异体）逐例读数同样只在 stdout，复算命令即上面第一条；
本节先前要求的那四枚变异体已全部落地并各自开火：合档语料 ⇒ `/api/ai/expert-chat` 转 LIVE（M1）、
撤 `baseURL` 双形态 ⇒ auth／sso／actuator 四档掉回数点名（M2）、撤常量展开 ⇒ sso 模板拼接判零命中（M3）、
退回父前缀判决 ⇒ `parent_only` 变空且 4 条假阳被静默收下（M4）；另两枚是本轮新增的判据面：
撤尾锚 ⇒ `/alliance/tasks/${id}/logs` 的调用者被算到列表端点头上（M5）、把 `any` 放进判决 ⇒ Map 取值算成调用者（M6）。
变异体是"撤通道"型的参数翻转（不是改源码），所以它们证明的是**判决挂在通道上**，不是"源码永不变坏"。

**仍未接线**：这道门禁故意不进 `scripts/gate/check-all.ps1`、不进 CI——
判决口径要等裁决点 2 与 4 落定，否则门禁会把待裁决项判成缺陷（`--check` 因此只把仪器自身不变量决定 rc，
债务条数按咨询口径打印）。

### 1.4 装配侧自己也有"两个源"：从未被挂上的 builder（比"未在册"更严重一档）

同一轮里顺着 §三 那一行"幻影对外面"扩到全仓量了一遍（驱动＝仓库外临时目录的 `dead_builders_scan.py`，
读数时刻 `2026-10-02 00:27:20+0800`；判据＝函数签名返回类型里出现 `Router`、名字以 `router|routes` 结尾，
"引用数"由全仓 `*.rs` 单次正则统计，跳过 `target/ .git/ node_modules/ legacy/ _archive/`）：

- 扫到 `.rs` 文件 **6768** 个，装配函数 **69** 个名字／**92** 处定义（同名多定义是允许的，按处数计）。
- **引用数 ≤ 定义处数**（即除了自己的定义之外没人叫它）的只有 **3** 个，全在 `mox-platform-gateway-svc`：

| 定义处 | 体内 `.route("` 处数 | 状态判定 |
|---|---|---|
| `src/system/approval.rs:386` `build_approval_router()` | 7 | **有等价挂载**：`src/system/mod.rs:530-536` 挂同 7 条 `/api/system/approval*`、用同一批 `approval::` handler ⇒ 这是装配侧的第二份副本，删掉不改变对外面 |
| `src/api_permission/api.rs:494` `build_api_permission_router<S>()` | 10 | **完全不可达**：体内是 `/endpoints`、`/permissions`、`/roles`、`/users/:id`、`/quick-grant`、`/check`、`/stats` 等相对路径；这些 handler 在全仓只被这个死 builder 引用一次（例 `list_endpoints_handler` 出现 2 次＝定义＋该 `.route`）。它需要的 `Arc<ApiPermissionState>` 却**已经**接进 `FromRef`（`src/lib.rs:161`）并被 `enterprise_features.rs:84` 的中间件用来自动登记端点 ⇒ 状态与中间件在线，路由表没接 |
| `src/batch_operation/api.rs:248` `build_batch_operation_router<S>()` | 5 | **完全不可达**，同上：`/templates`、`/templates/:id`、`/import/preview`、`/export/template`、`/stats`；`Arc<TemplateState>` 同样已接 `FromRef`（`src/lib.rs:155`） |

**这条为什么重要**：§1.1 那 140 条讲的是"挂了但没登记"，而这两族（**15 条**端点字面量）是"实现了、状态也接好了、但没有任何一处把它挂进路由树"——
所以它们既不在 `ROUTES` 表里，也不在装配树里，任何一把只扫挂载或只扫声明的尺子都永远看不见它们。
企业级口径上这是**功能缺失**，不是文档漂移：API 权限点管理与批量操作／导入导出模板两个管理面今天对外不可达。

**对手术量的影响（裁决点 1 要重算）**：§四 那 328 处字面量里含这 3 个死 builder 的 **22** 处（7 ＋ 10 ＋ 5），
所以 (B) 路线"把装配侧字面量统一走登记装饰"的真实手术量是 **306 处**（328 − 22；口径与 §四 同一条 grep，
差值由本节三行 `.route("` 计数现量给出，复算＝`grep -c '\.route("' ` 于那三个文件）。
死 builder 里的 22 处该删不该接，属裁决点 2 的邻域：**approvers 那 7 处直接删**（有等价挂载），
`api_permission`／`batch_operation` 那 15 处要么补 `.merge()` 接线（＝新功能上线，需产品点头），要么连模块一起退役。

**这台仪器的牙**：这台**驱动仍未落库**（仓库外临时目录的 `dead_builders_scan.py`）⇒ 没牙、不许接 CI；
§1.3 那台的驱动本轮已经落库，并且落库当场抓出一枚 rev6 的正则针洞（见该节"牙"一节），
所以"未落库的仪器不算证据"这句在本仓不是口号，是有账面后果的。它的形状判据有两处已知盲区要登记：
(a) 分母按名字后缀筛（`router|routes`），所以 69 个名字里含 `protected_kb_router` 这类"不像 builder 但确实返回 Router"的函数，
   而 `fn endpoints_api() -> Router` 这种不带后缀的名字**根本不在这把尺子的语料里**——它与 §三 那行"按签名在册"是同一课，
   常驻化时必须复用普查器的签名判据而不是另起一套名字判据；
(b) "引用数 ≤ 定义处数"把**只在测试里被调用**的 builder 也算成有引用，所以它给的是"疑似死码"而不是"确证死码"，
每一处都必须人工看一眼第二口径（本节的第二口径＝handler 的引用数与 `FromRef` 接线状态，逐条已在表里点名）。

---

## 二、分阶段计划（每阶段自带验收判据；不跨阶段夹带重构）

### P1a 普查器（已完成，本轮落库）

- 落点：`scripts/gate/check-api-surface.py`，模式 `--census` / `--selftest` / `--check`。
- `--check` **故意拒绝执行**（rc=2 并打印理由）：判据的覆盖面按形状算，
  在 §3 的盲区清单归零或逐条定性之前把数字钉成门禁，会把"解析不了"读成"没有缺陷"。
- 验收（已达成）：`--selftest` 9 例全绿，其中 4 例是变异体/正对照，且每一例都对应一个本轮真实撞出的缺陷形状：
  撤 `nest` 前缀必须改变结果、断 `let` 绑定必须点名、`let` 绑定但从未 merge 的 Router 不得计入对外表面、
  模块路径头名（`actuator::build_actuator_router()`）不得与同名 `let` 变量混淆。
- 状态：**普查器有牙（自检可被打红），但没有判决权（未接 CI，`--check` 未开）**。

### P1 路由单源化（下一步，需评审通过后开工）

目标：**声明侧从装配侧导出**，而不是两条平行账互相对抄。

- 动作 1：把 `ROUTES` 的生成方式改为可选两条路径之一，评审时二选一：
  - (A) 保留 `ROUTES` 为手写，但把 `r(...)` 条目的 `path` 字段与启动时实际装配结果交叉核对，
    差异写进 `/actuator/mappings` 的一个 `drift` 区块（运行态可见）。
  - (B) 在装配链上加一层登记装饰（`.route` 统一走一个记录宏/函数），启动时 dump 出 ROUTES，
    手写表退役为纯元数据（id/layer/domain/description 仍手写，path/method 由装配侧供给）。
- 动作 2：`scripts/doc/gen-api-registry.py` 的"声明即实现"这句话，改由判据承担；
  删不掉就把它降级为"本表由 X 生成，与装配树的核对见 Y"。
- 动作 3：`scripts/gate/check-api-surface.py --check` 开启双向判决（在册未挂载 → 红；挂载未在册 → 红），
  接入 `.github/workflows/ci.yml` 的 Architecture contracts job。
- 验收判据：
  1. `--check` rc=0 且双向差额均为 0 或被显式豁免清单点名（豁免必须打印条数与理由）；
  2. 逐枚变异体打红：删一条 `r(...)` 条目、把一条 `.route()` 改名、把一条 `.nest()` 前缀写错——三型各红一格且点名正确；
  3. `python scripts/gate/check-script-paths.py` 与 `python scripts/gate/check-frontend-module.py` 保持 rc=0；
  4. 文档重跑 `python scripts/doc/gen-api-registry.py` 后 `git diff --exit-code -- docs/API-REGISTRY.md` 通过。
- 风险：动 `actuator.rs` 的 ROUTES 会让 `docs/API-REGISTRY.md` 一次性大幅变长（10-02 图像上是 +140 条，见 §1.1 第三列），
  属于**预期内的补账**，不是回归；评审需先确认这 140 条是否都该对外（部分可能是内部/未完成面，
  §1.3 已经把"今天有前端调用者"的 21 条与"零证据"的 119 条分开了，可直接按那份账裁决），
  那就要走"标记 status=internal 并从对外清单剔除"的显式登记，而不是悄悄不写）。

### P2 前端治理装置接入 CI（本轮已接线，待维护者提交）

接线前现状（本轮实测）：`frontend-ui/scripts/gate/` 下 14 枚门禁脚本 + 1,125 个 vitest 用例 + `vite build`，
在 5 个 workflow 里**一处都没被引用**；`scripts/gate/check-locale-format-outlets.py` 在 `AGENTS.md` 自称"CI 门禁"，
实际只被 `scripts/gate/check-all.ps1` 调用。

已落地：`.github/workflows/ci.yml` 新增独立 job `frontend-governance`（不塞进已有编号步骤，
`yaml.safe_load` 复算 jobs=10、该 job steps=10），步骤为
`pnpm install --frozen-lockfile` → `pnpm run test`（vitest）→ `pnpm run build`（rollup 才是消费者账的验针）→
locale 口径门禁（`--selftest` + 体检）→ 5 枚前端探针 → 一枚显式打印未接线清单的步骤。

本轮逐枚实测的命令形态与判决（决定接哪几枚的唯一依据）：

| 探针 | 实测 | 处置 |
|------|------|------|
| `check-api-binding-kinds.py --check` | rc=0，`verdict=PASS（零容忍…0 处）` | 已接 |
| `check-ep-feedback-imports.py --check` | rc=0（探针型，债务不改 rc） | 已接 |
| `check-framework-imports.py --check` | rc=0，非测试源 0 处用而未绑 | 已接 |
| `check-theme-tokens.py --check` | rc=0，`CHECK new=0 verdict=PASS` | 已接 |
| `check-sfc-dead-refs.mjs --selftest` | rc=0，`cases=4 passed=4` | 已接 |
| `check-view-hex.py --check` | **rc=1**（`grown=1 new=1 textasfill=33`），且 `--selftest` FAIL=4 | **未接**，job 内打印 BLOCKED 与实测 rc |
| `check-import-reach.mjs --selftest` | **rc=1**（`stores/index.js 被判可达`的形状） | 未接，同上 |
| `check-route-links.mjs --selftest` | **rc=1**（`/alliance/console` 在路由表出现 2 次） | 未接，同上 |
| `check-locale-format-outlets.py` | 只认 `--selftest` 与无参；`--check` 是 unrecognized（rc=2） | 按无参写法接入 |

未接线三枚**不用 `continue-on-error` 蒙绿**，而是由 job 的最后一步把名字与实测 rc 打进日志。

验收状态：
1. 三门禁复跑回到本轮介入前的基线，证明新增未引入新账——
   `check-script-paths.py` rc=0 且"CI run 判缺"仍 2（第一版写法把它顶到 8，因为该门禁按
   `defaults.run.working-directory` 解析 run 文本里的路径，且会把 echo 里的裸文件名也当引用；
   改成仓根相对写法 + 不在 echo 里写裸文件名后归位）；
   `check-doc-links.py`：23:26 复跑时断链仍 43 条/9 文件、WARN 总体 212 → 213 → 212，
   中间那 +1 是**本文件自己造的一条悬空引用**（指向 docs/expert-alliance/FRONTEND-MODULE.md，
   该路径不存在，权威实际只在 _archive/v1/ 下），改指真实路径后归位。
   10-01 这轮复跑（扫描 393 文件 / 判定引用 2334 条）：**断链 0 条、rc=0，WARN 总体降到 207**——
   那是并发作者把存量断链补掉了，不是本文件的功劳；本文件此刻在 WARN 明细里命中 0 行
   （本段原先用反引号引用那条坏路径，门禁把说明文字里的反引号也当引用读，故此处一律不加反引号）。
   `check-frontend-module.py` ERROR=0。
2. **未本机验证项（挂起给维护者）**：`pnpm install/test/build` 三步本轮未在此窗口运行
   （5 个 node 进程在跑，且 `packageManager` 钉 pnpm@11.15.1 而本机 PATH 是 8.6.5）。
   合并前必须本地预跑：`cd frontend-ui && pnpm install --frozen-lockfile && pnpm run test && pnpm run build`，
   并把结果与权威用例数对照：该账目前只在 `docs/expert-alliance/_archive/v1/FRONTEND-MODULE.md` §8（"验证"节，:372），
   而 live 规范是 `docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md`——
   "权威复述源躺在归档里"这件事本身是 P3 要收的口（见下），不一致以复算为准。


### P3 文档权威收敛

- `docs/CORE-CAPABILITIES.md` 今天仍写 8080 唯一入口 / 98 条路由 / 7 域（2026-09-06 冻值），
  与 `docs/api/PORT-REGISTRY.md`（3080）和 P1 之后的 API-REGISTRY 三处冲突。
- 动作：冻值段落整体归档进 `docs/_archive/`，原位只留指针；同一事实只保留一个权威源。
- 验收：`python scripts/gate/check-doc-links.py` 不得新增悬空。
  本文件与其索引行落盘后于 09-30 23:16 复跑：rc=1，断链 43 条/9 文件、反引号引用 WARN 总体 212 条
  （打印截断至 120）⇒ 当时未引入新悬空。
  **10-01 14:04:09 复跑（扫描 393 文件／判定引用 2334 条）：rc=0、断链 0 条、WARN 总体 206 条
  （打印仍截断至 120），且明细里本文件名命中 0 行**——本文件没有新增悬空，这一条是本轮的验收结论。
  但 09-30 23:16 的 43 条断链到 10-01 14:04 归零**未被归因**，不要读成"有人修好了"：
  `git log --oneline -3` 最新仍是 `962ab70e`（提交时刻 09-30 01:26），没有任何提交能解释这批清零，
  而同一工作树里 `??`／`M` 条目数在两轮之间自己就变了（见 P4）⇒ 只能记账为「工作树内漂移，来源未查」。
  212→206 的 6 条差额同理不逐条归因：本轮确实改掉过本文件的三处悬空引用（改指 `_archive/v1/FRONTEND-MODULE.md`、
  把说明性旧路径的反引号摘掉），但"截断打印 + 并发作者"下不能把差额全算在自己头上。
  同轮复核 P3 的前提仍然成立：`docs/CORE-CAPABILITIES.md` 第 20/61/98 行**今天确实还写着**
  8080 唯一入口 / 98 条路由 / 7 域（`grep -n 8080` 现量三处命中）。
  写后自证：上面这批数取自**本段落盘之前**的那次运行，故 14:05:12 复跑一遍，
  扫描 393 文件／判定引用 2334 条／WARN 总体 206 条／rc=0／本文件命中 0 行——逐项不变
  ⇒ 引用的读数描述的是当前图像，不只是写前图像。

### P4 工作树收口（**需用户点名，勿代跑**）

10-01 14:04:20 `git status --porcelain` 计数：**710 条**，拆到两列状态码为
`??` 237 / `A ` 163 / `M ` 114 / ` M` 67 / `AM` 56 / `MM` 36 / `R ` 33 / `RM` 2 / `D ` 2
（合计 710，自闭合）。同一条命令在 09-30 记的是 610 条、本会话早些时候记的是 708 条——
差额全部落在 `??` 与 ` M` 两档，即工作树仍在被别人并发写入；因此这三个数都只是**快照**，
引用时必须连同上面的时刻一起读，不许当作"当前值"复述。
另：该命令在 stderr 打印了 3 条 `could not open directory 'tests/regression/*/runs/latest/'`
（目录名存在但不可打开），这些路径不进上面的计数，收口时属另一类议题。
`frontend-ui/src/modules` 于 14:04:49 现量：磁盘 126 个文件 / 索引 104 / HEAD 仅 13
（09-30 记的磁盘 125 已过期，一个文件之差同样来自并发写入）。

- 约束：本仓有并行提交者且配了 Git content filters，**禁止 `git add -A` / `commit -a` / `stash`**；
  只按名点自己改过的文件。P1/P2 产物（普查器、CI job、方案文档）待评审通过后由维护者提交。
- 本轮四件工件的在册状态（10-01 14:08:07 `git status --porcelain` 按名点这四条）：
  本方案文档、普查器 `scripts/gate/check-api-surface.py`、两份 `reports/data/api-surface-census-*.json`
  **全是 `??`**（未入库，不是被忽略——它们能被 `git add`）。⇒ `??` 意味着**没有 git 恢复源**：
  评审通过后第一件事是按名提交这四件，否则任何一次误清理会连恢复源一起丢，本文件里的读数就再也对不回产出它的那份图像。

### P5 网关巨型 crate（结构性，仅登记不排期）

`platform/gateway/mox-platform-gateway-svc` 一个 crate 承载 313 条 `.route("` 字面量（10-01 14:07:24 现量，
排除 `tests/`；含 `.nest("` 则 328），`src/system/mod.rs` 单文件 38 条 `.route("`
（同轮第二口径：不带引号的 `.route(` 在该文件 67 条——把参数化/非字面量的调用位点也数进来了，两者不是同一个量，
此前"67 累计"的写法属口径混用，已按本轮实测收窄）。
这个 crate 的 `src/` 现为 100 个 `.rs`／38055 行。
六层里 gateway 层实际退化为路由堆。是否按域拆 router crate 属架构级议题，需单独评审。

---

## 三、普查器的覆盖面账：已收干的与仍盲的（不许被读成"没有缺陷"）

判据换过一次：覆盖面**按形状算**，每种形状要么有夹具＋变异体，要么在表里点名是盲区；
"扫了多少文件"不算覆盖面。10-01 复跑现量 `UNRESOLVED = {}`（0 条），10-02 第三次复跑仍为 0 条。

| 形状 | 09-30 的状态 | 10-01（含 10-02 复跑）的状态 | 见证 |
|------|------|------|------|
| `.fallback()` 兜底路由 | 落 `unresolved`（2 条），读成"看不见" | **已归因**：记成 `<前缀>/{**}` 入兜底账，并按保守口径与在册 `<前缀>/{P}` 配对（2/2，10-02 仍 2/2） | 夹具 `SHELL_FIXTURE` ＋ 变异体 6（撤识别必须落盲区）＋ 配对单测（多段参数留余账） |
| `if role == ... { A } else { B }` 条件装配 | 只走 A 支，B 支整支消失 | **已归因**：一支表达式里的每个装配调用都走 | `BRANCH_FIXTURE` ＋ 变异体 4（只跟第一支必漏 else）＋ 活语料 `entered_signature_only`（10-01 为 `[domain_router]`，10-02 为 `[domain_router, protected_kb_router]`） |
| `match` 臂、`upgrade(...)` 外壳里的裸 builder 调用 | 只取 `.route/.nest/.merge` 步骤，臂内树消失 | **已归因**：链步骤消费区间外的调用位点一并解析 | 变异体 5／7；`_steps_in` 消费区间防重复挂载 |
| 非 `build_*` 命名但返回 `Router` 的装配节点 | 按名字认，整类静默 | **已归因**：按签名 `-> Router` 在册；10-02 现量在册 72 ＝ 名字 60 ＋ 签名独有 12（按签名匹配的 31 个里有 19 个本来就按名字在册，分解要闭合请看 §1.1） | 变异体 5；`indexed_by_signature` |
| 状态升级壳（`upgrade` 的体只做 `with_state`） | 会被读成"无法归类的表达式"（26 条噪音） | **已归因**：判据从函数体本身推（无链步骤且无装配调用才豁免），豁免计数打印 `passthrough_shells` | 变异体 8（撤豁免必落盲区）；10-02 仍 26 次 |
| **从未被调用的 builder（幻影对外面）** | 未查 | **10-02 新撞出，普查器口径正确地不收**：`platform/gateway/mox-platform-gateway-svc/src/system/approval.rs:386` 的 `pub fn build_approval_router()` 写着 7 条 `/api/system/approval*` 字面量（388–393 行），而全仓 `build_approval_router` 的引用数只有定义本身那 1 处——真正挂载的是 `src/system/mod.rs:530-536`（同 7 条路径、同一批 `approval::` handler）。仪器只走可达根，所以这 7 条没进差额，**是对的**；危险在 P1 的 (B) 路线：若 ROUTES 改为"静态扫全部 builder 导出"，这 7 条会以幻影条目进对外清单 | 复算入口：全仓 `grep -rn build_approval_router`（期望命中数＝1，即定义本身）；这条形状目前**只有活语料证据、没有夹具**，挂 P1。全仓按同一形状量到 **3 处**（其中 2 处是完全不可达的管理面），见 §1.4 |
| 形参名与别处同名函数撞车 | —（尚未暴露） | **已判为假阳并拦下**：调用位点判据（末段后必须 `(`/`::<…>(`，前不能是 `.`） | `COLLIDE_FIXTURE` ＋ 变异体 7；现量证据＝本轮曾把 371 顶成 429 |
| 同名装配节点多定义 | 取首个并点名 | 现量 0 条（名字池污染收干后不再有歧义名被走到） | `unresolved` 键集为空 |
| 语句切分按顶层 `;` | 宏内/闭包内 `;` 可能切错 | **仍是盲区**，但撞出即以 `unresolved` 露出，不静默 | 无夹具：这条余账挂 P1 |
| 同一 role 分桶的独立表面（HostRole::Kg/Cloud/Kb/Iam 各跑一遍） | 未跑 | **未做**：本轮只证了 All 桶与 else 支的路径集合关系（else 支 ＋0 条） | 需要 `--role` 维度，挂 P1 |

---

## 四、评审需要的四个裁决点

1. P1 走 (A) 交叉核对还是 (B) 装配侧导出？(B) 更彻底，但要动网关侧全部字面挂载写法——
   10-01 14:07:07 现量 **328 处**（口径：`platform/gateway` 下排除 `tests/` 目录的 `*.rs` 里
   `.route("` ＋ `.nest("` 的**出现次数**，按出现计不按去重计；拆到 `.route("` 313 ＋ `.nest("` 15，
   且非测试出现全部落在 `mox-platform-gateway-svc` 这一个 crate 内），另涉 **42 个**源文件。
   同一条口径在 09-30/10-01 早间记的是 **329 处**——差 1 处未归因（工作树有并发写入者，见 P4），
   复核请原样重跑这两条 grep，不要引用本行的任一数字当"当前值"。
   `.route(` 不带引号的口径会更大（`src/system/mod.rs` 上 67 配 38），因为那还数了变量名/参数化的调用位点。
   **10-02 00:20 按同一口径原样复算：仍 328 处（`.route("` 313 ＋ `.nest("` 15，42 个文件）**——
   所以那"差 1 处"是更早窗口的读数，本轮没有再漂；这一格是 (B) 路线的真实手术量，可直接定价。
   **但 328 里有 22 处住在从未被调用的 builder 里**（§1.4 现量：`approval` 7 ＋ `api_permission` 10 ＋ `batch_operation` 5），
   按可达面算，(B) 的真实手术量是 **306 处**——接线前先决定这 22 处是删还是补挂，别把死码一起迁进登记表。
2. **挂载未在册的 140 条里，有没有本就不该对外的？**（条数取自 `reports/data/api-surface-census-2026-10-02.json`，
   时刻 `2026-10-02 00:01:19+0800`；10-01 及更早窗口上是 138 条，见 §1.1）
   §1.3 已经把决策辅助做出来：**21 条有前端端点级调用者**（该入册）、**119 条零证据**
   （其中 `/api/enterprise/*` 109 条里 103 条无人调用）、4 条父前缀假阳、1 条被禁令台账点名的 `/api/ai/expert-chat`。
   需要裁决的是三选一：(a) 按 §1.3 的账把 21 条先入册、余 119 条逐域过一遍再定 internal；
   (b) 先全量入册再逐条标 internal（对外清单一次性 +140，可读性下降但零遗漏）；
   (c) 把"有调用者但未在册"视为缺陷直接要求装配侧与声明侧同轮改（最严，但会把并发作者的在途改动一起卷进来）。
   本文件推荐 (a)。理由要说准：这 119 条**不是"未完成面"**——现量相反，`/api/system/approval*` 那 7 条由
   `system/approval.rs` 的真 handler 实现并经 `system/mod.rs:530-536` 挂载，`/api/scheduler/status` 有
   `scheduler/api.rs:692` 的真 handler。准确的描述是**有实现、有挂载、零消费者**（多半留给尚未接线的管理后台）。
   对这一类，全量入册等于让对外清单替"没人用也没人验收过"的面背书；逐域过一遍才能把
   "该对外却漏记"（auth 那 8 条就是）与"预留面"分开——这正是 §一 里那句没被尺子验过的"声明即实现"要防的事。
3. P2 接 CI 时，前端门禁的存量红是先清账还是走棘轮。
4. **从未被挂上的两族企业级管理面**（§1.4：`api_permission` 10 条 ＋ `batch_operation` 5 条端点字面量，
   状态与 `FromRef` 已接线、路由表没接）：补 `.merge()` 让它们真的对外（＝新功能上线，要过鉴权与验收），
   还是连模块一起退役登记？另 `system/approval.rs` 那 7 条是 `system/mod.rs` 的重复副本，本文件判"直接删"，
   若不同意请点名——留着它的代价是任何"按 builder 静态导出 ROUTES"的方案都会凭空多 7 条幻影端点。


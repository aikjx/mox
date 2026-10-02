# API 表面单源化与企业级收口计划 v0.1（API-SURFACE-PLAN-V0.1）

> 层定位：L2 架构层 · 状态：**待评审**（本文件只提出计划与验收判据，未改任何生产码）
> 上层入口：[文档中心](../README.md) · 结构规范：[ARCHITECTURE-OF-DOCS.md](../ARCHITECTURE-OF-DOCS.md)
> 本层索引：[README.md](./README.md) · 端口权威：[PORT-REGISTRY.md](../api/PORT-REGISTRY.md)
> 仪器：`scripts/gate/check-api-surface.py`（P1a 普查器，现有六个模式：`--census`／`--selftest`／`--check`／`--role-matrix`（§1.5）／`--role-surface`（§1.6）／`--ledger`（§1.7，核心公式单一算源））· `scripts/gate/formula_ledger.py`（**核心公式识别器的唯一归宿**：散文里"什么算一条算术主张"只在这里判一次，`--ledger` 与全库普查都调它；判据形状由全库实测归纳，四条规则与七种盲区写在文件头）· `scripts/gate/check-doc-formulas.py`（§1.8 全 `docs/` 公式普查，四个模式 `--census`／`--dircheck`／`--recognizer`（§1.8b 识别器单归宿账：仪器里不许有第二把算术尺）／`--selftest`，判决模式待裁决点 6（这条"待"自 §1.8f 起按通道实测而非打包宣称），`--selftest` 一侧已接 CI 且"已接线"本身由 夹具M 对着 ci.yml 复算，见 §1.8e）· `scripts/gate/classify-unregistered-endpoints.py`（§1.3 调用者分类器，已落库含 `--selftest`，故意未接 CI）· 账：`reports/data/api-surface-census-2026-09-30.json`（立项图像）、`reports/data/api-surface-census-2026-10-01.json`（宽化后复跑）与 `reports/data/api-surface-census-2026-10-02.json`（并发作者再动装配后的第三次复跑）· 角色面账（§1.5/§1.6）：`reports/data/api-role-matrix-2026-10-02.json`、`reports/data/api-role-surface-2026-10-02.json`（`--ledger` 按前缀取最新一份，不钉带日期的文件名）· 全库公式账（§1.8）：`reports/data/doc-formula-census-2026-10-02.json`
> 调用者侧账（§1.3）：`reports/data/api-unregistered-callersite-2026-10-01.json`（rev4，含词频第二口径）、`reports/data/api-unregistered-callersite-2026-10-02.json`（rev6，判决口径，驱动未落库）与 `reports/data/api-unregistered-callersite-2026-10-02-landed.json`（rev7，落库门禁复算，判决与 rev6 逐条相等）

---

## 一、为什么要做这件事（本轮实测，非引用）

网关的对外 API 清单今天有**两个互不相同的源**：

1. **声明侧**：`platform/gateway/mox-platform-gateway-svc/src/actuator.rs:423` 的
   `pub static ROUTES: [ApiRoute; N]`（这一档长度本轮就动过两次：`236 → 242 → 243 → 246`，
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
本文件此前那一格印的是「71 ＝ 60 ＋ 30」——那个等式不闭合（60 加 30 是 90），属于分解不复算的账，已按三份工件的同一组键改正。
（这一行用「」把旧值框起来是**约定**：框住的是引文（登记"某轮曾印错成什么"），不是本文件现在的账；
`--ledger` 的散文算术审计会跳过引文并把它按名点名，见 §1.7。）

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

**10-02 上午的时效复核**（引本节的 21／25／4／1／119 前先看这一句）：把同一份挂载侧权威
（`reports/data/api-surface-census-2026-10-02.json`，140 条）重新喂给落库的分类器，产出的 140 行**逐行逐字段相同**
（`rows` 里 0 行有差异，`by_domain`、`parent_only_paths` 也相同），而语料本身在这期间被并发作者加过：
源档 271→272、测试档 82→83。⇒ 判决对"加了两个文件"不敏感，但**对挂载侧权威敏感**——
换一份 census 就是换一个分母（10-02 上午现量：同一份装配的第五次 `--census` 复跑，20 个键除
`generated_at` 与 `scanned_files`（1375→1376）外逐位相同，140 条序列仍逐位相同）。
`--selftest` 同轮 15 例 FAIL 0。

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

### 1.5 对外面按 HostRole 归属：在册账根本承载不了这一维（`--role-matrix`，本轮新增）

**为什么要有这一维**：`deployment::domain_router` 按 `HostRole {All, Kg, Cloud, Kb, Iam}` 逐臂装配不同的子树
（`platform/gateway/mox-platform-gateway-svc/src/deployment.rs:20-31`，现量 5 个具名臂——含
`HostRole::All => unreachable!("all uses the existing module registry")` 那一臂，它不挂任何路径，
所以矩阵里只出现 4 个标签——再加 2 个无枚举路径的臂（`_ =>` 之类，只继承外层）），
而 `ApiRoute` 的字段现量只有 `id,method,path,layer,domain,status,description,enabled`（同文件 `actuator.rs:390-399`），
**没有一个能承载角色**。所以"部署成 KB 单机时对外表面是什么"这个问题，在册账（条数现量见托管块 `DECL-PARSE`）与 `docs/API-REGISTRY.md` 都答不了，
只能从装配侧读。读数：`reports/data/api-role-matrix-2026-10-02.json`（`generated_at` = `2026-10-02 10:14:47+0800`，
`api_route_role_fields = []` 就是上面那句的机器见证）。

**现量（同一次运行，别拆开来引）**：归一挂载 375 条里 **111 条带角色归属**，按臂标签
`HostRole::Iam 76 ／ Kb 18 ／ Kg 13 ／ Cloud 4`（四档相加 111 ＝ 有归属总数，且 `multi_dim_paths` 空 ⇒
真实语料里没有一个端点同时挂在两个臂标签下）；`ApiRoute` 声明长度 243，挂载未在册 140 条（与 `--census` 同口径）里
**带角色归属 29 条**。

**"独占 76／18／13／4"这一列不许被读成"这些端点只属于该角色"——本轮就是在这里差点读反**。三档分账给出的是：
同 site 111 ／ 不同 site 0 ／ 根本没有 0。含义是**每一条有归属的路径都另有一条不带臂的读数挂在同一个文件行号上**，
因为普查根 `build_gateway_router` 就是 `build_host_router(state, All)`（`lib.rs:265-266`），
而 `lib.rs:289` 的 `if role == All { build_module_routers(..) } else { domain_router(role, ..) }` 两支都会被走
（§三 那行"条件装配两支都要走"就是这件事）——**同一棵子树既在 All 的模块注册表里挂过，又在某个角色的臂里挂过**。
所以"角色"是**走树路径**的属性，不是挂载位点的属性；这一维目前能量的是
"哪些子树是在按角色装配时挂上去的"，**不能**量"KB 单机部署对外少哪几条"。
后者要求按角色参数化各走一遍（把 `role` 当常量传入并只走匹配的臂），登记为下一台仪器，挂 P1，不在本轮做——
本轮如果把它当成"每角色独立表面账"来引，就是一次假阳。

**下一台仪器的设计约束（本轮现量把一条看似省事的路线判死了）**：不许用"标签剪枝"实现按角色分桶。
理由是 `unlabeled_paths` 现量＝**375＝全部**，而"无标签"这个桶同时装着两件不相干的事：
`if role == All` 的**注册表支**（单角色部署根本不走）与**角色无关的公共面**（每个角色都走）。
两件事现量可分：位点在 `lib.rs:289` 那条 `if` 之前的只有 **4 条**
（`/health`、`/api/v1/status`、`/api/v1/domains`、`/metrics`），其余无归属读数的位点在各自的 builder 文件里
（按文件计 Top5＝`api.rs` 100／`mod.rs` 67／`alliance.rs` 21／`handlers.rs` 18／`projects_ext.rs` 14 条），
**origin 里只有"文件:行号"，没有走树路径**，所以从现有读数根本分不出某条无归属读数来自注册表支还是臂外的链。
按标签剪枝会把那 4 条公共面一起剪掉，印出"KB 单机对外面比全量少 375−18 条"这种假数字。
正确形态是先把 `if <cond> { A } else { B }` 这一支表达式**按分支位置归因**（A 支记成 `cond:role==All`、
B 支记成 `cond:role!=All`，机制与既有 `match` 逐臂判据同构），再问"该角色的桶里有哪些路径"：
公共面（在 `if` 之外）每个角色都保留，注册表支只在 All 保留，`match` 臂只保留标签与请求角色相符的那一支。
验收判据（接手者照此自证）：(i) `--census` 的 20 个键除 `generated_at`／`scanned_files` 外逐位不变；
(ii) All 桶读数应包含那 4 条公共面 ＋ 注册表支，且不含任何单角色臂的独占子树；
(iii) Kb 桶必须包含 `/api/kb*` 而不包含任何只在 `HostRole::Iam` 臂里的路径，且这一条要用**夹具**（不是活语料）先证；
(iv) 每加一档就配一枚"撤该档判据必须变红"的变异体（本轮 9–15 那七枚是模板）。

**这台仪器的牙**：`--selftest` 现量 **27 例 FAIL 0**（比 §1.3/§1.4 那批多 9 例，全部是本轮角色维带来的；
分支维落库后为 36 例、§1.7 台账闭合维落库后为 40 例、同节补上"渲染占位符"判据后为 41 例（14:52）、
§1.8 把识别器收成一份并为行号口径补针后为 **43 例**（15:20 现量）——27／36／40／41 都只在各自那一轮成立过）。
角色夹具 `ROLE_FIXTURE` 的形状是真语料的形状：`let router = match role {…}` ＋ 臂里 `upgrade(...)` 壳 ＋
臂里嵌套第二把 `match tier` ＋ 臂里跨函数调 builder ＋ 臂外一条**同前缀**的自由装配（造出同 site 孪生）。
逐条归属由 5 例正对照钉住，其余是"每撤一条通道对应判决必须变红"的变异体：
变异体 9 撤逐臂判据 ⇒ 整张矩阵空而挂载一条不少（证明归属是注解）；
10 撤跨函数继承 ⇒ 臂里调的 builder 体内挂载丢角色；
11 撤臂枚举 ⇒ 矩阵与臂账同时清空（枚举是标签的唯一来源）；
12 只取最内层臂 ⇒ 嵌套 match 的外层维被读丢；
13 撤模式边界 ⇒ 外层臂把嵌套 match 的臂名一起吃进来（`/cloud-x` 被凭空加上第二维）；
14 撤接收者展开 ⇒ `let` 绑定的 match 各臂整棵树消失；
15 撤同 site 判据 ⇒ 三档分账变红而路径集合一条没少。
`--role-matrix` 自己带 6 条仪器不变量（语料非空且臂读得出、有归属∪无归属＝总数、标签必须来自臂枚举、
角色专属⊆有归属、三档不重不漏、"根本没有"与 `role_only` 由两条独立代码算得同一集合、差额口径与普查账一致），
全部 PASS；**它故意不接 CI**——归属是注解，判决要等 §四 的裁决点。

**落库当场抓到的四处仪器自身问题**（都是"能跑、也全绿、但读数不对"那一类）：

1. **臂模式正则吃不下单字符枚举变体**：`(?:::[A-Za-z_]\w+)+` 要求 `::` 之后至少两个字符，
   于是 `CloudTier::A` 全程不匹配——夹具里嵌的那第二把 match 整支读成无归属，而真语料的臂名都是两字符以上，
   所以这个洞在活语料上**从未开过火**。改成 `\w*` 后 `/cloud-a` 才拿到第二维。
2. **臂标签把嵌套 match 的臂名一起吞了**：按整条臂体取模式，`HostRole::Cloud => match tier { CloudTier::A => … }`
   的外层臂标签变成 `CloudTier::A+HostRole::Cloud`，于是内层的 `_ =>` 臂（`/cloud-x`）被凭空扣上 `CloudTier::A`。
   补了顶层 `=>` 边界（`_top_arrow`），只从模式段取标签。
3. **链的接收者若绑在 `let` 上就整支消失**：`let r = match role {…}; r.merge(c)` 只解析 `.merge(c)`，
   各臂的树一条不挂。宽化为"链步骤照走 ＋ 接收者若在 env 里也展开一次"。
   **宽化的价：活语料 0 次命中**——`grep -rn "router\.merge(\|router\.nest(" platform/gateway --include=*.rs` 为空，
   真语料的尾项是 `router.route_layer(…)`，不匹配 `\.(route|nest|merge)\s*\(`，走的是"头标识符回查 env"那条既有递归。
   同轮复算见证：改前改后 `--census` 的 20 个键除 `generated_at` 与 `scanned_files`（1375→1376，并发作者新增一个
   不带路由的文件）外**逐位相同**，`mounted_not_in_table` 140 条序列相同 ⇒ 这一维与这次宽化都只动注解、不动判决。
4. **新增函数把 `def cmd_census(args):` 那行当锚点吃掉，函数体落成上一个 `return` 之后的死码**：
   `ast.parse` 照过、`--selftest` 27 例照全绿，只有 `--census` 报 `NameError: cmd_census`。
   教训＝改完所有入口模式都要各跑一遍（§1.5 那轮是四个，§1.6 起是五个：`--census`／`--selftest`／`--check`／
   `--role-matrix`／`--role-surface`），语法检查和自检都看不见"某个入口整个没了"。

另有一处**口径分裂**在本轮被抓平：`--role-matrix` 第一版的"挂载未在册"没减兜底配对项，印出 142 而 `--census` 印 140，
同一本账两把尺子；现在按 `ca_prefix` 配对同口径（不变量里现量印 `142 − 2 配 140`）。

复算入口：

```
python scripts/gate/check-api-surface.py --selftest        # 43 例，期望 FAIL 0（§1.5 那轮 27、§1.6 那轮 36、§1.7 那轮 40、§1.8 之前那轮 41，只在各自一轮成立过）
python scripts/gate/check-api-surface.py --role-matrix     # 只读账，期望 ROLE-MATRIX PASS
python scripts/gate/check-api-surface.py --census --json <仓库外临时路径>   # 与在册工件逐键比，判决不许动
python scripts/gate/check-api-surface.py --role-surface    # 按角色分桶的对外表面账，期望 ROLE-SURFACE PASS
```

### 1.6 分支位置归因：把对外表面按 `HostRole` 取值真正分桶（`--role-surface`，本轮新增）

§1.5 只把 `match role {…}` 的**臂**归了属，装配里还有第二种长相：`platform/gateway/mox-platform-gateway-svc/src/lib.rs`
的 `let protected = if role == HostRole::All { build_module_routers(…) } else { deployment::domain_router(role, …) };`。
`--role-matrix` 对它是盲的，所以那边印出的「角色专属＝0 条」不是"没有专属"，而是"分支这一维没进判决"。
本轮补的不是标签的第三种写法，而是**位置**：落在 `then` 块里的挂载叠上 `=HostRole::All`，落在 `else` 块里的叠上
`!HostRole::All`；`else if` 链的外层否定罩住**整条链**（链尾那一支自己再判自己的条件），复合条件（`&&`）不做布尔求解，
按名进 `cond_blind` 盲区账——归不出约束就登记为覆盖面缺口，不许静默当"这里没有分支"（那会把只在 All 下挂的端点读成公共面）。
标签仍只进 origin 串、不进 `mounted` 的键 ⇒ `--census` 与 `--role-matrix` 两本账逐位不动（本轮现量见证，见下）。

**分档只许由两条独立通道互相见证**：(a) 符号走树（两支都走、逐位点打约束标签、事后对每个取值求值合取）；
(b) 具体执行（把角色钉成一个取值，逐次走树前先把这一轮不会被走到的分支／臂就地填成空白，长度不变故行号不漂移）。
两通道同集合才算数——标签读出来而执行走不到，或反之，都必有一边错。

**企业级读数**（`reports/data/api-role-surface-2026-10-02.json`，`generated_at` 2026-10-02 11:01:04+0800，
键 `buckets_tag`／`buckets_substituted` 各档长度，1376 个 `.rs`、375 条归一挂载）：

| 角色 | 标签档 | 钉死档 | 仅此档 | 未在册 |
|---|---|---|---|---|
| All（融合部署） | 375 | 375 | 248 | 140 |
| Kg | 29 | 29 | 0 | 2 |
| Cloud | 20 | 20 | 0 | 2 |
| Kb | 34 | 34 | 0 | 4 |
| Iam | 92 | 92 | 0 | 29 |

两通道对称差五档全为 0（键 `cross_channel_symmetric_diff`）、任何角色都到不了的 0 条（键 `bucket_delta_never_reachable`）、
角色取值读自 `platform/gateway/mox-platform-gateway-svc/src/deployment.rs:6` 的枚举声明（非硬编，键 `role_variants_source.backfilled_from_labels` 为 false）。
分档闭合自证：带分支约束标签 359 条 ＋ 公共面 16 条 ＝ 归一总数 375 条（键 `branch_tagged_paths` ＋ `common_surface_paths` 的长度对 `mounted_normalized`）。
「仅此档」一列给出的是包含关系而非互斥关系：Kg／Cloud／Kb／Iam 四档各为 0，说明单角色子树的端点**全是融合面的子集**。
现量到形状：四档里各只有 16 条"单读数"路径，而那 16 条正是公共面（无标签）；余下的 Kg 13／Cloud 4／Kb 18／Iam 76 条
各有 **2 条**挂载读数，其合取一条是 `=HostRole::All`（注册表支）、另一条是 `!HostRole::All ＋ HostRole::Kg`（角色臂），
例：`/ai/engine/analyze`。⇒ 同一件事在两个位点各挂一次，读成析取才不永不可达（折成一条合取就是把这 111 条读成 0 条）。
而这 **111** 恰是 §1.5 里"有归属"那一条账：按臂标签数出的 111 条与按"双读数"数出的 111 条**是同一个集合**
（对称差 0，两条独立代码现算）⇒ §1.5 与 §1.6 数的是同一批挂载，只是一个看臂、一个看分档。
这一条不由标签自证，由钉死通道见证——All 档那次走树把 `else` 支整段剪空后仍得 375 条，臂里的子树必然另有注册表来源。
**公共面（至少一条挂载读数不带任何条件）＝ 16 条，占 375 的 4.3%**（键 `common_surface_paths`）：
即 L0 的 `/health`、`/api/v1/status`、`/api/v1/domains`、`/metrics` 加上 `/actuator` 全家 12 条——
除这四条状态端点与管理面之外，**网关没有任何一个业务端点是"角色无关"的**。
这条读数是 §四 裁决点 1 的新证据：ROUTES 表那整张表（条数现量见托管块 `DECL-PARSE`）按构造不带角色维，因此它对"某个角色实际对外暴露多少"
根本承载不了答案——单角色进程暴露的是 20–92 条而不是 375 条。分支判据吃进判决的量（撤了就会多算进来，
键 `branch_judgment_removed`）＝ Kg 346／Cloud 355／Kb 341／Iam 283、All 0（All 档本来就含全部）。

**这台仪器的牙**：落库那轮 `--selftest` 现量 **36 例 FAIL 0**（§1.5 那 27 ＋ 本轮 4 例正对照 ＋ 5 枚变异体）。
§1.7 台账闭合维落库后同一入口为 41 例 FAIL 0（在上句那 36 之上另加 5 枚台账变异体 21–25，14:52 现量），
§1.8 把散文算术的识别器收成一份、并为托管块行号口径与"本地不留第二把尺"各补一枚针（26、27）后为 **43 例 FAIL 0**（15:20 现量）——
这两个数各属各的那一轮，引用时要点名是哪一轮。
夹具 `COND_FIXTURE` 复刻真语料形状：根函数 `let protected = if role == All {注册表支} else {按臂分派}` ＋
臂里一条多模式臂（`HostRole::Kg | HostRole::Cloud`）＋ 一枚 `&&` 复合条件 ＋ 链尾自由装配；
另一枚 `CHAIN_COND_FIXTURE` 专测 `if / else if / else` 三段链。正对照逐档核对分桶、逐路径核对两族标签并集
（臂标签与分支标签各读各的通道，析取标签按 `|` 分段后再排序归一，不钉字面顺序），并现场量 `cond_spans == 2`、
`cond_blind == 1` 且盲区串里真的含 `&&`。变异体各归各的通道：
16 撤分支枚举 ⇒ 分支标签整族消失、注册表支凭空挂进四个单角色档而挂载一条没少；
17 把负支读成正支 ⇒ `else` 支与臂约束叠成永假，条目从**所有**档里凭空消失（撤错了方向的失败形态是"读小"而不是"读大"）；
18 撤剪枝区间 ⇒ 钉死取值五档塌成一档、两个剪枝计数同时归零；
19 撤标签分族（把分支挤进臂通道）⇒ 分支账归零而臂账被顶掉；
20 撤整条 else-if 链的罩住 ⇒ 链尾那支凭空出现在 All 档。
`--role-surface` 自带 **12 条仪器不变量**且全部 PASS，其中三条是这台仪器专用的：公共面必须每档都在
（拿标签剪枝分桶就是把这条剪没）、同一批公共面在钉死通道里也必须每档都在、分支判据只能收窄不能放宽
（带分支的档 ⊆ 只看臂的档）。**它同样故意不接 CI**，判决仍等 §四。

**落库当场抓到的四处仪器自身问题**（仍是"能跑、也全绿、但读数不对"那一类，活语料上多数从未开火）：

1. **`} else {` 永远匹配不上**：`span_from` 返回的是**内容**区间，其上界就是配对右花括号自己的索引，
   而 `ELSE_HEAD_RE` 从该下标起匹配 ⇒ 首字符是 `}`，`\s*else\b` 恒落空 ⇒ 负支约束整族消失，
   `--role-surface` 第一次跑出来是「带分支标签 0 条／公共面 375（100%）／五档全同」。改成从 `hi + 1` 起匹配后，
   活语料同轮 old-vs-new：分支区间入账 **1 处 → 2 处**（一个 `if/else` 的两支），带分支标签路径 359 条、
   五档读数、12 条不变量**全不变**——因为 else 支里的挂载本来就另由臂标签排除了 All，这一维补的是**账**而不是判决。
2. **Python 链式比较把守卫写成恒假**：`if len(role_vals) > 1 == len(uniq)` 展开是 `len(role_vals)>1 且 1==len(uniq)`，
   后者与前者的定义互斥 ⇒ 析取标签那条分支**永远不成立**，多模式臂一律落成 `+`（合取）＝ 永不可达。
   改成 `… > 1 and len(role_vals) == len(uniq)`。这一枚在活语料上也是 0 次命中
   （`grep -rnE "HostRole::[A-Za-z_]+\s*\|" platform/gateway --include=*.rs` 为空，装配语料里没有一枚多模式角色臂），
   所以它只能由 `COND_FIXTURE` 里那枚 `/multi` 路径钉住：夹具上它必须同时进 Kg 档与 Cloud 档，撤掉判据就两档皆空。
3. **臂模式正则只认 `=>` 左边那一个名字**：`([A-Za-z_]\w*(?:::…)+)\s*(?:\{…\})?\s*` 后必须紧跟 `=>` ⇒
   `HostRole::Kg | HostRole::Cloud` 只解析出 `HostRole::Cloud`，另一半凭空丢。补 `|` 作 lookahead
   （`…(?=\s*(?:\{[^{}]*\})?\s*(?:=>|\|))`）后，`/multi` 才拿到 `HostRole::Kg|HostRole::Cloud`。
   同轮复算：`--role-matrix` stdout 与改前**逐字节相同**、`--census` 的 20 个键除 `generated_at` 外**逐位相同**。
4. **计数器替没做的事记功**：`self.prune_spans += len(ap) + len(cp)` 写在填空白的循环**之前** ⇒
   变异体 18 把循环掏空后照样印「剪枝 6 处」。改成"记真正填掉的段数"，并按坐标基去重另算一列：
   现印「剪枝 31 次（同一区间会在函数体／表达式／整文件三个坐标基上各剪一次），去重后 26 段」
   （`prune_occurrences_by_variant` ＝ All 3、其余各 7；`prune_distinct_intervals_by_variant` ＝ 2／6／6／6／6，
   均见 `reports/data/api-role-surface-2026-10-02.json`）。31→32 与 32→31 的那一枚差额已逐处归因：
   修好 else 边界后 All 档不再走进 `domain_router`，deployment.rs 里那 4 枚臂剪枝换成 1 枚分支剪枝
   （4 → 1），其余四档不变 ⇒ 不是剪枝漏了，是少做了无用功。

`--selftest` 里还钉了一条**仪器不许只靠符号通道自证**的对照：夹具上标签求值分档与钉死取值分档必须逐档相同
（这正是活语料五档对称差 0 的那条判据，先在夹具上开火，否则它在语料上只是"恰好没红"）。

复算入口（六个模式各跑一次；本轮改的正是被 `--selftest` 看不见的那几个入口）：

```
python scripts/gate/check-api-surface.py --selftest                       # 43 例，期望 FAIL 0（落库各轮曾为 27、36、40、41）
python scripts/gate/check-api-surface.py --census --json <仓库外临时路径>  # 与 §1.5 那份逐键比：判决键不许动，`generated_at`／`scanned_files` 允许差（14:44 现量差这两键，1375→1377）
python scripts/gate/check-api-surface.py --check                          # 期望 rc=2（故意拒绝）
python scripts/gate/check-api-surface.py --role-matrix                    # 期望 ROLE-MATRIX PASS
python scripts/gate/check-api-surface.py --role-surface                   # 期望 ROLE-SURFACE PASS，12 条不变量全绿
python scripts/gate/check-api-surface.py --ledger                         # 期望 LEDGER PASS（红 0 项），见 §1.7
```

---

### 1.7 核心公式的单一算源：把本文件所有加法交给仪器复算（`--ledger`，本轮新增）

前面六节的公式是**散文**：每条"A ＝ B ＋ C"靠人工复算，第 33 条教训（"文档里写下加号就必须算加法"）
就是这样抓出来的——一处是别人留下的「71 ＝ 60 ＋ 30」（右边相加是 90），一处是我自己新写的"零散 10 条"漏数 2 条。
（本段刚写完就被这台仪器打回一次：我在下面第 2 层里把那句旧值写成了行内代码而不是引文，
`--ledger` 判它是一条主张并按行号点名——引文与主张的界碑要由仪器守，不能由作者记性守。）
人工复算的问题不在算错，在于**算过一次就不再是判据**：下一次改数的人看不见上一次那道核对。
`--ledger` 把这批公式变成一台仪器，三层各有牙：

1. **结构闭合**：在**一份活的图像**上现量复算每条等式的左值与加数（条数与红数由本节末尾托管块
   那一行印出，散文不重打一遍——重打就是第二个源，见 §1.8 把这条口径推广到全库）。
   所有行的值都从**同一个命名空间**取 ⇒ 同一个数被两行引用时改一处必然两行同时红，
   这就是"单一算源"的机械含义（反例：各行自带一份数字，就退化成第二个源）。
   表里有一条**反对照** `ASM-ANTI`：按签名匹配的那一批与按名字在册的那一批**有重叠**，
   所以那个总数**不是加数**；重叠数由该行的取值现推（候选之和减并集），
   把它当加数写正是 §1.1 那格旧值的错法，这条判据不许被"看起来更完整"的写法替换。
2. **引用对账**：每个数对回 `reports/data/` 下被引用的工件键（条数与分档同由托管块印出）。
   **键找不到＝引用不成立，判红**；值不同＝语料漂移，判 INFO 但**按名点名并附工件 `generated_at`**——
   落库那轮三条漂移全是 `scanned_files`（1375／1376 配当轮现量 1377），15:18 复跑是 6 条
   （`scanned_files` 到 1379，另加 `indexed_by_signature` 与 `passthrough_shells`），因为并发作者在这一天里持续动源。
   漂移不判红的理由要说清：判据的强度来自"等式闭不闭合"，不来自"语料不动"；
   把"语料不动"钉成门禁，红的会是台账而不是代码。工件按名字前缀取**最新**那份，硬编今天的文件名明天就红。
3. **文档镜像**：本节末尾的托管块必须等于现渲染（逐字节）。`--ledger --write` 只在
   "块外逐行核验"通过时才落盘（前段逐行相同 ＋ 后段逐行相同），写后回读再比一次；
   块外任何一行被碰 ⇒ 一个字也不写。`--ledger` 不带 `--write` 时逐行打印差异位置。
   托管块之外，散文里写出的加法链必须自己成立；写在 `「…」`／`“…”` 里的是**引文**
   （登记"某轮曾印错成什么"，如 §1.1 那句 `「71 ＝ 60 ＋ 30」`），跳过复算但按名点名——
   这与"禁令台账里的合成样例会被读成活调用者"同族：**仪器分不清引文与主张时，主张这一侧就不可信**。
   本轮起这条不复算自家的正则，而是调 §1.8 那份**唯一**的识别器（一份口径两处用，本地不留第二把尺）；
   落不进主张的形状按 reason 出账，而"块外主张数 ≥1"本身是断言——判集塌缩会把零证据印成满把握。

复算入口（cwd＝仓根，约 30–40 s，因为 `BUCKET/SUBST/PRUNE` 三组公式要按五个角色取值各走一遍树）：

```
python scripts/gate/check-api-surface.py --ledger          # 期望 LEDGER PASS（红 0 项）
python scripts/gate/check-api-surface.py --ledger --write  # 只在语料或判据变了之后跑；跑完必须再跑上一条回到 PASS
```

判据的牙齿由 `--selftest` 里的变异体 21–27 钉住：改一个加数必须让**引用它的那几行**同时红（`m_only_mount`
被 M-PART 与 UNREG-CROSS 两行引用，改一处 ⇒ 两行红）、公式 id 撞车必须红、planted 的假加法必须被散文审计抓到、
托管块被改一个数字必须与现渲染不一致、渲染里留下未替换的格式化占位符必须判红。
第 25 枚是**落库之后补的**：前四枚全绿、`--ledger` 也 PASS，是我自己读那张表才看见 `ASM-ANTI` 的说明列印成
"重叠 %s 条"——占位符没替换会被"逐字节相同的镜像"原样放行（两次渲染同样错，镜像只问两次是否相同），
所以这一族缺陷要逐条问"哪根针穿得到它"。修法：那个重叠数由本行的值现推（候选之和减去并集，现量 19 条），
并把"渲染不许留占位符"升成一条判决（变异体 25 的夹具就是往另一行的说明里塞一个 `%s`，期望它单独判红）。
**本模式仍未接 CI**：它裁决的是本文件的账，而账里的分母还挂在 §四 的五个裁决点上。

<!-- LEDGER:BEGIN 本块由 scripts/gate/check-api-surface.py --ledger --write 现量生成，勿手改（改公式改脚本） -->

本表由 `scripts/gate/check-api-surface.py --ledger --write` 在一份活的图像上现量复算，**不在表里的加法不算账**。
扫描 .rs 1384 个｜公式 30 条（红 0）｜引用对账 55 条：一致 28、漂移 27、引用不成立 0。
读数的三个来源工件：reports/data/api-surface-census-2026-10-02.json（2026-10-02 00:01:19+0800 生成）、reports/data/api-role-surface-2026-10-02.json（2026-10-02 11:01:04+0800 生成）、reports/data/api-role-matrix-2026-10-02.json（2026-10-02 10:14:47+0800 生成）

| 公式 | 现量等式 | 判定 | 说明／引用 |
|---|---|---|---|
| `ASM-CLOSE` 在册装配节点 ＝ 按名字在册 ＋ 只靠签名新增 | 73 ＝ 61 ＋ 12 | 闭合 | census:assembly_nodes_indexed／indexed_by_name／indexed_by_signature |
| `ASM-ANTI` 反对照：按签名匹配的总数**不是**加数（与按名字那批有重叠） | 73 ≠ 61 ＋ 32（=93） | 闭合 | 重叠 20 条 ⇒ 只按名字会读小 |
| `DECL-PARSE` ROUTES 表声明长度 ＝ 真解析出的条目数 | 249 ＝ 249 | 闭合 | census:routes_declared_len／routes_parsed |
| `M-PART` 归一挂载面 ＝ 在册∩挂载 ＋ 兜底前缀覆盖 ＋ 挂载未在册 | 378 ＝ 236 ＋ 2 ＋ 140 | 闭合 | census:mounted_normalized／mounted_not_in_table |
| `T-PART` ROUTES 表的不同路径数 ＝ 在册∩挂载 ＋ 在册未挂载 ＋ 由兜底解释 | 241 ＝ 236 ＋ 3 ＋ 2 | 闭合 | 注意分母是**去重后的路径**，不是条目数（一条路径多_method_算两次） |
| `TAG-COMMON` 归一挂载面 ＝ 带分支约束标签 ＋ 公共面 | 378 ＝ 362 ＋ 16 | 闭合 | surface:branch_tagged_paths／common_surface_paths |
| `TAG-DJ` 上面那两档必须互斥（同一位点既能无条件挂又被分支罩住＝读重） | 0 ＝ 0 | 闭合 | 两集交集现量 |
| `ONLY-ALL` 归一挂载面 ＝ 只在 All 档 ＋ 跨多档 ＋ 任何角色都到不了 | 378 ＝ 251 ＋ 127 ＋ 0 | 闭合 | surface:only_all_bucket／bucket_delta_never_reachable |
| `NEVER-0` 任何角色都到不了的路径现在必须为 0（不为 0 就是分档漏了一档） | 0 ＝ 0 | 闭合 | surface:bucket_delta_never_reachable |
| `LBL-PART` 有归属路径 ＝ 同 site 双读数 ＋ 不同 site ＋ 根本没有 | 111 ＝ 111 ＋ 0 ＋ 0 | 闭合 | matrix:same_site_twin_count／different_site_twin_paths／no_twin_paths |
| `TWIN-NONE` 「根本没有无归属读数」必须与 role_only 独立算得同一个数 | 0 ＝ 0 | 闭合 | matrix:no_twin_paths／role_only_paths |
| `LBL-SUM` 有归属路径 ＝ 各臂独占数之和（前提：跨两维以上的路径为 0） | 111 ＝ 111 | 闭合 | matrix:labeled_paths／exclusive_by_label |
| `LBL-DJ` 跨两维以上必须为 0，否则 LBL-SUM 会把同一条路径数两遍 | 0 ＝ 0 | 闭合 | matrix:multi_dim_paths |
| `UNREG-CROSS` 未在册差额在两台仪器上必须同一个数（普查账 ＝ 分档 All 档） | 140 ＝ 140 | 闭合 | surface:unregistered_by_bucket.All ＝ census:mounted_not_in_table |
| `XCHAN-DJ` 标签通道与钉死取值通道的分歧必须为 0（两本账互为见证） | 0 ＝ 0 | 闭合 | surface:cross_channel_symmetric_diff |
| `BUCKET-All` All 档：只看臂的档 ＝ 带分支的档 ＋ 分支判据吃掉的条数 | 378 ＝ 378 ＋ 0 | 闭合 | surface:buckets_tag_arm_only／branch_judgment_removed |
| `SUBST-All` All 档：标签求值得到的面 ＝ 钉死取值走树得到的面 | 378 ＝ 378 | 闭合 | surface:buckets_tag／buckets_substituted |
| `PRUNE-All` All 档：剪枝出现次数 ≥ 按坐标去重后的段数 | 3 ≥ 2 | 闭合 | surface:prune_occurrences_by_variant／prune_distinct_intervals_by_variant |
| `BUCKET-Kg` Kg 档：只看臂的档 ＝ 带分支的档 ＋ 分支判据吃掉的条数 | 378 ＝ 29 ＋ 349 | 闭合 | surface:buckets_tag_arm_only／branch_judgment_removed |
| `SUBST-Kg` Kg 档：标签求值得到的面 ＝ 钉死取值走树得到的面 | 29 ＝ 29 | 闭合 | surface:buckets_tag／buckets_substituted |
| `PRUNE-Kg` Kg 档：剪枝出现次数 ≥ 按坐标去重后的段数 | 7 ≥ 6 | 闭合 | surface:prune_occurrences_by_variant／prune_distinct_intervals_by_variant |
| `BUCKET-Cloud` Cloud 档：只看臂的档 ＝ 带分支的档 ＋ 分支判据吃掉的条数 | 378 ＝ 20 ＋ 358 | 闭合 | surface:buckets_tag_arm_only／branch_judgment_removed |
| `SUBST-Cloud` Cloud 档：标签求值得到的面 ＝ 钉死取值走树得到的面 | 20 ＝ 20 | 闭合 | surface:buckets_tag／buckets_substituted |
| `PRUNE-Cloud` Cloud 档：剪枝出现次数 ≥ 按坐标去重后的段数 | 7 ≥ 6 | 闭合 | surface:prune_occurrences_by_variant／prune_distinct_intervals_by_variant |
| `BUCKET-Kb` Kb 档：只看臂的档 ＝ 带分支的档 ＋ 分支判据吃掉的条数 | 378 ＝ 34 ＋ 344 | 闭合 | surface:buckets_tag_arm_only／branch_judgment_removed |
| `SUBST-Kb` Kb 档：标签求值得到的面 ＝ 钉死取值走树得到的面 | 34 ＝ 34 | 闭合 | surface:buckets_tag／buckets_substituted |
| `PRUNE-Kb` Kb 档：剪枝出现次数 ≥ 按坐标去重后的段数 | 7 ≥ 6 | 闭合 | surface:prune_occurrences_by_variant／prune_distinct_intervals_by_variant |
| `BUCKET-Iam` Iam 档：只看臂的档 ＝ 带分支的档 ＋ 分支判据吃掉的条数 | 378 ＝ 92 ＋ 286 | 闭合 | surface:buckets_tag_arm_only／branch_judgment_removed |
| `SUBST-Iam` Iam 档：标签求值得到的面 ＝ 钉死取值走树得到的面 | 92 ＝ 92 | 闭合 | surface:buckets_tag／buckets_substituted |
| `PRUNE-Iam` Iam 档：剪枝出现次数 ≥ 按坐标去重后的段数 | 7 ≥ 6 | 闭合 | surface:prune_occurrences_by_variant／prune_distinct_intervals_by_variant |

漂移只说明语料动了（并发作者天天改源），不说明账错：红判据是「等式不闭合」「引用键找不到」「托管块与现渲染不一致」三种。
<!-- LEDGER:END -->

---

### 1.8 「什么是一条核心公式」收成一份识别器，并用它量全库（`scripts/gate/formula_ledger.py` ＋ 全库普查 `--census`，本轮新增）

§1.7 只治了**这一份文件**，而同一族账（"总数 ＝ 甲 ＋ 乙"）在全库还有几十处；更糟的是 `--ledger`
自己带着一把尺子（一条链正则加一段复算），于是"什么算主张"在仓里有**两份实现**——两份必然漂移，
这与 §1.1 说的"两个互不相同的源"是同一个病。本轮把它收成一份：识别与复算只在
`scripts/gate/formula_ledger.py` 判一次，`--ledger` 与全库普查都调它，两份仪器各自只保留扫描集、判决与牙齿。

第一版尺子在全库报"主张 171 条／不闭合 112 条"，**逐条读原文后只有仪器是坏的**：
`exit=0`、`rc=1`、`failed ≥ 649`、`**加粗**`、反引号里的路径、`鲲鹏920+飞腾2000`、`2.1=10 + 2.2=9`、
`W1=30+16=46`、`946 ＝ +1 文件 +9` 全都不是分解式加法。把这批反例归纳成四条规则之后，
同一份语料上的读数是（工件 `reports/data/doc-formula-census-2026-10-02.json`，15:25:49+0800 生成）：
**扫描集全 `docs/` 的 .md（分母现量在工件键 `files`：15:25 是 370，15:40 复跑就跳成 371——并发作者新加了文档）｜ 主张 6 条 ｜ 不闭合 0 条**，主张只落在两份文件——本文件托管块外的 5 条，与
`docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` 的 1 条。盲区与引文的**总数不在这里重打**：
每往散文里加一个 `15:25:49+0800` 这类时间戳都会多出一条盲区账，把总数抄进正文＝制造一条必然腐烂的复述，
所以分档由工件键 `blind_reasons` 出账。

四条规则，每条都由真语料反例逼出来（不是手感）：

1. **粘连即非数**：数字串紧贴字母（含中文）、数字、`_`、`.`，或右贴 `%`／`*` ⇒ 它是标识符的一部分。
   带小数点／下划线的串一并走这条，所以这份台账**只管整数加法**。
2. **等号要全角**：本仓写等式的记法是 `＝` 配 `＋`；ASCII `=` 在这个语料里被 `key=value`、markdown 与代码占满
   ⇒ 只出账（reason=`ascii_eq`）不判红。
3. **跨不过代码段与句子**：反引号区间与 `。；；！？`／换行都是硬边界；跨代码段进 `code`，跨句被切断。
4. **紧邻的 `+` 是增量记号不是加号**：`+1 文件` 表达"本轮多一个"，与"总数 ＝ 甲 ＋ 乙"不同阶。
   另有两种遮蔽：拉丁标签（`Setup 1 + BatchA 12`）进 `latin`；gap 里出现 `* _ < > ≥ ≤ ~ ^ | → % -`
   这类非加法律符号进 `noise`——**减法与不等式明说不在射程内**，而不是猜一个值去判。

七种盲区（`glued`／`latin`／`ascii_eq`／`increment`／`code`／`noise`／`no_fw_eq`）各自有编号的夹具与变异体：
`scripts/gate/check-doc-formulas.py` 的 `--selftest` 现量 **16 例 FAIL 0**（7 枚夹具 ＋ 9 枚变异体，15:22 现量），
§1.8b 那轮加"识别器单归宿"账后变 24 例（9 枚夹具 ＋ 15 枚变异体，15:47），§1.8c 补集外文档边界三枚后为 27 例，本轮再为识别器自己的扫描集边界补三枚后为 30 例（11 枚夹具 ＋ 19 枚变异体，16:07），§1.8d 把"每条判决都要有针"收成夹具L 后为 31 例（12 枚夹具 ＋ 19 枚变异体，16:23），§1.8e 给"接了 CI"这句话补针（夹具M ＋ 变异体20）后为 33 例（13 枚夹具 ＋ 20 枚变异体，16:46），同节撞出"echo 文案里的路径会被当成执行"并补上首词收窄与 变异体21 后为 **34 例 FAIL 0**（13 枚夹具 ＋ 21 枚变异体，16:55 现量），§1.8f 把"这条判决依赖哪个裁决点"做成反事实
（夹具N ＋ 变异体22）后为 **36 例 FAIL 0**（14 枚夹具 ＋ 22 枚变异体，17:06 现量），§1.8g 把 AGENTS.md 自己写的
「CI 门禁」接进对账（夹具O 加 变异体23/24/25，红模板从 13 涨到 15）后为 **40 例 FAIL 0**（15 枚夹具 加 25 枚变异体，
17:23 现量），§1.8h 加反向账（夹具Q ＋ 变异体26，两条新红模板，红模板 15→17）后为 **42 例 FAIL 0**
（17:43 现量，载体同节点名），§1.8i 把第三个出口接进对账（本地一键 runner 必须与 CI 的 gate 集合同账、
项数只许一个算源；夹具R ＋ 变异体27，红模板 17→24）后为 **44 例 FAIL 0**（18:06 现量，
载体 `reports/data/selftest-formula-2026-10-02-1806.txt` 第 48 行），§1.8j 再把一键与 CI 的**参数维**接进对账
（夹具S ＋ 变异体28，红模板 24→26）后为 **46 例 FAIL 0**（18:23 现量，
载体 `reports/data/selftest-formula-2026-10-02-1823.txt` 第 51 行），§1.8k 把 §1.8f 那句
"也能拆单接"变成一条命令 `--dir-account`（目录账独立口径 ＋ 自带分母塌缩红；夹具T ＋ 变异体29，红模板 26→27）后为
**48 例 FAIL 0**（18:42 现量，载体 `reports/data/selftest-formula-2026-10-02-1842.txt` 第 53 行），§1.8l 又把那条口径的盲侧
收成第五本账（`OUTSIDE_ACCOUNT` 双向登记账 ＋ 宇宙塌缩红；夹具U ＋ 变异体30／30·甲，红模板 27→31）后为
**56 例 FAIL 0**（20:57 现量，载体 `reports/data/selftest-formula-2026-10-02-2057.txt` 第 74 行）——引用例数要点名是哪一轮、由哪一行 stdout 印出；链：48→51（§1.8l，载体 1912 第 58 行）→54（§1.8m，载体 2003 第 62 行）→56（§1.8n，载体 2057 第 74 行）。
每撤一条规则都要看见判决翻转，其中**变异体 8 打的是假阴方向**：撤掉 `≥` 遮蔽后
「50 ＝ 甲 ≥ 20 ＋ 30」会被读成一条**闭合**主张——它不报红，只是安安静静多出一条账，这种缺陷只能靠针打反方向。
`--dircheck` 钉扫描集自己：`docs/_archive` 必须真的没被走进来、目录账每条理由 ≥8 字、分母现量打印。

同轮连带改的 `--ledger` 侧补了两枚针（`--selftest` 由 41 例变 **43 例**，15:20 现量）：
第 26 枚钉行号口径——`ledger_block_span` 给 0 基闭区间而 `scan_text` 按 1 基跳块，换算漏了**不报红**，
只会把块外那条真主张整条吞掉（主张 1 → 0）：**分母变小比多算一次难看见**，所以"判集非空"本身是断言；
第 27 枚钉"没有第二把尺"——拧 `formula_ledger` 的引文旋钮，`--ledger` 的判决必须跟着翻（引文被当成主张判红），
若 `check-api-surface.py` 里还留着自家正则，这枚针不会动。

盲区不是缺陷，把盲区读成"没有缺陷"才是缺陷 ⇒ 每条都按 reason 出账、按文件点名进工件。
分档全表与扫描集分母都只在工件键里出账（`blind_reasons`／`files`），本文**不再复述这两个计数**：
15:40 这一轮我把散文里的 `243` 换成"见托管块 `DECL-PARSE`"的写法，同一次普查的文件数分母就从 370 跳到 371
（并发作者又加了一份文档），而"钉住的数"要求本文永不改版——那不是台账是碑文。
报价改说量级：`ascii_eq` 属**百级**、`latin` 属**千级**（这两档不会因为本文改几行就换档），
其余五档会随正文里 `15:25:49+0800` 这类时间戳形状变动，连量级都不钉。
其中 `ascii_eq` 与 `latin` 两档是**迁移候选**：它们确实在写等式，只是用了本台账目前不敢判红的字形。
连底迁到全角记法就能进复算，但一次改三百多份文档不在本轮授权范围内，且必然与并发作者顶车 ⇒
**列为 §四 裁决点 6**：ASCII 等号族是连底迁全角，还是给台账加一把 ASCII 尺并逐处人工定性。
**本普查器同样故意未接 CI**：它的判决权取决于裁决点 6 选哪条路。（这一句自 §1.8e 起只管**判决模式**——
两台仪器的 `--selftest` 已经接线，而且"已接线"那句话本身有一枚对着 ci.yml 的针。）

### 1.8b 识别器只有一个归宿——这件事本身成了门禁口径（`check-doc-formulas.py --recognizer`，15:40 现量）

§1.8 把两份仪器的识别器收进一份模块；光收一次不等于"从此只有一份"。`--recognizer` 用 AST 扫仪器目录
（`scripts/gate/*.py` ＋ `frontend-ui/scripts/gate/*.py` ＋ `tools/**/*.py`，扫描集由 glob 现量、**不写文件名清单**——
清单会漏掉明天新加的仪器），把每个**正则位点**的模式串折成字面量，再问 FL 的词汇（`EQ_FW`／`ADD_FW`／`EQ_HW_RE`）
在不在里面：在、且不住在 `formula_ledger.py` ⇒ 判"第二把尺"，点名到文件与行号。
现量（15:47）：仪器 31 份——28 份 Python 走 AST 取正则位点，另 3 份 `.mjs` 走**弱尺**（整行含全角记号即算，不分位点；
弱尺要自称弱尺，否则那 3 份的"命中 0"会被读成"那里没有第二把尺"，而真实意思原本是"我没往那儿看"）；
算术记号命中**全在归宿文件**、别家 0 ⇒ RECOGNIZER PASS。
位点数不许钉在这里：本文件自己就是在册仪器，下一节为它自己补一枚 `re.search`，位点就从 358 变 359——
**散文钉住位点＝当场造第二份会漂的源**，所以只指判决行。
判决六种红：别家自带算术正则／归宿文件不在扫描集／判决集塌缩（一个位点都没看见）／有仪器解析失败／
集外候选读到 0 份／第二把尺在扫描集外（后两条是下面那格边界通道新增的）。
其中第二种是本轮现踩的：**把归宿文件撤掉后 `foreign` 必然为空**，全称判据在空集上恒真，
所以"归宿在不在场"必须自己成一条判决（变异体 11），而不是当前提默默成立。

两条自己踩出来的坑，同属"仪器看不见自己要测的结果"那一族：

1. `EQ2.search(x)` 这种**已编译模式走接收者**的写法，模式在接收者身上而不在第一个参数。
   第一版按 arg0 取值，把 `re.search(PAT, text)` 的 `text` 当成了模式 ⇒ 115 个位点整批落进"读不出"，
   而归宿账照样报"零第二把尺"——那枚红字永远不亮，因为针根本没穿这条通道。
   变异体 13 钉的是"撤这条通道 ⇒ 判决集必须真的少一位点"：这条通道买的是**盲区诚实**（173 → 73），不是新增红。
2. 我把 `＋` 收进 `ADD_FW` 常量、让三根针的模式串由词汇拼出之后，`re.compile("[" + ADD_FW + "]")` 在 AST 里是
   `BinOp` 而不是字面量 ⇒ 归宿文件自己的 `ADD_RE`／`ANY_OP_RE` 读不出，算术命中从 12 掉到 1。
   **归一化这个动作本身把尺子弄盲了一次**，露出来靠夹具H 的命中数从 12 变 1（不是靠它报红——它照样 PASS）。
   补法是串接折叠 `_fold`（常量、已解开的名字、`+` 串接折成整串；f-string 与函数返回值仍交盲区），
   变异体 14 钉"撤折叠 ⇒ 命中必须真的变少"，否则这条新通道又是一枚空转针。

盲区两档按名点名而不判红：模式在运行时构造或跨模块导入（现量 73 处），以及"含 `=` 与 `\d` 却不是等式尺"的
`key=value`／计数尺（`verify-ports.py` 的端口表、`SELFTEST PASS=n FAIL=m` 那类，现量 11 处）。
ASCII 那半把尺只认"转义加号＋等号＋数字"三者同现，这是**故意的收窄**：把 `[:=]` 这类字符类里的 `=` 也当等式尺，
端口门禁会全表判成第二把尺，而假阳的代价是人绕过闸门。收窄的账由变异体 10 的 benign 对照钉住——
它必须"不误报但仍出账"，只验"不误报"就是把盲区读成干净。
**本模式同样故意未接 CI**：它与裁决点 6 共用一把尺——若 ASCII 等号连底迁全角，`reco_is_arith` 的收窄就要重新量。

**扫描集自己的边界（16:07 现量，判决行末句）**：上面三条 glob 只圈住门禁目录，于是「其余 N 份仪器无一把自带」这句话的宇宙**原本是声明不是测量**。现在把同一套判据打在集外的仪器形状上（根 `scripts`／`tools`／`frontend-ui/scripts`／`platform`／`domains`／`.github` 加仓根的 `.py`／`.mjs`／`.js`／`.ts`；构建与派生目录按名跳过，并且**在判决行里印出跳过了几个**——豁免看不见数量就等于没披露）。集外候选是百份量级、集外位点两百处量级、集外自带算术记号 0 处，PASS 句因此同时点名在册与集外两份分母。
两处**靠读自己的输出**才抓到的问题（同轮 16:15 修）：① 表头印"扫描仪器 31 份"而 PASS 句印"在册 30 份"，
两个数都对（后者扣掉归宿那一份）却没有一处说明这个 −1 ⇒ 读者只能把它们读成两把尺数出的两本账。
现在表头自己写"含归宿 1 份 ⇒ 判分母 30 份"，PASS 句写"除归宿外在册 30 份"，两处由同一个表达式现推。
② 两本分母不相交此前只是**构造**（集外枚举走 `cand − have`），不是一格断言：一次重构若漏掉那个减法，
同一份仪器就在两边各数一遍，而"30 份与 97 份"合成一个假总数。夹具K 现在把交集大小印出来并要求为 0——
它是自检格（钉仪器自己的算术），不是能点名到文件的红，所以它的牙来自变异体 17／18 走同一批函数，而非来自自身。
牙齿三枚：夹具K 钉「集外分母非零＋跳过的目录数要印出来＋归宿文件在这条通道里也要在场」；变异体18 把一枚自造等式尺放进**真实的集外目录形状**里（`scripts/**` 下的 `.py`），要求点名到文件——否则「集外 0 处」只是没看；变异体19 钉的是本轮真撞到的一处隐身：`scripts/ci/ci.py` 首字节是 UTF-8 BOM，`ast.parse` 报「invalid character in identifier」，那份仪器整份落进「解析失败」而它自带的尺从此不进判决——**读码一律走 `utf-8-sig`**，不许假定仓里的脚本没有 BOM（本仓的门禁脚本另有 `check-script-paths.py` 管路径，两者管的不是同一件事）。

---

### 1.8c 「全库」这两个字的覆盖面是量出来的，不是声明的（`--dircheck` 与 `--census --json` 的 `outside` 键，15:59 现量）

§1.8 那句"全库普查"实际扫的是 `ROOTS = ["docs"]`——**集外有没有等式，这句话此前没有任何读数**。本轮把同一把尺
（`formula_ledger`，不新写正则）打在 `docs/` 之外：仓根那几份 `.md`、`AGENTS.md`／`ARCHITECTURE.md`／`README.md` 都在内，
逐文件复算后**主张按名点名**。现量（15:59，工件键 `outside`）：集外 `.md` 三百份量级、顶层目录 18 个、按名跳过的第三方/构建目录 10 个、
主张 1 条（落在 `reports/markdown/` 那份本轮产出快照里，闭合），盲区与引文各一档。

集外为什么不接 §1.7 的托管块：**集外没有可指认的单一算源**——那些数字是"某轮跑出来的读数"，给它们建台账等于
替派生面造第二个源。所以集外只判一条**不依赖账本**的事实：等式自己不闭合就红（`--dircheck` 里是判决，`--census` 的 rc 也吃它）。

这一轮真正值得登记的是三处**仪器自己错**（都与被审对象无关，全在同一台探针身上撞出）：

1. 集外原本"少看了七百多份"却毫无异样：`walk_md` 靠"目录名以 `.` 开头"这个顺手过滤器跳目录，
   而 pnpm 的真身恰好在 `node_modules/.pnpm/` 下——排除是**巧合不是口径**。修法是把排除名单按名写出来
   （`OUTSIDE_NESTED`）并**在判决行里印出跳过了几个目录**；豁免按名打印这件事，比豁免本身重要。
2. `os.path.relpath` 在 Windows 跨盘时抛 `ValueError`（临时目录在 `C:` 而仓在 `D:`）⇒ 自检不是判红而是**崩在没有判决的地方**。
   修法是 `_rel()` 回退成按原路径点名。症状值得记一遍：`rc=1` 与"某格红"在退出码上同形，只有读 stdout 才分得开。
3. 子串陷阱复发两型：`unclosed.md` 含 `closed.md`，于是"另一份没被点"用 `in` 判永远判错；
   Windows 绝对路径自带盘冒号，`split(":")[0]` 取出来的文件名是 `C`。修＝按 basename 全等、路径按 `:(\d+) ` 收尾取。

牙齿三枚（当时现量 27 例 FAIL 0，§1.8b 边界三枚落库后为 30 例，§1.8d 夹具L 落库后为 31 例，§1.8e 三枚落库后为 34 例）：夹具J 钉"集外探针必须真的读到东西、集内集外不许重叠（重叠＝同一本账复算两遍）、
红为空"；变异体16 在仓库外的临时目录写一份闭合一份不闭合，要求**两份都被读到**（主张数=2）才允许判红、
且红只许点不闭合那一份；变异体17 把集外枚举清空，验证"全称判据在空集上恒真"这条老账——分母塌了必须自己成一条红。

**这一节同时是"下一站该搬哪份文档"的答案**：全库写着可复算等式的 `.md` 只有 3 份（`docs/` 里 2 份共 6 条、
集外 1 份 1 条），其中带分解账的只有本文件——把 `--ledger` 的单一算源＋托管块推广到别处**没有对象**。
于是"所有核心公式模块化归一化"这件事的完成判据落成四件（此前这里写"三件"却列了四格——
两个数都对而它们的关系没写出来，正是 §1.8d 点名的那身病），全部有常驻读数：
识别器单归宿（`--recognizer`）／本文件的等式单一算源与文档镜像（`check-api-surface.py --ledger`）／
覆盖面自署（`--dircheck` 的集内目录账＋集外边界）／判决与针对账（夹具L 对门禁**自己这份源码**现取的红清单逐条要针，
夹具M 对 ci.yml 复算"接了 CI"这句话）。唯一还没有牙的是裁决点 6 的字形口径，未裁 ⇒ **判决模式**不接 CI
（哪半条通道真被它咬住是 §1.8f 实测的，不是打包宣称的）；
接进 CI 的只有两台仪器的 `--selftest`，而那一步自己也有针（§1.8e）。

### 1.8d 每条判决都要有一枚能打红它的针（夹具L，16:23 现量 31 例 FAIL 0）

前面几节把判据越写越多，却没有一处回答**"这些红里有没有哪条从来没被任何针打红过"**——一条没有对照的红只是文案。
现在这格账由门禁自己算：`--selftest` 用 AST 扫自己这份源码，把每个 `bad.append(...)` 的头一个字面量按 `%` 截成模板名
（**清单不手写**，手写清单会漏掉明天新加的那条红），再要求电池里有一枚内存对照真打出这条红：
未覆盖＝红、撤掉对照＝"死键"红、模板数为 0 也红（"分母塌了必须自己成一条判决"那条老律的新载体）。
现量 **10 条红模板全部有针**（§1.8e 给 CI 接线账补三条后为 13 条）：识别器 4 条＋识别器集外 2 条＋文档集外 2 条＋目录账 2 条，另配一枚"合法目录账不许误报"的 clean 对照。

落库当轮它抓到两处，都是仪器自己的问题：
① `cmd_dircheck` 的两条红写在命令函数里、自检碰不到 ⇒ 先抽成 `dircheck_verdict(dirs, account)` 才有归宿——
"抽不出来的红"与 §1.4 那条"判决要挂在归一化之后的对象上"是同族：**判据若不能被内存复现，就等于没有被实现**。
② 前缀截法原先按 `" %"` 切，遇到 `理由不合格：%s`（全角冒号紧贴占位符）截出的模板名里还带着 `%s`，
于是那枚真会打红的针被判成"未覆盖"。这一格 FAIL 是**仪器的错而不是被测面的错**，而账面上两者同形，
靠打印出来的模板名才分得开；补了"截得太短就退回整串"的守卫，免得 `%s` 打头的模板当万能前缀。
两次红（未覆盖 2 条 → 1 条）的原始 stdout 归档在 `reports/data/red-coverage-fail-2026-10-02.txt`（三跑拼接：两跑 FAIL＋修好后一跑 PASS，判决行逐字节照抄，不是事后重打）。

同轮另修两处只有**读自己的输出**才会发现的：表头"扫描仪器 31 份"与 PASS 句"在册 30 份"没说明那个 −1（§1.8b 末段），
以及夹具K 补上"两本分母不相交"——按构造成立不等于已经断言。

**这格的边界（移植前必读）**：夹具L 的宇宙是 `bad` 这个**累加器名**——本门禁的三份判决函数恰都用它，所以现取得到 10 条。
用别的写法累加的红对它不可见，而姊妹仪器 `check-api-surface.py` 正是那种形状：现量 2709 行里 `checks.append` 43 次、
`bad.append` **0 次**（它的自检按"每格自带期望红"组织，红不是累加出来的）⇒ 把这段代码原样搬过去会得到空模板清单，
而这里"模板数为 0 也红"那条守卫会当场把它报成红，不会被误读成"那边没有红"。
移植的第一步是先定那边的累加器口径，不是再抄一格。

### 1.8e 「接了 CI」这句话也要有针（夹具M ＋ 变异体20/21，16:55 现量 34 例 FAIL 0）

上面几节把"这台仪器有牙"做成了可复算的账，但**"这根针每天有人踩"仍然只是散文里的声明**——
而它恰恰是 §1.8c 那族病的新载体：宇宙（CI 到底执行了什么）是声明不是测量。
接线动作本身也会腐烂：明天有人删掉那一步，全库不会有任何一格变红。

现在这格账由门禁自己算，而且**不自写第二把尺**：
"什么是一条真会被敲下去的 run 命令"是 `check-script-paths.py` 的既有判据（靠缩进出/入块、shell 注释必须剥掉，
两处都写明是承重的），所以 夹具M 按路径装载那个模块、只调它的 `ci_command_tokens`，本文件新增的只有一枚
"行尾反斜杠续行"的拼接。名单 `CI_WIRED` 只两台、且都只吃 `--selftest`：
`--census`／`--dircheck`／`--recognizer` 仍待裁决点 6，谁把它们误接进去，这条账会打成第三种红而不是沉默。
（**这句"三条都待 6"当时是打包主张，§1.8f 已把它改成按通道实测**：`--dircheck` 的依赖只住在它的集外那半条通道上。）

**三条红**（都走 `bad.append`，因此 夹具L 当场向每条新红要一枚内存对照——红模板从 10 条涨到 **13 条，未覆盖 0**）：
CI 扫描集塌缩（run 块里一个可执行脚本都没读到 ⇒ 分母塌了自己成一条红，那条老律第 N 次落地）／
CI 未接线（名单里的仪器没出现在任何一条 run 命令里）／CI 接线未带 `--selftest`（接进去的是别的模式，或参数写丢）。

**两向各配一枚对照，因为这台仪器两个方向都会错**：
正对照钉"参数落在续行上的合法写法不许判红"（假阳的代价是人绕过闸门）；
变异体20 钉"名单仪器只写进 run 块的 shell 注释"必须落到『未接线』——
这一格同时是复用那条注释剥离通道的证据：剥完只剩 1 条可执行 token，而两台仪器各红一次，谁也没隐身。

**针咬不咬真语料，用 git HEAD 做独立反事实**（内存里跑，一行盘不写）：
HEAD 那份 ci.yml（本轮接线之前）读出 token 14／脚本种类 12，判出 **2 条『CI 未接线』并点名两台仪器**；
工作区那份读出 17／14，判 0 条。也就是说：如果本轮只在文档里写"已接 CI"而 YAML 里没落这一步，
`--selftest` 当场就是红的——这句话从声明变成了测量。

**复用尺的第二个洞，由这台仪器自己撞出来**：把名单跑在真 ci.yml 上时，宇宙里出现了`frontend-ui/scripts/gate/check-view-hex.py`——而它**根本没在 CI 里跑**，它只是三条 `BLOCKED` 说明行里的一条 echo 文案。原来 `ci_command_tokens` 按空白切词、只认脚本后缀，**分不出 `python x.py` 与 `echo "python x.py"`**；对一台判“接没接线”的仪器来说这是最坏方向的那个洞：**一行 echo 就能把『已接线』糊出来**。于是加一道按**命令首词**的收窄（`echo`／`printf` 起头的不算执行；`cd x && python y` 首词是 `cd`，照旧算），真语料当轮读数 17 条 token 中 15 条算执行、2 条住在打印语句里（按名印出，不静默少数）。
这一格由 变异体21 钉住，并带一枚**撤通道的反对照**：同一份宇宙把收窄关掉，必须读成“已接线”（判据红 0 条）——撤掉通道而针不红，说明针根本没穿这条通道。
**还有一条不许忘的边界（弱尺必须自称弱尺）**：复用来的后缀白名单是 `.py/.sh/.ps1/.js`，**不含 `.mjs`**，所以本仓那几枚 `.mjs` 门禁（`check-sfc-dead-refs.mjs` 真在 CI 里跑）在这条通道里**永远读成没跑**。`CI_WIRED` 今天只列两份 `.py` 因此不受影响，但把 `.mjs` 加进名单会得到“永远判未接线”——那是响亮地红、不是静默地绿，故维持现状而把它写在这里。

**接线这一步自己被本仓另一道门禁打红过一次，值得记下来**：
第一版把待接线的模式写成 `echo "NOT-WIRED check-doc-formulas.py --census/…"`，
`check-script-paths.py --check` 立刻判 `[BROKEN] ci.yml:105 -> check-doc-formulas.py（CI；盘上同名在 无）`——
它的 `_exec_words` 按空白切词、只认后缀白名单，**echo 文案里的裸文件名在它眼里就是一条要执行的路径**。
这个仓里已有的约定是"文案里写全仓根相对路径"（同文件那三条 `BLOCKED` 行都这么写），
所以修的是我这一行而不是给门禁开豁免桶：改成 `scripts/gate/check-doc-formulas.py` 后 `--check` 回 rc=0
（当轮覆盖面：判定引用 1805、CI run 判缺 2 且全部在册、执行面脱钩未在册 0）。
**假阳与假阴同价**这条律在这里的样子是：给"注释与 echo 里的写法"放宽判据，代价是让真脱钩从此隐身。

CI 侧现量（本机 2026-10-02 16:37）：`frontend-governance` 作业第 10 步，两条 `python … --selftest` 进 rc，
其余仍以 `NOT-WIRED` 明示；那一步与邻居两步同为仓根工作目录（该作业没有 job 级 `working-directory`，
`Install dependencies` 等三步各自带 `wd=frontend-ui`，所以路径按仓根写才对）。

**收尾时把这段说明自己复述的例数也撤了（同轮 17:00）**：那一步的注释原本写着"31 例含夹具L……43 例是 --ledger……"，
等于把两台仪器 stdout 末行的判决抄进 yaml 当第二源。§1.8d 那条"手写清单会漏掉明天新加的那条红"的律在这里同样成立，
而且已经兑现过一次：夹具M 落库把 31 改成 34 的那一刻，yaml 里那两行就在说谎，而**没有任何一格针会为它红**
（接线针判的是"名单里的仪器有没有带 `--selftest` 出现在 run 命令里"，注释被 `strip_ci_comment` 整行剥掉，本来就进不了宇宙）。
改成只指向判决行（"例数由各自 stdout 末行『SELFTEST 总 N 例，FAIL M 例』现印"）后逐项复跑：夹具M 宇宙不变
（run 块 token 17 条／执行 15 条／打印语句里 2 条）、`check-script-paths.py --check` 仍 rc=0
（本轮判定引用 1817、CI run 判缺 2 且全部在册）、两台仪器 `--selftest` 该轮读到 **34 例／43 例，FAIL 0**
（那个 34 只活到下一节的 夹具N 落库前——这正是"例数只该由 stdout 末行现印"的活例）。

### 1.8f 「这条判决待哪个裁决点」也是可测的（夹具N ＋ 变异体22，17:06 现量 36 例 FAIL 0）

上面那句"三条模式都待裁决点 6"读起来像边界，其实是打包：**依赖是代码的性质，不是模式名的性质**。
读 `cmd_dircheck` 就知道它由两条通道拼成——`dircheck_verdict(dirs, account)`（目录账：逐条要有名字与 ≥8 字理由、
归档目录不许进扫描集）与 `outside_verdict(outside_census())`（docs/ 之外那一圈 .md 的等式账）。
前者只做字符串形状判断，**一个字形常量都不读**；后者吃 `formula_ledger` 的等号尺，才是真被 6 咬住的那半条。

但"我读过源码，它不依赖"仍然是散文。于是把它做成反事实（夹具N）：运行时把 `globals()` 里的词汇表换成中性化替身
（两把等号尺都改成永不开火的模式＝裁决点 6 里"ASCII 连底迁全角"那条路的极端情形），同一个宇宙跑两遍，一个字节也不写盘：
- **目录账两遍逐字节相同** ⇒ 这半条通道与 6 无关（当轮读数：前后都是 `[]`，判据把"不变"印成 `True` 而不是沉默）；
- **同机植入的那枚真读字形表的判决必须变** ⇒ 当轮 `['fw=1', 'hw=1'] → ['fw=0', 'hw=0']`。
第二条决定第一条值多少钱：中性化若连一枚明知依赖字形的探针都改变不了，那条"不变"就是空转读出来的假话
（同族于 §1.8e 的"撤掉通道而针不红＝针没穿这条通道"，只是这次撤的是**替身**）。
变异体22 就把替身换成恒等，当场读出"植入判决变 `False`"，而夹具N 的判据是两个事实的合取——缺一即红。

**这一格不增红模板**：夹具N 与 变异体22 都是 `checks.append` 形状的真假判据，不往 `bad` 累加器里写文案，
所以 夹具L 的宇宙当时是 **13 条（未覆盖 0）**。"抽不出来的红＝没实现的红"那条律在这里要反着读：
不进累加器的判决本就不该向 夹具L 要针，它的牙由 变异体22 提供——把这句写出来，是为了明天有人看见
"例数里没有夹具N 对应的红模板"就误判成漏钉。（下一节 夹具O 新增两条红模板，该宇宙涨到 15 条；
"13"只是 17:06 那一轮的量，与 例数同理：引用要点名是哪一轮、由哪一行印出。）


### 1.8g 文档自己写的"CI 门禁"也不再是散文（夹具O ＋ 变异体23/24/25；读数载体 `reports/data/selftest-formula-2026-10-02-1729.txt`）

§1.8e 钉住的是**我自己那份名单**（`CI_WIRED` 两台仪器）有没有真接进 CI。但"某某是 CI 门禁"这句话在仓库里
有一个更权威、也更早存在的出口：`AGENTS.md` 的「常用命令」段——它是每个进来的人（和每个 AI 代理）读的持久约定。
名单住在仪器里、口径住在文档里，正是本文件反复在收的那种第二本账：改一边、另一边静默腐烂，而两边都能自证绿。

于是把名单来源换成文档自己（夹具O）：**同一行里既有 `CI 门禁` 标记、又挂着可核脚本路径**的行才算一条声称，
逐条要求在 `.github/workflows/` 下任一份 workflow 的**真 run 命令**里出现。"什么是一条会被敲下去的命令"
这条通道仍复用 `check-script-paths.py` 的判据（含 §1.8e 那两道剥注释与剥打印语句的收窄），不自写第二把尺。

读数载体那份工件是 17:29 现跑的（七条命令逐跑署名 md5，工件本身 md5 `6990d24da4e7`／28015 B，末行判决缺席即整份作废不写盘），夹具O 那一行印出：
workflow 5 份／可执行脚本种类 15／**声称 4 条／未接线红 0 条**，
四条落点全在 `ci.yml`（`verify-ports`／`check-frontend-module`／`check-locale-format-outlets`／`check-script-paths`）。
另有两处本格读不懂的东西，按名与按行号印出来而不是被跳过：
- 行 10 挂着标记却没有可挂的脚本路径（那行是 `cargo clippy`）——它落在盲区账里，"全绿"不等于"都核实了"；
- 同一条判据后缀白名单挡下一个 token（那是治理文档自己的路径，不是被执行对象）。

三枚变异体分别钉三条通道的双向性：
- **变异体23**：声称只住在 `echo "BLOCKED python x.py --check"` 的文案里时必须读成未接线。
  这枚当轮**首跑就是红的**，而且红得有价值——我把合成 workflow 写成了 step 内联的 `- run: |` 形状，
  而复用尺认的是自成一行的 `run: |`，于是整个块没被读到、执行宇宙为空。空集上"这条路径不在宇宙里"恒真，
  那枚针会在"仪器没电"上打出它想要的红。修法两半：形状改成与真 `ci.yml` 一致，并**另钉"执行宇宙恰 1 条"**
  ——先证明读到了东西，才谈得上证明它没把打印当执行。
- **变异体24**：把标记词换成文档里不存在的词，声称账必须当场塌成一条红，而不是安静印"0 条声称、全部核实"。
- **变异体25**：已知的后缀边界（`SCRIPT_SUFFIX` 不含 `.mjs`）必须落在**响亮地红**那一侧。
  今天这个形状在 AGENTS.md 里 0 处；`ci.yml` 确有一条 `.mjs` 门禁在跑，它进不了这本账，
  但一旦有人把 `.mjs` 写成"CI 门禁"，得到的是一条塌缩红而不是一句假绿——方向安全，故钉住不许反向腐烂。

这一格新增 **2 条红模板**（`CI 声称账塌缩`／`CI 声称未接线`），夹具L 的宇宙 13 涨到 **15 条（未覆盖 0，死键 0）**，
两条各配一枚内存对照；其余三枚判据是 `checks.append` 形状，不进累加器。

**三条不许被读成"已核实"的边界**（都要靠读者自己判断，本格只把量到的东西印出来）：
1. **"已接线"是下界**。它只保证某份 workflow 里有一条 run 命令执行该脚本，不保证那份 workflow 在当前分支或
   `paths:` 过滤下真会触发。两份带路径过滤的 workflow 是 `fusion-gate.yml`／`license-compliance.yml`——
   "带不带 `paths:`"读那两个文件的 `on:` 块即可复核，属结构事实；而"四条声称都不落在这两份里"由夹具O 印出的
   落点列承担（四条全部指向 `ci.yml`，该列在上面点名的读数载体里）——后半句是当轮账，不是永久事实。
   判"会不会触发"要另一把尺（YAML 触发的语义），本格不判，只把每条声称落在哪个文件印出来。
2. **只认 `CI 门禁` 这个字面标记**。AGENTS.md 里同义写法（"CI 门禁口径""阻断合并"那类）不进门，
   这是口径选择而非缺陷；变异体24 钉的是"标记通道有没有咬住文档"，不是"标记词族该有多大"。
   扩词族属于维护者裁决，与 裁决点 6 无关。
3. **名单侧与文档侧现在互相顶红**。`AGENTS.md` 与 `ci.yml` 都已有别的执行者在改（工作区里两份都在动），
   任一边单改（删掉一个标记、把某条门禁从 `ci.yml` 挪走）都会让这一格当场红——这是设计目的，
   但代价是这条红可能由**并发作者的语义学**而不是我的改动产生；接手的人先复跑，别引用上一轮读数。

### 1.8h 反向账：CI 天天在跑的门禁，指南里点不点得到（夹具Q ＋ 变异体26；读数载体 `reports/data/selftest-formula-2026-10-02-1743.txt`）

§1.8g 只问了一个方向——**文档声称的，CI 有没有真跑**。同一本账还有另一问：**CI 真在跑的，文档有没有写**。
只钉一侧时另一侧的漂移是静默的：维护指南少一格，下一个进来的人（或代理）就照少一格的口径跑，
而那条缺席的门禁自己天天绿——它不会报告"没人知道我在跑"。于是加 夹具Q：CI 执行宇宙里
凡是路径含 `/gate/` 的脚本，必须在 `AGENTS.md` 点得名，否则一条红点名到那份路径。

这台仪器**首跑就是红的**，而且红因在语料不在判据：gate 目录内 CI 真执行 10 份，指南里点不到 6 份——
其中包括本轮自己造的两台仪器。（两个数的载体不同：**10** 由 夹具Q 的 PASS 行现印在上面点名的读数载体里；
**6** 出自首跑那次红色 stdout，当轮没单独存工件，它的可复核替身是 `git diff --numstat -- AGENTS.md` 报的 6 行新增，
而这 6 行每一行都带 `CI 门禁` 标记。）把门禁接进 `ci.yml` 而不动维护指南，等于给仓库留一份少两格的清单；
这不是"文档还没补"，而是 §1.8g 的正向账看不见的那种缺口（正向账以文档为名单来源，文档没写的那条它无从得知）。

不对称是写进判据的，不是顺手默认的：口径是**目录名** `/gate/`，执行宇宙合计 15 份，
落在 gate 目录外的那 5 份只数不判、并把两个数一并印在行尾，免得"10 份"被读成"执行宇宙的全部"。
变异体26 分别钉两条通道：① 未登记必须点名，而同宇宙里住在 gate 目录外的脚本**不许**被一起点名
（口径过宽＝假阳，代价是人直接绕过指南）；② gate 宇宙为空时必须读成"反向账塌缩"，而不是"零条未登记＝齐备"。

账面变化（都由 stdout 现印）：新增 2 条红模板（`门禁脚本未登记在维护指南`／`反向账塌缩`），
夹具L 的宇宙 15 涨到 **17 条（未覆盖 0／死键 0）**，例数从 40 到 **42 FAIL 0**（两行新增例的名字
`夹具Q`／`变异体26` 都在载体里点名）；补完文档那 6 行后，**正向**声称账从 4 条涨到 **10 条**，
十条落点全部指向 `ci.yml`、未接线红 0，而行 10 那条 `cargo clippy`（挂标记却无可挂路径）仍按行号印在盲区。

一条"看着像缺陷、其实是我量错了口径"的记录：`frontend-ui/scripts/gate/check-theme-tokens.py:9` 的自述用法写的是
`python scripts/gate/check-theme-tokens.py`，我拿仓根 `os.path.isfile` 一探，得 False，险些报出"该文档路径已过期"。
真仪器的 `resolve()`（同文件 :126）允许第四种解释——**源文件所在包根相对**（`frontend-ui/package.json` 在场），
所以它判的是 EXISTS 而不是 MOVED。两条教训：判"某条路径写错了"之前，先量**被测仪器用的是哪一口径**；
`check-script-paths` 的包根豁免与 夹具O 的按字符串精确集合不是同一把尺，不许互相外推。

当轮另补一份见证（载体 B 段）：`AGENTS.md` 那 6 行**按书写形态从仓根逐条执行**，6 条 rc 行全为 0、
判决行全部在场（含 `verdict=PASS`／`CHECK new=0`）。这一条**故意不做成常驻门禁**——清单第一项就是本仪器自己的
`--selftest`，"照文档执行一遍"会递归；静态残余（路径可解析、真被 CI 执行、在指南在册）已由 夹具O ＋ 夹具Q ＋
`check-script-paths` 的 CI 支三处分担。因此这份见证**没牙，别接 CI**，它的价值只在"本轮确实敲过"。

§1.8g 边界 3 预言的那种红，本轮**真的撞上了**：写完 §1.8h 复跑 `--ledger`，镜像判红一项
（托管块 42 行配 42 行、首个差异在第 3 行、共 1 行不同），差异是 `扫描 .rs 1380 个` 对现渲 `1382`——
**不是我改的散文**，是并发作者动了 Rust 树。处理只走仪器自己的路（`--ledger --write` 重落一次），
并在落盘前后各做一次只读切分：块外前置区与后置区**逐字节相同**、CRLF 计数 0、块行数 40→40，
复跑 `--ledger` rc=0。这条记录的要害是：镜像判红的**第一嫌疑人是语料，不是我这轮的编辑**，
先复算差异落在哪一行再决定改谁，别拿"我刚改了文档"当因果。

### 1.8i 第三个出口：本地"一键质量检查"会不会是 CI 的子集却自称全量（夹具R ＋ 变异体27；读数载体 `reports/data/selftest-formula-2026-10-02-1806.txt`）

§1.8g 与 §1.8h 的两本账都画在"文档↔CI"之间。同一句话还有第三个出口：`AGENTS.md` 宣传的
`scripts/gate/check-all.ps1`（"一键质量检查"）。它比 CI 少跑一条门禁时，**本地全绿是假证据**——
开发者敲一遍全 PASS，推上去 CI 红，而红的恰好是"本地没有任何入口"的那条。于是 夹具R 直接复用
夹具Q 已经算出的 gate 宇宙（同一份名单，不再造第二把尺），逐条问：CI 真跑的这条 gate，
在一键脚本里作为**带引号的仓内路径**出现过吗。

它**首跑就红 7 条，而且红因在语料不在判据**：`AGENTS.md` 写着"一键质量检查（7 项）"，CI 真执行的
gate 有 10 份，那份 .ps1 只调用了其中 4 份——**6 条门禁没有任何本地入口**。第二个缺陷在同一格里：
段标签的分母不是变量，而是字面量 `[i/7]` 共 11 处。修完之后的现量由 夹具R 的 PASS 行印在载体第 45 行：
gate 宇宙 10 份／一键实际执行 10 份／`$Total` 为 8／文档项数 8／红 []。

为什么把"覆盖齐不齐"和"项数有没有第二个源"钉在同一格：那 11 处字面量正是本文件反复在收的形态——
同一个事实写 N 次 ⇒ 加一步就得记着改 11 个地方，**漏改的那几处不报错，只是安静地印错数**。
它不是抽象的整洁问题：它失效的方向恰好就是本格要防的那一条（本地一键全绿、CI 却红）。

被审对象的修法（顺序仍是"先加针看它红，再改对象，最后用变异体钉通道"）：`scripts/gate/check-all.ps1`
- :37 `$Total = 8` 定义一次，11 处段标签一律改写为 `[i/$Total]`；
- :123–124 新增第 8 段"其余 CI 门禁（与 ci.yml 同账）"，把缺的 6 条接进来；
- :32 `$env:PYTHONIOENCODING = "utf-8"`（要讲的下文）。

`AGENTS.md:21` 的"（8 项…）"不再是手写的数：同一行明写"项数由脚本里的 `$Total` 单源，
由 `scripts/gate/check-doc-formulas.py` 的 夹具R 逐条对账"。验 .ps1 的改动**不执行整条 CI**，
而用 PowerShell 自己的解析器复算语法：`Parser::ParseInput` 得 `PS-PARSE errors=0`（载体第 270 行），
再把第 8 段现抽出来单独跑，得 `SECTION8 steps=6 fails=0`（第 385 行）。

一次由仪器红在我自己刚写的 prose 上：那行 `AGENTS.md` 我起初带上了"CI 门禁"四个字，夹具O 当场判红——
CI 并不执行 `check-all.ps1`（`ci.yml:47` 的注释明写治理装置只在本地一键里跑）。
**修法是把措辞收窄（改成"其余闸门同账"），而不是把一键接进 CI 让声称变成真**：接 CI 会改变触发面，
那是另一件事的裁决，不该由"让某格转绿"顺手代劳。

本轮唯一一条"崩的是仪器而不是被测面"的记录：`SECTION8` 第一次跑是 `fails=2`，两条红因都是
`UnicodeEncodeError: 'gbk' codec can't encode '\u21d2'`——判决行里的 ⇒／＝ 在 GBK 控制台上把
**打印它的仪器自己**打崩，rc=1 会被读成"有红"，即仪器侧假阳。修法是补输出编码守卫而不是改判决文案：
`check-doc-formulas.py` 加模块级 `sys.stdout.reconfigure(encoding="utf-8", errors="replace")`
（仓内另外五份门禁早带同款守卫，照同一写法），runner 侧加 `$env:PYTHONIOENCODING`。
守卫有没有牙，载体末尾成对印出：未设环境变量时 `doc-formulas-noenv rc=0`（第 390 行，守卫在场）
对 `api-surface-noenv rc=1`（第 394 行，那份仍缺守卫）。后者**本轮只登记不动**（task #14）——
`check-api-surface.py` 同时带着并发作者的暂存改动，改它会把别人的未落库工作与我的守卫混成一次写。

变异体27 的四条通道（输出字符串逐字见载体第 46 行）：① 一条 gate 都没执行＝塌缩（合成宇宙 gate 有 1 份、
一键执行 0 份 ⇒ "塌缩"与"未进一键"两条同出）；② 分母写成字面量＝不合一，**哪怕 gate 都已执行、数字还自洽**
——那种"看起来全绿"也必须红，否则下一位加步骤的人得不到仪器提醒；③ 文档项数与 `$Total` 不符＝两个数
一起点名（合成：文档写 9 而 `$Total` 为 2）；④ 反对照：单源且项数相符时必须零红——收紧判据必须留
clean 侧，不然"红一切"也能自证绿。

本格的盲侧（印出来，不许让"全绿"冒充"都核实了"）：
- 名单只认**双引号里的整条仓内路径**。若 runner 改用相对路径、变量拼接或单引号，那条 gate 会判成
  "未进一键"——失效方向是响亮地红，不是安静地绿，这是故意选的。
- 分子按**集合**核（须为 1..`$Total`），同一步在 if/else 两分支各打一次标签属合法，本格不判"每步恰一次"。
- 文档项数只从**含 `scripts/gate/check-all.ps1` 的那一行**取，数写在别处的项数不在射程内。
- basename 不参与匹配：同名文件会撞，那是 §1.8h 记过的另一类假阳。

账面变化（都由 stdout 现印）：新增 7 条红模板（6 个头），夹具L 的宇宙 17→**24 条／未覆盖 []／死键 []**，
例数 42→**44 FAIL 0**（载体第 47、48 行）。中途 夹具L 自己红过一次：
`未覆盖 ['一键分母不合一：'] ／ 死键 ['一键分母不合一：$Total']`——红模板的头是在第一个 `%` 处截断的，
分子分支那条模板的头就是 `一键分母不合一：`，针登记成带 `$Total` 的长键永远对不上。
**这条不是判据的错，是台账键与截法口径不一致**，改键名后复跑即绿。

### 1.8j 路径对上还不算同账：参数也要对上（夹具S ＋ 变异体28；读数载体 `reports/data/selftest-formula-2026-10-02-1823.txt`）

夹具R 比的是**路径集合**。但一条门禁往往有两种调用形态，而它们判的不是同一件事：
`--selftest` 只测仪器自己（一眼不看仓库），裸跑或 `--check` 才判语料。
于是"一键里出现了这条路径"允许这样一种躲法：**一键跑 `X --selftest`，CI 跑 `X --check`**——
路径账全绿、项数账全绿，本地一键照样藏着一整条 CI 红。缺陷与 §1.8i 同族，只是这次藏在参数里。

夹具S 把 CI 的执行宇宙从『路径』升到『(路径, 参数组)』。名单仍完全来自复用尺：
`check-script-paths.py` 的 `ci_command_tokens` 给 run 块里的脚本路径，本文件的 `ci_exec_tokens` 按命令首词
撤掉 echo/printf（与 §1.8e 同一道闸——**一行 echo 骗不出参数账**）；参数是从同一条命令行里、路径之后那段
现取的派生（`_cmd_text_at` 本来就把 shell 续行并成一整串），不另起第三种切词，否则"同账"又退化成散文。
真语料现量由 夹具S 的 PASS 行印在载体第 48 行：**CI 侧 (路径,参数) 组 17／落在双账里的 12 组／涉及 10 份路径／红 []**。

它首跑也是红的，红因仍在语料：`ci.yml:87` 以 `--selftest` 执行 `scripts/gate/check-locale-format-outlets.py`，
而一键里那条路径只有裸跑形态。红原文长这样（当轮 stdout 未单独存工件，可核替身是
`git diff --numstat -- scripts/gate/check-all.ps1` 报的 +2 行，以及载体第 317 行那条以前没人敲过的
`[PASS] locale-format outlets gate selftest`）：

> `一键参数不合一 scripts/gate/check-locale-format-outlets.py：CI 以「--selftest」执行它（.github/workflows/ci.yml 行 87），一键里同路径只有 (裸跑，无参数)（本地全绿藏着一条 CI 红：CI 真跑的那组参数在本地没人敲过）`

修法只补被审对象（`scripts/gate/check-all.ps1` 第 6 段加两行：调用＋`Add-Result`；本机 `md5sum` 现量
`12d7755cb61d`，UTF-8 BOM 与 LF 保持原样）。这条新增**不新增步骤**，所以 夹具R 的分母账一处没动——
落盘驱动器把这件事写成断言而不是愿望：段标签集合改前改后必须逐条相同、`$Total = 8` 恰一处、
分子恰为 1..8，任一条不满足就不写盘。验它仍用 PowerShell 自己的解析器加"现抽真跑"：
`PS-PARSE errors=0`（载体第 283 行）、`SECTION6 steps=5 fails=0`（第 389 行）、第 8 段复跑
`SECTION8 steps=6 fails=0`（第 506 行）。

四条判据取向，各由 变异体28 的一枚通道钉住（输出逐字见载体第 49 行）：
- **裸跑也是一个参数组**。写成 `if args:` 把它跳过，就正好造出本格要防的形状：CI 裸跑判仓库、
  一键只跑 `--selftest`，两边"参数都对上"因为空参数根本没参与比较。通道 ④ 用"`X` 裸跑 vs 一键只有 `--check`"
  必须红来拒绝这种退化。
- **同一条路径在一行里敲两遍要各成一组**，且段落止于分隔词（`&&`／`;`／`|`）——把后半串的参数扒到前一段上
  会造出不存在的模式，通道 ⑤ 断言合成 run 块只出两组 `('--check',)` 与 `()`，echo 那行一条都不许进。
- **分母要防虚高**：复用尺按词交 token，同一行同一条路径会交来两个，而参数段是从整行取的——不去重就把同一组数两遍。
  组数是这格的总分母，虚高的分母与少数同罪。
- **同一个缺陷只出第一条红**：路径整条没进一键是 夹具R 的红，本格对它闭口。两条红指向同一处时，
  改其一会让人以为两条都好了（通道 ③ 反过来钉交集为空＝塌缩，不许读成"0 组不合一＝参数同账"）。

本格的盲侧（印出来）：每行只取引号路径的**第一次**出现（本仓没有一行敲两次的写法，真出现时宁可响亮地红）；
参数是**逐词相等**才算对上，语序不同判不合一（假阳方向，响亮）；`.mjs`／`node` 那类不在复用尺的后缀口径里，
和本文件所有 CI 账一样是盲区，只在 夹具O／夹具Q 的行尾按名印出来；参数指向的文件存不存在由
`check-script-paths.py` 判，本格不比。

账面变化（stdout 现印）：新增 2 条红模板（`一键参数账塌缩`／`一键参数不合一`），夹具L 的宇宙 24→**26 条／未覆盖 []／死键 []**，
例数 44→**46 FAIL 0**（载体第 50、51 行）。

两条仪器侧的记录，一并留在这节：
1. **载体驱动器要能被拒绝，也要拒绝得起来。** 第一版 `carrier_j.py` 把判决标记拼成 `8 steps=6` 而不是
   `SECTION8 steps=6`，`MUST` 清单当场缺两行 ⇒ 整份作废、不落盘，只把尾巴打到 stdout 上让我看见。
   这个"缺席就不写"的机制本轮第一次真的救了一次假证据（否则工件里会出现一条谁也复现不出的数）。
2. **落盘驱动器不许重跑。** 夹具S 那两格是"纯插入"，第二次跑会把同名格插两遍——而 `checks` 若按名字典化，
   重复 id 互相顶掉，例数照样自洽（这是记忆里记过的第 30 型）。所以修被审对象用了**单独**的脚本，
   并且插入前后各做一次"摘掉新增行必须逐字符还原原文"的断言。



### 1.8k 目录账能不能单独接：把 §1.8f 那句"也能拆单接"变成一条命令（夹具T ＋ 变异体29；读数载体 `reports/data/selftest-formula-2026-10-02-1842.txt`）

§1.8f 把"这条判决依赖哪个裁决点"从打包改成实测之后，留下一句可能性：`--dircheck` 的依赖只在集外那半条通道上，
目录账那半已由中性化反事实证明逐字节不变，"所以裁 6 之前也能把目录账单拆单接，只是本轮没动模式清单"。
**可能性不是测量**：判据在，命令没有——下一个人读到那句话，仍然只能整块等裁决，或者整块接进去
（后者更糟：让一条天天红的字形账替一条本来能绿的账背锅，而"绿"与"红"在同一个 rc 里分不开）。
本轮把那句话变成一条命令：`--dir-account`。

形状上只有一条实现：`dircheck_emit(dirs, 文件数, 集外读数, account_only)` 产出行与判决，
`--dircheck` 与 `--dir-account` 都是它的调用者，分母行与 `FAIL 扫描集` 行格式各只有一份。
等价性不是口头承诺（载体 B 段）：拆分前后 `--dircheck` 逐字节相同（第 277 行 `DIRCHECK rc=0 IDENTICAL=True｜拆分前像 362 B／拆分后 362 B`——
那两个 362 是**换行归一之后**的字节数，Bash 侧重定向出来的原始工件是 365 B，差的 3 个字节是三条行的 CR，
属呈现层不属测量），而两条口径的头一行逐字节相同（第 278 行 `HEAD-EQUAL=True`），
`--dir-account` 真语料只出一行、rc=0（第 281 行 `ACCOUNT rc=0 LINES=1`；同一行文本也出现在第 69、75 行，
分别是 `--dircheck` 与 `--dir-account` 的 stdout）。

新增的那条红是 `目录账分母塌缩：扫描目录 0 个／文件 0 个（扫描集为空时『目录账 3 条』不构成判决）`。
为什么一条"独立口径"必须自带分母断言：`dircheck_verdict` 的两条判据都是全称式（每条理由都合格／归档目录都不在集合里），
扫描集塌缩时它们在空集上恒真——第 35 型，把零证据印成满把握判决。从前这个风险被集外账替我挡着：
`outside_verdict` 里已经有一条"读到 0 份 .md"的红，所以拆出目录账时**若不把那条牙齿一起搬过来**，
新口径就成了一个"空扫描集也报绿"的命令，而这正是它被接进 CI 之后最危险的形状。

变异体29 五通道：① 同一份集外数据在两个口径里必须一个有一个没有（`account_only` 若只是纸面参数，两份输出同形，这条立刻红）；
② 分母行只许一处生成——两条口径头行逐字节相同，且其中"账 N 条"必须等于 `len(DIR_ACCOUNT)`（期望值从对象现推，不是抄一个 3）；
③ 扫描集塌缩必须自己出一条红并把两个数点名；④ 反对照：合法目录＋非零文件数必须零红，
而归档目录那一枚**必须有红在场**（收紧不允变成"红一切"，"不变"也不允来自空串对空串）；
⑤ 这条口径不读词汇表——把 `FL` 换成永不开火的替身，含一条真红的目录账输出必须逐字节不变。
⑤ 是 §1.8f 那句"目录账不依赖裁决点 6"从**代码事实**升级成**实测事实**的那一步：宣布之前先中性化看一遍。

撤通道见证（载体 C 段第 284–288 行，全部在内存里改模块全局，被审文件三轮之后仍是 102614 B／md5 `c3d3300f0c0a`）：
基线 rc=0 红格 []；变异A（撤掉 `account_only` 分支＝回到拆分前的形状）红格 [夹具T, 变异体29]；
变异B（撤掉分母塌缩那条红）红格 [变异体29, 夹具L]——夹具L 那格同时印 `未覆盖 ['目录账分母塌缩：扫描目录']` 与
`死键 ['目录账分母塌缩']`，第 34 型要的两半（针打不到 与 键没针）当场都在；
变异C（给分母行开第二个源）只红 变异体29 一格——这一枚存在的意义是把 ② 从前两枚里**隔离**出来，
否则"A、B 都红两格"会被读成"变异体反正都红"。

账面变化（都由 stdout 现印）：红模板 26→**27 条／未覆盖 []／死键 []**（第 52 行），例数 46→**48 FAIL 0**（第 53 行），
`--recognizer` 的模式位点 379→**380**（第 57 行；涨的那一处就是本轮 变异体29 里那把 `re.findall(r"\d+", head)`，
它是"从分母行反解数字"的小尺，只活在自检内存夹具里、不进文档账——但**位点账是现量的**，仪器加一处 re 就涨一格，
这恰好证明 夹具H 的分母不是手钉的）。同轮复跑：`--ledger` 镜像 PASS（第 131–132 行）、
`check-script-paths --check` 判决行在场（第 144 行）、`check-doc-links` 未发现断链（第 274 行）、
`SECTION6 steps=5 fails=0`／`SECTION8 steps=6 fails=0`（第 398、517 行，两段命令清单未改，
本轮没动 runner 也没有新增步骤）、`doc-formulas-noenv rc=0`／`api-surface-noenv rc=1`（第 522、526 行，
编码守卫的成对见证）。

**没接线，而且接线账现在管不到它**——这一句必须自己说，不许留给读者猜：
`--dir-account` 既不在 ci.yml 的执行宇宙里，也不在 AGENTS.md 的"CI 门禁"声称行里，所以 夹具M／O／Q／R／S 五格
今天都对它无话可说；本轮只有 变异体29 保证"这台仪器有牙"，没有任何仪器保证"每天有人踩它"。
给维护者的两条接法与各自的账：(a) 只把它接进 `frontend-governance`（不碰一键，`$Total` 与 AGENTS.md 的项数都不动，
但 夹具O 会在 AGENTS.md 里那条声称挂上路径时开始要求它出现在某份 workflow 的 run 命令里）；
(b) 接进 `check-all.ps1` 第 8 段并把该项计入 `$Total`——那会让 夹具R 的分母账从 8 变 9，
且 AGENTS.md 的"（8 项）"必须**同轮**改，否则 夹具R 当场印 `一键项数与文档不符`。两条都不等于裁决点 6 已裁：
集外账仍然只能等 6，因为它读字形。

一句"为什么没动 AGENTS.md"的记录：它第 15 行带着「CI 门禁」标记又挂着 `scripts/gate/check-doc-formulas.py`，
本身就是 夹具O 那本声称账里的一条（现量 10 条，载体第 42 行）。往那一行补 `--dir-account` 有两种坏法：
同一条路径在一行里出现两次，声称账就从 10 涨成 11（而 §1.8h 记的"10 条"当场过期，那是账不是装饰）；
另起一行不带标记，则这条口径进不了声称账，读者在别处读不到它。本轮两条都不做，新口径的去处就是本节，
而那行原有的话（`--census`／`--dircheck`／`--recognizer` 不进 rc）到今天仍然成立——成立不等于完整，
完整性由 §1.8h 的反向账（跑的有没有写）在 CI 侧兜，接线之后才会轮到这条新口径。

本格当时的盲侧（印出来，§1.8l 已把它收成一条判决）：目录账只判 `DIR_ACCOUNT` 那两条通用判决（理由长度、归档目录不许进来），
**不判"该进来的目录有没有进来"**——扫描集只有 `ROOTS = docs` 一根，没有任何仪器核实过 docs 之外是否还有活文档目录
该进这一根。那是第五本账的形状，与 §1.8g 的"名单来源换成文档自己"同族，本轮登记为待办，不复用 夹具T 的名义声称已覆盖。
（登记于 18:42；§1.8l 于 19:12 现量并落库判据，载体 `reports/data/selftest-formula-2026-10-02-1912.txt`。本节这句"不判"自 §1.8l 起不再是本口的状态，
但也不许反过来读成"目录账什么都判了"——§1.8l 末段又印出四条新的盲侧；§1.8m 于 20:03 再落第六本账（让出面账，载体 `reports/data/selftest-formula-2026-10-02-2003.txt`），
这四条盲侧里的 ④ 由它收口，其余三条仍未判，而 §1.8m 末段又印出四条它自己的盲侧；§1.8n 于 20:57 再落第七本账（出账执行账，载体 `reports/data/selftest-formula-2026-10-02-2057.txt`），它不收口那四条中的任何一条——那四条仍只是被 夹具W 印出来，判据一条没加。）

### 1.8l 第五本账：该进来的有没有进来（夹具U ＋ 变异体30／30·甲；读数载体 `reports/data/selftest-formula-2026-10-02-1912.txt`）

§1.8k 末段印出的那句盲侧，本轮不再只是"登记"：`--dir-account` 与 `--dircheck` 共用的目录账口径，
现在同时判**进来的合不合格**（`DIR_ACCOUNT` 两条）与**该进来的有没有进来**（`OUTSIDE_ACCOUNT` 双向登记账）。
这条新账一个字节不读字形表（只用 `os.walk` 给的目录名与文件名，不调 `_scan_paths`），
所以它跟着目录账那半条一起属于"不待裁决点 6、可单独接线"那一侧。

**宇宙是先量的，不是先声明的**（载体第 80–94 行）。顶层 18 个目录＋仓根 5 份共收到集外 .md 310 份，
它们落在 **11 个桶**（桶＝顶层目录＋仓根；第 80、81 行），出账条目也是 11 条，两侧集合相等=True（第 81 行）。
候选面只复用集外账那把 `outside_top`／`outside_walk`，**没有第三种切法**；第二口径用 git 索引逐桶复算
（第 82–93 行的表），三处非零差全部在第 94 行归因：`my-projects` 差 +83＝整体未入库、`projects` 差 +3 与
`reports` 差 +4＝未跟踪的导出物与报告，而 `docs` 在 git 口径里的 415 份属扫描集内、不进本表。
交叉复算再补一枚：同一批路径改走 `_scan_paths` 也得到 310（第 80 行），与只走路径的桶计数相等——
两条通道对"集外有多大"给出同一个数，这条口径才敢说不读尺也能对上分母。

**形状取双向登记账，不取"N 份以上才判"**。阈值是一个要人猜的第二个源，而"每个装活文档的桶都必须有一句理由"不要；
分母（桶数、条目数、份数）全部由本轮现量给出，代码里没有任何 `MIN_` 常量。代价如实写：新增一个带 .md 的顶层目录，
`--dir-account` 会当场红一条"漏登记"并要求补理由——这是设计意图，不是假阳；但它决定了接线成本（末段）。

**一实现两口径的形状没有被撑破**：`--dir-account` 仍 rc=0 且只出一行（第 69 行），两个口径的头一行逐字节相同（第 76 行，
六个数＝目录／账／文件／集外桶／集外 .md／出账），`集外主张` 字样只在全量口径出现（第 77 行）。

**两条真缺陷是本轮自己的针抓出来的，不是事后编的**（第一次自检 3 例 FAIL）：
⑴ 泄漏探针写成了子串 `not any("集外" in l ...)`，而第五本账的分母行合法地含"集外桶"三个字 ⇒ 夹具T 与 变异体29·① 双双**假红**。
改法是让探针点名为"主张通道"的字样（`集外主张`／`不闭合`），并反向要求分母行里真的出现"集外桶"——
禁令与探针只许枚举出口的**完整写法**，枚举局部片段就把同族合法变体判成缺陷（假阳的代价是人绕过闸门）。
⑵ 变异体30 的④ 原本把**整张出账表**换成一条 `("tools", "短")`，期望"恰 1 条理由红"，实测 11 条红——
另外 10 个桶被一起注销了。改成只改 tools 那一行的理由。这与"针要挂在被撤的那条通道上"是同一型错误，第二次犯在本仓库。

**牙齿与逐格归因**（第 97–101 行；四枚破坏都只写在临时副本上，真仪器一字节未动，第 316 行终检 md5 与运行前一致）：

| 破坏 | 撤的是什么 | 打红的格 |
|---|---|---|
| 甲 | 摘掉一条真在册的出账条目 | 夹具T、夹具U、变异体30·甲 |
| 乙 | 把宇宙判据退化成空操作 | 变异体30·甲、夹具L（**夹具U 保持绿**） |
| 丙 | 拔掉"宇宙塌缩"那枚唯一牙齿 | 变异体30、夹具L |
| 丁 | 分母行少印"出账 N 条"一个数 | 只红 变异体29（丁∩甲 为空，隔离成立） |

乙那一行是本轮最贵的读数：**真语料那格对"判据退化成恒真"是盲的**——`universe_verdict` 返回空表时，
"未登记为空、失效为空"照样全绿；只有 夹具L 的"死键"那格看得见。四枚破坏的红格集合两两不同=True（第 101 行）。

**台账增量**：例数 48→51（第 58 行）、红模板 27→31 且未覆盖与死键均为空（第 57 行）、
模式位点 380→381（第 106 行，盲区 86）。那 +1 的来路不是猜的：识别器把 `str.replace`／`split` 这类**方法位点**也计入，
本轮新增的 `outside_buckets` 里第 206 行 `rel.split("/")` 就是它（pattern=None ⇒ 落盲区，不是自带第二把尺），
判分未变——算术记号命中 12 处／别家 0 处（第 107 行），RECOGNIZER PASS rc=0（第 114–115 行）。
姊妹门禁同轮复跑：`--ledger` 文档镜像 41 行逐字节相同、公式 30 条 PASS（第 171–173 行），
`check-script-paths.py --check` rc=0（第 185 行），`check-doc-links.py` rc=0（第 315 行）。

**顺手修一处存量口径**（数字要挂在印出它的那份文件上）：从前别处复述的"doc-links WARN 120"是**被截断的打印数**，
工具自己印"打印 120 条／总体 206 条"并告示截断（本轮第 315 行现量：打印 120、总体 206、ERROR 0）。
本文件 P3 那两行当时就写的是"打印截断至 120"，所以文档没有假账，腐的是复述它的地方；比较存量要取总体那个数。

**未接线，而且接线账管不到它**（同 §1.8k）：没进 `ci.yml`、没进 `check-all.ps1` ⇒ 夹具R／夹具S 对它无话可说。
第五本账让接线成本变大了一档：`--dir-account` 一旦进 CI，任何人新增一个带 .md 的顶层目录都会红 CI，
而"该不该进"是文档治理裁决而不是代码缺陷。两条接法各自的账：(a) 以判决形态接 ⇒ 先规定新增目录由谁裁决；
(b) 以 advisory 形态接 ⇒ 按"advisory 不许只印不判"的规矩，必须配一枚"撤掉出账就少一格"的交叉针——
本轮的 甲 就是那枚针的形状，可以直接搬。AGENTS.md 本轮未动：那条"CI 门禁"标记载体里再列一次脚本路径，
会把 夹具O 的声称账从 10 条抬到 11 条并连带 §1.8h 的数字过期，那是另一轮要一起搬的账。

本格的盲侧（印出来，不许读成"第五本账全判了"）：① 桶的粒度钉死在**顶层**——`docs/` 里面新开的目录（在 ROOTS 内）
不出现在第五本账的视野，它只被目录账那两条通用判决管；② 出账理由是散文，没有任何尺复算"理由与该目录实际内容是否相符"
（本轮 11 条理由是逐条打开目录读过才落笔的，那是人工见证，不是仪器）；③ 仓根 5 份里 `CLAUDE.md` 与 `AGENTS.md`
是否同一事实的第二源，本格不判；④ `OUTSIDE_SKIP` 那 11 个名字本身无人复算——被跳过就不出现在宇宙里，
等于**用名单定义覆盖面**，与 §1.8 集外账同病。④ 是四子里唯一能由仪器自己判的——**已于 §1.8m 收口**（让出面账 ＋ 夹具V ＋ 变异体31／31·甲，载体 1957）；① ② ③ 本轮仍未判。


---

### 1.8m 第六本账：被挡掉的有没有出账（夹具V ＋ 变异体31／31·甲；读数载体 `reports/data/selftest-formula-2026-10-02-2003.txt`，它是本节正文落盘**之后**再跑的那张，故其中与文档有关的读数与发布态一致）

§1.8l 末段四条盲侧里的 ④ 本轮收口。旧形状是：**覆盖面由名单定义**——`OUTSIDE_SKIP` 的 11 个名字加上一条
没有名字的点前缀规则，把顶层目录挡在集外宇宙之外，而"挡掉了什么、挡掉几份、谁批准的"没有任何一本账。
新账不读字形表（只看路径与 git 跟踪面），所以它跟着目录账那半条属于"不待裁决点 6、可单独接线"那一侧。

**先量再写**（载体第 84–95 行）。git 跟踪的 .md 共 **642 份**，顶层目录 37 个，其中被挡掉 19 个
＝SKIP 命中 10 ＋ 点前缀 9，桶名重叠 0 个（第 85 行）。挡掉的桶里**真正让出过跟踪 .md 的只有 4 个、合计 7 份**
（`ais` 1／`log` 1／`.trae` 1／`.workbuddy` 4），其余 14 个桶让出 0 份——第 87 行那句"本账不许为它们留条目"
是这条账没有腐成第二份名单的唯一原因：域＝**确实让出了东西的桶**，不是"所有被提到的名字"。
第 88–93 行的表逐桶给份数、来源（名单／点前缀）与文件样例，在册 4 条与实测让出桶集合相等=True、红=[]（第 94 行）。

**形状沿用第五本账：双向登记账，不取阈值**。三条红分别是"让出面漏登记／让出面条目已失效／让出面理由不合格"，
理由尺本轮归一：`REASON_MIN` ＋ `reason_bad()` 一次定义，此前"空理由"与"短于 8 字"这两条判据在两个口径里
各写了一遍字面量 `< 8`（三条尺管同一件事就是三个源）。让出面上的全称判据在空集上恒真，所以"齐备"必须由
调用方的塌缩红兜（第 35 型），这条红把"读到 0 份"与"确实没有让出"分开印。

**一条此前不存在的红：`扫描集与集外宇宙桶名重叠`**。`docs` 同时在 `OUTSIDE_SKIP` 与 `ROOTS` 里，
它的作用是防双算而不是防覆盖面——这句话从前是推测，本轮第一次有数：`docs` 跟踪 .md 415 份属**已接管**
而非**已让出**（第 93 行），一旦宇宙吞掉这个桶名，同一批文档就被两本账各算一次。丙枚破坏专门拔掉重叠判据
（第 101 行），它同时红 变异体30·甲／夹具T／夹具U——那三格都按真宇宙现推，宇宙被污染时它们必须不平静。

**git 是外部依赖，所以必须有 withheld 通道**：`tracked_md()` 是这本账唯一的 git 出口，失败时返回
`(None, 原因)` 而**绝不返回空表**——空表会让"漏登记"在真实缺口上恒绿，把"没读到"印成"没有"。
withheld 时分母行印"让出面账 withheld（原因）：这一本今天没判，别读成让出面干净"，一条让出面红都不发，
而同一次调用的另两本账照旧出红（withheld ≠ 闸门停了）。乙枚＝git 不可用，只红 夹具V 且不红任何一条"齐备"
（第 100 行）；戊枚＝只印状态不印原因，只红 变异体31（第 103 行）⇒ **那句"原因"本身是一格判决，不是装饰**。

**五枚破坏的逐格归因**（第 99–105 行；破坏只写在临时副本上，真仪器一字节未动，第 320 行终检 md5 与运行前一致）：

| 破坏 | 撤的是什么 | 打红的格 |
|---|---|---|
| 甲 | 清空 `GIVEN_UP_ACCOUNT`（撤登记） | 夹具T、夹具V（真语料上当场 4 条漏登记） |
| 乙 | git 不可用（走 withheld 通道） | 只红 夹具V |
| 丙 | 让宇宙吞掉扫描集桶名 `docs` | 变异体30·甲、夹具T、夹具U |
| 丁 | 分母行整段不印让出面 | 变异体29、变异体31（两格，见下） |
| 戊 | withheld 只印状态不印原因 | 只红 变异体31 |

五枚的红格集合两两不同=True（第 104 行）。丁不是设计失误：分母行是两条口径共用的**唯一出口**（§1.8k 的形状），
撤一段同时打在 变异体29 的"九个数"与 变异体31 的"withheld 在场"两格上——一处改动红两格时归因不成立，
所以补戊做隔离枚（只被 变异体31 读到的那条通道）。丁不红 夹具V：那格吃的是 partition 与 verdict，不吃分母行。

**一实现两口径的形状没有被撑破**（第 80–82 行）：`--dir-account` 与 `--dircheck` 仍 rc=0／rc=0，目录账口径仍只出一行，
两个口径头一行逐字节相同，九个数依次为 目录／账／文件／集外桶／集外 .md／集外出账／让出桶／跟踪让出／让出出账；
"让出"字段在两个口径都在（这本账不读字形 ⇒ 与裁决点 6 无关），而"集外主张"字样只在全量口径。
合成格一律改为自带植入让出面（`g_synth`／`ga_synth`）⇒ 自检在内存里逐格定价，一字节不落盘、一次也不碰 git。
三面分区还有一枚交叉复算钉住不相交且完备（载体第 95 行）：ROOTS 侧 415 ＋ 宇宙侧 220 ＋ 让出侧 7 ＝ 642，
即跟踪面总数。

**这一行是本轮自己的门禁打红的，不是事后补的**：初稿把载体行号写在等号右边（那一行有两个等号，
末边是"跟踪面总数（第 95 行）"），`--ledger` 的散文加法判据把那个 95 当成第三个边，判"不闭合"并点名到行
（各边 [642, 642, 95]）。教训：**带数字的括号引用不许写在等式右侧**，否则引文自己变成账目的一项；
改法是引文挪到冒号前，并把这一型缺陷登记进正文（本段就是登记）。这条判据立在 §1.7，
本轮第一次落在我自己新写的段落上——它同时说明文档镜像那组判据是有牙的：托管块之外的散文加法也要自己成立。

**台账增量**（第 2、61–62、71、110–111 行）：仪器 127765 B／md5 `f47295cdd92d`；例数 51→**54**、FAIL 0；
红模板 31→**36** 且未覆盖与死键均为空（新增五条红各自配了一枚内存对照，撤对照当场就是死键）；
模式位点 381→**382**、盲区 86→**87**。+1 的来路不是猜的：唯一新增的方法位点是 `tracked_md` 里那个
`split("\0")`（pattern=None ⇒ 落盲区，所以位点与盲区同步 +1）；而 `top_bucket()` 抽出后，
`outside_buckets` 里那份 `rel.split("/")` 副本被删掉了——本轮自查抓到"注释写着共用一把切法、代码里还有第二份"，
若不合并这两处，位点会是 +2 而不是 +1（第 110 行那个 382 就是这次合并的收据）。判分未变：算术记号 12 处／别家 0 处（第 111 行）。
文档侧同轮再渲染普查（`--census --json` 由仪器自己落盘，不留第二个写者）：主张 6→7、违反 0、未闭合 0 条——多出来的那一条主张就是本节那枚交叉复算，两张载体各自第 13 行的自检行是它的收据。文档侧的盲区计数不钉在这里：它随正文行数走（本节每补一行账它就 +1），写进文档就是下一轮的腐烂项。
姊妹门禁同轮复跑：`--recognizer` rc=0（第 110 行）、`--ledger` rc=0 且散文加法复算 6 条主张红 0（第 169、177 行）、
`check-script-paths.py --check` rc=0（第 189 行）、`check-doc-links.py` rc=0 且总体 206 与上一张载体（1912）相同（第 319 行）。

**新的盲侧（印出来，不许读成"让出面全判了"）**：① 这本账的尺只有 git 跟踪面——未跟踪那一圈无账。
同一轮里 `ais` 在 6 秒预算内 fs 口径读到 2661 份 .md（走 5113 个目录，预算内没走完），而跟踪面只有 1 份（第 96 行）。
这两个数**不是不变量**：相隔六分钟的两张载体（1957 与 2003）分别印出 2978／5387 与 2661／5113，差别只在预算烧到哪一刻。
所以本段只登记形状（未跟踪那一圈比跟踪面大三个数量级），印出的两个数归当轮——谁复述谁重跑。② `coverage` 在 `OUTSIDE_SKIP` 里
但仓根没有这个目录 ⇒ 它是一个从不点火的名单项；本账只把它印出来，不判"名单里该不该有哑项"。
③ "该不该让出"仍是散文理由，没有尺复算理由与该目录实际内容是否相符（承 §1.8l 盲侧②）。
④ 嵌套跳过（`OUTSIDE_NESTED` 8 项）不参与本账：桶在顶层归堆，嵌套跳过的目录若整个落在某个桶里，份数会算进那个桶，
但"这个名字为什么被跳过"仍然无账。

**仍未接线**：没进 `ci.yml`、没进 `check-all.ps1`（本轮实际未动这两份文件）⇒ 夹具R／夹具S 对它无话可说。
接线成本比 §1.8l 记的那一档又大一档：接 `--dir-account` 等于同时把两条"新增即红"接进 CI——
新增带 .md 的顶层目录（第五本账）与**任何新被 git 跟踪的 .md 落在点目录或名单目录下**（第六本账，本轮新增）。
"该不该让出一个桶"是文档治理裁决而不是代码缺陷，两条接法各自的账同 §1.8l 末段。


### 1.8n 第七本账：出账那句"排除"到底执行了没有（夹具W ＋ 变异体32；判决行载体 `reports/data/selftest-formula-2026-10-02-2057.txt` 第 69、70、73、74 行，等价性收据 `reports/data/diraccount-delta-2026-10-02-2050.txt`，位点归因 `reports/data/pattern-site-attribution-2026-10-02-2051.txt`）

**本节不收口 §1.8m 那四条盲侧中的任何一条**（它们仍只被 夹具W 印出来，判据一条没加，见本节末段）。第七本账是量 SKIP 名单时撞出来的另一件事：前六本账判的是"进来的合不合格""没进来的有没有出账""让出的有没有登记"，而**出账条目自己那句"我排除了 X"从来没有一把尺去读它执行了没有**。`DIR_ACCOUNT` 三条理由（`docs/_archive`／`**/_verification`／`docs/**/_data`）从前是散文，两条读路各自硬编自己的剪枝名，两边不同步也不会红——仪器源码里第 55–56 行的注释就是这条缺陷的现场登记。

**先量，量出来的是缺陷**（收据第 5–10 行，三口径在**同一份盘、同一轮**里各读一次，一行盘未写）：
A 完全不剪：ROOTS 417 份／集外 311 份；
B 旧形状复现（ROOTS 只剪 `_archive`、集外侧不剪）：372 份／311 份——这两个数逐字等于上一轮载体（2003 第 13、78 行），所以"旧代码究竟剪了什么"不是我在猜，是被现渲染复现出来的；
C 本轮（剪枝集只从条目名派生）：370 份／307 份，逐字等于本轮载体（2057 第 24、85 行）。
B→C 才是本轮的真实动作：**6 份 .md 退出判决面**（`docs/enterprise/_data/test-report-20260824/t4-enterprise-test-report-20260824.md` 一份 ＋ `_verification` 五份，逐份列在收据第 13–18 行），合计盲区 562 行（ROOTS 侧 479／集外侧 83，第 19–20 行），**主张 0、不闭合 0**，但**引文 1**——那一条在 `platform/domains/alliance/_verification/backend-fix-report.md` 里，所以集外侧的引文计数从 1 条变成 0 条（2003 第 78 行配 2057 第 85 行；收据第 22 行把这一条单独钉了一格 MUST）。等式判决一条没丢，引文这一维确实动了一条：本节登记它，不写"零影响"。反事实 A→C 是 **51 份**——"理由只是文字、不是剪枝"这句话的价码就是这个数。

**两条真缺陷，本轮都修了**：① 三条出账声明里只有 `docs/_archive` 那条被执行，`_verification` 与 `_data` 在读路里仍被扫进 5 份与 1 份；现在剪枝集由 `prune_dirs()` 从条目名现切（`entry_dirname()` 跳过 `**` 段取末段），`walk_md` 与 `outside_walk` 共用它，原先散在三处的字面量副本删净。② `docs/**/_verification` 那句范围是假的：实测命中落在 docs／frontend-ui／platform 三个顶层桶，条目名已改为 `**/_verification`，并新加一条"出账范围不符"的红钉住它（条目名里的范围也是一句判决，不许比实现宽，也不许比实现窄）。

**形状**：三条新红（`出账条目已失效`／`出账范围不符`／`出账执行账塌缩`）＋ 一条 §1.8k 就存在的 B 红泛化（`出账目录出现在扫描集里`，从前只认 `_archive`，现按条目名逐条判）。第七本账挂在 **git 真读**那条通道上（`dir_account_audit` 的 `real_read` 分支）：合成格喂的是让出面专用假名单，拿它判"每条都排掉了东西"会把假名单判成缺陷，所以 夹具W 同时把"合成格未参与"印出来——第 35 型的老规矩：0 红不等于没判。分母自身是断言（条目数、跟踪面、扫描集任一为空时前两条恒真），故"塌缩"必须自己成一条红。

**四枚破坏各钉一条通道、红格两两不同**（载体第 70 行）：① 撤派生剪枝 ⇒ 只红 `出账目录出现在扫描集里`（两条，点名 `_verification` 与 `_data`）；② 条目名从 `**/_verification` 收窄回 `docs/**/_verification` ⇒ 只红 `出账范围不符`；③ 植入一条指向"本轮 git 跟踪面里根本不存在的目录名"的出账条目（那枚名字写在仪器里，此处不抄路径——反引号里的路径会被文档链接尺当真扫，而它从来不是一个链接） ⇒ 只红 `出账条目已失效`；④ 跟踪面读到 0 份 ⇒ 只红 `出账执行账塌缩`。

**一实现两口径没有撑破**（载体第 82、84 行）：`--dir-account` 与 `--dircheck` 同轮各 rc=0，分母行仍只有一行、九个数次序未动，所以 变异体29 那枚"撤分母行"的针照旧红在它自己那格上。

**台账增量**（载体第 2、4、34、73、74、87 行；归因见 `pattern-site-attribution-2026-10-02-2051.txt`）：仪器 127765 B→**138766 B**／md5 `74ec3415a773`／零 CRLF；例数 54→**56**、FAIL 0；红模板 36→**39** 且未覆盖与死键均为空；模式位点 382→**391**、盲区 87→**96**。+9 不是猜的：用仪器自己的 `reco_sites()` 把源码按本轮动过的八个行段切片逐段数——`entry_dirname`/`entry_scope`/`prune_dirs` 2 处、`walk_md` 1、`outside_walk` 1、`dircheck_verdict` 1、`exclusion_verdict` 1、夹具W（含 1905–1931 的现量段）2、变异体32 1，合计恰 9，且 9 处全是解不出字面量的路径切分（`x.split("/")` 型）⇒ 算术命中 12／别家 0 一个都没动。该收据的 MUST 行里"9"是**从两份载体的差额现推**的，不是硬编——硬编的话下一轮加一行针就假红。

**新的盲侧（印出来，不许读成"出账全判了"）**：① §1.8m 那四条一条没判：未跟踪那一圈仍无账（夹具W 现印 SKIP 名单双口径，`ais` 跟踪 1 份／文件系统 5015 份，名单口径合计挡下 5032 份 .md；这些是 filesystem 快照，相隔几十秒的两张载体就能差几百份，谁复述谁重跑）、`coverage` 是仓根根本不存在的名单项（印 -1）、"理由与目录内容是否相符"与嵌套跳过仍无尺。② **`RECO_OUTSIDE_NESTED`（仪器源码第 1007 行）里还留着一份 `_verification` 字面量**——它剪的是代码语料不是 .md，是不是"同一事实的第二源"是一条裁决，不在本轮自裁。③ 出账声明一旦真的开始执行，**改条目名就等于改判决面的大小**：本轮把 1 条引文移出了面，而"该不该移出"没有任何一本账判——它只在 A→C 那种反事实里看得见。④ 载体头部那张行号表在 2040 那一版印的是 SELFTEST 块内序号、却写着"正文引用按这些行号点名"，照它引用会整表偏 17 行；2057 起表里同时印两种行号，并加了一格偏移自校（块内第一行必须逐字等于组装后的第 off 行），撤掉自校不会红在任何现有门禁上——这是驱动自己的盲侧，登记在此。⑤ `--ledger` 在本轮内被并发作者从 rc=0 打到 rc=1（托管块那行的 `.rs` 计数 1382→1383，Rust 侧新增一份文件），我按仪器自己的 `--ledger --write` 重渲染才回到 rc=0；这既是"文档镜像有牙"的一次实测，也说明**载体里所有与 .rs 有关的数只在生成那一刻为真**。

**仍未接线**：没进 `ci.yml`、没进 `check-all.ps1`（本轮实际未动这两份文件）。接线成本再 +2：目录账条目名改成跟踪面里不存在的目录名 ⇒ `出账条目已失效` 当场红；条目名删掉 `**` 段而命中仍在多个顶层桶 ⇒ `出账范围不符` 当场红。两条都是"改一句登记就红 CI"，与 §1.8l／§1.8m 记的那两条同档，接法与各自的账见 §1.8m 末段。

**发布态复跑（本节是被自己落盘之后的那张载体判的）**：上面引用的 `-2057.txt` 是**正文落盘之前**的图像——它与文档有关的读数（夹具F 那行的文件数／盲区／引文）跟着本节行数一起动了，所以本节另跑一张 `reports/data/selftest-formula-2026-10-02-2120.txt`，它在"§1.8n 正文 ＋ 托管块现渲染"都已在盘上之后生成，与文档有关的读数才是发布态。仪器图像一字节未动 ⇒ 那张载体的例数与红模板必须**等于** 2057 的（驱动里这是两条按内容从 2057 现取的 MUST，不硬编；等不到就是 SELFTEST 自己变了，不是文档变了）。同轮复跑全套 rc=0，其中 `--ledger` 本轮第二次被并发作者从 rc=0 打到 rc=1（`.rs` 计数再一次上升，Rust 侧又新增一份文件），仍只有仪器自己的 `--ledger --write` 能送它回 rc=0，而那条 MUST 是"复跑真判 rc=0"不是"写成功"——写成功与闸门通过是两件事。文档侧计数不钉进本节：它随正文行数走，钉进来就是下一轮的腐烂项；差额与逐项归因（哪几份 .md 是并发新增的、盲区差由哪一份文档自己贡献）印在那张载体里，谁复述谁重跑。

### 1.8o 第八本账：嵌套"跳过"的名单本身也是排除主张（夹具X ＋ 变异体33；读数载体 `reports/data/selftest-formula-2026-10-02-2313.txt`）

§1.8n 把目录账那句"排除"变成了判决，但同一类主张在仪器里其实写了四份名单：`prune_dirs()`（由 `DIR_ACCOUNT` 条目名派生，喂两条 .md 读路）、`OUTSIDE_SKIP`（集外**顶层**按名跳过）、`OUTSIDE_NESTED`（集外 .md 的**嵌套**跳过）、`RECO_OUTSIDE_NESTED`（识别器集外**代码**语料的嵌套跳过）。本轮要回答的是最后一份里那个手写的 `_verification`：它和目录账派生名是不是同一个事实的两个源。先按只读内存反事实量了一遍（工件 `reports/data/nested-skip-recon-2026-10-02-2215.txt`，那张印着四份名单的两两交集与逐名点火面）：

- 目录账派生名与代码名单恰好只重一名，就是 `_verification`；另两份（顶层桶与集外嵌套）与目录账零交集。
- 代码名单里只有 `_verification` 真挡住候选文件（载体 `reports/data/selftest-formula-2026-10-02-2313.txt` 的"名单现量"行点名它放出 7 份、自带算术记号的命中 0 处），`__pycache__` 只少走目录、不放文件，其余几名在今天的语料里从不点火。
- 把这些被挡的候选放出来再判一次，集外侧的算术记号命中一处也不多——今天没有第二把尺被藏住。

于是"合并"有了一张收据：把目录账派生名并进代码名单，候选集与合并前逐份相同。这句话不在散文里靠推理成立——载体有一段 A/B，用本轮退役前的那份仪器备份图像与盘上现图像在**同一时刻**各走一遍集外代码语料，比候选集、位点、算术命中、盲区四项。合并之后代码侧那份字面量不再重打目录账的名字；将来撤一条目录账条目时，代码语料跟着放出来，不会留下一份静默的副本。

**不选的形状与理由**（假阳的代价是人绕过闸门）：更凶的判据是"任何两份名单同名就算第二源"，它会当场红掉 `OUTSIDE_NESTED ∩ OUTSIDE_SKIP` 那一批名字，而那两份回答的是不同问题（顶层桶与嵌套目录），属假阳；另一种是"名单每名都必须今天真挡住东西，否则删条目"（与 夹具W 的失效条目同型），它会要求删掉 `node_modules` 一类防御项，而那类目录一旦放出就是几千份第三方 README 与打包 JS，删它比留着危险。所以本轮只把**能判的方向**判成红，防御项的处置交给维护者（裁决点 8，见本节末）。

**第八本账的口径**：`--recognizer` 每轮对名单里每个名字做一次"撤掉它再走一遍集外代码语料"的反事实；每名必须有一行读数，读数里出现自带算术记号的候选就红并点名文件行号。五条红各住一条通道，由 变异体33 逐枚打红且五枚红的前缀两两不同：

1. `代码名单重打了目录账的名字`——那份构建字面量与目录账派生名不许有交集；针是往 `DIR_ACCOUNT` 植一条名字与构建字面量同名的合成条目，只红这一格。
2. `嵌套排除名单塌缩`——名单为空时逐名读数与隐身判据都在空集上恒真，所以空名单自己成一条红（第 35 型）。
3. `排除代价账塌缩`——名单 N 名而反事实只读出 N-1 行，缺的那名等于没有代价记录。
4. `排除代价账分母塌缩`——集外候选读到 0 份。
5. `嵌套排除挡住了自带算术记号的候选`——本轮真正要的牙：排除可以挡掉派生副本，不许挡掉一把尺。

`RECO_OUTSIDE_NESTED` 现在是「构建派生字面量 并上 目录账派生名」，于是 `--recognizer` 多印两行自署：名单来源（各几个名、谁派生自谁）与逐名代价（真挡住候选文件的名字按名点名，从不点火的也按名点名）。名单长度、集外候选份数、模式位点数都是随语料动的数，本文一律不钉，只由上面那张载体现印。

**本轮自己撞出的三处**（都是驱动或判据的错，不是语料的）：

- 第一版逐名探针把差额算反了。撤掉一个剪枝名只会**放出**文件，于是"基线减新集"恒为 0，它给每个名字都印了"藏 0 份"，而两行之外的整体清空段却印出净放出若干份——同一份输出里自相矛盾才被看见。这正是第 31 条记过的形态：一批针里若没有一枚期望非空，该去查前处理通道而不是相信绿灯。修好后当场补一枚"逐名放出全为 0 就拒绝出报告"的有牙断言。
- 位点归因第一次按 `(行号, 模式, 通道)` 做集合差：插入两百行使既有行号全体位移，于是全部位点都"新增"。改按 `(模式, 通道)` 算多重集，本轮新增恰一处（自检里那句按前缀切红的 `split`，接收者非字面量故模式记 `None`，落盲区不落判分母），既有位点零消失。集合差不等于逐位差这条老规矩第 49 条记过，本轮是在自己身上又撞一次。
- 夹具X 第一版把"合并零代价"写成「只走构建字面量」与「走并好的名单」两副名单候选集相同——那个对照对象根本不是合并前的图像（合并前的名单本来就有 `_verification`），而要在仪器内复现那个基线就得重打目录账的名字，正是本轮要退役的第二源。判据因此改成判方向：合并新挡掉的份数照印，但挡掉的里面不许有尺。这条主张在盘上红过一次，修完才 PASS。

**新盲侧**（本轮印出、未处置）：

1. 去重把耦合做实了——`DIR_ACCOUNT` 今后改条目名会同时改变**代码**语料的扫描集。第八本账只印代价、不放行也不是全判：隐身红只在被挡者自带算术记号时响，一把不带全角记号的尺（比如 ASCII 口径的第二把）仍会静默隐身，那仍是裁决点 6 的形状。
2. 名单里那些今天不点火的防御项要不要各配一句理由（像目录账每项那样），没有尺判，登记为**裁决点 8**。
3. 只把代码语料那一侧的嵌套名单收进判决。`OUTSIDE_NESTED`（集外 .md 的嵌套跳过）仍是无人复算的一份名单，本轮只在普查里量了它的点火面，没进判决。
4. 反事实按名走整份语料，`--recognizer` 的成本随名单长度线性上升，这条耗时没有定价。

**接线状态**：未接 CI、未进 `check-all.ps1`。第八本账住在 `--recognizer` 里，而该模式本身不在 CI 执行宇宙（裁决点 6 未裁，见 §四）；它的 `--selftest` 一侧已接（§1.8e），所以五条新红每轮都被敲一遍。"每条判决都有针"与"这条判决每天有人踩"仍是两件事。

## 二、分阶段计划（每阶段自带验收判据；不跨阶段夹带重构）

### P1a 普查器（已完成，本轮落库）

- 落点：`scripts/gate/check-api-surface.py`，模式 `--census` / `--selftest` / `--check` / `--role-matrix`（见 §1.5，只读账）/ `--role-surface`（见 §1.6，按角色分桶）/ `--ledger`（见 §1.7，核心公式单一算源）。
- `--check` **故意拒绝执行**（rc=2 并打印理由）：判据的覆盖面按形状算，
  在 §3 的盲区清单归零或逐条定性之前把数字钉成门禁，会把"解析不了"读成"没有缺陷"。
- 验收（已达成，指 P1a 落库那一轮）：`--selftest` 9 例全绿，其中 4 例是变异体/正对照，且每一例都对应一个本轮真实撞出的缺陷形状：
  撤 `nest` 前缀必须改变结果、断 `let` 绑定必须点名、`let` 绑定但从未 merge 的 Router 不得计入对外表面、
  模块路径头名（`actuator::build_actuator_router()`）不得与同名 `let` 变量混淆。
  同一入口的用例数往后再没停过：§1.5 那轮 27、§1.6 那轮 36、§1.7 落库那轮 40（14:39）、同节补上渲染占位符判据后 41（14:52）、§1.8 识别器单源化后 43（15:20 现量）——
  引用例数要点名是哪一轮，只写"现量 N 例"的副本都会腐烂。
- 状态：**普查器有牙（自检可被打红），但没有判决权（`--check` 未开）**。CI 侧的口径自 §1.8e 起要分两半写：
  两台仪器的 `--selftest` **已接线**（`frontend-governance` 第 10 步，且"已接线"这句话本身由 夹具M 对着 ci.yml 复算），
  而 `--census`／`--dircheck`／`--recognizer`／`--check` 仍不接——接进去等于替维护者选定裁决点 6 的字形口径。
  这条"待 6"自 §1.8f 起是**实测而不是打包**：`--dircheck` 的依赖只在它的集外那半条通道上（目录账那半已由中性化反事实
  证明逐字节不变），所以裁 6 之前也能把目录账单拆单接。**这句自 §1.8k 起已不再是可能性**：
  `--dir-account` 是那条命令的落点（夹具T 真语料 ＋ 变异体29 五通道，见 §1.8k；§1.8l 起这条口径含第五本账：
  集外宇宙登记账 ＋ 夹具U ＋ 变异体30／30·甲；§1.8m 起再加第六本账：让出面账 ＋ 夹具V ＋ 变异体31／31·甲；§1.8n 起再加第七本账：出账执行账 ＋ 夹具W ＋ 变异体32），
  但本轮仍**没有**把它接进 ci.yml 或 `check-all.ps1`——
  接法两条与各自的账都写在 §1.8k 末段，等维护者裁决；§1.8l 之后接它等于同时把"新增带 .md 的顶层目录即红"接进 CI，
  那条成本与两种接法各自的账记在 §1.8l 末段；§1.8m 之后又加一条：**任何新被 git 跟踪的 .md 落在点目录或名单目录下**
  （既不在 ROOTS 也不在宇宙里）会让同一本账当场红"让出面漏登记"；§1.8n 起再加两条"改动即红"——目录账条目名改成跟踪面里不存在的目录名（`出账条目已失效`）与条目名删掉 `**` 段而命中仍在多个顶层桶（`出账范围不符`）；接线成本逐条与两种接法各自的账见 §1.8m 末段与 §1.8n 末段。

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
- **上面那条状态已经翻了一次，10-02 11:06:37 重量**（`git status --porcelain` 按名点六条 ＋ `git ls-files`）：
  三份 `reports/data/api-surface-census-{2026-09-30,2026-10-01,2026-10-02}.json` 已被并发作者随提交 `164e53eb`
  **入库**（不再是 `??`，也就有了恢复源）；本方案文档现为 ` M`、普查器现为 `MM`（第一列那个 `M` 是对方已暂存的版本，
  不是我这轮的改动）；而 §1.5／§1.6 新出的两份角色工件 `reports/data/api-role-matrix-2026-10-02.json` 与
  `reports/data/api-role-surface-2026-10-02.json` 仍是 `??` ⇒ **这两份没有 git 恢复源**，
  本文件 §1.5/§1.6 的读数只能靠重跑仪器复现，不能靠 `git checkout` 找回。
  这条翻覆本身就是"引用前重量、别复述上次量的索引状态"的实例。

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
| **同一支条件的角色归属**（"两支都走"不等于"知道哪支在哪个角色下装配"） | 两支都走 ⇒ 每条带臂路径同时有一条无归属读数，`--role-matrix` 因此印出"独占 0 条" | **10-02 已归因**（§1.6）：`cond:` 位置标签 ＋ 逐取值合取求值 ＋ 钉死走树第二通道；活语料现量分支区间 2 处／归不出的角色分支 0 处／带标签路径 359 条 | `COND_FIXTURE` ＋ `CHAIN_COND_FIXTURE` ＋ 变异体 16–20；**口径提醒：盲区账只覆盖"被走到的文本"，不是 1376 文件的全库扫**——要全库口径得另开一次普查，勿把 0 读成"全仓没有归不出的角色分支" |
| **一枚臂写多个枚举模式**（臂名与臂名之间用竖线相连的那一类） | 臂模式正则只认 `=>` 左边那一个名字 ⇒ 另一半凭空丢；析取若按加号记就成了合取 ⇒ 该臂永不可达 | **10-02 已归因**：竖线作 lookahead ＋ 标签按竖线连（求值时逐段判，任一段为真即该臂可达） | 活语料 0 命中（复算命令在 §1.6 第 2 条，`grep -rnE` 找"角色枚举名后紧跟竖线"的臂，装配语料里为空）⇒ 只有夹具那枚 `/multi` 钉住：它必须同时进 Kg 与 Cloud 两档 |
| `match` 臂、`upgrade(...)` 外壳里的裸 builder 调用 | 只取 `.route/.nest/.merge` 步骤，臂内树消失 | **已归因**：链步骤消费区间外的调用位点一并解析 | 变异体 5／7；`_steps_in` 消费区间防重复挂载 |
| 非 `build_*` 命名但返回 `Router` 的装配节点 | 按名字认，整类静默 | **已归因**：按签名 `-> Router` 在册；10-02 现量在册 72 ＝ 名字 60 ＋ 签名独有 12（按签名匹配的 31 个里有 19 个本来就按名字在册，分解要闭合请看 §1.1） | 变异体 5；`indexed_by_signature` |
| 状态升级壳（`upgrade` 的体只做 `with_state`） | 会被读成"无法归类的表达式"（26 条噪音） | **已归因**：判据从函数体本身推（无链步骤且无装配调用才豁免），豁免计数打印 `passthrough_shells` | 变异体 8（撤豁免必落盲区）；10-02 仍 26 次 |
| **从未被调用的 builder（幻影对外面）** | 未查 | **10-02 新撞出，普查器口径正确地不收**：`platform/gateway/mox-platform-gateway-svc/src/system/approval.rs:386` 的 `pub fn build_approval_router()` 写着 7 条 `/api/system/approval*` 字面量（388–393 行），而全仓 `build_approval_router` 的引用数只有定义本身那 1 处——真正挂载的是 `src/system/mod.rs:530-536`（同 7 条路径、同一批 `approval::` handler）。仪器只走可达根，所以这 7 条没进差额，**是对的**；危险在 P1 的 (B) 路线：若 ROUTES 改为"静态扫全部 builder 导出"，这 7 条会以幻影条目进对外清单 | 复算入口：全仓 `grep -rn build_approval_router`（期望命中数＝1，即定义本身）；这条形状目前**只有活语料证据、没有夹具**，挂 P1。全仓按同一形状量到 **3 处**（其中 2 处是完全不可达的管理面），见 §1.4 |
| 形参名与别处同名函数撞车 | —（尚未暴露） | **已判为假阳并拦下**：调用位点判据（末段后必须 `(`/`::<…>(`，前不能是 `.`） | `COLLIDE_FIXTURE` ＋ 变异体 7；现量证据＝本轮曾把 371 顶成 429 |
| 同名装配节点多定义 | 取首个并点名 | 现量 0 条（名字池污染收干后不再有歧义名被走到） | `unresolved` 键集为空 |
| 语句切分按顶层 `;` | 宏内/闭包内 `;` 可能切错 | **仍是盲区**，但撞出即以 `unresolved` 露出，不静默 | 无夹具：这条余账挂 P1 |
| 同一 role 分桶的独立表面（HostRole::Kg/Cloud/Kb/Iam 各跑一遍） | 未跑 | **部分做到，且不能读过头**：`--role-matrix`（10-02 10:14:47 现量）把 5 个具名臂的归属读出来了（375 条里 111 条有归属，Iam 76／Kb 18／Kg 13／Cloud 4），但普查根是 `build_host_router(state, All)`，`if role == All` 两支都走 ⇒ **每条有归属路径都有同 site 的无归属读数**（同 site 111／不同 site 0／根本没有 0），所以"某端点专属某角色"这条问句现在答不了。见 §1.5 | `ROLE_FIXTURE` ＋ 变异体 9–15；三档分账由"根本没有＝role_only 两条独立代码同集合"这条不变量钉住。按角色参数化各走一遍仍**未做**，挂 P1 |
| 链的接收者绑在 `let` 上（`let r = match role {…}; r.merge(c)`） | 只解析 `.merge(c)`，各臂的树整支消失 | **已归因（本轮宽化）**：链步骤照走 ＋ 接收者在 env 里就展开一次；活语料现量 0 次命中（尾项形态是 `route_layer`，走的是既有的"头回查 env"递归），改前后 `--census` 20 键除 `generated_at`／`scanned_files` 逐位相同 | 变异体 14（撤接收者展开 ⇒ 只剩 `/common`＋`/api/kbdoc`）＋ 基线夹具 7 条路径齐全 |
| 臂里嵌第二把 `match`（模式段与臂体混为一谈） | 外层臂标签把内层臂名一起吃进来 | **已归因**：只从顶层 `=>` 之前的模式段取标签（`_top_arrow`），内层 `_ =>` 不再被扣上内层枚举名；同时单字符变体（`CloudTier::A`）要能匹配（`\w*` 而非 `\w+`） | 变异体 12／13；夹具 `/cloud-a`（两维齐全）与 `/cloud-x`（只有外层维）互为反证 |
| **本文件散文里手写的核心公式**（形如"总数 ＝ 甲 ＋ 乙"的账以文字存在，语料一动就腐烂，且此前没有任何尺子读过它们） | 未查——§1.1／§1.5／§1.6 各自零复述，复述之间无交叉核对 | **已收干（§1.7，`--ledger`）**：托管块里每一条等式都由同一个算源字典现算（条数与红数由块首行印出，散文不重打＝不留第二个源；改一个被两行引用的加数会让那两行同时红，这就是"单一算源"的机械含义），引用逐个解析到 `reports/data/` 工件的点分键，文档托管块由仪器逐字节写入并断言块外逐行未动 | 变异体 21–27（第 25 枚专打"渲染留未替换占位符"，镜像判据对它是盲的；第 26 枚专打托管块的 0 基／1 基换算——换算错了不报红，只把块外真主张整条吞掉；第 27 枚专打"本地不许留第二把尺"）。**三处盲区要点名：其一，主张只认全角 `＝` 配 `＋`，ASCII 等号与拉丁标签写法只出账不判红（见下一行）；其二，`「…」`／`“…”` 内的是**引文**（登记"某轮曾印错成什么"），不复算只按名点名——引文里允许合法地写着错的等式，别把它当现量读；其三，减法与不等式（`－ ≥ ≤`）明说不在射程内，落进 `noise` 档而不是被猜一个值** |
| **全库散文里的核心公式**（同一族账在 `docs/` 其余三百多份文件里，本仪器的托管块看不见） | 未查——此前没有任何尺子读过 | **已量出并收成一份口径（§1.8）**：识别与复算在 `scripts/gate/formula_ledger.py`，`--ledger` 与全库普查共用；读数只点名载体不复算——`reports/data/doc-formula-census-2026-10-02.json` 的工件键 `generated_at`／`files`／`claims`／`violations`／`blind`／`quoted`（外加 `outside` 一圈集外账）。**这里原先手抄的五个数已退役**：编辑本文件自己就会挪 `blind`（§1.8 那条律），抄一次就腐烂一次 | 七种盲区各配夹具与变异体，另有一枚专打**假阴方向**（撤掉 `≥` 遮蔽后不等式会被读成一条闭合主张，绿灯里读不出异常）。**盲区：`ascii_eq` 与 `latin` 那两档确实在写等式却不敢判红，连底迁全角还是加第二把尺＝§四 裁决点 6，未裁 ⇒ 普查器的判决模式未接 CI（`--selftest` 一侧已接，见 §1.8e）** |

---

## 四、评审需要的六个裁决点

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
5. **ROUTES 表要不要承载角色维**（§1.6 新撞出来的议题，10-02 11:01 现量，
   工件 `reports/data/api-role-surface-2026-10-02.json`）：`pub static ROUTES: [ApiRoute; N]` 按构造
   与 `MOX_HOST_ROLE` 无关，而装配侧的真实对外面按取值是 All 375／Kg 29／Cloud 20／Kb 34／Iam 92，
   且五档里只有 16 条是角色无关的（L0 四条 ＋ `/actuator` 12 条）。三选一：
   (a) 给 `ApiRoute` 加 `roles` 字段并在登记时逐条填（登记表与装配侧同轮改，手术量落在 actuator.rs 那整张表（条数见托管块 `DECL-PARSE`），
   但角色是"由位置推出来的"而非"由作者声明的"，声明与实现仍会漂）；
   (b) ROUTES 保持角色无关，另由仪器生成一份按角色的对外清单（即 §1.6 这台仪器出账，登记表不动）——
   代价是它仍未入库、且每次装配写法变化都要重跑；
   (c) 认定"对外面按角色分"这件事不该进登记表，只在部署文档里说明各角色进程的面。
   本文件不推荐任一档：(a) 与 (B) 路线（裁决点 1）会同时改 actuator.rs，两笔账若不同轮就互相顶红；
   (b) 要先把这台仪器接进 CI（它现在故意不接），接线前需要维护者判"分档账是不是门禁口径"。
6. **散文里的等式用哪种字形当判决口径**（§1.8 量出来的新议题，工件
   `reports/data/doc-formula-census-2026-10-02.json` 键 `blind_reasons`——逐档读数只在那里出账，
   本文不复述：本文件每改一行都可能改这些计数，钉住＝当场造第二份会漂的源）：识别器现在只把
   全角 `＝` 配 `＋` 的写法当**主张**（判红），ASCII `=` 那**百级**与拉丁标签那**千级**条只出账不判红。三选一：
   (a) 把 ASCII 族连底迁到全角记法（等于给三百多份文档做一次写法归一，改动面最大，但与并发作者必然顶车）；
   (b) 给识别器加一把 ASCII 尺并逐处人工定性哪些是真等式（人工量落在"百级＋千级"这批上，一把尺两种字形＝误判风险回升，
   本轮第一版就是这么写出 112 条假账的；§1.8b 的 `--recognizer` 已经把 ASCII 半把尺收窄成"转义加号＋等号＋数字"同现，
   选这条就要同时重钉那枚 benign 对照，否则端口门禁全表判成第二把尺）；
   (c) 认定 ASCII 族属代码／配置语料而非散文账，维持现状只出账（最便宜，代价是那部分等式永远没人复算）。
   本文件不推荐任一档；裁决前**三条判决模式**（`--census`／`--dircheck`／`--recognizer`）不接 CI，因为接进去等于替维护者选定口径
   （两台仪器的 `--selftest` 一侧已接，见 §1.8e）。**"哪条模式真读字形"自 §1.8f 起是实测而不是打包宣称**：
   把词汇表换成中性化替身跑两遍，`--dircheck` 的目录账那半条通道逐字节不变，只有它的集外等式账那半条会被 6 移动——
   所以选 (a) 之后要重量的对象是那半条通道与 `--recognizer` 的收窄，而不是整台普查器从头改尾。


# 专家联盟事件交付与恢复契约

核验日期：2026-10-03。本文是事件订阅主题的增量权威；整体实现仍以 [当前架构](CURRENT-ARCHITECTURE.md) 为准。产品模块与需求编号沿用 [企业功能登记](../modules/enterprise-capabilities/README.md)，不另建一套模块目录。

## 1. 模块边界与单一事实源

| 模块 | 唯一职责 | 权威实现 | 不承担的职责 |
|---|---|---|---|
| 计划、专家业务处理器 | 校验身份、推进业务状态、发布事实事件 | `experts_orchestration.rs` / `experts_registry.rs` | 不以投递成功决定业务成功 |
| 进程内事件总线 | 有界广播同一事件信封 | `experts_events.rs::EventBus` | 无持久队列、跨实例路由或补发保证 |
| SSE 出口 | 认证租户过滤、业务命名帧、缺口通知、有界持久续传 | `experts_streams.rs` / `event_replay.rs` | 不执行任务、不保证跨实例实时广播 |
| 前端传输契约 | Bearer 身份、UTF-8 分帧、资源释放、缺口回调 | `contract/event-stream.js` | 不持有业务列表或生成业务结果 |
| Vue 生命周期适配 | 从现有认证 store 取令牌，身份变化或组件释放时停止 | `composables/useAllianceEventStream.js` | 不自动订阅任何视图 |
| 页面恢复控制器 | 开流/缺口合并权威刷新、内存游标、错误状态与显式重连 | `contract/event-recovery.js` | 无自动重试、跨刷新游标或可靠消费回执 |
| 请求归属栅栏 | 身份代次与同出口请求版本控制迟到回填 | `model/request-fence.js` | 不取消服务端已经执行的操作 |
| 事件日志消费者 | 异步写 SQLite 可观察轨迹 | `spawn_event_log_consumer` | 不是与业务写入原子提交的 outbox |
| Webhook 管理 | 管理员角色、认证租户、分页查询、SQLite 提交后更新热投影 | `experts_streams.rs` / `EventBus` | 不授予跨租户管理、不以日志代替提交 |
| Webhook 出站策略 | 运维显式授权精确 origin，创建和投递均校验 | `webhook_policy.rs` | 不由租户管理员自行授权出站源、不提供 DNS 地址钉住 |
| Webhook 派发 | 对恢复后的同租户目标真实 HTTP POST，禁止重定向和环境代理 | `spawn_webhook_dispatcher` | 没有签名、死信队列或可靠投递保证 |
| 控制台订阅模块 | 契约端点 → API 严格投影 → 独立 Pinia store → 订阅组件 | `WebhookSubscriptions.vue` / `alliance-webhooks.store.js` | 不自动创建目标、不自动重试创建，不将保存当作送达 |

```mermaid
flowchart LR
  B[计划与专家业务处理器] --> E[单一事件信封与有界总线]
  E --> S[SSE 租户过滤]
  E --> L[SQLite 异步日志]
  E --> W[Webhook HTTP 派发]
  S --> T[前端流传输契约]
  A[现有认证 Store] --> V[Vue 生命周期适配]
  V --> T
  T --> R[页面重新查询权威列表]
  R --> Q[原有业务查询 API]
```

## 2. 请求与事件契约

入口为 `GET /api/alliance/events/stream`。原生 fetch 使用端点目录中的完整 `/api` 路径；`requestPath()` 为 Axios 去除 `/api` 的相对路径，不能直接用于原生 fetch。必须携带 `Authorization: Bearer <当前令牌>`；缺少身份时前端不发请求，后端认证仍为最终裁决。租户来源为后端认证身份，不接受查询参数租户覆盖。

业务帧字段为 `id: <真实事件ID>`、`event: <类型名>`、`data: <JSON信封>`。类型目前仅 `PlanCreated`、`PlanStatusChanged`、`ExpertRegistered`、`ExpertDisabled`。信封包含 `id/type/source/tenant/occurred_at` 及该类型载荷。`id` 可用于已持久化事件的显式续传，边界以 §8 为准。心跳注释不触发业务回调。

当 broadcast 积压导致缺口，发送 `event: StreamGap` 与 `{"reason":"lagged","action":"refresh"}`；不暴露全局丢弃数量，避免泄漏其他租户事件活动量。前端交给 `onGap`，不得把它解释为业务完成。序列化失败记错误并跳过，不能发送空对象冒充有效业务信封。

前端支持 LF、CRLF、CR、跨网络分块和多行 data；仅去掉字段冒号后的一个协议空格，保留业务文本。单帧上限为 1 Mi 字符，畸形 JSON、无效 UTF-8、错误响应类型和 HTTP 失败关闭连接并调用 `onError`。不生成替代结果。

## 3. 生命周期与业务恢复流程

```mermaid
flowchart TD
  A[页面显式 start] --> B{存在当前登录令牌}
  B -->|否| X[onError 提示登录]
  B -->|是| C[关闭旧连接并创建 AbortController]
  C --> D[真实 fetch 订阅]
  D --> E{状态与响应类型有效}
  E -->|否| X
  E -->|是| F[onOpen 后查询权威列表]
  F --> G[逐帧解析]
  G --> H{帧类型}
  H -->|业务事件| I[onEvent 提示刷新相关实体]
  H -->|StreamGap| J[onGap 全量刷新该页面查询]
  I --> G
  J --> G
  G -->|服务端结束| K[onClose 标记连接已断开]
  G -->|解析或网络错误| X
  D -->|stop 或身份切换或作用域释放| Z[Abort 取消请求与读取并释放 reader]
```

传输接口支持 `onEvent/onOpen/onGap/onClose/onError` 和 `start/stop`。重复 start 取消前一连接；旧连接不得回调到新订阅，正常 stop 不当作网络错误。专家页与编排页通过恢复控制器执行开流/缺口刷新、显示错误并提供显式重连；身份变化与作用域释放清除内存游标。精确行为与验收范围见 §9。没有自动重连或退避定时器。

## 4. 验收要求与未完成项

| ID | 验收要求 | 本轮状态 |
|---|---|---|
| EA-EVT-01 | 完整端点、真实登录令牌、缺少身份不发请求 | 已实现；Node 真实 TCP 验证 |
| EA-EVT-02 | 分帧、中文、空白、多行 data、异常与帧上限确定 | 已实现；纯函数验证 |
| EA-EVT-03 | 重复启动、停止和旧结果隔离 | 已实现；Node 真实 TCP 验证；Vue 身份钩子编译检查 |
| EA-EVT-04 | 同租户事件、真实 ID、明确缺口且不泄漏全局数量 | 已实现；真实 broadcast / SSE body 验证 |
| EA-EVT-05 | 页面刷新、断线可见、退避、恢复后的状态一致 | 视图已接入实时事件；缺口、可见错误、游标管理及真实浏览器恢复仍待验收 |
| EA-EVT-06 | 业务写入与 outbox 原子提交、跨实例消费、去重、重放 | 待开发；现有日志消费者不满足 |
| EA-EVT-07 | Webhook 管理授权、出站目标策略、防重定向与 DNS 重绑定、签名、持久订阅、死信与幂等 | 部分实现：授权、精确 origin、禁重定向、持久提交及控制台已核验；DNS 地址钉住、签名、outbox、死信、幂等仍待开发 |

后续优先顺序：出站 DNS 地址钉住与签名、持久 outbox 和投递幂等，再验证跨实例消费与恢复。前端不得拿 SSE 成功证明业务成功、Webhook 成功或外部模型成功。无跨实例、远程 CI 和完整浏览器端到端结论。

本轮证据见 [验证报告](../../reports/markdown/20261002-alliance-event-contract.md)。需求追踪关联 EM-09 执行、EM-10 协作与 EM-26 运维；这些模块整体仍按企业登记中的独立验收项逐项交付。

## 5. 2026-10-03 Webhook 管理增量与业务流程

三个管理动作共享现有 JWT 身份和 `RbacAction::ManageWebhooks`：GET/POST `/api/alliance/events/webhooks`、DELETE `/api/alliance/events/webhooks/:id`。未认证 401；非 `super_admin/tenant_admin` 角色 403；跨租户删除 404。租户只取认证上下文，超管也不通过请求参数切换租户。拒绝复用现有 RBAC 审计。

创建体为 `{url,event_types}`；URL 上限 2048 字节，HTTP(S)、无用户信息、查询参数或 fragment；过滤值只收四种真实事件，空数组表示全部。出站源配置及部署操作以 [部署模板 §2.6](09-deployment-templates.md#26-网关本地存储附件非联盟专属按需) 为权威。授权按 scheme/host/effective port 精确匹配，拒绝子域和端口漂移；仅创建、投递允许的 origin，由运维环境配置控制，默认拒绝全部。运维可显式授权内部接收源，因此这不是「所有内网地址永久禁止」策略；域名目标仍依赖受信 DNS，未做解析后 IP 钉住。

GET 返回 `{webhooks,total}`，按 `created_at,id` 排序，支持既有 `page/page_size`，默认 20、上限 200。对象 `{id,tenant,url,event_types,created_at}` 由前端严格投影为 `{id,tenantId,url,eventTypes,createdAt}`；格式错误显式失败。列表来自本进程投递投影，不能据此声称跨进程一致性。

```mermaid
flowchart TD
  UI[控制台显式读取或保存或确认删除] --> EP[端点契约与 API 信封归一]
  EP --> AUTH[真实 JWT 与认证租户]
  AUTH --> RBAC{管理员角色}
  RBAC -->|否| DENY[401 或 403 与拒绝审计]
  RBAC -->|是| OP{管理动作}
  OP -->|读取| LIST[同租户排序分页]
  OP -->|创建| VALID[精确 origin 与事件类型校验]
  OP -->|删除| OWN[同租户对象校验]
  VALID --> SQL[SQLite 实际写入]
  OWN --> SQL
  SQL -->|失败| FAIL[503 保留原投递投影]
  SQL -->|成功| MEMORY[更新或删除热投影]
  MEMORY --> OK[返回真实提交结果]
  LIST --> OK
  OK --> STORE{响应仍属于当前身份}
  STORE -->|是| VIEW[更新订阅页面]
  STORE -->|否| DROP[丢弃旧响应]
```

状态提交与事件投递是两条流程：SQLite 提交后才变更本进程目标；重建 state 从库恢复合法过滤器。过滤 JSON 损坏时跳过并记录错误，不能退化为空数组而扩大订阅。投递逐事件选取同租户、类型匹配目标，再检查当前 origin 策略，真实 POST；client 禁用重定向和环境代理，5 秒超时，最多两次尝试，中间等待 300ms。删除不会撤销已经取得目标快照或进入网络的请求。启动读库仍有旧 best-effort 路径，不能宣称依赖故障时启动健康已闭环。

控制台通过 `WebhookSubscriptions` 与原任务、调度模块融合；没有额外路由或第二套权限系统。显式读取订阅，保存经真实接口确认后才清空表单，删除需要二次确认。已确认的写入与后续刷新失败分别提示；令牌、租户、用户切换清空地址和待删除确认，旧响应不进入新身份。创建请求丢失响应后仍可能出现提交不确定，需要重新读取核对，当前尚无创建幂等键。

| 验收项 | 实际证据 | 当前边界 |
|---|---|---|
| EA-WH-01 管理授权与租户隔离 | 真实 JWT/TCP，普通用户 403、异租户 404；伪造前端角色仍被服务端拒绝 | 本租户管理员管理，不含个人 ACL |
| EA-WH-02 写失败不假成功 | SQLite INSERT/DELETE 触发器真实报错，503、投影及重载状态不变 | 同进程热投影，不是分布式事务 |
| EA-WH-03 出站源策略 | 未配置、其他端口、凭据、query、fragment、非 HTTP 协议 400；真实 302 接收端不被跟随 | DNS 钉住与签名尚未实现 |
| EA-WH-04 合法过滤与持久恢复 | 实际 A 事件送达，B 租户及不匹配类型不投递；损坏过滤不扩大订阅；删除后重载不恢复 | 无可靠队列、恢复不等于进程崩溃测试 |
| EA-WH-05 前后端融合 | 实际 Pinia/Axios → TCP → Rust → SQLite；切租户迟到响应、CRUD、注销不发请求 | 真实传输链路，尚无完整浏览器交互验收 |

本轮增量证据见 [Webhook 模块报告](../../reports/markdown/20261003-webhook-management.md)。本模块只是 EM-02/EM-10/EM-26 的部分验收，不提升为全模块完成。后续还需创建配额/限流、幂等、独立健康状态及存储 IO 调度，解决当前同步 SQLite 与串行投递在压力下的阻塞。

## 6. 2026-10-03 有界投递与存储调度增量

前节是此前管理模块快照。本节覆盖同主题的性能现状：订阅创建、删除、查询与排序经 `spawn_blocking` 执行，等待 SQLite 提交后才响应；join 失败返回 503 并提示结果未确认，不能假定没有提交。异步事件日志消费者也在线程池执行 SQLite 写入，避免忙锁等待阻塞 Tokio 工作线程。订阅 UUID 不可变且没有更新入口，注册/删除仅在提交之后短暂锁定热投影；不持注册表锁等待数据库。并发重复删除可能同时确认已删除，不产生重新订阅。

Webhook dispatcher 使用既有 `FuturesUnordered`，共享 client 与一次序列化载荷；在途数量按 [部署参数](09-deployment-templates.md#26-网关本地存储附件非联盟专属按需) 限制，不为每个事件无限 spawn。最多保留一个事件的订阅目标快照；满槽等待完成后再读广播。存在空闲槽时，慢目标不会阻塞另一个租户的快目标；全部槽被慢目标占用时仍会排队，未实现租户独立配额或公平调度。广播积压会丢事件，现在明确记录 lag 告警，仍不具备可靠队列。

并发改变完成顺序：默认不能保证同订阅事件按发布时间到达，接收方应按事件 ID 去重、按权威查询读取业务状态，不能把迟到事件覆盖为当前状态。设置并发 `1` 保留全局串行行为，但会恢复慢目标阻塞。签名、严格有序消费、durable outbox、投递回执、死信与跨主机恢复仍需单独实现。

```mermaid
flowchart LR
  E[有界 broadcast 业务事件] --> S[认证租户与类型选择目标]
  S --> P[一个事件的目标快照与共享载荷]
  P --> C{还有在途槽}
  C -->|有| W[共享 HTTP client 并发投递]
  C -->|无| WAIT[等待任务完成]
  WAIT --> C
  W --> R[真实响应或超时与最多一次重试]
  R --> RELEASE[释放槽并继续读取广播]
  SQL[订阅命令与异步事件日志] --> B[spawn_blocking 存储执行]
  B --> COMMIT[SQLite 提交或真实失败]
  COMMIT --> M[短临界区热投影与 HTTP 回执]
```

EA-WH-06：真实 TCP 慢接收端保持未释放时，另一租户在空闲槽上到达；设上限 2，六个慢事件压力下接收端观察峰值始终为 2。EA-WH-07：单线程 Tokio 运行时、另一 OS 线程实际持有 SQLite IMMEDIATE 写锁，订阅请求真实等待期间探针 HTTP 仍响应；事件日志写入和目标快照同时与忙锁重叠。测试使用本机临时服务和数据库，是调度回归及局部延迟样本，不是生产 SLO、跨主机或可靠事件交付验收。证据见 [架构与性能报告](../../reports/markdown/20261003-architecture-performance.md)。

## 7. 专家管理提交与事件记录原子化（2026-10-03）

EA-EVT-08：注册与软删除使用同一 SQLite 事务提交专家单行和完整事件信封至 `alliance_event_log`。任何专家行或事件行写入失败均回滚并返回 503；不修改专家热投影、不广播成功事件。更新没有新增事件类型，同样使用检查结果的单行写入。数据库提交成功后更新热投影，向既有总线广播同一事件 ID；原日志消费者重复记录由既有 event_id 唯一键去重。其他连接新增的同租户专家行不会被这三个入口整表覆盖。

这三个管理命令通过 `spawn_blocking` 执行 SQLite 和内存提交段，避免同步数据库等待占用 Tokio 工作线程。提交段仍持共享专家注册表互斥锁，因此依赖该投影的读操作可能等待；本轮没有宣称读写隔离性能。阻塞任务 join 失败返回“结果未知”，应读取核对，不可把它当作未提交。

```mermaid
flowchart TD
  A[JWT 租户与管理员授权] --> B[阻塞线程构造候选专家]
  B --> C[SQLite 单行写事务]
  C --> D[注册或禁用的事件信封写入同事务]
  C -->|专家写入失败| R[回滚并返回 503]
  D -->|事件写入失败| R
  D --> E[事务提交]
  E --> F[更新内存投影]
  F --> G[广播同一事件 ID]
  G --> H[HTTP 成功回执]
```

EA-EVT-09：实际 JWT/TCP/临时 SQLite 测试在不启动事件日志消费者的情况下验证提交后事件立即可查；真实 INSERT/UPDATE/事件 INSERT 触发器故障验证回滚、内存不变及无成功广播，另验证独立连接新增行保留和跨租户拒绝。详见 [专家事务报告](../../reports/markdown/20261003-expert-atomicity.md)。

该增量是持久事件记录，不是可靠投递 outbox：提交后、广播前进程退出仍可能使订阅者漏收，当前没有扫描重放、消费租约、投递回执或死信。计划事件尚未改为同事务；即时咨询计数仍走旧快照保存。不同进程对同一专家的更新尚无修订号/CAS，内存投影和配额也未跨进程统一，不能标记分布式完成。

## 8. SSE 持久续传契约（2026-10-03 增量）

EA-EVT-10：现有 GET /api/alliance/events/stream 接受可选 Last-Event-ID；不带游标保持实时订阅。游标来自已处理的业务帧，长度 1–256 ASCII 可见字符，不接受查询参数覆盖租户。按认证租户在同一 SQLite 读事务定位游标，再按持久追加顺序读取后续事件，最多 200 条，总载荷不超过 2 MiB，单条不超过 1 MiB。超限返回 409，要求刷新权威状态后以无游标方式建立实时流。该限制不是事件保留时间承诺。

未知或其他租户游标统一返回 410，不暴露其他租户存在性；非法游标返回 400，存储/载荷损坏返回 503，不把故障伪装成空历史。建立实时订阅后读取历史快照，先发送历史，再读取广播，去除快照事件 ID 与广播重叠。去重集合只含本次游标及最多 200 条历史 ID，不能增长为无界客户端状态。

已持久化专家注册/禁用可补发；计划事件仍由异步 best-effort 消费者记录，没有原子提交保证。广播积压仍返回 StreamGap，需刷新权威查询；这不是 Webhook outbox，也不保证 SSE exactly-once、跨实例即时广播或全局发生时间排序。前端本轮先提供显式续传选项，不自动重连、不自动持久化游标。

前端显式调用 `start({ lastEventId: 已处理信封.id })`；`start()` 保持实时语义。非法游标在发送请求前拒绝，HTTP 错误通过 `onError` 的 `error.status` 暴露状态，调用方可区分 409/410 的刷新需求与 503 的存储故障。引擎不保存游标，调用方必须按用户和租户隔离；认证变化仍关闭旧连接。网关只在现有明确 origin 策略下允许 `Last-Event-ID` 请求头，未扩展允许源。

```mermaid
sequenceDiagram
  participant C as 已认证调用方
  participant H as SSE 网关
  participant B as 实时广播
  participant D as SQLite 历史
  C->>H: Bearer 与 Last-Event-ID
  H->>B: 先订阅实时事件
  H->>D: 同租户游标定位与有界读快照
  D-->>H: 历史或 410/409/503
  H-->>C: 按追加顺序补发历史
  B-->>H: 新事件及可能重叠 ID
  H-->>C: 去除历史重叠后发送实时帧
```

EA-EVT-11：真实 JWT/TCP/SQLite 验证游标隔离、补发/实时衔接、重复 ID 去除、200 条边界、数量/字节超限、损坏载荷与实际读库故障。实际前端传输引擎连接该 Rust 服务，验证补发、410 回调和非法游标拒绝；生产 CORS 层真实 OPTIONS 验证允许源预检及未配置源拒绝。完整证据见 [SSE 续传报告](../../reports/markdown/20261003-event-resume.md)，不是完整浏览器 DOM 或进程崩溃测试。

## 9. 页面恢复与迟到响应归属（2026-10-03 增量）

EA-EVT-12：专家页与编排页复用 createAllianceEventRecovery，状态唯一集合为 idle/connecting/refreshing/live/disconnected/error。每次成功开流重新查询页面权威数据；StreamGap 清游标并合并刷新。同连接代次只运行一个刷新 worker，突发缺口只保留一个待刷新标记。查询失败保持可见 error，不生成成功数据。实体事件防抖刷新仍由业务 store 持有。

业务帧只在当前页内存保留最近 ASCII 事件 ID，显式重连才携带。409/410/401/403 清游标并提示操作；用户触发重试。stop、身份变化和组件释放清游标与连接。fetch 使用 cache=no-store，避免可缓存的 410 阻止再次请求。不提供自动重连、跨刷新游标或持久消费位点，业务帧不构成工具执行或业务提交的回执。

EA-EVT-13：专家预约/统计/能力目录与编排请求采用共用请求归属栅栏；同出口只允许最新请求回填，身份改变使旧请求失效。专家列表/收藏沿用已有身份代次和请求序号；指标/匹配令牌同时失效。编排重置使在途回执失效。该保护不取消服务端操作，不解决跨实例 CAS、计划提交原子性或其他模块全部异步回填。

```mermaid
flowchart TD
  START[用户连接或重连] --> STREAM[同身份 SSE 可选内存游标]
  STREAM -->|开流| REFRESH[合并查询权威列表或统计历史]
  STREAM -->|StreamGap| CLEAR[清除内存游标]
  CLEAR --> REFRESH
  REFRESH --> OWN{当前身份与最新请求}
  OWN -->|是且查询成功| LIVE[回填并显示已连接]
  OWN -->|否| DROP[丢弃迟到回填]
  REFRESH -->|查询失败| ERROR[可见错误与重连入口]
  STREAM -->|409 410 401 403| ERROR
  ERROR -->|清游标后用户重试| START
  STREAM -->|身份变化或卸载| STOP[停止连接并清除游标]
```

新增验收使用生产 Rust 路由、启用的 JWT、临时 SQLite、真实网络故障与独立浏览器。广播缺口通过测试专用路由占满真实 EventBus 注入，该路由不进入产品。迟到统计通过延迟真实响应验证，不替换返回内容。既有单元测试仍有历史测试替身，其通过不作为真实业务完成证据；新收藏读取直接调用 Rust。运行结果与范围见 [页面恢复报告](../../reports/markdown/20261003-event-page-recovery.md)。

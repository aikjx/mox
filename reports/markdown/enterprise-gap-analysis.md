# 企业级能力差距分析与设计（阶段一）

> 日期：2026-09-27 · 仓库 infotopograph（Rust workspace 149 crates，网关 3080 / 编排器 3001）
> 方法：以代码事实为准。每条结论引用源码文件:行号或端点。不造桩、不夸大。
> 状态标记：✅ 已闭环（有真实实现）｜⚠️ 部分（有骨架但缺关键环节）｜❌ 缺失。

---

## 0. 基线速览（已确认事实）

- 网关路由注册 `platform/gateway/mox-platform-gateway-svc/src/actuator.rs::ROUTES`（约 236 条），`/actuator/mappings` 为视图。
- IAM 真实 SQLite 仓储：`system/`（user/role/dept/post/menu/tenant/config/dict/operlog/logininfor/security），RBAC 走 `IamRepository`。
- 企业功能统一挂 `/api/enterprise/*`（`enterprise_features.rs:99-135`）：integration / designer / sso / message / document / admin / scheduler / config / dictionary / mailer / org / files / operation-logs。
- 前端 9 内核模块（`frontend-ui/src/modules/index.js:3-11`）：expert-alliance、admin-lowcode、project、ai、graph、workflow、market、operators、system；管理面板 18 个（`views/admin/panels/`）。
- 近三轮已交付：存储/LLM 管理面 8 端点（`backend-storage-llm.md`）、存储抽象+S3 adapter（`oss-storage-backends.md`）、LLM 配置持久化（`llm-config-persistence.md`）。

---

## 1. 六维差距矩阵

### 1.1 安全与合规

| 能力项 | 状态 | 企业级要求 | 差距 / 证据（文件:行） | 建议 |
| --- | --- | --- | --- | --- |
| JWT 验签 | ✅ | HS256/RS256 真验签、alg 白名单、恒定时间比较 | `auth.rs:73-133` 已实现真 HMAC-SHA256，拒绝 alg=none/篡改/过期/错 iss；含回归测试 `auth.rs:311-388` | 保持；生产强制关闭 dev_mode |
| 口令哈希 | ⚠️ | bcrypt/argon2id + per-user salt | `system/auth_session.rs:51-55` 为 SHA-256 hex（无盐）；`:76-77` 仍留明文兜底 `TODO(security)` | **P0**：迁移 argon2，存量灰度重哈希 |
| MFA/双因素 | ❌ | TOTP/短信/邮箱二次验证 | 全仓无 TOTP/MFA 代码；登录仅 `login_handler`（`auth_session.rs:198`） | **P1**：TOTP 真实实现（可纯代码闭环） |
| 登出 / 令牌吊销 | ❌ | 服务端会话表/令牌黑名单，可强制下线 | `auth_session.rs` 仅 login/register/refresh（`:360-362`），无 logout；JWT 无状态即无法吊销 | **P0**：引入吊销表/黑名单（纯代码） |
| 本地会话可视/管理 | ❌ | 管理员查看在线会话、踢人 | SSO 有 `sessions: RwLock<HashMap>`（`sso/api.rs:22`）但为内存且无管理端点；本地 JWT 无会话表 | P1：会话落库 + 管理端点 |
| API Key 管理 | ⚠️ | 持久化、可禁用、审计 | `auth.rs:39` `api_keys: Arc<RwLock<HashMap>>` 内存态，重启即丢 | P1：迁入 IAM 库 |
| 限流 | ✅ | 令牌桶/IP+Key | `rate_limit.rs:85-116` 令牌桶，返回 `x-ratelimit-remaining`（`:155-158`） | 登录接口建议单独更严阈值（P1） |
| 审计日志 | ⚠️ | 记录+可检索+可导出 | 有 `/api/enterprise/admin/audit-logs`、`/audit-stats`（`admin_api.rs:340-341`）、`operation_log/`、`system/operlog.rs`、登录日志 `auth_session.rs:244`；**无导出端点**（路由表无 export） | **P1**：补 CSV/JSON 导出（纯代码） |
| SSO | ⚠️ | 协议真实打通、配置持久化 | 数据模型完整（`sso/mod.rs`），路由 protocols/providers CRUD/login/callback/logout（`sso/api.rs:379-384`）；**providers/sessions/pending 全内存**（`sso/api.rs:21-24`） | 配置落库；真实 IdP 联调需外部凭据 |
| dev 直通风险 | ⚠️ | 生产零后门 | `auth.rs:158` dev_mode 下 `dev-secret-token` 直通 admin | 部署门禁：prod 构建禁用 dev_mode（P0） |

### 1.2 治理与权限

| 能力项 | 状态 | 企业级要求 | 差距 / 证据 | 建议 |
| --- | --- | --- | --- | --- |
| RBAC | ✅ | 角色-权限-用户真实链路 | `rbac.rs:46-108` roles/permissions/current 走 IAM SQLite；角色 CRUD `system/role.rs`；权限中心 `api_permission/api.rs:503-518`（endpoints/permissions/roles/quick-grant/check/stats） | 保持 |
| 多租户 | ✅ | 租户隔离 | `system/tenant.rs`、`enterprise/tenant.rs`；登录按 tenant 解析（`auth_session.rs:211-215`） | 保持 |
| 数据权限/行级 | ⚠️ | data_scope 落地（本部门/全部/自定义） | 角色有 `data_scope` 字段（`rbac.rs:64`），但未见按 data_scope 过滤查询的统一拦截 | P1：数据范围中间件 |
| API 权限自动注册 | ✅ | 端点即权限点 | `enterprise_features.rs:75-80` init_standard_permissions + auto_register_endpoints | 保持 |

### 1.3 可靠性

| 能力项 | 状态 | 企业级要求 | 差距 / 证据 | 建议 |
| --- | --- | --- | --- | --- |
| 对象存储 | ⚠️ | 本地+S3 真实可用 | 本地磁盘 `cloud.rs:380-386` 真实；S3 adapter 手写 SigV4 已建但**本环境无 MinIO，未做成功往返 E2E**（`oss-storage-backends.md`） | 起 MinIO 做往返验证（外部条件） |
| LLM 配置持久化 | ✅ | 重启不丢 | 线三已交付 `.runtime/llm-config.json` 原子写盘，重启实测通过（`llm-config-persistence.md`） | 保持 |
| 备份 / 恢复 | ❌ | 数据库/配置可备份可恢复 | 路由表无 `/backup*`、无导出整库端点 | **P1**：SQLite 备份+恢复端点（纯代码） |
| 定时任务引擎 | ⚠️ | cron 真实触发、持久化、失败重试 | `scheduler/api.rs:440-450` CRUD + enable/disable/trigger/executions；任务存内存 HashMap（`api.rs:20,24`）；**无 tokio spawn 后台循环**（scheduler/ 内无 spawn/interval/tick），即不会到点自动执行 | **P1**：补后台执行循环+任务落库（纯代码主体） |
| Webhook 出站 | ❌ | 事件订阅回调 | 路由表无 `/webhook*` | P2：事件总线+webhook 投递 |
| 内存态易失面 | ⚠️ | 关键状态落库 | SSO providers/sessions、scheduler tasks、mailer config/templates、限流桶均为内存 HashMap | P1：关键配置迁 IAM/JSON 持久化 |

### 1.4 可观测性

| 能力项 | 状态 | 企业级要求 | 差距 / 证据 | 建议 |
| --- | --- | --- | --- | --- |
| 指标/监控 | ✅ | metrics/业务/节点/链路 | `monitor.rs:676-687` metrics-detail/quality/business/alerts-summary/nodes/logs/trace/timeseries；`actuator.rs:912-923` health/info/mappings/metrics/env/loggers/logs-tail | 保持 |
| 告警规则 | ⚠️ | 规则可配+真送达 | 规则 CRUD+toggle（`monitor.rs:683-685`）；**告警→送达通道未接线**（mailer 存在但无 monitor→mailer 调用链证据） | **P1**：告警触发即走 mailer/IM（纯代码可闭环） |
| 链路追踪 | ⚠️ | traceID 贯穿 | `monitor.rs:682` nodes/:name/trace 存在；跨服务 trace 贯通未验证 | P2 |
| 日志 | ✅ | 可查/可tail | `actuator.rs:919-920` logs + tail；`system/logininfor.rs` 登录日志 | 保持 |

### 1.5 运营效率

| 能力项 | 状态 | 企业级要求 | 差距 / 证据 | 建议 |
| --- | --- | --- | --- | --- |
| 字典/参数/文件 | ✅ | 基础运营配套 | `dictionary/api.rs:218-223`、`system/config.rs`、`file_storage/api.rs:347-350` | 保持 |
| 批量操作 | ✅ | 导入预览/导出模板 | `batch_operation/api.rs:257-263` | 保持 |
| 低代码设计器 | ✅ | 流程/表单/报表 | `designer/api.rs:416-430` process/form/report definitions + publish | 保持 |
| 邮件通道 | ⚠️ | 真实发信 | `mailer/api.rs:255-261` send/test/config/templates/history；有真实 `SmtpClient`（`api.rs:20,35`）；**需外部 SMTP 服务器** | 配真实 SMTP 即闭环（外部条件） |
| RAG / 知识检索 | ⚠️ | 文档切分+向量检索+召回 | 有 `kb_ext.rs`、`alliance/experts_session.rs:647` semantic-search / `:653` semantic-search；未见独立 RAG 编排端点 | P2：RAG pipeline 收口 |
| 消息中心 | ✅ | 站内通知 | `notification.rs:196-199` 列表/未读/已读；`message_center/` | 保持 |

### 1.6 用户体验

| 能力项 | 状态 | 企业级要求 | 差距 / 证据 | 建议 |
| --- | --- | --- | --- | --- |
| 管理面板覆盖 | ✅ | 各域有面板 | 18 面板（Overview/Api/Audit/Config/Dict/Logs/Menu/Storage/Tenant/Docs/Access/Hitl/Department/Role/User/Monitor/Llm/Sso） | 保持 |
| 站内通知 | ✅ | 未读角标 | `notification.rs:197` unread-count | 保持 |
| 端侧会话管理 | ❌ | 用户看自己会话、主动登出 | 无 `/api/auth/logout`；用户无法撤销已有 token | P1（与 1.1 登出同源） |
| MFA 设置入口 | ❌ | 用户自助开启二次验证 | 无对应面板/端点（与 1.1 MFA 同源） | P2 随 MFA |

---

## 2. 优先级清单

### P0 — 安全合规（建议本轮做，均可纯代码闭环）
1. **口令哈希升级 argon2id**（`auth_session.rs:51-78`），存量灰度重哈希、移除明文兜底。
2. **登出 + 令牌吊销/黑名单**（当前 `auth_session.rs` 无 logout）：吊销表 + 验签时校验黑名单。
3. **生产 dev 直通门禁**：构建/启动断言 `dev_mode=false` 或对 `dev-secret-token` 加白名单开关（`auth.rs:158`）。

### P1 — 运营/治理效率
4. **审计日志导出**（CSV/JSON），`admin_api.rs:340` 旁补 export。
5. **备份/恢复端点**：SQLite 整库快照 + 还原（路由表当前无）。
6. **定时任务后台执行循环**：补 tokio 调度循环 + 任务落库（`scheduler/` 现仅 CRUD+内存）。
7. **告警→送达接线**：monitor 告警触发走 mailer（`monitor.rs` ↔ `mailer/api.rs`）。
8. **关键内存态落库**：SSO providers、scheduler tasks、mailer config、api_keys 迁入持久层。
9. 数据权限 data_scope 统一过滤中间件。

### P2 — 体验 / 增强
10. Webhook 出站订阅。
11. RAG 编排端点收口。
12. MFA（TOTP）+ 用户自助设置入口。
13. 跨服务链路追踪贯通。

---

## 3. 纯代码可闭环 vs 需外部条件

### ✅ 纯代码本轮可闭环（无外部依赖）
- 口令哈希升级、登出/令牌黑名单、dev_mode 门禁（P0）。
- 审计导出、备份恢复端点、定时任务后台循环、告警→mailer 接线、内存态落库、data_scope 中间件（P1）。
- Webhook、TOTP MFA、RAG 收口（P2，纯逻辑；联调另算）。

### 🔌 需外部条件（代码已就绪或半就绪，缺真实环境/凭据）
| 能力 | 代码现状 | 缺什么外部条件 |
| --- | --- | --- |
| SSO 真实打通（飞书/钉钉/企微/OIDC/SAML） | 模型+回调链路已建（`sso/`） | 真实 IdP 应用凭据（client_id/secret、回调域名） |
| S3/对象存储生产可用 | adapter 手写 SigV4 已建（`oss-storage-backends.md`） | MinIO/S3 实例 + AK/SK |
| 邮件/告警送达 | `SmtpClient` 真实存在（`mailer/api.rs`） | 可用 SMTP 服务器（账号/端口/授权码） |
| LLM 真实对话 | 配置持久化已闭环（线三） | 真实 LLM API Key（DeepSeek/OpenAI 兼容） |
| Web-search 增强 | **无任何客户端代码** | 需先引入 DuckDuckGo/SearXNG/Tavily 任一 SDK/HTTP 客户端 + Key |

---

## 4. 结论

- **认证骨架已真实**（真 JWT 验签、限流、RBAC 落库、审计/监控/日志齐备），企业级差距集中在"会话生命周期管理"与"关键状态易失"两处。
- **P0 三项均为纯代码可闭环的安全收口**，建议优先做。
- 存储/LLM 两条线已从"零路由"补到"真实能力"；S3 与邮件/SSO 的最后一公里是外部环境（MinIO/SMTP/IdP 凭据），不是代码。
- 真正空白且需新建抽象的是：备份恢复、Webhook、RAG 编排、定时任务执行引擎。

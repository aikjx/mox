# 数据库与多套前后端归一化审计报告

> 审计范围：`D:\a10\aikjx\gitcode\infotopograph`
> 审计方式：只读（SELECT / PRAGMA / Grep / 目录扫描），未执行任何写操作
> 审计日期：2026-09-01

---

## 1. 所有 .db 文件清单

仓库内共发现 **6 个业务相关 SQLite 库**（另有 4 个 WebView2 浏览器缓存文件，已排除）。

| # | 路径 | 大小(KB) | 表数 | 有真实数据 | 用途定位 |
|---|------|----------|------|-----------|----------|
| 1 | `data/mox.db` | 644 | 22 | **是**（iam_user=2, iam_role=16, iam_permission=140, iam_resource=20, iam_role_permission=280, iam_tenant=2, sys_api_key=1, sys_config=1, sys_dict_type=1, iam_department=1） | **主库**：Rust 网关 IAM/系统权限 |
| 2 | `platform/legacy/mox-server/mox_meta.db` | 184 | 18 | **是**（users=3, roles=3, messages=7, ai_requests=25, audit_logs=282, apps=2, datasources=2, dsql_sqls=26, kg_vertices=32, kg_edges=44, publish_logs=15, store_apps=1, store_ratings=1, field_permissions=2, dsql_apps=1） | Python legacy 元数据库 |
| 3 | `platform/legacy/mox-server/mox_business.db` | 32 | 7 | **是**（banners=1, cases=3, news=6, products=6, team=3, messages=0） | Python legacy 业务库（官网内容） |
| 4 | `.runtime/operator_dialogue.db` | 28 | 2 | **是**（dialogue_sessions=9, dialogue_messages=9） | AI Agent 对话持久化 |
| 5 | `.runtime/operator_data.db` | 0 | 0 | 否（空文件） | AI Agent 数据工具（未初始化） |
| 6 | `platform/domains/ai/svc/mox-ai-agent-svc/operator_data.db` | 0 | 0 | 否（空文件） | AI Agent Svc 源码目录内残留空库 |

> **排除项**：`release-pkg/xiaobai-desktop/.../EBWebView/` 和 `target/debug/mox-voice-desktop-app.exe.WebView2/` 下的 `declarative_performance_cache.db`、`DawnCache.db` 为 WebView2 浏览器运行时缓存，非业务库。

---

## 2. 主库 `data/mox.db` 完整表结构

主库共 **22 张表**，全部以 `iam_` 或 `sys_` 前缀命名，采用 **TEXT 类型 UUID 主键**，时间字段为 `TEXT`（ISO 格式字符串）。

### 2.1 IAM 权限域（15 张表）

| 表名 | 字段数 | 关键字段 | 行数 |
|------|--------|----------|------|
| `iam_tenant` | 11 | tenant_id(PK TEXT), tenant_code, tenant_name, tenant_mode, tenant_status, tenant_plan, config_json, settings, created_at, updated_at, version | 2 |
| `iam_tenant_setting` | 8 | setting_id(PK), tenant_id, setting_key, setting_value, setting_value_type, description, updated_by, updated_at | 0 |
| `iam_user` | 19 | user_id(PK TEXT), tenant_id, user_code, username, password_hash, real_name, nickname, email, phone, avatar, dept_id, position, user_status, is_superuser, last_login_at, last_login_ip, created_at, updated_at, version | 2 |
| `iam_role` | 15 | role_id(PK TEXT), tenant_id, role_code, role_name, role_type, parent_id, inherit_path, is_builtin, data_scope, description, sort_order, status, created_at, updated_at, version | 16 |
| `iam_role_inherit` | 6 | ri_id(PK), tenant_id, parent_role_id, child_role_id, inherit_level, created_at | 0 |
| `iam_user_role` | 7 | ur_id(PK), tenant_id, user_id, role_id, assigned_by, assigned_at, created_at | 2 |
| `iam_permission` | 14 | perm_id(PK TEXT), tenant_id, perm_code, perm_name, resource_id, resource_type, perm_action, perm_category, description, sort_order, status, created_at, updated_at, version | 140 |
| `iam_resource` | 15 | resource_id(PK TEXT), tenant_id, resource_code, resource_name, resource_type, parent_id, resource_category, api_methods_sql, api_paths_sql, description, sort_order, status, created_at, updated_at, version | 20 |
| `iam_role_permission` | 6 | rp_id(PK), tenant_id, role_id, perm_id, created_at, created_by | 280 |
| `iam_data_permission` | 15 | dp_id(PK), tenant_id, dp_code, dp_name, subject_type, subject_id, subject_uuids_json, resource_code, scope_type, custom_rule_expression_sql, custom_rule_expression_json, status, created_at, created_by, updated_at | 0 |
| `iam_department` | 14 | dept_id(PK TEXT), tenant_id, parent_id, dept_code, dept_name, dept_type, dept_level, dept_path, sort_order, manager_user_id, status, created_at, updated_at, version | 1 |
| `iam_menu` | 25 | menu_id(PK TEXT), tenant_id, parent_id, menu_code, menu_name, menu_type, menu_category, route_path, route_name, component_path, icon, color, sort_order, is_visible, is_cached, is_external, link_target, permission_code, api_scope, menu_config, children_json, status, created_at, updated_at, version | 0 |
| `iam_role_menu` | 6 | rm_id(PK), tenant_id, role_id, menu_id, created_by, created_at | 0 |
| `iam_user_menu` | 8 | um_id(PK), tenant_id, user_id, menu_id, is_favorite, sort_order, created_at, updated_at | 0 |
| `sys_post` | 10 | post_id(PK), tenant_id, post_code, post_name, dept_id, sort_order, status, remark, created_at, updated_at | 0 |

### 2.2 系统配置域（7 张表）

| 表名 | 字段数 | 关键字段 | 行数 |
|------|--------|----------|------|
| `sys_api_key` | 11 | key_id(PK TEXT), tenant_id, name, api_key, user_id, scopes, status, expires_at, last_used_at, created_at, revoked_at | 1 |
| `sys_config` | 10 | config_id(PK TEXT), tenant_id, config_name, config_key, config_value, config_type, status, remark, created_at, updated_at | 1 |
| `sys_dict_type` | 8 | dict_id(PK), tenant_id, dict_name, dict_type, status, remark, created_at, updated_at | 1 |
| `sys_dict_data` | 13 | dict_code(PK), tenant_id, dict_sort, dict_label, dict_value, dict_type, css_class, list_class, is_default, status, remark, created_at, updated_at | 0 |
| `sys_logininfor` | 10 | info_id(PK), tenant_id, user_name, ipaddr, login_location, browser, os, status, msg, login_time | 0 |
| `sys_oper_log` | 18 | oper_id(PK), tenant_id, title, business_type, method, request_method, operator_type, oper_name, dept_name, oper_url, oper_ip, oper_location, oper_param, json_result, status, error_msg, oper_time, cost_time | 0 |
| `audit_log` | 23 | log_id(PK TEXT), tenant_id, trace_id, request_id, user_id, user_ip, action, action_detail, resource_type, resource_id, resource_code, biz_id, biz_code, status_code, http_method, http_path, latency_ms, snapshot_before, snapshot_after, changed_fields, prev_hash, curr_hash, created_at | 0 |

---

## 3. 后端数据库连接映射表

### 3.1 Rust 新后端（`platform/`）

| 组件 | 连接的库 | 驱动/ORM | 建表脚本位置 | 连接方式 |
|------|---------|----------|-------------|----------|
| **网关** `mox-platform-gateway-svc` | `data/mox.db`（相对 cwd） | `rusqlite` 0.31 (bundled) | IAM 表由 `mox-platform-iam-core` repo.rs 手写 SQL 创建 | `rusqlite::Connection::open(&db_path)`，启动期同步初始化 |
| **系统核心** `mox-platform-system-core` | `{data_dir}/mox.db`（默认） | `rusqlite`（SQLite）+ `sqlx`（Postgres/MySQL）+ `sea-query`（DDL 生成） | `repo/schema.rs` — 9 张通用表（moxs/members/tasks/channels/messages/notifications/bindings/tokens/audit），用 sea-query 按方言生成 | `Store::open(Backend::Sqlite, &db_path)`，启动重放（Replay）模式 |
| **IAM 核心** `mox-platform-iam-core` | 共享 `data/mox.db` | `rusqlite` 直接手写 SQL | `src/repo.rs` — 22 张 iam_/sys_ 表的 CREATE TABLE | 通过网关传入的 `rusqlite::Connection` |
| **AI Agent Svc** `mox-ai-agent-svc` | `.runtime/operator_dialogue.db` + `operator_data.db` | `SqlitePersistence`（mox-system 封装的 rusqlite trait） | `dialogue_graph.rs` — dialogue_sessions/dialogue_messages 两张表，幂等 CREATE | `SqlitePersistence::file(db_path)`，失败回退内存 |
| **KG 存储 Svc** `mox-kg-storage-svc` | 运行时动态路径（可选） | `rusqlite` | `src/lib.rs` — graph_nodes/graph_edges 表 | `rusqlite::Connection::open(path)`，可选持久化 |
| **Primiflow Svc** `mox-flow-primiflow-svc` | `primiflow.db`（out_dir 下） | `SqlitePersistence` | `persistence.rs` — 知识库+六维溯源主图序列化存储 | `Persistence::sqlite(&path)` |
| **Datastore Core** `mox-platform-datastore-core` | 默认 `sqlite:mox.db?mode=rwc` | `rusqlite`（多后端抽象：SQLite/Postgres/MySQL） | `dao.rs` + `tx.rs` — 通用 DAO 层 | `DatastoreConnection::new(config)` |
| **DSQL Core** `mox-dsql-core` | 内存或文件 SQLite | `rusqlite` | `storage.rs` — dsql 定义表 | `rusqlite::Connection::open(exec_path)` |

**Rust 后端关键发现**：
- 全仓库 `rusqlite` 为统一 SQLite 驱动（workspace 统一版本，bundled feature）
- `mox-platform-system-core` 设计为**唯一**允许直接 `use rusqlite` 的 crate（AC-11 架构规则），其他 crate 通过 `PersistenceProvider` trait 间接使用
- 但实际代码中 `mox-kg-core`、`mox-kg-storage-svc`、`mox-platform-iam-core`、`mox-platform-enterprise-svc`、`mox-dsql-core`、`mox-platform-datastore-core`、`mox-platform-orchestrator-core` 均直接 `use rusqlite`，**架构规则未严格执行**
- 无 migrations/ 目录，建表全部为代码内 `CREATE TABLE IF NOT EXISTS`（幂等启动初始化）
- 支持多后端（SQLite/Postgres/MySQL），但 Postgres/MySQL 仅在 `mox-platform-system-core` 的 `repo/postgres.rs`、`repo/mysql.rs` 中有骨架实现，实际生产默认 SQLite

### 3.2 Python Legacy 后端（`platform/legacy/mox-server/`）

| 组件 | 连接的库 | 驱动 | 建表脚本位置 | 连接方式 |
|------|---------|------|-------------|----------|
| **元数据库** | `platform/legacy/mox-server/mox_meta.db` | `sqlite3`（标准库） | `mox/seed_data.py` — META_SCHEMA 列表（12 张表原始 SQL） | `sqlite3.connect(META_DB)`，row_factory=Row |
| **业务数据库** | `platform/legacy/mox-server/mox_business.db` | `sqlite3`（标准库） | `mox/seed_data.py` — BUSINESS_SCHEMA 列表（6 张表原始 SQL） | `sqlite3.connect(BUSINESS_DB)` |
| **动态数据源** | 元库 `datasources` 表配置的任意库 | `db_adapters.py` — DBAdapter 抽象（SQLite/MySQL/Postgres/DuckDB） | 无（连接外部已有库） | `build_adapter(driver, config)` 动态构建 |

**Python legacy 关键发现**：
- `META_DB = os.path.join(BASE_DIR, "mox_meta.db")`，`BUSINESS_DB = os.path.join(BASE_DIR, "mox_business.db")`，路径硬编码为脚本所在目录
- 启动时 `reset_and_seed()` 自动建表 + 幂等填充种子数据
- 业务库通过元库 `datasources` 表的 `default` 数据源（dsn 指向 mox_business.db 绝对路径）间接访问
- 无 ORM（SQLAlchemy），全部手写 SQL + sqlite3.Row
- 支持多数据库适配器（SQLite/MySQL/Postgres/DuckDB），但默认仅 SQLite

### 3.3 Legacy backend-rust（`platform/legacy/backend-rust/`）

| 组件 | 连接的库 | 驱动 | 说明 |
|------|---------|------|------|
| **API 网关** | **无数据库连接** | — | 纯 axum 网关，提供限流/熔断/重试/路由/零信任认证，代理转发到后端服务，不持久化任何数据 |

---

## 4. 数据模型不一致清单

### 4.1 同业务实体跨库分裂

| 业务实体 | `data/mox.db`（Rust 主库） | `mox_meta.db`（Python 元库） | `mox_business.db`（Python 业务库） | 不一致程度 |
|---------|---------------------------|-------------------------------|-----------------------------------|-----------|
| **用户** | `iam_user`（19 字段，TEXT UUID PK，tenant_id 隔离，password_hash，dept_id，is_superuser，version） | `users`（4 字段，INTEGER AUTOINCREMENT PK，username/role/display_name） | — | **严重**：主键类型、字段数、认证方式完全不同 |
| **角色** | `iam_role`（15 字段，TEXT UUID PK，role_code/role_type/parent_id/inherit_path/is_builtin/data_scope，tenant_id） | `roles`（3 字段，INTEGER PK，name/description） | — | **严重**：Rust 是 RBAC+继承+数据权限，Python 是扁平角色 |
| **权限** | `iam_permission` + `iam_resource` + `iam_role_permission`（140+20+280 行，完整 RBAC） | `field_permissions`（2 行，仅字段级白名单） | — | **严重**：模型范式不同（资源-动作 vs 字段过滤） |
| **消息/留言** | — | `messages`（8 字段，官网联系表单，7 行） | `messages`（8 字段，**同构重复**，0 行） | **中等**：同一张表在两个库重复定义，元库有数据业务库为空 |
| **审计日志** | `audit_log`（23 字段，TEXT UUID PK，prev_hash/curr_hash 哈希链，snapshot_before/after，tenant_id） | `audit_logs`（6 字段，INTEGER PK，ts/trace_id/actor/action/detail） | — | **严重**：Rust 是哈希链不可篡改审计，Python 是简单日志 |
| **知识图谱** | —（Rust KG 有独立存储，表结构不同） | `kg_vertices`（vid TEXT PK, type/label/props/domain）+ `kg_edges`（source/relation/target/weight） | — | **中等**：Python 用 vid 字符串主键，Rust KG 用 INTEGER id + node_type |
| **应用/发布** | — | `apps` + `publish_logs`（无限发布系统） | — | Rust 侧无对应表 |
| **DSQL 定义** | —（Rust dsql-core 有独立存储） | `dsql_sqls`（26 行，code/template/datasource/cache_ttl/status/version） | — | **中等**：Python 用 code 作为业务主键，Rust dsql-core 用 INTEGER id |
| **数据源** | — | `datasources`（2 行，name/driver/config_json/enabled） | — | Rust 侧无统一数据源注册表 |
| **AI 请求** | — | `ai_requests`（25 行，ts/app_key/user_message/reply/engine/trace_id/duration_ms） | — | Rust AI Agent 用 dialogue_messages 表，字段不同 |
| **应用商店** | — | `store_apps` + `store_installs` + `store_ratings` | — | Rust 侧无对应表 |

### 4.2 字段命名口径差异

| 维度 | `data/mox.db`（Rust） | `mox_meta.db` / `mox_business.db`（Python） |
|------|----------------------|----------------------------------------------|
| **主键类型** | TEXT（UUID 字符串） | INTEGER（AUTOINCREMENT 自增） |
| **时间字段** | `created_at` TEXT（ISO 8601 字符串） | `created_at` INTEGER（Unix 时间戳秒）；`ai_requests` 用 `ts` |
| **表命名前缀** | `iam_` / `sys_` 前缀 | 无前缀（users, roles, messages...） |
| **租户隔离** | 全部表含 `tenant_id` | 无租户概念（单租户） |
| **乐观锁** | 全部表含 `version` 字段 | 无 version 字段 |
| **软删除/状态** | `status` TEXT（active/disabled） | `enabled` INTEGER（0/1）或 `status` TEXT |
| **审计字段** | `created_by` / `updated_by` | 无 created_by/updated_by |
| **用户标识** | `user_id`（UUID） | `id`（INTEGER）+ `username` |
| **角色关联** | `iam_user_role` 中间表（ur_id/user_id/role_id） | `users.role` 直接存角色名字符串（反范式） |

### 4.3 同一实体分散多库的具体案例

1. **messages 表**：`mox_meta.db` 和 `mox_business.db` 各有一张 `messages` 表，字段完全一致（id/name/phone/email/company/content/status/created_at）。元库有 7 行数据，业务库 0 行。Python server.py 的 `/api/website/message` 写入元库，但 `/api/stats` 从业务库查询 COUNT——**写入和查询不在同一个库**。

2. **用户认证**：Rust 网关用 `iam_user`（含 password_hash）做认证，Python legacy 用 `users`（无密码字段，仅 role 字符串）做简单角色判断。两套用户体系完全独立，无同步机制。

3. **知识图谱**：Python `mox_meta.db` 有 `kg_vertices`/`kg_edges`（32+44 行），Rust `mox-kg-core` 和 `mox-kg-storage-svc` 有独立的图存储实现（不同表结构），两者数据不互通。

---

## 5. 多前端归一化方案表

### 5.1 前端产物清单

| # | 产物路径 | 类型 | 技术栈 | 入口文件 | 自有 API 调用 | 与主 SPA 功能重叠度 |
|---|---------|------|--------|---------|-------------|-------------------|
| 1 | `frontend-ui/` | **主 SPA 工程** | Vue 3 + Vite + Element Plus + axios + vue-router + pinia + ECharts + 3d-force-graph | `index.html` → `src/main.js` | **是**（`src/api/` 目录统一封装） | 基准（100%） |
| 2 | `frontend-ui/mox-website/` | 独立静态 HTML 站点 | 原生 HTML/JS（无构建） | `index.html` | **是**（内联 fetch 调用 `/api/dsql/execute` 等） | **高**：官网产品/新闻/案例展示，主 SPA market 视图可覆盖 |
| 3 | `frontend-ui/mox-console/` | 独立静态 HTML 站点 | 原生 HTML/JS（无构建） | `index.html` | **是**（内联 fetch 调用管理接口） | **高**：管理控制台功能，主 SPA admin 视图可覆盖 |
| 4 | `frontend-ui/mox-store/` | 独立静态 HTML 站点 | 原生 HTML/JS（无构建） | `index.html` | **是**（内联 fetch 调用应用商店接口） | **中**：应用商店浏览/安装，主 SPA market 视图部分重叠 |
| 5 | `frontend-ui/chip-website/` | 独立静态 HTML 站点 | 原生 HTML/JS + Python 检查脚本 | `index.html` | **是**（内联 fetch） | **低**：芯片产品专题页，与主 SPA 无直接重叠 |
| 6 | `xuanji-ux-redesign/` | 独立静态 HTML 原型 | 单文件 HTML（设计原型） | `xuanji-ux-redesign.html` | 否（纯展示原型） | **设计参考**：UX 重设计稿，非运行时产物 |
| 7 | `platform/legacy/mox-store/` | legacy 商店（需确认） | — | — | — | 与 mox-store 可能重复 |

### 5.2 主 SPA 视图结构（`frontend-ui/src/views/`）

```
views/
├── admin/      # 管理后台（用户/角色/权限/系统配置）
├── ai/         # AI 助手/对话
├── expert/     # 专家联盟
├── graph/      # 知识图谱可视化
├── market/     # 应用市场/模板
├── misc/       # 杂项页面
├── operators/  # 算子管理
├── project/    # 项目管理
├── workflow/   # 工作流编排
└── workspace/  # 工作空间
```

### 5.3 归一化建议

| 产物 | 建议 | 理由 | 具体方案 |
|------|------|------|---------|
| **`frontend-ui/`（主 SPA）** | **保留并扩展**，作为唯一前端工程 | 已有完整 Vue 3 工程化体系（路由/状态/API 封装/组件库/测试），10 个业务视图覆盖绝大部分功能 | 作为统一入口，吸收其他静态站点的功能 |
| **`mox-website/`** | **合并进主 SPA** | 纯静态 HTML，功能为官网展示（产品/新闻/案例/Banner），与主 SPA `market/` 视图高度重叠；内联 fetch 调用 DSQL 接口，无独立状态管理 | 将页面拆为 Vue 组件放入 `views/market/website/`，API 调用迁移到 `src/api/` 统一封装；路由 `/website/*` |
| **`mox-console/`** | **合并进主 SPA** | 管理控制台功能与主 SPA `admin/` 视图重叠；静态 HTML 无法复用组件和权限指令 | 迁移到 `views/admin/console/`，复用 Element Plus 组件和 iam 权限指令 |
| **`mox-store/`** | **合并进主 SPA**（或保留为独立路由模块） | 应用商店功能主 SPA `market/` 已有基础；但商店有独立的浏览/安装/评分交互，可作为 market 下的子模块 | 迁移到 `views/market/store/`，与模板市场共用布局；API 统一到 `src/api/store.js` |
| **`chip-website/`** | **保留为独立静态产物**，但纳入统一部署 | 芯片产品专题页，面向外部访客，与主 SPA（内部运营平台）受众不同；含 Python 检查脚本，可能为独立营销页 | 保留独立 HTML，但 API base URL 统一通过环境变量注入；部署时与主 SPA 同域不同路径（`/chip/`） |
| **`xuanji-ux-redesign/`** | **保留为设计参考文档**，不纳入运行时 | 单文件 HTML 设计原型，无 API 调用，非可运行产品 | 移至 `docs/ux-redesign/` 或 `design/` 目录，作为设计规范参考 |
| **`platform/legacy/mox-store/`** | **评估后删除或归档** | 与 `frontend-ui/mox-store/` 可能为同一功能的 legacy 版本，位于 legacy 后端目录内 | 确认是否为前端代码，若是则归档到 `legacy/`，统一以后端 API + 主 SPA 前端为准 |

### 5.4 统一规范建议

| 维度 | 统一方案 |
|------|---------|
| **入口/路由** | 主 SPA 为唯一入口（`/`），静态站点迁移为路由模块；外部营销页（chip-website）保留独立路径但同域部署 |
| **API 命名** | 统一 `/api/{domain}/{resource}` 风格；DSQL 接口保留 `/api/dsql/execute` 但封装到 `src/api/dsql.js`；禁止页面内联 fetch |
| **API 客户端** | 统一使用 axios 实例（`src/api/http.js`），统一 baseURL、拦截器、错误处理、token 注入 |
| **数据模型** | 前端 TypeScript 接口（或 JSDoc）与后端 Rust 结构体对齐；用户统一用 `user_id`(UUID)，时间统一用 ISO 字符串 |
| **配置管理** | 统一 `.env` / `vite.config.js` 环境变量（`VITE_API_BASE_URL`、`VITE_APP_TITLE`），禁止硬编码 API 地址 |
| **组件库** | 统一 Element Plus + 自定义业务组件库（`src/components/`），静态站点的内联 CSS/JS 组件重构为 Vue SFC |
| **状态管理** | 统一 Pinia（`src/stores/`），按领域分 module（user/iam/project/workflow/ai/graph） |

---

## 6. 多后端归一化方案

### 6.1 三套后端职责对比

| 维度 | Rust 新后端（`platform/`） | Python Legacy（`platform/legacy/mox-server/`） | Legacy backend-rust（`platform/legacy/backend-rust/`） |
|------|---------------------------|------------------------------------------------|-------------------------------------------------------|
| **架构** | 微服务（网关 + 多 domain svc） | FastAPI 单体 | axum 纯网关 |
| **语言** | Rust | Python 3 | Rust |
| **数据库** | `data/mox.db`（IAM）+ 各 svc 独立库 | `mox_meta.db` + `mox_business.db` | 无 |
| **核心能力** | IAM/权限、系统配置、AI Agent、知识图谱、DSQL、工作流编排、数据存储、云存储、市场模板 | DSQL 执行、KG 查询、应用管理、官网接口、AI 助手、数据源管理、字段权限 | 限流、熔断、重试、路由、零信任(mTLS/Spiffe)、AIOps 预测 |
| **端点风格** | `/api/system/*`、`/api/security/*`、各 domain `/api/{domain}/*` | `/api/dsql/*`、`/api/admin/*`、`/api/kg/*`、`/api/website/*`、`/api/apps/*`、`/api/ai/*` | `/health`、`/ready`、`/api/*`（代理转发） |
| **认证** | IAM（iam_user + password_hash + api_key + JWT） | 简单 role 字段（无密码认证） | 零信任 mTLS + Spiffe |
| **ORM/驱动** | rusqlite（多 crate 直接使用）+ sqlx（PG/MySQL 骨架）+ sea-query（DDL） | sqlite3 手写 SQL + DBAdapter 抽象 | 无 |
| **建表方式** | 代码内 CREATE TABLE IF NOT EXISTS（无 migrations） | seed_data.py 原始 SQL + 幂等种子 | 无 |
| **数据量** | IAM 有真实数据（2 用户/16 角色/140 权限） | 元库有真实数据（3 用户/26 SQL/32 顶点/282 审计） | 无 |

### 6.2 端点重叠分析

| 功能域 | Rust 新后端 | Python Legacy | 重叠程度 |
|--------|-----------|-------------|---------|
| **DSQL 动态 SQL** | `mox-dsql-core`（storage.rs + engine.rs） | `/api/dsql/execute`、`/api/admin/sqls` | **高**：两者都实现了 SQL 模板定义+执行+缓存 |
| **知识图谱** | `mox-kg-core` + `mox-kg-storage-svc` | `/api/kg/graph`、`/api/kg/query`、`/api/admin/kg/*` | **高**：两者都有顶点/边 CRUD + 图查询 |
| **AI 助手** | `mox-ai-agent-svc`（conversation + dialogue_graph） | `/api/ai/assistant`、`/api/ai/requests` | **中**：Rust 是多轮 Agent 对话，Python 是单轮规则助手 |
| **应用管理** | 无直接对应（orchestrator 有业务实体版本管理） | `/api/apps/*`、`/api/apps/{key}/transition`、`/api/apps/{key}/logs` | **低**：Python 的无限发布系统 Rust 侧未实现 |
| **官网接口** | 无 | `/api/website/message`、`/api/website/resume`、`/api/website/consultation` | **无重叠**：Python 独有 |
| **用户/角色/权限** | `mox-platform-iam-core`（完整 RBAC+数据权限+租户） | `/api/admin/users`、`/api/admin/roles`、`/api/admin/permissions`（极简） | **Rust 完全覆盖**：Python 可废弃 |
| **数据源管理** | 无统一注册表 | `/api/admin/datasources`、`/api/admin/datasources/{name}/reload` | **Python 独有**：Rust 需补充 |
| **系统配置** | `sys_config`、`sys_dict_*` | 无 | **Rust 独有** |
| **审计日志** | `audit_log`（哈希链）+ `sys_oper_log` + `sys_logininfor` | `/api/audit`（audit_logs 简单表） | **Rust 完全覆盖** |
| **API 网关** | `mox-platform-gateway-svc` | 无（FastAPI 自带） | `legacy/backend-rust` 纯网关 | **三者都有网关能力** |

### 6.3 数据库重叠分析

| 数据库 | Rust 新后端 | Python Legacy | 归一化方向 |
|--------|-----------|-------------|-----------|
| `data/mox.db` | **主库**（网关 + IAM + 系统） | 不访问 | 保留为唯一 IAM/系统主库 |
| `mox_meta.db` | 不访问 | **元库**（DSQL/KG/apps/users/audit/store） | 迁移 DSQL/KG 数据到 Rust 对应服务；users/roles/audit 废弃（Rust IAM 覆盖） |
| `mox_business.db` | 不访问 | **业务库**（官网内容） | 迁移到 Rust 内容管理服务或保留为只读内容库 |
| `operator_dialogue.db` | AI Agent Svc 使用 | 不访问 | 保留，归 Rust AI Agent 管理 |
| `operator_data.db`（空） | AI Agent Svc 工具使用 | 不访问 | 清理空文件，运行时按需创建 |

### 6.4 归一化建议

#### 总体策略：**以 Rust 新后端为主，Python Legacy 逐步退役，legacy backend-rust 合并进 Rust 网关**

| 后端 | 归一化决策 | 保留内容 | 退役内容 | 迁移路径 |
|------|----------|---------|---------|---------|
| **Rust 新后端** | **主后端，持续扩展** | 全部 | — | 补充缺失能力（数据源管理、应用发布、官网接口） |
| **Python Legacy** | **逐步退役，过渡期保留** | DSQL 执行引擎（作为 Rust dsql-core 的参考实现）、KG 数据（32 顶点/44 边需迁移）、官网内容数据 | 用户/角色/权限管理（Rust IAM 覆盖）、审计日志（Rust 哈希链覆盖）、AI 助手（Rust Agent 覆盖）、应用管理（Rust 需补充） | 阶段1：数据迁移（KG/DSQL定义/业务内容）→ 阶段2：API 兼容层（Rust 网关代理 `/api/dsql/*`、`/api/kg/*` 到 Python）→ 阶段3：功能替代后下线 Python |
| **legacy backend-rust** | **合并进 Rust 网关，然后归档** | 限流算法、熔断器实现、零信任 mTLS/Spiffe 参考、AIOps 预测模块 | 整个独立网关进程 | 将限流/熔断/零信任能力移植到 `mox-platform-gateway-svc`，代码归档到 `platform/legacy/` |

#### 是否需要适配层：**需要，过渡期 API 兼容适配层**

在 Rust 网关中增加 **Python Legacy 代理适配层**，将以下端点转发到 Python 服务，实现前端无感切换：

| 端点前缀 | 转发目标 | 适配内容 |
|---------|---------|---------|
| `/api/dsql/*` | Python `mox-server` | 请求/响应格式统一（Python 返回 `{success,code,message,data,trace_id}`，Rust 需适配） |
| `/api/kg/*` | Python `mox-server` | 图数据格式对齐（Python kg_vertices 用 vid 字符串，Rust 用 id+type） |
| `/api/website/*` | Python `mox-server` | 官网留言/简历/咨询接口，过渡期保留 |
| `/api/apps/*` | Python `mox-server` | 应用发布管理，Rust 侧补充后下线 |

适配层实现位置：`platform/gateway/mox-platform-gateway-svc/src/legacy_proxy.rs`（新增），使用 `reqwest` 异步转发。

---

## 7. P0 归一化行动项

按优先级排序，P0 为必须立即执行的阻断性问题。

### P0-1：统一数据库，消除多库数据分裂

| 行动项 | 具体内容 | 影响范围 | 风险 |
|--------|---------|---------|------|
| **P0-1.1** 确定唯一主库 | 以 `data/mox.db` 为唯一 IAM/系统主库，Python `mox_meta.db` 的 users/roles 数据迁移到 `iam_user`/`iam_role` | Rust 网关 + Python legacy | 中（需映射 INTEGER→UUID 主键） |
| **P0-1.2** 修复 messages 表双写问题 | Python `/api/website/message` 写入 `mox_meta.db`，但 `/api/stats` 从 `mox_business.db` 读——统一到一个库 | Python legacy | 低（改连接指向即可） |
| **P0-1.3** 清理空库文件 | 删除 `.runtime/operator_data.db`（0KB）和 `platform/domains/ai/svc/mox-ai-agent-svc/operator_data.db`（0KB），运行时按需创建 | AI Agent Svc | 极低 |
| **P0-1.4** KG 数据迁移 | 将 `mox_meta.db` 的 32 顶点/44 边迁移到 Rust KG 存储，统一图数据模型 | KG 域 | 中（vid→id 映射） |

### P0-2：统一后端入口，退役冗余网关

| 行动项 | 具体内容 | 影响范围 | 风险 |
|--------|---------|---------|------|
| **P0-2.1** legacy backend-rust 能力移植 | 将限流/熔断/重试/零信任 mTLS 能力移植到 `mox-platform-gateway-svc` | Rust 网关 | 中（需保持 API 兼容） |
| **P0-2.2** 新增 Python Legacy 代理适配层 | 在 Rust 网关中增加 `/api/dsql/*`、`/api/kg/*`、`/api/website/*`、`/api/apps/*` 的转发适配 | Rust 网关 + Python | 低（纯代理） |
| **P0-2.3** 前端 API base URL 统一 | 所有前端（含静态 HTML）统一指向 Rust 网关，不再直连 Python | 全部前端 | 低（改配置） |

### P0-3：前端归一化，消除静态站点分裂

| 行动项 | 具体内容 | 影响范围 | 风险 |
|--------|---------|---------|------|
| **P0-3.1** mox-website 合并进主 SPA | 将 `frontend-ui/mox-website/index.html` 重构为 Vue 组件，放入 `views/market/website/` | 主 SPA | 中（需保持视觉一致） |
| **P0-3.2** mox-console 合并进主 SPA | 将管理控制台功能迁移到 `views/admin/console/`，复用 IAM 权限指令 | 主 SPA | 中 |
| **P0-3.3** mox-store 合并进主 SPA | 将应用商店迁移到 `views/market/store/`，API 统一封装 | 主 SPA | 中 |
| **P0-3.4** 静态站点内联 fetch 消除 | 所有页面禁止内联 `fetch()`，统一使用 `src/api/` 封装的 axios 实例 | 全部前端 | 低 |
| **P0-3.5** xuanji-ux-redesign 归档 | 将设计原型移至 `docs/design/`，不纳入运行时构建 | 设计文档 | 极低 |

### P0-4：数据模型与命名规范统一

| 行动项 | 具体内容 | 影响范围 | 风险 |
|--------|---------|---------|------|
| **P0-4.1** 主键类型统一 | 全部新表使用 TEXT UUID 主键（与 `data/mox.db` 一致），废弃 INTEGER AUTOINCREMENT | 全部后端 | 中（迁移期需双写） |
| **P0-4.2** 时间字段统一 | 全部使用 `created_at`/`updated_at` TEXT（ISO 8601），废弃 Unix 时间戳 INTEGER | 全部后端 | 中（需数据转换） |
| **P0-4.3** 表命名前缀统一 | 业务表按 domain 加前缀（`iam_`、`sys_`、`kg_`、`dsql_`、`ai_`、`wf_`），废弃无前缀表名 | 全部后端 | 低（新表遵循，旧表迁移） |
| **P0-4.4** 租户隔离字段统一 | 全部多租户表含 `tenant_id`，Python legacy 数据迁移时分配默认租户 | Rust + Python 迁移 | 中 |
| **P0-4.5** 乐观锁 version 字段统一 | 全部可更新表含 `version` INTEGER，Python legacy 表迁移时补充 | 全部后端 | 低 |

### P0-5：架构规则执行与建表规范

| 行动项 | 具体内容 | 影响范围 | 风险 |
|--------|---------|---------|------|
| **P0-5.1** rusqlite 使用收敛 | 执行 AC-11 规则：仅 `mox-platform-system-core` 可直接 `use rusqlite`，其他 crate 必须通过 `PersistenceProvider` trait | `mox-kg-core`、`mox-kg-storage-svc`、`mox-platform-iam-core`、`mox-platform-enterprise-svc`、`mox-dsql-core`、`mox-platform-datastore-core`、`mox-platform-orchestrator-core` | 高（涉及 7+ crate 重构） |
| **P0-5.2** 引入 migrations 机制 | 建立 `platform/migrations/` 目录，使用 `sqlx-cli` 或自定义 migration 框架，替代代码内 `CREATE TABLE IF NOT EXISTS` | 全部 Rust 后端 | 中（需梳理所有建表 SQL） |
| **P0-5.3** 数据库连接配置统一 | 全部服务通过环境变量 `DATABASE_URL` 或统一配置文件获取数据库路径，禁止硬编码（如网关的 `data/mox.db`、AI Agent 的 `operator_dialogue.db`） | 全部 Rust 服务 | 低 |

---

## 附录：审计方法说明

- **.db 扫描**：`Get-ChildItem -Recurse -Filter *.db`，排除 WebView2 缓存目录
- **表结构审计**：Python `sqlite3` 模块，`SELECT name FROM sqlite_master` + `PRAGMA table_info()`
- **数据量统计**：`SELECT COUNT(*)` 对所有业务表
- **后端连接定位**：Grep `sqlite|\.db|DATABASE_URL|rusqlite|sqlx|sea-orm|diesel`（Rust），`sqlite3|SQLALCHEMY|engine =`（Python）
- **前端产物识别**：`package.json` 存在性 + 目录结构 + `fetch/axios/api` Grep
- **全程只读**：未执行任何 INSERT/UPDATE/DELETE/DROP/ALTER，未修改任何文件

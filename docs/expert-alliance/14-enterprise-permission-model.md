# 14 专家联盟企业级权限模型

> **文档编号**：14 ｜ **权威等级**：🟢 现状核证（先读码后落笔） ｜ **核证日期**：2026-09-30
>
> **本文性质**：把专家联盟现行权限/鉴权/审计链路逐层对账到代码 `文件:行号`，并与 39 号文档
> 的历史角色快照、12 号路线图的 A1/G3 演进项对齐。所有结论均有证据；不确定处标「待核」。
> 现状裁决基准仍为 `CURRENT-ARCHITECTURE.md` V1.1，本文不替代它。

---

## 一、权限模型总览

### 1.1 四层链路图

```
┌──────────────────────────────────────────────────────────────────────┐
│ ① 认证（网关层）                                                      │
│    JWT 中间件把 UserInfo 注入 request extensions                      │
│    experts_common.rs:605  parts.extensions.get::<UserInfo>()          │
│    OptionalAuthUser（:585）：取不到身份返回 Ok(None)，不拒绝请求        │
└───────────────────────────┬──────────────────────────────────────────┘
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ ② 路由级校验（前端全局守卫 router/index.js:61-136）                    │
│    requiresPermission（:94-113）→ hasAnyPermission / hasPermission    │
│    requiresRole（:115-133）→ hasAnyRole / hasRole                     │
│    不通过 → ElMessage.warning + next('/403', { redirect })            │
│    专家联盟 6 处 requiresRole（index.js:50/71/91/115/139/152）          │
└───────────────────────────┬──────────────────────────────────────────┘
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ ③ 按钮级权限（v-permission / v-role 指令）                             │
│    指令基础设施已存在并注册：directives/permission.js:86-128            │
│    main.js:14,47 setupPermissionDirectives(app)                       │
│    专家联盟管理写面按钮已挂 v-role-any：7 处（2026-09-30 G-1 闭环）      │
│    （Experts:注册/编辑/停用；Graph:重建；Console:保存配置/重置/全量重置）│
│    全站仅 views/admin/panels/{AdminSso,AdminRole,AdminUser,            │
│    AdminDepartment}.vue 使用 v-role="'admin'"（历史遗留，见 G-3）     │
└───────────────────────────┬──────────────────────────────────────────┘
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│ ④ 后端审计（actor 注入，不做角色强制）                                  │
│    actor_from_opt_user（experts_common.rs:613-621）：                  │
│      Some(u) → AuditActor::human(u.id, roles.join(","))               │
│      None    → AuditActor::system()（降级）                           │
│    emit_audit（:628-655）→ NDJSON + SHA-256 哈希链 + HMAC              │
│    11 个写面 handler 注入真实身份（dispatcher/registry/session）       │
│    注意：后端联盟 handler 不做 RBAC 拒绝，只审计；角色强制在前端守卫     │
└──────────────────────────────────────────────────────────────────────┘
```

### 1.2 现状结论表（每层：已实现 / 缺口 / 证据）

| 层 | 状态 | 证据（文件:行号） | 缺口说明 |
|---|---|---|---|
| ① 认证 | ✅ 已实现 | `experts_common.rs:585-607`（OptionalAuthUser FromRequestParts） | 可选提取：未认证请求不被拒，降级 system actor |
| ② 路由级角色 | ✅ 已实现 | `router/index.js:115-133`；`modules/expert-alliance/index.js:50/71/91/115/139/152` | 仅前端 UI 层；后端对应写面未二次强制 |
| ② 路由级权限码 | 🟡 部分 | `router/index.js:94-113`（requiresPermission 支持） | 联盟路由未挂 requiresPermission（无权限码可挂） |
| ③ 按钮级指令 | ✅ 已闭环（2026-09-30 G-1） | `directives/permission.js:124-128` v-role-any；7 个管理写面按钮已挂（见 §五按钮级说明） | 自服务写面按钮（咨询/会话/收藏/预约）有意不挂，登录即可 |
| ④ 审计 actor | ✅ 已实现 | `experts_common.rs:613-621`；11 处 handler 调用 | 未认证降级 system（设计取舍，向后兼容） |
| ④ 审计防篡改 | ✅ 已实现 | `experts_common.rs:524`（SHA-256 哈希链）；`:573-574`（HMAC） | 单租户硬编码 `tenant_id="experts-alliance"`（:640） |
| 后端 RBAC 强制 | ✅ 已闭环（2026-09-30） | `alliance/experts_rbac.rs`：`ADMIN_ROLES:50`、`enforce_admin`、`enforce_admin_or_respond`；7 个管理写面 handler 入口强制（registry create/update/delete、graph rebuild、dispatcher update_config/reset/reset_all） | 未认证 401 / 非管理角色 403 + `rbac.denied` 审计；自服务写面与读面仅要求认证 |

---

## 二、角色体系（权威）

### 2.1 现行 5 角色（权威源：前端 `ROLE_TEMPLATES`）

权威定义见 `frontend-ui/src/api/system.api.js:139-180`。这是现行工程里**真实存在**的角色码全集：

| 角色码 | 中文名 | dataScope | 定位 | 用到的联盟路由证据 |
|---|---|---|---|---|
| `super_admin` | 超级管理员 | all（`menuCodes:['*']`） | 全系统权限 | 6 处 requiresRole 均含（index.js:50/71/91/115/139/152） |
| `tenant_admin` | 租户管理员 | dept_and_sub | 本租户用户/角色/部门/菜单管理 | 同上 6 处 requiresRole |
| `dept_manager` | 部门主管 | dept_and_sub | 本部门及子部门用户管理 | 联盟路由**未单独授权**；登录后可访问 requiresAuth 路由 |
| `normal_user` | 普通员工 | self | 本人数据 | 联盟 requiresAuth 路由（工作台/广场/协作/会话） |
| `readonly_auditor` | 只读审计员 | all（仅 view 权限码） | 全量只读、合规审计 | requiresAuth 路由可读；6 处管理路由**不可进** |

补充事实：
- `permission.store.js:53-56`：`'admin'` 为**历史遗留别名**，并入 `ADMIN_ROLES=[admin, super_admin, tenant_admin]`，
  供 `isAdmin` 判定（:127）；`hasRole`（:172-175）**不做别名展开**，严格 `roles.includes(role)`。
- 后果：`views/admin/panels/*.vue` 里的 `v-role="'admin'"`（AdminDepartment:16/51、AdminRole:36/37/87、
  AdminUser:61/109）只匹配字面量 `'admin'`，不匹配 `super_admin`——这是 admin 面板的历史遗留不一致，
  **不在专家联盟模块内**，本文仅记录、不改（见 §六缺口 G-3）。
- 现行 5 角色**彼此平行，无继承关系**（前端无 extends 概念；`hasAnyRole` 是集合 OR）。

### 2.2 目标态参考：39 号文档 6 角色继承链（⚠️ 历史快照，非现行）

> **来源与定性**：`docs/enterprise/39-开发专家联盟-架构诊断与SaaS化最优方案-V1.1.md:24`
> 记载「6 内置角色 ✅：admin→editor→viewer + safety_approver + operator + auditor，代码在
> `platform/domains/mox-expert/src/rbac/policy.rs:133-158`」。
>
> **核证结论**：
> 1. 旧路径 `platform/domains/mox-expert/` **已删除**（`Test-Path` = False；全仓 Cargo.toml grep `mox-expert`
>    零命中）。39 号文档文首（:9）已自行声明该 crate 在 Rust 统一后退役。
> 2. 但这套 RBAC 引擎**没有消失**，它整体迁移到了新 crate：
>    `platform/domains/ai/svc/mox-ai-expert-svc/src/rbac/policy.rs:131-158`，6 角色定义逐字存活：
>
>    | 角色 | 继承 extends | 授予权限（policy.rs 行号） |
>    |---|---|---|
>    | `admin` | editor | `admin:*`（:133） |
>    | `editor` | viewer | `write db:test/*`、`write db:staging/*`、`write flow:*`、`execute flow:*`（:134-139） |
>    | `viewer` | — | `read db:*`、`read flow:*`、`read mem:*`、`execute flow:readonly/*`（:140-144） |
>    | `safety_approver` | — | `admin db:prod/*`、`write/execute flow:gov-pii/*`（:145-148） |
>    | `operator` | — | `read:*`、`execute flow:*`（:149-151） |
>    | `auditor` | — | `read db:*`、`read flow:*`、`read mem:*`、`read audit:*`（:152-156） |
>
> 3. **关键差异**：该 crate 是独立 RBAC 引擎，网关 `mox-platform-gateway-svc/Cargo.toml`
    对其**零依赖**（grep expert/alliance 空）。专家联盟 HTTP handler 走本地 `emit_audit`，**不调用**
    `check_with_audit`。故这 6 角色继承链**不是专家联盟现行授权模型**，仅作为目标态参考存在。

**未来映射建议**（对齐 12 号规划 P0/P1）：
- `safety_approver` → 映射 12 号 P0「人工审批节点 HITL」（11 对标弱⑤）：生产/破坏性写需双人审批。
- `auditor` → 映射现行 `readonly_auditor`（已在前端 5 角色内），后端 `read audit:*` 权限码可未来接入。
- `operator` → 12 号未单独立项；现行工程无此角色码（前端 fix-report 2026-09-27 已注明不引入）。

---

## 三、资源级权限与租户

### 3.1 通配符资源权限

- **引擎存在**：`mox-ai-expert-svc/src/rbac/policy.rs:32-45` 实现 `wildcard_match`：
  `db:prod/*` 匹配 `db:prod/citizen_info`；`*` 全匹配；测试见 :174-205。
- **现行联盟未接线**：专家联盟网关不依赖该 crate，HTTP handler 内 grep `rbac` 零业务命中
  （仅 `experts_common.rs:593` 一条注释提及 `ApiAuth`）。
- **结论**：资源通配权限码（如 `db:test/*`、`flow:gov-pii/*`）在专家联盟域**为目标态**，现行未启用；
  前端 `requiresPermission` 机制虽支持，但联盟路由未挂任何权限码。

### 3.2 租户隔离

- **现状**：单租户。`emit_audit` 内 `tenant_id` 硬编码为 `"experts-alliance"`
  （`experts_common.rs:640`）。
- 前端虽有租户管理 API（`system.api.js:131-136`：`/tenant` CRUD + `/tenant/switch/:id`），
  但联盟运行时无租户维度概念，所有数据平铺。
- **目标态**：12 号 roadmap A1「多租户隔离（数据/配额/密钥）」列为 **P1**
  （`12-innovation-roadmap.md:63`、§4.1 :165-172），依赖 P0 的 N2 审计身份 + G3 角色权限先落地。

---

## 四、审计闭环

### 4.1 `check_with_audit` 核证结果

- **存在**：`platform/domains/ai/svc/mox-ai-expert-svc/src/rbac/check.rs:130-151`。
  语义：先 `check(ctx)`，拒绝时若有 audit_ctx 则 `log_rbac_denied`（:141）。
  导出链：`rbac/mod.rs:19` → `lib.rs:91` → 业务入口 `context.rs:272`。
- **与联盟关系**：⚠️ 该函数在 AI 域 crate，**专家联盟网关未调用**（无依赖、无 import）。
  39 号文档 :27 所写 `platform/domains/mox-expert/src/rbac/mod.rs#L12` 路径已随旧 crate 删除而失效。

### 4.2 现行审计 actor 注入（11 个写面 handler）

证据：grep 网关 `src/alliance/` 下 `emit_audit` / `actor_from_opt_user`：

| 模块 | 文件:行号 | 审计 action |
|---|---|---|
| 调度器 | `experts_dispatcher.rs:581` | `ExpertDispatch` |
| 调度器 | `experts_dispatcher.rs:653` | `expert.consult` |
| 调度器 | `experts_dispatcher.rs:752` | `expert.multi_consult` |
| 注册表 | `experts_registry.rs:376` | `expert.register` |
| 注册表 | `experts_registry.rs:399` | `expert.update` |
| 注册表 | `experts_registry.rs:422` | `expert.disable`（软删） |
| 注册表 | `experts_registry.rs:818` | `expert.consult_now` |
| 会话 | `experts_session.rs:184` | `session.create` |
| 会话 | `experts_session.rs:411` | `session.delete` |
| 会话 | `experts_session.rs:452` | `session.append_message` |
| 会话 | `experts_session.rs:617` | `session.archive` |

共 **11 个生产 handler**。降级路径：未认证/公开路径（`OptionalAuthUser(None)`）→
`AuditActor::system()`（`experts_common.rs:619`），与历史行为兼容。

### 4.3 审计链路防篡改

- **NDJSON 文件 Sink**：`FileAuditSink` 逐行追加 `data/audit/experts-audit.ndjson`
  （`experts_common.rs:528-549`；路径 env `MOX_AUDIT_LOG_PATH` :565-566）。
- **SHA-256 哈希链**：写操作事件写入哈希链防篡改（设计注释 :524；自洽测试 :1003-1056）。
- **HMAC 签名**：密钥 env `MOX_AUDIT_HMAC_SECRET`，默认 `"mox-experts-alliance-audit"`
  （:573-574；`AuditContext.with_hmac_secret` :577）。
- **开关**：`MOX_AUDIT_SINK=noop` 切不落盘（开发/CI，:558-560）。
- 审计发射失败静默不阻断业务（`emit_audit` 末尾 `let _ = state.audit.emit(ev)` :654）。

---

## 五、权限矩阵总表

> 行 = 权限操作；列 = 现行 5 角色。格内：✅ = 可访问（前端路由守卫放行）；
> — = 守卫拦截（跳 /403）。**后端已强制管理写面**（2026-09-30 R1 闭环）：
> 管理写面（专家注册/CRUD、图谱重建、调度配置写、负载重置、专家删除）在网关 handler
> 入口经 `experts_rbac::enforce_admin_or_respond` 强制 super_admin / tenant_admin，
> 未认证 401、非管理角色 403 + `rbac.denied` 审计；自服务写面（consult/会话/收藏/预约）
> 与读面仅要求认证，不强制角色。详见 `platform/domains/alliance/_verification/backend-fix-report.md` R1 节。

| # | 权限操作 / 路由 | super_admin | tenant_admin | dept_manager | normal_user | readonly_auditor | 证据 |
|---|---|:---:|:---:|:---:|:---:|:---:|---|
| 1 | 联盟工作台 `/expert-workspace` | ✅ | ✅ | ✅ | ✅ | ✅ | index.js:25 requiresAuth |
| 2 | 专家广场 `/expert-plaza` | ✅ | ✅ | ✅ | ✅ | ✅ | index.js:79 requiresAuth |
| 3 | 智能协作 `/alliance/collab` | ✅ | ✅ | ✅ | ✅ | ✅ | index.js:102 仅 requiresAuth |
| 4 | 会话中心（自服务）`/alliance/sessions` | ✅ | ✅ | ✅ | ✅ | ✅ | index.js:126 仅 requiresAuth |
| 5 | 联盟管理后台 `/expert-center` | ✅ | ✅ | — | — | — | index.js:50 requiresRole |
| 6 | 专家配置 `/expert-config` | ✅ | ✅ | — | — | — | index.js:71 requiresRole |
| 7 | 联盟控制台（负载重置/调度写）`/alliance/console` | ✅ | ✅ | — | — | — | index.js:91 requiresRole |
| 8 | 协作图谱（重建破坏性）`/alliance/graph` | ✅ | ✅ | — | — | — | index.js:115 requiresRole |
| 9 | 专家编排台 `/alliance/orchestration` | ✅ | ✅ | — | — | — | index.js:139 requiresRole |
| 10 | 专家广场管理（注册/CRUD）`/alliance/experts` | ✅ | ✅ | — | — | — | index.js:152 requiresRole |
| 11 | 专家注册/写（后端） | ✅ | ✅ | — | — | — | registry.rs `create/update/delete_expert` 入口 enforce_admin；experts_rbac.rs |
| 12 | 单专家咨询（后端 consult） | ✅ | ✅ | ✅ | ✅ | ✅ | dispatcher.rs:653；路由 requiresAuth |
| 13 | 多专家咨询（multi_consult） | ✅ | ✅ | ✅ | ✅ | ✅ | dispatcher.rs:752 |
| 14 | 会话创建/追加消息/归档 | ✅ | ✅ | ✅ | ✅ | ✅ | session.rs:184/452/617 |
| 15 | 审计日志查看 `/security/audit-log` | ✅ | ✅* | — | — | ✅ | system.api.js:29；模板 :177 给 auditor operlog/logininfor view |
| 16 | 任务创建/管理（联盟任务页） | ✅ | ✅ | ✅ | ✅ | ✅ | index.js:59 子路由仅 requiresAuth |
| 17 | 调度配置写（dispatcher config） | ✅ | ✅ | — | — | — | 随 /alliance/console 路由收口（index.js:91） |
| 18 | 破坏性写（负载重置/图谱重建） | ✅ | ✅ | — | — | — | 随 /console、/graph 路由收口 |

**继承关系**：现行 5 角色**平行无继承**（前端 store 无 extends）。
目标态继承链（admin→editor→viewer）见 §2.2，属 mox-ai-expert-svc crate，未接线。

**按钮级说明**：表中 ✅ 表示路由可进。路由内 7 个管理写面按钮已于 2026-09-30 挂
`v-role-any="['super_admin','tenant_admin']"`（与路由 requiresRole、后端 ADMIN_ROLES 三端同源），
无权限时按钮从 DOM 移除（隐藏式降级）：专家注册/编辑/停用（ExpertsView:11/218/219）、
图谱重建（GraphView:14）、调度配置保存/单专家负载重置/全量重置（ConsoleView:41/228/245）。
自服务写面按钮（咨询/会话/收藏/预约）有意不挂，登录即可；前端隐藏 ≠ 后端放行，7 个管理写面
后端 handler 入口仍强制（见 §五注释）。详见 `frontend-ui/.../_verification/frontend-fix-report.md`
「2026-09-30 按钮级权限接入」节。

---

## 六、缺口清单与建议

| 编号 | 缺口 | 现状证据 | 闭环状态 | 建议 |
|---|---|---|---|---|
| G-1 | 联盟模块按钮级权限指令 0 使用 | directives/permission.js:124-128 v-role-any 已存在；联盟 views 原 grep 零命中 | **已闭环（2026-09-30）** | 7 个管理写面按钮挂 `v-role-any="['super_admin','tenant_admin']"`（与路由 requiresRole、后端 ADMIN_ROLES 三端同源同语义）；自服务写面按钮不挂。详见 `frontend-ui/.../_verification/frontend-fix-report.md`「2026-09-30 按钮级权限接入」节 |
| G-2 | 后端联盟写面不做角色强制 | 网关 Cargo.toml 未依赖 rbac crate；handler 仅 OptionalAuthUser 审计 | **已闭环（2026-09-30）** | R1 落地：新增 `alliance/experts_rbac.rs`（路径 b 联盟域轻量 RBAC），管理写面 7 个 handler 入口强制 super_admin/tenant_admin，未认证 401 / 非管理 403 + `rbac.denied` 审计；详见 `platform/domains/alliance/_verification/backend-fix-report.md` R1 节 |
| G-3 | admin 面板 `v-role="'admin'"` 字面量与 5 角色码不一致 | permission.store.js:172 hasRole 无别名展开；AdminSso/Role/User/Department 用 `'admin'` | **观察项（非联盟域，不改）** | 未来统一改为 `v-role-any="['super_admin','tenant_admin']"` 或 hasRole 加别名；不在本次范围 |
| G-4 | 未认证请求降级 system actor | experts_common.rs:619 `None => AuditActor::system()` | **已闭环（设计取舍）** | 公开/健康路径可接受；管理写面应要求网关强制登录后再路由 |
| G-5 | 租户维度硬编码 | experts_common.rs:640 `tenant_id="experts-alliance"` | **留待 P1**（12 号 A1） | A1 多租户落地时改为从 UserInfo 提取 |
| G-6 | requiresRole 不匹配降级行为 | router/index.js:125-131 → warning + `/403`；/403 路由存在（fallback.js，bare:true） | **已闭环** | 重定向而非放行，行为正确，无需修 |
| G-7 | 39 号 6 角色/通配权限未接线 | policy.rs:131-158 存活但网关零依赖 | **留待 P1 评估** | 若 A1 多租户启动，可把该 RBAC 引擎接入网关作为资源级判定 |

**本轮（2026-09-30）实际动作**：
- R1（下午）：仅文档固化与证据核证，未改前端代码。原因见下；随后 G-2 后端 RBAC 闭环。
- R1+（傍晚）：**G-1 按钮级闭环**——推翻早前「待后端权限码落地后再补」的判断，改用 `v-role-any`
  角色码路径（而非 v-permission 权限码）：与路由 requiresRole、后端 ADMIN_ROLES 三端同源，
  零新增机制，7 个管理写面按钮挂指令。自服务写面保持登录即可。

---

## 七、结论

1. **现行权限模型 = 前端 5 平行角色 + 路由级 requiresRole 收口 + 后端审计 actor 注入 + 后端管理写面 RBAC 强制（experts_rbac.rs，2026-09-30 闭环）**。
   认证（OptionalAuthUser）→ 路由守卫（requiresRole/requiresPermission）→ 按钮指令（基础设施在、联盟未用）
   → 后端 RBAC 强制（管理写面 7 handler 入口 enforce_admin）→ 审计（NDJSON 哈希链 + HMAC）五层证据齐全。
2. **39 号文档的 6 角色继承链是历史快照**：旧 crate `mox-expert` 已删；引擎迁移至
   `mox-ai-expert-svc` 且逐字存活，但**未接入专家联盟网关**，不能当现行模型引用。
3. **已闭环**：路由级 6 处管理员收口、审计 actor 真实身份注入（11 handler）、未认证降级、
   哈希链 + HMAC、守卫不匹配重定向 /403、后端管理写面 RBAC 强制（R1：专家注册/CRUD、
   图谱重建、调度配置写、负载重置共 7 个 handler；未认证 401 / 非管理角色 403 + `rbac.denied` 审计）。
4. **已闭环**：路由级 6 处管理员收口、审计 actor 真实身份注入（11 handler）、未认证降级、
   哈希链 + HMAC、守卫不匹配重定向 /403、后端管理写面 RBAC 强制（R1：专家注册/CRUD、
   图谱重建、调度配置写、负载重置共 7 个 handler；未认证 401 / 非管理角色 403 + `rbac.denied` 审计）、
   **前端按钮级 v-role-any（G-1，7 个管理写面按钮，三端同源）**。
   **留待 P1**：多租户（G-5/A1）、RBAC 引擎接线（G-7）。
   自服务写面（consult/multi_consult/debate、会话、收藏/预约）与所有读面保持认证即可，不强制角色。
5. 本文所有结论均可凭 `文件:行号` 复核；无编造权限码、无虚构实现。

# 模块治理闭环体检报告（Module Governance Health）

> 生成时间：2026-09-26 ｜ 体检对象：`frontend-ui/`（Vite Vue3 + Pinia + Element Plus）
> 体检方式：新增可重复运行的体检器 `src/modules/health-check.js`（+ `health-check.test.js`），并人工交叉核对后端实况。
> 权威内核：`src/modules/_kernel/module-registry.js`（`defineModule` 单源登记，`collectRoutes`/`collectNav` 派生）。

---

## 0. 结论速览

- **内核注册表当前只登记了 1 个模块：`expert-alliance`**。它四要素齐备、契约往返测试 101 例、模块内 18 个测试文件——是唯一"全绿"的样板。
- `src/modules/admin-lowcode/` 已有低代码引擎骨架（contract/engine/composables/pages）和 1 个测试，但**未在 `src/modules/index.js` 登记、无 defineModule、无 routes/nav/endpoints 契约**，是"半拉子"脚手架（其 `tenant.page.js` 已被 `/admin/tenant-lc` 试点挂用）。
- **导航漂移**：注册表派生的 6 个 `/alliance/*` 页全部未进 64px 图标侧栏；侧栏另有 4 个手写条目来自 legacy 路由，未走注册表。
- **权限矩阵两处真问题**：① `/admin/*` 的 `requiresRole:['admin']` 与后端实况不匹配——后端 `roles` 是对象数组且实际角色码为 `tenant_admin`，前端却用 `includes('admin')` 字符串比对；② `/expert-center/*` 管理台只有 `requiresAuth`，无角色守卫。
- **vitest**：原"30 文件/677 全绿"基线在工作区其实是红的——`auth.store.test.js` 因改名后残留 import 路径而加载失败。已修复，**新基线 31 文件 / 684 用例全绿**。

---

## 1. 四要素完成度表

> 四要素：a) routes（`meta.{title,module,layout}` 齐备）b) nav（`collectNav` 输出且挂载进侧栏）c) endpoints（契约文件 + 双向往返测试）d) tests（`*.test.js` 且 vitest 绿）。
> ✅=齐备 ⚠️=部分/有条件 ❌=缺失 N/A=不适用。

| 模块 | registered | routes | nav | endpoints | tests | 说明 |
|------|:---:|:---:|:---:|:---:|:---:|------|
| **expert-alliance** | ✅ | ✅ 6 条 | ✅ 6 项已挂载 | ✅ `contract/endpoints.js` + `contract.test.js`（101 例双向守护） | ✅ 18 个测试文件 | 唯一全绿样板。`ENDPOINTS`/`FORBIDDEN_ENDPOINTS`/`UNMOUNTED_ROUTES` 三表齐全，与 `docs/API-REGISTRY.md` 逐字对齐 |
| **admin-lowcode** | ❌ 未登记 | ❌ 无 | ❌ 无 | ❌ 无 `endpoints.js` 契约表 | ⚠️ 1 个测试（`contract/pageSchema.test.js` 7 例） | 低代码引擎试点：有 `contract/pageSchema.js`/`engine/widgetRegistry.js`/`SchemaCrudPage.vue`/`composables/useCrudPage.js`/`pages/*.page.js`，但**没有 `index.js`、没调 `defineModule`、没在 `src/modules/index.js` 注册**；`tenant.page.js` 已被 `router/modules/system.js` 以 props 方式挂到 `/admin/tenant-lc` |

**补充：legacy 域路由（未走内核注册表，四要素不适用但登记在册）**

以下业务域走 `router/modules/*.js` 手写路由，不在 `defineModule` 治理范围内，故四要素列记 N/A：

| 域 | 路由文件 | 主要页面 | 内核登记 |
|----|---------|---------|:---:|
| project 项目域 | `router/modules/project.js` | /dashboard /projects /tasks /resources /workbench | ❌ |
| ai 能力域 | `router/modules/ai.js` | /ai /caomei /algolab /botCenter /infinite-optimizer /melody2score /share/:token | ❌ |
| graph 图谱域 | `router/modules/graph.js` | /graph /mox-fusion /flow-graph | ❌ |
| workflow 工作流域 | `router/modules/workflow.js` | /workflow/{flows,plugins,mcp,automation} /browser | ❌ |
| market 市场域 | `router/modules/market.js` | /market /market/:id | ❌ |
| operators 算子域 | `router/modules/operators.js` | /operators | ❌ |
| system 管理域 | `router/modules/system.js` | /admin/*（17 子页）+ /admin/tenant-lc | ❌ |
| expert 联盟 legacy | `router/modules/alliance.js` | /expert-workspace /expert-center/* /expert-config /expert-plaza | ❌ |
| public 游客 | `router/modules/public.js` | /login /register /portal /hall | N/A |

> 治理含义：内核"四要素"目前只覆盖 `expert-alliance` 一家。其余 8 个域是 legacy 手写路由，没有契约端点台账、没有由注册表派生的 nav。要把治理闭环铺到全量，需逐域按 `MODULE-MANIFEST.md §9.1` 迁入 `modules/<域>/`。

---

## 2. admin/panels/ 17 面板完成度表

> 扫描 `src/views/admin/panels/*.vue`。tests=是否有对应测试；unifiedApi=是否走 `@/api` 统一封装。

| # | 面板 | 路由 | 测试 | 统一 API 封装 |
|---|------|------|:---:|:---:|
| 1 | AdminOverview.vue | /admin/overview | ❌ | ✅ @/api |
| 2 | AdminApi.vue | /admin/api | ❌ | ✅ @/api |
| 3 | AdminAudit.vue | /admin/audit | ❌ | ✅ @/api |
| 4 | AdminConfig.vue | /admin/config | ❌ | ✅ @/api |
| 5 | AdminDepartment.vue | /admin/department | ❌ | ✅ @/api |
| 6 | AdminDict.vue | /admin/dict | ❌ | ✅ @/api |
| 7 | AdminDocs.vue | /admin/docs | ❌ | ✅ `import * as api from '@/api'` |
| 8 | AdminHitl.vue | /admin/hitl | ❌ | ⚠️ `@/utils/hitl-ws`（WebSocket 实时审批，**有意例外**，非 REST 面板） |
| 9 | AdminLlm.vue | /admin/llm | ❌ | ✅ `import * as api from '@/api'` |
| 10 | AdminLogs.vue | /admin/logs | ❌ | ✅ @/api |
| 11 | AdminMenu.vue | /admin/menu | ❌ | ✅ @/api |
| 12 | AdminMonitor.vue | /admin/monitor | ❌ | ✅ @/api + `@/api/monitor.api` |
| 13 | AdminRole.vue | /admin/role | ❌ | ✅ @/api |
| 14 | AdminStorage.vue | /admin/storage | ❌ | ✅ @/api |
| 15 | AdminTenant.vue | /admin/tenant | ❌ | ✅ @/api |
| 16 | AdminAccess.vue | /admin/access | ❌ | ✅ @/api |
| 17 | AdminUser.vue | /admin/user | ❌ | ✅ @/api |

**小结**：API 封装统一性 16/17 达标（AdminHitl 为 WS 例外）；**测试覆盖 0/17**——admin 面板是测试盲区。

---

## 3. 导航漂移清单

导航有三层互不从属的来源：① `collectNav()`（注册表派生）② `MODULE_SIDEBAR_CONFIG`（TheSidebar 实际渲染）③ `ICON_NAV_GROUPS`（64px IconSidebar 渲染）。

### 3.1 注册了但导航不出现
- **空**。`expert-alliance` 的 6 个 nav 项全部经 `nav.config.js` 末尾的自动挂载循环落进 `MODULE_SIDEBAR_CONFIG.expert.sections[控制台]`。`wiring.test.js` 第 55-62 例与本体检器均复核为零漂移。

### 3.2 导航有但模块未注册（手写条目）
以下 `MODULE_SIDEBAR_CONFIG` 条目是硬编码、不来自任何 `defineModule`：

| 所属模块 | path | 标签 | 来源 |
|---------|------|------|------|
| expert | /expert-workspace | 联盟工作台 | legacy `router/modules/alliance.js` |
| expert | /expert-plaza | 专家广场 | legacy `router/modules/alliance.js` |
| expert | /expert-center | 联盟管理 | legacy `router/modules/alliance.js` |
| expert | /expert-config | 专家配置 | legacy `router/modules/alliance.js` |

> 另有 `MODULE_SIDEBAR_CONFIG.dashboard/projects/tasks` 的若干条目**无 path**（纯筛选分类，不可点，TheSidebar 已用 `navigable` 类区分），属设计内。`ai/graph/operators/workflow/market/admin` 的 `sections` 为空数组——这些域靠页面内 Tabs 导航，TheSidebar 不渲染侧栏。

### 3.3 图标侧栏（IconSidebar）缺失模块页
`ICON_NAV_GROUPS` 是独立硬编码清单，完全没有收录注册表派生页：

```
/alliance/console /alliance/collab /alliance/orchestration
/alliance/graph   /alliance/sessions /alliance/experts
```

即：用户在 64px 图标栏看不到任何新联盟模块页，只能先落到"专家联盟"图标进 `/expert-workspace` 再绕进去。

---

## 4. 权限矩阵核对结论

### 4.1 前端守卫现状（全量遍历）
- 唯一的路由级角色守卫：`/admin/**` 全部子路由 `meta.requiresRole: ['admin']`（`router/modules/system.js`）。
- **`meta.requiresPermission`：0 处使用**（守卫基础设施在 `router/index.js` 93-113 已就绪，但没有任何路由挂它）。
- `/expert-center/**`（专家管理后台）：只有 `requiresAuth:true`，另有 `isExpertAdmin:true` 这个 meta 标记，**但路由守卫根本不读 `isExpertAdmin`**——任何登录用户都能打开专家管理台。

### 4.2 后端实况（对 :3080 实跑 `/api/system/permissions`）
```jsonc
{
  "permissions": ["user:view","user:create",...,"role:view",...],   // 字符串数组 ✅ 形状对
  "roles": [ {"id":"...","code":"tenant_admin","name":"租户管理员"}, ... ],  // 对象数组
  "menus": [],
  "user_id": "admin-user", "tenant_id": "t001-tenant"
}
```

### 4.3 假权限（前端要求、后端对不上）
| 项 | 前端 | 后端实况 | 判定 |
|----|------|---------|------|
| `/admin/*` 角色守卫 | `hasAnyRole(['admin'])` → `roles.includes('admin')` | `roles` 是 `[{code:'tenant_admin'}]` 对象数组；`permissionStore.setRoles()` 原样存对象，再用字符串 `includes` 比对 | ❌ **形状+码值双重错位**：对象永不等于字符串 `'admin'`；即便归一到 `.code`，实际角色码是 `tenant_admin` 也不是 `admin` |
| `isAdmin` 短路 | `roles.includes('admin') \|\| roles.includes('super_admin')` | 同上，对象数组 | ❌ 同样失效（admin 实际靠 `localStorage` 残留或 dev 态放行，非契约路径） |

> 根因：`permission.store.js` 349-394 直接 `setRoles(data.roles)`，没有把后端 `{id,code,name}` 映射成 `code` 字符串数组；且路由守卫写死 `'admin'`，而后端角色模板（`system.api.js` ROLE_TEMPLATES / `enterprise/tenant.rs`）用的是 `super_admin/tenant_admin/dept_manager/normal_user/readonly_auditor`。

### 4.4 缺权限（后端有能力、前端未加守卫）
| 路由 | 现状 | 建议 |
|------|------|------|
| `/expert-center/**`（专家管理/编排引擎/企业管理） | 仅 `requiresAuth` | 后端存在专家管理能力面，前端应加 `requiresRole:['tenant_admin'/'admin']` 或 `requiresPermission` |
| 全量业务路由 | 均无 `requiresPermission` | 后端已下发细粒度 `permissions` 字符串数组，前端守卫却没用起来，等于后端能力未被前端消费 |

---

## 5. vitest 回归结果

命令：`cd frontend-ui; npx vitest run`

| 项 | 体检前 | 体检后（新基线） |
|----|--------|-----------------|
| 测试文件 | 29 通过 / **1 失败加载**（`auth.store.test.js`） | **31 通过 / 0 失败** |
| 用例 | 674 通过（auth 套件 0 收集） | **684 通过** |

- **修复**：`auth.store.test.js` 第 3、5 行残留旧路径 `../api/auth`（模块早已改名 `auth.api.js`），导致套件加载即红、3 个用例静默丢失。改为 `../api/auth.api` 后恢复。
- **新增**：`health-check.test.js`（4 例）。
- 详细日志：`reports/data/vitest-baseline-after.txt`。

---

## 6. 机制落点（如何重复运行）

新增只读聚合器，**未改动 `module-registry.js` 内核**：

| 文件 | 作用 |
|------|------|
| `frontend-ui/src/modules/health-check.js` | 只读聚合：扫 `src/modules/*`、交叉 `listModules()/collectNav()/collectRoutes()`、扫 admin 面板、算导航漂移，返回结构化台账 |
| `frontend-ui/src/modules/health-check.test.js` | 跑聚合器、`console.table` 打印台账、守住硬不变量（注册即挂载、已注册必有路由导航） |

**重复运行**：
```bash
cd frontend-ui
npx vitest run src/modules/health-check.test.js   # 打印四要素台账 + 守不变量
npx vitest run                                    # 全量回归（新基线 31 文件/684 例）
```

新增业务模块时，台账自动把它列进 `modules` 表；若其 nav 没挂载或已注册却无路由，硬不变量会红。

---

## 7. 遗留缺口（未在本轮修复）

1. **权限守卫断裂（高优）**：`permission.store.js` 需把后端 `roles:[{code}]` 归一为 `code[]`，且 `/admin/*` 的 `requiresRole` 应由 `'admin'` 改为与后端一致的角色码（或直接用 `isAdmin` 短路）。本轮只诊断、未改运行时权限代码（避免动登录链路）。
2. **`/expert-center/*` 缺角色守卫**。
3. **admin 17 面板 0 测试**。
4. **admin-lowcode 未登记**：补 `index.js` + `defineModule`（或明确降级为 admin 域内部实现，不进内核）。
5. **图标侧栏未收录 `/alliance/*`**；4 个 expert legacy 侧栏条目待迁入注册表。
6. 其余 8 个 legacy 业务域尚未迁入 `modules/<域>/` 内核治理。

---

## 8. 裸 hex 换肤迁移 · 批次 4（RegisterExpertDialog 15→0、AlgoLabView 11→0）

两处的共同病灶：组件里写死一套「浅皮淡染底 + 深字对」，所以对换肤全盲，且在深色三皮下真跌破（错误框近白底配深红字只在浅皮成立；选中态深靛字压深蓝底不足 2:1）。迁到既有三档契约：`--x-dim`（淡染底）/ `--x`（字与描边）/ `--x-fill` + `--on-x`（色块底与其上的字）。

四皮逐对 AA 实测（正文阈 4.5，默认/dark/sky/cyberpunk 顺序）：

| 字档 / 底档 | 默认 | dark | sky | cyber |
|---|---|---|---|---|
| `--brand-accent` / `--accent-dim` | 7.81 | 8.02 | 5.57 | 11.65 |
| `--accent` / `--accent-dim`（对照，未采用） | **3.49 不达标** | 8.85 | **2.28 不达标** | 4.52 |
| `--warning` / `--warning-dim` | 7.25 | 8.97 | 6.84 | 8.83 |
| `--success` / `--success-dim` | 6.14 | 5.06 | 5.21 | 12.98 |
| `--danger` / `--danger-dim` | 5.63 | 5.84 | 5.91 | 6.39 |
| `--brand-accent` / 对话框面 | 7.34 | 7.34 | 5.93 | 10.46 |
| `--on-brand` / `--brand-fill` | 6.29 | 6.29 | 5.93 | 10.13 |
| `--on-success` / `--success-fill` | 5.48 | 5.48 | 5.48 | 15.22 |
| `--text-primary` / `--accent-dim` | 12.92 | 14.59 | 8.87 | 16.07 |

结论：AI 章的字取 `--brand-accent` 而非同族的 `--accent`——后者压淡染底在默认皮与浅皮下只有 3.49 / 2.28:1。

真机核验（dev 3020，编译后 scoped 选择器 `.k-gate[data-v-b5524b60]` 等 4 条已在样式表内；探针携带同一 scoped 属性挂进 `.page-container`，逐皮读 computed）：默认皮下三枚章分别为 `rgba(245,158,11,.12)/rgb(245,158,11)`、`rgba(16,185,129,.12)/rgb(16,185,129)`、`rgba(99,102,241,.12)/rgb(165,180,252)`，与上表的 token 解析值逐一对齐。注：章与结论框需后端返回 `analysis.nodes` / `spiralReport` 才真正渲染，此处证据是「编译产物里的规则 + 活页面的 token 解析」，非该节点的真数据像素截图。

门禁与回归：

| 项 | 结果 |
|---|---|
| `check-view-hex.py --check` | 裸 hex 现存 998 处 / 81 文件（基线 1035 / 82），`verdict=PASS` |
| `text-as-fill` 棘轮 | 33 处 / 15 文件，未动 |
| `--selftest` | PASS=58 FAIL=4（与批次前同，未降任何下限） |
| 命名变异体 | M1 迁好的裸 hex 复发→报 `NEW`；M2 字色读 `--x-fill`→报 `ROLE`；M3 色块底读文字档 `--success`→报 `TIER`；3/3 被打红且字节级还原一致 |
| `vite build` | 通过，32.29s |
| `vitest run` | 31 文件 / 684 例全绿（与 §5 基线一致） |

两处 `BASELINE` 条目按规则**删除**（而非写 0），所以复发会以 `NEW` 记账，比 `GROWN` 更严。批次产出一律未提交。

## 9. 裸 hex 换肤迁移 · 批次 5（`src/components/ai/AIChatPanel.vue` 13 → 1）

候选由分类脚本从"纯 `<style>` 且有裸 hex 的活组件"里挑出（同一集合还剩 `FlowDetailDialog` 14、`KnowledgeBasePanel` 14、`AgentFlowPanel` 13、`AgentTaskRunner` 12…）。本文件的 13 处全在 `<style scoped>` 内，挂载点两处：`views/ai/ChatView.vue`（路由 `/ai`）与 `views/expert/panels/ExpertOverviewPanel.vue`。

迁了 12 处，留 1 处：

| 规则 | 迁后 | 保留理由 |
|---|---|---|
| `.suggestion-chip:hover` | `--accent-dim` + `--brand-accent` | 淡染底与字成对换 |
| `.user-bubble` | `--brand-fill` + `--on-brand` | 色块底三件套 |
| `:deep(code)` / `:deep(pre)` / `:deep(pre code)` | `--text-primary` / `--bg-primary` | 代码块不再钉死深蓝底浅字 |
| `:deep(a)` | `--brand-accent` | 链接压在 `--bg-tertiary` 气泡上 |
| `.typing-dot` | `--text-tertiary` | 见下：第一次选 `--brand-fill` 被真机测出退化 |
| `.message-row.error .ai-bubble` | `--danger`（描边）+ `--danger-dim`（底） | 原来的近白底配深红字只在浅皮成立 |
| `.input-wrap:focus-within` | `--brand-accent` + `--shadow-focus` | 见下：`--border-focus` 太弱被否 |
| `.msg-avatar.ai` 的白字 | **保留字面色** | 它的底来自 JS 里 per-assistant 的渐变色，不跟随皮肤 ⇒ 白字才是正确配对，判据是"底跟不跟肤"而不是"看见字面色就换" |

四皮实测（浏览器内解析 token、按 rgba 自下而上合成后算对比度；默认/dark/sky/cyberpunk）：

| 组合 | default | dark | sky | cyberpunk |
|---|---|---|---|---|
| `--on-brand` / `--brand-fill` | 6.29 | 6.29 | 5.93 | 10.13 |
| `--text-primary` / `--bg-tertiary`（AI 气泡） | 13.25 | 16.30 | 9.04 | 15.58 |
| `--text-primary` / `--bg-primary`（代码块） | 15.66 | 17.58 | 8.87 | 17.75 |
| `--brand-accent` / `--bg-tertiary`（链接） | 8.01 | 8.96 | 5.67 | 11.29 |
| `--brand-accent` / `--accent-dim`+`--bg-tertiary`（chip 悬停） | 7.04 | 8.02 | 5.57 | 11.65 |
| `--text-primary` / `--danger-dim`+`--bg-tertiary`（错误框） | 11.80 | 14.74 | 8.65 | 17.01 |
| `--text-tertiary` / `--bg-card`（打字点，装饰） | 5.62 | 5.71 | 5.36 | 5.53 |
| `--brand-accent` / `--bg-card`（聚焦边框） | 7.34 | 7.34 | 5.93 | 10.46 |

三个候选是被数字否掉的：

1. `:deep(code)` 曾用 `--cat-6`（粉字档）：默认/dark/cyber 4.53 / 6.74 / 5.02 尚可，**sky 只有 4.39** ⇒ 跌破 AA。这暴露一处色板欠账：既往 AA 验收只做在 `--cat-n-fill` / `--on-cat-n` 成对上，**`--cat-n` 文字档本身没有四皮 AA 保证**，按家族名挑档不等于挑到合格对比度。
2. `.typing-dot` 第一版迁到 `--brand-fill`，真机读回来是 **2.33 / 2.33 / 5.93 / 9.09** —— 而旧值 `#94a3b8` 在默认与 dark 皮下有 5.70 / 5.71。也就是说"换肤安全"的那一侧（浅皮）确实修好了（2.56 → 5.93），却把两枚深皮换退了 ⇒ 改选 `--text-tertiary`，四皮 5.36–5.71，最坏值优于旧的最坏值。
3. 聚焦边框第一版用 `--border-focus`：它是半透明档（default `rgba(99,102,241,.4)`），压在面板上只有 1.43–3.30，比旧的不透明字面色（3.27–4.47）还虚 ⇒ 改 `--brand-accent`，同时保留 `box-shadow: var(--shadow-focus)`（四皮各配一套光晕，替掉写死的 indigo 半透明）。

门禁与回归：

| 项 | 结果 |
|---|---|
| `check-view-hex.py --check` | 裸 hex 现存 986 处 / 81 文件（基线 1023 / 82），`verdict=PASS`，`grown=0 new=0 palette=0 tier=0 surface=0 role=0` |
| `text-as-fill` 棘轮 | 33 处 / 15 文件，未动 |
| `--selftest` | PASS=58 FAIL=4（与批次前同，未降任何下限；四条红的逐条归因见 §9.1，其中两条属"欠账清得越好越红"的反咬型下限） |
| 命名变异体（二进制 I/O 驱动，逐例字节级还原 + 还原后要求门禁重新 PASS） | M1 复发裸 hex → `GROWN … 1 → 2`；M2 气泡底读文字档 `--danger` → `TIERNEW`；M3 色块底不配 `--on-x` → `ROLE fill-unpaired … .user-bubble`；3/3 打破 |
| `vite build` | 通过（首跑 46.97s，改档后复跑 61s） |
| `vitest run` | 31 文件 / 684 例全绿（纯样式改动，不新增用例） |
| 真机 | dev 3020 活页面：`.ai-chat-panel` 已挂载，scoped 属性 `data-v-cf35ebe1`，编译后样式表内 `.user-bubble` / `.ai-bubble` / `.typing-dot` / `:focus-within` 四条均为 `var()` 形态；上表数值即从该页面逐皮解析所得 |

台账 `BASELINE` 由 `13` **手改成 `1`**（只改这一行，没跑 `--baseline` 全量回填）——因为全量回填会把并行会话的瞬时状态、以及 `src/components/ai/PhasePipeline.vue` 那条按约定永不回填的建议账一起钉进去。留 1 处 ⇒ 复发字面色会被记成 `GROWN`，而不是批次 4 那种删条目后的 `NEW`。批次产出一律未提交。

> **2026-09-27 复核：上面这条守卫已在盘上失效。** 闸门里 `BASELINE['src/components/MessageBubble.vue']` 现为 **154**（与本文件实测 154 处相符：`<style>` 123 + 内联 31，单是 `#6366f1` 就占 31 处），不是那句写的 `1`。⇒ 批次 4 之后有人跑过全量 `--baseline`（正是"不许回填"那条理由所指的事），13 → 154 的增量已被钉成新基线，`--check` 今日仍 PASS（grown=0）。**原段不涂改**，只在此声明守卫已丢：该文件要涨到 155 处才会变红，153 处以内的复发字面色会被静默放行。要把守卫改回按批次 4 的口径，需点名 —— 那会让 `--check` 立刻红 153 处。

### 9.1 那四条 `--selftest` 红的归因（不是本轮造的，且两条是"反咬型"）

`--check` 判 PASS 而 `--selftest` 挂 4 条，逐条读实现后分成三类，处置完全不同：

| # | 用例 | 判的是什么 | 此刻实测 | 性质 |
|---|---|---|---|---|
| ① | components 在账上且不是零头 | `文件数 ≥ 20` **且** `活站点数 ≥ 500` | 20 文件 / **356** 处 | **反咬**：分母是"还剩多少债"，清得越好越红；文件数已压在 20 的下限上，再清零一个组件就同时红两项 |
| ② | 旧三根之外的文件在账上 | `文件数 ≥ 13` 且 `站点 ≥ 81` | **12** 文件 / **67** 处 | **反咬**：同上，且 `scan_sources()` 只登记有命中的文件 ⇒ 把一个文件清零会把它从集合里摘掉，分子按设计就该掉 |
| ③ | 被排除那一层此刻 0 处 | 绕过排除直扫 `src/modules/**` | **6 处 / 2 文件**（`admin-lowcode/engine/SchemaCrudPage.vue` 2、`admin-lowcode/pages/tenant.page.js` 4） | **真 tripwire 命中**：这正是当初设计它要抓的形态——字面量写进了扫描集看不见的目录 |
| ④ | 台账与扫描集同集合 | 账上无已消失文件、也无未记的活文件 | 缺 `src/components/ai/PhasePipeline.vue` | **按约定留红**：该条 37 的建议账用户指示不回填，红了即欠账摆在脸上 |

①②是守卫写法的问题而非代码问题：把"覆盖面"表达成"剩余欠账的下限"，那么迁移进展本身就会把它做红 —— 而按纪律**降下限属于放宽守卫，要用户点名**。可用的替代写法是把判据换成"某批具名文件仍在集合内 / 命中集非空"这类与欠账无关的形式，这样它只会在扫描真的退化时红。③是新模块 `admin-lowcode`（HEAD `a79d434f`）带进来的 6 处，要么清掉它、要么把该层纳入扫描集，两种都是用户的裁决。

另外记一条读红字时的坑：①的标题里写着「实测 25 个文件 696 处」，那是**用例标签里手写的散文**，不是本次测得值（本次为 20 / 356）—— 红字里带写死的数字，只能当作"当初写这条时的现场快照"，别拿来当本轮证据。

## 10. 裸 hex 换肤迁移 · 批次 6（`src/views/project/panels/KnowledgeBasePanel.vue` 14 → 11）

2730 行的活面板，14 处裸 hex 全在 `<style scoped>` 内。挂载点两处：`src/router/modules/project.js:44`（`/resources/knowledge`，`requiresAuth`）与 `src/views/workspace/ExpertWorkspaceView.vue:186`；`src/stories/KnowledgeBasePanel.stories.js` 也引用它，但 story 不入账。

迁 3 留 11。迁的是版本对比对话框的图例色块：

| 规则 | 迁前 | 迁后 |
|---|---|---|
| `.legend-added` | `background: #dcfce7` | `background: var(--success-dim)` + `border: 1px solid var(--success)` |
| `.legend-removed` | `background: #fef2f2` | `background: var(--danger-dim)` + `border: 1px solid var(--danger)` |
| `.legend-changed` | `background: #fef3c7` | `background: var(--warning-dim)` + `border: 1px solid var(--warning)` |

同时给 `.legend-item i` 补 `box-sizing: border-box` —— 不加它，12×12 的色块会因新增描边变成 14×14，改的是布局不是配色。

留的 11 处 = hero 横幅的 5 色 `linear-gradient(135deg, …)` + 压在它上面的 3 处 `#fff` + 三枚 `.bg-orb` 径向光晕（`#818cf8` / `#22d3ee` / `#a78bfa`）。保留理由不是"难改"，而是**渐变令牌的命名空间根本不平行**（本轮实测）：`--hero-gradient` 在 default/dark/cyberpunk 是半透明淡染、在 sky 是近白渐变，且**零消费者**；`--gradient-brand` / `--gradient-hero` 只在 cyberpunk 定义，`--bg-gradient-brand` 只在 sky 定义。⇒ 把这块恒深的品牌横幅"迁到现有令牌"，在 sky 皮下会得到白底白字。要动它得先补一档四皮平行的渐变令牌，那是设计裁决。

四皮实测（dev 3020 活页面 `#/resources/knowledge`，浏览器内解析编译后的规则与 token，按 rgba 自下而上合成，相邻面取 `--bg-card`）：

| 皮肤 | 卡面 | 新增（描边 vs 合成底 / 描边 vs 卡面 / 淡染底 vs 卡面） | 删除 | 修改 |
|---|---|---|---|---|
| 默认（无 `data-theme`） | `rgb(36,40,56)` | 4.77 / 5.77 / 1.21 | 4.73 / 5.29 / 1.12 | 5.50 / 6.81 / 1.24 |
| `default` | 同上 | 4.77 / 5.77 / 1.21 | 4.73 / 5.29 / 1.12 | 5.50 / 6.81 / 1.24 |
| `dark` | `rgb(30,41,59)` | 5.06 / 7.61 / 1.51 | 5.84 / 5.29 / 1.02 | 8.97 / 8.76 / 1.02 |
| `sky` | `rgb(255,255,255)` | 5.21 / 5.48 / 1.05 | 5.91 / 6.47 / 1.09 | 6.84 / 7.09 / 1.04 |
| `cyberpunk` | `rgb(20,33,56)` | 12.98 / 12.37 / 1.05 | 6.39 / 5.42 / 1.18 | 8.83 / 8.43 / 1.05 |

三点读法：

1. 12×12 的图例色块是非文字元素（WCAG 1.4.11 ⇒ 门槛 3:1，不是 4.5:1），但成对后的描边对比最坏值 4.73，四皮全过更严的那条线。
2. 最后一列（淡染底 vs 卡面）**全部 1.02–1.51** ⇒ 光靠底色在四皮下都近似隐形，**描边是承重的**。这正是不能只做"hex → `var(--x-dim)`"机械替换的地方：单换底会"过了门禁、丢了可见性"。
3. 旧值的问题恰恰是皮肤相关的：`#dcfce7/#fef2f2/#fef3c7` 压 default/dark/cyberpunk 的卡面有 13.14–14.72，压 **sky 的白卡面只有 1.09–1.11** —— 浅皮下三枚图例色块整块消失。这是"字面色只在深皮可见"的换肤不对称，也是这批近白 pastel 的由来（它们本来是 Tailwind 浅皮专用色）。

被数字否掉的两版候选：`--*-fill` 实色块压卡面在 default/dark 只有 2.67 / 3.03 / 2.91，跌破非文字 3:1；`--*-dim` 单用即上表最后一列 1.02–1.51。另注意 `--*-dim` 在默认皮下落成半透明（`rgba(16,185,129,.12)`）、在 dark/sky/cyberpunk 是实心 900 系，**不合成直接比色值会高估对比度**。

顺带两处核对：

- `--bg-panel-2`（对话框脚底，第 2678 行）不是影子名：`src/styles/global.css:73` 定义 `--bg-panel-2: var(--bg-tertiary)`，四皮各自覆盖 `--bg-tertiary` ⇒ 跟肤，合法别名，与 `--text-1/2/3` 同类。
- **`.diff-legend` 描述的是屏幕上不存在的颜色**：`renderedCompareTo = computed(() => simpleMarkdownRender(compareTo.value.content || ''))`，而该渲染器只产出 h1–h3 / strong / em / `code.inline-code` / 链接，全文件没有任何 diff 高亮 class ⇒ 图例是假功能。本轮只迁色、不动语义；真正的二选一（实现行级 diff 高亮 vs 删掉图例）是产品裁决，需点名。

门禁与回归：

| 项 | 结果 |
|---|---|
| `check-view-hex.py --check` | 裸 hex 现存 983 处 / 81 文件，基线 1020 / 82（批次 5 后为 986 / 1023），`grown=0 new=0 palette=0 tier=0 surface=0 role=0 textasfill=33/0 verdict=PASS` |
| `--selftest` | PASS=58 FAIL=4（与批次前同，四条红的逐条归因见 §9.1，未动任何下限） |
| `check-theme-tokens.py --check` | 断链令牌 0 / 0，`new=0 verdict=PASS` |
| 命名变异体（二进制 I/O 驱动，逐例字节级还原 + 还原后要求 `--check` 重新 PASS） | M1 图例复发裸 hex → `GROWN … 11 → 12`；M2 淡染底写成文字档 `--success` → `TIERGROWN … 3 → 4`；M3 实色底配 `--text-1` 字 → `ROLE fill-unpaired …:2646 底=--success 字=var(--text-1) 规则=.legend-added`；3/3 打破，存活 0 |
| `vite build` | 通过（46.60s） |
| `npx vitest run` | 35 文件 / 700 例全绿（纯样式改动，不新增用例；用例数由 `FRONTEND-MODULE.md` §8 作权威） |
| 真机 | 编译后样式表内三条图例规则均为 `var()` 形态，scope 属性 `data-v-f5283333`；上表数值从该活页面逐皮解析所得（探针 `<i>` 携带该 scope，挂在 `background: var(--bg-card)` 的面内） |
| 字节复核 | 目标文件 71965 字节、BOM 保留、CR 0、LF 2729 → 2731（+2 行＝一行说明 + 一行 `box-sizing`）；变异体还原后 sha256 与基线一致 |

台账 `BASELINE['src/views/project/panels/KnowledgeBasePanel.vue']` 由 14 **手改成 11**（只动 `check-view-hex.py:146` 这一行，不跑 `--baseline` 全量回填，理由同 §9）。批次 2/3/4/5/6 产出一律未提交。

## 11. 裸 hex 换肤迁移 · 批次 7（`src/views/project/ProjectsView.vue` 15 → 0、`src/components/FlowDetailDialog.vue` 12 → 11）

批次 7 清 −16 处（983 → 967），并把 `ProjectsView.vue` 这个 15 处文件**清零**。挂载点：`src/router/modules/project.js:9-14`（`/projects`，`requiresAuth`）；`FlowDetailDialog.vue` 由项目/工作流多处 `import`，是活组件。

### 11.1 ProjectsView 的六处改动

| 位置 | 迁前 | 迁后 |
|---|---|---|
| 成员头像色板（8 色字面量数组） | `['#6366f1','#06b6d4','#10b981','#f59e0b','#ef4444','#ec4899','#8b5cf6','#14b8a6']` | `AVATAR_RAMP`：`[catFill(n), catInk(n)]` 成对取，第 5 位走 `[var(--danger-fill), var(--on-danger)]`；新增 `avatarInk(i)` 与 `avatarColor(i)` 同源 |
| 头像模板内联样式 | `:style="{ background: avatarColor(0) }"`（字色靠 `.mini-avatar{color:white}`） | `:style="{ background: avatarColor(i), color: avatarInk(i) }"`，并**删掉** `.mini-avatar` 里的 `color: white` |
| 「+N」汇总徽标 | `background:#f1f5f9; color:#64748b` | `background:var(--bg-tertiary); color:var(--text-secondary)` |
| `.top-btn.primary` / `:hover` | `#6366f1` / `#4f46e5` 底 + 白字 | `var(--brand-fill)` / `var(--brand-fill-hover)` 底 + `var(--on-brand)` 字 |
| 语法高亮五档 | Material 常量 `#c792ea/#c3e88d/#546e7a/#82aaff/#f78c6c` | `var(--brand)` / `var(--success)` / `var(--text-tertiary)` / `var(--info)` / `var(--danger)` |
| （未动）`.member-avatar-lg{color:white}`、`.list-item.active .mini-avatar{border-color:rgba(99,102,241,.2)}` | — | 白字压在 JS 渐变兜底上，属渐变批次（§10 的命名空间不平行问题），本轮保留 |

头像为什么必须**成对**迁：白字压 `--cat-n` 文字档在 dark/cyberpunk 只有 1.77–2.98:1，只换底不换字等于把不可读从一种皮搬到另一种皮。

语法色有一处诚实性代价，已写进代码注释：**sky 皮下 `--brand` 与 `--info` 取同一值**，关键字与函数会并成同一种色相。这是现有令牌面里唯一四皮全过 AA 的五档组合；拉开色相属 #27 的色板裁决，不在本轮动手。

### 11.2 FlowDetailDialog 迁 1 留 11

`.nd-type--cyan` 的 `#0891b2` → 底 `var(--accent-50)` + 字 `var(--cat-2)`（成对）。余 11 处：4 枚 pastel 节点类型徽标（`#f5f3ff/#7c3aed`、`#fff7ed/#ea580c`、`#f0fdfa/#0d9488`、`#fdf2f8/#db2777`）卡在 #27（现有 8 档没有对应的橙/紫/青/粉文字档）；`.log-pre` 的 `#0b1020` 与两处视频底 `#000` 是恒深表面，保留并附实测。

### 11.3 四皮实测（真机 3020，`#/projects`，浏览器内解析 token 并按 rgba 自下而上合成）

语法五档（文字 vs 承载面，门槛 4.5:1）：

| 皮肤 | keyword | string | comment | function | number |
|---|---|---|---|---|---|
| 默认 | 8.00 | 7.44 | 7.24 | 7.42 | 6.82 |
| `dark` | 6.46 | 10.02 | 7.51 | 7.57 | 6.96 |
| `sky` | 5.57 | 5.14 | 5.03 | 5.57 | 6.07 |
| `cyberpunk` | 11.19 | 15.22 | 6.80 | 7.79 | 6.66 |

头像八档（`catFill` 底 vs `catInk` 字）：

| 皮肤 | cat1 | cat2 | cat3 | cat4 | danger | cat6 | cat5 | cat8 |
|---|---|---|---|---|---|---|---|---|
| 默认 | 4.94 | 7.77 | 7.44 | 8.79 | 4.83 | 5.35 | 4.72 | 7.58 |
| `dark` | 6.46 | 10.65 | 10.02 | 11.53 | 4.83 | 7.27 | 7.08 | 10.34 |
| `sky` | 6.29 | 5.36 | **4.71** | 5.02 | 6.47 | **4.60** | 5.70 | **4.69** |
| `cyberpunk` | 11.19 | 15.22 | 15.41 | 10.38 | **4.92** | 5.72 | 4.99 | 12.11 |

「+N」徽标：default 8.40 / dark 12.02 / sky 8.71 / cyberpunk 11.28。14 组配对 × 4 皮，**失败 0**；最薄四组全在 sky 皮（4.60–4.71），比旧值（白字压深皮 1.77–2.98）是质变。

**未取得的证据要说清**：`.top-btn.primary` 与 `.mini-avatar` 两行**没有真机渲染元素级数值** —— 该页 `loadAll` 在取数失败时抛 `ReferenceError`（见 §11.5），页面上根本没有渲染出按钮与头像，探针 `hasPrimary:false / miniAvatarCount:0`。这两处只有「编译后样式表里已是 `var()` 形态」+「token 在四皮下解析成的色值」两级证据，不算渲染断言。

### 11.4 门禁与回归

| 项 | 结果 |
|---|---|
| `check-view-hex.py --check` | 裸 hex 现存 **967 处 / 80 文件**，基线 1004 / 81；`grown=0 new=0 palette=0 tier=0 surface=0 role=0 textasfill=33/0 verdict=PASS`（`textasfill` 存量 33 处/15 文件未动） |
|  advisory | `SHRANK src/components/ai/PhasePipeline.vue 37 → 0` 仍在输出里，**故意不回填**（回填即抹掉那 37 处的账，见 §9.1） |
| `--selftest` | PASS=58 FAIL=4（与批次前逐条同，未动任何下限） |
| `check-theme-tokens.py --check` | 断链令牌 0 / 基线 0，`new=0 verdict=PASS` |
| 命名变异体（二进制 I/O 驱动，needle 命中数先断言、`finally` 里字节级还原 + 校验 sha） | M1 `--danger` 改回 `#f78c6c` → `NEW src/views/project/ProjectsView.vue:628`；M2 `--brand-fill` 改文字档 `--cat-2` → `TIERGROWN 6 → 7`；M3 `--on-brand` 改成 `--text-primary` → `ROLE fill-unpaired :1301/:1307`；**3/3 打破，存活 0**，还原后 sha 与基线一致 |
| `vite build` | 通过（31.07s） |
| `npx vitest run` | **跑到底了，且是真判决**：`Test Files 51 passed (51)` / `Tests 718 passed (718)` / rc=0 / `Duration 1023.84s`。修之前同一棵树是 `3 failed \| 48 passed (51)`、`708 passed \| 3 failed`、`135.37s`，再往前（本批次开头）连汇总行都吐不出来。判决是怎么来的、代价是什么，见 §11.4.2。 |
| 字节复核 | `ProjectsView.vue` sha256 `2ba864c6…caddcc`、`git diff --stat` 68+/54−；`FlowDetailDialog.vue` 674eba7b…（未变动部分）；`check-view-hex.py` 只在 `BASELINE` 删了 ProjectsView 一行、把 FlowDetailDialog 由 12 改成 11 |

台账口径：清零文件的 `BASELINE` 行**删除**而非写 0 —— `scan_sources()` 只登记有命中的文件，留 0 会让复发被判成 `SHRANK` 而不是 `NEW`。

#### 11.4.1 `vitest` 跑不到底的两个具名原因（本轮逐文件单跑钉死）

先立正对照：`npx vitest run src/api/http.test.js` 单独跑 **1 文件 / 2 例通过，9.58s** ⇒ 测试框架、`vitest.config.js`、`happy-dom` 环境本身没坏，"跑不到底"是**文件特定**的，不能拿它当"环境问题"糊过去。

四个嫌疑文件逐个单跑（每个 `timeout 90` 上限）：

| 文件 | 单跑结果 | 判决 |
|---|---|---|
| `AdminUser.smoke.test.js` | `Error: [vitest] No "registerProjectIdGetter" export is defined on the "@/api" mock` → `Test Files 1 failed`，`Tests no tests`，6.35s | **采集期就失败**，一条用例都没收集 |
| `AdminDepartment.smoke.test.js` | 同上，8.91s | 同上 |
| `AdminLlm.smoke.test.js` | 90s 内只吐出 `RUN` 横幅，被 `timeout` 杀 ⇒ 外层显示 `Error: Worker exited unexpectedly` | **真挂起** |
| `AdminMonitor.smoke.test.js` | 同上 | **真挂起** |

两类各一个来源：

1. **秒失败**那一类：`vi.mock('@/api', () => api)` 手写的假象没有覆盖 `@/api` 的真实导出面。`registerProjectIdGetter` 定义在 `src/api/http.js:196`，经 `src/api/index.js:6` 再导出，被 `src/composables/projectContext.js:9` 导入并在 `:21` 顶层调用 ⇒ 采集期就抛。这与 §11.5 的 `ElMessage` 账同族：**mock 是逐字面量手抄的，导出面一动就静默裂开**。
2. **挂起**那一类：`vi.mock('@/api', () => new Proxy({}, { get: () => vi.fn().mockResolvedValue(…) }))`（`AdminLlm.smoke.test.js:8` 用 `[]`、`AdminMonitor.smoke.test.js:16` 用 `{}`）。这种 Proxy 对**任何**属性都返回函数，于是 `then` 也是函数 ⇒ 假象对象成了 thenable，`await` 它永不 settle。最小复现（纯 node，不涉 Vue）：`Promise.resolve(proxyThenable)` 5s 后既不 settle 也不 reject，只打印 `still-pending after 5s`。因为卡在采集/interop 而不是用例体内，`--testTimeout` / `--hookTimeout` 都管不到它 —— 这解释了为什么挂起时连一条 `×` 都不吐。

对全量账的影响（诚实口径）：全量那轮 `48 有上报 / 0 失败标记 / 3 永不上报`，其中 `AdminDepartment` 属"秒失败"类，它在整跑里既没报 `✓` 也没报 `×` ⇒ 48 这个数字只覆盖**池子来得及转达**的文件，不能读成"其余 48 个都通过"。同理，前几轮报告里写的 `35 文件 / 700 例` 与本轮实测的 **51 个磁盘测试文件** 已经不一致 —— 用例数权威仍是 `FRONTEND-MODULE.md` §8，但该处数字需要复算（不在本轮纯样式改动范围内，未动）。

因此本轮回归口径降级为：**hex 门禁 + 三枚命名变异体 + `vite build` 通过；全量用例未通过验证，且已知两个文件挂起、两个文件采集期失败（均与本批次的 `<style>` 改动无关）**。修这四条属"硬引用/mock 面"半边，需点名后再动（同 §11.5）。

> 上面这段是**修法落地前**的现场记录，保留不删；§11.4.1 列的两个挂起文件与两个采集期失败文件已在同批次内修完，判决与代价见 §11.4.2。

#### 11.4.2 把判决修回来：18 个测试文件（堵死判决的 7 个 + 顺手拆雷的 11 个）+ 3 个源文件，以及一项要披露的代价

`npx vitest run` 第一次跑到底时给出的不是绿，是 `Test Files 3 failed | 48 passed (51)`。三类原因，逐个钉死后重跑为 `51 passed (51)` / `Tests 718 passed (718)` / rc=0。

**(a) 采集期就失败的 4 个文件**：本轮整跑点名的是 `AdminRole.smoke.test.js` 与 `modules/admin-lowcode/schema-crud-page.smoke.test.js`（两条 `Failed Suite`，`No "registerProjectIdGetter" export is defined on the "@/api" mock`），加上 §11.4.1 已经钉过的 `AdminUser` / `AdminDepartment`。根因是 `vi.mock('@/api', () => api)` 手抄的导出面永远比真实面少几个 —— 少的那几个不一定被本文件用到，而是被**传递依赖**在采集期要：`src/composables/projectContext.js` 要 `registerProjectIdGetter`（`src/api/index.js:6` 再导出）、`src/stores/auth.store.js:8` 要 `login/refreshToken/getCurrentUser/register/logout`、`src/modules/expert-alliance/api/alliance.api.js:53` 的默认参要 `http`。⇒ 报的都是别人家的账，本文件一条用例也收不到。
改法只认一种：`vi.mock(import('@/api'), async (importOriginal) => ({ ...(await importOriginal()), ...api }))`。**另外 11 个同型文件一并换掉**（`AdminAudit` `AdminConfig` `AdminApi` `AdminAccess` `AdminDict` `AdminLogs` `AdminMenu` `AdminOverview` `AdminStorage` `AdminTenant` `AdminDocs`）—— 它们本轮是绿的，但裂口与上面 4 个是同一处，导入图再动一下就点亮，留雷比改更贵。逐个 `grep -c` 复核为 1 处、EOL 仍为 LF；⇒ 本项合计改 **15 个测试文件**。

**(b) 挂起的 2 个文件**（`AdminLlm` `AdminMonitor`）：懒桩 Proxy 把 `then` 答成函数 ⇒ 命名空间成了 thenable，interop 在采集期 `await` 它永不 settle（`--testTimeout` 管不到）。`get` 陷阱里把 `then` 答 `undefined`、`__esModule` 答 `true`、symbol 透传即可。
反直觉的一条：**Proxy 的 target 一旦非空**，vitest 的 `assertMissingExport` 就按 `in`（走 `has`）判缺，于是"给几个端点配真形状、其余懒答"这条路是堵的（实测 `No "http" export is defined on the "@/api" mock`）⇒ 要形状就只走 (a) 的 partial mock。

**(c) 真断言红的 1 个文件**：`src/stores/auth.store.test.js` 3 例全报 `authApi.getCurrentUser is not a function`。根因不在我改的配色批次里 —— 工作区里 `api/auth.js` → `api/auth.api.js` 那次拆分（该文件与 `auth.store.js` 均已是未提交状态）把 store 的取值口换成了 `@/api` 的**命名**导出（`auth.store.js:8` + `src/api/index.js:27` 的 `export *`），而测试的替身只补了 `default` ⇒ 三条用例从头到尾没在演 store。修法是同一个 `vi.fn` 同时挂命名口与 `default`，store 与本文件指的才是同一个假象。

**(d) 修 test 才照出来的 3 个源文件缺陷**（`<script setup>` 里的裸自由变量，`vite build` 看不见、浏览器必抛）：`AdminLlm.vue` 缺 `catFill`、`AdminMonitor.vue` 缺 `catColor/tokenColor/withAlpha/themeRevision`、`views/expert/panels/ExpertEnterprisePanel.vue` 缺 `tokenColor/themeRevision`。补 import 后写了一个只读扫描器（剥注释、按 palette 出口名匹配、认 barrel `@/constants` 与本地绑定），命中数 3 → **0**；正对照＝4 个走 barrel 导入的文件不被 flag。这一类的账与 §11.5 的 `ElMessage` 同族，但比它小得多（`ElMessage` 那 33 文件/458 处仍未动，等点名）。

**非空转证据（两枚命名变异体，各自打红，跑完 `finally` 里还原并 `grep -c` 复核 import 仍在）**：

| 变异针 | 结果 |
|---|---|
| M1 删掉 `AdminLlm.vue` 的 `import { catFill } from '@/constants'` | `Tests 2 failed (2)`，两条都是 `ReferenceError: catFill is not defined` ⇒ "挂载不崩"这条断言真的在演渲染 |
| M2 `catFill(3)` → 字面量 `'#6366f1'` | 只有色档那条红：`AssertionError: expected 'background: #6366f1;' to contain 'var(--cat-'`，计数那条照常 `✓` ⇒ 两枚断言各钉各的性质，不是同一条在撑 |

**要披露的代价**：整跑 `Duration` 从 **135.37s 涨到 1023.84s**（约 7.6×）。归因探针：单独跑那 11 个被改成 partial mock 的文件 = `11 passed (11)`，`Duration 172.87s`（≈16s/文件，而整轮其余 40 个文件合计约 2.7s/文件）⇒ 慢在 `importOriginal` 把真实 `@/api` 图（含 `expert-alliance` 侧的采集期副作用）在每个 fork 里重新transform/导入一遍。**没有**测过"只补 3 个缺失导出的快写法"能省多少，也没为省时间回退——这是"判决正确 vs 跑得快"的取舍，留给点名：要么接受 17 分钟，要么在 `_smoke.js` 里放一份**完整**导出面替身 + 一枚"替身必须覆盖真实命名导出"的守卫用例（守卫里才用 `importOriginal`，只付一次钱）。

**未取得的证据**：本轮整跑是在网关与 `vite dev` 同机共存下测的，`Duration` 含同机噪声，只当量级看；三条 auth 用例修的是测试替身的形状，没验证 `loginWithToken` 对真后端的行为（那要 §11.5 之外的联调）。

### 11.5 本轮撞出来的第二个账：`硬引用`半边是真故障

批次 7 的取证过程逼出一个与配色无关、但属 #17「硬引用」范畴的功能缺陷：**全应用的接口失败提示路径都会抛 `ReferenceError: ElMessage is not defined`**。

- 现场：真机 `#/projects` 控制台栈 `ReferenceError: ElMessage is not defined at loadAll (src/views/project/ProjectsView.vue:392)`，以及共享层 `src/api/http.js:157`。
- 静态账（只读探针，5 项正对照全 OK 后才取数）：引用 EP 函数式 API 的文件 80 个，其中**缺 import 的 33 个 / 458 处**——`views/admin/panels/*` 15 文件 206 处、其他 view/panel 12 文件 223 处、基础设施 3 文件 21 处（含 `api/http.js` 6、`utils/message.utils.js` 4、`composables/useMessageActions.js` 11）、测试文件 3 个 8 处。
- 根因：`vite.config.js:63` 只挂了 `Components({ resolvers: [epSubpathResolver()] })`，**没有 `unplugin-auto-import`** ⇒ 模板里的 `<el-*>` 能自动解析，脚本里的 `ElMessage` 必须逐文件 import（正确写法见 `src/main.js:3` 的 `element-plus/es/components/message/index`）。已按此改的文件（47 个）不受影响。
- 为什么单测全绿却线上报错：`src/views/admin/panels/_smoke.js:52-53` 与各面板 smoke 测试自己 `globalThis.ElMessage = {…}`（注释写着「面板 script 里 ElMessage/ElMessageBox 是全局自由变量（成功路径不触发），兜底为 no-op」）。⇒ **测试在运行时补上了源码假装存在的那个全局**，成功路径不触发，失败路径在生产里必抛。这条比色值更要紧，但它是功能改动、超出配色批次，**未动任何文件**，等点名。
- 探针自身的教训：第一版把 `from 'element-plus'` 写成精确匹配，而真语料走的是 `element-plus/es/components/...` 子路径 ⇒ 报出「80/80 全缺」的假账（正对照 `main.js not flagged` 当场判 BAD）。修好匹配并保留 5 项正对照后才得到 33/458。

### 11.6 过程教训（记进变异驱动口径）

驱动脚本第一次跑在 GBK 控制台上，`print('→')` 抛 `UnicodeEncodeError` 且**还原步骤排在 print 之后** ⇒ 目标文件带着变异体 `#f78c6c` 留在磁盘上。三律：① `sys.stdout.reconfigure(encoding='utf-8')` + `PYTHONIOENCODING=utf-8`；② 还原必须在 `finally`，不能跟在自家公司 print 后面；③ 每轮开工前存 sha、收工后 `grep -c` 那个变异针还在不在。本轮是靠 `grep -c f78c6c == 1` 抓回来的。

批次 7 产出一律**未提交**（含 §11.5 的只读探针，未改任何源文件）。

### 11.7 #17 余量的分类账：抽样一个文件 23 处，处处卡在"谁的裁决"上而不是"肯不肯花力气"

抽样对象是本轮刚给它建了真断言冒烟用例的 `AdminLlm.vue`（改测试要能被打红才建得起断言，反过来该文件的色值也最好验）。

**先记一次自我更正**：本节第一版写作「该文件余 11 处」，那是**按行数**数的，而计数器给的是 **23 处** —— `:602-611` 每条 `linear-gradient(135deg,#a,#b)` 算 **2 处**（10 条 = 20 处），加 `:572` 一枚 `#94a3b8`，再加第一版**整个漏掉**的 `<style>` 内两枚 `#fff`（`:962` `.provider-icon`、`:1102` `.preset-icon`）。⇒ 口径更正：**「余 N 处」必须由计数器给，不能由看代码的人给**。逐类看：

| 处 | 现场 | 为什么不是"有现成令牌可迁" |
|---|---|---|
| `:602-611` 10 条 `linear-gradient(135deg,#a,#b)` = **20 处** | `getProviderColor()` 的**厂牌识别色**（deepseek / volcengine / qwen / zhipu / openai / anthropic / google / ollama / local / custom），消费口 `:43` `.provider-icon` 与 `:230` `.preset-icon` 的 `background` | 识别色**不许跟皮肤**：迁到 `--cat-n-fill` 后同一个渠道在四皮下换四种颜色，等于拿色档语义当品牌用。与 `EXPERT_COLORS`（16 身份色 vs 8 档）同案 ⇒ 卡在 #27，不在我能自行下的范围 |
| `:572` 一枚 `'#94a3b8'` | `kpis` 里"当前使用＝无"那一条的条形底色（模板 `:22` 把它当 `background` 用；其余三条同位置走 `catFill(1..4)`） | 中性**色档**家族不存在：本轮 `--check` 打印的齐备家族只有 accent brand cat-1..cat-8 danger success warning；对 `src/styles`+`src/constants`+`src/assets` 扫 `--slate/--neutral/--gray/--grey/--muted/--ink/--mute` 前缀，在 615 条变量定义里 **0 命中**。唯一能承载灰色的是表面阶梯名（`--bg-tertiary`、`--border*` 8 个），但那些是"底"不是"色块"，进度条用它等于把它压成与卡片同色 ⇒ 可见性取舍，也不是机械修 |
| `:962` `.provider-icon` 一枚 `color: #fff` | 厂牌图标底上的字母/首字，底色由 `:43` 内联 `background: getProviderColor(p.type)` 给 | 这处**按判据本来就不该迁**：底色是恒深识别渐变、不跟皮肤，而 `--on-cat-n` 那一档是与 `--cat-n-fill` 配成 AA 对儿的（四皮逐对 ≥4.5）。把白字换成跟随皮肤的 `--on-cat-n`、底仍是不跟皮肤的 #4f46e5，等于**拆掉自己配的 AA 对** |
| `:1102` `.preset-icon` 一枚 `color: #fff` | 同上，消费口 `:230` 的预置图标 | 与 `:962` 同案。⇒ 该文件 23 处里**没有一处属"无需点名即可迁"**：20 处卡 #27、1 处缺中性档、2 处迁了反而错 |

顺带把**判据 5 的第二本账**钉清楚（脚本 `:217-219` 的原注释）：`ROLE_TIER_BASELINE` 存量 **33 处 / 15 文件**（第一版按旧快照写的 36 / 16 已按 `--check` 实测更正）是"拿文字档当色块底"的装饰性色块（圆点 / 进度条 / 滚动条），换成 `--x-fill` 会让它们在四皮下**同时变深** —— 脚本自己写明"属设计取舍而不是机械修，先钉住不许增长，等用户点头再逐处收"。该账最大四户：`ProjectsView.vue` 6、`IconSidebar.vue` 4、`AdminMonitor.vue` 4、`AllianceTaskView.vue` 4。**两本账记的是不同东西，不可相加**：前者数 hex 站点，后者数「拿文字档当色块底」的令牌用法。

把抽样放大成**全账分区**（同一支只读探针直接 `import` 闸门模块、调它自己的 `scan_sources()` / `role_scan()`，不复制判据 ⇒ 口径不会和 `--check` 分家）。本轮 `BASELINE` 覆盖的 **967 处 / 80 文件**按所在区间分布：

| 区间 | 处 | 含义 |
|---|---|---|
| `<style>` | 419 | 纯样式块内的裸 hex，多与选择器/皮肤直接挂钩 |
| template / JS 内联 | 386 | 内联 `:style`、`color:` 字段等，能解析 `var()`（见「JS 侧色值的唯一出口」） |
| `<script>` 常量表 | 162 | 色值表（`EXPERT_COLORS` / `getProviderColor` 一类），**最可能不跟皮肤**，也是 #27 的腹地 |

探针的四道对照（对照失败也是产出）：

| 对照 | 结果 | 处置 |
|---|---|---|
| C1 与 `--check` 同源 | OK（子进程再跑一次 `--check`，正则解析「现存 N 处 / M 个文件」与探针逐项相符） | 分区账可引用 |
| C2 `AdminLlm.vue` 应为 11 | **FAIL ⇒ 实为 23** | 本节第一版按行数写错了，已更正（见上） |
| C3 账里不该出现 `PhasePipeline` | **FAIL ⇒ 出现 18 处** | 同名两文件：已清零的是 `src/components/ai/PhasePipeline.vue`（不在账上），仍有 18 处的是 `src/components/PhasePipeline.vue`。⇒ 任务标题只写「PhasePipeline」不够，**必须带全路径** |
| C4 `.css` 里的 hex 应全在 `<style>` | **空转**（`.css` 命中 0 处，真式子集，判据没通电） | 不计为证据；`.css` 不在 `scan_sources` 的扩展名集合内，该对照要换载体才成立 |

⇒ 诚实口径：**本轮没有减少任何裸 hex 计数**，因为抽样文件的 23 处逐处看下来：20 处卡 #27、1 处缺中性档、2 处迁了反而错。"无需点名即可迁"的余量还剩多少，要把 `BASELINE` 逐文件跑一遍"同族可迁令牌在不在"才知道 —— 该项的**分区**部分已做（上表 419 / 386 / 162），**逐处判"同族可迁令牌在不在"仍未做**，不能记成已完成。

**2026-09-27 补：那道未做的判据已跑第一遍（口径是"同名同值"，不是"该迁"）。** 从 `src/**` 的 .css/.js/.vue 里抽出所有 `--x: #hex` 定义（119 个不同色值有令牌），与 967 处逐处对色值：**556 处（57.5%）此刻就有一个同值令牌**，分区 `template/js` 259、`<style>` 178、`<script>` 119（各对 386 / 419 / 162）。命中最多的字面量：`#6366f1` 85、`#10b981` 57、`#06b6d4` 50、`#f59e0b` 44、`#8b5cf6` 30、`#4f46e5` 25。命中最多的文件：`MessageBubble.vue` 99/154、`expert.constants.js` 34/51、`ExpertConfigView.vue` 33/49、`ProjectChip.vue` **13/13**。两道对照：`556+411=967` 成立、命中文件集合 ⊆ 扫描集成立。
**这个 556 不许当"无需点名即可迁"的余量**：它只证明"值恰好相等的令牌存在"，不证明该迁。反例就在榜上 —— `expert.constants.js` 那 34 处是**身份色**（同 #27 的 16 身份色 vs 8 档），迁成跟皮肤的令牌等于让专家换皮换色；`<script>` 那 119 处整体都是这类。真正的判据仍是逐处问一句「这个颜色该不该跟皮肤」（见 §「JS 侧色值的唯一出口」）。可下手的第一个单元是 `ProjectChip.vue`：13/13 全命中且**活**（`src/components/index.js` 与 `views/workflow/panels/AutomationPanel.vue` 两处引用），含一组徽标底/字对（`:118` `#fff7ed` 压 `#c2410c`）⇒ 按"字色连底一起迁"成对处理，**本轮未动**，留作下一单元的验收场（改完必须 `--check` 报 SHRANK 且被一枚具名变异体打红）。

### 11.8 量出来的新缺陷：淡染档 `--x-50` 压文字档 `--x` 在**默认皮**只有 1.96–2.27:1，活组件已中招

打算按"邻居怎么写就怎么写"迁 `ProjectChip.vue:118` 的 warn 徽标（`#fff7ed` 底 + `#c2410c` 字，实测 **4.88:1** 但四皮不跟），照抄上一行 `.pc-status.done` 的 `background: var(--success-50); color: var(--success)` 之前先算了对比度，结果是**这个"现成写法"本身有缺陷**（默认皮的 `--x-50` 是 `rgba(16,185,129,.12)` 一类淡染，`--x` 是中调，压不住）：

| 皮肤 | `--success` 压 `--success-50` | `--warning` 压 `--warning-50` | `--on-x` 压 `--x-fill`（AA 档） |
|---|---|---|---|
| 默认 | **2.27** ❌ | **1.96** ❌ | 5.48 |
| dark | 5.06 | 8.97 | 5.48 |
| cyberpunk | 12.98 | 8.83 | 15.22 |
| sky | 5.21 | 6.84 | 5.48 |

⇒ 三条后果：① `ProjectChip.vue:117`（`.pc-status.done`）在默认皮下是**真实不可读**（2.27:1），不是待迁的欠账而是已上线的缺陷；② 556 处"同值令牌存在"的余量里 `<style>` 那 178 处**不许按同值机械迁** —— 迁到淡染档就是把 4.88 换成 1.96；③ 判据该补一条静态可检的：同一规则里出现 `background: var(--x-50)` 且 `color: var(--x)` ⇒ 缺陷（这条不需要真机就能执法，且不会反咬：它只钉组合，不钉存量数字）。

**未动任何源文件**（本轮 #17 的裸 hex 计数仍为 0 减）。修法三案待点名：A 把这类徽标迁到 `--x-fill`+`--on-x`（AA 有保证，但从"淡底深字"变"深底白字"，属 #27 的观感验收）；B 给默认皮补一档"压得住淡染的文字档"（新增令牌，属 #27 色板裁决）；C 只按 A 修已经 2.27 的那一处，`#fff7ed`/`#c2410c` 留原样。

**2026-09-27 复核：上面这张表的标题级结论撤回。** 把 6 家族 × 4 皮 × 3 档整矩阵算完（只读探针 `D:/tmp/b13_pairaudit2.py`；5 枚控制项全过，其中一枚是「拿 `--success` 压回 `#ffffff` 复算 == 2.27」专门与上表对号，另一枚变异体是「字色 == 底色必须被判红」），真相是**上表把两种基底混写在了同一列里**：

| 基底 | 判的是什么事 | FAIL 格数 | 具体 |
|---|---|---|---|
| A＝各皮自己的 `--bg-card`（默认 `#242838`） | 这对令牌**自洽**吗 | **4 / 46**，全在 `--accent` | 默认 wash 2.69、dim 2.90；sky wash 2.33、dim 2.28 |
| B＝组件写死的近白底（`#fff`→`#f8fafc`） | 这对令牌**落在别人家的底上**会怎样 | **14 / 46** | 默认皮 6 族 wash 全灭（success 2.27、warning 1.96、danger 2.37、brand 2.03、info 2.29、accent 3.99）；dark/cyberpunk 只输 `--info` 2.29；sky 输 `--accent` |

⇒ 四条更正：

1. **`--success-50` 压 `--success` 在默认皮不是 2.27 而是 4.79**（warning 同理 5.51）。上表那两格是**基底 B** 的数，被错标成了「默认皮肤」。原段不涂改，只在此声明作废。
2. **真正的淡染档缺陷只有一族**：`--accent`（默认皮 wash 2.69 / dim 2.90，sky 2.33 / 2.28），而**两皮病因不同**，且两因都是 grep 出来的现值、不是推断：
   - **默认皮＝淡染档串了门**。「global.css:148」`--accent-50: rgba(6,182,212,.12)` 是**青**色，而「:26」`--accent: #6366f1` 是**靛**色 ⇒ 这一档压根不是 accent 家的淡染，而是 brand/accent 拆档（任务 #24）后留下的**旧青色残值**；同文件「:90」的 `--brand-50: rgba(99,102,241,.12)` 才是 `#6366f1` 这一族的淡染（`--brand` 自己被抬成了 `#93a3fd`）。**后果**：任何写 `background: var(--accent-50)` 的地方拿到的都是青色底。
   - **sky 皮＝文字档本身太亮**：`--accent: #06b6d4` 配 `--accent-50: #ecfeff` ⇒ 2.33；同皮的同族对照 `--brand: #0369a1` 压 `--brand-50: #f0f9ff` = **5.57 过线** ⇒ 浅色皮要的是"一档更深的文字档"（brand 已有该形状），属 #27 色板裁决而非改值白拿。
   - ⇒ 默认皮这一条**是否要点头**待裁：把 `--accent-50` 换回本家靛淡染是**值修正**（该档现值与族名不符，属"名不副实"缺陷），但它会改变所有 `--accent-50` 消费点的观感（青色→靛色），与 #24 的收口结论纠缠 ⇒ 本轮**未动**。
3. **只有 AA 档（`--on-x` 压 `--x-fill`）与基底无关**：两色皆不透明 ⇒ A、B 下同一比值（`--on-success`/`--success-fill` 均 5.48，控制 C5）。这才是「底不可控的界面必须走 AA 档」的硬理由，不是观感偏好。
4. **§11.8 第 ③ 条建议的那条静态判据不该按那个形状入库**：静态扫「同一规则里 `color: var(--x)` 且 `background: var(--x-50|-x-dim)`」当前命中 **53 处**，而按基底 A 真属缺陷的只有 **1 处**（「src/components/ai/PhasePipeline.vue:484」的 `--accent`）⇒ **52 处误报**，入库只会造一个没人信的闸门。且上一版口径里的 67 处也偏高：其中 10 处实为 `border-color: var(--accent)` 配 `background: var(--accent-dim)`，把**描边**当成了**文字**（判据改用 `(?<![-])color:` 后 67→53）。这条判据要成立，得先把「这层的底到底谁负责」编进去，静态层做不到 ⇒ 撤。

**`ProjectChip.vue:117` 的不可读是真的，但根因换了**：不是「淡染档与文字档不成对」，而是**该组件自己写死了浅色底**（`:67` 那条 `linear-gradient(180deg,#ffffff,#f8fafc)`，账上 2 处）却让徽标去吃为暗色皮设计的令牌 ⇒ 令牌系统对整个 chip 是盲的。所以第一刀落点变成 **`:67` 的表面档迁移**（属 #17「无需点名即可迁」：surface 阶梯 `--bg-card`/`--bg-raised` 四皮齐备），迁完 chip 的底色跟皮之后，`:116/:117/:118` 三行才谈得上成对迁移。**本轮仍未动任何源文件。**

**探针自身也记一笔**：这份矩阵第一次跑出的输出**格式完整、内部自洽**，却把 `--success-50` 印成了 `#dcfce7`（该串在 `src/` 下 0 次命中）、把两列 FAIL 计数印成 0 而同一行仍带 `**` 标记 —— 是靠「grep 那个 hex 在不在盘上」这一枚独立口径抓回来的，不是靠输出可读性。**推论：只读探针的每个关键单元格要有至少一格能用第二种工具（grep/stat）复核，否则"看起来对"的表比没有表更危险。**

### 11.9 「无需点名即可迁」的余量终于量出来了：**22 处**，不是 556，也不是 178

§11.7 结尾把这件事记成**未做**（"要把 `BASELINE` 逐文件跑一遍'同族可迁令牌在不在'才知道"）。本轮把它做完了，口径是**三层收窄**，每层都带控制项：

| 收窄层 | 判据 | 剩多少 |
|---|---|---|
| 全部账上位点 | 闸门 `--check` 口径 | **967 处 / 80 文件** |
| ① 同值令牌存在 | `src/**` 里某 `--x: #hex` 与之逐字相等 | 556 处（其中 `<style>` 区 **178**） |
| ② 迁过去**不改变默认皮渲染**且**不引入文字/底错配** | 逐位点定位其所在 CSS 规则，看**对手属性**（字看底、底看字）是什么 | **22 处** |

第②层的分桶（178 处候选全分配，`sum==cand: True`）：

| 桶 | 处 | 属不属于"无需点名" |
|---|---|---|
| `2-single-decor`（只有 border/box-shadow/fill/stroke 等装饰属性，无文字/底配对） | 7 | **属** |
| `5-pair-both-value-preserving`（字与底都是裸 hex，且**两边各有同值令牌** ⇒ 成对迁完默认皮逐字不变） | 14 | **属**（但要**成对**动，单动一边即造出"深字压深底"） |
| `7-partner-tokenized-solid`（一边已跟皮、另一边裸 hex，对手不透明） | 1 | **属** |
| `3-partner-absent->bg` / `->fg`（本规则里没有对手属性 ⇒ 底/字来自**父级或皮肤默认**） | 66 + 31 | **不属**，但也不需要裁决 —— 需要的是**查清谁供给底**（调查，不是点头） |
| `6-pair-partner-has-no-token`（成对但对手连同值令牌都没有） | 31 | 不属（缺档 ⇒ #27 案） |
| `4-partner-wash-token`（对手是 `--x-50/--x-dim` 淡染档） | 4 | 不属 ⇒ 卡 #32（就是 §11.8 那一族） |
| `8-partner-other` / `9-other`（渐变多停点、`@keyframes`、规则内自定义属性等未被上述形态覆盖） | 10 + 14 | 不属（形态待分类，先不吹成余量） |

**分类器的语义是被真值验过的，不是靠它自己说话**：拿 `ProjectChip.vue` 逐行对（该行内容本轮已用 Read 亲验），9 处位点全部落进与手工判断一致的桶 ——
`:75 #6366f1`（hover 描边）→ `single-decor` ✓；`:84/:90/:108`（文字色，底在父级）→ `partner-absent->bg` ✓；`:116 #4338ca`（底是 `var(--accent-dim)`）→ `wash-token` ✓（正是 §11.8 判出的那一族）；`:118 #fff7ed`+`#c2410c`（同规则一对裸 hex，两边都有同值令牌）→ `5-pair-both-value-preserving` ✓（"必须连底一起迁"在这条上成立且可机械成立）；`:67` 那两枚白渐变 → `partner-absent->fg` ✓（它自己是底，本规则里没有字）。另附两枚具名变异体：淡染档必被识别为 wash、实心 `var(--bg-card)` 必不被识别为 wash，均 True。

⇒ **诚实口径**：#17 在**没有任何新裁决**的前提下还能机械拿下的，是这 **22 处 = 存量的 2.3%**；其余 945 处分别卡在「查底」（97）、「缺档/身份色」（31 + `<script>` 那 119 + template/js 那 259）、「#32 淡染档」（4）、「形态未分类」（24）与「压根没有同值令牌」（411）。**本轮未动任何源文件，闸门账仍 967 处 / 80 文件、grown=0、verdict=PASS。**

**下一批（点名即开），22 处的逐文件实账**（与桶和为 22 一致）："src/components/MessageBubble.vue" 9（6 成对 + 3 装饰）、"ProjectChip.vue" 3（2 成对 + 1 装饰）、"MarketView.vue" 2、"FlowDetailDialog.vue" 2、"AdminDocs.vue" 2、"PhasePipeline.vue" 2（皆装饰）、"ExpertCard.vue" 1、"AdminHitl.vue" 1（唯一一处"对手已跟皮"）。`5-pair` 那 14 处**分散在 8 条规则**里，最挤的一条也只有 3 处（MessageBubble `:1725`）⇒ **不是"一条渐变吃掉大半"，是真有 8 个成对点**。

**顺手记一次自己抓自己**：本节先用的是另一版分桶（把 `border/box-shadow/渐变` 等一律记作 `single-*` 免裁），那版给出"免裁 22 处"并点名 "BotCenterView.vue 6 处 / GateResult.vue 3 处"为最干净的首场单元。**该点名是错的** —— 收窄判据后那两个文件在 22 处里**一处都没有**（它们的位点全落进 `9-other`/`3-partner-absent`）。两版都恰好报 22，纯属数值巧合，不构成互证。**教训**：同一份数据换判据后必须重算**逐文件明细**再点名，总数相同不代表内容相同；已按第一版口径写进正文的推荐单元就地作废，以上表为准。

验收不变：**闸门必须报 SHRANK，且还原原地后必须被具名变异体打红**，二者缺一就不算数。**本轮未动任何源文件。**

### 11.10 上节的「22 处」作废：把候选令牌收窄到**外壳命名空间**后，可机械迁的是 **0 处**

§11.9 那一条 967 → 556 → 178 → 22 的收窄链，第一层就用错了候选源。它的"同值令牌"来自对 `src/**` 全量 `--x: #hex` 的正则表（`D:/tmp/b11_census.py` 那版），**把组件自己 `<style>` 里定义的自定义属性与 JS 色表里的键也算作令牌**。那类名字不是可迁目标：换肤只覆写 `src/styles/global.css` + `themes/*.css`，组件内定义的 `--foo` 既不参与四皮覆写、scoped 下也不外溢。

本轮改用**闸门自己的权威表**重算：`check-view-hex.py` 的 `skin_token_table('default')` + `resolve_token()`（判据 2/3/6 读的就是这张，跟完 `var()` 别名链）。探针 `D:/tmp/b18_shell.py`，关键值另用 grep 第二口径复核：

| 层 | 数 | 口径 |
|---|---|---|
| 裸 hex 位点 | 967 / 80 文件 | 与 `--check` 同值（同一 `scan_sources`） |
| 落在 `<style>` 段 | 419 | 其余 **548** 在 `<script>`/template 数据里，归「JS 侧色值的唯一出口」那条判据管，同值与否都不构成可迁 |
| `<style>` 且**外壳表有同值令牌** | **102 = 419 的 24.3%** | §11.9 的 178 与它不同源（宽候选 + 3 位缩写未建键），不可复用 |
| 其中**候选唯一** | **4** | 逐个验过，4 处全是陷阱 |
| 其中候选 ≥2 | 98 | `#6366f1` 一支就有 `--accent`/`--brand-500`/`--cat-1`/`--el-color-primary-dark-2` 四个同值 |

⇒ **结论换向：「同值」从不指定令牌，指定令牌的是家族裁决。** 而家族恰恰在"默认皮同值、其它皮分家"处不可代换：`global.css:100 --brand-fill` 与 `:107 --accent-fill` 都是 `#4f46e5`，但文字档从默认皮就不同（`:26 --accent: #6366f1` / `:86 --brand: #93a3fd`），另三皮彻底分家 —— cyberpunk `--accent #b14aff` vs `--brand #00d4ff`、dark `#22d3ee` vs `#818cf8`、sky `#06b6d4` vs `#0369a1`（`themes/*.css:9/:11/:52/:53/:56` 均 grep 实测）。**在 `#6366f1` 上选 `--accent` 还是 `--brand`，是在 3/4 个皮下改观感**，属 #27。

那 4 处"唯一候选"逐条（原行本轮 Read 亲验）：

- `MarketView.vue:871  color: #dc2626` ⇒ 唯一同值是 `--danger-fill`（`global.css:41`，**色块底档**）。写进 `color:` 正是闸门 docstring 记录的那次历史误伤（`color: #047857` → `var(--success-fill)`）⇒ 迁它等于新造一条 ROLE `fill-as-text`。**它是反例，不是余量。**
- `AgentTaskRunner.vue:118  background: linear-gradient(135deg, #ef4444, #dc2626)` ⇒ 只有第二停点有同值档，`#ef4444` 外壳无档 ⇒ 迁半条渐变，换肤后两停点深浅会翻面。
- `MessageBubble.vue:1505  #c7d2fe → --el-color-primary-light-5` ⇒ EP **影子名**（"两套命名空间互不相交"病里那 30 个）。
- `MessageBubble.vue:1725  #8b5cf6 → --cat-5` ⇒ 图表分类档，落点是主行动按钮渐变。

唯一"契约指定家族"的一支 —— 色块底上的白字 `--on-x`（`<style>` 段 65 处：MessageBubble 14、AgentTaskRunner 5、ExpertConfigView 5、Dashboard 5…，散在 27 文件） —— **也不是无损迁**：13 个家族里**没有一族在四皮下取同一值**（探针逐族跟链），已 grep 坐实的一例是 `--on-danger`：默认/dark/sky 三皮 `#ffffff`，cyberpunk `themes/theme-cyberpunk.css:68 → #001a22`，注释写明理由「霓虹底压深字 4.92:1（白字压 #ff2d55 只有 3.65）」。而这些白字压的是**写死的**渐变底、底不跟肤 ⇒ 迁上去＝"只在赛博皮下把白字改深"，是要验收的观感改动。

**另一个独立的方法论缺口**：`<style>` 段 419 处里 **62 处是 3 位缩写**（`#fff` 58 + 其它 4），旧同值表按 6 位建键 ⇒ 这 62 处在旧口径下既不可能命中、也没被单独交代。

⇒ **#17 的余量口径改写为一句**：存量 967 处里，**在没有任何新裁决时可机械迁的位点 = 0**（不是 22）。剩余只有四条出路：① #27 家族/色板裁决（含 `--accent` vs `--brand` 谁是谁）；② 渐变令牌命名空间（恒深横幅不许迁现有档）；③ JS 侧 548 处按「底跟不跟肤」逐处判；④ 孤儿退役。

**④ 当场复算即空**（这段先写"推荐走 ④"、随后自己把它推翻）：`node scripts/gate/check-import-reach.mjs --json` 本轮实测 `seen 255 / universe 257 / unresolved 0`，孤儿只剩 **2 个** —— `modules/health-check.js`（138 行，在 `SCAN_EXCLUDE` 那一层，按定义不进账）与 `views/admin/panels/_smoke.js`（57 行），**两者的 `hex` 字段都是 0**。即"删文件即销账"这条路**已经没有账可销**：#26 那本"孤儿 31 个承载 413 处"的册子在那之后被接线/退役消化掉了，本轮不复算就引用它会得出一个错误的推荐单元（这个错在本节上一段就犯了）。

⇒ 因此 **#17 的诚实收束是：967 处存量全部卡在裁决侧，无一条机械路**。要么点名 ①②③ 之一开工，要么把这笔账按"设计如此（身份色不跟肤）+ 待裁决"归档，让棘轮只做"不许增长"的守卫。**本轮未动任何源文件**；闸门复跑 `--check`：现存 967 处 / 80 文件、基线 1004 / 81、grown=0、verdict=PASS、rc=0。§11.9 的表与「22 处」按本节作废，引用以本节为准。

**但本轮量到一处该先修的，恰好也是 22 处**（与 §11.9 那个作废的 22 无关，纯数值撞车）：`src/views/graph/GraphView.vue` 的 22 处**全在 `<style>` 段之外**：本轮拿闸门自己的 `hex_sites()` 逐处点名 ⇒ **8 处在 template 内联 `style=`**（`:218/:225/:232/:239` 四对徽标），**14 处在 `<script>` 色表**（分解见本节末段。上一版本句写作"全部落在 template 的内联 `style=`"，未逐处点名即断言，**作废**）—— 内联样式**能**解析 `var()`（见「JS 侧色值的唯一出口」），所以**那 8 处、且只有这 8 处**是全场唯一"机械通路敞开"的一批；而它们的值是**照抄 sky 皮三对档冻结下来的**（`themes/theme-sky.css:56 --accent: #06b6d4`、`:57 --accent-50: #ecfeff`）⇒ 底不跟肤、字不跟肤，四皮下同一值。按 WCAG 实算（sRGB 线性化，口径同 §11.8）：

| 冻结对（GraphView 内联） | 实测 | 判定 |
|---|---|---|
| `#06b6d4` 压 `#ecfeff`（accent 系） | **2.33** | FAIL |
| `#10b981` 压 `#ecfdf5`（success 系） | **2.41** | FAIL |
| `#d97706` 压 `#fef3c7`（warning 系） | **2.86** | FAIL |
| `#ec4899` 压 `#fce7f3`（pink/`--cat-6` 系，第四对，上一版漏算） | **3.00** | FAIL |
| 参考：`#4338ca` 压 `#e0e7ff`（深字压淡染） | 6.41 | AA |
| 参考：AA 档 `#0f1117` 压 `#06b6d4` | 7.77 | AA |

⇒ 三点：① 这是 §11.8「淡染档压不住文字档」同一族的**批量实例**，且因冻结成字面量而**在四皮同时不可读**（§11.8 那一条只在默认皮下发作，这一批更严重）；② 把它们迁成 `var(--accent-50)/var(--accent)` 是把"四皮 2.33"换成"三皮 ≥5、默认皮 2.69"（§11.8 实测）⇒ **换一处缺陷，不是修缺陷**；③ 要真修只能走 §11.8 的 A 案（连底迁到 `--x-fill` + `--on-x`，深底白字，AA 与基底无关）或 B 案（给淡染档补一档压得住的文字档＝#32）。

**代价与限制（本轮复算后改写）**：A 案会把**这 4 对（8 处）徽标**从"淡底深字"变成"深底白字"，属观感改动，要真机看过才算验收。另**"整文件清零"这条上一版也说满了**：GraphView 只有 8 处走内联样式，**其余 14 处在 `<script>` 色表里 ⇒ 本文件一次点名最多把 22 → 14**，要真清零必须先解 #27（色板裁决）。**本轮仍未动任何源文件。**

**那 14 处的逐处点名**（同一份 `hex_sites()` 输出，与上表 8 处相加 = 22，无重无漏）：

| 位点 | 值 | 谁在消费 |
|---|---|---|
| `:400`…`:407`（8 处） | `#6366f1 #8b5cf6 #10b981 #06b6d4 #f59e0b #3b82f6 #ef4444 #f97316` | `typeColors`：`:398` 先 `...NODE_TYPE_COLORS`（单源在 `src/constants/operator.constants.js:29`，经 `src/types.js` → `constants/index.js` 桶口再导出，**不是**断链，本轮已顺到定义处）后叠加这 8 个业务实体键。同一张表两头用：`:263 .legend-dot :style="{background: color}"`（DOM，能吃 `var()`）与 `:1368 .nodeColor(…)`（ForceGraph3D，**WebGL**） |
| `:416`（4 处） | `#10b981 #f59e0b #fbbf24 #94a3b8` | `STATUS_COLORS`：同样既画 `:270` 图例点，又喂 `:537` 画布节点色 |
| `:732` / `:1368`（2 处） | `#60a5fa` / `#64748b` | 骨架与 `nodeColor` 的**兜底值**，只走画布 |

⇒ 三个后果：① 画布侧不吃 CSS 变量（该文件 `:726` 自己就写着这条注释），所以这 14 处只能走 `tokenColor()` + `watch(themeRevision)` 那条既有机器（文件已在 `:355` import 了它），**不能**像那 8 处一样直接换 `var(...)`；② 它们是**系列/身份色**（八类节点 + 四类状态要在同一张图里互相分得开），正是 #27 那一题，与 `EXPERT_COLORS` 16 身份色 vs 8 档同构 ⇒ 换档即撞色，不是 substitution；③ **一张表两个消费者**（图例 DOM 点 + 画布节点）⇒ 底色跟不跟肤必须一次裁决两处同迁，拆开做会让图例与图形在换肤后指向不同颜色 —— 这正是「迁字色要连底一起迁」的画布版。**所以 GraphView 的诚实定位是：8 处等 §11.8 的 A/B 裁决（AA 缺陷，先修），14 处等 #27（色板裁决），二者都不是"无需点名即可迁"。**

### 11.11 上一段那把 AA 尺子对那 4 个徽标是错的（本轮实测推翻自己第三次），但因此量出一条真能机械收的 4 处

§11.10 末尾「8 处等 §11.8 的 A/B 裁决（AA 缺陷，先修）」是**本轮第三次自己推翻**，且推翻有据：那 4 个 `.qa-icon` 里放的是 **emoji**（🧩🛤️📊🔥），不是文字。彩色 emoji 由彩色字体绘制，`color:` 对其不起作用 ⇒ 我把「字压底 2.33–3.00」那张表挂在了一对**根本不会渲染到像素上的值**上。判据不是靠「我记得 emoji 不吃 color」，是量出来的（browser-use 的 Chromium，`fillText` 到 canvas 后逐字节比 `getImageData`）：

| 样本 | `color:#f00` vs `#0000ff` 的像素和 | 结论 |
|---|---|---|
| `A`（拉丁，正对照） | **不同**（334084 / 334782） | 探针通电，`color` 确实被读 |
| `☺`（文本呈现形态） | **不同**（582269 / 583773） | 单色回退honours color ⇒ 上表不是空判 |
| 🧩 📊 🔥 🛤️ 🎯（正是 GraphView 用到的五枚） | **逐字节相同** | `color:` 在此不产生像素 |

⇒ 三条修正：① 那 4 个比值算术没错但**尺子错** —— WCAG 1.4.3 管文字，非文字图形按 1.4.11 的 3:1 且量「图形 vs 邻接色」，不是这个 `color`；② 这 4 行真正的病是**底不跟肤**（写死的 sky 皮淡染档，换到 dark/cyberpunk 仍是亮块）**加一条假装有作用的死声明**；③ 既然 `color:` 无像素后果，**删它就是零观感变化的真收口** —— 本轮唯一「不需要任何新裁决」的机械路，已落地。

**本轮实做**（`src/views/graph/GraphView.vue`：四处 `style="background:#…;color:#…"` 只删 `;color:#…`；字节 68330 → 68274；仍 LF、无 CR；`vue/compiler-sfc` 报 parse errors 0、`compileScript` 通过 bindings 130 / style 段 1）：

- 闸门 `--check`：**现存 963 处 / 80 文件**（上轮 967，−4 与净改动相符），基线 1000 / 81，grown=0、new=0、palette/tier/surface/role 全 0、`verdict=PASS`、rc=0；`SHRANK GraphView 22 → 18` 已按约定回填 `BASELINE`（22 ⇒ 18，**不写 0**）。
- 具名变异体：把本轮改前的快照盖回原文件 ⇒ `GROWN src/views/graph/GraphView.vue 18 → 22`、`verdict=FAIL`、rc=1。**SHRANK 与「还原即打红」两条都在**，不是只报绿。
- 守卫自检：`--selftest` 仍 `PASS=58 FAIL=4`（那 4 条＝两条反咬型 + `admin-lowcode` 6 处 tripwire + PhasePipeline 故意不回填），**本轮未新增红**。
- 归因：`git diff --stat` 对该文件报 41+/31−，**不可归因于本轮** —— GraphView 带着批次 2–7 的既有未提交改动，本轮净改动只有那 4 行（用改前/改后快照对差分证明）。

**边界要说清**：像素等值是在**本机 Chromium + 彩色 emoji 字体**这条路径上量的；系统缺彩色字体时这些码位退成单色字模，那时 `color` 才又起作用 —— 那是**降级路径的观感**，不构成保留一条写死 sky 皮色值的理由。若将来要「图标色随皮」，正确做法是容器 `filter` 或改 SVG，而不是往 emoji 上写 `color`。

**余账（GraphView 18 处）**：4 处＝上述 chip 的**底**（病是「不跟肤」而非 AA，要迁必须先裁「图标底跟不跟肤」）；14 处＝`<script>` 色表 ⇒ #27。

**顺手把 A 案的可执行性量死**（为 #32/#27 的点名准备，未动源码）：`--cat-n-fill` + `--on-cat-n` 在**四皮 × cat2/3/4/6** 共 16 组全部存在，逐组实算 **≥4.5**（最低 4.60＝sky cat6，次低 4.71＝sky cat3；最高 15.41）⇒ 对**真文字**位点，A 案不需要新增令牌、不等 #27，机器已在库里；卡在裁决侧的只是「这块底要不要跟肤 / 观感是否接受深底白字」。**本轮改动：源文件 1 个（4 行）+ 闸门基线 1 行 + 本报告，均未提交。**

### 11.12 「emoji 上的死 `color`」一类**已全域清零**；余下 9 处内联色值全是**活色**（普查，以本轮改前快照做正对照；零源文件改动）

§11.11 那条机械路能不能在别处再走一次？做了一次全 `src` 普查。判据形态：内联 `style="…color:#hex…"` 的元素**内文只有 emoji、不含任何 `[A-Za-z0-9一-鿿]`** ⇒ `color` 无像素后果 ⇒ 可零裁决删除。

| 口径 | 结果 |
|---|---|
| **正对照**：本轮改前的 GraphView 快照 | 命中 **4**（`:218 🧩`、`:225 🛤️`、`:232 📊`、`:239 🔥`）—— 与 §11.11 删掉的四位点**逐一对上** |
| 改后 GraphView | 0 |
| **整个 `frontend-ui/src`（306 个 `.vue/.js/.ts` 全扫）** | **0** |

⇒ 判据在真值上打满过（4/4），此后再无同类位点 ⇒ **「零裁决、零观感变化」的删除路已经走完，余量 0**，#17 剩下的 963 处全在裁决侧（#27 家族/身份色、#32 淡染文字档、「底跟不跟肤」、渐变令牌命名空间、孤儿退役）。

**第二个问题更要紧**：内联 `style="…color:#hex…"` 这个**形状**总共还剩几处、有没有第二处能机械处理？全域枚举 = **9 处**，逐处打开看过后分三类，**没有一处是死声明**：

| 类 | 位点 | `color` 有像素后果？ | 该用哪把尺子 / 卡在哪条裁决 |
|---|---|---|---|
| 真文字配写死色 | `components/MessageBubble.vue:172`「覆盖」徽标 `#b45309` 压 `rgba(245,158,11,.12)`；`:185` 跳过原因 `#991b1b` | **有** | 1.4.3 文字 AA；**淡底配中调色**＝§11.8/#32 那一族（写死的 amber 淡染，四皮同值） |
| `<el-icon style="color:#hex">` 包 SVG 图标 | `components/NotificationCenter.vue:39` `#ef4444`、`:64` `#cbd5e1`；`components/ProjectPicker.vue:170` `#8b5cf6` | **有**（图标吃 `currentColor`） | 非文字图形 ⇒ **1.4.11 的 3:1**，量「图标 vs 邻接底」而非文字对比；且这三处**底跟不跟肤**未裁 |
| SVG 渐变 `stop-color` | `views/workspace/panels/GraphCanvasPanel.vue:77/78/81/82`（`#6366f1`/`#06b6d4` 各配 opacity `.35`/`0`） | **有** | 属**渐变令牌命名空间**那一题（§11.10 与批次 6：渐变档与文字/填充档不平行，不许迁现有档） |

⇒ 用闸门自己的 `hex_sites()` 复算，这 9 处**都已挂在各自 `BASELINE` 里**（MessageBubble 154/154、NotificationCenter 8/8、ProjectPicker 13/13、GraphCanvasPanel 8/8、GraphView 18/18，实测值与基线逐项相等）⇒ 内联形状里**既无未入账的隐藏余量，也无第二处零裁决可删**。**本节未动任何源文件。**

**三条方法坑（本轮实撞）**：

1. **码位段判据会把汉字一起吃进去**：v1 用 `ord(c) > 0x2100` 判「非拉丁」，中文全被算成 emoji ⇒ 假阳 1 条，正是上表第一类的 `MessageBubble.vue:172`「覆盖」配 `#b45309`（**真文字**，AA 尺子对它有效，删不得）。修法：用正向判据「内文不含 `[A-Za-z0-9一-鿿]`」，不要反过来用「含高位码位」。
2. **Git-Bash 的 `/tmp` 对 Windows 侧 Python 不可见**：首版对照从 `/tmp/b21_pre.vue` 读快照 ⇒ `FileNotFoundError`，对照**根本没跑**；若当时只看「whole src = 0」就会把一次无效普查当成「已清零」。快照挪到 `D:/tmp/` 重跑才拿到 4/4。**对照必须与主判据同解释器、同路径域跑通并打印命中数**。
3. **「0 命中」只有配上界才成立**：窄判据报 0，只说明「这个形状没了」，不说明「这类账清了」。所以补了一次宽判据（凡内联 `color:#hex` 全枚举＝9 处）并把 9 处**逐处打开分类**——上表就是这么来的；否则一句「emoji 死色已清零」会掩盖「另有 9 处内联写死色，其中 4 处是 SVG 渐变的 `stop-color`、3 处是 `<el-icon>` 的图标色，只有 2 处是文字」。顺带一条：宽判据按 `style="…"` 整段匹配，所以 `stop-color:#hex` 也算命中（**`color:` 的子串**）——这 4 处不是假阳，它们确实是内联写死色，只是**不吃文字的尺子**。

### 11.13 硬引用侧的账重量：缺 import 的 EP 反馈 API 实为 **29 文件 / 446 处**（旧口径 33/458 作废；普查只读，零源文件改动）

#17 的「硬引用」半边一直挂着一条旧数（`33 文件 / 458 处`，源头 `src/api/http.js:157`）。本轮按 §11.10 的同一规矩**先复算再引用**，结果两条：数变了，且**旧数的判据本身有系统性假阳**。

**判据 v1（41/478）被自己的漏认打回**：用「`import { … } from 'element-plus…'` 的**第一条**子句」判定"该文件已导入" ⇒ 凡是把 `ElMessage` 与 `ElMessageBox` 拆成**两行** subpath import 的文件，第二行的人名就被判成缺失。实测 5 枚假阳：`main.js`（`:3` `ElMessage`、`:4` `ElLoading`，两个都在）、`composables/useKnowledgeBase.js`、`modules/admin-lowcode/composables/useCrudPage.js`、`modules/admin-lowcode/pages/tenant.page.js`、`modules/admin-lowcode/pages/access.page.js`（后四个都是 `:11/:12` 或 `:7/:8` 两行式）。⇒ 判据改成 `finditer` **全部** import 子句，并额外认得**动态解构形式** `const { ElMessage } = await import('element-plus')`（`utils/message.utils.js:154/173`，v1/v2 都差点把它算成缺陷）。

**第二条**：`.test.js` 里的 `ElMessage: vi.fn()` 是 `vi.mock` 的**对象键**不是使用位点 ⇒ 测试文件整体剔出缺陷账（本轮实测：非测试文件之外 0 条误入）。

**判据 v2（采用口径）**：**29 个非测试文件 / 446 处**调用位点，按处数降序的头五名：`views/expert/ExpertConfigView.vue`（45+4）、`views/admin/panels/AdminMonitor.vue`（27+1）、`views/project/ProjectsView.vue`（26+1）、`views/workspace/ExpertWorkspaceView.vue`（25+1）、`views/admin/panels/AdminRole.vue`（24+1）；最小的是 `views/admin/panels/AdminStorage.vue`（2+1）。

**七条对照全过**（判据没通电就不配报数）：6 枚"必须**不**被 flag"＝上面那 5 个双行 import 文件 + `utils/message.utils.js`（全部 clean）；1 枚"必须**被** flag"＝`api/http.js`（只 import 了 `getToken`，`:157` 起裸用 `ElMessage`）⇒ flagged。

**"这确实是运行故障"的三个前提也各自复验**：① `vite.config.js` 无 auto-import（plugins 只有 `vue()` 与 `Components({resolvers:[epSubpathResolver()…]})`，后者只管**组件**不管 `ElMessage` 这类函数式 API）；② 全仓 `globalThis.ElMessage =` / `window.ElMessage` **只出现在三个测试文件**（`AdminRole.smoke.test.js:19-20`、`AdminUser.smoke.test.js:26-27`、`admin/panels/_smoke.js:54-55`）⇒ 生产侧没有兜底全局；③ 那两处 `_smoke.js` 桩正是**掩盖**——它把"面板一调消息就 ReferenceError"抹成 smoke 绿灯（旧结论仍成立，本轮未重跑真机）。

⇒ 修法是**零设计裁决**的机械改动（每文件顶部补 1–2 行 `element-plus/es/components/{message,message-box}/index` 的 subpath import，与 `useKnowledgeBase.js:11-12` 同款），但要一次动 29 个源文件，且与「删 `_smoke.js` 桩」是同一件事的两半（只补 import 不撤桩＝掩盖仍在）⇒ **等点名再做**。

**引用纪律**：旧口径 `33 文件 / 458 处`（含本报告早期版本与记忆条目）**自本节起作废**，以 **29 / 446** 为准；两数之差不是"修好了三处"，而是**判据假阳**被拆掉。普查脚本已**落库为常驻探针**：`frontend-ui/scripts/gate/check-ep-feedback-imports.py`（未入库＝尚未 `git add`，该目录整体是 `??`），复现命令 `python frontend-ui/scripts/gate/check-ep-feedback-imports.py --check`。

**探针不是棘轮**（刻意设计，避免重演 hex 闸门那两条"债清得越好越红"的反咬型失败）：债务多寡**不影响 rc**，只有「判据自身失效」才打红 —— `--check` 钉 7 条真实文件对照（6 枚必须 clean ＋ `api/http.js` 必须 flagged），`--selftest` 钉 4 枚合成变异针。落库后实跑：`--check` rc=0 且**逐字复现本节账**（29 个非测试文件 / 446 处；`ExpertConfigView.vue` 45+4 居首、`api/http.js` 6@L157、`AdminStorage.vue` 2+1 最小），`--selftest` rc=0 `PASS (4/4)`。

**探针自己也要被变异**（4 枚变异体全部把 `--selftest` 打红，缺一即视为空转）：

| 变异体 | 打红的对照 |
| --- | --- |
| A `import` 只认第一条子句（＝ v1 假阳根因） | `two-line subpath imports -> clean` 报出 `['ElMessageBox']` |
| B 撤掉「`vi.mock` 行 ＋ 对象键位点」双豁免 | `vi.mock object key -> not a use site` 报出 `['ElMessage']` |
| C 不再识别动态解构 import | `dynamic destructure import -> clean` 报出 `['ElMessage']` |
| D 把"判缺"那半边短路（探针失明） | `missing import -> flagged` 报 `[]`（该报的不报） |

驱动额外要求 stdout 必须含 `SELFTEST:` 判决行 —— 崩溃/报错冒充打红一律记 INVALID，不计证据。

### 11.14 「硬引用」不外溢：vue / vue-router / pinia 一族实测 **非测试源文件 0 处用而未绑**（普查只读，零源文件改动；闸门落库并被 6 枚变异体打红）

**要回答的问题**：§11.13 那批缺陷的成因是「无 auto-import ⇒ 脚本里的名字是普通标识符」，这条成因**并不偏爱 element-plus** —— `ref / computed / watch / useRouter / defineStore / storeToRefs` 同理。所以「硬引用半边是否只有 29/446」必须先量再答。普查面：vue 33 名 + vue-router 9 名 + pinia 8 名（`FAMILIES` 逐名列举，非通配）。

**第一版把判据说成 167 文件 / 2679 处 —— 是判据自己的三处假阳**：① `import { computed } from 'vue'` 的**子句内部**被当成使用位点（每个正常文件都中）；② 通用名 `version` 撞上数据字段；③ 单字母 `h` 撞上形参。修法分别是「按 span 排除 import 语句」「剔出 `version`」「`h` 要求 `h(` 形态，后又整名剔除」。归一后剩 **2 个测试文件 / 4 处**：`modules/expert-alliance/contract/contract.test.js`（`computed` 1@L1146、`watch` 1@L1396）、`contract/orchestration.test.js`（`computed` 2@L510）—— 逐行打印复核，三处全是 `expect(VIEW).toMatch(/const reopenHint = computed(…/)` 型**正则字面量**（判据不遮蔽 regex ⇒ 假阳）。**非测试源文件：0 处。**

⇒ 「硬引用半边」的账**就是** §11.13 的 29 文件 / 446 处，没有第二族。这是本轮唯一的负结果，因此它的**举证责任**与正结果一样重：探针不能是盲的。

**落库为常驻闸门**：`frontend-ui/scripts/gate/check-framework-imports.py`（与 EP 探针同目录，同样**尚未 `git add`**——**此句已作废，见 §11.17**）。它**既不是棘轮也不是容忍型探针**，而是**零容忍不变量**——这一族缺 import 会让页面首屏即崩，真语料干净是应然而非偶然，所以「非测试源文件 > 0 处」就直接 rc=1；测试文件命中只打印作参考、永不改 rc（连同上面那条正则字面量边界一起写进了输出文案）。

**实跑（本轮，只读）**：
- `--check` rc=0 → 「非测试源文件「用而未绑」：0 个 / 0 处（判据＝必须为 0）」＋「测试文件命中…2 个 / 4 处」。
- `--selftest` rc=0 → `FRAMEWORK-IMPORTS SELFTEST: PASS (11 合成针 + 4 种缺陷对照)`，15 条全 `[OK]`。
- **种缺陷对照**（就地抹掉 `from 'vue' | 'vue-router' | 'pinia'` 整行，改动只在内存）：`views/graph/GraphView.vue` → 9 名（`computed, markRaw, nextTick, onBeforeUnmount, onMounted, reactive, ref, useRouter, watch`）、`views/project/ProjectsView.vue` → 5 名、`stores/alliance.store.js` → 3 名（含 `defineStore`）、`composables/useKnowledgeBase.js` → 3 名；四个文件原样一律 clean。⇒ 0 处不是探针失明换来的。

**变异体（6 枚全部把 `--selftest` 打红，缺一即视为空转）**：

| 变异体 | 打红的对照 |
| --- | --- |
| A 撤掉 import 子句的位置豁免 | `跨行花括号 import ⇒ clean，alias 绑定末端名` 报出 `['ref']`（alias 的左端名被当使用位点） |
| B 撤掉「对象键不算」的前瞻 | `对象键不算` 报出 `['computed', 'ref']` |
| C 不再排除 `.vue` 的 template/style 段 | `template 段不参与` 报出 `['computed']` |
| D 不再遮蔽注释与字符串 | `字符串/注释不算` 报出 `['computed', 'useRouter', 'watch']` |
| E 把"判缺"那半边短路（探针失明） | 四枚种缺陷对照全报 `CLEAN(探针失明!)` |
| F 不再识别解构形参 | `形参同名 ⇒ clean` 报出 `['storeToRefs']` |

**本节最有价值的一行：B 与 D 第一次跑**打不红 **，把 6 枚合成针的空转抓了出来**。那些夹具是纯脚本片段、没有 `<script>` 段，而判据对 `.vue` 先做 script 掩码 ⇒ 整段被抹空，`analyze()` 恒返回 `{}`，"必须 clean"类针全部**因为什么都没看见而通过**。修法是每枚针自带文件名（纯脚本片段用 `T.js`，`.vue` 专属的用 `T.vue`）；**修完立刻红出一处真缺口**：`params()` 只按逗号切形参表，认不得解构形参 `{ storeToRefs }`（`{` 被当名字、真名漏绑）。⇒ 「全绿不算证据，断言必须被变异体打破才算数」这条纪律在本节第三次自证：这次不是源文件有问题，是**我的夹具**在撒谎。

**边界（登记，不动手）**：两把探针都**没接进 CI** —— `.github/workflows/ci.yml` 只跑根 `scripts/gate/verify-ports.py` 与 `scripts/gate/check-frontend-module.py`，`frontend-ui/scripts/gate/*` 整目录不在门禁列表里；接入要改 ci.yml，与「`scripts/gate/` 是否 `git add`」是同一批待点头事项。

**待点头清单不变**：补 29 个文件的 EP import（`api/http.js:157` 优先）**并**撤 `_smoke.js:54-55` 的 `globalThis` 桩；本轮不属清单的零改动。

### 11.14续 §11.14 的前提补测，以及它撞出的**闸门自身缺陷**（2026-09-26 同日追加）

§11.14 写"硬引用半边＝29/446"时，隐含一个没量过的前提：**除 EP 函数式 API 与 vue/vue-router/pinia 外，没有第三族"裸全局"**。本轮把前提补上，方式是借 §11.14 那台已验过的引擎（`check-framework-imports.py` 以模块方式加载，只改内存里的候选名单，**不写任何源文件**），换成 46 个第三方库候选名（`echarts / mermaid / THREE / axios / MarkdownIt / Vex / ForceGraph3D / 16 个 @vueuse 组合式 / z / 13 个 EP 图标名`）重跑同一判据。

**初测：非测试文件 5 个 / 6 处命中，逐处手工裁决后真缺陷 0 处** —— 但其中 3 处暴露了闸门自己的洞：

| 命中 | 裁决 |
| --- | --- |
| `api/http.js` `axios`@L11×2 | **闸门假阳**：`:2` 就写着 `import axios from 'axios'` |
| `utils/markdown.js` `MarkdownIt`@L6 | **闸门假阳**：`:3` `import MarkdownIt from 'markdown-it'` |
| `components/MessageBubble.vue` `MarkdownIt`@L772 | **闸门假阳**：`:465-467` 三行默认导入（MarkdownIt / anchor / taskLists）都在 |
| `components/layout/TheSidebar.vue` `z`@L74、`GraphTeamPanel.vue` `z`@L91 | 正则字符类字面量（`/^[A-Z][A-Za-z]+$/`、`/_([a-z])/g`）里的 `z` ⇒ 已知"regex 字面量不遮蔽"限制，非缺陷（8 个测试文件命中同因） |

**根因（一条正则的形状）**：`DEFAULT_RE` 写成 `...(?P<ns>[\w$]*)\s*(?:,|$)`，即默认导入的名字后面必须是逗号或行尾才算绑定；真语料里 `import axios from 'axios'` 后面跟的是 ` from` ⇒ **默认导入整族不绑定**。修：`(?:\s*(?:,|from\b|$))`。方向要说清 —— 这是**假阳**（把干净文件判缺陷），不是假阴，所以不会掩盖任何真 ReferenceError；但 §11.14 那台闸门钉的是**零容忍不变量**（非测试源文件必须 0 处），假阳等于把干净的仓判 FAIL，代价与假阴同级。

**为什么 §11.14 的全绿没抓到它**：vue/vue-router/pinia 三族的具名导出**没有一个是默认导入**，被问到的那个形状在这份语料里不存在 ⇒ 闸门"跑通"只证明它通电，不证明它对别的形状也成立。这是"负结果也要有覆盖面"的形态：换一族语料＝给闸门免费加一批夹具。

**证据链（先钉针、后修、再变异，顺序不许倒）**：
1. 先加第 12 枚合成针 `default import 必须绑定该名字`（夹具形状取自 `api/http.js:2`）⇒ 未修时 `--selftest` rc=1、`[FAIL] default import 必须绑定该名字 -> ['markRaw'] (expect [])`；
2. 改 `DEFAULT_RE` ⇒ `--selftest` rc=0 `PASS (12 合成针 + 4 种缺陷对照)`，`--check` 仍 `0 个 / 0 处`、`verdict=PASS`（判据没被放宽）；
3. 复跑第三方普查 ⇒ 6 处降到 **2 处**，且两处都是上面那个 `z` 的正则字面量（同一列已登记为已知限制）；
4. 变异体 **G**＝把 `DEFAULT_RE` 还原成修复前的形状 ⇒ 必须把第 12 枚针打红。连同旧 6 枚：**`FRAMEWORK-IMPORTS MUTANTS: PASS (7 枚变异体都必须把 --selftest 打红)`**，rc=0，`_mutant_framework_imports.py` 无残留（`grep -c` = 0）。
5. 回归：EP 那把未改动，`--check` 仍 **29 个 / 446 处**、`--selftest` `PASS (4/4)`；本文件字节卫生 12608 B / CR=0 / 无 BOM / ctrl=0 / `ast.parse` OK。

⇒ **§11.14 的结论不变、账不变（29/446、框架族 0 处）**，但"只有这两族"这句从此**是量出来的而不是假设的**。待点头清单与本节初版一致，未增不减。

**复现性口径（诚实登记）**：本节那 46 个候选名的普查脚本**没有落库**，是本轮的一次性只读驱动 `D:/tmp/b27_third_party.py`（它 `SourceFileLoader` 加载 `check-framework-imports.py`、替换模块级 `ALL_NAMES`、只走 `analyze()`）。⇒ 临时目录清空后，重算须按本节文字重建候选表；若要长期钉住这条前提，得先给它配变异体（否则就是一台没牙的尺子）——**不要**因为"它跑出 0"就直接把它当闸门接进 CI。

### 11.15 把"regex 字面量会假阳"从**已知限制**做成**能力**：第三方 46 名进零容忍名单的前提落地（普查只读，零源文件改动）

§11.14续 收尾时留了一句诚实登记：那两处 `z` 假阳属"已知『regex 字面量不遮蔽』限制"。本节把该限制补掉，从而让第三方裸全局名可以进**零容忍**判据（而非只当参考计数）——顺序仍是：先加针看它红 → 改判据 → 把旧形状做成变异体。

**改了什么（全在 `frontend-ui/scripts/gate/check-framework-imports.py`，仍只读源文件）**：
1. `FAMILIES` 增 `'third-party'` 一族 46 名（axios / echarts / mermaid / THREE / MarkdownIt / VexFlow 系列 / ForceGraph* / `z` / @vueuse 组合式 / EP 图标名），`ALL_NAMES` 自动并表；
2. 新增正则字面量遮蔽：`REGEX_OK_AFTER`（前一个非空白字符判定）+ `REGEX_OK_KEYWORDS`（`return` / `case` / `in` 等后面允许跟正则）+ `regex_starts()` + `regex_end()`（认 `[...]` 字符类与 `\` 转义、吞尾部 flags、**只在同一行内**，找不到闭 `/` 就放弃遮蔽而非吞到行尾）；遮蔽走同一套长度保持 `blank()` ⇒ 行号仍可映射回原文；
3. 种缺陷对照 4 → **6**：`CONTROL_FILES` 改为 `(相对路径, 模块正则)` 元组，`strip_framework_imports` 参数化为 `strip_imports_of(src, mod)`，新增 `api/http.js`（抹 `axios`）与 `utils/markdown.js`（抹 `markdown-it`）两条**默认导入形状**的对照 ⇒ 兼作 §11.14续 那处修复的长期回归护栏；
4. 合成针 12 → **15**：`正则字符类里的 z 不算使用位点`（豁免必须豁免得住）、`正则体被遮蔽但其后代码仍要看`（豁免不许顺手失明）、`除号不是正则：右边的裸名仍要判缺`（**必须仍是缺陷**的正对照）。

**除法那枚针的夹具被刻意加宽过**：初版写成 `total / z`（单个 `/`），实测"任何 `/` 都当正则开头"这种放宽在该夹具与真语料上**都抓不住**（`regex_end` 撞行即放弃），于是改成 `total / z / scale` —— 两个 `/` 才让"放宽＝吞代码"这条路必须被同形针显形。教训：**豁免类判据的配套正对照，夹具必须能区分"保守"与"吞代码"两个方向**，否则放宽到最坏值仍全绿。

**遮蔽在真语料上值多少（只读对比，`D:/tmp/b29_regex_mask_check.py`，跑完删副本）**：

| 变体 | 非测试源文件（判据＝必须 0） | 测试文件命中 | rc |
| --- | --- | --- | --- |
| base（现状） | 0 个 / 0 处 | 0 个 / 0 处 | 0 |
| 撤掉正则遮蔽 | **2 个 / 2 处**：`components/layout/TheSidebar.vue` `z@L74`、`modules/expert-alliance/components/GraphTeamPanel.vue` `z@L91` | 9 个 / 12 处 | **1** |
| 任何 `/` 都算正则开头 | 0 个 / 0 处（与 base 分不出来） | 0 个 / 0 处 | 0 |

⇒ 前两行说明遮蔽不是"顺手清洁"：**没有它，把 46 名并进零容忍名单会立刻把干净仓判 FAIL，而且被判的是两个活文件**（不是测试文件）。第三行说明另一半：语料对"吞代码型放宽"是**盲**的，只有第 15 枚针能抓 ⇒ 该针是这条通道唯一的牙。

**证据链**：
1. 先加 3 枚正则针 + 2 条默认导入对照 ⇒ 中途一次自伤要登记：控制环改写后 `print` 少传一个参数，`--selftest` 抛 `TypeError: not enough arguments for format string`，**stdout 无 `SELFTEST:` 判决行**，按本文件口径那是 INVALID 而不是通过（`--check` 同期仍 rc=0、0/0，判据未被放宽）；补齐实参后 `--selftest` rc=0 `PASS (15 合成针 + 6 种缺陷对照)`，6 条对照逐条 `原样=clean` → `抹掉 <mod> import 后=[非空]`；
2. 复跑 `--check`：非测试 **0 个 / 0 处**、测试 **0 个 / 0 处**、`verdict=PASS` ⇒ 加宽后的名单在真语料上干净；
3. 变异体 7 → **10**：H 撤正则遮蔽（红在第 13 枚针）、I 任何 `/` 都算正则开头（红在第 15 枚针）、J 对照不再按模块删 import（红在 `种缺陷对照 api/http.js -> ... =CLEAN(探针失明!)`）。连同旧 7 枚：**`FRAMEWORK-IMPORTS MUTANTS: PASS (10 枚变异体都必须把 --selftest 打红)`**，rc=0，本轮副本 `_mutant_framework_imports.py` 已删无残留（`scripts/gate/` 下另有两个**上一批遗留**的 `_mutant_role_judgments.py` / `_mutant_scan_set.py`，未动，`git add` 前须裁决）；
4. 回归：EP 那把未改，`--check` 仍 **29 个 / 446 处**、`--selftest` `PASS (4/4)`。

**边界更新（同日追加）**：本节证据**已落库为常驻驱动** `frontend-ui/scripts/gate/check-framework-imports-mutants.py`（默认模式＝10 枚具名变异体，`--variants`＝三档遮蔽变体真语料对比，锚点计数≠1 即报 INVALID、无 `SELFTEST:` 判决行即 INVALID）⇒ 临时目录清空后仍可重跑，"没牙"那句对此二者作废；至于 §11.14续 的 46 名候选表，它已并进闸门的 `FAMILIES`，普查即闸门本身，也不再是临时脚本。**落库首跑撞一处自伤要登记**：`--variants` 的变体表解包写错（`IndexError`，rc=1），修正后两模式 `MUTANTS: PASS (10 …)` / `VARIANTS: PASS (3 档遮蔽变体)`，且 `--variants` 复算出的三个数与上表逐项相符（base 0/0＋0/0；撤遮蔽 rc=1、2 个活文件、测试 9 个/12 处；放宽档与 base 同形⇒"语料盲"这句由驱动自己印出）。不变的部分：两把探针**仍未接进 CI**（`.github/workflows/ci.yml` 目前只有根 `scripts/gate/verify-ports.py` + `check-frontend-module.py`），`frontend-ui/scripts/gate/` 整目录仍是 `??`（**此句作废，见 §11.17**：10 个脚本已被并发执行者 `git add`）（`git add` 前须一并裁决上一批遗留的 `_mutant_role_judgments.py` / `_mutant_scan_set.py`）。硬引用侧的账仍是 **29 个 / 446 处**（EP 族）+ 框架/第三方族 **0 处**，只是"0 处"这句话现在覆盖三族而非两族，且带 15 枚针 + 10 枚变异体兜着。**待点头清单不变**（六条，见 §11.14 末），源文件一个都没改。

**"没改源文件"这句本轮是量出来的，不是声明的**：本轮唯一被写过的文件是 `frontend-ui/scripts/gate/check-framework-imports.py`（mtime `2026-09-27 02:21`）；`git status` 里 `api/http.js`、`components/layout/TheSidebar.vue` 都是 ` M`，但 mtime 同为 `2026-09-26 10:17`（早于本轮）⇒ 属**既有未提交批次**，不是本节动的手。顺带一条必须登记的口径：`api/http.js` 那份工作树改动里含 `-import { ElMessage } from 'element-plus'` ⇒ **29 个 / 446 处是按工作树量的，不是按 HEAD 量的**（探针 `os.walk` 读的是磁盘当前内容），引用这个数时别把它说成"相对最新提交"的账。

### 11.16 动态 import / require 形状：五枚"必须 clean"的针挂上通道，变异体 10 → 12

**起因是 §11.15 收工时留下的一个没牙的账**：五种绑定形状（动态 import 解构、动态 import 的 `.then` 解构形参、动态 import 赋给局部变量、`require` 解构、命名空间解构）在探针里**全部报 `clean`**。按本文件的口径（第 31、32 条），**期望空集的针只有在具名变异体把它打红时才算证据**——`clean` 有两种成因：「看见了且判为已绑」与「整段根本没看见」，绿灯自己分不出这两种。

**语料侧先量清（只读）**：`src/` 下 `await import(` 只有 **3 行**——`frontend-ui/src/utils/message.utils.js:154/173`（`const { ElMessage } = await import('element-plus')`，EP 那把探针早已认得这一形制）与 `frontend-ui/src/views/ai/Melody2ScoreView.vue:244`（`const mod = await import('vexflow')`，命名空间赋值）；`= require(` **0 行**。⇒ 对 vue / vue-router / pinia 三族与 46 个第三方名而言，这批形状**在当前语料里不存在**（第 32 条：报负结果必须一并写清"被问到的形状在这份语料里存不存在"，否则"80 个文件全绿"会被读成"所有形状都验过"）。

**裁决：不改判据，只补牙。** 这五种形状的 `clean` 是**正确的**——解构走"局部声明绑定"通道、`.then(({ ref }) => …)` 走"解构形参"通道、`mod.computed` 走"属性访问不算使用位点"豁免；所以本轮没有放宽任何东西，而是往 `--selftest` 加第 16–20 枚针，再给常驻电池加 **K、M** 两枚变异体去**撤掉这些针所挂的通道**：

| 变异体 | 撤掉的通道 | 实际打红的针（电池逐枚印出） |
|---|---|---|
| K | `const { … } =` 解构声明算绑定 | **恰好 2 格**：动态 import 解构、require 解构 |
| F | 解构形参 `{ storeToRefs }` 算绑定 | **2 格**：形参同名、动态 import `.then` 解构形参 |
| M | 属性访问豁免（lookbehind） | **7 格**：属性访问不算、namespace import 兜住全部名字、动态 import 命名空间（+4 条种缺陷对照） |

⇒ 三格新加的"必须 clean"各有专属通道证据：**被看见后才判为已绑，不是靠隐身通过**。第 20 枚（`const mod = await import('vue')` 之后仍裸用 `computed` ⇒ 期望非空 `['computed']`）本身就有牙，不需要变异体替它作证。

**判据没被顺手放宽的证据**：`--check` 仍是非测试 **0 个 / 0 处**、测试 **0 个 / 0 处**、`verdict=PASS`；`--selftest` `PASS (20 合成针 + 6 种缺陷对照)`；电池默认模式 `FRAMEWORK-IMPORTS MUTANTS: PASS (12 枚变异体都必须把 --selftest 打红)`；`--variants` 三档判决与 §11.15 逐字同形（base rc=0 两桶全 0；撤遮蔽 rc=1 且非测试 **2 个 / 2 处**、点名 `components/layout/TheSidebar.vue`、`modules/expert-alliance/components/GraphTeamPanel.vue`；放宽档与 base 同形 ⇒ 语料对它盲，唯一牙＝第 15 枚针）。EP 那把未动，仍 **29 个 / 446 处**。

**顺带把"证据"变成常驻输出**：电池此前只印"点名的那枚针红没红"，本轮起逐枚追加"该枚共打红 N 格 ＋ 针名"。这不是装饰——"期望空集的针到底挂在哪个通道上"原本只能靠人在场回忆，现在每次重跑都印出来。两份落库脚本自验：`check-framework-imports.py` **17069 B / 369 行**、`check-framework-imports-mutants.py` **9141 B / 187 行**，均 CR=0、无 BOM、裸 hex **0 处**；本轮**未新增一次性脚本**，也未写任何 `src/` 文件。

### 11.17 收尾复测撞见并发写者：`frontend-ui/scripts/gate/` 已被别人 `git add`，且索引快照落后于工作树（2026-09-27 09:42–09:45）（**索引状态已被 §11.19 推翻：那批暂存随后被提交，我的两把工件回到未跟踪**）

**起因**：按"收束时重跑而非回忆"复测。三道判决逐字未变 —— framework `--check` 非测试 **0 个 / 0 处**＋测试 **0 个 / 0 处**、`--selftest PASS (20 合成针 + 6 种缺陷对照)`、`FRAMEWORK-IMPORTS MUTANTS: PASS (12 枚变异体…)`，红针名册实测 **K＝2 格**〔动态 import 解构、require 解构〕、**F＝2 格**、**M＝7 格**；EP 那把仍 **29 个 / 446 处**（头名 `ExpertConfigView.vue` 45@L3001＋4@L3176，`api/http.js` 6@L157 仍在账上）。顺带 `find src -mmin -480` 量到 **6 个源文件在本轮窗口内被写下**：`api/index.js`、`api/sso.api.js`、`views/auth/Login.vue`、`views/admin/AdminView.vue`、`views/admin/panels/AdminSso.vue`、`modules/system/index.js`（时间戳 09:20–09:42，最新一枚距复测不足 1 分钟），合计 **+409 / −22** 行，主题是 SSO 登录面。**均非本 agent 所为**（"本任务至今零 `src/` 改动"这条不变），且都不在 EP 那 29 个的名单里。

**由此翻出本报告两处已作废的口径**：`git status --porcelain` 按**第一列**分桶为 **193 `A ` / 157 `M ` / 36 `R ` / 5 `MM` / 5 `AM` / 1 `D ` / 1 ` M` / 7 `??`**，索引里共 **397** 条与 HEAD 有差异 —— 即在"要不要 `git add`"还挂在待点头上的这段时间里，**有人做了宽范围 `git add`**（形状像 `git add -A`；它没盖住后续写入，所以留下了那 5 枚 `AM`/`MM`）：

| 旧陈述（出处） | 现状（实测） |
| --- | --- |
| 「`frontend-ui/scripts/gate/` 整目录仍是 `??`」（§11.15 末） | **10 个脚本已全部进索引**，含上一批遗留待裁决的 `_mutant_role_judgments.py`、`_mutant_scan_set.py`，也含本轮两把 |
| 「两把探针……同样**尚未 `git add`**」（§11.14） | 同上 ⇒ 待点头 ② 不再是"做不做"，而变成"这 397 条已入索引的东西以什么形状提交" |
| 「两把探针都没接进 CI」（§11.14 边界） | 仍成立，但对照物变了：索引里的 `ci.yml` 相对 HEAD **新增了一步** `python scripts/gate/check-frontend-module.py`（HEAD 只有 `verify-ports.py`）⇒ 别人已在往 ci.yml 接别的门禁，而我的两把探针**不在任何版本的 ci.yml 里** |

**更要紧的一条（只登记、不动手）**：索引里那份 `check-framework-imports-mutants.py` 是**旧快照**，与工作树差 3 行，恰好就是"红针名册常驻输出"那一处（`allred` 从 `[:40]` 截断改为 `[7:].split(' ->')[0]`，并在末尾打印 `'、'.join(allred)`）。`git diff --numstat` 实测：电池 **3/3**、本报告 **20/0**（§11.16 未全量入索引）。含义：**此刻若按索引提交，落库的电池并不打印名册，而 §11.16 里"名册已成常驻输出"这句话在 HEAD 上就是假的。** 这是"权威文档自署不可信"的一个新变体——不是写错了，是**提交动作会挑到旧版本**。

**还有一条与治理规则直接冲突**：索引含 **19 条**运行时目录文件（`.sessions_tmp/*.json`、`.logs/status_check.json` 等），而 AGENTS.md 明写这类为本地运行态、不入库。整批提交即入库。

**本 agent 不动索引**：不 `git reset`、不 `git add`、不提交、不 `stash`。那 397 条是别人的进行中工作，撤索引等于丢别人的暂存。要处置只能由用户点名，且点名时要说清是"照现状提交"还是"先剔掉运行时目录那 19 条 / 先把本轮两把脚本重新 `git add` 到最新"。

**对本节结论的影响**：判据、针数、变异体数**零影响**（全部为工作树实测且刚重跑）。受影响的是**引用纪律**：本报告所有数字是"这一分钟的工作树快照"，语料正被别人同时改 ⇒ 提交前必须复测，并且**不得再引用早期版本里"未入库/未跟踪"的表述**。

### 11.18 把 §11.17 那批新写的 SSO 代码当**新语料**过一遍三把闸门：硬引用侧全对，裸 hex 侧让棘轮当场报 FAIL（2026-09-27 09:51）

并发写者落进 `src/` 的 6 个文件既是"别人的进行中工作"，也是**闸门的新输入**。逐把量，不替它担保也不替它背账。

**硬引用两把：干净，而且顺手给了一个活体正对照。** `check-framework-imports.py --check` 仍 **0 个 / 0 处**。EP 那把的名单里**没有**这两个文件，而它们合计 **14 次**用到 `ElMessage`（`AdminSso.vue` 9、`Login.vue` 5）——"没进名单"有两种成因（**已正确 import** ／ **探针漏网**），所以逐行开文件看：`AdminSso.vue:82` `import { ElMessage } from 'element-plus/es/components/message/index.mjs'`（＋`:83` 显式 `style/css.mjs`）、`Login.vue:155` `... from 'element-plus/es/components/message/index'` ⇒ **前者**，判据对两种子路径后缀（`index` 与 `index.mjs`）都认得，这是**量出来的**不是假设的。这条顺带补上了 §11.13 立的那条纪律——"正对照必须含一个已知不该被 flag 的样本"——现在这个样本由**别人新写的代码**天然提供，比合成针硬。

**裸 hex 那把：`verdict=FAIL`，成因不在我。** `check-view-hex.py --check` 实测：现存 **968 处 / 81 个文件**，基线 1000 处 / 81 个文件；三行判决 `GROWN src/views/auth/Login.vue 9 → 12`、`NEW src/views/admin/panels/AdminSso.vue 2 处`（基线里没这个文件 ⇒ 整份都是新增债务）、`SHRANK src/components/ai/PhasePipeline.vue 37 → 0`（该条是**故意不回填**的咨询项，见 [[project-ci-gates-reality]]）。算术自洽可复核：**963（本报告 §11.10–11.12 口径）＋ 3（Login 增量）＋ 2（AdminSso 全新）＝ 968** ⇒ §11.17 那句"数字是快照"在一小时内就兑现了。

**三个"不做"，以及为什么**：① **不改他们那 6 个文件** —— 迁别人的进行中代码要用户点名，而且 `AdminSso.vue` 在索引里还是 `AM`（暂存的是旧版），改它等于给一次尚未定形的提交添噪（**此句已过期**：那批此刻已随 `12b6021f` 入库，见 §11.19——"不改"的裁决不变，但理由换成"那是别人已提交的代码"）；② **不跑 `--baseline`** —— 那会把 968（含别人刚写的 5 处）冻结成新地板，属于"债务被基线掩盖"那一类反咬，正是本仓库闸门历来禁止的动作；③ **不降覆盖面下限、不改判据** —— 这次 FAIL 是**真增量**，不是判据假阳（假阳与假阴在零容忍判据下同价，但这一条两边都不是）。

**留给用户的形状（并入待点头，不新增项）**：要让棘轮回绿只有三条路，都得点名——(a) 由本 agent 把 `Login.vue` 的 3 处增量与 `AdminSso.vue` 的 2 处迁到 `var(--cat-n)`／语义档（等于授权我改别人的活代码）；(b) 由写它的人自己收，收完我这把闸门自然回到 SHRANK 轨道；(c) 显式接受"这批 SSO 提交完成前 `check-view-hex.py --check` 保持 rc=1"。注意 (a)(b) 都不动 `PhasePipeline` 那条咨询项，也都不需要回填基线。

### 11.19 那 4 枚红的重新配平，以及二十分钟内发生的**第二次索引翻覆**（2026-09-27 10:00–10:04，只读复测，零源文件改动）

**先说翻覆，因为它把上两节的时间状语全作废了。** §11.17 记录"我的 `scripts/gate/` 已被并发执行者 `git add`，且索引快照落后工作树 3 行"，§11.18 据此写下"`AdminSso.vue` 在索引里还是 `AM`（暂存的是旧版）"。本轮复测：

- `git status --porcelain` 第一列分布 = **165 ` `／115 `??`／37 `D`**，**`A`/`M` 开头的暂存条目为 0**（`grep -c "^[AM]"` 实测 `0`）；
- `git ls-files -- frontend-ui/scripts/gate reports/markdown/module-governance.md` → **空**，两把工件**回到未跟踪**；
- `git rev-parse --short HEAD` 从 `99f61f71` 前进到 **`12b6021f`**（`feat(sso): 登录页企业 SSO 接入 + AdminSso 管理面板`，7 文件 +485/−22，含 `views/auth/Login.vue`、`panels/AdminSso.vue`、`api/sso.api.js`、`modules/system/index.js`）。

⇒ 那 397 条暂存不是被 `git reset` 掉的，是**被提交掉了**。所以 §11.17/§11.18 里"索引冻住了我工件的旧快照 ⇒ 此刻提交会记下一把不打印针名的电池"这条危险**已经消失**（不是被我化解，是别人的动作让它过期）。留下的正确陈述只有一句：**HEAD 前进了一格，而我的两份工件仍在跟踪之外**。顺手记一条量法教训：判"我的报告是否已入库"时我用 `grep -E "module-governance|frontend-ui/scripts/gate"` 扫 `git ls-tree -r HEAD`，得到 **2 条命中**——逐条看是 `reports/architecture/module-governance-20260905.json` 与 `-20260906.json`，**同前缀的不同家族**。子串匹配在这里假报"已入库"，必须把命中行印出来才算证据（与"没进名单"两种成因是同一类病）。

**裸 hex 那把的 `--check`：本轮复测与 §11.18 完全一致**（现存 968 处／81 文件，基线 1000／81；`GROWN Login.vue 9→12`、`NEW AdminSso.vue 2 处`、`SHRANK ai/PhasePipeline.vue 37→0`、`verdict=FAIL`）——**但这三行的语义变了**：`Login.vue` 的 +3 与 `AdminSso.vue` 的 2 处此刻**已在 HEAD 里**，不再是"别人正在写的 WIP"。⇒ §11.18 留给用户的三条路里，**(b)"由写它的人自己收"的含义从"把他没收完的那批收完"变成"要再来一次后续提交"**，代价被抬高了；(a)(c) 不变。

**`--selftest`：`PASS=58 FAIL=4`，总数没变，但四红的内部数值全变了，且我记忆里的配平只对一半。** 逐枚按本轮实测重钉（标签里的"实测 N"一律按散文快照处理，不进账）：

| 红针 | 本轮实测 | 判据两槽 | 定性 |
|---|---|---|---|
| `components 在账上且不是零头` | 20 文件／355 处（下限 `>=20`／`>=500`；标签写的是旧的 25/696） | `(True, False)` | **反咬型**：我把这层从 696 清到 355。且**第一槽此刻 20/20 = 零余量**，这层再清空一个文件就两槽齐红 |
| `旧三根之外的文件在账上` | 12 文件／67 处（下限 `>=13`／`>=81`） | `(False, False)` | **反咬型**：跨下限的那个文件正是 `src/App.vue`——它既不在扫描集、也**已从 BASELINE 删掉**（清零即删条目，是规则要求的动作），而这条判据的标签恰恰拿"含 App.vue"当存在理由。清对了 ⇒ 红给了 |
| `被排除那一层此刻 0 处裸 hex` | **6 处**（期望 0） | 得 6 期望 0 | **真 tripwire**，仍指向 admin-lowcode，本轮钉到行：`modules/admin-lowcode/engine/SchemaCrudPage.vue:142 var(--c, #8BC8EA)`、`:145 color: #1A1B1C`；`pages/tenant.page.js:74-77` 四个 `color: '#8BC8EA'/'#52C41A'/'#FAAD14'/'#C9A7E8'`。这批**也已入库**（`d144628a`/`9c792e6f`），所以"等作者自己收完"对这条同样不成立 |
| `台账与扫描集同集合` | 账上多出 `ai/PhasePipeline.vue`（BASELINE 37 vs 扫描 0）；扫描集多出 `AdminSso.vue`（2 处、账上无名） | 得两条非空 | **一格两因**：前者是我被明令**不许回填**的咨询项；后者由 §11.18 那批 SSO 提交造成 |

对照记忆里的旧配平（"2 反咬 + 1 tripwire + 1 PhasePipeline 故意不回填"）：**形状仍成立，但第 4 枚从"一个名字"变成"两个名字、两个成因"**，第 1 枚的 `>=20` 槽从有余量变成零余量。⇒ 复述旧账不算复测，本轮这两个差别只有跑过才知道。

**对验收口径的诚实影响**：#17 的验收是"闸门必须报 SHRANK，且还原原地后必须被具名变异体打红"。此刻 `--check` 是 `verdict=FAIL`，而唯一的 SHRANK 行恰好是我**不许回填**的那条咨询项 ⇒ 在 §11.18 的三选一被点名之前，**棘轮侧无法出示一次干净的 SHRANK 判决**（此前各批的 SHRANK 证据仍在其各自的记录里，不被本节追溯撤销）。这是"卡住"，不是"通过"，也不是我做错了判据——两把零容忍探针本轮仍 `verdict=PASS`（`0 个/0 处`、`20 合成针 + 6 种缺陷对照`、电池 `PASS (12 枚)`）。

**本轮仍然不做的三件事，理由各自独立**：① 不降那两条覆盖面下限（反咬型红的正确处置是**用户点名 renegotiate**，不是把 `>=500`/`>=81` 调小让它闭嘴）；② 不跑 `--baseline`（会把 968 连同别人的 5 处一起冻成地板）；③ 不碰 `modules/admin-lowcode` 那 6 处（那是别人已入库的代码，且它是**唯一一处"排除层此刻真有账"的证据**，抹掉它等于把 tripwire 变成哑弹）。另登记一条对既有记忆的更正：`src/modules/` **不再是整棵未跟踪**——`git ls-files` 实测已跟踪 **12** 个文件、`??` 条目 **17** 条（半跟踪状态），故"绝不 `git add -A`，否则会卷走整棵模块树"这条纪律**结论不变但理由要换**：现在的危险不是卷走一棵新树，而是把 17 条**别人正在进行**的半跟踪改动混进我的提交。

### 11.20 棘轮那侧原来真的没牙：补一把逐枚指名针的变异体电池（**只写自己的闸门目录，零 `src/` 改动**，2026-09-27 10:11）

**为什么这一节不需要用户点名。** 硬引用两把闸门各有常驻电池，而**裸 hex 那把只有 `--selftest`、没有电池**——它 58 格针里"棘轮判得对不对"这一族（`ratchet()` 的 GROWN/NEW/SHRANK 语义、`scan_ok` 的排除规则、`palette_problems` 的缺档/非色值判定）从未被任何"撤通道"动作复验过。按本仓库口径（见 [[assertions-need-mutants]] 第 34 条）：**撤掉某条通道而挂着它的针不红＝针根本没穿这条通道，不是闸门更稳**。补电池只动 `frontend-ui/scripts/gate/`（我的、未跟踪、无人并发），属"无需点名即可做"的一类。

**判据不能用 rc——这是本把电池与另两把最大的形状差异。** 真语料此刻 `--selftest` 本就 `FAIL=4`（§11.19），"变异体让 rc≠0"是**恒真**、零信息。改判为：基线先取一次红针**集合**（实测 4 格，逐名印出），每枚变异体必须让**它指名的那一格**由不红变红，并把"该枚新打红几格、是哪几格"常驻打印。两条护栏：① 无 `SELFTEST PASS=` 判决行＝INVALID——本轮**真用上了**：初版两枚变异体把 `o(hits, base)` 用 `*` 摊给一元 lambda，`TypeError` 掀掉整个驱动（rc=1 却什么都没判）；改成"崩溃只记该枚 INVALID"后，同样的错只红那一行而不再伪造全局失败。② 驱动**从不调用 `main()`**（那是唯一会打印/改写 `BASELINE` 的入口），并在前后各取一次被测闸门 sha256 做"我没改它"的见证。

**实测判决（7/7，rc=0）**：`A 棘轮退化成只比总量 → 多一处就 FAIL，且按文件记`；`B 不再报 SHRANK → 低于基线只记 SHRANK`；`C 空基线也判 NEW → 基线为空 ⇒ 不判 NEW`；`D scan_ok 放过 test/spec → 测试文件不进账`；`E scan_ok 踢掉 constants → constants 目录确实在扫描范围内`；`F 色板对 missing 失明 → 缺档被抓到`；`G 色板对 notcolor 失明 → 值不是颜色字面量也要被抓到`。见证行：`闸门文件 sha256 前/后 = b62dd1dd9661b01e / b62dd1dd9661b01e （未改写）`。

**两处只有跑出来才知道的副产品**：① **D 一枚打红 3 格**（`测试文件不进账`、`扫描集判定 scan_ok 是纯函数且逐条如预期`、`src/modules 一个都不进扫描集`）⇒ `scan_ok` 是三条针共用的通道，将来动它的排除规则会同时惊动三格，不会静默退化；② A/B/C 各自只红自己那一格、**互不越界** ⇒ "逐文件比／报 SHRANK／空基线不判 NEW"是三条独立通道，不是一个总开关的三种叫法。

**自纠一条量法**：我第一版用 `re.findall(r"^\s+.([A-G]), ", t, re.M)` 数电池里有几枚变异体，实测印回 **0**（我把引号里的括号写成了通配符位置）。该数作废，枚数以驱动自己判决行里的 `len(MUTANTS)`＝**7** 为准——与"行数要和钉住它的计数器同源"是同一条规矩，也再次说明为什么"由被判据自己印出的数"比外部数法硬。

**边界（不许夸大）**：本把只覆盖 `ratchet` / `scan_ok` / `palette_problems` 三个入口的针，**判据 5（串门）与判据 6（表面阶梯）那两族仍是无电池状态**——它们占了 58 格里的大多数，却没有一枚"撤通道"变异体证明针穿着通道；下一轮若要续，形状就是给 `role_scan` / `tier_problems` / `surface_problems` 各配一枚。**这把电池同样尚未接 CI**（与 §11.13 那两把探针同批待点名）。落库脚本自验：`check-view-hex-mutants.py` **6466 B / 150 行**、CR=0、BOM=False、裸 hex **0**（合成色值全在被测闸门一侧，本文件不携带色值字面量）。

### 11.21 批次 9：把 §11.20 留下的"判据 5/6 无电池"补掉（只动我自己的闸门目录，`src/` 一处未改）

**做了什么**：给 `check-view-hex-mutants.py` 追加 **H–O 共 8 枚**变异体，电池从 7 枚涨到 **15 枚**。每枚仍是同一个形状：`importlib` 载闸门 ⇒ 取未变异基线的红针**集合** ⇒ 只 monkey-patch 一个属性 ⇒ 重跑 `selftest()` ⇒ 要求"该枚指名的那格"从不在红集合变成红。目标属性分布：棘轮 A/B/C、扫描集边界 D/E、色板 F/G、**角色串门与文字档做底（判据 5）H/I/J、表面阶梯（判据 6）K/L/M、正文灰阶 N/O**。

**本轮实测判决**（全部重跑，非回忆）：`HEX-MUTANTS: PASS（15 枚变异体都必须把 --selftest 指名的针打红）` rc=**0**；`SELFTEST PASS=58 FAIL=4`（与 §11.19 逐格同：components 覆盖面 / 旧三根之外覆盖面 / 排除层 6 处 / 台账两文件）；`--check` 仍 verdict=**FAIL**，`GROWN src/views/auth/Login.vue 9 → 12`、`NEW src/views/admin/panels/AdminSso.vue 2 处`、`SHRANK src/components/ai/PhasePipeline.vue 37 → 0`，存量 **968 处 / 81 个文件**，`文字档做底 33 处 / 15 个文件`（基线同值 ⇒ textasfill 侧无增长）。逐枚新红格数：**H 1 / I 2 / J 2 / K 3 / L 1 / M 1 / N 1 / O 1**，A–G 那七枚各自仍命中指名牌。K 一枚打红三格是诚实结果——塌陷这一通道同时挂着"合成表要红""未登记照红""豁免只放行登记那档对"三条针，不是过度红。

**两条自纠（口径与时效）**：① §11.20 写"下一轮形状是给 `role_scan`/`tier_problems`/`surface_problems` 各配一枚"，本轮实际做了 8 枚，且 `role_scan` 的针是**由 `role_hits` 组起来的**，所以变异打在 `role_hits`（更靠近病灶，一处撤通道能被多格针同时见证）；② §11.20 钉的"6466 B / 150 行"随本次追加作废，现值见下。另登记一条口径伤：报告行数本节我用 `text.count('\n')+1` 量得 **902**，§11.20 记的是 **901**——两个计数器口径不同（后者按 `splitlines()`），按纪律"行数要与钉住它的计数器同源"，此后统一用前者。

**落盘自验**：`check-view-hex-mutants.py` **8872 B / 183 行**、CR=0、BOM=False、裸 hex **0**、sha256 前缀 `dd6d7b9ff4e55f0d`，变异体条数由驱动自己的 `len(MUTANTS)` 给（本轮 15）；被测闸门 `check-view-hex.py` sha256 前缀电池前后均 `b62dd1dd9661b01e`（**未改写**，本驱动从不调用 `main()`，因此不可能回填 BASELINE）。

**仍要说出口的边界**：这把电池覆盖的是闸门**自己 `--selftest` 的六面判据**，不等于 #17 的验收达成——① `--check` 此刻 FAIL，唯一 SHRANK 行是我不许回填的那条咨询项，**干净的 SHRANK 判决仍出示不了**（§11.18 三选一仍未被点名，即 §11.20 的 ⑦）；② 15 枚变异体与两把探针、本报告**都还没接进 `.github/workflows/ci.yml`**（③）；③ 电池只证"通道被撤时指名的针会红"，不证闸门对**未预见**的洗账形态有牙。另：本轮零 `src/` 改动、零索引动作、未跑 `--baseline`、未降任何下限。

### 11.22 批次 10：把 §11.21 说出口的"判据自身盲区"补掉（改的是闸门，不是 `src/`）

**改了什么**：`check-view-hex.py` 的判据 5 那一族里，「真语料的『文字档做底』不许越过基线」写的是 `ratchet(rtier, ROLE_TIER_BASELINE)[:2]` ⇒ `ratchet` 的第三条腿（`gone`／回填提示）被切掉，"文件清零了但账没删"和"新文件漏记账"两种形态在这一格上是**静默的**。本轮在它后面补一格台账同步不变量：「「文字档做底」台账与扫描集同集合（[:2] 切掉的 gone 腿由此格见证）」，比的是 `(sorted(set(rtier)-set(ROLE_TIER_BASELINE)), sorted(set(ROLE_TIER_BASELINE)-set(rtier)))` 对 `([], [])`。**这一格只落在 `frontend-ui/scripts/gate/` 里，`src/` 一处未动。**

**动手前先量（这条决定它是绿是红，不能猜）**：`role_scan` 给 rtier **15 个文件 / 33 处**，`ROLE_TIER_BASELINE` 亦 **15 / 33**，两个方向集合差均空、逐文件计数无不等者 ⇒ 新针**落地即绿**，不是我造出来的新红。

**动手后实测**：`--selftest PASS=59 FAIL=4`（分母 58→**59**；四红逐格与 §11.19 同，标签未变）；`--check` 仍 verdict=**FAIL**，存量 **968 处 / 81 个文件**，`GROWN Login.vue 9 → 12`／`NEW AdminSso.vue 2 处`／`SHRANK PhasePipeline.vue 37 → 0` 三行原样。电池的见证才是关键：**I（`role_hits` 对 text-as-fill 失明）新打红从 2 格涨到 3 格，点名包含新那格；J（撤掉纯 `.css` 整份判）同样从 2 涨到 3 并包含新那格** ⇒ 原先只有合成表针能抓的洗账形态，现在**真语料台账也会撞**，[:2] 的静默角落无处可藏；`HEX-MUTANTS: PASS（15 枚…）` rc=**0**。

**一条必须说出的连带（sha 与"58 格"都是历史值）**：闸门文件被我改写 ⇒ `check-view-hex.py` sha256 前缀 `b62dd1dd9661b01e` → **`65a4bf39c0c9ed95`**；§11.19/§11.20/§11.21 里钉的旧 sha 与"PASS=58"自此是**历史快照**，不是此刻值（电池内部的前后见证仍相等：`65a4bf39… / 65a4bf39…`，因为电池从不调 `main()`，不可能回填 BASELINE）。

**边界（别把它当验收）**：新针是**台账同步**不变量，与 #17 的验收两半句都无关——`--check` 仍 FAIL，干净的 SHRANK 判决**仍出示不了**（⑦ 未点名）。余项：①–⑥ 同 §11.20 清单，⑦ 三选一，⑧ 已并入本轮（判据盲区已补），新增 ⑨ **凡引用"58 格／b62dd1dd"的既有段落需按本轮值重钉**（本报告除 §11.22 外不改写历史数字，只在此声明其过期）。本轮零 `src/` 改动、零索引动作、未跑 `--baseline`／`--role-baseline`、未降任何下限。

### 11.23 批次 11：给 ⑦ 定价——"迁 5 处"是我说错的口径，真数是 **14 处**（全部内存反事实，磁盘一字未改）

**为什么做这个**：⑦ 卡在用户点上，而我只报过"GROWN 3 + NEW 2"。GROWN 报的是**与基线的差额**，不是**要动的手术量**，更不是验收要的那半句。于是拿闸门自己的代码做反事实：monkey-patch `read()` 让 `views/auth/Login.vue` 与 `views/admin/panels/AdminSso.vue` 返回"把每个裸 hex 替换成 `var(--cat-6)`"的内存副本，再跑 `scan_sources()` 与 `ratchet()`。**不写盘、不碰 `src/`、不跑 `--baseline`。**

**站点实数（由 `hex_sites` 给，含行号）**：`AdminSso.vue` **2 处**＝`:235 #909399`、`:236 #67c23a`（正是 EP 的 info/success 语义色字面量，即 NEW 那两枚）；`Login.vue` **12 处**＝`:274 #5264bf`、`:275 #606266`、`:280 #f5f7fa`、`:286 #667eea + #764ba2`、`:364 #1a1b1c`、`:402 #667eea`、`:416 #d1d5db`、`:425 #e5e7eb`、`:445 #e5e7eb`、`:448 #667eea + #f5f7ff` ⇒ 合计 **14 处 / 11 个不同字面量**（`#667eea` 三次、`#e5e7eb` 与 `#f5f7fa`/`#f5f7ff` 各成对）。

**反事实判决**：把这两份**清空**后 `现存 954 处 / 79 个文件`（原 968/81），`grown=[] new=[]` ⇒ **`--check` 会转 verdict=PASS**；`gone=[PhasePipeline 37→0, Login 9→0]` ⇒ **同时出示两条 SHRANK**，#17 那句"闸门必须报 SHRANK"第一次能被出示（不是靠回填造出来的，是靠真降）。台账侧：把清零文件的 `BASELINE` 行**删掉**（纪律：不是写 0）后 `grown/new/gone` 全空、phantom 空、unrecorded 空 ⇒ "台账与扫描集同集合"那格与判据 5 新加的那格都干净。

**由此得出一条以前没说的验收算术**：GROWN 只要求回到基线（`Login` 12→9 即可消红），但**"必须报 SHRANK"要求清到基线以下** ⇒ 只迁"新写的 3 处"永远出示不了 SHRANK；要出示就得把 `Login.vue` 的 12 处**全清**。这条把 ⑦ 的三个选项重排成：**(a′) 授权我清 14 处**（其中 `Login` 12 处含 9→0 的越基线清账）＝唯一能一次出示"PASS + 具名 SHRANK + 变异体打红"的路；**(b′) 原作者清**（SSO 那 2 处归 `12b6021f` 的作者，`Login.vue` 归 `71cbf3b3`/SSO 批次作者）；**(c) 接受 rc=1**。

**两处手术陷阱先亮明（都有记忆出处，别到动手时才发现）**：① `:286 #667eea → #764ba2` 是**恒深渐变横幅**，不许"迁到现有令牌"——`--hero-gradient` 在 sky 皮下是近白渐变且零消费者，迁了就把恒深改成跟肤；② 迁字色必须连底一起迁（`:275 #606266` 压 `:280/#448 #f5f7fa/#f5f7ff` 是成对的，只换字色会让深字压深底）。③ `:235/236` 的 `#909399/#67c23a` 是 EP 语义色，正解走 `--danger/--success` 族还是 `--el-*` 影子名要按判据 5 的家族契约选——**影子名不参与家族 ⇒ 用 `--success` 系而不是 `--el-color-success`**。

**边界**：以上是**反事实**，磁盘上两份文件此刻与 `12b6021f` 一致、我未写入；`--check` 真值仍是 FAIL / 968 处 / 81 文件 / GROWN 1 / NEW 1 / SHRANK 1。本轮零 `src/` 改动、零索引动作、未跑 `--baseline`／`--role-baseline`、未降任何下限。

### 11.24 批次 12：把 §11.23 那次性的反事实变成常设仪器 `plan-view-hex-clearance.py`

一次性探针不能替决策定价——它下一轮就腐烂。于是把同样的形状落成第五把闸门目录脚本：**待清文件自动取自 `ratchet` 的 grown/new**（不硬编文件名），**站点表与计数全部由闸门自己的 `hex_sites`／`scan_sources`／`ratchet` 给**（本脚本不携带第二把口径，也不携带任何色值字面量），反事实靠 monkey-patch `read()` 在内存里替换，并自带三条判据：① `现量 − 待清 = 实得`，不等即 INVALID；② 清后仍非空 ⇒ 印"仍不清绿"；③ 只有新量**低于**基线才把该文件列进"本次清账产生的 SHRANK"。

**实测（与 §11.23 那次独立手量做交叉见证）**：候选 2 文件，`AdminSso.vue` 2 处 / 2 字面量、`Login.vue` 12 处 / 9 字面量，**两文件的 14 处全部不在注释里**（注释侧 0 ⇒ 清它们是真减债，不是改账）；`968 − 14 = 954，实得 954 ⇒ 算术自洽`；`grown=[] new=[]`，`gone=[PhasePipeline 37→0, Login 9→0]`，**由本次清账产生的 SHRANK＝`src/views/auth/Login.vue`**；删零头条目后 phantom 空、未记账空；`闸门 sha 前后 65a4bf39c0c9ed95（未改写）`，`CLEARANCE: 定价完成（只读）` rc=**0**。

**落盘自验**：`plan-view-hex-clearance.py` **5460 B / 132 行**、CR=0、BOM=False、裸 hex **0**、ctrl=[]；首轮曾因我把第二处推导式的循环变量写成 `_r` 而 `NameError: name 'r' is not defined`（rc=1，无判决行 ⇒ 按纪律记 INVALID 而非"跑过了"），改名为 `rel` 后到判决。

**边界**：仪器仍**未接 CI**，与另四把工件同批待点名（②③）；`--check` 真值此刻仍 FAIL，#17 的验收仍等 ⑦（**a′ 清 14 处**／b′ 原作者清／c 接受 rc=1）。本轮零 `src/` 改动、零索引动作。

### 11.25 挂起快照：本批工件在 HEAD `12b6021f` 之后的一次全量复测（2026-09-27，只读）

目标轮次将尽，按纪律把"收束时的当前真值"钉在这里，**重跑而非回忆**：

- `plan-view-hex-clearance.py` rc=**0**，`CLEARANCE: 定价完成（只读，未写任何文件）`；现量 **968 处 / 81 文件**，基线 1000 处 / 81 文件；候选 2 文件＝`AdminSso.vue` 2 处 / 2 字面量、`Login.vue` 12 处 / 9 字面量（两处注释侧均 0）；`968 − 14 = 954，实得 954 ⇒ 算术自洽`；反事实 `gone=[PhasePipeline 37→0, Login 9→0]`，**本次清账产生的 SHRANK＝`src/views/auth/Login.vue`**；删零头条目后 grown/new/gone/phantom/未记账全空。
- `check-view-hex-mutants.py` rc=**0**，`HEX-MUTANTS: PASS（15 枚变异体都必须把 --selftest 指名的针打红）`，末枚 O 命中「缺一档被抓到」新打红 1 格。
- `check-view-hex.py --check` rc=**1**，`verdict=FAIL`，`文字档做底 33 处 / 15 个文件`（与基线同值 ⇒ role 侧无增长），三行账＝`GROWN Login.vue 9→12`／`NEW AdminSso.vue 2 处`／`SHRANK PhasePipeline 37→0`（后者是**不许回填**的咨询项，见 §11.13）；`--selftest` 红格数 **4**（未变）。
- 三把工件 sha256 前缀＝`65a4bf39c0c9ed95`（闸门）／`dd6d7b9ff4e55f0d`（变异电池）／`b11d1c3a41f1322e`（清账定价器）；电池与定价器各自打印"闸门 sha 前后相等"，该见证**只对当轮有效**（§11.22 的教训）。
- 索引状态：`git status --porcelain` 首列 `[AM]` 计数 **0**（未动索引），HEAD＝`12b6021f feat(sso): 登录页企业 SSO 接入 + AdminSso 管理面板`（并行执行者的提交，本批工件均在其之后且仍 `??` 未入库）。

**挂起结论**：#17 的两个验收条件里，"具名变异体打红"这半边已由 15 枚覆盖到判据 5/6 与扫描集/色板/棘轮全部通道；"闸门必须报 SHRANK"那半边**只差 ⑦ 的一个点名**——a′ 授权我清这 14 处是唯一能一次同时出示 `PASS ＋ 具名 SHRANK ＋ 变异体打红` 的路。目标未达成，不报完成。

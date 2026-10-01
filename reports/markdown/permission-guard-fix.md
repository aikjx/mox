# 权限守卫断裂修复报告（Permission Guard Fix）

> 日期：2026-09-26 ｜ 诊断依据：`reports/markdown/module-governance.md` 第 4 节
> 范围：只改权限判定层与路由守卫 meta，**未触碰登录链路本身**（login/logout/token 流程不变）。

---

## 1. 诊断回顾（改前问题）

对 `:3080` 实跑 `GET /api/system/permissions`（Bearer dev-secret-token）：

```jsonc
{
  "permissions": ["user:view","user:create",...],          // 字符串数组 ✅ 形状对
  "roles": [ {"id":"...","code":"tenant_admin","name":"租户管理员"}, ... ]  // 对象数组 ❌
}
```

两处断裂：

| # | 问题 | 改前实现 | 后果 |
|---|------|---------|------|
| 假权限-形状 | `permission.store.js` `setRoles()` 原样把后端 `[{id,code,name}]` 对象数组塞进 state，再 `includes('admin')` 字符串比对 | 对象永不等于字符串 | `/admin/*` 的 `hasAnyRole(['admin'])` 恒为 false，守卫形同虚设；admin 实际靠 localStorage 残留或 dev 态放行 |
| 假权限-码值 | 路由写死 `requiresRole: ['admin']` | 后端预置 role_code 是 `super_admin`/`tenant_admin`（`enterprise/tenant.rs` 291/308、`system.api.js` ROLE_TEMPLATES），从不发字面 `'admin'` | 即便形状归一，`'admin'` 也匹配不到 `tenant_admin` |
| 缺权限 | `/expert-center/*` 管理后台只有 `requiresAuth:true`，`isExpertAdmin` meta 无人消费 | 任何登录用户可进专家管理台 | 专家管理能力面未被前端守卫 |

---

## 2. 改动文件清单

| 文件 | 改动 |
|------|------|
| `frontend-ui/src/stores/permission.store.js` | ① 新增 `TENANT_ADMIN_ROLE` 与 `ADMIN_ROLES` 常量；② `isAdmin` 改为 `ADMIN_ROLES.some(...)`；③ `setRoles()` 归一化：`[{id,code,name}]`→`[code]`，兼容字符串数组，缺 code 过滤 |
| `frontend-ui/src/router/modules/system.js` | 全部 `requiresRole: ['admin']` → `requiresRole: ['super_admin','tenant_admin']`（父路由 + 20 子路由，含 3 个低代码试点页，共 21 处） |
| `frontend-ui/src/router/modules/alliance.js` | `/expert-center` 父路由 meta 新增 `requiresRole: ['super_admin','tenant_admin']`（vue-router 父 meta 合并进 4 个子路由） |
| `frontend-ui/src/stores/permission.store.test.js` | **新增** 6 例：归一化、兼容字符串、缺 code 过滤、isAdmin、普通用户拒绝、守卫命中 |

> 未改：`auth.store.js`、`router/index.js` 守卫段本身、登录/登出/token 流程。

---

## 3. 归一化逻辑（permission.store.js）

```js
const ADMIN_ROLES = ['admin', 'super_admin', 'tenant_admin']   // 'admin' 为历史别名兼容

const isAdmin = computed(() => ADMIN_ROLES.some((r) => roles.value.includes(r)))

function setRoles(roleList) {
  const normalized = Array.isArray(roleList)
    ? roleList.map((r) => (typeof r === 'string' ? r : r?.code)).filter(Boolean)
    : []
  roles.value = normalized
  _safeSet(ROLES_KEY, normalized)
}
```

要点：
- 双向兼容：后端对象数组取 `.code`；旧 localStorage 里的字符串数组原样保留。
- 缺 `code` 的脏数据被过滤，不会把对象混进角色列表。
- `loadPermissions()` 仍原样 `setRoles(data.roles)`，归一化内聚在 setter，登录调用点零改动。

---

## 4. 守卫对照表（改前 / 改后）

| 路由 | 改前 meta | 改后 meta | 实际效果（admin-user 角色码 tenant_admin） |
|------|----------|----------|------------------------------------------|
| `/admin/**`（21 处） | `requiresRole:['admin']` | `requiresRole:['super_admin','tenant_admin']` | 归一化后 roles=`['tenant_admin']`，`hasAnyRole` 命中 → ✅ 可进 |
| `/expert-center/{overview,enterprise,orchestrator,tasks}` | 仅 `requiresAuth` | 父路由加 `requiresRole:['super_admin','tenant_admin']` | tenant_admin 可进；普通用户被拦 /403 |
| 普通用户 `normal_user` | — | 同上 | `isAdmin=false`，`hasAnyRole([...])=false` → ✅ 正确拦截 |

---

## 5. 验证结果

### 5.1 vitest
- 新增 `permission.store.test.js`：**6/6 通过**。
- 全量回归：`cd frontend-ui; npx vitest run` → **35 文件 / 700 用例全绿，exit 0**（日志 `reports/data/vitest-after-perm-fix.txt`）。
- 过程中 `useTheme.test.js` 一度报 `Unterminated block comment`，经核对文件磁盘完整（245 行正常闭合），清 `node_modules/.vite` 缓存后 19/19 通过——属 vite transform 缓存抖动，非本次改动引入。

### 5.2 curl 权限端点（后端实况）
- `permissions` 为字符串数组（与 `requiresPermission` 守卫形状一致，目前仍 0 路由使用）。
- `roles` 为对象数组、admin-user 实际 code 为 `tenant_admin`——已据此对齐前端角色码。

### 5.3 已验证 / 未验证边界
- **已验证**：归一化与判定逻辑（单测）、后端 payload 形状与角色码、全量 vitest 绿。
- **未验证（需真实 IAM 账号 + 浏览器）**：端到端登录后 `/admin`、`/expert-center` 的实际重定向/放行行为。`dev-secret-token` 仅后端 API 调用，GUI 登录需真实账号；建议登录后用一个 `normal_user` 账号确认被拦到 `/403`、用 admin 账号确认可进。

---

## 6. 行为变化与风险

| 变化 | 说明 |
|------|------|
| admin 入口从"靠残留/dev 放行"变为"按真实角色码判定" | 修复后守卫真正生效：tenant_admin/super_admin 可进，其余角色被拦 |
| **风险** | 若线上存在既非 super_admin 也非 tenant_admin、却曾因旧守卫失效而能进 `/admin` 的账号，修复后将被重定向 `/403`。需确认此类账号是否应升为 tenant_admin |
| `/expert-center` 新拦截 | 修复前任何登录用户可进；修复后仅管理角色可进。普通专家账号将失去该入口（本就不该有） |

**回退方案**：
- 单文件回退即恢复旧行为：`git checkout -- frontend-ui/src/router/modules/system.js frontend-ui/src/router/modules/alliance.js`。
- `permission.store.js` 的归一化向后兼容（不改变登录调用点），即使保留也无害；如需完全回退再 `git checkout --` 该文件。
- 本次改动均为新增测试 + meta 字符串，无数据迁移、无后端改动。

---

## 7. 遗留缺口
1. `/expert-config`（专家配置引擎，顶层路由非 /expert-center 子路由）仍仅 `requiresAuth`，按 admin 面应同补守卫（本轮未扩，避免越界）。
2. `meta.requiresPermission` 守卫基础设施就绪但 0 路由使用——后端已下发细粒度 `permissions` 字符串，后续可逐面板按 `user:view` 等细化。
3. 端到端浏览器守卫验证待真实 IAM 账号执行。

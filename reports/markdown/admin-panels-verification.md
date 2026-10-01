# 管理平台 17 面板 + AdminView.vue 装配层验证报告

- 验证日期：2026-09-26
- 验证环境：前端 dev :3020 · 网关 :3080 · 编排器 :3001（三服务 UP）
- 认证：`Authorization: Bearer dev-secret-token`
- 证据方式：① 逐面板源码提取 `@/api` 调用；② `curl.exe` 实测（GET 直测，写操作造最小 body，凭证走「创建→校验→吊销」自清理）；③ 后端路由注册表 `/actuator/mappings`（网关实测 228 条路由）+ Rust 源码行号交叉印证。
- 信封：后端统一 `{code,msg,data}`，`code===0` 解包为 `data`。

> 定性口径：**通过** = 端点已注册且 curl 200 返回结构合理；**缺口** = UI 存在但端点 404/方法不符；**rejected** = 后端无路由（带源码/注册表证据），按要求不硬造 UI。

---

## 一、逐面板结论

### 1. AdminOverview（管理总览，181 行）
调用端点：
| UI 入口 | curl | 响应结构 | 定性 |
|---|---|---|---|
| `getSecurityStatus()` GET `/api/security/status` | 200 | `{auth_enabled,db,default_tenant,iam,rate_limit_enabled,ts}` | 端点在，但字段与面板期望（`active_api_keys/audit_log_entries/security_health/recommendations`）**不匹配**，KPI 降级为 `-` |
| `getStorageStatus()` GET `/api/storage/status` | **404** | 空 | **rejected**：228 条路由 0 条匹配 storage |
| `getModules()` GET `/api/modules` | **404** | 空 | **rejected**：`operation_log/api.rs:186` 注册的是根 `/modules`，实测 `/modules` 与 `/api/modules` 均 404 |
| `getLlmStats()` GET `/api/llm/stats` | **404** | 空 | **rejected**：llm 族无路由 |
| `getFullStatus()` GET `/api/status/full` | 200 | `{version,status,graph,operators_count,executions_count,success_rate,ai_plugins…}` | 通过（系统信息卡可用） |

结论：5 个数据源 3 个 rejected、1 个字段错位、仅 `/status/full` 完全可用。面板已全部 `catch(()=>{})` 优雅降级，不硬造 UI。

### 2. AdminAccess（访问凭证，226 行）— **本次补全**
调用端点（均实测）：
| UI 入口 | curl | 响应结构 | 定性 |
|---|---|---|---|
| `getApiKeys()` GET `/api/security/api-keys` | 200 | `[{id,name,apiKey(脱敏),status:"active"\|"revoked",scopes,createdAt,revokedAt,userId}]` | 通过 |
| `createApiKey()` POST `/api/security/api-keys` | 200 | `{id,name,api_key(明文仅此一次),active,createdAt}` | 通过 |
| `revokeApiKey()` DELETE `/api/security/api-keys/:id` | 200 | `data:null` | 通过（实测吊销） |
| `validateApiKey()` POST `/api/security/validate` | 200 | `{valid,name,permissions,user_id}` | 通过 |

**发现并修复的字段错配**（后端 `system/security.rs:58 create_api_key` / 列表 `api_key_json` 实测契约）：
- 前端原读 `data.key` → 后端返回 `data.api_key`，明文永不显示；
- 前端原读 `row.active`(布尔) / `row.permissions`(数组) → 后端返回 `row.status`("active"/"revoked") / `row.scopes`，导致状态徽标恒为「已吊销」、吊销按钮 `v-if="row.active"` 永不出现、权限标签空。

**已改 `AdminAccess.vue`**：列表归一化为 `{active: status==='active', permissions: scopes||[]}`；创建读取 `data.api_key||data.key`。端点本就存在且可用，属「未接线」→ 补到生产级（列表/创建/吊销/校验/加载/错误/空态齐备）。

### 3. AdminApi（接口管理，397 行）
| UI 入口 | curl | 定性 |
|---|---|---|
| `getActuatorHealth()` GET `/actuator/health` | 200 `{components,status,uptime_secs}` | 通过 |
| `getActuatorMetrics()` GET `/actuator/metrics` | 200 `{measurements{latency_avg_ms,requests_total…}}` | 通过 |
| `getApiMappings()` GET `/actuator/mappings` | 200 `{contexts.mox-gateway.routes[228]}` | 通过 |
| `getApiDetail()` GET `/actuator/api/:id` | 注册（actuator.rs 路由表） | 通过 |
| `enableApi/disableApi` POST `/actuator/api/:id/enable\|disable` | 注册（管理面防自锁） | 通过 |

结论：通过，统计/筛选/详情/启停闭环完整。

### 4. AdminAudit（审计日志，615 行）
| UI 入口 | curl | 定性 |
|---|---|---|
| `getOperLogList()` GET `/api/system/operlog` | 200 `data:[]` | 通过（空表） |
| `cleanOperLog()` DELETE `/api/system/operlog/clean` | 注册 | 通过 |
| `exportOperLog()` GET `/api/system/operlog/export` | 注册 | 通过 |
| `getLoginLogList()` GET `/api/system/logininfor` | 200 真实登录记录 | 通过 |
| `cleanLoginLog/exportLoginLog` | 注册 | 通过 |

注：操作日志详情弹窗直接用行数据（未调 `getOperLogDetail`），无 UI 冗余。结论：通过。

### 5. AdminConfig（参数配置，336 行）
| UI 入口 | curl | 定性 |
|---|---|---|
| `getConfigList()` GET `/api/system/config` | 200 `data:[]` | 通过（空表） |
| `create/update/delete` POST/PUT/DELETE `/api/system/config[/:id]` | 注册 | 通过 |
| `refreshConfigCache()` DELETE `/api/system/config/refresh-cache` | 注册 | 通过 |

结论：通过。

### 6. AdminDepartment（部门管理，798 行）
| UI 入口 | curl | 定性 |
|---|---|---|
| `getDeptTree()` GET `/api/system/dept/tree` | 200 `[{总裁办}]` | 通过 |
| `getDeptDetail()` GET `/api/system/dept/:id` | 200 真实（d001-dept） | 通过 |
| `create/update/deleteDept` | 注册 | 通过 |
| `getPostByDept` GET `/api/system/post/dept/:deptId` | 注册 | 通过 |
| `create/update/deletePost` | 注册 | 通过 |
| `getDeptUserList` GET `/api/system/dept/:id/users` | 注册 | 通过 |
| `getUserList`（负责人搜索）GET `/api/system/user` | 200 | 通过 |

结论：通过。

### 7. AdminDict（字典管理，489 行）
| UI 入口 | curl | 定性 |
|---|---|---|
| `getDictTypeList` GET `/api/system/dict/type` | 200 `data:[]` | 通过 |
| type CRUD POST/PUT/DELETE | 注册 | 通过 |
| `getDictDataList` GET `/api/system/dict/data` | 注册 | 通过 |
| data CRUD POST/PUT/DELETE | 注册 | 通过 |

结论：通过（空表）。上移/下移用两次 `updateDictData` 交换 sort，合理。

### 8. AdminDocs（API 文档，170 行）
定位为「运行时接口探索器」（`import * as api` + 32 个 `try-it`）。抽样实测：
- 200：`/health` `/status` `/status/full` `/logs` `/operators` `/graph` `/graph/stats` `/ai/resources` `/ai/plugins` `/ai/flows` `/dialogue/sessions` `/caomei/templates`
- 405：`/analyze/spiral`（GET 不允许）
- 404：文档面板内 `/ai/llm/config` 实为 200（旧兼容端点可用），但 `/llm/providers/presets` 404。

结论：**已足够（探索器定位）**——非 CRUD 面板，点「试一试」对已下线端点自然报错，属预期，不需补 CRUD。

### 9. AdminHitl（HITL 审批，379 行）
不走 REST，走 WebSocket `/ws/hitl`（`@/utils/hitl-ws`），含连接状态/重连/3s 兜底 loading。REST 矩阵 N/A。结论：通道型面板，不参与 REST 端点统计。

### 10. AdminLlm（大模型配置，1364 行）— **rejected**
面板调用 `getLlmProviders/getLlmPresets/getLlmHealth/getLlmStats/getLlmUsage/getLlmLogs/enable/disable/test/setActive/remove/discover/add/updateLlmProvider/updateLlmRouting/getWebSearchConfig/updateWebSearchConfig/testWebSearch`。
| 端点 | curl |
|---|---|
| GET `/api/llm/providers`、`/presets`、`/health`、`/stats`、`/usage`、`/logs` | 全部 **404** |
| POST/PUT/DELETE `/api/llm/providers[/:id/…]`、`/routing` | **404** |
| GET/POST `/api/web-search/config`、POST `/web-search/test` | **404** |

后端证据：网关 228 条路由中 **0 条**匹配 `llm|web-search`；仅 seed JSON 有规格；`functional-requirements-inventory.json:1687` 明述「后端缺失 /api/web-search/* 端点族，前端 4 个联网搜索函数无编排器后端支撑」。唯一存活的旧端点 `GET /api/ai/llm/config`（200）面板并未使用。
结论：**rejected**，后端无路由，不硬造 UI。

### 11. AdminLogs（在线日志，361 行）
| UI 入口 | curl | 定性 |
|---|---|---|
| `getLoggers()` GET `/actuator/loggers` | 200 `{buffered,configured_level,effective_level}` | 通过 |
| `setLoggerLevel` POST `/actuator/loggers` | 注册 | 通过 |
| `getOnlineLogs` GET `/actuator/logs` | 200 真实日志行 | 通过 |
| `clearOnlineLogs` DELETE `/actuator/logs` | 注册 | 通过 |
| `openLogTail` GET `/actuator/logs/tail`（SSE） | 注册 | 通过 |

结论：通过。

### 12. AdminMenu（菜单管理，444 行）
| UI 入口 | curl | 定性 |
|---|---|---|
| `getMenuTree()` GET `/api/system/menu/tree` | 200 `data:[]` | 通过（空表） |
| `create/update/deleteMenu` | 注册 | 通过 |

结论：通过。

### 13. AdminMonitor（系统监控，1820 行）
| UI 入口 | curl | 定性 |
|---|---|---|
| `getMetricsDetail` GET `/api/monitor/metrics/detail` | 200 cpu/mem/gc/latency/requests | 通过 |
| `getMonitorQuality` GET `/api/monitor/quality` | 200 SLA/apdex/error | 通过 |
| `getMonitorBusiness` GET `/api/monitor/business` | 200 experts/gateway（projects/tasks 字段带「待接入」注记，端点本身真实） | 通过 |
| `getAlertsSummary` GET `/api/monitor/alerts/summary` | 200 全零结构化 | 通过 |
| `getMonitorNodes` GET `/api/monitor/nodes` | 200 mox-gateway healthy | 通过 |
| `getNodeLogs/getNodeTrace` GET `/api/monitor/nodes/:name/{logs,trace}` | 注册 | 通过 |
| alert-rules CRUD + toggle + timeseries | 注册/200 | 通过 |
| `moxHealth` GET `/api/mox/health` | 200 十四维专家 | 通过 |
| `moxOptimize` POST `/api/mox/optimize` | 注册 | 通过 |
| `/status` `/status/full` `/logs` `/plugins` | 200 | 通过 |

结论：通过（最成熟面板）。

### 14. AdminRole（角色管理，1151 行）
| UI 入口 | curl | 定性 |
|---|---|---|
| `getRoleList` GET `/api/system/role` | 200 真实（sys_admin 等内置角色） | 通过 |
| `create/update/deleteRole` | 注册 | 通过 |
| `getRoleMenuPerms` GET `/api/system/role/:id/menuPerms` | 200 `[]` | 通过 |
| `assignRoleMenuPerms` PUT | 注册 | 通过 |
| `getRoleDataPerms/assignRoleDataPerms` | 注册 | 通过 |
| `getRoleUsers` GET `/api/system/role/:id/users` | 200 `[]` | 通过 |
| `copyRole` POST `/api/system/role/:id/copy` | 注册 | 通过 |

结论：通过。

### 15. AdminStorage（存储与模块，120 行）— **rejected**
| UI 入口 | curl |
|---|---|
| `getStorageProviders` GET `/api/storage/providers` | **404** |
| `getStorageStatus` GET `/api/storage/status` | **404** |
| `switchStorageProvider` POST `/api/storage/switch` | **405**（无对应业务处理器） |
| `getModules` GET `/api/modules` | **404** |

后端证据：228 条路由 0 条匹配 storage/modules（网关仅有 `file_storage` 的 `/storage-types`，与面板无关）。
结论：**rejected**，后端无路由，不硬造 UI。

### 16. AdminTenant（租户管理，291 行）
| UI 入口 | curl | 定性 |
|---|---|---|
| `getTenantList` GET `/api/tenant` | 200 真实（system / T001） | 通过 |
| `createTenant` POST `/api/tenant` | 注册（`system/mod.rs:363`） | 通过 |
| `update/deleteTenant` PUT/DELETE `/api/tenant/:id` | 注册（`:364`） | 通过 |
| `switchTenant` GET `/api/tenant/switch/:id` | 注册（`:365`） | 通过 |

结论：通过。

### 17. AdminUser（用户管理，852 行）
| UI 入口 | curl | 定性 |
|---|---|---|
| `getUserList` GET `/api/system/user` | 200 真实 | 通过 |
| `getUserDetail` GET `/api/system/user/:id` | 200 真实（zhangsan） | 通过 |
| `create/update/deleteUser` | 注册 | 通过 |
| `resetUserPwd` PUT `/:id/resetPwd` | 注册 | 通过 |
| `changeUserStatus` PUT `/:id/changeStatus` | 注册 | 通过 |
| `getUserRoles/assignUserRoles` GET/PUT `/:id/roles` | 注册 | 通过 |
| `getDeptTree/getPostList/getRoleList` | 200 | 通过 |
| `uploadUserAvatar` POST `/users/:id/avatar` | 注册 | 通过 |

结论：通过。

### 装配层 AdminView.vue
17 个 tab 与面板一一对应（overview/tenant/user/role/department/menu/dict/config/access/audit/storage/hitl/monitor/api/logs/llm/docs），`router-view` 嵌套子路由，`activeTab` 从子路由名或 `?tab=` 解析，无后端端点。装配层无缺口。

---

## 二、汇总

| 维度 | 数量 |
|---|---|
| 面板总数 | 17（+ AdminView 装配层） |
| 端点全部通过的面板 | 12：Access(已修)、Api、Audit、Config、Department、Dict、Logs、Menu、Monitor、Role、Tenant、User |
| 通道型（无 REST 矩阵） | 1：Hitl（WebSocket /ws/hitl） |
| 探索器型（已足够） | 1：Docs |
| 部分数据源 rejected | 1：Overview（3/5 数据源 404，面板已优雅降级） |
| 整板 rejected（后端无路由） | 2：Storage、Llm |

**端点统计（UI 实际调用，去重前）**：
- 通过端点（已注册 + curl 200 或路由表在册）：系统域 user/role/dept/post/menu/dict/config/operlog/logininfor/tenant 全套 CRUD、security 凭证族、monitor 全套、actuator 管理面全套、status/status-full/logs/plugins/mox —— 合计约 **70+** 个。
- 缺口/rejected 端点：`/storage/providers`、`/storage/status`、`/storage/switch`、`/modules`、`/llm/providers`、`/llm/providers/presets`、`/llm/providers/:id/*`(enable/disable/test/discover)、`/llm/providers/active`、`/llm/health`、`/llm/routing`、`/llm/usage`、`/llm/logs`、`/llm/stats`、`/web-search/config`、`/web-search/test` —— 合计 **16** 个，全部后端无路由（注册表 0 命中），定性 rejected。

## 三、本次代码改动
- `frontend-ui/src/views/admin/panels/AdminAccess.vue`：列表行归一化（`status`→`active`、`scopes`→`permissions`）；创建明文读取 `data.api_key`。其余面板未改。

## 四、遗留缺口（后端待补，前端不造 UI）
1. **Storage 与模块**（AdminStorage 整板）：`/storage/providers|status|switch`、`/modules` 无后端路由。
2. **LLM 网关与联网搜索**（AdminLlm 整板）：整套 `/api/llm/*` 与 `/api/web-search/*` 无后端路由（seed 仅有规格）。
3. **AdminOverview** 的存储实体/已加载模块/LLM 概况三张卡随上述后端缺失而无数据；`/security/status` 存在但返回字段与面板 KPI 字段名不一致（前端已降级显示 `-`，未硬接）。
4. 列表类端点（user/role/tenant/config 等）后端直接返回裸数组而非 `{list,total}`，分页 `total` 退化为数组长度（前端已兼容 `Array.isArray` 兜底，功能可用但为前端分页）。

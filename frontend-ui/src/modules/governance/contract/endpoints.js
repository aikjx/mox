// 治理台（双璇玑十四维）端点清单唯一来源。
// 权威源：platform/domains/platform/svc/mox-platform-orchestrator-svc/src/routes/governance.rs
// 该域由编排器 :3001 承载，网关 :3080 通过 proxy.rs 的 /api/{*path} 通配反代暴露给前端。
// nesting 一律 flat：这些 handler 返回 mox_api_protocol::api_ok，信封是 {code,msg,data} 单层。
//
// ⚠️ 本域在 docs/API-REGISTRY.md 里一条都没有：该文档由 scripts/doc/gen-api-registry.py
// 从网关 actuator.rs 的 ROUTES 静态表生成，而通配反代的下游路由不进那张表 ⇒ 注册表对治理台是盲的。
// 因此这里的路径/方法对齐由 governance-contract.test.js 直接解析 Rust 路由表来守，不查注册表。

export const ENDPOINTS = Object.freeze({
  dashboard: {
    key: 'dashboard',
    handler: 'dashboard_handler',
    method: 'GET',
    path: '/api/governance/dashboard',
    nesting: 'flat',
    // DashboardData.expertStates 是以维度为键的 map，不是数组 ⇒ 不走 unwrapList
    listKey: null
  },
  expertsStatus: {
    key: 'expertsStatus',
    handler: 'experts_status_handler',
    method: 'GET',
    path: '/api/governance/experts/status',
    nesting: 'flat',
    // 出参是 { businessLeague: { experts: [...] }, devLeague: { experts: [...] } } 两段
    listKey: 'experts'
  },
  vetoEvents: {
    key: 'vetoEvents',
    handler: 'veto_events_handler',
    method: 'GET',
    path: '/api/governance/veto/events',
    nesting: 'flat',
    listKey: 'events'
  },
  auditLogs: {
    key: 'auditLogs',
    handler: 'audit_logs_handler',
    method: 'GET',
    path: '/api/governance/audit/logs',
    nesting: 'flat',
    listKey: 'entries'
  },
  rbacConfig: {
    key: 'rbacConfig',
    handler: 'get_rbac_config_handler',
    method: 'GET',
    path: '/api/governance/config/rbac',
    nesting: 'flat',
    listKey: 'roles'
  },
  rbacConfigUpdate: {
    key: 'rbacConfigUpdate',
    handler: 'update_rbac_config_handler',
    method: 'PUT',
    path: '/api/governance/config/rbac',
    nesting: 'flat',
    listKey: 'roles'
  },
  expertConfig: {
    key: 'expertConfig',
    handler: 'get_expert_config_handler',
    method: 'GET',
    path: '/api/governance/config/experts',
    nesting: 'flat',
    // businessWeights / devWeights 都是维度为键的 map
    listKey: null
  },
  expertConfigUpdate: {
    key: 'expertConfigUpdate',
    handler: 'update_expert_config_handler',
    method: 'PUT',
    path: '/api/governance/config/experts',
    nesting: 'flat',
    listKey: null
  },
  governanceWs: {
    key: 'governanceWs',
    handler: 'governance_ws_handler',
    method: 'GET',
    path: '/api/governance/ws',
    nesting: 'flat',
    listKey: null
  },
  assess: {
    key: 'assess',
    handler: 'assess_handler',
    method: 'POST',
    path: '/api/governance/assess',
    nesting: 'flat',
    listKey: null
  }
})

/** 路由表里的 (方法, 路径) → 端点键。PUT 与 GET 共用路径，故按两字段联合查。 */
const BY_ROUTE = new Map(
  Object.values(ENDPOINTS).map((ep) => [`${ep.method} ${ep.path}`, ep.key])
)

export function endpointOfRoute(method, path) {
  return BY_ROUTE.get(`${method.toUpperCase()} ${path}`) ?? null
}

/**
 * 生成 http 实例可直接使用的相对路径（baseURL 已是 /api，见 `api/http.js:12`）。
 * `ep.path` 保留完整 wire 路径是**契约身份**（与 Rust 路由表逐字比对），不是请求 URL；
 * 直接把 path 交给 http 会打出 `/api/api/governance/…` ⇒ 404，
 * 而这一层被 vi.mock 掉时测试全绿，故剥前缀这件事必须留在这里并写死。
 * 与 `expert-alliance/contract/endpoints.js` 的 requestPath 同口径。
 * @param {string} name ENDPOINTS 键
 * @param {Record<string,string|number>} [params] 路径参数，键名与 :placeholder 一致
 */
export function requestPath(name, params = {}) {
  const ep = ENDPOINTS[name]
  if (!ep) throw new Error(`未知治理端点: ${name}`)
  return ep.path.replace(/:([A-Za-z_]+)/g, (_, k) => String(params[k] ?? `:${k}`)).replace(/^\/api/, '')
}

// ── 查询参数的键名：VetoQuery / AuditLogQuery 都带 #[serde(rename_all = "camelCase")]，
//    且全字段 Option ⇒ 发 snake_case 不会报错、只是**静默不过滤**。这类缺陷不响铃，故键名单源。
export const VETO_QUERY_KEYS = Object.freeze({
  page: 'page',
  pageSize: 'pageSize',
  flowId: 'flowId',
  expertId: 'expertId',
  dimension: 'dimension',
  fromTs: 'fromTs',
  toTs: 'toTs',
  blocked: 'blocked'
})

export const AUDIT_QUERY_KEYS = Object.freeze({
  page: 'page',
  pageSize: 'pageSize',
  flowId: 'flowId',
  subject: 'subject',
  action: 'action',
  fromTs: 'fromTs',
  toTs: 'toTs'
})

/// 后端把 page_size 夹到 `unwrap_or(20).min(200)` ⇒ 超过 200 不报错，只按 200 返回。
export const PAGE_SIZE_MAX = 200
export const PAGE_SIZE_DEFAULT = 20

/** 治理台列表查询：只发非空键，键名取自上面的单表（snake_case 一律不出门）。 */
export function vetoQuery(input = {}) {
  const q = {}
  if (input.page) q[VETO_QUERY_KEYS.page] = clampPage(input.page)
  if (input.pageSize) q[VETO_QUERY_KEYS.pageSize] = clampPageSize(input.pageSize)
  if (input.flowId) q[VETO_QUERY_KEYS.flowId] = input.flowId
  if (input.expertId) q[VETO_QUERY_KEYS.expertId] = input.expertId
  if (input.dimension) q[VETO_QUERY_KEYS.dimension] = input.dimension
  if (input.fromTs) q[VETO_QUERY_KEYS.fromTs] = input.fromTs
  if (input.toTs) q[VETO_QUERY_KEYS.toTs] = input.toTs
  // blocked 是三态：undefined=不过滤，true=只看在途拦截，false=只看放行
  if (input.blocked === true || input.blocked === false) q[VETO_QUERY_KEYS.blocked] = input.blocked
  return q
}

export function auditQuery(input = {}) {
  const q = {}
  if (input.page) q[AUDIT_QUERY_KEYS.page] = clampPage(input.page)
  if (input.pageSize) q[AUDIT_QUERY_KEYS.pageSize] = clampPageSize(input.pageSize)
  if (input.flowId) q[AUDIT_QUERY_KEYS.flowId] = input.flowId
  if (input.subject) q[AUDIT_QUERY_KEYS.subject] = input.subject
  if (input.action) q[AUDIT_QUERY_KEYS.action] = input.action
  if (input.fromTs) q[AUDIT_QUERY_KEYS.fromTs] = input.fromTs
  if (input.toTs) q[AUDIT_QUERY_KEYS.toTs] = input.toTs
  return q
}

const clampPage = (n) => Math.max(1, Math.floor(Number(n) || 1))
const clampPageSize = (n) => Math.min(PAGE_SIZE_MAX, Math.max(1, Math.floor(Number(n) || PAGE_SIZE_DEFAULT)))

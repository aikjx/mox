// 治理台契约守卫：所有 Rust 侧事实都在测试现场解析源码，不抄副本。
// 权威源：
//   platform/domains/platform/svc/mox-platform-orchestrator-svc/src/routes/governance.rs   （十条路由）
//   同包 src/handlers/governance.rs                                                          （DTO 与 json! 字面量）
//   platform/domains/ai/core/mox-ai-expert-core/src/govern/mod.rs                            （FlowStatus 词表）
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

import {
  AUDIT_QUERY_KEYS, ENDPOINTS, PAGE_SIZE_DEFAULT, PAGE_SIZE_MAX, VETO_QUERY_KEYS, auditQuery, vetoQuery
} from './endpoints'
import {
  BUSINESS_DIM_IDS, DEFAULT_THRESHOLDS, DEV_DIM_IDS, DIM_IDS, FLOW_STATUS, VETO_EVENT_THRESHOLD_LITERAL
} from './dimensions'
import {
  normAssessSummary, normAuditEntry, normAuditPage, normDashboard, normExpertConfig, normExpertsStatus,
  normExpertState, normGateResult, normRbacConfig, normVetoEvent, normVetoPage, secsToMs
} from '../model/normalize'
import { createGovernanceApi } from '../api/governance.api'

const HERE = path.dirname(fileURLToPath(import.meta.url))
const REPO = path.resolve(HERE, '../../../../..')
const ROUTES_RS = path.join(
  REPO,
  'platform/domains/platform/svc/mox-platform-orchestrator-svc/src/routes/governance.rs'
)
const HANDLERS_RS = path.join(
  REPO,
  'platform/domains/platform/svc/mox-platform-orchestrator-svc/src/handlers/governance.rs'
)
const GOVERN_RS = path.join(REPO, 'platform/domains/ai/core/mox-ai-expert-core/src/govern/mod.rs')

const MAIN_RS = path.join(
  REPO,
  'platform/domains/platform/svc/mox-platform-orchestrator-svc/src/main.rs'
)
const PROXY_RS = path.join(
  REPO,
  'platform/gateway/mox-platform-gateway-svc/src/proxy.rs'
)

// 请求 URL 的正确形状由**两方**共同决定：Rust 侧的 nest 前缀 + http 实例的 baseURL。
// 两者都是现场读，不抄副本：http.js 改了 baseURL，下面这一格会立刻红。
const HTTP_JS = path.join(REPO, 'frontend-ui/src/api/http.js')

// Rust 源为 CRLF：先归一为 LF，切片锚点（'\n}'）与行号才和磁盘一致。
const readSrc = (p) => readFileSync(p, 'utf-8').replace(/\r\n/g, '\n')

const routesSrc = readSrc(ROUTES_RS)
const handlersSrc = readSrc(HANDLERS_RS)
const governSrc = readSrc(GOVERN_RS)
const mainSrc = readSrc(MAIN_RS)
const proxySrc = readSrc(PROXY_RS)

// routes/governance.rs 里注册的是**相对**路径（/dashboard），完整对外路径 = nest 前缀 + 相对路径。
// 前缀现场取自 main.rs，不抄副本：改了 nest 而没改端点表，这一格会立刻红。
const NEST_PREFIX = (() => {
  const m = mainSrc.match(/\.nest\(\s*"([^"]*\/api\/governance)"/)
  expect(m, 'main.rs 里找不到 /api/governance 的 nest 前缀，端点表失去对齐目标').toBeTruthy()
  return m[1]
})()

// http 实例的 baseURL：交给 axios 的 url 必须是**去掉 baseURL 之后**的那一段。
const HTTP_BASE = (() => {
  const m = readSrc(HTTP_JS).match(/baseURL:\s*'([^']+)'/)
  expect(m, 'api/http.js 里读不到 baseURL 字面量，请求 URL 的形状失去对齐目标').toBeTruthy()
  return m[1]
})()

// ── Rust 解析工具 ───────────────────────────────────────────────────────────
const camel = (s) => s.replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase())

/** 结构体字段清单：返回 { wire, rust } 对；同时要求该结构体显式声明 rename_all = camelCase。 */
function structFields(name, { expectCamelCase = true } = {}) {
  const start = handlersSrc.indexOf(`pub struct ${name} {`)
  expect(start, `Rust 里找不到 struct ${name}，这一格会空转`).toBeGreaterThan(-1)
  const head = handlersSrc.slice(Math.max(0, start - 400), start)
  const isCamel = /rename_all\s*=\s*"camelCase"/.test(head)
  if (expectCamelCase) expect(isCamel, `struct ${name} 不再是 camelCase 重命名，前端的读法要跟着改`).toBe(true)
  const body = handlersSrc.slice(start, handlersSrc.indexOf('\n}', start))
  const out = []
  const re = /pub\s+([a-z0-9_]+)\s*:\s*([^,\n]+)/g
  let m
  while ((m = re.exec(body))) out.push({ rust: m[1], wire: camel(m[1]), type: m[2].trim() })
  expect(out.length, `struct ${name} 一个字段都没解析出来`).toBeGreaterThan(0)
  return out
}

/** 单个 handler 的源码切片 */
function handlerSrc(fnName) {
  const start = handlersSrc.indexOf(`pub async fn ${fnName}`)
  expect(start, `Rust 里找不到 handler ${fnName}`).toBeGreaterThan(-1)
  const next = handlersSrc.indexOf('\npub async fn', start + 10)
  return handlersSrc.slice(start, next === -1 ? handlersSrc.length : next)
}

/** 取 json! 字面量的顶层键（按花括号深度扫，嵌套对象的键不算） */
function topLevelJsonKeys(fnName) {
  const src = handlerSrc(fnName)
  const at = src.indexOf('json!({')
  expect(at, `${fnName} 的响应不是 json! 字面量`).toBeGreaterThan(-1)
  const keys = []
  let depth = 0
  let i = at + 'json!'.length
  for (; i < src.length; i++) {
    const c = src[i]
    if (c === '{') depth++
    else if (c === '}') {
      depth--
      if (depth === 0) break
    } else if (c === '"' && depth === 1) {
      const close = src.indexOf('"', i + 1)
      const key = src.slice(i + 1, close)
      // 只收形如 "key": 的顶层键
      if (/^[a-z_]+$/.test(key) && src.slice(close + 1, close + 2) === ':') keys.push(key)
      i = close
    }
  }
  return keys
}

/** Rust 里以数组字面量出现 N 次的那份十四维清单 */
function dimArraysIn(src) {
  const out = []
  const re = /\[\s*((?:"[a-z_]+",?\s*)+)\]/g
  let m
  while ((m = re.exec(src))) {
    const ids = [...m[1].matchAll(/"([a-z_]+)"/g)].map((x) => x[1])
    if (ids.length === 7) out.push(ids)
  }
  return out
}

// ── 断言 helper：wire 字段清单 ≡ fixture 键 ≡ 前端消费的映射 ────────────────
function pathOf(obj, dotted) {
  return dotted.split('.').reduce((o, k) => (o === undefined || o === null ? undefined : o[k]), obj)
}

/**
 * @param struct   Rust 结构体名
 * @param fixture  按 wire 键构造的样本，值必须唯一且非默认
 * @param map      wire 键 → 归一化结果的点路径，或 { path, xform }（xform 把 fixture 值换算成
 *                 期望的归一值：epoch 秒→毫秒、map 取某一维、嵌套对象取字段）；
 *                 值为 null 表示"前端确认不读"，必须写理由
 * @param fn       归一化函数
 */
function covers({ struct, fixture, map, fn, handlerCamelCase = true }) {
  const fields = structFields(struct, { expectCamelCase: handlerCamelCase })
  const wire = fields.map((f) => f.wire)
  expect(new Set(wire).size, `${struct} 的 wire 键有重复`).toBe(wire.length)
  // 双向：Rust 多一个字段 → 必须显式登记"不读"；前端多一个键 → 是幻影字段
  expect(Object.keys(map).sort(), `${struct}：前端消费清单与 Rust 字段清单不一致`).toEqual([...wire].sort())
  for (const key of Object.keys(fixture)) {
    expect(wire, `${struct}：fixture 里的 ${key} 不在 Rust 字段表里`).toContain(key)
  }
  for (const [key, spec] of Object.entries(map)) {
    if (spec === null) continue
    const dotted = typeof spec === 'string' ? spec : spec.path
    expect(dotted, `${struct}.${key}：消费映射必须给出归一化结果里的点路径`).toBeTruthy()
    const xform = typeof spec === 'string' ? (v) => v : (spec.xform ?? ((v) => v))
    const got = pathOf(fn({ ...fixture }), dotted)
    expect(got, `${struct}.${key} 没有落到 ${dotted}（读到的是兜底值 ⇒ 键名或口径错）`).toEqual(xform(fixture[key]))
  }
  return fields
}

describe('治理台契约 · 路由表', () => {
  it('十条路由逐枚点名，且方法/路径/handler 与端点表双向相等', () => {
    const re = /\.route\(\s*"([^"]+)"\s*,\s*(get|put|post|delete)\(gov::(\w+)\)/g
    const rows = [...routesSrc.matchAll(re)].map((m) => `${m[2].toUpperCase()} ${NEST_PREFIX}${m[1]} → ${m[3]}`)
    expect(rows.length, '路由表解析出的行数变了，守卫的分母要跟着改').toBe(10)
    const fromRust = new Set(rows)
    const fromFront = new Set(Object.values(ENDPOINTS).map((ep) => `${ep.method} ${ep.path} → ${ep.handler}`))
    expect([...fromFront].filter((x) => !fromRust.has(x)), '端点表里有 Rust 没挂的路由').toEqual([])
    expect([...fromRust].filter((x) => !fromFront.has(x)), 'Rust 挂了但端点表没登记（零消费者新债）').toEqual([])
  })

  it('本域确实挂在编排器 /api/governance 下，网关靠通配反代可达', () => {
    expect(mainSrc).toContain('.nest("/api/governance"')
    expect(NEST_PREFIX).toBe('/api/governance')
    expect(proxySrc).toContain('/api/governance/*')
    // 端点表的 path 必须以 nest 前缀开头，否则 requestPath 拼出来的就是打不通的 URL
    for (const ep of Object.values(ENDPOINTS)) expect(ep.path.startsWith(NEST_PREFIX), `${ep.key} 不在 nest 前缀下`).toBe(true)
  })

  it('发给 http 的 url 必须剥掉 baseURL：/api/api/… 是 404，而打桩的测试看不见这件事', () => {
    const calls = []
    const stub = { request: (cfg) => (calls.push(cfg), Promise.resolve({ code: 0, msg: 'ok', data: {} })) }
    const api = createGovernanceApi(stub)
    // 七个有出口的方法全跑一遍，分母钉死：少一个方法被接进来，这格会因数量变化而红
    return Promise.all([
      api.getDashboard(), api.getExpertsStatus(), api.listVetoEvents({ page: 1 }),
      api.listAuditLogs({ page: 1 }), api.getExpertConfig(), api.getRbacConfig(),
      api.assessFlow({ flowId: 'f', flowName: 'n', flow: {} })
    ]).then(() => {
      expect(calls.length, 'api 层出口方法数变了，本格的分母要跟着改').toBe(7)
      const rel = NEST_PREFIX.slice(HTTP_BASE.length) // '/governance'
      expect(rel).toBe('/governance')
      for (const c of calls) {
        expect(c.url.startsWith(HTTP_BASE + '/'), `${c.url} 把 baseURL 又拼了一遍`).toBe(false)
        expect(c.url.startsWith(rel), `${c.url} 不是 ${rel}/… 的形状`).toBe(true)
      }
      // 正对照：把 bug 形状（未剥前缀）喂给同一条判据，它必须判红
      const buggy = HTTP_BASE + calls[0].url
      expect(buggy.startsWith(HTTP_BASE + '/'), '判据打不中 /api/api 形状 ⇒ 这条禁令没牙').toBe(true)
    })
  })

  it('docs/API-REGISTRY.md 对本域零登记：这是已知缺口，不许让端点表改成查注册表', () => {
    const registry = readFileSync(path.join(REPO, 'docs/API-REGISTRY.md'), 'utf-8')
    const hits = Object.values(ENDPOINTS).filter((ep) => registry.includes(ep.path))
    // 现状是 0；若哪天注册表补了行，这格会红，提示改为"双向核对"而不是悄悄放过
    expect(hits.map((h) => h.path), '注册表已开始登记治理台路由，请收紧本格判据').toEqual([])
  })
})

describe('治理台契约 · 信封与 json! 字面量的两副键名', () => {
  it('api_ok 的信封是 {code,msg,data} 单层 ⇒ nesting 一律 flat', () => {
    const proto = readFileSync(path.join(REPO, 'platform/foundation/mox-api-protocol/src/lib.rs'), 'utf-8')
    const at = proto.indexOf('pub struct ApiResponse')
    const body = proto.slice(at, proto.indexOf('\n}', at))
    const keys = [...body.matchAll(/pub\s+([a-z_]+)\s*:/g)].map((m) => m[1])
    expect(keys.sort(), '信封键名变了，unwrap 的判据要跟着改').toEqual(['code', 'data', 'msg'])
    for (const ep of Object.values(ENDPOINTS)) expect(ep.nesting).toBe('flat')
  })

  it('experts/status 的顶层键是 snake_case（json! 字面量不吃 rename_all），数组元素才是 camelCase', () => {
    const keys = topLevelJsonKeys('experts_status_handler')
    expect(keys, '顶层出现重复键：serde_json 后写覆盖前写，前端读不到第一个值').toEqual(
      expect.arrayContaining(['business_league', 'dev_league'])
    )
    expect(keys.filter((k) => k === 'mox').length, 'mox 键在 Rust 侧写了两遍，界面上只余最后一个').toBe(2)
    // 前端读法：壳走 snake，行走 camel
    const normed = normExpertsStatus({
      timestamp: 1700000000,
      business_league: { dimensions: BUSINESS_DIM_IDS, experts: [], average_health: 0.7 },
      dev_league: { dimensions: DEV_DIM_IDS, experts: [], average_health: 0.8 }
    })
    expect(normed.business.averageHealth).toBe(0.7)
    expect(normed.dev.averageHealth).toBe(0.8)
    expect(normExpertsStatus({ businessLeague: {}, devLeague: {} }).business.dimensions).toEqual([])
  })

  it('veto/events 的分页壳是 snake（page_size/total_pages），audit/logs 的是 camel（struct 出参）', () => {
    expect(topLevelJsonKeys('veto_events_handler').sort()).toEqual([
      'events', 'page', 'page_size', 'total', 'total_pages'
    ])
    const audit = structFields('AuditLogResponse').map((f) => f.wire).sort()
    expect(audit).toEqual(['entries', 'page', 'pageSize', 'total', 'totalPages'])

    const veto = normVetoPage({ events: [], total: 0, page: 1, page_size: 20, total_pages: 0 })
    expect(veto.pageSize).toBe(20)
    expect(veto.totalPages).toBe(0)
    // 读错口径的表现就是"每页 undefined"，所以反面对照必须为 0
    expect(normVetoPage({ events: [], pageSize: 20, totalPages: 3 }).pageSize).toBe(0)

    const auditPage = normAuditPage({ entries: [], total: 0, page: 1, pageSize: 20, totalPages: 3 })
    expect(auditPage.pageSize).toBe(20)
    expect(auditPage.totalPages).toBe(3)
  })
})

describe('治理台契约 · 结构体字段逐枚落地', () => {
  it('VetoEvent', () => {
    covers({
      struct: 'VetoEvent',
      fixture: {
        id: 'evt-1',
        flowId: 'flow-9',
        flowName: '流程名',
        expertId: 'algorithm',
        dimension: 'algorithm',
        reason: '专家 algorithm 健康分 0.21 低于阈值 0.5',
        severity: 'critical',
        ts: 1700000000,
        blocked: true,
        gateResult: { status: 'Blocked', approved: false, slaOk: true, budgetOk: true, blockingRisks: 2, algorithmVeto: true, reason: 'gate' }
      },
      map: {
        id: 'id',
        flowId: 'flowId',
        flowName: 'flowName',
        expertId: 'expertId',
        dimension: 'dimension',
        reason: 'reason',
        severity: 'severity',
        ts: { path: 'tsMs', xform: secsToMs },
        blocked: 'blocked',
        gateResult: { path: 'gate.status', xform: (v) => v.status }
      },
      fn: normVetoEvent
    })
    // 派生键：dimLabel 必须由词表给出，不是把串再显一遍
    const row = normVetoEvent({ dimension: 'api_compat', severity: 'warning' })
    expect(row.dimLabel).toBe('接口兼容')
    expect(row.league).toBe('dev')
    expect(row.isCritical).toBe(false)
  })

  it('GateResultDto', () => {
    covers({
      struct: 'GateResultDto',
      fixture: {
        status: 'Approved',
        approved: true,
        slaOk: true,
        budgetOk: false,
        blockingRisks: 3,
        algorithmVeto: true,
        reason: 'r'
      },
      map: {
        status: 'status',
        approved: 'approved',
        slaOk: 'slaOk',
        budgetOk: 'budgetOk',
        blockingRisks: 'blockingRisks',
        algorithmVeto: 'algorithmVeto',
        reason: 'reason'
      },
      fn: normGateResult
    })
  })

  it('ExpertStatus', () => {
    covers({
      struct: 'ExpertStatus',
      fixture: {
        expertId: 'security',
        dimension: 'security',
        healthScore: 0.42,
        enabled: true,
        lastUpdated: 1700000001,
        vetoCount: 4,
        totalChecks: 9
      },
      map: {
        expertId: 'expertId',
        dimension: 'dimension',
        healthScore: 'healthScore',
        enabled: 'enabled',
        lastUpdated: { path: 'lastUpdatedMs', xform: secsToMs },
        vetoCount: 'vetoCount',
        totalChecks: 'totalChecks'
      },
      fn: (raw) => normExpertState(raw.dimension, raw)
    })
  })

  it('DashboardData', () => {
    covers({
      struct: 'DashboardData',
      fixture: {
        timestamp: 1700000002,
        totalFlows: 12,
        approvedFlows: 5,
        blockedFlows: 7,
        draftFlows: 3,
        reviewFlows: 1,
        vetoRate: 0.58,
        auditEventCount: 21,
        expertStates: { business: { expertId: 'business', dimension: 'business', healthScore: 0.9, enabled: true, lastUpdated: 1, vetoCount: 0, totalChecks: 1 } },
        recentVetoes: [{ id: 'a', dimension: 'business' }],
        auditChainVerified: true,
        businessLeagueHealth: 0.71,
        devLeagueHealth: 0.66
      },
      map: {
        timestamp: { path: 'serverTsMs', xform: secsToMs },
        totalFlows: 'counts.totalFlows',
        approvedFlows: 'counts.approvedFlows',
        blockedFlows: 'counts.blockedFlows',
        draftFlows: 'counts.draftFlows',
        reviewFlows: 'counts.reviewFlows',
        vetoRate: 'vetoRate',
        auditEventCount: 'auditEventCount',
        // expertStates / recentVetoes 在 wire 上分别是 map 与数组，归一层换成了有序行数组
        expertStates: { path: 'experts.0.healthScore', xform: (v) => Object.values(v)[0].healthScore },
        recentVetoes: { path: 'recentVetoes.0.id', xform: (v) => v[0].id },
        auditChainVerified: 'auditChainVerified',
        businessLeagueHealth: 'leagueHealth.business',
        devLeagueHealth: 'leagueHealth.dev'
      },
      fn: normDashboard
    })
    // expertStates 是 map：顺序由 DIM_IDS 决定，而不是 Object.values 的插入序
    const d = normDashboard({
      expertStates: {
        style: { dimension: 'style', healthScore: 0.5 },
        business: { dimension: 'business', healthScore: 0.6 }
      }
    })
    expect(d.experts.map((e) => e.dimension)).toEqual(['business', 'style'])
    expect(d.businessExperts.length).toBe(1)
    expect(d.devExperts.length).toBe(1)
  })

  it('AuditLogEntry 与 AuditLogResponse', () => {
    covers({
      struct: 'AuditLogEntry',
      fixture: {
        id: 'e1',
        ts: 1700000003,
        subject: 'governance-api',
        flowId: 'f1',
        action: 'mox_optimize',
        decision: 'blocked',
        prevHash: '0000',
        hash: 'abcd'
      },
      map: {
        id: 'id',
        ts: { path: 'tsMs', xform: secsToMs },
        subject: 'subject',
        flowId: 'flowId',
        action: 'action',
        decision: 'decision',
        prevHash: 'prevHash',
        hash: 'hash'
      },
      fn: normAuditEntry
    })
    structFields('AuditLogResponse')
  })

  it('ExpertConfig 与 ExpertThresholds：权重 map 按维度序展开，缺权重要报"未配置"', () => {
    const cfgFields = structFields('ExpertConfig').map((f) => f.wire)
    expect(cfgFields.sort()).toEqual(['businessWeights', 'devWeights', 'thresholds', 'updatedAt', 'updatedBy', 'version'])
    expect(structFields('ExpertThresholds').map((f) => f.wire).sort()).toEqual([
      'healthMin', 'vetoThreshold', 'warnThreshold'
    ])
    const c = normExpertConfig({
      version: 2,
      updatedAt: 1700000004,
      updatedBy: 'admin',
      businessWeights: { business: 1.5 },
      devWeights: { style: 0.5 },
      thresholds: { vetoThreshold: 0.3, warnThreshold: 0.6, healthMin: 0.5 }
    })
    expect(c.businessWeights[0]).toMatchObject({ dimension: 'business', present: true, weight: 1.5 })
    expect(c.businessWeights[1]).toMatchObject({ dimension: 'algorithm', present: false })
    expect(c.devWeights.find((r) => r.dimension === 'style')).toMatchObject({ present: true, weight: 0.5 })
    expect(c.thresholds).toMatchObject({ vetoThreshold: 0.3, warnThreshold: 0.6, healthMin: 0.5 })
  })

  it('RbacConfig 与 RolePermission', () => {
    expect(structFields('RbacConfig').map((f) => f.wire).sort()).toEqual(['roles', 'updatedAt', 'updatedBy', 'version'])
    expect(structFields('RolePermission').map((f) => f.wire).sort()).toEqual(['description', 'permissions', 'role'])
    const r = normRbacConfig({
      version: 1,
      updatedAt: 1700000005,
      updatedBy: 'system',
      roles: [{ role: 'auditor', permissions: ['governance:read'], description: '审计员' }]
    })
    expect(r.roles[0]).toMatchObject({ role: 'auditor', permissions: ['governance:read'] })
  })

  it('GovernanceReportSummary（assess 的响应）', () => {
    covers({
      struct: 'GovernanceReportSummary',
      fixture: {
        flowId: 'f',
        flowName: 'n',
        businessScores: { business: 0.9 },
        devScores: { style: 0.4 },
        businessLeagueScore: 0.9,
        devLeagueScore: 0.4,
        gateResult: { status: 'Blocked', approved: false, slaOk: false, budgetOk: true, blockingRisks: 1, algorithmVeto: true, reason: 'x' },
        adoptionCount: 2,
        suggestionCount: 3,
        moxPassed: false,
        ts: 1700000006
      },
      map: {
        flowId: 'flowId',
        flowName: 'flowName',
        // 分数在 wire 上是「维度 → 分数」的 map，归一层展开成按维度序的行
        businessScores: { path: 'businessScores.0.score', xform: (v) => Object.values(v)[0] },
        devScores: { path: 'devScores.0.score', xform: (v) => Object.values(v)[0] },
        businessLeagueScore: 'businessLeagueScore',
        devLeagueScore: 'devLeagueScore',
        gateResult: { path: 'gate.status', xform: (v) => v.status },
        adoptionCount: 'adoptionCount',
        suggestionCount: 'suggestionCount',
        moxPassed: 'moxPassed',
        ts: { path: 'tsMs', xform: secsToMs }
      },
      fn: normAssessSummary
    })
  })
})

describe('治理台契约 · 查询参数与分页', () => {
  it('VetoQuery / AuditLogQuery 都是 camelCase，键名单源', () => {
    expect(structFields('VetoQuery').map((f) => f.wire).sort()).toEqual(
      ['blocked', 'dimension', 'expertId', 'flowId', 'fromTs', 'page', 'pageSize', 'toTs'].sort()
    )
    expect(structFields('AuditLogQuery').map((f) => f.wire).sort()).toEqual(
      ['action', 'flowId', 'fromTs', 'page', 'pageSize', 'subject', 'toTs'].sort()
    )
    expect(Object.values(VETO_QUERY_KEYS).sort()).toEqual(structFields('VetoQuery').map((f) => f.wire).sort())
    expect(Object.values(AUDIT_QUERY_KEYS).sort()).toEqual(structFields('AuditLogQuery').map((f) => f.wire).sort())
  })

  it('发 snake_case 会被静默忽略 ⇒ 查询构造器只产 camelCase，且不发空键', () => {
    const q = vetoQuery({ flowId: 'f-1', pageSize: 500, page: 3, dimension: 'style', blocked: false, fromTs: 1 })
    expect(Object.keys(q).sort()).toEqual(['blocked', 'dimension', 'flowId', 'fromTs', 'page', 'pageSize'].sort())
    expect(q.pageSize).toBe(PAGE_SIZE_MAX)
    expect(q.page).toBe(3)
    expect(q.blocked).toBe(false)
    expect('flow_id' in q).toBe(false)
    // page:0 与 page:1 在后端同解（unwrap_or(1)），构造器选择不发这一键而不是发 1
    expect(vetoQuery({ page: 0 })).toEqual({})
    expect(vetoQuery({ page: -5 })).toEqual({ page: 1 })
    expect(auditQuery({})).toEqual({})
    expect(vetoQuery({ blocked: undefined })).toEqual({})
    // 三态：undefined 不过滤 / false 是有效值，不许被当成空丢掉
    expect(vetoQuery({ blocked: true })).toEqual({ blocked: true })
    expect(auditQuery({ pageSize: 5000 })).toEqual({ pageSize: PAGE_SIZE_MAX })
  })

  it('分页夹逼常量来自 Rust 源码：unwrap_or(20).min(200)', () => {
    const veto = handlerSrc('veto_events_handler')
    expect(veto).toContain('query.page_size.unwrap_or(20).min(200)')
    expect(PAGE_SIZE_MAX).toBe(200)
    expect(PAGE_SIZE_DEFAULT).toBe(20)
  })

  it('AssessRequest 三键：flow 无 serde 缺省 ⇒ 缺一键就是 422，api 层必须整张图透传', () => {
    const fields = structFields('AssessRequest')
    expect(fields.map((f) => f.wire).sort()).toEqual(['flow', 'flowId', 'flowName'])
    const calls = []
    const stub = { request: (cfg) => (calls.push(cfg), Promise.resolve({ code: 0, msg: 'ok', data: {} })) }
    const api = createGovernanceApi(stub)
    return api.assessFlow({ flowId: 'f', flowName: 'n', flow: { nodes: [] } }).then(() => {
      expect(calls[0].data).toEqual({ flowId: 'f', flowName: 'n', flow: { nodes: [] } })
      // 期望值不从 requestPath 反推（那是自己证自己，双前缀 bug 就是这样全绿通过的）
      expect(calls[0].url).toBe(`${NEST_PREFIX.slice(HTTP_BASE.length)}/assess`)
      expect(calls[0].method).toBe('POST')
    })
  })
})

describe('治理台契约 · 十四维与词表', () => {
  it('Rust 里三处七维数组手抄本 + 前端表 = 四份，必须逐字符一致，分母 14', () => {
    // ① experts_status_handler ② trigger_governance ③ ExpertConfig::default（走 business/dev 两张权重表的键）
    const a = dimArraysIn(handlerSrc('experts_status_handler'))
    const b = dimArraysIn(handlerSrc('trigger_governance'))
    expect(a.length, 'experts_status_handler 里没解析出两个七维数组').toBe(2)
    expect(b.length, 'trigger_governance 里没解析出两个七维数组').toBe(2)
    expect(a[0]).toEqual(BUSINESS_DIM_IDS)
    expect(a[1]).toEqual(DEV_DIM_IDS)
    expect(b[0]).toEqual(BUSINESS_DIM_IDS)
    expect(b[1]).toEqual(DEV_DIM_IDS)
    expect(DIM_IDS.length).toBe(14)
    expect(new Set(DIM_IDS).size).toBe(14)
    expect([...BUSINESS_DIM_IDS, ...DEV_DIM_IDS].sort()).toEqual([...DIM_IDS].sort())
  })

  it('权重表键集合与十四维一致（ExpertConfig::default 的第三份手抄）', () => {
    const at = handlersSrc.indexOf('impl Default for ExpertConfig')
    expect(at, '找不到 ExpertConfig::default，这一格会空转').toBeGreaterThan(-1)
    // 注意：Rust 源是 CRLF，切片锚点只能用 '\n}'（'\n}\n' 在这份文件上匹配不到，会一路取到文件尾）
    const def = handlersSrc.slice(at, handlersSrc.indexOf('\n}', at))
    // 只取 `for dim in &[...]` 的维度数组：整段里还有 "system".to_string() 之类的字符串，
    // 按"所有引号串"去数会把 updated_by 的值当成第十五维。
    const arrays = dimArraysIn(def)
    expect(arrays.length, 'default 权重表里没解析出两个七维数组').toBe(2)
    expect(arrays[0]).toEqual(BUSINESS_DIM_IDS)
    expect(arrays[1]).toEqual(DEV_DIM_IDS)
    expect([...new Set([...arrays[0], ...arrays[1]])].sort()).toEqual([...DIM_IDS].sort())
  })

  it('否决条件是字面量 0.5，不读配置 ⇒ 界面文案不许暗示"改阈值即改否决"', () => {
    const trig = handlerSrc('trigger_governance')
    expect(trig).toContain(`*score < ${VETO_EVENT_THRESHOLD_LITERAL}`)
    expect(trig).toContain('低于阈值')
    expect(trig).toContain(`if *score < 0.3`)
    expect(VETO_EVENT_THRESHOLD_LITERAL).toBe(0.5)
    // 配置里的 vetoThreshold 缺省值不等于否决判定用的 0.5，两者是两个数
    expect(DEFAULT_THRESHOLDS.vetoThreshold).toBe(0.3)
  })

  it('FlowStatus 词表逐枚点名，severity 只有两档', () => {
    const at = governSrc.indexOf('pub enum FlowStatus')
    const body = governSrc.slice(at, governSrc.indexOf('\n}', at))
    const variants = [...body.matchAll(/^\s{4}([A-Z][A-Za-z]+),/gm)].map((m) => m[1])
    expect(variants.sort()).toEqual([...FLOW_STATUS].sort())
    const sev = [...handlerSrc('trigger_governance').matchAll(/"([a-z]+)"\.to_string\(\)/g)].map((m) => m[1])
    expect(new Set(sev)).toEqual(new Set(['critical', 'warning']))
  })
})

describe('治理台契约 · 时间是 epoch 秒', () => {
  it('unix_ts() 用 as_secs()，归一层必须乘 1000 才接得上 utils/time', () => {
    expect(handlersSrc).toContain('as_secs()')
    expect(secsToMs(1700000000)).toBe(1700000000000)
    expect(secsToMs(1700000000000)).toBe(1700000000000) // 已是毫秒不重复放大
    expect(Number.isNaN(secsToMs(0))).toBe(true)
    expect(Number.isNaN(secsToMs(null))).toBe(true)
    // 不乘 1000 的话会落到 1970：这条断言钉住"界面显示的年份由归一层负责"
    const row = normExpertState('business', { healthScore: 0.9, lastUpdated: 1700000000 })
    expect(new Date(row.lastUpdatedMs).getFullYear()).toBe(2023)
  })
})

describe('治理台契约 · 禁令棘轮（本模块源码自身）', () => {
  const MODULE_DIR = path.resolve(HERE, '..')
  const SCAN = ['contract/endpoints.js', 'contract/dimensions.js', 'model/normalize.js', 'api/governance.api.js', 'store/governance.store.js', 'views/GovernanceConsoleView.vue', 'index.js']
  const srcOf = (rel) => readFileSync(path.join(MODULE_DIR, rel), 'utf-8')
  const codeOnly = (t) => t.split('\n').filter((l) => !/^\s*(\/\/|\*|\/\*|<!--)/.test(l)).join('\n')

  // 每条禁令自带正对照：一个文件一条目标行。正对照缺失本身就是失败，
  // 否则"禁令只钉住了恰好写得像样例的那个文件"会伪装成全绿。
  const BANNED = [
    {
      id: 'wire-snake-outside-model',
      re: /\.(health_score|veto_count|total_checks|last_updated|business_league|dev_league|average_health|page_size|total_pages)\b/g,
      why: '视图/store/api 只许消费归一化后的 camel 模型；snake wire 键只允许出现在 model/normalize.js 一处',
      files: ['views/GovernanceConsoleView.vue', 'store/governance.store.js', 'api/governance.api.js'],
      pos: {
        'views/GovernanceConsoleView.vue': '<span>{{ row.veto_count }}</span>',
        'store/governance.store.js': 'const n = raw.total_pages',
        'api/governance.api.js': 'const ps = body.page_size'
      }
    },
    {
      id: 'json-shell-read-as-camel',
      re: /\.businessLeague\b|\.devLeague\b|\.averageHealth\b/g,
      why: 'experts/status 的外壳出自 serde_json::json! 字面量，不吃 rename_all ⇒ 只有这一层是 snake，读成 camel 会静默 undefined',
      files: ['model/normalize.js'],
      pos: { 'model/normalize.js': 'const sec = payload.businessLeague' }
    },
    {
      id: 'list-key-candidates',
      re: /unwrapList\(/g,
      why: '本域出参是结构体或 map，列表键逐端点名，不给候选表兜底的机会',
      files: SCAN,
      pos: Object.fromEntries(SCAN.map((f) => [f, "const rows = unwrapList(payload, 'items')"]))
    },
    {
      id: 'element-plus-root-import',
      re: /from\s+'element-plus'/g,
      why: '根导入会废掉 tree-shake（FE-MOD-GOV §2.2）',
      files: SCAN,
      // 样例串必须运行时可命中，又不能以字面量出现在本文件里 —— 否则模块化门禁 E1
      // 会把我自己的正对照当成真违规（拼接串对它是隐形的）。
      pos: Object.fromEntries(SCAN.map((f) => [f, `import { ElMessage } from 'element-${''}plus'`]))
    },
    {
      id: 'cross-dir-relative-import',
      re: /from\s+['"]\.\.\//g,
      why: '跨目录相对导入：模块内一律走 @/modules/governance/…（门禁 E2）',
      files: SCAN,
      pos: Object.fromEntries(SCAN.map((f) => [f, "import { dimLabel } from '../contract/dimensions'"]))
    }
  ]

  it('每条禁令都必须点名自己的目标行（正对照），且合法写法不吃（反对照）', () => {
    for (const ban of BANNED) {
      for (const rel of ban.files) {
        const sample = ban.pos[rel]
        expect(sample, `禁令 ${ban.id} 缺 ${rel} 的正对照样例（没样例的禁令等于没牙）`).toBeTruthy()
        ban.re.lastIndex = 0
        expect(ban.re.test(sample), `禁令失能：${ban.id} 认不出自己的样例（${ban.why}）`).toBe(true)
        const text = codeOnly(srcOf(rel))
        ban.re.lastIndex = 0
        expect(ban.re.test(text), `违反禁令 ${ban.id}：${rel} —— ${ban.why}`).toBe(false)
      }
    }
    // 反面对照：真被禁的写法确实会红（把样例塞进被测文本再跑一次）
    const poisoned = codeOnly(srcOf('model/normalize.js')) + '\nconst x = payload.businessLeague'
    BANNED[1].re.lastIndex = 0
    expect(BANNED[1].re.test(poisoned), '禁令可以删掉而不被发现').toBe(true)
  })

  it('json! 顶层的 snake 键只许出现在被点名的五处，多一处就是新债', () => {
    const text = codeOnly(srcOf('model/normalize.js'))
    const hits = [...text.matchAll(/'?(business_league|dev_league|average_health|page_size|total_pages)'?/g)].map((m) => m[1])
    expect(hits.length).toBe(5)
    expect([...new Set(hits)].sort()).toEqual(['average_health', 'business_league', 'dev_league', 'page_size', 'total_pages'])
  })
})

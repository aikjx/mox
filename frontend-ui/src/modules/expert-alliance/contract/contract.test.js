// 跨语言一致性门禁：前端契约必须与 Rust 权威源、docs/API-REGISTRY.md 逐字对齐。
// 后端改枚举/改路由而没同步前端时，本文件先红。
import { describe, it, expect } from 'vitest'
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs'
import path from 'node:path'

import { PHASE_IDS, AUDIT_EVENTS_7, PHASE_META } from './phases.js'
import { TASK_STATUS, NODE_STATUS, NODE_STATUS_LABELS, PRIORITY, MODE_WIRE, MODE_DISPLAY, FUSION_STRATEGY, GATE_THRESHOLDS, gradeOf, modeToDisplay, EXPERT_AVAILABILITY, EXPERT_TYPE, PRICING_MODEL, VERIFICATION_STATUS, BOOKING_STATUS, EXPERT_SORT_LABELS, canCancelBooking, expertStatsCells, expertDerivedCells, isConsultable, pricingText } from './enums.js'
import { ENDPOINTS, FORBIDDEN_ENDPOINTS, UNMOUNTED_ROUTES, unmountedByVerdict, EXPERT_QUERY_KEYS, EXPERT_SORT, expertListQuery, requestPath } from './endpoints.js'
import {
  COLLAB_MODE, COLLAB_MODES, COLLAB_LAZY_FIELDS, DEBATE_MIN_PARTICIPANTS, DEBATE_PARTICIPANT_CAP,
  collabAccepts, collabBody, collabMode, collabProblem, consensusText, answerSourceText, confidenceText,
  debateCapacityText, intentLabel, INTENT_LABELS, COMPLEXITY_ORDER, complexityLevel
} from './collab.js'
import { unwrap } from '../../_kernel/envelope.js'
import { RANK_BOARDS, buildBoard } from '../model/rank.js'
import {
  normTask, normDag, normFusion, normLogEntry, normExecutionStatus, normRuntime,
  normExpert, normExpertStats, normCapabilities, normExpertMetrics, normBooking, normBookingList, normBookingCancel, normConsultNow, normConsultRoom, normTeamApplication,
  normRouteResult, normSingleConsult, normMultiConsult, normDebate, normIntelligentConsult, normAlgorithmAnalysis,
  collabRefId, collabOutcome, collabQuestionText, normDispatch, normToggleDone
} from '../model/normalize.js'
import { DISPATCH_STRATEGY, dispatchRunBody, dispatchRunFindings, dispatchRunProblem } from './dispatcher.js'
import {
  EXPERT_ALIAS_FIELDS, EXPERT_PROFICIENCY_MAX, EXPERT_REGISTER_FIELDS, EXPERT_U32_MAX, EXPERT_UNMOUNTED_FIELDS,
  deleteConsequences, expertDraftProblem, expertFieldProblem, EXPERT_WRITE_IDENTITY
} from './registry.js'

// vitest 下 import.meta.url 非 file: 协议，改为向上定位仓库根
function findRepoRoot(from) {
  let dir = from
  for (let i = 0; i < 8; i++) {
    if (existsSync(path.join(dir, 'docs/API-REGISTRY.md')) && existsSync(path.join(dir, 'platform'))) return dir
    const parent = path.dirname(dir)
    if (parent === dir) break
    dir = parent
  }
  throw new Error(`未找到仓库根（自 ${from} 向上）`)
}

const ROOT = findRepoRoot(process.cwd())
const MODULE_DIR = path.join(ROOT, 'frontend-ui/src/modules/expert-alliance')
const src = (rel) => readFileSync(path.join(ROOT, rel), 'utf8').replace(/\r\n/g, '\n')

const EVENT_RS = src('platform/shared/mox-unified-contract/src/event.rs')
const QUALITY_RS = src('platform/shared/mox-unified-contract/src/quality.rs')
const NAMING_RS = src('platform/domains/alliance/proto/mox-alliance-common-proto/src/naming.rs')
const TYPES_RS = src('platform/domains/alliance/proto/mox-alliance-common-proto/src/types.rs')
const ALLIANCE_RS = src('platform/domains/alliance/sdk/mox-alliance-http-sdk/src/alliance.rs')
const EXPERTS_COMMON_RS = src('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_common.rs')
const EXPERTS_EXT_RS = src('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_ext.rs')
const EXPERTS_REGISTRY_RS = src('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_registry.rs')
const EXPERTS_DISPATCHER_RS = src('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_dispatcher.rs')
const ALLIANCE_REMOTE_RS = src('platform/domains/alliance/sdk/mox-alliance-http-sdk/src/alliance_remote.rs')
const REGISTRY = src('docs/API-REGISTRY.md')

function rustStrArray(name) {
  const m = EVENT_RS.match(new RegExp(`pub const ${name}: \\[&str; \\d+\\] = \\[([\\s\\S]*?)\\];`))
  if (!m) throw new Error(`Rust 源缺少常量 ${name}`)
  return [...m[1].matchAll(/"([^"]+)"/g)].map((x) => x[1])
}

function modeArms(fnName) {
  const m = NAMING_RS.match(new RegExp(`pub fn ${fnName}\\(m: AllianceMode\\)([\\s\\S]*?)\\n\\}`))
  if (!m) throw new Error(`naming.rs 缺少函数 ${fnName}`)
  return [...m[1].matchAll(/=> "([^"]+)"/g)].map((x) => x[1])
}

function fusionVariants() {
  const m = TYPES_RS.match(/pub enum FusionStrategy \{([\s\S]*?)\n\}/)
  if (!m) throw new Error('types.rs 缺少 FusionStrategy')
  return [...m[1].matchAll(/^\s{4}([A-Z]\w*),$/gm)].map((x) => x[1])
}

const pascalToSnake = (s) => s.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toLowerCase()

// 专家域枚举在 Rust 侧只是 String 字段，取值清单写在 /// 注释里（experts_common.rs），
// 注释一改本测试即红，避免前端注释与后端定义悄悄分叉。
function docChoices(rustSrc, first) {
  const m = rustSrc.match(new RegExp(`///[^\\n]*?\\b(${first}(?:\\s*/\\s*[a-z_]+)+)`))
  if (!m) throw new Error(`Rust 源缺少取值清单（以 ${first} 开头的 /// 注释）`)
  return m[1].split('/').map((s) => s.trim())
}

describe('联盟阶段契约 ↔ event.rs', () => {
  it('PHASE_IDS 与 PHASE_NAMES 逐项等序', () => {
    expect(PHASE_IDS).toEqual(rustStrArray('PHASE_NAMES'))
  })

  it('AUDIT_EVENTS_7 与 Rust 同名等序', () => {
    expect([...AUDIT_EVENTS_7]).toEqual(rustStrArray('AUDIT_EVENTS_7'))
  })

  it('阶段元数据覆盖全部阶段，审计名不引入 Rust 之外的值', () => {
    expect(Object.keys(PHASE_META)).toEqual([...PHASE_IDS])
    const declared = Object.values(PHASE_META).map((m) => m.audit).filter(Boolean)
    for (const a of declared) expect(AUDIT_EVENTS_7).toContain(a)
  })
})

describe('联盟枚举 ↔ Rust', () => {
  it('任务状态线名全部出现在 alliance.rs 归一表', () => {
    for (const v of Object.values(TASK_STATUS)) expect(ALLIANCE_RS).toContain(`"${v}"`)
  })

  it('节点状态取自 NodeExecStatus，ready 在 proto 侧', () => {
    for (const v of Object.values(NODE_STATUS)) {
      const hit = ALLIANCE_RS.includes(`"${v}"`) || TYPES_RS.includes(`"${v}"`) || /enum NodeStatus \{[\s\S]*?\}\)/.test(TYPES_RS)
      expect(hit, `节点状态 ${v} 未在 Rust 源找到`).toBe(true)
    }
  })

  it('优先级四档与 Rust 一致', () => {
    for (const v of Object.values(PRIORITY)) expect(ALLIANCE_RS).toContain(`"${v}"`)
  })

  it('MODE_WIRE 即 mode_serde，MODE_DISPLAY 即 mode_display，两者不可混用', () => {
    expect(Object.values(MODE_WIRE)).toEqual(modeArms('mode_serde'))
    expect(Object.values(MODE_DISPLAY)).toEqual(modeArms('mode_display'))
    expect(modeToDisplay('parallel')).toBe('expert_alliance')
    expect(modeToDisplay('sequential')).toBe('single_expert')
    expect(modeToDisplay('expert_alliance')).toBe('expert_alliance')
  })

  it('融合策略集合与 FusionStrategy 变体一一对应', () => {
    expect(Object.values(FUSION_STRATEGY)).toEqual(fusionVariants().map(pascalToSnake))
  })

  it('门禁阈值与 GATE_THRESHOLDS 一致并正确分级', () => {
    const m = QUALITY_RS.match(/GATE_THRESHOLDS: GateThresholds = GateThresholds \{\s*a: ([\d.]+),\s*b: ([\d.]+),\s*c: ([\d.]+),/)
    expect(m).not.toBeNull()
    expect(GATE_THRESHOLDS).toEqual({ A: Number(m[1]), B: Number(m[2]), C: Number(m[3]) })
    expect(gradeOf(0.95)).toBe('A')
    expect(gradeOf(0.9)).toBe('A')
    expect(gradeOf(0.85)).toBe('B')
    expect(gradeOf(0.7)).toBe('C')
    expect(gradeOf(0.69)).toBe('D')
    expect(gradeOf(null)).toBeNull()
  })
})

describe('端点清单 ↔ docs/API-REGISTRY.md', () => {
  const registered = new Set([...REGISTRY.matchAll(/`(\/api\/[^`]+)`/g)].map((m) => m[1]))

  it('每个端点路径都在 API-REGISTRY 登记', () => {
    for (const [name, ep] of Object.entries(ENDPOINTS)) {
      expect(registered.has(ep.path), `${name} → ${ep.path} 未登记`).toBe(true)
    }
  })

  it('每个端点都挂着可回溯的 registry 行 ID', () => {
    for (const [name, ep] of Object.entries(ENDPOINTS)) {
      expect(REGISTRY.includes(`\`${ep.registry}\``), `${name} 的 registry id ${ep.registry} 不存在`).toBe(true)
    }
  })

  // 全维覆盖台账：注册表两域的每一行要么已被模块挂载，要么在 UNMOUNTED_ROUTES 里定性并给出后端证据。
  // 「还有哪些面没做」因此不靠记忆，也不靠文档措辞。
  const ROWS = new Map(
    [...REGISTRY.matchAll(/^\| `(alliance\.[a-z_.]+|experts\.[a-z_.]+)` \| (\w+) \| `([^`]+)`/gm)]
      .map((m) => [m[1], { method: m[2], path: m[3] }])
  )
  const MOUNTED_IDS = new Set(Object.values(ENDPOINTS).map((ep) => ep.registry))
  const LEDGER_IDS = UNMOUNTED_ROUTES.map((x) => x.registry)

  // 接线扫描：判定「ENDPOINTS 里声明过、api 层也给了方法，但模块里没有任何调用点」的面。
  // 放在 describe 顶层而不是单个 it 里，是因为覆盖率必须按"真的被调到"计数：
  // 否则下架一个二手面会让分子凭空掉一格，门禁就会诱使人把假面留着充数。
  function scanWiring() {
    const files = []
    const walk = (d) => {
      for (const entry of readdirSync(d)) {
        const full = path.join(d, entry)
        if (statSync(full).isDirectory()) walk(full)
        else if (/\.(js|vue)$/.test(full) && !full.endsWith('endpoints.js') && !/\.test\.js$/.test(full)) files.push(full)
      }
    }
    walk(MODULE_DIR)
    const apiFile = path.join(MODULE_DIR, 'api', 'alliance.api.js')
    const apiText = readFileSync(apiFile, 'utf8').replace(/\r\n/g, '\n')
    const outside = files.filter((f) => f !== apiFile).map((f) => readFileSync(f, 'utf8').replace(/\r\n/g, '\n'))
    // api 工厂内的方法块：4 空格缩进的 `name(` 或 `async name(`，块尾取下一个同级方法
    const starts = [...apiText.matchAll(/^ {4}(?:async )?([A-Za-z]\w*)\(/gm)]
    const methods = starts.map((m, i) => ({
      name: m[1],
      body: apiText.slice(m.index, i + 1 < starts.length ? starts[i + 1].index : apiText.length)
    }))
    const unwired = []
    for (const name of Object.keys(ENDPOINTS)) {
      if (outside.some((t) => t.includes(`'${name}'`) || t.includes(`"${name}"`))) continue
      const owners = methods.filter((mm) => mm.body.includes(`'${name}'`))
      const reached = owners.some((mm) => outside.some((t) => new RegExp(`[.\`'"( ]${mm.name}[.\`'"( ]`, '').test(t)))
      if (!reached) unwired.push(name)
    }
    return { unwired, methods, outsideCount: outside.length }
  }
  const WIRING = scanWiring()
  /** 一个注册表 id 只要有任一同源端点真被调到就算覆盖（详情/更新/删除共用 detail 行） */
  const WIRED_IDS = new Set(
    [...MOUNTED_IDS].filter((id) => Object.entries(ENDPOINTS)
      .some(([name, ep]) => ep.registry === id && !WIRING.unwired.includes(name)))
  )

  it('两域注册表行非空且 id 唯一', () => {
    expect(ROWS.size).toBeGreaterThan(60)
    expect(LEDGER_IDS.length).toBe(new Set(LEDGER_IDS).size)
  })

  it('每个 ready 行要么被挂载，要么在台账里定性', () => {
    const unmounted = [...ROWS.keys()].filter((id) => !MOUNTED_IDS.has(id)).sort()
    expect(unmounted).toEqual([...LEDGER_IDS].sort())
    for (const id of MOUNTED_IDS) expect(ROWS.has(id), `${id} 不属于两域注册表`).toBe(true)
  })

  it('台账条目只收真实行，理由须指向后端源码位置', () => {
    for (const x of UNMOUNTED_ROUTES) {
      expect(['backlog', 'rejected'], `${x.registry} 定性非法：${x.verdict}`).toContain(x.verdict)
      expect(ROWS.has(x.registry), `${x.registry} 不是注册表里的 id（后端已删？连条一起删）`).toBe(true)
      expect(x.reason.length, `${x.registry} 理由不足`).toBeGreaterThan(30)
      expect(x.reason, `${x.registry} 理由没给后端位置`).toMatch(/\.rs[:\d]/)
    }
  })

  it('待办面数量只减不增（新增待办要先说明理由）', () => {
    expect(unmountedByVerdict('rejected').length + unmountedByVerdict('backlog').length).toBe(UNMOUNTED_ROUTES.length)
    // 最后一个 backlog 面（编排台）已于本批挂载：从此 backlog 归零，再出现一条就是倒退，
    // 必须先挂载或举证它不该挂（不再有"排队中"这一档可躲）。
    expect(unmountedByVerdict('backlog'), 'backlog 又长出条目').toEqual([])
    // 分子是 WIRED_IDS（真被调到的注册表 id），不是 MOUNTED_IDS（含"声明未接线"的假面）。
    // 实测 2026-09-25：63 / 74 = 0.8514。假面清零后两个数字才相等：下架 tasks.node 与 tasks.status_poll
    // 之前 MOUNTED 为 65（其中两格是"api 有方法、界面 never calls"的二手面）。
    expect(WIRED_IDS.size / ROWS.size, '覆盖率倒退').toBeGreaterThanOrEqual(0.85)
    // 这条差值断言才是"分子按真实接线"的证明：若有人把分子换回 MOUNTED_IDS，或往 ENDPOINTS 里
    // 塞一条没有调用者的独名面，差值即非 0 而红（覆盖率本身在 0.85 门上看不出来）。
    // expertDetail 与 update/delete 同源一行，不构成本差值——它由 DECLARED_NOT_WIRED 那条账看着。
    expect(MOUNTED_IDS.size - WIRED_IDS.size, '声明未接线且独名的注册表 id 数').toBe(0)
  })

  it('已挂载端点的方法与路径逐字对齐注册表行（ANY 行放行任意方法）', () => {
    for (const [name, ep] of Object.entries(ENDPOINTS)) {
      const row = ROWS.get(ep.registry)
      expect(row, `${name} 的行 ${ep.registry} 不在两域注册表`).toBeTruthy()
      expect(row.path, `${name} 路径与注册行不符`).toBe(ep.path)
      expect(row.method === ep.method || row.method === 'ANY', `${name} 方法 ${ep.method} 与注册行 ${row.method} 不符`).toBe(true)
    }
  })

  it('禁用端点在模块代码中不得复活（endpoints.js 的禁用清单除外）', () => {
    const dir = MODULE_DIR
    const files = []
    const walk = (d) => {
      for (const entry of readdirSync(d)) {
        const full = path.join(d, entry)
        if (statSync(full).isDirectory()) walk(full)
        // 门禁自身要引用禁用路径才能断言它仍在原位，测试文件按文字出现不算调用
        else if (/\.(js|vue)$/.test(full) && !full.endsWith('endpoints.js') && !/\.test\.js$/.test(full)) files.push(full)
      }
    }
    walk(dir)
    expect(files.length).toBeGreaterThan(0)
    for (const f of files) {
      const text = readFileSync(f, 'utf8')
      for (const bad of FORBIDDEN_ENDPOINTS) {
        expect(text.includes(bad.path), `${path.relative(dir, f)} 引用了禁用端点 ${bad.path}`).toBe(false)
      }
    }
  })

  // 台账的权威源 docs/API-REGISTRY.md 自身在漏登记：实测 Rust 里有 3 条路由不在文档上
  // （flow-graph / tasks/:id/qa / ai/expert-chat），前两条既无注册行也就进不了本 describe 的
  // 双向等集 ⇒ 第四种状态"谁都没提过"。本用例绕开文档，直接从 Rust 源取路由全集记账。
  const RUST_ROUTE_DIRS = [
    'platform/gateway/mox-platform-gateway-svc/src/alliance',
    'platform/domains/alliance/sdk/mox-alliance-http-sdk/src'
  ]
  // 文档欠登记、模块也尚未定性的 Rust 路由：钉成明账，只减不增（挂载或后端删除后连条一起删）。
  const DOC_UNREGISTERED_PENDING = Object.freeze([
    '/api/ai/engine/flow-graph',
    '/api/alliance/tasks/:id/qa',
    // T4 webhook CRUD：后端 experts_streams.rs 已真实落地并经 E2E 验证，
    // 但属运维管理面，本前端模块不挂 UI（接入留给独立运维控制台），暂欠登记。
    '/api/alliance/events/webhooks',
    '/api/alliance/events/webhooks/:id'
  ])

  it('Rust 侧每条联盟/专家路由都在前端四态账上（挂载/禁用/待办/欠登记）', () => {
    const rustFiles = []
    const walk = (d) => {
      for (const entry of readdirSync(d)) {
        const full = path.join(d, entry)
        if (statSync(full).isDirectory()) walk(full)
        else if (full.endsWith('.rs')) rustFiles.push(full)
      }
    }
    for (const rel of RUST_ROUTE_DIRS) walk(path.join(ROOT, rel))
    expect(rustFiles.length, 'Rust 源目录疑似扫空').toBeGreaterThan(10)

    const routes = new Set()
    for (const f of rustFiles) {
      const text = readFileSync(f, 'utf8').replace(/\r\n/g, '\n')
      for (const m of text.matchAll(/\.route\("(\/api\/[^"]+)"/g)) routes.add(m[1])
    }
    expect(routes.size, 'Rust 路由扫描疑似空转').toBeGreaterThanOrEqual(70)

    const accounted = new Set([
      ...Object.values(ENDPOINTS).map((ep) => ep.path),
      ...FORBIDDEN_ENDPOINTS.map((x) => x.path),
      ...LEDGER_IDS.map((id) => ROWS.get(id)?.path).filter(Boolean),
      ...DOC_UNREGISTERED_PENDING
    ])
    const stray = [...routes].filter((p) => !accounted.has(p)).sort()
    expect(stray, `Rust 有路由没被任何一态收走: ${stray.join(', ')}`).toEqual([])
    for (const p of DOC_UNREGISTERED_PENDING) {
      expect(routes.has(p), `欠登记账上的 ${p} 已不在 Rust 源里，连条一起删`).toBe(true)
      expect(registered.has(p), `${p} 已补进 docs/API-REGISTRY.md，该转进台账了`).toBe(false)
    }
  })

  // 「挂载」的第二态漏洞：端点写在 ENDPOINTS 里、api 层也给了方法，但没有任何 store/视图调用它
  // ⇒ 覆盖率把这种面算进分子，报出来的 89% 是虚的。本用例要求每条端点都能走到代码证据。
  // 判据（两条任一成立即算接上）：① alliance.api.js 里含该端点名的那个方法，在 api 目录之外被引用；
  // ② 端点名以字符串出现在 modules 内除 endpoints.js / api 层之外的任何文件（协作模式表、视图直接拼 URL 都属此类）。
  const DECLARED_NOT_WIRED = Object.freeze([
    'expertDetail'
  ])

  it('每条已声明端点都要么被接上，要么在未接线台账里挂名（只减不增）', () => {
    expect(WIRING.outsideCount, '接线扫描没读到模块文件').toBeGreaterThan(20)
    expect(WIRING.methods.length, 'api 方法扫描没读到方法块').toBeGreaterThan(50)
    expect(WIRING.unwired.sort(), `未接线端点集合漂移（新长出＝假挂载；变少＝把条目从台账删掉）`).toEqual([...DECLARED_NOT_WIRED].sort())
    for (const name of DECLARED_NOT_WIRED) {
      expect(ENDPOINTS[name], `${name} 已不在 ENDPOINTS 里，连条一起删`).toBeTruthy()
      // api 层确实给了方法，否则不是"待接"而是"根本没写"，那种该直接从 ENDPOINTS 删
      expect(WIRING.methods.some((mm) => mm.body.includes(`'${name}'`)), `${name} 在 api 层没有调用者`).toBe(true)
    }
  })

  it('requestPath 填充参数并剥掉 baseURL 前缀', () => {
    expect(requestPath('taskDetail', { id: 't-1' })).toBe('/alliance/tasks/t-1')
    expect(requestPath('graphPath', { source: 'e1', target: 'e2' })).toBe('/expert-graph/path/e1/e2')
    expect(requestPath('expertsList')).toBe('/experts')
    // 缺参数要一次报全，并且报出的是缺失的那个键名
    expect(() => requestPath('graphPath', { source: 'e1' })).toThrow(/^端点 graphPath 缺少路径参数: target$/)
    expect(() => requestPath('graphPath')).toThrow(/source, target/)
  })
})

describe('信封与规范化', () => {
  it('alliance 双层 data 与 experts 单层 data 均正确剥离', () => {
    const nested = { code: 0, msg: 'ok', data: { elapsed_ms: 3, params: { task_id: 't1' }, data: { task_id: 't1', status: 'running' } } }
    const flat = { code: 0, msg: 'ok', data: { experts: [], total: 0 } }
    expect(unwrap(nested, { nesting: 'auto' })).toEqual({ task_id: 't1', status: 'running' })
    expect(unwrap(nested, { nesting: 'nested' })).toEqual({ task_id: 't1', status: 'running' })
    expect(unwrap(flat, { nesting: 'auto' })).toEqual({ experts: [], total: 0 })
    expect(unwrap({ data: flat }, { nesting: 'auto' })).toEqual({ experts: [], total: 0 })
  })

  it('code!=0 抛 ApiError 并带 msg', async () => {
    const { ApiError } = await import('../../_kernel/envelope.js')
    try {
      unwrap({ code: 404, msg: 'not found', data: null })
      throw new Error('should have thrown')
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError)
      expect(e.msg).toBe('not found')
    }
  })

  it('任务字段归一', () => {
    const t = normTask({ task_id: 't1', title: 'x', status: 'planning', progress: 40, mode: 'parallel', created_at: '2026-09-23T00:00:00Z' })
    expect(t).toMatchObject({ id: 't1', status: 'planning', progress: 40, mode: 'parallel', modeDisplay: 'expert_alliance' })
    expect(t.durationMs).toBeNull()
  })

  it('DAG 节点保留坐标与依赖，终态判定正确', () => {
    const d = normDag({
      nodes: [{ id: 'node-1', label: '意图', expert_id: 'e1', status: 'completed', progress: 100, dependencies: [], position: { x: 1, y: 2 } }],
      edges: [{ source: 'node-1', target: 'node-2', label: '依赖' }],
      stats: { total: 2, completed: 1, running: 1 }
    })
    expect(d.nodes[0]).toMatchObject({ id: 'node-1', terminal: true, position: { x: 1, y: 2 } })
    expect(d.edges[0].label).toBe('依赖')
    expect(d.stats.failed).toBe(0)
  })

  it('融合结果按置信度定级', () => {
    const f = normFusion({ task_id: 't1', fusion_strategy: 'weighted', fusion_result: { summary: 's', confidence: 0.93, key_findings: ['a'], recommendations: [] }, participating_nodes: ['node-1'] })
    expect(f).toMatchObject({ strategy: 'weighted', summary: 's', confidence: 0.93, grade: 'A' })
    expect(f.participatingNodes).toEqual(['node-1'])
  })

  it('日志流裸 LogEntry 归一', () => {
    expect(normLogEntry({ seq: 3, ts: '2026-09-23T00:00:01Z', level: 'warn', node_id: 'node-2', message: 'hi' })).toEqual({
      seq: 3, ts: '2026-09-23T00:00:01Z', level: 'warn', nodeId: 'node-2', message: 'hi'
    })
    expect(normLogEntry({}).level).toBe('info')
  })

  it('执行状态保留 skipped 零值桩事实', () => {
    const s = normExecutionStatus({ task_id: 't1', status: 'running', progress: 50, total_nodes: 4, skipped_nodes: 0, cancelled_nodes: 1 })
    expect(s.counts).toMatchObject({ total: 4, skipped: 0, cancelled: 1 })
  })

  it('本地预览标记为模拟执行', () => {
    expect(normRuntime({ execution_ready: false, mode: 'local_preview', message: 'm' })).toMatchObject({ simulated: true, mode: 'local_preview' })
    expect(normRuntime({ execution_ready: true, mode: 'remote', scheduler_ready: true, executor_ready: false })).toMatchObject({ simulated: false, schedulerReady: true, executorReady: false })
  })
})

describe('专家域枚举 ↔ experts_common.rs / experts_ext.rs', () => {
  it('在线状态、类型、计费、认证取值清单与 Rust 文档注释等序', () => {
    expect(Object.values(EXPERT_AVAILABILITY)).toEqual(docChoices(EXPERTS_COMMON_RS, 'online'))
    expect(Object.values(EXPERT_TYPE)).toEqual(docChoices(EXPERTS_COMMON_RS, 'human'))
    expect(Object.values(PRICING_MODEL)).toEqual(docChoices(EXPERTS_COMMON_RS, 'free'))
    expect(Object.values(VERIFICATION_STATUS)).toEqual(docChoices(EXPERTS_COMMON_RS, 'unverified'))
  })

  it('计费文案按「分」换算，缺费率时不谎报价格', () => {
    // 后端字段名带 _cents、注释标注单位为分，前端必须按 1/100 换算后再展示
    expect(EXPERTS_COMMON_RS).toMatch(/\/\/\/ 每小时费率（分）\n\s*#\[serde\(default\)\]\n\s*pub hourly_rate_cents: u32/)
    expect(pricingText('paid', 12000)).toBe('按次计费 · 120 元/时')
    expect(pricingText('subscription', 5000)).toBe('订阅制 · 50 元/时')
    // default_pricing 即 free，free 与未设置都不显示价格；计费型但费率为 0 时显式说明未设价
    expect(EXPERTS_COMMON_RS).toContain('fn default_pricing() -> String { "free".into() }')
    expect(pricingText('free', 0)).toBe('免费')
    expect(pricingText('', 0)).toBe('免费')
    expect(pricingText('paid', 0)).toBe('按次计费 · 未设价')
  })

  it('预约四态与 Booking.status 的 Rust 字面量一致', () => {
    for (const v of Object.values(BOOKING_STATUS)) expect(EXPERTS_EXT_RS).toContain(`"${v}"`)
    // 后端创建时写入 pending，取消时写入 cancelled
    expect(EXPERTS_EXT_RS).toContain('status: "pending".into()')
    expect(EXPERTS_EXT_RS).toContain('b.status = "cancelled".into()')
  })

  it('后端预约/收藏不按登录用户切分，前端文案只能自陈这一现状', () => {
    // create_booking 写死 user_id、my_bookings 无过滤 → 「我的预约」实为全量列表
    expect(EXPERTS_EXT_RS).toContain('user_id: "admin-user".into()')
    const mine = EXPERTS_EXT_RS.match(/async fn my_bookings\([\s\S]*?\n\}/)
    expect(mine, '未找到 my_bookings handler').not.toBeNull()
    expect(mine[0]).not.toMatch(/user_id\s*==/)
    // 收藏集是进程内 HashSet，重启即失，前端不得当作持久偏好渲染成"已同步"
    expect(EXPERTS_COMMON_RS).toMatch(/pub favorites: Arc<Mutex<std::collections::HashSet<String>>>/)
  })

  it('可取消判定与后端拒绝条件互斥', () => {
    expect(canCancelBooking('pending')).toBe(true)
    expect(canCancelBooking('confirmed')).toBe(true)
    expect(canCancelBooking('completed')).toBe(false)
    expect(canCancelBooking('cancelled')).toBe(false)
    expect(EXPERTS_EXT_RS).toContain('if b.status == "cancelled" || b.status == "completed"')
  })

  it('即时咨询以后端 online 判定为唯一门槛', () => {
    expect(isConsultable('online')).toBe(true)
    expect(isConsultable('busy')).toBe(false)
    expect(EXPERTS_REGISTRY_RS).toContain('exp.availability.status == "online"')
  })

  it('专家列表查询参数逐一对应后端读取的键', () => {
    const handler = EXPERTS_REGISTRY_RS.match(/async fn list_experts\([\s\S]*?\n\}/)
    expect(handler, '未找到 list_experts handler').not.toBeNull()
    // page/page_size 由共用的 parse_pagination 读取（experts_common.rs:875）
    const pager = EXPERTS_COMMON_RS.match(/pub fn parse_pagination\([\s\S]*?\n\}/)
    expect(pager, '未找到 parse_pagination').not.toBeNull()
    const scope = handler[0] + pager[0]
    for (const wire of Object.values(EXPERT_QUERY_KEYS)) {
      expect(scope.includes(`params.get("${wire}")`), `后端未读取 ${wire}`).toBe(true)
    }
    for (const s of EXPERT_SORT) expect(handler[0]).toContain(`"${s}" =>`)
    // 后端认识的每个 sort 值都要有文案，反向也不能多——多一个就会向后端发无效 sort
    expect(Object.keys(EXPERT_SORT_LABELS)).toEqual(EXPERT_SORT)
  })

  it('expertListQuery 丢弃空值并转成 wire 名', () => {
    expect(expertListQuery({ search: '架构', status: '', expertType: 'ai', page: 2, pageSize: 24 }))
      .toEqual({ search: '架构', expert_type: 'ai', page: 2, page_size: 24 })
    expect(expertListQuery({ sort: 'rating', unknownKey: 'x' })).toEqual({ sort: 'rating' })
    expect(expertListQuery()).toEqual({})
  })
})

describe('专家榜单口径 ↔ ExpertMetrics / ExpertDescriptor', () => {
  // 存量 ExpertPlazaView 用 goodRate / monthGrowth / responseTime 造过排行榜，
  // 这些字段后端从来没有。榜单改吃真实指标后，把口径钉在 Rust 结构体上。
  const structFields = (name, where) => {
    const m = where.match(new RegExp(`pub struct ${name} \\{([\\s\\S]*?)\\n\\}`))
    if (!m) throw new Error(`experts_common.rs 缺少结构体 ${name}`)
    return [...m[1].matchAll(/pub (\w+):/g)].map((x) => x[1])
  }
  const METRIC_FIELDS = structFields('ExpertMetrics', EXPERTS_COMMON_RS)

  it('normExpert 的 metrics 键与 Rust ExpertMetrics 字段逐一对应', () => {
    expect(Object.keys(normExpert({ metrics: {} }).metrics).map(pascalToSnake)).toEqual(METRIC_FIELDS)
  })

  it('榜单依据的字段真实存在，臆造字段在 Rust 源里查无此名', () => {
    for (const f of ['total_consultations', 'avg_rating', 'rating_count']) {
      expect(METRIC_FIELDS).toContain(f)
    }
    expect(structFields('ExpertDescriptor', EXPERTS_COMMON_RS)).toContain('created_at')
    for (const invented of ['good_rate', 'month_growth', 'response_time', 'consult_count', 'is_new', 'recommended']) {
      expect(EXPERTS_COMMON_RS, `后端不该有 ${invented}`).not.toMatch(new RegExp(`pub ${invented}`))
    }
  })

  it('rank.js 只引用后端字段名，不夹带历史假指标', () => {
    const source = readFileSync(path.join(MODULE_DIR, 'model/rank.js'), 'utf8').replace(/\r\n/g, '\n')
    expect(source).not.toMatch(/goodRate|monthGrowth|responseTime|consultCount|isNew|recommended|hot/)
    expect(source).toMatch(/totalConsultations/).toMatch(/avgRating/).toMatch(/ratingCount/)
  })

  it('三张榜单都能吃归一后的真实响应跑出第一名', () => {
    const raw = {
      id: 'e1', name: '林架构', title: '首席架构师', created_at: '2026-01-05T08:00:00Z',
      metrics: { total_consultations: 180, avg_rating: 4.7, rating_count: 56 }
    }
    for (const board of RANK_BOARDS) {
      const result = buildBoard(board.key, [normExpert(raw)])
      expect(result.rows, `${board.key} 榜应当有数据`).toHaveLength(1)
      expect(result.rows[0].name).toBe('林架构')
    }
  })
})

describe('平台统计 ↔ experts_stats_real', () => {
  // 存量广场页顶部 KPI 读的是 expert_count / consult_count / good_rate / avg_response，
  // 后端 stats 响应里这四个键一个都没有，四张卡因此永远停在占位值。新口径逐键对齐真实响应。
  const handler = EXPERTS_REGISTRY_RS.match(/async fn experts_stats_real\([\s\S]*?ok\(json!\(\{([\s\S]*?)\}\)\)/)

  it('normExpertStats 的键与后端 stats 响应键等集', () => {
    expect(handler, '未找到 experts_stats_real 的 json! 响应').not.toBeNull()
    const wireKeys = [...handler[1].matchAll(/"([a-z_]+)":/g)].map((m) => m[1])
    const feKeys = Object.keys(normExpertStats({})).map(pascalToSnake)
    expect(feKeys.slice().sort()).toEqual(wireKeys.sort())
  })

  it('历史假指标键在 stats 响应里确实不存在', () => {
    expect(handler).not.toBeNull()
    for (const ghost of ['expert_count', 'consult_count', 'good_rate', 'avg_response']) {
      expect(handler[1], `stats 不该有 ${ghost}`).not.toContain(`"${ghost}"`)
    }
  })

  it('domains 是映射而非数组，归一后按人数降序成列表', () => {
    const s = normExpertStats({ domains: { ai: 3, architecture: 5, kb: 3 } })
    expect(s.domains).toEqual([{ name: 'architecture', count: 5 }, { name: 'ai', count: 3 }, { name: 'kb', count: 3 }])
    expect(normExpertStats(null).domains).toEqual([])
  })

  it('KPI 文案：计数 0 照实显示，无样本的评分/响应显示「—」且不泄露浮点尾数', () => {
    const cells = expertStatsCells(normExpertStats({
      total_experts: 11, online_experts: 11, busy_experts: 0, offline_experts: 0,
      total_consultations: 3561, today_consultations: 0,
      avg_rating: 4.363636363636363, avg_response_minutes: 5
    }))
    expect(cells.map((c) => c.value)).toEqual(['11 位', '11 / 0 / 0', '3561 次', '0 次', '4.4', '5 分钟'])
    expect(expertStatsCells(null).every((c) => c.value === '—')).toBe(true)
  })
})

describe('能力目录与单专家派生指标 ↔ list_capabilities / expert_metrics', () => {
  const caps = EXPERTS_REGISTRY_RS.match(/async fn list_capabilities\([\s\S]*?\n\}/)
  const metrics = EXPERTS_REGISTRY_RS.match(/async fn expert_metrics\([\s\S]*?\n\}/)
  const jsonKeys = (block) => [...block.matchAll(/"([a-z_]+)"\s*:/g)].map((m) => m[1])
  // 同一 handler 里有两处 json!：能力条目与顶层响应，按条目特征键区分
  const capItemBlock = [...(caps?.[0].matchAll(/json!\(\{([\s\S]*?)\}\)/g) || [])]
    .map((m) => m[1]).find((b) => b.includes('"expert_count"')) || ''
  const capTopBlock = caps?.[0].match(/ok\(json!\(\{([\s\S]*?)\}\)\)/)?.[1] || ''
  const derivedBlock = metrics?.[0].match(/"derived": \{([\s\S]*?)\}/)?.[1] || ''

  it('两个面已从台账搬进挂载清单，且逐字对齐注册表行', () => {
    expect(caps, '未找到 list_capabilities handler').not.toBeNull()
    expect(ENDPOINTS.expertCapabilities).toEqual({
      registry: 'experts.registry.capabilities', method: 'GET', path: '/api/experts/capabilities', nesting: 'flat'
    })
    expect(ENDPOINTS.expertMetrics).toEqual({
      registry: 'experts.registry.detail_metrics', method: 'GET', path: '/api/experts/:id/metrics', nesting: 'flat'
    })
    for (const id of ['experts.registry.capabilities', 'experts.registry.detail_metrics']) {
      expect(UNMOUNTED_ROUTES.some((x) => x.registry === id), `${id} 仍留在台账里`).toBe(false)
    }
    expect(requestPath('expertMetrics', { id: 'exp 1' })).toBe('/experts/exp%201/metrics')
  })

  it('normCapabilities 的两层键与后端 json! 键集等集', () => {
    const wireTop = jsonKeys(capTopBlock)
    expect(wireTop).toEqual(['capabilities', 'total', 'domains'])
    // 唯一的刻意改名：后端的 capabilities 数组在前端叫 items，与 normExpertList/normSessionList 同构
    expect(Object.keys(normCapabilities({}))).toEqual(['items', 'total', 'domains'])
    const wire = jsonKeys(capItemBlock)
    expect(wire).toEqual(['id', 'name', 'domain', 'expert_count', 'avg_proficiency'])
    expect(Object.keys(normCapabilities({ capabilities: [{}] }).items[0]).map(pascalToSnake)).toEqual(wire)
  })

  it('目录只统计已启用专家、按 id 升序，前端保持后端次序', () => {
    expect(caps[0]).toContain('.filter(|e| e.enabled)')
    expect(caps[0]).toMatch(/capabilities\.sort_by\([\s\S]*?"id"/)
    // 乱序输入必须原样输出：一旦本地按人数重排，页面顺序就不再是后端承诺的那个
    const out = normCapabilities({ capabilities: [{ id: 'z', expert_count: 1 }, { id: 'a', expert_count: 9 }], total: 2, domains: ['ai'] })
    expect(out.items.map((c) => c.id)).toEqual(['z', 'a'])
    expect(out.items.map((c) => c.expertCount)).toEqual([1, 9])
    expect(out.total).toBe(2)
    expect(out.domains).toEqual(['ai'])
  })

  it('缺字段与 null 响应归一出空目录而不是崩，人数/熟练度缺省为 0', () => {
    for (const bare of [null, undefined, {}, { capabilities: 'x' }]) {
      const c = normCapabilities(bare)
      expect(c).toEqual({ items: [], total: 0, domains: [] })
    }
    const row = normCapabilities({ capabilities: [{ id: 'c1' }] }).items[0]
    expect(row).toEqual({ id: 'c1', name: '', domain: '', expertCount: 0, avgProficiency: 0 })
  })

  it('normExpertMetrics 的顶层与 derived 键等集，指标结构复用 normExpert', () => {
    expect(metrics, '未找到 expert_metrics handler').not.toBeNull()
    const top = metrics[0].match(/ok\(json!\(\{([\s\S]*?)\n\s*\}\)\)/)?.[1] || ''
    // derived 是嵌套对象，只取它之前的顶层键，否则三个派生值会被当成顶层字段
    const wireTop = [...jsonKeys(top.slice(0, top.indexOf('"derived"'))), 'derived']
    expect(wireTop).toEqual(['expert_id', 'metrics', 'availability', 'derived'])
    expect(Object.keys(normExpertMetrics({})).map(pascalToSnake)).toEqual(wireTop)
    expect(jsonKeys(derivedBlock)).toEqual(['rank_percentile', 'load_ratio', 'efficiency_score'])
    const m = normExpertMetrics({
      expert_id: 'e1', metrics: { avg_rating: 4.5, resolution_rate: 0.8 },
      availability: { current_load: 3, max_concurrent: 4, status: 'online' },
      derived: { rank_percentile: 62.5, load_ratio: 0.75, efficiency_score: 0.7836 }
    })
    expect(m).toEqual({
      expertId: 'e1',
      metrics: {
        totalConsultations: 0, todayConsultations: 0, avgRating: 4.5, ratingCount: 0,
        resolutionRate: 0.8, firstResponseAccuracy: 0, totalServiceMinutes: 0
      },
      availability: {
        status: 'online', lastActive: '', avgResponseMinutes: 0,
        currentLoad: 3, maxConcurrent: 4, loadRatio: 0.75
      },
      derived: { rankPercentile: 62.5, loadRatio: 0.75, efficiencyScore: 0.7836 }
    })
  })

  it('三个派生值由后端算：权重 40/30/30 写死在 Rust，前端只做换算', () => {
    expect(metrics[0]).toMatch(/\* 0\.4\n\s*\+ [\s\S]*?\* 0\.3\n\s*\+ [\s\S]*?\* 0\.3/)
    expect(metrics[0]).toContain('let efficiency_score =')
    const model = src('frontend-ui/src/modules/expert-alliance/model/normalize.js')
    const cells = src('frontend-ui/src/modules/expert-alliance/contract/enums.js')
    for (const text of [model, cells]) {
      expect(text, '前端不得自己算效率分/百分位').not.toMatch(/efficiencyScore\s*[*/+-]|[*/]\s*0\.4\b/)
    }
  })

  it('派生指标文案：百分位留一位小数、负载率可超 100%、缺值显示「—」', () => {
    expect(expertDerivedCells({ rankPercentile: 62.5, loadRatio: 1.5, efficiencyScore: 0.7836 }).map((c) => c.value))
      .toEqual(['62.5%', '150%', '78.4%'])
    expect(expertDerivedCells({ rankPercentile: 0, loadRatio: 0, efficiencyScore: 0 }).map((c) => c.value))
      .toEqual(['0.0%', '0%', '0.0%'])
    const empty = expertDerivedCells(null)
    expect(empty.map((c) => c.value)).toEqual(['—', '—', '—'])
    // 每格都要说明口径，否则"效率分 78.4%"会被读成前端自己打的分
    for (const c of empty) expect(c.note.length).toBeGreaterThan(10)
  })

  it('停用专家没有派生指标：后端 404，前端不得折叠成零值', () => {
    expect(metrics[0]).toContain('Some(e) if e.enabled => e.clone(),')
    expect(metrics[0]).toMatch(/_ => return err\(404, format!\("expert not found: \{\}"/)
    expect(normExpertMetrics({}).derived).toEqual({ rankPercentile: 0, loadRatio: 0, efficiencyScore: 0 })
    // 后端对无上限的专家给 load_ratio 0，而 availability.loadRatio 本地判为 null，两者口径不同源
    const m = normExpertMetrics({ availability: { current_load: 2, max_concurrent: 0 }, derived: { load_ratio: 0 } })
    expect(m.availability.loadRatio).toBeNull()
    expect(m.derived.loadRatio).toBe(0)
  })
})

describe('专家广场响应归一', () => {
  // 载荷逐字段照抄 expert_json（ExpertDescriptor 序列化 + 注入 type）
  const descriptor = () => ({
    id: 'exp-1',
    name: '林架构',
    avatar: '',
    title: '首席架构师',
    organization: '璇玑科技',
    bio: '分布式系统 12 年',
    domains: ['architecture', 'backend'],
    skills: ['Rust', 'gRPC'],
    capabilities: [{ id: 'c1', name: '架构评审', domain: 'architecture', proficiency: 92, description: 'd' }],
    availability: { status: 'online', last_active: '2026-09-23T10:00:00Z', avg_response_minutes: 4.5, current_load: 2, max_concurrent: 5 },
    metrics: { total_consultations: 180, today_consultations: 3, avg_rating: 4.7, rating_count: 56, resolution_rate: 0.93, first_response_accuracy: 0.88, total_service_minutes: 5400 },
    expert_type: 'human',
    pricing_model: 'paid',
    hourly_rate_cents: 12000,
    languages: ['zh', 'en'],
    timezone: 'Asia/Shanghai',
    verification_status: 'certified',
    tags: ['架构'],
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-09-01T00:00:00Z',
    enabled: true,
    metadata: {},
    type: '架构'
  })

  it('availability/capabilities/metrics 的结构体形状完整保留', () => {
    const e = normExpert(descriptor())
    expect(e).toMatchObject({
      id: 'exp-1',
      name: '林架构',
      online: true,
      status: 'online',
      expertType: 'human',
      verificationStatus: 'certified',
      timezone: 'Asia/Shanghai',
      enabled: true
    })
    expect(e.capabilities[0]).toEqual({ id: 'c1', name: '架构评审', domain: 'architecture', proficiency: 92, description: 'd' })
    expect(e.availability).toMatchObject({ status: 'online', avgResponseMinutes: 4.5, currentLoad: 2, maxConcurrent: 5, loadRatio: 0.4 })
    expect(e.metrics).toEqual({
      totalConsultations: 180, todayConsultations: 3, avgRating: 4.7, ratingCount: 56,
      resolutionRate: 0.93, firstResponseAccuracy: 0.88, totalServiceMinutes: 5400
    })
  })

  it('max_concurrent 为 0 时负载率记 null，不当作 0% 展示', () => {
    const e = normExpert({ ...descriptor(), availability: { status: 'busy', current_load: 1, max_concurrent: 0 } })
    expect(e.availability.loadRatio).toBeNull()
    expect(e.online).toBe(false)
    expect(e.status).toBe('busy')
  })

  it('缺字段的老数据不炸：默认按离线/未启用不可判定处理', () => {
    const e = normExpert({})
    expect(e).toMatchObject({ id: '', name: '', online: false, status: 'offline', enabled: true, capabilities: [], skills: [] })
    expect(e.availability.loadRatio).toBeNull()
  })

  it('预约列表四态计数与可取消标记', () => {
    const b = (status) => ({ id: `b-${status}`, expert_id: 'e1', expert_name: 'N', user_id: 'u', topic: 't', scheduled_at: '', duration_minutes: 60, status, created_at: '' })
    const out = normBookingList({ bookings: [b('pending'), b('cancelled')], total: 2, pending: 1, confirmed: 0, completed: 0, cancelled: 1 })
    expect(out.items.map((x) => x.cancellable)).toEqual([true, false])
    expect(out.counts).toEqual({ pending: 1, confirmed: 0, completed: 0, cancelled: 1 })
    expect(out.total).toBe(2)
  })

  it('取消响应只回四个字段', () => {
    expect(normBookingCancel({ booking_id: 'b1', status: 'cancelled', cancelled_at: '2026-09-23T11:00:00Z', message: '预约已取消' }))
      .toEqual({ bookingId: 'b1', status: 'cancelled', cancelledAt: '2026-09-23T11:00:00Z', message: '预约已取消' })
  })

  it('即时咨询成功/不在线两分支都归一到 sessionId 判据', () => {
    const ok = normConsultNow({ expert_id: 'e1', session_id: 'sess-1', status: 'connected', channel: 'text', topic: '即时咨询', question: 'q', expert_online: true, chat_url: '/chat/sess-1', created_at: 't' })
    expect(ok).toMatchObject({ sessionId: 'sess-1', status: 'connected', expertOnline: true, chatUrl: '/chat/sess-1', question: 'q' })
    const off = normConsultNow({ expert_id: 'e1', session_id: null, status: 'unavailable', channel: 'text', topic: '即时咨询', question: null, expert_online: false, chat_url: null, created_at: 't', message: '专家当前不在线，请稍后重试或预约' })
    expect(off).toMatchObject({ sessionId: null, expertOnline: false, chatUrl: null, question: null })
  })

  it('咨询室在专家缺失时 expert_info 为 null', () => {
    const r = normConsultRoom({ booking_id: 'b1', room_id: 'r1', room_token: 'tk', join_url: '/consult/room/r1', webrtc_config: { ice_servers: [{ urls: 'stun:stun.l.google.com:19302' }] }, expert_info: null, status: 'waiting', expires_in: 3600, created_at: 't' })
    expect(r.expertInfo).toBeNull()
    expect(r.iceServers).toEqual(['stun:stun.l.google.com:19302'])
    const withExpert = normConsultRoom({ expert_info: { id: 'e1', name: 'N', title: 'T', avatar: '', online: true } })
    expect(withExpert.expertInfo).toEqual({ id: 'e1', name: 'N', title: 'T', avatar: '', online: true })
  })

  it('团队申请区分自动批准与待审批', () => {
    expect(normTeamApplication({ application_id: 'a1', status: 'approved', team_id: 't1', expert_id: 'e1', role: 'member', applied_at: 't', estimated_review_hours: 0, message: '专家已验证，自动批准加入团队' }))
      .toMatchObject({ status: 'approved', estimatedReviewHours: 0 })
    expect(EXPERTS_REGISTRY_RS).toContain('("approved", eid.clone())')
    expect(EXPERTS_REGISTRY_RS).toContain('("pending_approval", eid.clone())')
  })
})

describe('智能协作契约 ↔ experts_collaboration.rs', () => {
  const COLLAB_RS = src('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_collaboration.rs')
  const HANDLER_OF = {
    route: 'route_query',
    single: 'consult_expert',
    multi: 'multi_consult',
    debate: 'debate',
    smart: 'intelligent_consult',
    algorithm: 'algorithm_analysis'
  }

  const handlerOf = (fn) => {
    const m = COLLAB_RS.match(new RegExp(`async fn ${fn}\\([\\s\\S]*?\\n\\}`))
    if (!m) throw new Error(`experts_collaboration.rs 缺少 handler ${fn}`)
    return m[0]
  }
  const structFields = (name) => {
    const m = COLLAB_RS.match(new RegExp(`struct ${name} \\{([\\s\\S]*?)\\n\\}`))
    if (!m) throw new Error(`experts_collaboration.rs 缺少结构体 ${name}`)
    return [...m[1].matchAll(/^\s+(\w+):/gm)].map((x) => x[1])
  }
  const bodyStructOf = (fn) => {
    const m = handlerOf(fn).match(/Json<(\w+)>/)
    if (!m) throw new Error(`handler ${fn} 未使用 Json<结构体> 提取器`)
    return m[1]
  }
  /** 取 json! 字面量的顶层键；字符串字面量内的花括号不参与深度计算 */
  const topLevelJsonKeys = (text, marker = 'ok(json!') => {
    const at = text.lastIndexOf(marker)
    if (at < 0) throw new Error(`未找到 ${marker} 返回体`)
    const body = text.slice(at + marker.length)
    const keys = []
    let depth = 0
    for (let j = 0; j < body.length; j++) {
      const ch = body[j]
      if (ch === '"') {
        let k = j + 1
        while (k < body.length && body[k] !== '"') k += body[k] === '\\' ? 2 : 1
        let m = k + 1
        while (m < body.length && /\s/.test(body[m])) m++
        if (depth === 1 && body[m] === ':') keys.push(body.slice(j + 1, k))
        j = k
        continue
      }
      if (ch === '{' || ch === '[') depth += 1
      else if (ch === '}' || ch === ']') {
        depth -= 1
        if (depth === 0) break
      }
    }
    return keys
  }
  // 归一化器按约定统一把 payload 命名为 p，因此 p.xxx 就是「前端读了哪些后端键」
  const NORM_SRC = src('frontend-ui/src/modules/expert-alliance/model/normalize.js')
  const normReads = (fnName) => {
    const m = NORM_SRC.match(new RegExp(`export function ${fnName}\\([\\s\\S]*?\\n\\}`))
    if (!m) throw new Error(`normalize.js 缺少 ${fnName}`)
    return [...new Set([...m[0].matchAll(/\bp\.([a-z_]+)/g)].map((x) => x[1]))]
  }

  it('六个模式的后端端点都在协作路由装配里注册', () => {
    const router = COLLAB_RS.match(/pub fn build_experts_collaboration_router\([\s\S]*?\n\}/)
    expect(router, '未找到协作路由装配函数').not.toBeNull()
    for (const m of COLLAB_MODES) {
      const ep = ENDPOINTS[m.endpoint]
      expect(ep, `模式 ${m.key} 指向了未声明端点`).toBeTruthy()
      expect(router[0], `后端未注册 ${ep.path}`).toContain(`.route("${ep.path}", post(${HANDLER_OF[m.key]}))`)
    }
    // 一个端点只服务一个模式，否则结果渲染口径会串
    expect(new Set(COLLAB_MODES.map((m) => m.endpoint)).size).toBe(COLLAB_MODES.length)
  })

  it('辩论只认 topic：后端 DebateBody 没有 question 字段', () => {
    // 存量两份实现都发 question（ExpertWorkspaceView.vue:520、ExpertCenterView.vue:401），
    // 而 topic 没有 serde(default)，axum 直接拒成 422 —— 该功能在存量页面从未真正跑通过
    const fields = structFields('DebateBody')
    expect(fields).toContain('topic')
    expect(fields).not.toContain('question')
    expect(collabMode(COLLAB_MODE.DEBATE).field).toBe('topic')
    expect(collabProblem(collabMode(COLLAB_MODE.DEBATE), { topic: '' })).toBe('请填写辩题')
    expect(collabBody(collabMode(COLLAB_MODE.DEBATE), { topic: '辩题', question: '错字段' })).toEqual({ topic: '辩题', rounds: 3 })
  })

  it('数值控件的边界即后端 clamp 字面量', () => {
    const multi = collabMode(COLLAB_MODE.MULTI)
    const route = collabMode(COLLAB_MODE.ROUTE)
    const debate = collabMode(COLLAB_MODE.DEBATE)
    expect(collabMode(COLLAB_MODE.SMART).controls).toEqual([])
    expect(handlerOf('multi_consult')).toContain('body.max_experts.unwrap_or(3).clamp(1, 10)')
    expect(handlerOf('route_query')).toContain('body.max_experts.unwrap_or(5).clamp(1, 20)')
    expect(handlerOf('debate')).toContain('body.rounds.unwrap_or(3).clamp(1, 10)')
    expect(multi.controls[0]).toMatchObject({ wire: 'max_experts', min: 1, max: 10, default: 3 })
    expect(route.controls[0]).toMatchObject({ wire: 'max_experts', min: 1, max: 20, default: 5 })
    expect(debate.controls[0]).toMatchObject({ wire: 'rounds', min: 1, max: 10, default: 3 })
    // 越界入参在发出前就被夹住，与后端同序，不会出现「前端填 99、后端跑 10」
    expect(collabBody(multi, { question: 'q', max_experts: 99 }).max_experts).toBe(10)
    expect(collabBody(route, { question: 'q', max_experts: 0 }).max_experts).toBe(1)
    expect(collabBody(debate, { topic: 'q' }).rounds).toBe(3)
  })

  it('辩论 2–4 人的上下限写在后端代码里，前端校验与提示同步', () => {
    const text = handlerOf('debate')
    expect(text).toContain('.take(4)')
    expect(text).toMatch(/debaters\.len\(\) < 2[\s\S]*400, format!\("辩论至少需要 2 名专家/)
    expect(DEBATE_PARTICIPANT_CAP).toBe(4)
    expect(DEBATE_MIN_PARTICIPANTS).toBe(2)
    expect(collabMode(COLLAB_MODE.DEBATE).participantCap).toBe(DEBATE_PARTICIPANT_CAP)
    const debate = collabMode(COLLAB_MODE.DEBATE)
    expect(collabProblem(debate, { topic: 't', expertIds: ['a'] })).toContain('至少需要')
    expect(collabProblem(debate, { topic: 't', expertIds: ['a', 'b'] })).toBe('')
    expect(collabProblem(debate, { topic: 't', expertIds: [] })).toBe('')
    expect(debateCapacityText(6)).toContain('仅取前 4 位')
    expect(debateCapacityText(3)).toBe('')
  })

  it('意图文案覆盖 classify_intent 产出的每个领域', () => {
    const mappings = COLLAB_RS.match(/pub fn classify_intent\([\s\S]*?let mappings: Vec<\(&str, &str\)> = vec!\[([\s\S]*?)\];/)
    expect(mappings, '未找到 classify_intent 关键词映射表').not.toBeNull()
    const domains = [...new Set([...mappings[1].matchAll(/,\s*"([a-z_]+)"\s*\)/g)].map((x) => x[1]))]
    expect(domains.length).toBeGreaterThan(5)
    expect(Object.keys(INTENT_LABELS).sort()).toEqual([...domains, 'general'].sort())
    expect(intentLabel('enterprise')).toBe('企业')
    expect(intentLabel('后端新增的领域')).toBe('后端新增的领域')
    expect(handlerOf('intelligent_consult')).toContain('classify_intent(&body.question)')
  })

  it('复杂度记号全部出自 analyze_complexity', () => {
    const text = COLLAB_RS.match(/pub fn analyze_complexity\([\s\S]*?\n\}/)[0]
    for (const notation of COMPLEXITY_ORDER) {
      expect(text, `后端不再产出 ${notation}，前端清单需同步`).toContain(`"${notation}"`)
    }
    expect(complexityLevel('O(2^n)').tag).toBe('danger')
    expect(complexityLevel('O(1)').tag).toBe('success')
    expect(complexityLevel('O(9^n)').tag).toBe('info')
    expect(handlerOf('algorithm_analysis')).toContain('analyze_complexity(&body.algorithm_description)')
  })

  it('六个 handler 的返回键与归一化读取的键双向闭合', () => {
    const runDebate = COLLAB_RS.match(/pub fn run_debate\([\s\S]*?\n\}/)[0]
    const cases = [
      [COLLAB_MODE.ROUTE, 'normRouteResult', handlerOf('route_query')],
      [COLLAB_MODE.SINGLE, 'normSingleConsult', handlerOf('consult_expert')],
      [COLLAB_MODE.MULTI, 'normMultiConsult', handlerOf('multi_consult')],
      // debate handler 直接 ok(result)，返回结构在 run_debate 里
      [COLLAB_MODE.DEBATE, 'normDebate', runDebate],
      [COLLAB_MODE.SMART, 'normIntelligentConsult', handlerOf('intelligent_consult')],
      [COLLAB_MODE.ALGORITHM, 'normAlgorithmAnalysis', handlerOf('algorithm_analysis')]
    ]
    for (const [key, fnName, text] of cases) {
      const wire = topLevelJsonKeys(text, key === COLLAB_MODE.DEBATE ? 'json!(' : 'ok(json!')
      expect(wire.length, `handler ${key} 未解析出返回键`).toBeGreaterThan(3)
      // 后端产出的键前端都要读，前端读的键后端必须真产出——两个方向都不能有缺口
      expect(normReads(fnName).sort()).toEqual([...new Set(wire)].sort())
      expect(fnName).toMatch(/^norm/)
    }
  })

  it('以网关 :3080 实测载荷跑通协作归一化', () => {
    // 夹具取自 2026-09-23 对 /api/experts/* 协作端点的真实响应
    const fusion = normMultiConsult({
      session_id: 'sess-multi-1',
      question: '如何设计微服务架构',
      experts: [{
        id: 'exp-architecture-001',
        name: '林架构',
        match_score: 0.319,
        answer: { analysis: 'a', solution: 's', references: ['《architecture领域工程实践指南》'], confidence: 0.942 }
      }],
      fused_answer: { summary: '综合1位专家', consensus_score: 1, dominant_view: '【林架构】s', alternative_views: [], confidence: 0.613 },
      created_at: '2026-09-23T18:05:00Z'
    })
    expect(fusion.contributions[0].answer).toMatchObject({ solution: 's', confidence: 0.942, source: '', vetoed: false, blocked: false })
    // 单专家时后端恒给 consensus_score=1.0，那是无从比对，不能显示成满共识
    expect(consensusText(fusion.fusion, fusion.contributions.length)).toContain('无从比对')
    expect(consensusText({ consensusScore: 0.42 }, 3)).toBe('0.42')
    expect(answerSourceText(fusion.contributions[0].answer)).toBe('模板兜底作答')
    expect(collabRefId(fusion)).toBe('sess-multi-1')
    expect(collabQuestionText(fusion)).toBe('如何设计微服务架构')
    expect(collabOutcome(fusion)).toBe('ok')

    const routed = normRouteResult({
      query: '微服务拆分',
      matched_experts: [{
        id: 'e1', name: 'N', title: 'T', match_score: 0.5, domains: ['architecture'], skills: ['Rust'],
        availability: { status: 'online', avg_response_minutes: 4, current_load: 1 },
        metrics: { avg_rating: 4.7, total_consultations: 180, resolution_rate: 0.93 }
      }],
      routing_decision: { recommended_expert_id: 'e1', reason: '推荐 N', alternative_ids: [] },
      total_scanned: 11,
      ts: '2026-09-23T18:05:00Z'
    })
    expect(routed.candidates[0]).toMatchObject({ id: 'e1', matchScore: 0.5, status: 'online', avgRating: 4.7, resolutionRate: 0.93 })
    expect(collabQuestionText(routed)).toBe('微服务拆分')

    const blocked = normMultiConsult({
      session_id: 'sess-2',
      question: 'q',
      experts: [{
        id: 'e1', name: '甲', match_score: 0.9,
        answer: { analysis: '拦截说明', solution: '【已拦截】高危操作', references: [], confidence: 0, source: 'llm', vetoed: true, veto_reason: '高危操作', blocked: true }
      }],
      fused_answer: { summary: '1 位专家的回复全部被治理闸门否决', consensus_score: 0, dominant_view: '', alternative_views: [], confidence: 0, vetoed: true, blocked: true },
      created_at: 't'
    })
    expect(blocked.fusion.blocked).toBe(true)
    expect(collabOutcome(blocked)).toBe('blocked')
    expect(answerSourceText(blocked.contributions[0].answer)).toBe('已被治理闸门拦截')
    expect(confidenceText(blocked.contributions[0].answer.confidence)).toBe('—')
    expect(collabRefId({ mode: 'route', ts: '2026-09-23T18:05:00Z' })).toBe('2026-09-23T18:05:00Z')

    const algo = normAlgorithmAnalysis({
      analysis_id: 'algo-1', algorithm_description: '递归求解', input_constraints: null, requirements: null,
      complexity: { time_complexity: 'O(2^n)', space_complexity: 'O(n)', big_o_notation: 'Time: O(2^n), Space: O(n)', explanation: '指数级' },
      feasibility: { score: 0.35, blockers: ['n>30 时超时'], risks: ['常数因子'] },
      recommended_experts: [{ id: 'e1', name: '数专家', title: 'T', domains: ['math'], match_score: 0.4 }],
      optimization_suggestions: ['引入记忆化'],
      created_at: 't'
    })
    expect(algo.complexity.time).toBe('O(2^n)')
    expect(algo.echoed).toEqual({ inputConstraints: '', requirements: '' })
    expect(algo.suggestions).toEqual(['引入记忆化'])
    expect(complexityLevel(algo.complexity.time).label).toContain('高风险')
    expect(collabOutcome(algo)).toBe('ok')
  })

  it('模式声明的每个 wire 都是 body 结构体字段，且真的被 handler 读取', () => {
    for (const m of COLLAB_MODES) {
      const fn = HANDLER_OF[m.key]
      const fields = structFields(bodyStructOf(fn))
      const reads = new Set((handlerOf(fn).match(/\bbody\.[a-z_]+/g) || []).map((s) => s.slice('body.'.length)))
      expect(m.wires, `模式 ${m.key} 未声明 field`).toContain(m.field)
      for (const wire of m.wires) {
        expect(fields, `模式 ${m.key} 的 ${wire} 不是 ${bodyStructOf(fn)} 的字段`).toContain(wire)
        expect(reads.has(wire), `模式 ${m.key} 的 ${wire} 结构体收得到但 handler 不读`).toBe(true)
      }
      for (const c of m.controls) expect(m.wires, `控件 ${c.wire} 未列入 ${m.key}.wires`).toContain(c.wire)
      // 全量输入也只能落进声明的 wire：新增字段必须先进契约表
      const body = collabBody(m, {
        [m.field]: '示例输入', expertIds: ['e1', 'e2'], domain: 'architecture',
        context: 'ctx', sessionId: 'sess-1', question: 'q', topic: 't', stance: 'pro',
        history: [1], priority: 'high', max_experts: 99, rounds: 99,
        min_rating: 4, max_response_time: 30, require_online: true
      })
      expect(Object.keys(body).sort(), `模式 ${m.key} 发出了未声明的 wire`).toEqual([...m.wires].sort())
      for (const f of m.constraintFields || []) {
        // 嵌套子键经 constraints.as_ref()?.get("key") 读取，没读就不该出现在前端
        expect(handlerOf(fn), `${m.key} 的 constraints.${f.key} handler 不读`).toContain(`c.get("${f.key}")`)
        expect(body.constraints, `${m.key} 未发出已填的 constraints.${f.key}`).toHaveProperty(f.key)
      }
    }
  })

  it('路由约束等于后端默认值时不发该子键，越界先夹到边界', () => {
    const route = collabMode(COLLAB_MODE.ROUTE)
    // route_query：min_rating→0.0、max_response_time→f64::MAX、require_online→false
    expect(collabBody(route, { question: 'q', max_experts: 5 })).not.toHaveProperty('constraints')
    expect(collabBody(route, { question: 'q', max_experts: 5, min_rating: 0, max_response_time: null, require_online: false }))
      .not.toHaveProperty('constraints')
    expect(collabBody(route, { question: 'q', max_experts: 5, min_rating: 4.5, require_online: true }).constraints)
      .toEqual({ min_rating: 4.5, require_online: true })
    // 边界来自 handler 的 as_f64 语义：评分上限 5，超出会被过滤成空候选，故前端先夹住
    expect(collabBody(route, { question: 'q', max_experts: 5, min_rating: 99, max_response_time: 9999 }).constraints)
      .toEqual({ min_rating: 5, max_response_time: 240 })
  })

  it('结构体接受但 handler 从不读的键，前端既不发也不渲染', () => {
    expect(COLLAB_LAZY_FIELDS.map((x) => `${x.struct}.${x.key}`).sort())
      .toEqual(['ConsultBody.context', 'ConsultBody.priority', 'DebateBody.stance', 'IntelligentConsultBody.history'])
    for (const { struct, key, mode, reason } of COLLAB_LAZY_FIELDS) {
      expect(structFields(struct), `${struct} 已无 ${key} 字段`).toContain(key)
      expect(handlerOf(HANDLER_OF[mode]), `${mode} handler 开始读取 ${key}，前端可接入`).not.toContain(`body.${key}`)
      expect(collabAccepts(collabMode(mode), key), `${mode} 声明了惰性字段 ${key}`).toBe(false)
      expect(reason.length, `${struct}.${key} 缺原因说明`).toBeGreaterThan(8)
    }
  })

  it('统一分发端点 /api/ai/expert-chat 读不到子结果，保持禁用', () => {
    const dispatch = handlerOf('expert_chat_dispatch')
    // multi 分支读 results、debate 分支读顶层 summary —— 两个键子 handler 都不产出，content 恒退化成兜底串
    expect(dispatch).toContain('.get("results")')
    expect(dispatch).toMatch(/d\.get\("summary"\)/)
    expect(handlerOf('multi_consult')).not.toContain('"results"')
    // 辩论的 summary 嵌在 verdict 里，顶层没有该键，dispatch 的 d.get("summary") 永远取不到
    expect(topLevelJsonKeys(COLLAB_RS.match(/pub fn run_debate\([\s\S]*?\n\}/)[0], 'json!(')).not.toContain('summary')
    expect(FORBIDDEN_ENDPOINTS.map((f) => f.path)).toContain('/api/ai/expert-chat')
    expect(COLLAB_RS).toContain('.route("/api/ai/expert-chat", post(expert_chat_dispatch))')
    expect(REGISTRY).not.toContain('/api/ai/expert-chat')
  })

  it('多专家自动匹配有阈值，未匹配到直接 404，前端不得假装总能出结果', () => {
    const text = handlerOf('multi_consult')
    expect(text).toContain('match_top_experts(&body.question, &reg, max_experts, 0.3')
    expect(text).toMatch(/matched\.is_empty\(\)[\s\S]*404, "未找到匹配的可用专家"/)
    const multi = collabMode(COLLAB_MODE.MULTI)
    expect(multi.expertChoice).toBe('optional')
    expect(collabProblem(multi, { question: 'q' })).toBe('')
    // 指定专家时后端不再排序过滤，只按 enabled 取前 max_experts 位
    expect(text).toContain('.take(max_experts)')
  })

  it('模式清单覆盖六个端点且各自声明渲染口径与结果标识', () => {
    expect(COLLAB_MODES.map((m) => m.key)).toEqual(['route', 'single', 'multi', 'debate', 'smart', 'algorithm'])
    for (const m of COLLAB_MODES) {
      expect(m.label, `${m.key} 缺文案`).toBeTruthy()
      expect(m.placeholder, `${m.key} 缺输入提示`).toBeTruthy()
      expect(['none', 'one', 'optional']).toContain(m.expertChoice)
      expect(['routing', 'answer', 'fusion', 'debate', 'complexity']).toContain(m.resultKind)
      expect(m.outcome, `${m.key} 未说明后端实际产出什么`).toBeTruthy()
    }
    // 单专家咨询的专家走路径参数，不出现在 body 里
    expect(collabBody(collabMode(COLLAB_MODE.SINGLE), { question: 'q', expertIds: ['e1'] })).toEqual({ question: 'q' })
    expect(collabProblem(collabMode(COLLAB_MODE.SINGLE), { question: 'q' })).toBe('请先选择 1 位专家')
    expect(collabProblem(collabMode(COLLAB_MODE.SMART), { question: 'q', expertIds: ['e1'] })).toContain('无需选择')
  })
})

// 两个动作面的形状门禁：/toggle-done 一个端点两条分支返回不同键集，实跑请求体的键集由
// dispatch handler 真正读取了哪些字段决定。这两件事都会随重构变，靠注释与记忆守不住。
describe('任务标记完成与分发实跑 ↔ toggle_task_done / dispatch', () => {
  const MODEL = src('frontend-ui/src/modules/expert-alliance/model/normalize.js')
  const STORE = src('frontend-ui/src/modules/expert-alliance/store/alliance-console.store.js')
  const VIEW = src('frontend-ui/src/modules/expert-alliance/views/AllianceConsoleView.vue')
  const RUNNER = src('frontend-ui/src/modules/expert-alliance/contract/dispatcher.js')
  const jsonKeys = (block) => [...block.matchAll(/"([a-z_]+)"\s*:/g)].map((m) => m[1])
  // 从签名起取到第一个顶格 }：Rust 侧函数体内部一律缩进，所以这一刀就是整个函数
  const fnOf = (text, sig) => text.match(new RegExp(`${sig}[\\s\\S]*?\\n\\}`))?.[0] || null
  const local = fnOf(ALLIANCE_RS, 'async fn toggle_task_done\\(')
  const remoteDone = fnOf(ALLIANCE_REMOTE_RS, 'pub async fn remote_toggle_done\\(')
  const remoteAction = fnOf(ALLIANCE_REMOTE_RS, 'pub async fn remote_task_action\\(')
  const dispatch = fnOf(EXPERTS_DISPATCHER_RS, 'async fn dispatch\\(')
  const dispatchTask = fnOf(EXPERTS_DISPATCHER_RS, 'pub fn dispatch_task\\(')

  it('两个面已从台账挂进清单，方法与路径逐字对齐注册行与 route 宏', () => {
    expect(local, '未找到 toggle_task_done handler').not.toBeNull()
    expect(dispatch, '未找到 dispatch handler').not.toBeNull()
    expect(ENDPOINTS.taskToggleDone).toEqual({
      registry: 'alliance.tasks.toggle_done', method: 'PUT', path: '/api/alliance/tasks/:id/toggle-done', nesting: 'nested'
    })
    expect(ENDPOINTS.dispatcherRun).toEqual({
      registry: 'experts.dispatch.dispatch', method: 'POST', path: '/api/experts/dispatcher/dispatch', nesting: 'flat'
    })
    for (const id of ['alliance.tasks.toggle_done', 'experts.dispatch.dispatch']) {
      expect(UNMOUNTED_ROUTES.some((x) => x.registry === id), `${id} 仍留在台账里`).toBe(false)
    }
    expect(ALLIANCE_RS).toContain('.route("/api/alliance/tasks/:id/toggle-done", put(toggle_task_done))')
    expect(EXPERTS_DISPATCHER_RS).toContain('.route("/api/experts/dispatcher/dispatch", post(dispatch))')
    expect(requestPath('taskToggleDone', { id: 't 1' })).toBe('/alliance/tasks/t%201/toggle-done')
  })

  it('同一端点两条分支的 data 键集不同，前端读的恰是两者并集', () => {
    const localKeys = jsonKeys(local.match(/\n {24}"data": \{([\s\S]*?)\n {24}\}/)[1])
    expect(localKeys).toEqual(['task_id', 'previous_status', 'current_status', 'toggled', 'completed_at', 'message'])
    const remoteData = remoteAction.match(/"data": \{([^}]*)\}/)[1]
    expect(jsonKeys(remoteData)).toEqual(['success', 'message'])
    // task_id 在远程落在与 data 同级的 params 里，剥完信封就取不到：所以 taskId 允许是空串
    expect(remoteData).not.toContain('task_id')
    expect(remoteAction).toMatch(/"params": \{\s*"task_id": task_id,/)
    const reads = [...(fnOf(MODEL, 'export function normToggleDone\\(').matchAll(/\bs\.([a-z_]+)/g))].map((m) => m[1])
    expect([...new Set(reads)].sort())
      .toEqual([...new Set([...localKeys, ...jsonKeys(remoteData)])].sort())
  })

  it('方向只由 toggled/success 判定：缺证据就是 unknown，200 不等于"已重开"', () => {
    expect(local).toMatch(/\(TaskStatus::Running, false\)/)
    expect(local).toMatch(/\(TaskStatus::Completed, true\)/)
    expect(normToggleDone({ toggled: true, current_status: 'completed' })).toMatchObject({ branch: 'local', direction: 'completed' })
    expect(normToggleDone({ toggled: false, current_status: 'running' })).toMatchObject({ branch: 'local', direction: 'reopened' })
    expect(normToggleDone({ success: true, message: '任务 t1 已标记为完成' })).toMatchObject({ branch: 'remote', direction: 'completed', taskId: '' })
    expect(normToggleDone({})).toMatchObject({ branch: 'unknown', direction: 'unknown' })
  })

  it('远程已完成任务经网关重开回 409，界面先把按钮挡住而不是等报错', () => {
    expect(remoteDone).toMatch(/current\["status"\]\.as_str\(\) == Some\("completed"\)/)
    expect(remoteDone).toMatch(/409,\s*format!\("任务 \{\} 已完成，远程任务不支持通过网关重新打开"/)
    expect(VIEW).toContain('reopenBlocked')
    expect(VIEW).toMatch(/current\.value\?\.status === TASK_STATUS\.COMPLETED && store\.runtime\?\.mode === 'remote'/)
    expect(VIEW).toMatch(/:title="reopenHint"/)
    expect(VIEW).toMatch(/const reopenHint = computed\([\s\S]*?409'/)
  })

  it('本地整批置完成会写执行表并落盘，所以 store 必须连节点一起重取', () => {
    expect(local).toContain('if n.status != NodeExecStatus::Failed && n.status != NodeExecStatus::Cancelled')
    expect(local).toMatch(/task\.progress = 1\.0/)
    expect(local).toMatch(/match s\.tasks\.save\(&task\) \{\s*Ok\(_\) => api_ok/)
    const act = fnOf(STORE, 'async function toggleTaskDone\\(')
    // Failed/Cancelled 节点不被后端改动：只刷任务不刷节点，界面会停在旧的节点配色上
    expect(act).toMatch(/api\.getNodes\(id\)/)
    expect(act).toMatch(/api\.getTask\(id\)/)
    expect(act).toMatch(/if \(selectedId\.value !== id\) return res/)
  })

  it('实跑只发 handler 读取的键：constraints 照收不读所以不发', () => {
    const struct = EXPERTS_DISPATCHER_RS.match(/pub struct DispatchBody \{([\s\S]*?)\n\}/)[1]
    const lines = struct.split('\n').map((l) => l.trim()).filter(Boolean)
    const declared = lines.map((l) => l.match(/pub ([a-z_]+):/)?.[1]).filter(Boolean)
    expect(declared).toEqual(['task_type', 'input', 'expert_ids', 'constraints'])
    const defaulted = lines.map((l, i) => (l === '#[serde(default)]' ? lines[i + 1].match(/pub ([a-z_]+):/)?.[1] : null)).filter(Boolean)
    expect([...defaulted].sort()).toEqual(['constraints', 'expert_ids'])
    const read = [...new Set([...dispatch.matchAll(/body\.([a-z_]+)/g)].map((m) => m[1]))].sort()
    expect(read).toEqual(['expert_ids', 'input', 'task_type'])
    expect(dispatch).not.toContain('body.constraints')
    expect(Object.keys(dispatchRunBody({ taskType: 't', input: 'i', expertIds: ['e1'] })).sort()).toEqual(read)
    // 无 serde default 的两个键必须恒在，否则整段 JSON 被 axum 拒收且拒绝体不是信封
    expect(Object.keys(dispatchRunBody())).toEqual(['task_type', 'input'])
    // 界面上那句"不发会被整段 JSON 拒绝"是对这条 serde 事实的转述，改口即判红
    expect(VIEW).toContain('该键无 serde 缺省')
  })

  it('strategy_used 取值集合由后端字面量决定，fallback 不是可配置项', () => {
    const produced = [...new Set([...dispatchTask.matchAll(/"([a-z_()]+)"\.into\(\)/g)].map((m) => m[1]))].sort()
    expect(produced).toEqual(['best_match', 'best_match(fallback)', 'least_load', 'round_robin', 'specified', 'weighted_random'])
    const selectable = DISPATCH_STRATEGY.map((s) => s.value)
    expect(produced.filter((p) => !selectable.includes(p)).sort()).toEqual(['best_match(fallback)', 'specified'])
    for (const p of produced) {
      const f = dispatchRunFindings({ strategyUsed: p, assigned: [] }, { strategy: p, weights: { e1: 1 } })
      expect(f.length, `${p} 没有对应结论`).toBeGreaterThan(0)
      expect(['info', 'success', 'danger', 'warning']).toContain(f[0].tone)
      expect(f[0].text).toContain(p === 'specified' ? '指定专家' : p)
    }
  })

  it('0.5 硬编码与"空串判满分"都取自后端字面量，判据跟着数字走', () => {
    const hardcoded = Number(EXPERTS_DISPATCHER_RS.match(/intelligent_matching \{\s*compute_match_score\(input, e\)\s*\} else \{\s*([\d.]+)\s*\};/)[1])
    const front = Number(RUNNER.match(/scores\.every\(\(v\) => v === ([\d.]+)\)/)[1])
    expect(front, '后端改了硬编码分而前端判据没跟').toBe(hardcoded)
    const withScore = (v) => dispatchRunFindings({ strategyUsed: 'best_match', assigned: [{ matchScore: v }] }, { strategy: 'best_match', weights: { e1: 1 } })
    expect(withScore(hardcoded)[1].tone).toBe('warning')
    expect(withScore(hardcoded)[1].text).toContain(String(hardcoded))
    expect(withScore(hardcoded + 0.1)).toHaveLength(1)

    const score = fnOf(EXPERTS_COMMON_RS, 'pub fn compute_match_score\\(')
    // 只有 bio 分支过滤空 token：领域/技能分支对空串 contains("") 恒真，所以空需求=全员判满分
    expect([...score.matchAll(/!q\.is_empty\(\)/g)]).toHaveLength(1)
    expect(score).toMatch(/let domain_hits[\s\S]*?\.contains\(q\) \|\| q\.contains/)
    expect(dispatchRunProblem({ input: '  ' })).toContain('领域匹配')
    expect(dispatchRunProblem({ input: '架构 微服务' })).toBe('')
  })

  it('无可用专家是 503 而非空结果，实跑不碰任何专家负载，失败留着上次结果', () => {
    expect(dispatch).toMatch(/assigned_ids\.is_empty\(\)[\s\S]{0,60}err\(503, "no available experts for dispatch"/)
    expect(dispatch).toContain('emit_audit(&state, &actor_from_opt_user(&user), AuditAction::ExpertDispatch')
    // 行动者不是装饰：未带身份时降级为 system，带身份时记真人 ⇒ 这两处形状变了，界面上"谁做的"就变了
    expect(dispatch).toContain('OptionalAuthUser(user): OptionalAuthUser')
    expect(EXPERTS_COMMON_RS).toContain('type Rejection = std::convert::Infallible')
    expect(EXPERTS_COMMON_RS).toContain('None => AuditActor::system()')
    expect(dispatchTask).not.toMatch(/current_load\s*=[^=]/)
    // 界面写"不会改动任何专家的 current_load"，这句承诺的权威源就是上面那条 not.toMatch
    expect(VIEW).toContain('但不会改动任何专家的 current_load')
    const run = fnOf(STORE, 'async function runDispatch\\(')
    expect(run).toContain('error.dispatch = e?.msg')
    // 上一次实跑自带 dispatch_id 与 created_at，能自证是哪一次的证据，失败时不得清空
    expect(run).not.toMatch(/dispatchResult\.value = null/)
    expect(VIEW).toMatch(/v-if="store\.error\.dispatch"/)
  })
})

// ── 专家注册面（POST / PUT / DELETE /api/experts）────────────────────
// 本组把 contract/registry.js 的每一项声明钉回 merge_expert_from_value 与三条 handler：
// 后端加字段、改校验、或者反过来给了「再启用」的端点，这里先红。
describe('专家注册面契约 ↔ merge_expert_from_value / create|update|delete_expert', () => {
  const rsFn = (text, sig) => text.match(new RegExp(`${sig}[\\s\\S]*?\\n\\}`))?.[0] || ''
  const MERGE = rsFn(EXPERTS_REGISTRY_RS, 'fn merge_expert_from_value')
  const CREATE = rsFn(EXPERTS_REGISTRY_RS, 'async fn create_expert')
  const UPDATE = rsFn(EXPERTS_REGISTRY_RS, 'async fn update_expert')
  const REMOVE = rsFn(EXPERTS_REGISTRY_RS, 'async fn delete_expert')
  const FORM = src('frontend-ui/src/modules/expert-alliance/components/ExpertRegistryForm.vue')

  // 后端认识的键：顶层来自 body.get(...)，嵌套来自 availability / metrics 两支的 av.get / mt.get
  const TOP = new Set([...MERGE.matchAll(/body\.get\("([a-zA-Z_]+)"\)/g)].map((m) => m[1]))
  const AV = new Set([...MERGE.matchAll(/av\.get\("([a-z_]+)"\)/g)].map((m) => m[1]))
  const MT = new Set([...MERGE.matchAll(/mt\.get\("([a-z_]+)"\)/g)].map((m) => m[1]))

  // 不带点的键（metrics / metadata）指整个子对象，认识的判据就是顶层 body.get
  const knownInBackend = (key) => {
    const [head, sub] = key.split('.')
    if (sub === undefined) return TOP.has(head)
    if (head === 'availability') return AV.has(sub)
    if (head === 'metrics') return MT.has(sub)
    return TOP.has(head)
  }

  it('merge 被解析出来了，否则本组是空转', () => {
    expect(MERGE.length).toBeGreaterThan(500)
    expect(TOP.size).toBeGreaterThan(15)
    expect(AV.size).toBe(5)
    expect(MT.size).toBe(7)
  })

  it('挂载项、未挂载项、别名项都在后端白名单里', () => {
    const declared = new Set()
    for (const f of EXPERT_REGISTER_FIELDS) {
      expect(knownInBackend(f.wire), `契约发的 ${f.wire} 后端不认（发了就被静默丢弃）`).toBe(true)
      declared.add(f.wire.split('.')[0])
    }
    for (const x of EXPERT_UNMOUNTED_FIELDS) {
      if (x.source === 'create_expert') {
        expect(CREATE.includes(`body.get("${x.key}")`), `${x.key} 不再是 create_expert 读的键`).toBe(true)
      } else {
        expect(knownInBackend(x.key), `${x.key} 不再是 merge 认识的键`).toBe(true)
      }
      declared.add(x.key.split('.')[0])
    }
    for (const a of EXPERT_ALIAS_FIELDS) {
      expect(TOP.has(a.key), `${a.key} 不再是 merge 的兼容别名`).toBe(true)
      declared.add(a.key)
    }
  })

  it('后端新增字段时本模块必须先表态（正向覆盖）', () => {
    const covered = new Set([
      ...EXPERT_REGISTER_FIELDS.map((f) => f.wire),
      ...EXPERT_UNMOUNTED_FIELDS.map((x) => x.key),
      ...EXPERT_ALIAS_FIELDS.map((a) => a.key)
    ])
    for (const k of TOP) expect(covered.has(k) || [...covered].some((c) => c.startsWith(`${k}.`)), `merge 新增了 ${k}`).toBe(true)
    for (const k of AV) expect(covered.has(`availability.${k}`)).toBe(true)
    for (const k of MT) expect(covered.has(`metrics`) || covered.has(`metrics.${k}`), `metrics 新增了 ${k}`).toBe(true)
  })

  it('两条 400 分支仍在，前端校验文案据此而来', () => {
    expect(CREATE).toMatch(/err\(400, "expert name is required"\)/)
    expect(CREATE).toMatch(/expert id already exists/)
    // 查重用的是整表 contains_key（含停用记录），所以「删了再注册同一个 id」必 400
    expect(CREATE).toMatch(/reg\.contains_key\(&id\)/)
    expect(expertDraftProblem({ name: '  ' })).toMatch(/不能为空/)
  })

  it('PUT 要求 enabled、DELETE 写 false，且全网关没有再启用的写入口', () => {
    expect(UPDATE).toMatch(/Some\(exp\) if exp\.enabled =>/)
    expect(UPDATE).toMatch(/err\(404, format!\("expert not found/)
    expect(REMOVE).toContain('exp.enabled = false;')
    expect(REMOVE).toContain('"soft_delete": true')
    const dir = path.join(ROOT, 'platform/gateway/mox-platform-gateway-svc/src/alliance')
    const all = readdirSync(dir).filter((f) => f.endsWith('.rs')).map((f) => readFileSync(path.join(dir, f), 'utf8')).join('\n')
    expect(all, '出现了把 enabled 写回 true 的赋值，删除不再是单向操作').not.toMatch(/\.enabled\s*=\s*true/)
    // 唯一为 true 的地方是构造默认值，即「重新注册一位新专家」
    expect(EXPERTS_COMMON_RS).toContain('enabled: true')
  })

  it('软删会落盘，重启也回不来', () => {
    expect(REMOVE).toContain('save_registry(&reg);')
    const DB = src('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_db.rs')
    expect(DB).toMatch(/enabled\s+INTEGER NOT NULL DEFAULT 1/)
    expect(DB).toMatch(/e\.enabled as i64/)
  })

  it('字符串能力简写会伪造熟练度，所以契约走对象形并要求手填', () => {
    expect(MERGE).toMatch(/proficiency:\s*85/)
    expect(MERGE).toMatch(/format!\("cap-\{\}", name\)/)
    expect(MERGE).toMatch(/from_value::<ExpertCapability>\(v\.clone\(\)\)\.ok\(\)/)
    expect(EXPERTS_COMMON_RS).toMatch(/pub proficiency: u8/)
    expect(expertFieldProblem('capabilities', [{ name: 'a', proficiency: EXPERT_PROFICIENCY_MAX + 1 }])).toMatch(/u8/)
  })

  it('u32 截断仍在，这是时薪与并发上限取 2³²−1 的依据', () => {
    expect(MERGE).toMatch(/exp\.hourly_rate_cents = rate as u32/)
    expect(MERGE).toMatch(/max_concurrent = mc as u32/)
    expect(EXPERT_U32_MAX).toBe(0xffffffff)
    expect(expertFieldProblem('hourlyRateCents', EXPERT_U32_MAX + 1)).toMatch(/截/)
    expect(expertFieldProblem('maxConcurrent', 12.5)).toMatch(/整数/)
  })

  it('停用后四个读面都过滤 enabled，所以"消失"是四处同时发生', () => {
    const filters = [...EXPERTS_REGISTRY_RS.matchAll(/\.enabled\b/g)].length
    expect(filters, 'registry.rs 里的 enabled 判定变少了，删除后果清单要重写').toBeGreaterThanOrEqual(6)
    expect(EXPERTS_DISPATCHER_RS).toMatch(/\.filter\(\|e\| e\.enabled\)/)
    const lines = deleteConsequences({ name: '张三' })
    expect(lines.length).toBeGreaterThanOrEqual(4)
    for (const line of lines) expect(line).toMatch(/\.rs[:\d]/)
  })

  it('表单与 store 不绕过契约', () => {
    // 字段清单来自契约，视图里不得再手写一份 el-form-item 列表
    expect(FORM).toContain('EXPERT_REGISTER_FIELDS')
    for (const banned of ['avg_rating', 'current_load', 'total_consultations']) {
      expect(FORM, `表单出现了未挂载字段 ${banned}`).not.toContain(banned)
    }
    const store = src('frontend-ui/src/modules/expert-alliance/store/alliance-experts.store.js')
    const save = store.match(/async function saveExpert\([\s\S]*?\n  \}/)?.[0] || ''
    expect(save).toContain('expertPatch(')
    expect(save).toMatch(/Object\.keys\(patch\)\.length/)
    expect(save, '空 patch 也发请求会白盖 updated_at').toMatch(/notice\.value = '没有需要保存的改动'/)
    const api = src('frontend-ui/src/modules/expert-alliance/api/alliance.api.js')
    expect(api).toMatch(/registerExpert[\s\S]{0,200}registerBody\(draft\)/)
    expect(api).toMatch(/call\(httpClient, 'expertUpdate'/)
  })

  it('改删两条复用 detail 的 ANY 行', () => {
    expect(ENDPOINTS.expertUpdate.registry).toBe(ENDPOINTS.expertDetail.registry)
    expect(ENDPOINTS.expertDelete.registry).toBe(ENDPOINTS.expertDetail.registry)
    expect(ENDPOINTS.expertUpdate.path).toBe('/api/experts/:id')
    expect(ENDPOINTS.expertDelete.method).toBe('DELETE')
  })

  it('写面身份的前提都在源码里：认证默认开、联盟域无角色判定、登录由路由守卫统一管', () => {
    const cfg = src('platform/gateway/mox-platform-gateway-svc/src/config.rs')
    const def = cfg.match(/impl Default for AuthConfig \{[\s\S]*?\n\}/)?.[0] || ''
    expect(def, 'AuthConfig 默认值取不到了，"认证默认开启"就成了无据的话').toContain('enabled: true')
    const mods = src('platform/gateway/mox-platform-gateway-svc/src/modules.rs')
    expect(mods, '业务路由不再统一挂 auth_middleware，写请求的身份前提变了').toMatch(/route_layer\([\s\S]{0,160}auth_middleware/)
    // 认证在、授权不在：联盟域 handler 没有一处读调用方身份
    const dir = path.join(ROOT, 'platform/gateway/mox-platform-gateway-svc/src/alliance')
    const alliance = readdirSync(dir).filter((f) => f.endsWith('.rs')).map((f) => readFileSync(path.join(dir, f), 'utf8')).join('\n')
    // 认证在、授权不在：联盟 handler 读调用方身份只为写审计 actor，不据身份拒绝。
    // 判据只看可执行行——experts_common.rs:593 的文档注释里提了一句 auth.rs `ApiAuth`，
    // 那是"我们为何手写 async_trait 展开"的说明，不是角色判定。
    const allianceCode = alliance.split('\n').filter((l) => !/^\s*\/\//.test(l)).join('\n')
    expect(allianceCode, '后端开始做角色判定了，界面提示要改成按角色而不是按身份')
      .not.toMatch(/require_role|has_role|check_permission|ensure_admin|roles\.contains/)
    expect(allianceCode, '联盟写路径换成强制身份提取器（缺 token 会 401），界面上的身份口径要重测')
      .not.toMatch(/:\s*ApiAuth\b|Extension<UserInfo>|current_user/)
    // 403 可以有，但只能落在"对象状态"上而不是"调用方是谁"上：现存唯一一处是协作面对已禁用专家的拒绝。
    // 逐枚点名 ⇒ 新增一处 403 就必须先来说明它拒的是谁。
    const forbidden = allianceCode.split('\n').filter((l) => /err\(403/.test(l)).map((l) => l.trim())
    expect(forbidden, `403 站点数从 1 变成 ${forbidden.length}：每一处都要重新判定它拒的是身份还是对象状态`).toHaveLength(1)
    expect(forbidden[0]).toContain('已被禁用')
    // 正对照：真出现角色判定时上面几条必须认得出（否则禁令是死的）
    const poisoned = 'if has_role(user, "admin") { ... }\nlet u: ApiAuth = ...;\nreturn err(403, "无权修改他人专家");'
    expect(/require_role|has_role|check_permission/.test(poisoned), '角色判定禁令失能').toBe(true)
    expect(/:\s*ApiAuth\b/.test(poisoned), '强制身份提取器禁令失能').toBe(true)
    expect(/无权/.test(poisoned.split('\n')[2]), '按身份拒绝的 403 与按状态拒绝的 403 必须可区分').toBe(true)
    // 而注释里那句 ApiAuth 确实存在——它不该被算作缺陷，这条把"为什么要剥注释"钉住
    expect(alliance, 'experts_common.rs 的 ApiAuth 说明文字被改掉了，剥注释这一步不再有必要').toContain('与 auth.rs `ApiAuth` 同策略')
    // 登录与否归外壳路由守卫，模块页不再造一套置灰（那会让人以为模块懂权限）
    expect(src('frontend-ui/src/router/index.js'), '路由不再要求身份，写面提示要重做').toMatch(/if \(!token\) \{/)
    const view = src('frontend-ui/src/modules/expert-alliance/views/AllianceExpertsView.vue')
    expect(view).toContain('EXPERT_WRITE_IDENTITY.statement')
    expect(view, '模块页自己判角色等于替后端编造授权模型').not.toMatch(/isAdmin|hasRole|isLoggedIn/)
    expect(EXPERT_WRITE_IDENTITY.statement).toMatch(/没有角色判定/)
    expect(EXPERT_WRITE_IDENTITY.statement, '提示里出现权限措辞，等于替后端编造角色判定').not.toMatch(/管理员|无权限|权限不足|授权/)
    // 证据锚点必须真的指到它声称的那一行：界面会把 file:line 印出来给用户看，行号漂了就是谎。
    const ANCHOR_FILES = {
      'config.rs': 'platform/gateway/mox-platform-gateway-svc/src/config.rs',
      'modules.rs': 'platform/gateway/mox-platform-gateway-svc/src/modules.rs',
      'router/index.js': 'frontend-ui/src/router/index.js',
      'experts_common.rs': 'platform/gateway/mox-platform-gateway-svc/src/alliance/experts_common.rs',
      'experts_dispatcher.rs': 'platform/gateway/mox-platform-gateway-svc/src/alliance/experts_dispatcher.rs',
      'experts_collaboration.rs': 'platform/gateway/mox-platform-gateway-svc/src/alliance/experts_collaboration.rs'
    }
    const ANCHOR_TEXT = {
      'config.rs:41': 'enabled: true',
      'modules.rs:234': 'modules.route_layer(',
      'router/index.js:74': 'if (!token) {',
      'experts_common.rs:585': 'pub struct OptionalAuthUser',
      'experts_dispatcher.rs:588': 'AuditAction::ExpertDispatch',
      'experts_collaboration.rs:797': '已被禁用'
    }
    for (const ref of EXPERT_WRITE_IDENTITY.evidence) {
      expect(ref, '身份提示缺了后端位置').toMatch(/\.[a-z]+:\d+/)
      const base = ref.slice(0, ref.lastIndexOf(':'))
      const line = Number(ref.slice(ref.lastIndexOf(':') + 1))
      const file = ANCHOR_FILES[base]
      expect(file, `证据 ${ref} 的文件不在锚点表里，等于没人核对它`).toBeTruthy()
      const at = src(file).split('\n')[line - 1] || ''
      // 缺锚点文本时不要退化成 toContain(undefined)：那一格的报错会指向错的地方
      expect(ANCHOR_TEXT[ref], `证据 ${ref} 没登记「这一行该写什么」`).toBeTruthy()
      expect(at, `证据 ${ref} 指的那一行不含「${ANCHOR_TEXT[ref]}」⇒ 行号漂了`).toContain(ANCHOR_TEXT[ref])
    }
    // 分母：登记了锚点的证据条数，与提示里印出的证据数必须同为 5
    expect(EXPERT_WRITE_IDENTITY.evidence.length).toBe(Object.keys(ANCHOR_TEXT).length)
  })

  it('写失败要留在发起它的那个弹窗里，不能只剩一条会自己消失的 toast', () => {
    const view = src('frontend-ui/src/modules/expert-alliance/views/AllianceExpertsView.vue')
    const store = src('frontend-ui/src/modules/expert-alliance/store/alliance-experts.store.js')
    // store 侧：写操作的失败统一落进 error.action，界面才有可留痕的东西
    expect(store).toMatch(/error\.action = e\?\.msg \|\| e\?\.message/)
    // 弹窗盖住页面，页面级 alert 在弹窗内不可见 ⇒ 两个写弹窗各自带一条错误
    for (const [model, title] of [['registryVisible', '注册 / 编辑'], ['disableVisible', '停用']]) {
      const dlg = view.match(new RegExp(`<el-dialog v-model="${model}"[\\s\\S]*?<\\/el-dialog>`))?.[0] || ''
      expect(dlg, `${title} 弹窗取不到了`).not.toBe('')
      expect(dlg, `${title} 失败时弹窗内没有留痕，用户只会看到一条转瞬即逝的 toast`).toMatch(
        /v-if="store\.error\.action"[\s\S]*?:description="store\.error\.action"/
      )
    }
    // 失败即留在弹窗里改，成功才关窗
    expect(view).toMatch(/if \(!saved && store\.error\.action\) return/)
    expect(view).toMatch(/const res = await store\.removeExpert\(disableTarget\.value\)\s*\n\s*if \(!res\) return/)
    // 开面先清旧错：否则上一次收藏失败会被读成"这次注册被后端拒了"
    expect(view).toMatch(/watch\(\[registryVisible, disableVisible\][\s\S]*?if \(reg \|\| dis\) store\.error\.action = ''/)
    // 关闭页面级错误条清的是这条错，不是把还没显示的 success notice 顺手抹掉
    expect(view).toMatch(/:description="store\.error\.action" @close="store\.error\.action = ''"/)
  })
})

describe('联盟 DAG 面 ↔ alliance.rs', () => {
  const CONSOLE_VUE = src('frontend-ui/src/modules/expert-alliance/views/AllianceConsoleView.vue')
  const NORMALIZE_JS = src('frontend-ui/src/modules/expert-alliance/model/normalize.js')
  const DAG_MODEL_JS = src('frontend-ui/src/modules/expert-alliance/model/dag.js')

  const rsFn = (sig) => {
    const hit = ALLIANCE_RS.match(new RegExp(`${sig}[\\s\\S]*?\\n\\}`))
    if (!hit) throw new Error(`alliance.rs 缺少 ${sig}`)
    return hit[0]
  }
  const DAG_FN = rsFn('async fn get_task_dag\\(')
  const NODE_LIT = DAG_FN.match(/json!\(\{[\s\S]*?"dependencies"[\s\S]*?\}\)/)[0]
  const EDGE_LIT = DAG_FN.match(/json!\(\{\s*"source":[\s\S]*?\}\)/)[0]
  const STATS_LIT = DAG_FN.match(/"stats": \{([\s\S]*?)\n\s+\}/)[1]
  const topKeys = (block) => [...block.matchAll(/^[ \t]+"([a-z_]+)":/gm)].map((m) => m[1])

  it('NodeExecStatus 每个变体都有 wire 出口、中文标签与 DAG 节点样式', () => {
    const variants = [...rsFn('pub enum NodeExecStatus \\{').matchAll(/^\s{4}([A-Z]\w*),$/gm)].map((m) => m[1])
    expect(variants.length).toBeGreaterThanOrEqual(6)
    const arms = [...rsFn('fn node_status_str').matchAll(/NodeExecStatus::(\w+) => "([a-z_]+)"/g)]
    expect(arms.map((a) => a[1])).toEqual(variants)
    const wire = arms.map((a) => a[2])
    expect(new Set(wire).size).toBe(wire.length)
    for (const w of wire) {
      expect(Object.values(NODE_STATUS)).toContain(w)
      expect(NODE_STATUS_LABELS[w], `节点状态 ${w} 没有中文标签`).toBeTruthy()
      expect(CONSOLE_VUE, `节点状态 ${w} 在 DAG 图上无样式`).toMatch(new RegExp(`\\.ac-dag-node\\.is-${w}\\b`))
    }
  })

  it('node_stats 把 skipped 与 cancelled 折叠成第 6 格，界面按节点级分开计数', () => {
    const slots = rsFn('fn node_stats').match(/\(\s*self\.nodes\.len\(\),\s*([^)]+)\)/)[1]
      .split(',')
      .map((s) => s.trim())
    expect(slots).toEqual(['completed', 'running', 'failed', 'pending', 'skipped + cancelled'])
    expect(DAG_FN).toContain('let (total, completed, running, failed, pending, other) = exec.node_stats();')
    expect(DAG_FN).toContain('"skipped": other')
    expect(CONSOLE_VUE).toMatch(/跳过 \{\{ dagView\.tally\.skipped \}\}/)
    expect(CONSOLE_VUE).toMatch(/取消 \{\{ dagView\.tally\.cancelled \}\}/)
    expect(DAG_MODEL_JS).toMatch(/tally\[status\]/)
  })

  it('DAG 出参三处键集与前端读取一一对上', () => {
    expect(topKeys(NODE_LIT)).toEqual([
      'id', 'label', 'name', 'type', 'expert_id', 'status', 'progress', 'dependencies', 'started_at', 'completed_at', 'duration_ms', 'position'
    ])
    expect(topKeys(EDGE_LIT)).toEqual(['source', 'target', 'label'])
    expect(topKeys(STATS_LIT)).toEqual(['total', 'completed', 'running', 'pending', 'failed', 'skipped'])
    const normNode = NORMALIZE_JS.match(/export function normNode\([\s\S]*?\n\}/)[0]
    const reads = new Set([...normNode.matchAll(/\bn\.([a-z_]+)/g)].map((m) => m[1]))
    for (const k of topKeys(NODE_LIT)) {
      // type 后端恒为 "expert"，界面不按它分支，所以不要求被读取
      if (k === 'type') { expect(NODE_LIT).toMatch(/"type": "expert",/); continue }
      expect(reads.has(k), `normNode 未读后端键 ${k}`).toBe(true)
    }
    const normDag = NORMALIZE_JS.match(/export function normDag\([\s\S]*?\n\}/)[0]
    for (const k of topKeys(STATS_LIT)) expect(normDag).toContain(`payload?.stats?.${k}`)
    for (const k of ['nodes', 'edges', 'stats']) expect(normDag).toContain(`payload?.${k}`)
  })

  it('视图对 dag.stats 的每次取数都落在后端产出的键集内', () => {
    const produced = topKeys(STATS_LIT)
    const reads = [...new Set([...CONSOLE_VUE.matchAll(/dag\??\.stats\.([A-Za-z_]\w*)/g)].map((m) => m[1]))]
    expect(reads.length).toBeGreaterThan(0)
    for (const r of reads) expect(produced, `界面读了后端 DAG stats 不产出的 ${r}`).toContain(r)
  })

  it('分层与依赖计数同源：页脚不用后端 edges 数组当依赖数，算法住在 model 层', () => {
    expect(CONSOLE_VUE).toMatch(/\{\{ dagView\.drawn \}\} 条依赖参与分层/)
    expect(CONSOLE_VUE).not.toMatch(/dag\.edges\.length/)
    expect(CONSOLE_VUE).toMatch(/dagView\.edgeDelta/)
    expect(CONSOLE_VUE).toMatch(/dagView\.backEdges/)
    expect(CONSOLE_VUE).toMatch(/layoutDag\(store\.detail\.dag\?\.nodes \?\? \[\], store\.detail\.dag\?\.edges \?\? \[\]\)/)
    expect(CONSOLE_VUE).not.toMatch(/const buckets = new Map\(\)/)
  })
})

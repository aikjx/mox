// 调度器配置契约：前端字段名、边界、默认值与合并语义逐条对齐网关源码。
// 后端改校验分支或改默认值而没同步前端时，本文件先红。
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { readFileSync, existsSync } from 'node:fs'
import path from 'node:path'
import { createPinia, setActivePinia } from 'pinia'

// store 只经由 allianceApi 取数：把它换成 spies，同时保留 createAllianceApi 本体供取数层用例使用
const { cfgApi } = vi.hoisted(() => ({
  cfgApi: {
    getDispatcherConfig: vi.fn(), updateDispatcherConfig: vi.fn(),
    dispatcherStatus: vi.fn(), resetDispatcherLoad: vi.fn(), resetAllDispatcherLoads: vi.fn(),
    runDispatch: vi.fn(), toggleTaskDone: vi.fn(), listExperts: vi.fn(), getTask: vi.fn(), getNodes: vi.fn()
  }
}))
vi.mock('../api/alliance.api.js', async (importOriginal) => ({
  ...(await importOriginal()),
  allianceApi: cfgApi
}))

import {
  DISPATCH_CONFIG_FIELDS, DISPATCH_ENGINE_STATUS_LITERAL, DISPATCH_RESET_ALL_CONFIRM,
  DISPATCH_STATUS_KEYS, DISPATCH_STRATEGY, DISPATCH_UNMOUNTED_FIELDS,
  breakerEmptyNote, dispatchResetAllNotice, dispatchResetBody, dispatchResetLines, dispatchResetNotice,
  dispatchField, dispatchInvalid, dispatchPatch, dispatchProblem, dispatchRows, dispatchStrategyLabel, successRateNote,
  dispatchRunBody, dispatchRunFindings, dispatchRunProblem
} from './dispatcher.js'
import { ENDPOINTS } from './endpoints.js'
import { createAllianceApi } from '../api/alliance.api.js'

const { useAllianceConsoleStore } = await import('../store/alliance-console.store.js')

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
const src = (rel) => readFileSync(path.join(ROOT, rel), 'utf8').replace(/\r\n/g, '\n')

const DISPATCHER_RS = src('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_dispatcher.rs')
const COMMON_RS = src('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_common.rs')
const REGISTRY = src('docs/API-REGISTRY.md')

const UPDATE_FN = DISPATCHER_RS.match(/async fn update_config\([\s\S]*?\n\}/)[0]
const CONFIG_STRUCT = COMMON_RS.match(/pub struct DispatcherConfig \{[\s\S]*?\n\}/)[0]
const BODY_STRUCT = DISPATCHER_RS.match(/pub struct UpdateConfigBody \{[\s\S]*?\n\}/)[0]

const REGISTRY_RS = src('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_registry.rs')
const NORM_SRC = src('frontend-ui/src/modules/expert-alliance/model/normalize.js')
const CONSOLE_VUE = src('frontend-ui/src/modules/expert-alliance/views/AllianceConsoleView.vue')
const DISPATCHER_PROD = DISPATCHER_RS.split('#[cfg(test)]')[0]

const RESET_ONE = DISPATCHER_RS.match(/async fn reset_expert\([\s\S]*?\n\}/)[0]
const RESET_ALL = DISPATCHER_RS.match(/async fn reset_all\([\s\S]*?\n\}/)[0]
const STATUS_FN = DISPATCHER_RS.match(/async fn dispatcher_status\([\s\S]*?\n\}/)[0]
const RESET_BODY_STRUCT = DISPATCHER_RS.match(/pub struct ResetBody \{[\s\S]*?\n\}/)[0]
/** dispatcher_status 的 ok(json!({...})) 出参键：它是 DISPATCH_STATUS_KEYS 的唯一权威 */
const STATUS_JSON_AT = STATUS_FN.indexOf('ok(json!({')
const STATUS_OUT_KEYS = [...STATUS_FN.slice(STATUS_JSON_AT).matchAll(/"([a-z_]+)"\s*:/g)].map((m) => m[1])
/** 归一化对 status 产出的键（少一个就是界面少一格读数，多一个就是前端发明了字段） */
const NORM_STATUS_KEYS = [...NORM_SRC.match(/export function normDispatcherStatus\([\s\S]*?\n\}/)[0]
  .matchAll(/^ {4}([A-Za-z][A-Za-z0-9_]*):/gm)].map((m) => m[1])
/** 控制台界面对 status 读数实际伸手取的键（`?.` 与 `.` 两种写法都算） */
const VIEW_STATUS_READS = [...new Set([...CONSOLE_VUE.matchAll(/dispatcherStatus\??\.\s*([A-Za-z_][A-Za-z0-9_]*)/g)].map((m) => m[1]))]

const fieldsOf = (block) => [...block.matchAll(/pub ([a-z_]+):\s*/g)].map((m) => m[1])
/**
 * `#[serde(default = "default_x")]` 指向的默认函数返回值。
 * 必须按行回看属性：Rust 字段以逗号结尾，跨字段的正则会一路匹到结构体里第一个属性，
 * 于是所有字段都拿到同一个默认值（夹具自身踩坑，实测过一次）。
 */
const serdeDefault = (block, key) => {
  const lines = block.split('\n')
  const idx = lines.findIndex((l) => new RegExp(`pub ${key}:\\s*[\\w<>,\\s]+`).test(l))
  if (idx < 0) throw new Error(`${key} 不在该结构体里`)
  let fn = null
  for (let i = idx - 1; i >= 0; i -= 1) {
    const attr = /#\[serde\(([^)]*)\)\]/.exec(lines[i])
    if (attr) {
      fn = /default = "([a-z_]+)"/.exec(attr[1])?.[1] || null
      break
    }
    if (!/^\s*(\/\/\/|#!|\s*$)/.test(lines[i])) break
  }
  const body = fn ? (new RegExp(`fn ${fn}\\(\\)[^}]*\\{([^}]*)\\}`).exec(COMMON_RS)?.[1] ?? '') : ''
  const num = /(-?\d+\.?\d*)/.exec(body)
  const str = /"([^"]+)"/.exec(body)
  const isBool = new RegExp(`pub ${key}:\\s*bool`).test(lines[idx])
  const boolHit = isBool ? /\b(true|false)\b/.exec(body) : null
  return {
    literal: str
      ? str[1]
      : (num ? Number(num[1])
        : (boolHit ? boolHit[1] === 'true'
          : (fn === null ? (isBool ? false : 0) : null))),
    via: fn
  }
}

const CONTRACT_KEYS = DISPATCH_CONFIG_FIELDS.map((f) => f.key)

describe('调度器配置契约', () => {
  it('策略清单与后端 valid 数组等集且同序', () => {
    const valid = [...UPDATE_FN.match(/let valid = \[([^\]]*)\]/)[1].matchAll(/"([a-z_]+)"/g)].map((m) => m[1])
    expect(DISPATCH_STRATEGY.map((s) => s.value)).toEqual(valid)
    // 展示名不得与传输名混淆：每项都要有中文文案，且不同策略不同文案
    const labels = DISPATCH_STRATEGY.map((s) => s.label)
    expect(new Set(labels).size).toBe(valid.length)
    expect(labels.every((l) => /[一-龥]/.test(l))).toBe(true)
    expect(dispatchStrategyLabel('least_load')).toBe('最小负载')
    expect(dispatchStrategyLabel('unknown_x')).toBe('unknown_x')
  })

  it('契约字段并集正好盖住 PUT 结构体，多一个少一个都算漂移', () => {
    expect([...CONTRACT_KEYS, ...DISPATCH_UNMOUNTED_FIELDS.map((f) => f.key)].sort())
      .toEqual(fieldsOf(BODY_STRUCT).sort())
    for (const u of DISPATCH_UNMOUNTED_FIELDS) {
      expect(u.reason.length, `${u.key} 未挂载却缺原因`).toBeGreaterThan(8)
      expect(CONTRACT_KEYS, `${u.key} 既挂载又登记为未挂载`).not.toContain(u.key)
    }
  })

  it('GET 响应字段面与 DispatcherConfig 等集，视图不会读到不存在的键', () => {
    expect(fieldsOf(CONFIG_STRUCT).sort())
      .toEqual([...CONTRACT_KEYS, ...DISPATCH_UNMOUNTED_FIELDS.map((f) => f.key)].sort())
  })

  it('checked 标记与后端是否真的执法一致，边界取 400 文案里的区间', () => {
    for (const f of DISPATCH_CONFIG_FIELDS) {
      // 合并式更新：每个键都得有 Some 分支，否则前端改了不生效
      expect(new RegExp(`if let Some\\([^)]*\\) = body\\.${f.key}`).test(UPDATE_FN),
        `update_config 不再有 body.${f.key} 的合并分支`).toBe(true)
      if (f.kind === 'switch') {
        expect(f.checked, `${f.key} 是布尔开关，后端无区间可校验`).toBe(false)
        continue
      }
      if (f.kind === 'select') {
        // 策略的执法方式是 valid 数组 + invalid strategy 文案，不是区间
        expect(/let valid = \[/.test(UPDATE_FN) && /invalid strategy/.test(UPDATE_FN))
          .toBe(f.checked)
        continue
      }
      // 后端是否为该键写了 400 文案 —— 这就是前端 checked 的唯一依据
      const rejected = new RegExp(`${f.key} must be ([\\d.]+)-([\\d.]+)`).exec(UPDATE_FN)
      expect(f.checked, `${f.key} 的 checked 与后端 400 分支不符`).toBe(!!rejected)
      if (!rejected) {
        expect(f.hint, `${f.key} 后端不校验，前端却冒充执法者`).toContain('后端不校验')
        continue
      }
      expect([f.min, f.max]).toEqual([Number(rejected[1]), Number(rejected[2])])
    }
    // 文案之外还要有真判据：0.0..=1.0 / 1..=3600 / r > 10
    expect(UPDATE_FN).toContain('0.0..=1.0')
    expect(UPDATE_FN).toContain('1..=3600')
    expect(Number(/r > (\d+)/.exec(UPDATE_FN)[1])).toBe(dispatchField('max_retries').max)
  })

  it('默认值取后端 default_* 函数体，不靠记忆', () => {
    for (const f of DISPATCH_CONFIG_FIELDS) {
      const via = serdeDefault(CONFIG_STRUCT, f.key)
      expect(via.via, `${f.key} 的默认值须来自具名 default_* 函数，而不是 serde 隐式零值`).toBeTruthy()
      expect(via.literal, `${f.key} 的默认值与后端不符`).toBe(f.default)
    }
    // 「后端默认」字样只能标在真正的默认策略上，且只能有一项
    const labeled = DISPATCH_STRATEGY.filter((s) => s.label.includes('后端默认')).map((s) => s.value)
    expect(labeled).toEqual([serdeDefault(CONFIG_STRUCT, 'strategy').literal])
  })

  it('两条配置端点已登记进 API-REGISTRY，且方法与路径逐字对齐', () => {
    for (const name of ['dispatcherConfig', 'dispatcherConfigUpdate']) {
      const ep = ENDPOINTS[name]
      expect(REGISTRY, `${ep.path} 未登记`).toContain(ep.path)
      expect(REGISTRY).toContain(`| \`${ep.registry}\` | ${ep.method} | \`${ep.path}\` |`)
      expect(ep.nesting).toBe('flat')
    }
  })

  it('前端校验与后端 400 分支互斥：越界的值一律发不出去', () => {
    expect(dispatchProblem('match_threshold', 1)).toBe('')
    expect(dispatchProblem('match_threshold', 1.2)).toContain('0–1')
    expect(dispatchProblem('max_retries', 11)).toContain('0–10')
    expect(dispatchProblem('timeout_seconds', 0)).toContain('1–3600')
    expect(dispatchProblem('strategy', 'random')).toContain('round_robin')
    // 后端不校验的项前端不冒充执法者，只挡明显不可用的输入
    expect(dispatchProblem('circuit_breaker_threshold', 999)).toBe('')
    expect(dispatchProblem('circuit_breaker_threshold', 'abc')).toContain('必须是数字')
    expect(dispatchProblem('intelligent_matching', 'yes')).toContain('必须是布尔值')
    expect(dispatchProblem('nope', 1)).toContain('未知配置项')
  })

  it('合并式更新：与后端同值的键不进请求体', () => {
    const current = { strategy: 'best_match', match_threshold: 0.3, max_retries: 3, timeout_seconds: 120, circuit_breaker_threshold: 5, intelligent_matching: true, concurrency_control: true }
    expect(dispatchPatch(current, {})).toEqual({ patch: {}, problem: '' })
    expect(dispatchPatch(current, { match_threshold: 0.3 }).patch).toEqual({})
    expect(dispatchPatch(current, { match_threshold: '0.45', max_retries: 5, intelligent_matching: false }).patch)
      .toEqual({ match_threshold: 0.45, max_retries: 5, intelligent_matching: false })
    // 一项越界：拦下来并给出原因，其余合法改动也不偷发
    const bad = dispatchPatch(current, { match_threshold: 2, max_retries: 5 })
    expect(bad.patch).toEqual({ max_retries: 5 })
    expect(bad.problem).toContain('0–1')
    expect(Object.keys(dispatchInvalid({ ...current, timeout_seconds: 9999 })).sort()).toEqual(['timeout_seconds'])
  })

  it('行派生：后端没返回的键标 missing 而不是伪造成功', () => {
    const rows = dispatchRows({ strategy: 'least_load', match_threshold: 0.6 })
    const strategy = rows.find((r) => r.key === 'strategy')
    expect([strategy.value, strategy.present, strategy.dirty]).toEqual(['least_load', true, true])
    const timeout = rows.find((r) => r.key === 'timeout_seconds')
    expect([timeout.value, timeout.present, timeout.dirty]).toEqual([120, false, false])
    // false 是真实值，不是"没返回"
    const off = dispatchRows({ intelligent_matching: false }).find((r) => r.key === 'intelligent_matching')
    expect([off.value, off.present, off.dirty]).toEqual([false, true, true])
    expect(dispatchRows().every((r) => r.present === false)).toBe(true)
    expect(dispatchRows({ match_threshold: 'abc' }).find((r) => r.key === 'match_threshold').value).toBe(0.3)
  })
})

const fakeClient = (calls, response) => ({
  request: (cfg) => {
    calls.push(cfg)
    return Promise.resolve({ data: { code: 0, msg: 'ok', data: response } })
  }
})

const FULL = {
  strategy: 'best_match',
  intelligent_matching: true,
  match_threshold: 0.3,
  max_retries: 3,
  timeout_seconds: 120,
  weights: {},
  circuit_breaker_threshold: 5,
  concurrency_control: true
}

describe('调度配置取数', () => {
  it('GET 取整份配置，PUT 只发改动项，路径与方法逐字对齐端点表', async () => {
    const calls = []
    const api = createAllianceApi(fakeClient(calls, FULL))
    expect(await api.getDispatcherConfig()).toEqual(FULL)
    expect(await api.updateDispatcherConfig({ max_retries: 5 })).toEqual(FULL)
    expect(calls.map((c) => [c.url, c.method, c.data])).toEqual([
      ['/experts/dispatcher/config', 'GET', undefined],
      ['/experts/dispatcher/config', 'PUT', { max_retries: 5 }]
    ])
    expect(calls.every((c) => c.silent === true)).toBe(true)
  })

  it('后端返回缺字段时按信封原样交给 store，由行派生标 missing', async () => {
    const calls = []
    const api = createAllianceApi(fakeClient(calls, { strategy: 'least_load' }))
    expect(await api.getDispatcherConfig()).toEqual({ strategy: 'least_load' })
    expect(calls).toHaveLength(1)
    const rows = dispatchRows(await api.getDispatcherConfig())
    expect(rows.filter((r) => !r.present).map((r) => r.key)).toEqual(
      DISPATCH_CONFIG_FIELDS.filter((f) => f.key !== 'strategy').map((f) => f.key)
    )
  })
})

describe('调度配置状态', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('载入即以后端值 priming 草稿，改动后才允许保存', async () => {
    cfgApi.getDispatcherConfig.mockResolvedValue(FULL)
    cfgApi.updateDispatcherConfig.mockResolvedValue({ ...FULL, timeout_seconds: 300 })
    const store = useAllianceConsoleStore()
    await store.loadDispatcherConfig()
    expect(store.error.config).toBe('')
    expect(store.configChanged).toBe(false)
    expect(store.configRows.find((r) => r.key === 'timeout_seconds').value).toBe(120)

    store.setConfigValue('timeout_seconds', 300)
    expect(store.configChanged).toBe(true)
    expect(store.configPatch.patch).toEqual({ timeout_seconds: 300 })

    await store.saveDispatcherConfig()
    expect(cfgApi.updateDispatcherConfig).toHaveBeenCalledWith({ timeout_seconds: 300 })
    expect(store.dispatcherConfig.timeout_seconds).toBe(300)
    // 保存后以响应回读为基线：改动清零，草稿也回到新值，不留"看似还要保存"的假状态
    expect(store.configChanged).toBe(false)
    expect(store.configDraft.timeout_seconds).toBe(300)
  })

  it('无改动时保存是空操作，不发 PUT', async () => {
    cfgApi.getDispatcherConfig.mockResolvedValue(FULL)
    const store = useAllianceConsoleStore()
    await store.loadDispatcherConfig()
    expect(await store.saveDispatcherConfig()).toEqual(FULL)
    expect(cfgApi.updateDispatcherConfig).not.toHaveBeenCalled()
  })

  it('越界改动不发请求，直接给出后端会拒绝的原因', async () => {
    cfgApi.getDispatcherConfig.mockResolvedValue(FULL)
    const store = useAllianceConsoleStore()
    await store.loadDispatcherConfig()
    store.setConfigValue('max_retries', 42)
    expect(await store.saveDispatcherConfig()).toBeNull()
    expect(cfgApi.updateDispatcherConfig).not.toHaveBeenCalled()
    expect(store.error.config).toContain('0–10')
  })

  it('后端取不到配置时表单落在契约默认值上并显式标记，不当作已加载', async () => {
    cfgApi.getDispatcherConfig.mockRejectedValue({ msg: '调度器未启用' })
    const store = useAllianceConsoleStore()
    await store.loadDispatcherConfig()
    expect(store.dispatcherConfig).toBeNull()
    expect(store.error.config).toBe('调度器未启用')
    expect(store.configRows.filter((r) => !r.present)).toHaveLength(DISPATCH_CONFIG_FIELDS.length)
    expect(store.configChanged).toBe(false)
  })
})

describe('分发实跑请求体', () => {
  it('只发 handler 认识的三个键：task_type 恒在，空 expert_ids 不占位', () => {
    expect(dispatchRunBody({ taskType: '  ', input: ' 前端 架构 ' })).toEqual({ task_type: '', input: '前端 架构' })
    expect(dispatchRunBody({ taskType: 'code_review', input: 'x', expertIds: [] })).toEqual({ task_type: 'code_review', input: 'x' })
    expect(dispatchRunBody({ taskType: 't', input: 'x', expertIds: [' a ', '', null, 'b'] }))
      .toEqual({ task_type: 't', input: 'x', expert_ids: ['a', 'b'] })
    // constraints 在 DispatchBody 里存在但 dispatch 从不读，前端不得替后端臆造一个消费方
    expect('constraints' in dispatchRunBody({ taskType: 't', input: 'x', constraints: { top_k: 3 } })).toBe(false)
    expect(dispatchRunBody()).toEqual({ task_type: '', input: '' })
  })

  it('空需求描述不发：空串会让后端把领域匹配对全员判满分', () => {
    expect(dispatchRunProblem({ input: '   ' })).toContain('全员判满分')
    expect(dispatchRunProblem({})).toContain('需求描述不能为空')
    expect(dispatchRunProblem({ input: '前端' })).toBe('')
  })
})

describe('实跑结论判据', () => {
  const cfg = { strategy: 'least_load', weights: {} }
  const run = (strategyUsed, scores = [0.7]) => ({ strategyUsed, assigned: scores.map((matchScore) => ({ matchScore })) })

  it('策略一致才说"与配置一致"，不一致就指名两边', () => {
    expect(dispatchRunFindings(run('least_load'), cfg)[0]).toMatchObject({ tone: 'success' })
    expect(dispatchRunFindings(run('best_match'), cfg)[0]).toMatchObject({ tone: 'danger' })
    expect(dispatchRunFindings(run('best_match'), cfg)[0].text).toContain('least_load')
  })

  it('specified 分支不算策略生效，fallback 单独定性', () => {
    expect(dispatchRunFindings(run('specified'), cfg)[0].text).toContain('指定专家')
    expect(dispatchRunFindings(run('best_match(fallback)'), cfg)[0]).toMatchObject({ tone: 'danger' })
    expect(dispatchRunFindings(run('best_match(fallback)'), cfg)[0].text).toContain('不认识')
  })

  it('配置没取到时不下"一致"的结论', () => {
    expect(dispatchRunFindings(run('best_match'), null)[0].text).toContain('无法比对')
    expect(dispatchRunFindings(run('best_match'), {})[0].tone).toBe('info')
    expect(dispatchRunFindings(null, cfg)).toEqual([])
  })

  it('全员 0.5 是"智能匹配关闭"的指纹，不是巧合', () => {
    const off = dispatchRunFindings(run('best_match', [0.5, 0.5]), cfg)
    expect(off).toHaveLength(2)
    expect(off[1].text).toContain('intelligent_matching=false')
    // 只有一个候选时同样成立；混合分数则不给这条提示
    expect(dispatchRunFindings(run('best_match', [0.5]), cfg)[1].tone).toBe('warning')
    expect(dispatchRunFindings(run('best_match', [0.5, 0.6]), cfg)).toHaveLength(1)
    expect(dispatchRunFindings(run('best_match', []), cfg)).toHaveLength(1)
  })

  it('加权随机且权重表为空时，声明"两次实跑可能不同人"', () => {
    const note = dispatchRunFindings(run('weighted_random'), cfg)[1]
    expect(note.text).toContain('时间戳')
    expect(dispatchRunFindings(run('weighted_random'), { strategy: 'weighted_random', weights: { e1: 2 } })).toHaveLength(1)
  })
})

describe('分发实跑状态', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('越界表单一个请求都不发，原因写进 error.dispatch', async () => {
    const store = useAllianceConsoleStore()
    expect(await store.runDispatch({ taskType: 't', input: '  ' })).toBeNull()
    expect(cfgApi.runDispatch).not.toHaveBeenCalled()
    expect(store.error.dispatch).toContain('需求描述不能为空')
  })

  it('成功后结论与判据同时可得，503 只留错误不清结果', async () => {
    cfgApi.getDispatcherConfig.mockResolvedValue({ ...FULL, strategy: 'best_match' })
    const store = useAllianceConsoleStore()
    await store.loadDispatcherConfig()
    cfgApi.runDispatch.mockResolvedValue({ dispatchId: 'd1', strategyUsed: 'best_match', assigned: [{ id: 'e1', matchScore: 0.9 }] })
    const res = await store.runDispatch({ taskType: 't', input: '前端' })
    expect(res.dispatchId).toBe('d1')
    expect(store.dispatchFindings[0].tone).toBe('success')
    cfgApi.runDispatch.mockRejectedValue({ msg: 'no available experts for dispatch' })
    expect(await store.runDispatch({ taskType: 't', input: '架构' })).toBeNull()
    expect(store.error.dispatch).toContain('no available experts')
    expect(store.dispatchResult.dispatchId).toBe('d1')
  })

  it('候选专家取不到只让下拉空着，不阻塞实跑', async () => {
    const store = useAllianceConsoleStore()
    cfgApi.listExperts.mockRejectedValue({ msg: '注册表未启用' })
    await store.loadDispatchCandidates()
    expect(store.error.candidates).toBe('注册表未启用')
    expect(store.dispatchCandidates).toEqual([])
    cfgApi.listExperts.mockResolvedValue({ items: [{ id: 'e1', name: '玄枢', status: 'online' }], total: 1 })
    await store.loadDispatchCandidates()
    expect(store.dispatchCandidates).toEqual([{ id: 'e1', name: '玄枢', status: 'online' }])
    expect(store.error.candidates).toBe('')
  })
})

describe('任务标记完成状态', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('本地分支重取任务与节点：整批置完成的节点必须同步刷新', async () => {
    const store = useAllianceConsoleStore()
    store.tasks.splice(0, store.tasks.length, { id: 't1', title: 'A', status: 'running', progress: 0.4 })
    store.selectedId = 't1'
    cfgApi.toggleTaskDone.mockResolvedValue({ branch: 'local', direction: 'completed', currentStatus: 'completed', message: '任务 t1 已标记为完成' })
    cfgApi.getTask.mockResolvedValue({ id: 't1', title: 'A', status: 'completed', progress: 1 })
    cfgApi.getNodes.mockResolvedValue({ items: [{ id: 'n1', status: 'completed' }, { id: 'n2', status: 'failed' }], total: 2 })
    const res = await store.toggleTaskDone()
    expect(res.direction).toBe('completed')
    expect(store.detail.task).toMatchObject({ status: 'completed', progress: 1 })
    expect(store.tasks[0].status).toBe('completed')
    expect(store.detail.nodes.map((n) => n.status)).toEqual(['completed', 'failed'])
    expect(cfgApi.getNodes).toHaveBeenCalledWith('t1')
  })

  it('失败只记 error.toggle，不动已有详情', async () => {
    const store = useAllianceConsoleStore()
    store.selectedId = 't1'
    store.detail.task = { id: 't1', status: 'running' }
    cfgApi.toggleTaskDone.mockRejectedValue({ msg: '任务 t1 不存在' })
    expect(await store.toggleTaskDone()).toBeNull()
    expect(store.error.toggle).toBe('任务 t1 不存在')
    expect(store.detail.task.status).toBe('running')
    expect(cfgApi.getTask).not.toHaveBeenCalled()
  })

  it('等待期间切了任务就不把上一个的状态写进详情', async () => {
    const store = useAllianceConsoleStore()
    store.selectedId = 't1'
    let release
    cfgApi.toggleTaskDone.mockReturnValue(new Promise((resolve) => {
      release = () => resolve({ branch: 'local', direction: 'completed', currentStatus: 'completed' })
    }))
    const pending = store.toggleTaskDone('t1')
    store.selectedId = 't2'
    release()
    expect(await pending).toMatchObject({ direction: 'completed' })
    expect(cfgApi.getTask).not.toHaveBeenCalled()
    expect(store.detail.task).toBeNull()
  })
})

// ── 调度状态面与两个负载重置动作（台账里的破坏性 backlog 已搬出）──────────
describe('调度状态契约', () => {
  it('status 的出参键集就是后端 json! 里那十个键，且每个键都被归一化读过', () => {
    expect(STATUS_OUT_KEYS.length, '解析空转：没从 dispatcher_status 里取到出参键').toBeGreaterThan(6)
    expect([...STATUS_OUT_KEYS].sort()).toEqual([...DISPATCH_STATUS_KEYS].sort())
    const norm = NORM_SRC.match(/export function normDispatcherStatus\([\s\S]*?\n\}/)[0]
    for (const key of DISPATCH_STATUS_KEYS) {
      expect(norm, `归一化没读后端返回的 ${key}（漏一个键就等于界面少一格真实读数）`).toContain(`p.${key}`)
    }
  })

  it('界面不读后端不返回的键——存量面板那三个假字段一个都不许跟过来', () => {
    // ExpertEnterprisePanel.vue 读 circuit_breaker?.states / dispatcher.recent_dispatches / cb.status，
    // 三者都不在 dispatcher_status 的出参里，于是它的熔断卡恒显示"服务正常"、KPI 恒 0。
    // 逐名点字面会被 `?.` 绕过（变异电池 D10 实测过一次），所以这里按结构核：
    // 界面对 status 伸的每一次手，都必须落在归一化产出的键集里。
    expect(VIEW_STATUS_READS.length, '解析空转：没从界面里取到 status 读数键').toBeGreaterThan(4)
    for (const key of VIEW_STATUS_READS) {
      expect(NORM_STATUS_KEYS, `界面读了 status.${key}，但归一化不产这个键`).toContain(key)
    }
    expect(CONSOLE_VUE).not.toMatch(/circuit_breaker\??\.\s*states/)
    expect(CONSOLE_VUE).not.toMatch(/recent_dispatches/)
    expect(CONSOLE_VUE).not.toMatch(/\bcb\??\.status\b/)
    expect(CONSOLE_VUE).not.toMatch(/所有专家(服务)?(正常|健康)/)
  })

  it('engine_status 是后端字面量，界面必须自陈它不是探活', () => {
    expect(STATUS_FN).toMatch(/"engine_status":\s*"running"/)
    expect(DISPATCH_ENGINE_STATUS_LITERAL).toBe('running')
    expect(CONSOLE_VUE).toMatch(/不是探活结果/)
  })

  it('熔断计数只有读侧：空列表要说成"没有数据"，不能说成健康', () => {
    expect(DISPATCHER_PROD, '后端出现了写侧，本契约的"恒为空"措辞要跟着改').not.toMatch(/(fc_guard|FAILURE_COUNTS)[\s\S]{0,160}\.insert\(/)
    const note = breakerEmptyNote([])
    expect(note).toMatch(/(恒|永远)为空/)
    expect(note).toMatch(/不代表/)
    expect(breakerEmptyNote([{ expertId: 'e1', failureCount: 2, state: 'open' }])).toBe('')
  })

  it('无终态样本时后端给的 1.0 要说成默认值，不是 100% 成功', () => {
    expect(successRateNote({ totalDispatches: 0 })).toMatch(/不是"100% 成功"/)
    expect(successRateNote({ totalDispatches: 4 })).toMatch(/不含进行中/)
  })
})

describe('负载重置契约', () => {
  it('两个 handler 的请求体要求相反，由源码签名钉住', () => {
    expect(RESET_ONE).toMatch(/Json\(body\): Json<ResetBody>/)
    expect(RESET_ALL, '全量重置一旦开始读 body，前端的"不发体"就成了漏发').not.toMatch(/Json</)
    expect(fieldsOf(RESET_BODY_STRUCT)).toEqual(['reason'])
    expect(RESET_BODY_STRUCT).toMatch(/pub reason:\s*Option<String>/)
    expect(dispatchResetBody()).toEqual({})
    expect(dispatchResetBody('   ')).toEqual({})
    expect(dispatchResetBody('演练后清场')).toEqual({ reason: '演练后清场' })
  })

  it('单专家重置取不到 id 也不 404：回执不能长成存在性断言', () => {
    expect(RESET_ONE).toMatch(/\.unwrap_or\(0\)/)
    expect(RESET_ONE).toMatch(/"reset":\s*true/)
    const text = dispatchResetNotice({ expertId: 'exp-1', previousLoad: 3 })
    expect(text).toContain('exp-1')
    expect(text).toMatch(/重置前负载为 3/)
    expect(text).toMatch(/不代表该专家存在/)
    expect(text).toMatch(/不代表已落库/)
    expect(dispatchResetNotice({})).toMatch(/重置前负载为 0/)
  })

  it('两个 reset 都不落库，而咨询会落库——所以"归零"是可被回写的内存态', () => {
    expect(RESET_ONE).not.toMatch(/save_registry/)
    expect(RESET_ALL).not.toMatch(/save_registry/)
    const consult = REGISTRY_RS.match(/exp\.availability\.current_load \+= 1;[\s\S]{0,140}/)[0]
    // A1 多租户后 save_registry 先取租户：签名由 save_registry(&reg) 变为 save_registry(tenant.as_str(), reg)
    expect(consult).toMatch(/save_registry\(tenant\.as_str\(\), reg\)/)
  })

  it('全量重置覆盖整张注册表（含停用者），人数只能引后端计数', () => {
    expect(RESET_ALL).toMatch(/registry\.values_mut\(\)/)
    expect(RESET_ALL).not.toMatch(/values_mut\(\)[\s\S]{0,60}\.filter/)
    expect(RESET_ALL).toMatch(/"reset_count":\s*reset_count/)
    const text = dispatchResetAllNotice({ resetCount: 7 })
    expect(text).toContain('7 位')
    expect(text).toMatch(/不落库/)
  })

  it('二次确认清单五条齐全，每条都带源码坐标', () => {
    const one = dispatchResetLines({ id: 'exp-1', name: '架构师·玄枢' })
    const all = dispatchResetLines({ all: true })
    expect(one).toHaveLength(5)
    expect(all).toHaveLength(5)
    for (const line of [...one, ...all]) {
      expect(line, `这条后果没有可核对的位置：${line}`).toMatch(/:\d{2,4}/)
    }
    const oneText = one.join('\n')
    expect(oneText).toMatch(/if let Some/)
    expect(oneText).toMatch(/没有写入失败计数/)
    expect(oneText).toMatch(/reset:true/)
    expect(oneText).toMatch(/max_concurrent/)
    const allText = all.join('\n')
    expect(allText).toMatch(/values_mut/)
    expect(allText).toMatch(/reset_count/)
    expect(oneText).not.toBe(allText)
  })

  it('全量重置要手动输入确认词才放行：后端只认证不授权，没有第二道闸', () => {
    expect(CONSOLE_VUE).toMatch(/resetWord\.value === DISPATCH_RESET_ALL_CONFIRM/)
    expect(CONSOLE_VUE).toMatch(/:disabled="!resetReady"/)
    expect(DISPATCH_RESET_ALL_CONFIRM).toMatch(/^[A-Z-]+$/)
  })
})

describe('负载重置的取数与状态', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  // store 看到的是 api 层归一化后的读数，所以这里给 camelCase；线上形状由 normalize/api 用例钉
  const STATUS = {
    engineStatus: 'running', currentStrategy: 'best_match', activeDispatches: 0, totalDispatches: 2,
    successRate: 1, avgDispatchMs: 0, circuitBreakers: [], lastDispatchAt: '', serverTs: '2026-09-24T00:00:00Z',
    expertLoads: [{ expertId: 'exp-1', currentLoad: 5, maxConcurrent: 3, loadRatio: 1.6667 }]
  }
  // 线上（后端）形状：给 fakeClient 用，走的是 api 层的归一化入口
  const REONE = { expert_id: 'exp-1', reset: true, previous_load: 3, previous_failures: 0, reset_at: '2026-09-24T00:00:00Z', reason: null }
  const REALL = { reset_count: 2, reset_expert_ids: ['exp-1', 'exp-2'], reset_at: '2026-09-24T00:00:00Z' }

  it('请求形状：单专家必发对象、全量不发体，路径逐字对齐端点表', async () => {
    const calls = []
    const api = createAllianceApi(fakeClient(calls, REONE))
    await api.resetDispatcherLoad('exp-1')
    await api.resetDispatcherLoad('exp-1', '演练后清场')
    await api.resetAllDispatcherLoads()
    expect(calls.map((c) => [c.url, c.method, JSON.stringify(c.data)])).toEqual([
      ['/experts/dispatcher/reset/exp-1', 'POST', '{}'],
      ['/experts/dispatcher/reset/exp-1', 'POST', '{"reason":"演练后清场"}'],
      ['/experts/dispatcher/reset-all', 'POST', undefined]
    ])
    expect(calls.every((c) => c.silent === true)).toBe(true)
  })

  it('两种回执归一化后互不混形：单专家有 previousLoad，全量有 resetExpertIds', async () => {
    const one = createAllianceApi(fakeClient([], REONE))
    const all = createAllianceApi(fakeClient([], REALL))
    expect(await one.resetDispatcherLoad('exp-1')).toEqual({
      expertId: 'exp-1', reset: true, previousLoad: 3, previousFailures: 0,
      resetAt: '2026-09-24T00:00:00Z', reason: ''
    })
    expect(await all.resetAllDispatcherLoads()).toEqual({
      resetCount: 2, resetExpertIds: ['exp-1', 'exp-2'], resetAt: '2026-09-24T00:00:00Z'
    })
  })

  it('重置后必须重取状态，且不在本地把负载抹成 0', async () => {
    setActivePinia(createPinia())
    cfgApi.dispatcherStatus.mockResolvedValue(STATUS)
    cfgApi.resetDispatcherLoad.mockResolvedValue({ expertId: 'exp-1', previousLoad: 5 })
    const store = useAllianceConsoleStore()
    await store.loadDispatcherStatus()
    expect(store.loadRows[0].currentLoad).toBe(5)
    await store.resetExpertLoad('exp-1', '演练')
    expect(cfgApi.resetDispatcherLoad).toHaveBeenCalledWith('exp-1', '演练')
    expect(cfgApi.dispatcherStatus).toHaveBeenCalledTimes(2)
    // 后端没改读数之前，界面不许先替它归零
    expect(store.loadRows[0].currentLoad).toBe(5)
    expect(store.resetReceipt.scope).toBe('one')
  })

  it('全量重置同样重取，回执带 scope 供卡片分清是哪一次动作', async () => {
    setActivePinia(createPinia())
    cfgApi.dispatcherStatus.mockResolvedValue(STATUS)
    cfgApi.resetAllDispatcherLoads.mockResolvedValue({ resetCount: 2, resetExpertIds: ['exp-1', 'exp-2'] })
    const store = useAllianceConsoleStore()
    await store.loadDispatcherStatus()
    await store.resetAllLoads()
    expect(cfgApi.dispatcherStatus).toHaveBeenCalledTimes(2)
    expect(store.resetReceipt.scope).toBe('all')
    expect(store.resetReceipt.resetCount).toBe(2)
  })

  it('重置失败：错误留在 error.reset，上一次读数与回执都不被清空', async () => {
    setActivePinia(createPinia())
    cfgApi.dispatcherStatus.mockResolvedValue(STATUS)
    cfgApi.resetAllDispatcherLoads.mockResolvedValue({ resetCount: 2 })
    cfgApi.resetDispatcherLoad.mockRejectedValue({ msg: '网关 500' })
    const store = useAllianceConsoleStore()
    await store.loadDispatcherStatus()
    await store.resetAllLoads()
    const receipt = store.resetReceipt
    await store.resetExpertLoad('exp-1')
    expect(store.error.reset).toBe('网关 500')
    expect(store.resetReceipt).toBe(receipt)
    expect(store.loadRows.map((r) => r.currentLoad)).toEqual([5])
    expect(cfgApi.dispatcherStatus).toHaveBeenCalledTimes(2)
  })

  it('状态取不到时读数被清空，且两个重置入口都不许可用', async () => {
    setActivePinia(createPinia())
    cfgApi.dispatcherStatus.mockRejectedValue({ msg: '网关未就绪' })
    const store = useAllianceConsoleStore()
    await store.loadDispatcherStatus()
    expect(store.dispatcherStatus).toBeNull()
    expect(store.error.status).toBe('网关未就绪')
    expect(CONSOLE_VUE).toMatch(/:disabled="!store\.dispatcherStatus \|\| store\.loading\.reset"/)
    expect(CONSOLE_VUE).toMatch(/没有拿到状态读数/)
  })
})

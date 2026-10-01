// 协作 store 单元测试：状态流转与「后端拒绝条件前移」的落点。
// wire 名与 clamp 由 contract.test.js 守住，这里只验 store 是否把它们正确交给 api 层。
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

const { api } = vi.hoisted(() => ({ api: { collaborate: vi.fn() } }))
vi.mock('../api/alliance.api.js', () => ({ allianceApi: api }))

const { useAllianceCollabStore, HISTORY_CAP } = await import('./alliance-collab.store.js')
const { COLLAB_MODE } = await import('../contract/collab.js')
const { COLLAB_MODES } = await import('../contract/collab.js')

const fail = (msg) => Object.assign(new Error(msg), { name: 'ApiError', msg })
const okResult = (over = {}) => ({ mode: COLLAB_MODE.MULTI, sessionId: 'sess-1', question: 'q', contributions: [], fusion: {}, ...over })

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
})

describe('模式切换与控件同步', () => {
  it('默认落在智能咨询：它不需要选专家，是六个模式里门槛最低的入口', () => {
    const store = useAllianceCollabStore()
    expect(store.mode).toBe(COLLAB_MODE.SMART)
    expect(store.controlList).toHaveLength(0)
  })

  it('切模式会按后端 unwrap_or 默认值重建控件，并清掉上一次结果', async () => {
    const store = useAllianceCollabStore()
    api.collaborate.mockResolvedValue(okResult())
    store.input.question = 'q'
    await store.run()
    expect(store.result).toBeTruthy()

    store.setMode(COLLAB_MODE.DEBATE)
    expect(store.result).toBe(null)
    expect(store.controls.rounds).toBe(3)
    // 辩题与咨询问题是两个后端字段，输入框按 textField 取放，不能串台
    expect(store.textField).toBe('topic')
    expect(store.current.fieldLabel).toBe('辩题')
  })

  it('未知模式不会改变状态', () => {
    const store = useAllianceCollabStore()
    expect(store.setMode('nope')).toBe(false)
    expect(store.mode).toBe(COLLAB_MODE.SMART)
  })

  it('每个模式都能被选中，且控件默认值等于后端 unwrap_or 值', () => {
    const store = useAllianceCollabStore()
    for (const m of COLLAB_MODES) {
      expect(store.setMode(m.key), `无法切到 ${m.key}`).toBe(true)
      for (const c of m.controls) expect(store.controls[c.wire]).toBe(c.default)
    }
  })
})

describe('运行前校验：后端的 400/404/422 条件不再靠试出来', () => {
  it('主文本为空即不可运行，且不会发出请求', async () => {
    const store = useAllianceCollabStore()
    expect(store.runnable).toBe(false)
    expect(store.validation).toContain('请填写')
    expect(await store.run()).toBe(null)
    expect(api.collaborate).not.toHaveBeenCalled()
  })

  it('单专家咨询必须恰好选 1 位（专家走路径参数）', () => {
    const store = useAllianceCollabStore()
    store.setMode(COLLAB_MODE.SINGLE)
    store.input.question = 'q'
    expect(store.validation).toBe('请先选择 1 位专家')
    store.toggleExpert('e1')
    store.toggleExpert('e2')
    // 单选语义：后选的替换先选的
    expect(store.input.expertIds).toEqual(['e2'])
    expect(store.validation).toBe('')
  })

  it('路由/智能咨询/算法分析由后端自行匹配专家，前端选了也无效', () => {
    const store = useAllianceCollabStore()
    for (const key of [COLLAB_MODE.ROUTE, COLLAB_MODE.SMART, COLLAB_MODE.ALGORITHM]) {
      store.setMode(key)
      store.input[key === COLLAB_MODE.ALGORITHM ? 'algorithm_description' : 'question'] = 'x'
      expect(store.toggleExpert('e1')).toBe(false)
      expect(store.input.expertIds).toEqual([])
      expect(store.validation).toBe('')
    }
  })

  it('辩论选 1 人给不出对手，选 6 人只有前 4 位上场', () => {
    const store = useAllianceCollabStore()
    store.setMode(COLLAB_MODE.DEBATE)
    store.input.topic = 't'
    store.toggleExpert('e1')
    expect(store.validation).toContain('至少需要 2 名专家')
    store.clearExperts()
    // 不选则由后端 match_top_experts 自动配对，所以空集合是可运行的
    expect(store.validation).toBe('')
    for (const id of ['a', 'b', 'c', 'd', 'e', 'f']) store.toggleExpert(id)
    expect(store.pickedCount).toBe(6)
    expect(store.capacityNote).toContain('仅取前 4 位')
  })

  it('输入与控件合并后交给契约层组装 body，store 不自己拼 wire', async () => {
    const store = useAllianceCollabStore()
    store.setMode(COLLAB_MODE.MULTI)
    store.input.question = 'q'
    store.input.domain = 'architecture'
    store.controls.max_experts = 99
    await store.run()
    // 越界夹取与 camel→wire 的转换都在 collabBody，store 只负责原样递过去
    expect(api.collaborate).toHaveBeenCalledWith(COLLAB_MODE.MULTI, {
      question: 'q', topic: '', algorithm_description: '', expertIds: [], domain: 'architecture', context: '', max_experts: 99
    })
    expect(store.controlList[0]).toMatchObject({ wire: 'max_experts', min: 1, max: 10 })
  })

  it('路由约束随模式建立与销毁，切走再切回即回到后端默认', async () => {
    const store = useAllianceCollabStore()
    store.setMode(COLLAB_MODE.ROUTE)
    expect(store.constraintList.map((f) => [f.key, f.value])).toEqual([
      ['min_rating', 0], ['max_response_time', null], ['require_online', false]
    ])
    store.controls.min_rating = 4.5
    store.controls.require_online = true
    expect(store.constraintList.find((f) => f.key === 'min_rating').value).toBe(4.5)
    // 夹取发生在契约层取值时：store 里存的仍是用户输入的原值
    store.controls.max_response_time = 9999
    expect(store.constraintList.find((f) => f.key === 'max_response_time').value).toBe(240)
    store.input.question = 'q'
    await store.run()
    expect(api.collaborate).toHaveBeenCalledWith(
      COLLAB_MODE.ROUTE,
      expect.objectContaining({ question: 'q', min_rating: 4.5, max_response_time: 9999, require_online: true })
    )
    // 换到没有约束的模式即清空，切回来也不该带残留值
    store.setMode(COLLAB_MODE.MULTI)
    expect(store.constraintList).toEqual([])
    expect(store.controls.min_rating).toBeUndefined()
    store.setMode(COLLAB_MODE.ROUTE)
    expect(store.controls).toMatchObject({ max_experts: 5, min_rating: 0, max_response_time: null, require_online: false })
  })
})

describe('运行结果与历史记录', () => {
  it('成功时写入结果并记一条带后端标识的运行记录', async () => {
    const store = useAllianceCollabStore()
    store.setMode(COLLAB_MODE.MULTI)
    store.input.question = '如何拆分网关'
    api.collaborate.mockResolvedValue(okResult({ sessionId: 'sess-42', question: '如何拆分网关' }))
    const out = await store.run()
    expect(out.sessionId).toBe('sess-42')
    expect(store.resultKind).toBe('fusion')
    // 记录里的标识与文本都取自后端回显，不是本地拼的
    expect(store.history[0]).toMatchObject({ mode: 'multi', modeLabel: '多专家协同', refId: 'sess-42', text: '如何拆分网关', ok: true })
    expect(store.loading.run).toBe(false)
    expect(store.error.run).toBe('')
  })

  it('失败时把 ApiError.msg 单点交给视图显示，并记一条失败记录', async () => {
    const store = useAllianceCollabStore()
    store.setMode(COLLAB_MODE.MULTI)
    store.input.question = '无人匹配的问题'
    api.collaborate.mockRejectedValue(fail('未找到匹配的可用专家'))
    expect(await store.run()).toBe(null)
    expect(store.error.run).toBe('未找到匹配的可用专家')
    expect(store.result).toBe(null)
    expect(store.history[0]).toMatchObject({ refId: '', text: '无人匹配的问题', ok: false })
  })

  it('结果自带的 mode 戳决定渲染口径，戳不认识就不渲染而不是错渲染', () => {
    const store = useAllianceCollabStore()
    store.result = { mode: 'multi' }
    expect(store.resultKind).toBe('fusion')
    store.result = { mode: '后端新增的模式' }
    expect(store.resultKind).toBe('')
  })

  it('融合结果被治理闸门否决时按拦截口径提示，而不是当成功展示', () => {
    const store = useAllianceCollabStore()
    store.result = { mode: 'multi', fusion: { blocked: true } }
    expect(store.blocked).toBe(true)
    store.result = { mode: 'multi', fusion: { blocked: false } }
    expect(store.blocked).toBe(false)
  })

  it('运行记录只保留最近 HISTORY_CAP 条，且 reset 不清记录', async () => {
    const store = useAllianceCollabStore()
    store.setMode(COLLAB_MODE.MULTI)
    store.input.question = 'q'
    api.collaborate.mockResolvedValue(okResult())
    for (let i = 0; i < HISTORY_CAP + 3; i++) await store.run()
    expect(store.history).toHaveLength(HISTORY_CAP)
    store.reset()
    expect(store.input.question).toBe('')
    expect(store.result).toBe(null)
    expect(store.history).toHaveLength(HISTORY_CAP)
  })

  it('reset 会清空六个模式的输入字段，不留上一次的残值', async () => {
    const store = useAllianceCollabStore()
    store.input.question = 'q'
    store.input.topic = 't'
    store.input.algorithm_description = 'a'
    store.input.domain = 'arch'
    store.input.context = 'ctx'
    store.input.expertIds = ['e1']
    store.reset()
    expect(store.input).toMatchObject({ question: '', topic: '', algorithm_description: '', domain: '', context: '', expertIds: [] })
    // 控件回到后端默认值
    expect(store.setMode(COLLAB_MODE.ROUTE) && store.controls.max_experts).toBe(5)
  })
})

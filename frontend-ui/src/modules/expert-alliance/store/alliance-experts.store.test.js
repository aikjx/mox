// 广场 store 单元测试：只验状态流转与错误单点，信封/字段归一由 contract 层负责。
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

const { api } = vi.hoisted(() => ({
  api: {
    listExperts: vi.fn(),
    listMyBookings: vi.fn(),
    createBooking: vi.fn(),
    cancelBooking: vi.fn(),
    toggleFavorite: vi.fn(),
    consultNow: vi.fn(),
    consultRoom: vi.fn(),
    expertsStats: vi.fn(),
    listExpertCapabilities: vi.fn(),
    getExpertMetrics: vi.fn(),
    searchExperts: vi.fn(),
    joinTeam: vi.fn(),
    registerExpert: vi.fn(),
    updateExpert: vi.fn(),
    deleteExpert: vi.fn()
  }
}))
vi.mock('../api/alliance.api.js', () => ({ allianceApi: api }))

const { useAllianceExpertsStore, EXPERT_FILTER_KEYS } = await import('./alliance-experts.store.js')
const { EXPERT_QUERY_KEYS } = await import('../contract/endpoints.js')
const { emptyExpertDraft, expertFormDraft } = await import('../contract/registry.js')

const expert = (id, status = 'online') => ({
  id,
  name: `专家${id}`,
  online: status === 'online',
  availability: { status, currentLoad: 0, maxConcurrent: 0, loadRatio: null },
  metrics: { totalConsultations: 0, avgRating: 0, ratingCount: 0, resolutionRate: 0 }
})
const booking = (id, status = 'pending') => ({ id, expertId: 'e1', expertName: 'N', topic: 't', status, cancellable: status === 'pending' })
const fail = (msg) => Object.assign(new Error(msg), { name: 'ApiError', msg })

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
  api.listMyBookings.mockResolvedValue({ items: [], total: 0, counts: { pending: 0, confirmed: 0, completed: 0, cancelled: 0 } })
})

describe('列表与筛选', () => {
  it('可筛选字段必须是后端认识的子集', () => {
    for (const key of EXPERT_FILTER_KEYS) expect(EXPERT_QUERY_KEYS).toHaveProperty(key)
    // 分页不归筛选管，由 store 的 page/pageSize 单独维护
    expect(EXPERT_FILTER_KEYS).not.toContain('page')
    expect(EXPERT_FILTER_KEYS).not.toContain('pageSize')
  })

  it('成功时写入 items/total 并回到服务端页码', async () => {
    api.listExperts.mockResolvedValue({ items: [expert('e1')], total: 41, page: 3, pageSize: 24 })
    const store = useAllianceExpertsStore()
    store.page = 3
    await store.loadExperts()
    expect(store.experts).toHaveLength(1)
    expect(store.total).toBe(41)
    expect(store.pageCount).toBe(2)
    expect(store.error.list).toBe('')
    // page/pageSize 以本地字段名下发，wire 名由 api 层转换
    expect(api.listExperts).toHaveBeenCalledWith(expect.objectContaining({ page: 3, pageSize: 24 }))
  })

  it('失败时清空列表并把 ApiError.msg 交给页面单点显示', async () => {
    api.listExperts.mockRejectedValue(fail('网关不可达'))
    const store = useAllianceExpertsStore()
    store.experts = [expert('e1')]
    store.total = 1
    await store.loadExperts()
    expect(store.error.list).toBe('网关不可达')
    expect(store.experts).toEqual([])
    expect(store.total).toBe(0)
  })

  it('setFilters 只接受后端认识的字段', async () => {
    api.listExperts.mockResolvedValue({ items: [], total: 0, page: 1, pageSize: 24 })
    const store = useAllianceExpertsStore()
    store.setFilters({ search: 'rust', keyword: 'x', page: 9 })
    expect(store.filters.search).toBe('rust')
    expect(store.filters.keyword).toBeUndefined()
    expect(store.filters.page).toBeUndefined()
    expect(store.page).toBe(1)
  })

  it('goPage 夹在 1..pageCount', async () => {
    api.listExperts.mockResolvedValue({ items: [], total: 100, page: 1, pageSize: 24 })
    const store = useAllianceExpertsStore()
    await store.loadExperts()
    store.goPage(99)
    expect(store.page).toBe(store.pageCount)
    store.goPage(-5)
    expect(store.page).toBe(1)
  })

  it('在线数按 availability 统计，不数标签', async () => {
    api.listExperts.mockResolvedValue({
      items: [expert('a', 'online'), expert('b', 'busy'), expert('c', 'online')],
      total: 3, page: 1, pageSize: 24
    })
    const store = useAllianceExpertsStore()
    await store.loadExperts()
    expect(store.onlineCount).toBe(2)
  })
})

describe('收藏', () => {
  it('本地集合以服务端返回为准，失败不改动', async () => {
    api.toggleFavorite.mockResolvedValue({ expertId: 'e1', favorite: true, action: 'favorited', updatedAt: 't' })
    const store = useAllianceExpertsStore()
    await store.toggleFavorite(expert('e1'))
    expect(store.isFavorite('e1')).toBe(true)
    expect(store.notice).toBe('已收藏 专家e1')

    api.toggleFavorite.mockRejectedValue(fail('401 未登录'))
    await store.toggleFavorite(expert('e2'))
    expect(store.isFavorite('e2')).toBe(false)
    expect(store.error.action).toBe('401 未登录')
  })

  it('后端没有收藏读接口，store 明确标为会话级', () => {
    const store = useAllianceExpertsStore()
    expect(store.favoriteSessionOnly).toBe(true)
    expect(store.favoriteCount).toBe(0)
  })
})

describe('预约与咨询', () => {
  it('下单成功后重拉预约列表', async () => {
    api.createBooking.mockResolvedValue({ id: 'b1', expertId: 'e1', expertName: 'N', topic: '架构' })
    api.listMyBookings.mockResolvedValue({ items: [booking('b1')], total: 1, counts: { pending: 1, confirmed: 0, completed: 0, cancelled: 0 } })
    const store = useAllianceExpertsStore()
    await store.createBooking({ expertId: 'e1', topic: '架构' })
    expect(api.createBooking).toHaveBeenCalledWith({ expertId: 'e1', topic: '架构', scheduledAt: undefined, durationMinutes: undefined })
    expect(store.bookings).toHaveLength(1)
    expect(store.bookingCounts.pending).toBe(1)
    expect(store.notice).toContain('已预约')
  })

  it('后端 404（专家不存在）时不重拉列表，错误留在 action', async () => {
    api.createBooking.mockRejectedValue(fail('expert not found: e404'))
    const store = useAllianceExpertsStore()
    const res = await store.createBooking({ expertId: 'e404', topic: 'x' })
    expect(res).toBeNull()
    expect(store.error.action).toBe('expert not found: e404')
    expect(api.listMyBookings).not.toHaveBeenCalled()
  })

  it('取消预约走 intent 通道并刷新计数', async () => {
    api.cancelBooking.mockResolvedValue({ bookingId: 'b1', status: 'cancelled', cancelledAt: 't', message: '预约已取消' })
    api.listMyBookings.mockResolvedValue({ items: [booking('b1', 'cancelled')], total: 1, counts: { pending: 0, confirmed: 0, completed: 0, cancelled: 1 } })
    const store = useAllianceExpertsStore()
    await store.cancelBooking('b1')
    expect(store.notice).toBe('预约已取消')
    expect(store.bookingCounts.cancelled).toBe(1)
  })

  it('专家不在线时以后端提示入队，不当作成功', async () => {
    api.consultNow.mockResolvedValue({ expertId: 'e1', sessionId: null, status: 'unavailable', expertOnline: false, chatUrl: null, message: '专家当前不在线，请稍后重试或预约' })
    const store = useAllianceExpertsStore()
    const res = await store.consultNow(expert('e1', 'busy'))
    expect(res.sessionId).toBeNull()
    expect(store.notice).toBe('专家当前不在线，请稍后重试或预约')
  })

  it('咨询室凭据原样透出，团队申请按后端状态返回', async () => {
    api.consultRoom.mockResolvedValue({ bookingId: 'b1', roomId: 'r1', joinUrl: '/consult/room/r1' })
    api.joinTeam.mockResolvedValue({ applicationId: 'a1', status: 'pending_approval' })
    const store = useAllianceExpertsStore()
    expect(await store.openRoom('b1')).toMatchObject({ roomId: 'r1' })
    expect(await store.joinTeam({ teamId: 't1', expertId: 'e1' })).toMatchObject({ status: 'pending_approval' })
    expect(api.joinTeam).toHaveBeenCalledWith({ teamId: 't1', expertId: 'e1', role: 'member' })
  })
})

describe('平台统计', () => {
  it('统计与列表互不牵连：stats 只由 /api/experts/stats 决定', async () => {
    api.listExperts.mockResolvedValue({ items: [expert('e1')], total: 1, page: 1 })
    api.expertsStats.mockResolvedValue({ totalExperts: 149, onlineExperts: 120, totalConsultations: 3916, avgRating: 4.6 })
    const store = useAllianceExpertsStore()
    await Promise.all([store.loadExperts(), store.loadStats()])
    expect(store.stats.totalExperts).toBe(149)
    // 平台在线数与本页在线数是两套口径，store 不把其中之一冒充另一个
    expect(store.onlineCount).toBe(1)
  })

  it('统计失败只影响统计面板，不清洗列表也不抛给全局', async () => {
    api.listExperts.mockResolvedValue({ items: [expert('e1')], total: 1, page: 1 })
    api.expertsStats.mockRejectedValue(fail('stats 502'))
    const store = useAllianceExpertsStore()
    await store.loadExperts()
    await store.loadStats()
    expect(store.error.stats).toBe('stats 502')
    expect(store.stats).toBeNull()
    expect(store.experts).toHaveLength(1)
    expect(store.error.list).toBe('')
  })
})

describe('能力目录', () => {
  const caps = () => ({
    items: [{ id: 'c1', name: '架构评审', domain: 'architecture', expertCount: 3, avgProficiency: 88.5 }],
    total: 1,
    domains: ['architecture']
  })

  it('目录与列表互不牵连：capabilities 只由 /api/experts/capabilities 决定', async () => {
    api.listExperts.mockResolvedValue({ items: [expert('e1')], total: 1, page: 1 })
    api.listExpertCapabilities.mockResolvedValue(caps())
    const store = useAllianceExpertsStore()
    await Promise.all([store.loadExperts(), store.loadCapabilities()])
    expect(store.capabilities.total).toBe(1)
    expect(store.capabilities.items[0]).toMatchObject({ id: 'c1', expertCount: 3 })
    expect(store.error.capabilities).toBe('')
  })

  it('未取过时为 null（不是空目录），失败时回到 null 并单点报错', async () => {
    const store = useAllianceExpertsStore()
    expect(store.capabilities).toBeNull()
    api.listExpertCapabilities.mockRejectedValue(fail('capabilities 500'))
    await store.loadCapabilities()
    expect(store.capabilities).toBeNull()
    expect(store.error.capabilities).toBe('capabilities 500')
    expect(store.loading.capabilities).toBe(false)
  })
})

describe('单专家派生指标', () => {
  const metricsOf = (id) => ({ expertId: id, metrics: {}, availability: {}, derived: { rankPercentile: 50, loadRatio: 0, efficiencyScore: 0.5 } })
  const deferred = () => {
    let resolve
    const p = new Promise((r) => { resolve = r })
    return { p, resolve }
  }

  it('按 id 取，成功即持有该专家的派生指标', async () => {
    api.getExpertMetrics.mockResolvedValue(metricsOf('e1'))
    const store = useAllianceExpertsStore()
    await expect(store.loadExpertMetrics('e1')).resolves.toMatchObject({ expertId: 'e1' })
    expect(api.getExpertMetrics).toHaveBeenCalledWith('e1')
    expect(store.expertMetrics.derived.rankPercentile).toBe(50)
  })

  it('后端 404 不折叠成零值：留错误文案、指标仍为 null、不占用写操作错误位', async () => {
    api.getExpertMetrics.mockRejectedValue(fail('expert not found: e9'))
    const store = useAllianceExpertsStore()
    await expect(store.loadExpertMetrics('e9')).resolves.toBeNull()
    expect(store.error.metrics).toBe('expert not found: e9')
    expect(store.expertMetrics).toBeNull()
    expect(store.error.action).toBe('')
  })

  it('空 id 不发请求', async () => {
    const store = useAllianceExpertsStore()
    await expect(store.loadExpertMetrics('')).resolves.toBeNull()
    expect(api.getExpertMetrics).not.toHaveBeenCalled()
    expect(store.loading.metrics).toBe(false)
  })

  it('连点两位专家时，先请求的慢响应不得覆盖后请求的结果', async () => {
    const first = deferred()
    const second = deferred()
    api.getExpertMetrics
      .mockImplementationOnce(() => first.p)
      .mockImplementationOnce(() => second.p)
    const store = useAllianceExpertsStore()
    const a = store.loadExpertMetrics('e1')
    const b = store.loadExpertMetrics('e2')
    expect(store.loading.metrics).toBe(true)
    second.resolve(metricsOf('e2'))
    expect(await b).toMatchObject({ expertId: 'e2' })
    first.resolve(metricsOf('e1'))
    expect(await a).toBeNull()
    // 旧响应回来后既不覆写数据，也不把新一次请求的 loading 提前抹掉
    expect(store.expertMetrics.expertId).toBe('e2')
    expect(store.loading.metrics).toBe(false)
  })
})

// ── 注册中心写面 ────────────────────────────────────────────────────
// 三条端点都不回列表，所以「页面接下来显示什么」全在 store 里定；
// 这里逐条钉住：校验发在前端、不确认就不改本地投影、单向操作要说清代价。
describe('专家注册中心写面', () => {
  const ok = (over = {}) => ({
    expert: null, id: '', created: false, updated: false, deleted: false, softDelete: false, message: '', ...over
  })
  const listed = () => api.listExperts.mockResolvedValue({ items: [], total: 0, page: 1, pageSize: 24 })

  beforeEach(() => {
    api.expertsStats.mockResolvedValue({ totalExperts: 0, onlineExperts: 0 })
    api.listExpertCapabilities.mockResolvedValue({ total: 0, items: [] })
  })

  describe('注册（POST /api/experts）', () => {
    it('空名称在前端就拦下：后端 400 的原因串是英文的，不该让用户去网关报错里找字段', async () => {
      const store = useAllianceExpertsStore()
      await expect(store.registerExpert({ ...emptyExpertDraft(), name: '  ' })).resolves.toBeNull()
      expect(api.registerExpert).not.toHaveBeenCalled()
      expect(store.error.action).toBe('专家名称不能为空')
      expect(store.loading.action).toBe(false)
    })

    it('成功后重查列表与两处聚合，并把 id 写进提示（新行未必落在当前筛选内）', async () => {
      listed()
      api.registerExpert.mockResolvedValue(ok({ created: true, id: 'exp-9', expert: expert('exp-9') }))
      const store = useAllianceExpertsStore()
      store.experts = [expert('e1')]
      store.total = 1
      const res = await store.registerExpert({ ...emptyExpertDraft(), name: '新专家' })
      expect(api.registerExpert).toHaveBeenCalledWith(expect.objectContaining({ name: '新专家' }))
      expect(res.id).toBe('exp-9')
      expect(api.listExperts).toHaveBeenCalledTimes(1)
      expect(api.expertsStats).toHaveBeenCalledTimes(1)
      expect(api.listExpertCapabilities).toHaveBeenCalledTimes(1)
      // 列表是重查回来的，不是本地 push 的
      expect(store.experts).toEqual([])
      expect(store.notice).toContain('exp-9')
      expect(store.notice).toContain('筛选')
    })

    it('后端没回 created + expert 就不乐观收单：行不增、列表不刷', async () => {
      api.registerExpert.mockResolvedValue(ok({ id: 'exp-9' }))
      const store = useAllianceExpertsStore()
      await expect(store.registerExpert({ ...emptyExpertDraft(), name: '新专家' })).resolves.toBeNull()
      expect(store.error.action).toContain('不可信')
      expect(api.listExperts).not.toHaveBeenCalled()
      expect(store.notice).toBe('')
    })

    it('id 撞号 400 经 error.action 单点冒泡，不静默重试', async () => {
      api.registerExpert.mockRejectedValue(fail('expert id already exists: exp-9'))
      const store = useAllianceExpertsStore()
      await expect(store.registerExpert({ ...emptyExpertDraft(), name: '新专家' })).resolves.toBeNull()
      expect(store.error.action).toBe('expert id already exists: exp-9')
      expect(api.listExperts).not.toHaveBeenCalled()
    })
  })

  describe('编辑（PUT /api/experts/:id，合并式）', () => {
    const target = () => ({ ...expert('e1'), title: '', domains: ['rust'] })

    it('没有改动不发请求：发了会把后端里的值原样盖一遍，还白盖一次 updated_at', async () => {
      const row = target()
      const store = useAllianceExpertsStore()
      store.experts = [row]
      await expect(store.saveExpert(row, expertFormDraft(row))).resolves.toBeNull()
      expect(api.updateExpert).not.toHaveBeenCalled()
      expect(store.notice).toBe('没有需要保存的改动')
    })

    it('patch 只含改动过的键，未改的数组一个都不发', async () => {
      const row = target()
      listed()
      api.updateExpert.mockResolvedValue(ok({ updated: true, expert: { ...row, title: '资深' } }))
      const store = useAllianceExpertsStore()
      store.experts = [row]
      const res = await store.saveExpert(row, { ...expertFormDraft(row), title: '资深' })
      expect(api.updateExpert).toHaveBeenCalledWith('e1', { title: '资深' })
      expect(res.title).toBe('资深')
      expect(api.listExperts).not.toHaveBeenCalled()
    })

    it('数组改动发整值，并且就地替换列表里那一条而不重查', async () => {
      const row = target()
      api.updateExpert.mockResolvedValue(ok({ updated: true, expert: { ...row, domains: ['rust', 'golang'] } }))
      const store = useAllianceExpertsStore()
      store.experts = [row, expert('e2')]
      await store.saveExpert(row, { ...expertFormDraft(row), domains: ['rust', 'golang'] })
      expect(api.updateExpert).toHaveBeenCalledWith('e1', { domains: ['rust', 'golang'] })
      expect(store.experts.map((e) => e.domains?.[1])).toEqual(['golang', undefined])
      expect(store.experts).toHaveLength(2)
    })

    it('清空名称由契约拦下：PUT 不再校验 name，后端会真把它写成空串', async () => {
      const row = target()
      const store = useAllianceExpertsStore()
      store.experts = [row]
      await expect(store.saveExpert(row, { ...expertFormDraft(row), name: '' })).resolves.toBeNull()
      expect(store.error.action).toBe('专家名称不能为空')
      expect(api.updateExpert).not.toHaveBeenCalled()
    })

    it('已停用的专家 PUT 必 404：文案落到 action 位，本地行不动', async () => {
      const row = target()
      api.updateExpert.mockRejectedValue(fail('expert not found: e1'))
      const store = useAllianceExpertsStore()
      store.experts = [row]
      await expect(store.saveExpert(row, { ...expertFormDraft(row), title: '资深' })).resolves.toBeNull()
      expect(store.error.action).toBe('expert not found: e1')
      expect(store.experts[0].title).toBe('')
    })

    it('后端没回 updated 时不覆盖本地行', async () => {
      const row = target()
      api.updateExpert.mockResolvedValue(ok({ expert: { ...row, title: '资深' } }))
      const store = useAllianceExpertsStore()
      store.experts = [row]
      await expect(store.saveExpert(row, { ...expertFormDraft(row), title: '资深' })).resolves.toBeNull()
      expect(store.error.action).toContain('改动未确认')
      expect(store.experts[0].title).toBe('')
    })
  })

  describe('停用（DELETE /api/experts/:id，软删且不可逆）', () => {
    it('后端确认后四处本地投影一起剔除，提示讲明这是单向操作', async () => {
      api.deleteExpert.mockResolvedValue(ok({ deleted: true, softDelete: true, id: 'e1' }))
      const store = useAllianceExpertsStore()
      store.experts = [expert('e1'), expert('e2')]
      store.total = 41
      store.favorites = new Set(['e1', 'e2'])
      store.bookings = [{ ...booking('b1') }, { ...booking('b2', 'confirmed'), expertId: 'e2' }]
      await expect(store.removeExpert(expert('e1'))).resolves.toMatchObject({ deleted: true })
      expect(api.deleteExpert).toHaveBeenCalledWith('e1')
      expect(store.experts.map((e) => e.id)).toEqual(['e2'])
      expect(store.total).toBe(40)
      expect(store.favoriteCount).toBe(1)
      expect(store.bookings.map((b) => b.expertId)).toEqual(['e2'])
      expect(store.notice).toContain('不能再次注册')
      expect(store.notice).toContain('再启用')
      expect(api.expertsStats).toHaveBeenCalledTimes(1)
      expect(api.listExpertCapabilities).toHaveBeenCalledTimes(1)
    })

    it('后端没给 deleted 就不动行：删没删成由响应说话，不按 200 猜', async () => {
      api.deleteExpert.mockResolvedValue(ok({ id: 'e1' }))
      const store = useAllianceExpertsStore()
      store.experts = [expert('e1')]
      store.total = 1
      await expect(store.removeExpert(expert('e1'))).resolves.toBeNull()
      expect(store.error.action).toContain('未确认')
      expect(store.experts).toHaveLength(1)
      expect(store.total).toBe(1)
      expect(api.listExpertCapabilities).not.toHaveBeenCalled()
    })

    it('空 id 不发请求', async () => {
      const store = useAllianceExpertsStore()
      await expect(store.removeExpert(null)).resolves.toBeNull()
      expect(api.deleteExpert).not.toHaveBeenCalled()
    })
  })
})

describe('智能匹配（POST /api/alliance/experts/search）', () => {
  const deferred = () => {
    let resolve
    const p = new Promise((r) => { resolve = r })
    return { p, resolve }
  }
  const candidate = (id, score) => ({
    id, name: `匹配${id}`, description: 'd', domains: ['kg'], status: 'online', matchScore: score
  })
  const result = (items, total = items.length) => ({ items, total })

  it('成功即持有候选并记下打分依据的描述', async () => {
    api.searchExperts.mockResolvedValue(result([candidate('m1', 0.8), candidate('m2', 0.35)]))
    const store = useAllianceExpertsStore()
    await expect(store.searchExpertMatches({ query: '  需要图谱补全  ', domains: ['kg'] }))
      .resolves.toMatchObject({ total: 2 })
    expect(api.searchExperts).toHaveBeenCalledWith({ query: '需要图谱补全', domains: ['kg'], limit: 10 })
    expect(store.expertMatches.query).toBe('需要图谱补全')
    expect(store.expertMatches.items[0].matchScore).toBe(0.8)
    expect(store.loading.match).toBe(false)
    expect(store.error.match).toBe('')
  })

  it('描述不足两个字不发请求，原因落在 match 而不是 action', async () => {
    const store = useAllianceExpertsStore()
    await expect(store.searchExpertMatches({ query: '图' })).resolves.toBeNull()
    expect(api.searchExperts).not.toHaveBeenCalled()
    expect(store.error.match).toContain('任务描述')
    expect(store.error.action).toBe('')
    expect(store.expertMatches).toBeNull()
  })

  it('后端失败不折叠成空结果：留错误文案、候选仍为 null', async () => {
    api.searchExperts.mockRejectedValue(fail('matcher unavailable'))
    const store = useAllianceExpertsStore()
    await expect(store.searchExpertMatches({ query: '做一次知识图谱补全' })).resolves.toBeNull()
    expect(store.error.match).toBe('matcher unavailable')
    expect(store.expertMatches).toBeNull()
    expect(store.loading.match).toBe(false)
  })

  it('零命中是事实而不是错误', async () => {
    api.searchExperts.mockResolvedValue(result([], 0))
    const store = useAllianceExpertsStore()
    await store.searchExpertMatches({ query: '量子色动力学格点计算' })
    expect(store.expertMatches.items).toEqual([])
    expect(store.expertMatches.total).toBe(0)
    expect(store.error.match).toBe('')
  })

  it('连点两次时慢的那次响应不能盖掉新查询', async () => {
    const first = deferred()
    const second = deferred()
    api.searchExperts.mockImplementationOnce(() => first.p).mockImplementationOnce(() => second.p)
    const store = useAllianceExpertsStore()
    const a = store.searchExpertMatches({ query: '第一个查询' })
    const b = store.searchExpertMatches({ query: '第二个查询' })
    expect(store.loading.match).toBe(true)
    second.resolve(result([candidate('m2', 0.6)]))
    expect(await b).toMatchObject({ total: 1 })
    first.resolve(result([candidate('m1', 0.9), candidate('m1b', 0.1)]))
    expect(await a).toBeNull()
    expect(store.expertMatches.items[0].id).toBe('m2')
    expect(store.expertMatches.query).toBe('第二个查询')
    expect(store.loading.match).toBe(false)
  })

  it('清空后候选、错误与加载位一并归零', async () => {
    api.searchExperts.mockResolvedValue(result([candidate('m1', 0.5)]))
    const store = useAllianceExpertsStore()
    await store.searchExpertMatches({ query: '随便一个任务描述' })
    store.clearExpertMatches()
    expect(store.expertMatches).toBeNull()
    expect(store.error.match).toBe('')
    expect(store.loading.match).toBe(false)
  })
})

describe('T4 SSE 注册表事件帧 → 防抖真拉列表', () => {
  // 帧信封形状对齐后端 experts_events.rs（ExpertRegistered/ExpertDisabled 带 expert_id）。
  const expertRegistered = { id: 'evt-r1', type: 'ExpertRegistered', expert_id: 'exp-new', source: 'register_expert', tenant: 't', occurred_at: '2026-10-03T09:00:00Z' }
  const expertDisabled = { id: 'evt-d1', type: 'ExpertDisabled', expert_id: 'exp-old', source: 'delete_expert', tenant: 't', occurred_at: '2026-10-03T09:01:00Z' }
  const planFrame = { id: 'evt-p1', type: 'PlanStatusChanged', plan_id: 'plan-1', from: 'draft', to: 'running', source: 'execute', tenant: 't', occurred_at: '2026-10-03T09:02:00Z' }

  beforeEach(() => {
    vi.useFakeTimers()
    api.listExperts.mockResolvedValue({ items: [], total: 0, page: 1, pageSize: 24 })
    api.expertsStats.mockResolvedValue({ total: 0, online: 0 })
    api.listExpertCapabilities.mockResolvedValue({ items: [], capabilities: [] })
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('带 expert_id 的注册帧命中，800ms 防抖后真拉列表与统计', () => {
    const store = useAllianceExpertsStore()
    expect(store.applyRegistryEvent('ExpertRegistered', expertRegistered)).toBe(true)
    // 防抖窗口内不应发请求
    expect(api.listExperts).not.toHaveBeenCalled()
    vi.advanceTimersByTime(800)
    expect(api.listExperts).toHaveBeenCalledTimes(1)
    expect(api.expertsStats).toHaveBeenCalledTimes(1)
    // 能力目录用户没点开过（capabilities 为 null），不替他起请求
    expect(api.listExpertCapabilities).not.toHaveBeenCalled()
  })

  it('突发两帧防抖合并成一次重拉', () => {
    const store = useAllianceExpertsStore()
    store.applyRegistryEvent('ExpertRegistered', expertRegistered)
    store.applyRegistryEvent('ExpertDisabled', expertDisabled)
    vi.advanceTimersByTime(800)
    expect(api.listExperts).toHaveBeenCalledTimes(1)
    expect(api.expertsStats).toHaveBeenCalledTimes(1)
  })

  it('不带 expert_id 的帧（Plan*）与注册表无关，忽略且不重拉', () => {
    const store = useAllianceExpertsStore()
    expect(store.applyRegistryEvent('PlanStatusChanged', planFrame)).toBe(false)
    vi.advanceTimersByTime(2000)
    expect(api.listExperts).not.toHaveBeenCalled()
    expect(api.expertsStats).not.toHaveBeenCalled()
  })

  it('能力目录已加载过时，身份变更帧一并补拉目录', () => {
    const store = useAllianceExpertsStore()
    store.capabilities = { items: [], capabilities: [] }
    store.applyRegistryEvent('ExpertDisabled', expertDisabled)
    vi.advanceTimersByTime(800)
    expect(api.listExperts).toHaveBeenCalledTimes(1)
    expect(api.listExpertCapabilities).toHaveBeenCalledTimes(1)
  })

  it('clearRegistryEventTimer 挂起未触发的防抖重拉', () => {
    const store = useAllianceExpertsStore()
    store.applyRegistryEvent('ExpertRegistered', expertRegistered)
    store.clearRegistryEventTimer()
    vi.advanceTimersByTime(2000)
    expect(api.listExperts).not.toHaveBeenCalled()
  })
})

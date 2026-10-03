// 专家广场状态：列表/筛选/收藏/预约/即时咨询的唯一持有者。
// 视图只读绑定 + 只发意图；所有后端错误经 error 单点冒泡，不在 api 层吞掉。
import { defineStore } from 'pinia'
import { computed, reactive, ref, watch } from 'vue'
import { useAuthStore } from '@/stores'
import { createRequestFence } from '@/modules/expert-alliance/model'
import { allianceApi } from '@/modules/expert-alliance/api'
import { BOOKING_STATUS, expertDisplayName, expertNameOr, isConsultable } from '@/modules/expert-alliance/contract'
import { deleteResultText, expertDraftProblem, expertFormDraft, expertPatch } from '@/modules/expert-alliance/contract'
import { createFavoriteRequest, favoriteRejectionIsDefinitive } from '@/modules/expert-alliance/contract'
import { favoriteJournalScope, readFavoriteJournal, writeFavoriteJournal } from '@/modules/expert-alliance/contract'

const EMPTY_FILTERS = () => ({ search: '', domain: '', skill: '', status: '', expertType: '', sort: '' })
// 分页由 store 单独管（page/pageSize），可被筛选覆盖的只有后端认识的这几个字段
export const EXPERT_FILTER_KEYS = Object.freeze(Object.keys(EMPTY_FILTERS()))

export const useAllianceExpertsStore = defineStore('allianceExperts', () => {
  const api = allianceApi
  const auth = useAuthStore()
  const readFence = createRequestFence()
  const favoriteAttempts = new Map()
  let favoriteEpoch = 0
  let listSequence = 0
  let favoriteVersion = 0
  let journalScope = null
  let journalError = ''

  function restoreFavoriteAttempts() {
    journalScope = null
    journalError = ''
    favoriteAttempts.clear()
    if (!auth.accessToken) return
    try {
      journalScope = favoriteJournalScope(auth.userInfo)
      for (const [id, attempt] of readFavoriteJournal(globalThis.sessionStorage, journalScope)) favoriteAttempts.set(id, attempt)
    } catch (e) { journalError = e?.message || '待确认收藏记录不可读取' }
  }

  function persistFavoriteAttempts() {
    if (journalError) throw new Error(journalError)
    if (!journalScope) throw new Error('收藏操作需要完整的用户和租户身份')
    writeFavoriteJournal(globalThis.sessionStorage, journalScope, favoriteAttempts)
  }
  restoreFavoriteAttempts()

  const experts = ref([])
  const total = ref(0)
  const page = ref(1)
  const pageSize = ref(24)
  const filters = reactive(EMPTY_FILTERS())

  // 当前页投影来自 SQLite 批量快照；待确认请求沿用同一幂等键恢复。
  const favorites = ref(new Set())
  const favoriteSessionOnly = false

  const bookings = ref([])
  const bookingCounts = reactive({ pending: 0, confirmed: 0, completed: 0, cancelled: 0 })

  // 平台统计来自 GET /api/experts/stats，与本页列表是两套口径，单独持有
  const stats = ref(null)
  // 能力目录（GET /api/experts/capabilities）：全平台聚合，与本页筛选结果无关
  const capabilities = ref(null)
  // 单专家派生指标（GET /api/experts/:id/metrics）：一次只看一位，切换即清空
  const expertMetrics = ref(null)
  // 智能匹配（POST /api/alliance/experts/search）：打分来自联盟匹配器（alliance.rs:878-906），
  // 它命中的是匹配器自己的专家集，与本页 /api/experts 注册表分属两套口径，不假设 id 可互查。
  const expertMatches = ref(null)

  const loading = reactive({ list: false, bookings: false, action: false, stats: false, capabilities: false, metrics: false, match: false })
  const error = reactive({ list: '', bookings: '', action: '', stats: '', capabilities: '', metrics: '', match: '' })
  const notice = ref('')
  watch(() => [auth.accessToken, auth.userInfo?.id, auth.userInfo?.tenant_id], (current, previous) => {
    const samePrincipal = current[0] && previous[0] && current[1] && current[1] === previous[1] && current[2] === previous[2]
    favoriteEpoch++
    readFence.invalidate()
    metricsToken++; matchToken++
    clearRegistryEventTimer()
    listSequence++
    if (!samePrincipal) {
      // Keep the old principal's uncertain journal isolated; never replay it as the new user.
      restoreFavoriteAttempts()
      favorites.value = new Set()
      experts.value = []
      total.value = 0
      bookings.value = []
      Object.keys(bookingCounts).forEach(key => { bookingCounts[key] = 0 })
      stats.value = null; capabilities.value = null
      expertMetrics.value = null; expertMatches.value = null
    }
    Object.keys(loading).forEach(key => { loading[key] = false })
    Object.keys(error).forEach(key => { error[key] = '' })
    error.action = ''
    notice.value = ''
  }, { flush: 'sync' })

  const onlineCount = computed(() => experts.value.filter((e) => isConsultable(e.availability.status)).length)
  const pageCount = computed(() => Math.max(1, Math.ceil(total.value / pageSize.value)))
  const favoriteCount = computed(() => favorites.value.size)
  const bookedExpertIds = computed(() => new Set(
    bookings.value
      .filter((b) => b.status === BOOKING_STATUS.PENDING || b.status === BOOKING_STATUS.CONFIRMED)
      .map((b) => b.expertId)
  ))

  // 候选下拉与 id→名字映射的唯一出处：编排页与会话页此前各自抄了一份同样的三行，
  // 且都直接把注册表 name 印到界面上（那里有两行的名字在写入侧就丢成了问号）。
  const expertOptions = computed(() =>
    experts.value.map((e) => ({ value: e.id, label: expertDisplayName(e) }))
  )
  const expertNames = computed(() => {
    const map = {}
    for (const e of experts.value) map[e.id] = expertDisplayName(e)
    return map
  })

  function isFavorite(expertId) {
    return favorites.value.has(expertId)
  }

  /** 只把后端认识的字段写进 filters，视图 v-model 绑定同一对象 */
  function setFilters(patch = {}) {
    for (const key of EXPERT_FILTER_KEYS) {
      if (patch[key] !== undefined) filters[key] = patch[key]
    }
  }

  function resetFilters() {
    Object.assign(filters, EMPTY_FILTERS())
    page.value = 1
  }

  function goPage(next) {
    page.value = Math.min(Math.max(1, next), pageCount.value)
  }

  async function loadExperts() {
    const sequence = ++listSequence
    const epoch = favoriteEpoch
    const current = () => sequence === listSequence && epoch === favoriteEpoch
    loading.list = true
    error.list = ''
    try {
      const res = await api.listExperts({ ...filters, page: page.value, pageSize: pageSize.value })
      if (!current()) return
      await recoverFavorites(current)
      if (!current()) return
      const version = favoriteVersion
      const states = await api.readFavorites(res.items.map(e => e.id))
      if (!current()) return
      if (version !== favoriteVersion || loading.action) throw new Error('收藏状态正在更新，请稍后刷新')
      favorites.value = new Set(states.filter(e => e.favorite).map(e => e.expertId))
      experts.value = res.items
      total.value = res.total
      page.value = res.page
    } catch (e) {
      if (!current()) return
      error.list = e?.msg || e?.message || '专家列表获取失败'
      experts.value = []
      total.value = 0
    } finally {
      if (current()) loading.list = false
    }
  }

  async function recoverFavorites(current = () => true) {
    if (journalError) throw new Error(journalError)
    // Do not race recovery against a user-initiated write.
    if (loading.action) throw new Error('收藏请求正在处理，请稍后刷新')
    for (const attempt of [...favoriteAttempts.values()]) {
      if (!current()) return
      const receipt = await toggleFavorite({ id: attempt.expertId })
      if (!current()) return
      if (!receipt) throw new Error(error.action || '待确认收藏请求恢复失败')
    }
  }

  async function loadBookings() {
    const current = readFence.begin('bookings')
    loading.bookings = true
    error.bookings = ''
    try {
      const res = await api.listMyBookings()
      if (!current()) return
      bookings.value = res.items
      Object.assign(bookingCounts, res.counts)
    } catch (e) {
      if (!current()) return
      error.bookings = e?.msg || e?.message || '我的预约获取失败'
      bookings.value = []
    } finally {
      if (current()) loading.bookings = false
    }
  }

  async function loadStats() {
    const current = readFence.begin('stats')
    loading.stats = true
    error.stats = ''
    try {
      const next = await api.expertsStats()
      if (!current()) return
      stats.value = next
    } catch (e) {
      if (!current()) return
      error.stats = e?.msg || e?.message || '平台统计获取失败'
      stats.value = null
    } finally {
      if (current()) loading.stats = false
    }
  }

  async function loadCapabilities() {
    const current = readFence.begin('capabilities')
    loading.capabilities = true
    error.capabilities = ''
    try {
      const next = await api.listExpertCapabilities()
      if (!current()) return
      capabilities.value = next
    } catch (e) {
      if (!current()) return
      error.capabilities = e?.msg || e?.message || '能力目录获取失败'
      capabilities.value = null
    } finally {
      if (current()) loading.capabilities = false
    }
  }

  // 抽屉里连点两位专家时，慢的那次响应后到；没有这次序号判断，旧专家的派生指标会盖掉新专家的
  let metricsToken = 0

  async function loadExpertMetrics(expertId) {
    const token = ++metricsToken
    loading.metrics = true
    error.metrics = ''
    expertMetrics.value = null
    if (!expertId) {
      loading.metrics = false
      return null
    }
    try {
      const res = await api.getExpertMetrics(expertId)
      if (token !== metricsToken) return null
      expertMetrics.value = res
      return res
    } catch (e) {
      if (token !== metricsToken) return null
      // 后端对未注册与已停用的专家都返回 404，这里不把它折叠成"该专家没有指标"
      error.metrics = e?.msg || e?.message || '派生指标获取失败'
      return null
    } finally {
      if (token === metricsToken) loading.metrics = false
    }
  }

  // 连点两次「开始匹配」时慢的那次响应后到，旧查询的候选会盖掉新查询的结果
  let matchToken = 0

  /** 空描述不发请求：后端把 query 当任务描述打分，空串只会得到一屏 0 分噪声 */
  async function searchExpertMatches({ query, domains = [], limit = 10 } = {}) {
    const q = (query || '').trim()
    if (q.length < 2) {
      error.match = '请先写出要交给专家的任务描述（至少两个字）'
      expertMatches.value = null
      return null
    }
    const token = ++matchToken
    loading.match = true
    error.match = ''
    expertMatches.value = null
    try {
      const res = await api.searchExperts({ query: q, domains, limit })
      if (token !== matchToken) return null
      expertMatches.value = { ...res, query: q }
      return res
    } catch (e) {
      if (token !== matchToken) return null
      error.match = e?.msg || e?.message || '专家匹配失败'
      return null
    } finally {
      if (token === matchToken) loading.match = false
    }
  }

  function clearExpertMatches() {
    matchToken++
    expertMatches.value = null
    error.match = ''
    loading.match = false
  }

  /** 写操作单点执行：成功可选回填提示，失败写入 error.action 并返回 null */
  async function run(fn, successMessage = '', isCurrent = () => true) {
    loading.action = true
    error.action = ''
    notice.value = ''
    try {
      const result = await fn()
      if (!isCurrent()) return null
      notice.value = successMessage
      return result
    } catch (e) {
      if (!isCurrent()) return null
      error.action = e?.msg || e?.message || '操作失败'
      return null
    } finally {
      if (isCurrent()) loading.action = false
    }
  }

  async function toggleFavorite(expert) {
    if (!expert?.id) return null
    if (loading.action) return null
    favoriteVersion++
    const epoch = favoriteEpoch
    const isCurrent = () => epoch === favoriteEpoch
    const res = await run(async () => {
      const attempt = favoriteAttempts.get(expert.id) || createFavoriteRequest(expert.id)
      favoriteAttempts.set(expert.id, attempt)
      persistFavoriteAttempts()
      try {
        const receipt = await api.toggleFavorite(attempt.expertId, { idempotencyKey: attempt.key })
        if (isCurrent()) {
          favoriteAttempts.delete(expert.id)
          try { persistFavoriteAttempts() } catch (e) { favoriteAttempts.set(expert.id, attempt); throw e }
        }
        return receipt
      } catch (e) {
        if (isCurrent() && favoriteRejectionIsDefinitive(e)) {
          favoriteAttempts.delete(expert.id)
          try { persistFavoriteAttempts() } catch (failure) { favoriteAttempts.set(expert.id, attempt); throw failure }
        }
        throw e
      }
    }, '', isCurrent)
    favoriteVersion++
    if (!res) {
      if (isCurrent() && favoriteAttempts.has(expert.id)) error.action += '；再次点击或刷新页面将沿用原请求恢复确认'
      return null
    }
    // 以服务端返回为准，本地 Set 只做镜像
    const set = new Set(favorites.value)
    if (res.favorite) set.add(res.expertId)
    else set.delete(res.expertId)
    favorites.value = set
    notice.value = `${res.favorite ? '已收藏' : '已取消收藏'} ${expert.name}`
    return res
  }

  async function createBooking({ expertId, topic, scheduledAt, durationMinutes }) {
    const res = await run(() => api.createBooking({ expertId, topic, scheduledAt, durationMinutes }))
    if (!res) return null
    await loadBookings()
    notice.value = `已预约「${res.expertName || res.expertId}」· ${res.topic || topic}`
    return res
  }

  async function cancelBooking(bookingId) {
    const res = await run(() => api.cancelBooking(bookingId), '预约已取消')
    if (!res) return null
    await loadBookings()
    return res
  }

  /** 即时咨询：后端对不在线的专家返回 200 + sessionId=null，这里统一按结果判定 */
  async function consultNow(expert, { topic = '即时咨询', question = '', channel = 'text' } = {}) {
    if (!expert?.id) return null
    const res = await run(() => api.consultNow(expert.id, { topic, question, channel }))
    if (!res) return null
    notice.value = res.sessionId
      ? `已接入 ${expert.name} 的会话`
      : res.message || `${expert.name} 当前不可即时咨询`
    return res
  }

  function openRoom(bookingId) {
    return run(() => api.consultRoom(bookingId))
  }

  function joinTeam({ teamId, expertId, role = 'member' }) {
    return run(() => api.joinTeam({ teamId, expertId, role }))
  }

  // ── 注册中心写面（POST / PUT / DELETE /api/experts）───────────────
  // 三条都不回列表：POST/PUT 只回那一条专家，DELETE 只回一个标记，
  // 所以「改完之后页面该显示什么」必须由 store 明确决定，不能让视图各自猜。

  /** 校验在前端做完再发：空 name 后端必 400，而 400 的原因串是英文的 */
  async function registerExpert(draft) {
    const invalid = expertDraftProblem(draft)
    if (invalid) {
      error.action = invalid
      notice.value = ''
      return null
    }
    const res = await run(() => api.registerExpert(draft))
    if (!res) return null
    if (!res.created || !res.expert) {
      error.action = '后端未回 created/expert，注册结果不可信'
      return null
    }
    // 新专家是否落在当前筛选与本页之内不可知，所以刷列表而不是本地插一条，
    // 并把 id 写进提示，让用户能直接搜到它
    await Promise.all([loadExperts(), loadStats(), loadCapabilities()])
    notice.value = `已注册 ${res.expert.name}（id ${res.expert.id}），如未显示说明它不符合当前筛选条件`
    return res.expert
  }

  /** 编辑保存：只发改动过的键；空改动不发请求，否则后端会白盖一次 updated_at */
  async function saveExpert(expert, draft) {
    if (!expert?.id) return null
    const { patch, problem } = expertPatch(expertFormDraft(expert), draft)
    if (problem) {
      error.action = problem
      notice.value = ''
      return null
    }
    if (!Object.keys(patch).length) {
      error.action = ''
      notice.value = '没有需要保存的改动'
      return null
    }
    const res = await run(() => api.updateExpert(expert.id, patch))
    if (!res) return null
    // 后端对「不存在」与「已停用」都回 404，ApiError 已在 run 里落到 error.action
    if (!res.updated || !res.expert) {
      error.action = '后端未回 updated/expert，改动未确认'
      return null
    }
    const idx = experts.value.findIndex((e) => e.id === res.expert.id)
    if (idx >= 0) experts.value[idx] = res.expert
    await Promise.all([loadStats(), loadCapabilities()])
    notice.value = `已保存 ${res.expert.name} 的改动（${Object.keys(patch).length} 项）`
    return res.expert
  }

  /** 停用（软删）：单向操作，提示文案来自 contract/deleteConsequences */
  async function removeExpert(expert) {
    if (!expert?.id) return null
    const res = await run(() => api.deleteExpert(expert.id))
    if (!res) return null
    if (!res.deleted) {
      error.action = deleteResultText(res)
      return null
    }
    experts.value = experts.value.filter((e) => e.id !== expert.id)
    total.value = Math.max(0, total.value - 1)
    const set = new Set(favorites.value)
    set.delete(expert.id)
    favorites.value = set
    bookings.value = bookings.value.filter((b) => b.expertId !== expert.id)
    await Promise.all([loadStats(), loadCapabilities()])
    notice.value = `已停用 ${expertNameOr(expert, expert.id)}：该 id 不能再次注册，也没有再启用入口`
    return res
  }

  // ── T4 SSE 注册表实时面（useAllianceEventStream → 本 store）──────────────
  // 与编排台同口径：事件帧只是「该重拉了」的防抖提示，绝不本地猜算列表/计数——
  // 真值永远由 loadExperts/loadStats 真拉回。只按信封形状（带 expert_id＝注册表身份变更：
  // 注册/停用/改档）判定，不硬编码事件名字面量，避免在 store 里重打契约值域。
  let registryTimer = null

  /**
   * 收一帧 T4 业务事件。带 expert_id 的帧意味着注册表现行已变（别处注册/停用/改档），
   * 防抖 800ms 合并突发帧后真拉列表与统计；不带 expert_id 的帧（Plan* 等）与本页无关，忽略。
   * 返回是否命中注册表身份帧，便于单测断言「store 因该帧而动」。
   */
  function applyRegistryEvent(kind, envelope = {}) {
    if (!envelope.expert_id) return false
    if (registryTimer) clearTimeout(registryTimer)
    registryTimer = setTimeout(() => {
      registryTimer = null
      loadExperts()
      loadStats()
      // 能力目录只在用户已点开过的前提下才补拉，不替用户起一次他没要的请求
      if (capabilities.value) loadCapabilities()
    }, 800)
    return true
  }

  function clearRegistryEventTimer() {
    if (registryTimer) { clearTimeout(registryTimer); registryTimer = null }
  }

  return {
    experts, total, page, pageSize, filters, bookings, bookingCounts, stats,
    capabilities, expertMetrics, expertMatches,
    favorites, favoriteSessionOnly, favoriteCount, onlineCount, pageCount, bookedExpertIds,
    expertOptions, expertNames,
    loading, error, notice,
    isFavorite, setFilters, resetFilters, goPage,
    loadExperts, loadBookings, loadStats, loadCapabilities, loadExpertMetrics,
    searchExpertMatches, clearExpertMatches,
    toggleFavorite, recoverFavorites, createBooking, cancelBooking,
    consultNow, openRoom, joinTeam,
    registerExpert, saveExpert, removeExpert,
    applyRegistryEvent, clearRegistryEventTimer
  }
})

// 编排面状态：只持有表单、最近一次响应与读数，一切规则取自 contract/orchestration.js。
// 实际执行、模型产出、计划模板与初值分别标来源；视图不另算这些口径。
import { defineStore } from 'pinia'
import { computed, reactive, ref, watch } from 'vue'
import { useAuthStore } from '@/stores'
import { createRequestFence } from '@/modules/expert-alliance/model'
import { allianceApi } from '@/modules/expert-alliance/api'
import {
  ORCH_DEFAULT_FUSION_STRATEGY, ORCH_DEFAULT_TASK_TYPE, ORCH_MAX_EXPERTS_DEFAULT, ORCH_SIMULATED,
  ORCH_TASK_TYPE_TABLES, ORCH_ZERO_COUNTERS, orchEmptyExpertsNote, orchExecuteOutcome, orchHistoryAnomaly,
  orchFallbackNote, orchHistoryPages, orchProvenanceOf, orchRunDisclaimer, orchStatusSplitNote, orchStepExpertNote,
  orchTopologyNote, orchUsesFallbackTable, orchVolatileNote, orchZeroCounterNote, orchestrateProblem, planExecuteProblem
} from '@/modules/expert-alliance/contract'

/** 历史页大小默认与后端一致（:937 的 unwrap_or(20)） */
const HISTORY_PAGE_SIZE = 20

export const useAllianceOrchStore = defineStore('allianceOrch', () => {
  const auth = useAuthStore()
  const requests = createRequestFence()
  const api = allianceApi

  const form = reactive({
    task: '',
    taskType: '',
    fusionStrategy: '',
    maxExperts: ORCH_MAX_EXPERTS_DEFAULT,
    expertIds: []
  })
  const orchestration = ref(null)
  const plan = ref(null)
  const execution = ref(null)
  const stats = ref(null)
  const history = ref({ records: [], total: 0, page: 1, pageSize: HISTORY_PAGE_SIZE })
  const filters = reactive({ status: '', taskType: '' })
  const loading = reactive({ run: false, plan: false, execute: false, stats: false, history: false })
  const error = reactive({ run: '', plan: '', execute: '', stats: '', history: '' })

  // ── T4 SSE 实时事件面（experts_streams.rs → useAllianceEventStream）──
  // 这里只把真实到达的帧原样记一份（视图可见「事件到了」），绝不本地猜算计划计数——
  // 统计/历史的真值永远由 loadStats/loadHistory 真拉回，帧只作「该重拉了」的提示。
  const liveEvents = ref([])
  let statsTimer = null
  watch(() => [auth.accessToken, auth.userInfo?.id, auth.userInfo?.tenant_id], () => {
    requests.invalidate()
    clearLiveEvents()
    orchestration.value = null; plan.value = null; execution.value = null; stats.value = null
    history.value = { records: [], total: 0, page: 1, pageSize: HISTORY_PAGE_SIZE }
    form.expertIds = []
    Object.keys(loading).forEach(key => { loading[key] = false })
    Object.keys(error).forEach(key => { error[key] = '' })
  }, { flush: 'sync' })


  // 一次页面动作只留一条最近结果：编排与计划互不覆盖，视图分别绑定三个 ref
  const wire = computed(() => ({
    task: form.task,
    taskType: form.taskType || ORCH_DEFAULT_TASK_TYPE,
    fusionStrategy: form.fusionStrategy || ORCH_DEFAULT_FUSION_STRATEGY,
    maxExperts: form.maxExperts,
    expertIds: form.expertIds
  }))

  const validation = computed(() => orchestrateProblem(wire.value))
  const runnable = computed(() => !validation.value && !loading.run)
  const executable = computed(() => !planExecuteProblem({ planId: plan.value?.planId }) && !loading.execute)
  /** 当前计划是否还能执行：plans 是进程内表，网关重启后旧 plan_id 会被 404（:738-740） */
  const hasPlan = computed(() => !!plan.value?.planId)

  const disclaimer = computed(() => (orchestration.value || execution.value ? orchRunDisclaimer() : ''))
  const volatility = computed(() => orchVolatileNote())
  const fallbackTable = computed(() => orchUsesFallbackTable(wire.value.taskType))
  // 兜底与否由契约判定，视图只看这段文案有没有
  const fallbackNote = computed(() => (fallbackTable.value ? orchFallbackNote() : ''))
  const taskTypeTables = computed(() => ORCH_TASK_TYPE_TABLES)

  const steps = computed(() => orchestration.value?.steps || plan.value?.steps || [])
  const topologyNote = computed(() => orchTopologyNote(steps.value))
  const expertNotes = computed(() => {
    const out = []
    const orphan = orchStepExpertNote(steps.value)
    if (orphan) out.push(orphan)
    if (orchestration.value) {
      const empty = orchEmptyExpertsNote(orchestration.value)
      if (empty) out.push(empty)
    }
    return out
  })

  /** 执行结果的成败判定：HTTP 码不参与，只看 body（成环失败也是 200） */
  const outcome = computed(() => (execution.value ? orchExecuteOutcome(execution.value) : { failed: false, status: '', error: '', note: '' }))

  const simulatedLegend = computed(() => ORCH_SIMULATED.map((x) => ({ id: x.id, field: x.field, text: x.text, at: x.at })))
  const provenance = (path) => orchProvenanceOf(path)

  const statCells = computed(() => {
    const s = stats.value
    if (!s) return []
    return [
      { key: 'total_plans', label: '计划总数', value: s.totalPlans },
      { key: 'plans_draft', label: 'draft', value: s.plansDraft },
      { key: 'plans_ready', label: 'ready', value: s.plansReady },
      { key: 'plans_running', label: 'running', value: s.plansRunning },
      { key: 'plans_completed', label: 'completed', value: s.plansCompleted },
      { key: 'plans_failed', label: 'failed', value: s.plansFailed }
    ].map((c) => ({ ...c, zero: ORCH_ZERO_COUNTERS.includes(c.key) }))
  })
  const zeroCounterNotes = computed(() => ORCH_ZERO_COUNTERS.map((key) => ({ key, text: orchZeroCounterNote(key) })))
  const statusSplit = computed(() => orchStatusSplitNote(stats.value, history.value?.records || []))

  const historyPages = computed(() => orchHistoryPages(history.value))
  const historyAnomaly = computed(() => orchHistoryAnomaly(history.value))

  async function runOrchestrate() {
    if (validation.value) return null
    const current = requests.begin('run')
    loading.run = true
    error.run = ''
    orchestration.value = null
    try {
      const next = await api.orchestrate(wire.value)
      if (!current()) return null
      orchestration.value = next
      return orchestration.value
    } catch (e) {
      if (!current()) return null
      error.run = e?.msg || e?.message || '编排请求失败'
      return null
    } finally {
      if (current()) loading.run = false
    }
  }

  async function generatePlan() {
    if (validation.value) return null
    const current = requests.begin('plan')
    loading.plan = true
    error.plan = ''
    plan.value = null
    execution.value = null
    try {
      const next = await api.generateOrchPlan(wire.value)
      if (!current()) return null
      plan.value = next
      return plan.value
    } catch (e) {
      if (!current()) return null
      error.plan = e?.msg || e?.message || '计划生成失败'
      return null
    } finally {
      if (current()) loading.plan = false
    }
  }

  async function executePlan(stepIds) {
    const problem = planExecuteProblem({ planId: plan.value?.planId })
    if (problem) {
      error.execute = problem
      return null
    }
    const current = requests.begin('execute')
    loading.execute = true
    error.execute = ''
    execution.value = null
    try {
      // step_ids 传数组＝只跑这些步（Some(集合)），传 undefined＝全跑；空数组是"一步都不跑"，语义不同故不合并
      const next = await api.executeOrchPlan({ planId: plan.value.planId, stepIds })
      if (!current()) return null
      execution.value = next
      return execution.value
    } catch (e) {
      if (!current()) return null
      error.execute = e?.msg || e?.message || '计划执行失败'
      return null
    } finally {
      if (current()) loading.execute = false
    }
  }

  async function loadStats() {
    const current = requests.begin('stats')
    loading.stats = true
    error.stats = ''
    try {
      const next = await api.getOrchStats()
      if (!current()) return null
      stats.value = next
      return stats.value
    } catch (e) {
      if (!current()) return null
      error.stats = e?.msg || e?.message || '编排统计读取失败'
      return null
    } finally {
      if (current()) loading.stats = false
    }
  }

  async function loadHistory(page) {
    const current = requests.begin('history')
    loading.history = true
    error.history = ''
    try {
      const next = await api.getOrchHistory({
        page: page ?? history.value.page,
        pageSize: history.value.pageSize,
        status: filters.status,
        taskType: filters.taskType
      })
      if (!current()) return null
      history.value = next
      return next
    } catch (e) {
      if (!current()) return null
      error.history = e?.msg || e?.message || '编排历史读取失败'
      return null
    } finally {
      if (current()) loading.history = false
    }
  }

  function setHistoryFilter(key, value) {
    filters[key] = value || ''
    return loadHistory(1)
  }

  function toggleExpert(id) {
    if (!id) return false
    const idx = form.expertIds.indexOf(id)
    if (idx >= 0) form.expertIds.splice(idx, 1)
    else form.expertIds.push(id)
    return true
  }

  function reset() {
    requests.invalidate()
    Object.keys(loading).forEach(key => { loading[key] = false })
    form.task = ''
    form.taskType = ''
    form.fusionStrategy = ''
    form.maxExperts = ORCH_MAX_EXPERTS_DEFAULT
    form.expertIds = []
    orchestration.value = null
    plan.value = null
    execution.value = null
    error.run = ''
    error.plan = ''
    error.execute = ''
  }

  /**
   * 接收一帧 T4 业务事件（来自 useAllianceEventStream 的 onEvent）。
   *
   * 帧信封是 experts_events.rs 的扁平 serde：{ id, type, ...payload, source, tenant, occurred_at }。
   * - 带 plan_id 的帧（PlanCreated / PlanStatusChanged）→ 防抖 800ms 真拉统计与历史（合并突发帧）；
   * - 不带 plan_id 的帧（ExpertRegistered / ExpertDisabled）→ 只入事件流，不牵动编排读数。
   * 返回规范化后的那一行，便于单测断言「store 因帧而变」。
   */
  function applyAllianceEvent(kind, envelope = {}) {
    const ev = {
      id: envelope.id || '',
      kind,
      planId: envelope.plan_id || '',
      from: envelope.from || '',
      to: envelope.to || '',
      executionId: envelope.execution_id || '',
      taskType: envelope.task_type || '',
      title: envelope.title || '',
      expertId: envelope.expert_id || '',
      source: envelope.source || '',
      occurredAt: envelope.occurred_at || '',
      receivedAt: new Date().toISOString()
    }
    liveEvents.value.unshift(ev)
    if (liveEvents.value.length > 30) liveEvents.value.length = 30

    if (ev.planId) {
      if (statsTimer) clearTimeout(statsTimer)
      statsTimer = setTimeout(() => {
        statsTimer = null
        loadStats()
        loadHistory(history.value.page)
      }, 800)
    }
    return ev
  }

  function clearLiveEvents() {
    liveEvents.value = []
    if (statsTimer) { clearTimeout(statsTimer); statsTimer = null }
  }

  return {
    form, filters, orchestration, plan, execution, stats, history, loading, error, liveEvents,
    wire, validation, runnable, executable, hasPlan, steps, outcome,
    disclaimer, volatility, fallbackNote, taskTypeTables, topologyNote, expertNotes,
    simulatedLegend, provenance, statCells, zeroCounterNotes, statusSplit, historyPages, historyAnomaly,
    runOrchestrate, generatePlan, executePlan, loadStats, loadHistory, setHistoryFilter, toggleExpert, reset,
    applyAllianceEvent, clearLiveEvents
  }
})

// 编排面状态：只持有表单、最近一次响应与读数，一切规则取自 contract/orchestration.js。
// 本 store 的一条硬约束：**不把后端的字面量当成运行结果**——execution.status / step.status /
// plan.status 都是常量（ORCH_SIMULATED），界面要显示它们就得同时显示来源角标，
// 所以这里把角标与提示语一并派生，视图不得另算。
import { defineStore } from 'pinia'
import { computed, reactive, ref } from 'vue'
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
    loading.run = true
    error.run = ''
    orchestration.value = null
    try {
      orchestration.value = await api.orchestrate(wire.value)
      return orchestration.value
    } catch (e) {
      error.run = e?.msg || e?.message || '编排请求失败'
      return null
    } finally {
      loading.run = false
    }
  }

  async function generatePlan() {
    if (validation.value) return null
    loading.plan = true
    error.plan = ''
    plan.value = null
    execution.value = null
    try {
      plan.value = await api.generateOrchPlan(wire.value)
      return plan.value
    } catch (e) {
      error.plan = e?.msg || e?.message || '计划生成失败'
      return null
    } finally {
      loading.plan = false
    }
  }

  async function executePlan(stepIds) {
    const problem = planExecuteProblem({ planId: plan.value?.planId })
    if (problem) {
      error.execute = problem
      return null
    }
    loading.execute = true
    error.execute = ''
    execution.value = null
    try {
      // step_ids 传数组＝只跑这些步（Some(集合)），传 undefined＝全跑；空数组是"一步都不跑"，语义不同故不合并
      execution.value = await api.executeOrchPlan({ planId: plan.value.planId, stepIds })
      return execution.value
    } catch (e) {
      error.execute = e?.msg || e?.message || '计划执行失败'
      return null
    } finally {
      loading.execute = false
    }
  }

  async function loadStats() {
    loading.stats = true
    error.stats = ''
    try {
      stats.value = await api.getOrchStats()
      return stats.value
    } catch (e) {
      error.stats = e?.msg || e?.message || '编排统计读取失败'
      return null
    } finally {
      loading.stats = false
    }
  }

  async function loadHistory(page) {
    loading.history = true
    error.history = ''
    try {
      const next = await api.getOrchHistory({
        page: page ?? history.value.page,
        pageSize: history.value.pageSize,
        status: filters.status,
        taskType: filters.taskType
      })
      history.value = next
      return next
    } catch (e) {
      error.history = e?.msg || e?.message || '编排历史读取失败'
      return null
    } finally {
      loading.history = false
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

  return {
    form, filters, orchestration, plan, execution, stats, history, loading, error,
    wire, validation, runnable, executable, hasPlan, steps, outcome,
    disclaimer, volatility, fallbackNote, taskTypeTables, topologyNote, expertNotes,
    simulatedLegend, provenance, statCells, zeroCounterNotes, statusSplit, historyPages, historyAnomaly,
    runOrchestrate, generatePlan, executePlan, loadStats, loadHistory, setHistoryFilter, toggleExpert, reset
  }
})

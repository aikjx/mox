/**
 * 专家联盟 Store
 *
 * 职责：
 * - 保存一次联盟分析的结果快照（专家、观点、共识度、质量门禁）
 * - 提供历史记录与持久化
 *
 * 注意（2026-09-27 归一化）：本 store 的运行面已停用 —— 它原先假设的「整流程流式端点」
 * 在 Rust 侧从来不存在。执行链路一律走 modules/expert-alliance 的六模式协作契约；
 * 阶段/审计口径的权威源是 modules/expert-alliance/contract/phases.js（Rust 投影），
 * 本文件不再自建管线。
 *
 * 与 ai.store 的区别：
 * - ai.store: 通用 AI 对话，单助手/多助手聊天
 * - alliance.store: 仅承载联盟分析的历史与结果快照
 */
import { formatClockMinute } from '@/utils'
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { PHASE_META as CONTRACT_PHASE_META } from '@/modules/expert-alliance/contract'
import { ElMessage } from 'element-plus/es/components/message/index'

// ===== 类型定义 =====

/**
 * 联盟分析阶段
 */
export const AlliancePhase = {
  INTENT: 'intent',
  TEAM: 'team',
  DEBATE: 'debate',
  SYNTHESIZE: 'synthesize',
  GATE: 'gate',
  LEARN: 'learn',
  DONE: 'done',
}

/**
 * 阶段元数据
 *
 * 阶段集合与名称的唯一权威源是 Rust PHASE_NAMES，前端投影为模块契约
 * contract/phases.js；本表只补展示用图标，标签一律取自契约，不再自带一份文案。
 */
const PHASE_ICON = {
  intent: '🎯',
  team: '👥',
  debate: '💬',
  synthesize: '📝',
  gate: '🚦',
  learn: '🧠',
  done: '✅',
}

export const PHASE_META = Object.freeze(Object.fromEntries(
  Object.entries(CONTRACT_PHASE_META).map(([id, meta]) => [id, {
    index: meta.index,
    label: meta.label,
    icon: PHASE_ICON[id] || '📌',
  }])
))

/**
 * 质量等级
 */
export const GateGrade = {
  A: 'A',
  B: 'B',
  C: 'C',
  D: 'D',
}

/**
 * 质量等级元数据
 */
// ADR-SSOT-3：min 必须与后端 mox-unified-contract 的 GATE_THRESHOLDS 一致
// （HC-8 硬约束：A=0.90 / B=0.80 / C=0.70），跨端不得各写一套。
export const GRADE_META = {
  [GateGrade.A]: { label: '优秀', color: '#10b981', min: 0.90, description: '通过，优质交付' },
  [GateGrade.B]: { label: '良好', color: '#06b6d4', min: 0.80, description: '通过，标准交付' },
  [GateGrade.C]: { label: '合格', color: '#f59e0b', min: 0.70, description: '有条件通过，可重试优化' },
  [GateGrade.D]: { label: '不合格', color: '#ef4444', min: 0, description: '阻断，必须修复后重新提交' },
}

// ===== 工具函数 =====

function genId(prefix = 'alliance') {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`
}

// ===== Store 定义 =====

export const useAllianceStore = defineStore('alliance', () => {
  // ===== 状态 =====

  // 运行状态
  const runState = ref('idle') // idle | running | done | error
  const currentPhase = ref(null)
  const phaseProgress = ref({}) // { phase: { current, total, message } }
  const runId = ref(null)
  const traceId = ref(null)
  const startTime = ref(null)
  const endTime = ref(null)

  // 请求参数
  const currentQuery = ref('')
  const teamSize = ref(4)
  const enableLLMDebate = ref(true)
  const sessionId = ref(null)
  const context = ref({})

  // 结果数据
  const intentResult = ref(null)
  const teamResult = ref(null)
  const experts = ref([]) // [{ id, name, dimension, description, color }]
  const opinions = ref([]) // [{ expert_id, dimension, answer, score, confidence, latency_ms }]
  const consensus = ref(0)
  const debateRounds = ref(0)
  const synthesis = ref('')
  const synthesisReasoning = ref('')
  const gateResult = ref(null) // { grade, score, dimensions: {...}, passed }
  const learnResult = ref(null)

  // 事件流
  const events = ref([]) // 所有 SSE 事件
  const messages = ref([]) // 格式化后的消息列表（用于UI展示）

  // 历史记录
  const history = ref([]) // 最近的分析记录
  const maxHistory = ref(20)

  // 配置
  const config = ref({
    apiBase: '/api',
    maxRetries: 3,
    timeoutMs: 120000,
  })

  // ===== 计算属性 =====

  const isRunning = computed(() => runState.value === 'running')
  const isDone = computed(() => runState.value === 'done')
  const isError = computed(() => runState.value === 'error')
  const hasResult = computed(() => !!synthesis.value || opinions.value.length > 0)
  const durationMs = computed(() => {
    if (!startTime.value) return 0
    const end = endTime.value || Date.now()
    return end - startTime.value
  })

  const currentPhaseMeta = computed(() => {
    if (!currentPhase.value) return null
    return PHASE_META[currentPhase.value] || null
  })

  const gateGradeMeta = computed(() => {
    if (!gateResult.value?.grade) return null
    return GRADE_META[gateResult.value.grade] || null
  })

  const sortedOpinions = computed(() => {
    return [...opinions.value].sort((a, b) => b.score - a.score)
  })

  const topOpinions = computed(() => sortedOpinions.value.slice(0, 3))

  // ===== 方法 =====

  /**
   * 重置状态（开始新分析前调用）
   */
  function reset() {
    runState.value = 'idle'
    currentPhase.value = null
    phaseProgress.value = {}
    runId.value = null
    traceId.value = null
    startTime.value = null
    endTime.value = null
    intentResult.value = null
    teamResult.value = null
    experts.value = []
    opinions.value = []
    consensus.value = 0
    debateRounds.value = 0
    synthesis.value = ''
    synthesisReasoning.value = ''
    gateResult.value = null
    learnResult.value = null
    events.value = []
    messages.value = []
  }

  /**
   * 开始联盟分析
   * @param {Object} params - 分析参数
   * @param {string} params.query - 用户查询
   * @param {number} [params.teamSize=4] - 团队规模
   * @param {boolean} [params.enableLLM=true] - 是否启用 LLM 辩论
   * @param {string} [params.sessionId] - 会话 ID
   * @param {Object} [params.context] - 上下文
   */
  async function startAnalysis({ query, teamSize: size = 4, enableLLM = true, sessionId: sid = null, context: ctx = {} }) {
    if (!query?.trim()) {
      ElMessage.warning('请输入分析内容')
      return
    }

    reset()

    currentQuery.value = query.trim()
    teamSize.value = size
    enableLLMDebate.value = enableLLM
    sessionId.value = sid
    context.value = ctx
    runId.value = genId()
    startTime.value = Date.now()
    runState.value = 'running'

    // 添加用户消息
    messages.value.push({
      id: genId('msg'),
      role: 'user',
      name: '我',
      content: currentQuery.value,
      time: formatClockMinute(),
    })

    // 运行面已停用（2026-09-27 归一化）：本 store 原先假设的联盟整流程流式端点在 Rust 侧
    // 从来不存在（模块 contract/endpoints.js 的 FORBIDDEN_ENDPOINTS 记着这条），
    // 真接上只会得到一个 404 与卡在 running 的界面。协作请走 modules/expert-alliance
    // 的六模式契约（/api/experts/*）；任务日志流是 /api/alliance/tasks/:id/logs/stream。
    const reason = '联盟流式执行端点在后端不存在，本 store 的运行面已停用'
    finishAnalysis('error', new Error(reason))
    ElMessage.error(reason)
  }

  /**
   * 完成分析
   */
  function finishAnalysis(state, err = null) {
    runState.value = state
    if (!endTime.value) {
      endTime.value = Date.now()
    }
    currentPhase.value = AlliancePhase.DONE

    if (state === 'done') {
      messages.value.push({
        id: genId('msg'),
        role: 'system',
        name: '分析完成',
        icon: '✅',
        color: '#10b981',
        content: `分析完成，耗时 ${(durationMs.value / 1000).toFixed(1)}秒`,
        time: formatClockMinute(),
      })
      saveToHistory()
    } else if (state === 'error' && err) {
      messages.value.push({
        id: genId('msg'),
        role: 'system',
        name: '分析失败',
        icon: '❌',
        color: '#ef4444',
        content: `分析失败：${err.message}`,
        time: formatClockMinute(),
      })
    }
  }

  /**
   * 停止分析（运行面已停用，仅落终态）
   */
  function stopAnalysis() {
    finishAnalysis('error', new Error('用户手动停止'))
  }

  /**
   * 保存到历史记录
   */
  function saveToHistory() {
    const record = {
      id: runId.value,
      traceId: traceId.value,
      query: currentQuery.value,
      timestamp: startTime.value,
      durationMs: durationMs.value,
      teamSize: teamSize.value,
      consensus: consensus.value,
      gateGrade: gateResult.value?.grade,
      gateScore: gateResult.value?.score,
      synthesis: synthesis.value?.slice(0, 500),
      expertCount: experts.value.length,
    }
    history.value.unshift(record)
    if (history.value.length > maxHistory.value) {
      history.value = history.value.slice(0, maxHistory.value)
    }
    // 持久化到 localStorage
    try {
      localStorage.setItem('mox.alliance.history', JSON.stringify(history.value))
    } catch (e) {
      console.warn('保存联盟历史失败:', e)
    }
  }

  /**
   * 从历史记录加载
   */
  function loadHistory() {
    try {
      const saved = localStorage.getItem('mox.alliance.history')
      if (saved) {
        history.value = JSON.parse(saved)
      }
    } catch (e) {
      console.warn('加载联盟历史失败:', e)
    }
  }

  /**
   * 清空历史记录
   */
  function clearHistory() {
    history.value = []
    try {
      localStorage.removeItem('mox.alliance.history')
    } catch (e) {
      console.warn('清空联盟历史失败:', e)
    }
  }

  /**
   * 更新配置
   */
  function updateConfig(newConfig) {
    config.value = { ...config.value, ...newConfig }
  }

  // ===== 初始化 =====
  loadHistory()

  // ===== 返回 =====
  return {
    // 状态
    runState,
    currentPhase,
    phaseProgress,
    runId,
    traceId,
    startTime,
    endTime,
    currentQuery,
    teamSize,
    enableLLMDebate,
    sessionId,
    context,
    intentResult,
    teamResult,
    experts,
    opinions,
    consensus,
    debateRounds,
    synthesis,
    synthesisReasoning,
    gateResult,
    learnResult,
    events,
    messages,
    history,
    config,
    // 计算属性
    isRunning,
    isDone,
    isError,
    hasResult,
    durationMs,
    currentPhaseMeta,
    gateGradeMeta,
    sortedOpinions,
    topOpinions,
    // 方法
    reset,
    startAnalysis,
    stopAnalysis,
    loadHistory,
    clearHistory,
    updateConfig,
    // 常量
    AlliancePhase,
    PHASE_META,
    GateGrade,
    GRADE_META,
  }
})

export default useAllianceStore

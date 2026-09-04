/**
 * 专家联盟 Store
 *
 * 职责：
 * - 管理联盟分析的完整生命周期（idle → running → phases → done/error）
 * - 维护专家团队、观点、共识度、质量门禁结果
 * - 封装 SSE 连接与事件处理
 * - 提供历史记录与持久化
 *
 * 与 ai.store 的区别：
 * - ai.store: 通用 AI 对话，单助手/多助手聊天
 * - alliance.store: 专家联盟mox 模块化系统架构分析，6阶段管线，多专家辩论
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useSSE, SSEState } from '@/composables/useSSE'
import { ElMessage } from 'element-plus'

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
 */
export const PHASE_META = {
  [AlliancePhase.INTENT]: { label: '意图识别', icon: '🎯', color: '#6366f1' },
  [AlliancePhase.TEAM]: { label: '组队匹配', icon: '👥', color: '#06b6d4' },
  [AlliancePhase.DEBATE]: { label: '专家辩论', icon: '💬', color: '#f59e0b' },
  [AlliancePhase.SYNTHESIZE]: { label: '综合归纳', icon: '📝', color: '#10b981' },
  [AlliancePhase.GATE]: { label: '质量门禁', icon: '🚦', color: '#ef4444' },
  [AlliancePhase.LEARN]: { label: '知识学习', icon: '🧠', color: '#8b5cf6' },
  [AlliancePhase.DONE]: { label: '完成', icon: '✅', color: '#10b981' },
}

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
export const GRADE_META = {
  [GateGrade.A]: { label: '优秀', color: '#10b981', min: 0.85, description: '通过，优质交付' },
  [GateGrade.B]: { label: '良好', color: '#06b6d4', min: 0.70, description: '通过，标准交付' },
  [GateGrade.C]: { label: '合格', color: '#f59e0b', min: 0.50, description: '有条件通过，可重试优化' },
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
    streamEndpoint: '/alliance/stream',
    maxRetries: 3,
    timeoutMs: 120000,
  })

  // SSE 控制器（延迟初始化）
  let sseController = null

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
    if (sseController) {
      sseController.disconnect()
      sseController = null
    }
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
      time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
    })

    // 构建 SSE 请求
    const url = `${config.value.apiBase}${config.value.streamEndpoint}`
    const body = {
      query: currentQuery.value,
      team_size: teamSize.value,
      enable_llm_debate: enableLLMDebate.value,
      session_id: sessionId.value,
      context: {
        project_id: ctx.project_id || null,
        mode: ctx.mode || 'alliance',
        ...ctx,
      },
    }

    sseController = useSSE({
      url,
      method: 'POST',
      body,
      maxRetries: config.value.maxRetries,
      timeoutMs: config.value.timeoutMs,
      onEvent: handleSSEEvent,
      onError: handleSSEError,
      onOpen: () => {
        console.log('[alliance] SSE connected')
      },
      onClose: () => {
        if (runState.value === 'running') {
          // 非正常关闭
          finishAnalysis('error', new Error('SSE connection closed unexpectedly'))
        }
      },
    })

    try {
      await sseController.connect()
    } catch (e) {
      finishAnalysis('error', e)
    }
  }

  /**
   * 处理 SSE 事件
   */
  function handleSSEEvent(event) {
    events.value.push(event)

    const { event: eventType, payload } = event
    if (!payload) return

    // 更新 trace_id
    if (payload.trace_id && !traceId.value) {
      traceId.value = payload.trace_id
    }

    switch (eventType) {
      case 'phase_started':
        handlePhaseStarted(payload)
        break
      case 'phase_data':
        handlePhaseData(payload)
        break
      case 'progress':
        handleProgress(payload)
        break
      case 'complete':
        handleComplete(payload)
        break
      case 'error':
        handleErrorEvent(payload)
        break
      default:
        // 未知事件类型，记录但不处理
        console.debug('[alliance] Unknown event:', eventType, payload)
    }
  }

  /**
   * 处理阶段开始
   */
  function handlePhaseStarted(payload) {
    const phase = payload.phase
    currentPhase.value = phase
    phaseProgress.value[phase] = { current: 0, total: 0, message: '处理中...' }

    const meta = PHASE_META[phase]
    if (meta) {
      messages.value.push({
        id: genId('msg'),
        role: 'system',
        name: meta.label,
        icon: meta.icon,
        color: meta.color,
        phase,
        content: `开始${meta.label}...`,
        time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
      })
    }
  }

  /**
   * 处理阶段数据
   */
  function handlePhaseData(payload) {
    const phase = payload.phase
    const data = payload.payload || payload

    switch (phase) {
      case AlliancePhase.INTENT:
        intentResult.value = data
        addPhaseMessage(phase, `意图分类：${data.intent || 'unknown'}（置信度 ${(data.confidence || 0).toFixed(2)}）`)
        break

      case AlliancePhase.TEAM:
        teamResult.value = data
        experts.value = data.experts || data.team || []
        addPhaseMessage(phase, `已匹配 ${experts.value.length} 位专家：${experts.value.map(e => e.name || e.id || e).join('、')}`)
        break

      case AlliancePhase.DEBATE:
        // 辩论阶段可能有多个专家观点
        if (data.opinions) {
          opinions.value = data.opinions
        } else if (data.expert_id || data.expert_name) {
          // 单个专家观点
          const existing = opinions.value.findIndex(o =>
            o.expert_id === data.expert_id || o.dimension === data.expert_type
          )
          const opinion = {
            expert_id: data.expert_id || data.expert_type || 'unknown',
            dimension: data.expert_type || data.expert_id || 'unknown',
            answer: data.content || data.argument || data.answer || '',
            score: data.score ?? 0.5,
            confidence: data.confidence ?? 0.5,
            latency_ms: data.latency_ms || 0,
          }
          if (existing >= 0) {
            opinions.value[existing] = opinion
          } else {
            opinions.value.push(opinion)
          }
        }
        if (data.consensus != null) {
          consensus.value = data.consensus
        }
        if (data.debate_rounds != null) {
          debateRounds.value = data.debate_rounds
        }
        break

      case AlliancePhase.SYNTHESIZE:
        synthesis.value = data.synthesis || data.summary || ''
        synthesisReasoning.value = data.reasoning || data.synthesis_reasoning || ''
        addPhaseMessage(phase, '综合归纳完成')
        break

      case AlliancePhase.GATE:
        gateResult.value = {
          grade: data.grade || data.gate_grade,
          score: data.score || data.gate_score,
          passed: data.passed ?? (data.grade !== 'D'),
          dimensions: data.dimensions || {},
        }
        const gradeMeta = GRADE_META[gateResult.value.grade]
        addPhaseMessage(phase, `质量门禁：${gradeMeta?.label || gateResult.value.grade}级（${(gateResult.value.score || 0).toFixed(2)}分）${gateResult.value.passed ? '，通过' : '，阻断'}`)
        break

      case AlliancePhase.LEARN:
        learnResult.value = data
        addPhaseMessage(phase, '知识学习完成')
        break
    }
  }

  /**
   * 处理进度更新
   */
  function handleProgress(payload) {
    const phase = payload.phase
    if (phase && phaseProgress.value[phase]) {
      phaseProgress.value[phase] = {
        current: payload.current || 0,
        total: payload.total || 0,
        message: payload.message || '',
      }
    }
  }

  /**
   * 处理完成事件
   */
  function handleComplete(payload) {
    if (payload.final_answer || payload.result) {
      synthesis.value = payload.final_answer || payload.result || synthesis.value
    }
    if (payload.gate_passed != null && gateResult.value) {
      gateResult.value.passed = payload.gate_passed
    }
    if (payload.total_ms) {
      endTime.value = startTime.value + payload.total_ms
    }
    finishAnalysis('done')
  }

  /**
   * 处理错误事件
   */
  function handleErrorEvent(payload) {
    const error = new Error(payload.message || payload.error || '未知错误')
    addPhaseMessage('error', `错误：${error.message}`)
    finishAnalysis('error', error)
  }

  /**
   * 处理 SSE 错误
   */
  function handleSSEError(err) {
    console.error('[alliance] SSE error:', err)
    ElMessage.error(`联盟分析失败：${err.message}`)
    finishAnalysis('error', err)
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
        time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
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
        time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
      })
    }

    if (sseController) {
      sseController.disconnect()
      sseController = null
    }
  }

  /**
   * 添加阶段消息
   */
  function addPhaseMessage(phase, content) {
    const meta = PHASE_META[phase] || { label: phase, icon: '📌', color: '#6b7280' }
    messages.value.push({
      id: genId('msg'),
      role: 'assistant',
      name: meta.label,
      icon: meta.icon,
      color: meta.color,
      phase,
      content,
      time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
    })
  }

  /**
   * 停止分析
   */
  function stopAnalysis() {
    if (sseController) {
      sseController.disconnect()
      sseController = null
    }
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

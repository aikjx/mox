/**
 * 工作流执行引擎
 * 功能：基于状态机执行 Agent 工作流，支持顺序/并行/DAG 执行
 *
 * 核心能力：
 * 1. DSL 解析与初始化
 * 2. Agent 状态管理
 * 3. 顺序/并行/DAG 执行
 * 4. 错误处理与重试
 * 5. 进度追踪与事件通知
 */

import { ref, computed, reactive } from 'vue'

// ============================================================
// Agent 状态枚举
// ============================================================
const AGENT_STATUS = {
  WAITING: 'waiting',     // 等待执行
  PENDING: 'pending',     // 即将执行
  RUNNING: 'running',     // 执行中
  DONE: 'done',           // 执行完成
  ERROR: 'error',         // 执行出错
  SKIPPED: 'skipped',     // 被跳过
  PAUSED: 'paused'        // 暂停
}

// ============================================================
// 工作流状态枚举
// ============================================================
const FLOW_STATUS = {
  IDLE: 'idle',           // 空闲
  RUNNING: 'running',     // 运行中
  PAUSED: 'paused',       // 已暂停
  DONE: 'done',           // 已完成
  ERROR: 'error'          // 出错
}

// ============================================================
// Agent 执行器（模拟，实际应调用后端 API）
// ============================================================

/**
 * 执行单个 Agent（模拟实现）
 * 实际项目中应替换为真实的 API 调用
 * @param {object} agent Agent 配置
 * @param {object} context 执行上下文
 * @param {function} onProgress 进度回调
 * @returns {Promise<object>} 执行结果
 */
async function executeAgent(agent, context = {}, onProgress = null) {
  // 模拟执行时间（根据 Agent 角色估算）
  const baseTime = 1000 + Math.random() * 2000
  const steps = 10

  for (let i = 1; i <= steps; i++) {
    await new Promise(resolve => setTimeout(resolve, baseTime / steps))
    if (onProgress) {
      onProgress(i * 10, `正在${agent.name}... ${i * 10}%`)
    }
  }

  // 模拟 10% 的出错概率（用于演示错误处理）
  if (Math.random() < 0.1) {
    throw new Error(`${agent.name}执行失败：模拟错误`)
  }

  // 返回模拟结果
  return {
    agentId: agent.id,
    agentName: agent.name,
    status: 'success',
    output: {
      summary: `${agent.name}完成`,
      artifacts: generateMockArtifacts(agent),
      metrics: {
        duration_ms: baseTime,
        tokens_used: Math.floor(Math.random() * 5000) + 1000
      }
    },
    timestamp: new Date().toISOString()
  }
}

/**
 * 生成模拟产物
 * @param {object} agent Agent 配置
 * @returns {Array} 产物列表
 */
function generateMockArtifacts(agent) {
  const artifactTypes = {
    '数据采集': ['raw_data.json', 'source_log.txt'],
    '数据清洗': ['cleaned_data.csv', 'quality_report.md'],
    '实体关系抽取': ['entities.json', 'relations.csv'],
    '本体建模': ['schema.owl', 'model_diagram.png'],
    '知识融合': ['merged_entities.json', 'fusion_report.md'],
    '质量评估': ['quality_report.pdf', 'validation_results.json'],
    '图谱入库': ['graph_stats.json', 'import_log.txt'],
    '可视化呈现': ['visualization.html', 'charts.png'],
    '需求分析': ['requirements.md', 'use_cases.png'],
    '架构设计': ['architecture.md', 'api_spec.yaml'],
    '代码实现': ['source_code.zip', 'unit_tests.js'],
    '测试验证': ['test_report.md', 'coverage_report.html'],
    '文档生成': ['api_docs.md', 'README.md']
  }

  return artifactTypes[agent.name] || ['result.json']
}

// ============================================================
// 组合式函数：工作流执行引擎
// ============================================================

/**
 * 工作流执行引擎
 */
export function useFlowEngine() {
  // 工作流状态
  const flowStatus = ref(FLOW_STATUS.IDLE)
  const dsl = ref(null)
  const agents = ref([])
  const currentAgentIndex = ref(-1)
  const startTime = ref(null)
  const endTime = ref(null)
  const error = ref(null)

  // 执行结果
  const results = reactive({})
  const allArtifacts = ref([])

  // 计算属性
  const progress = computed(() => {
    if (agents.value.length === 0) return 0
    const doneCount = agents.value.filter(a => a.status === AGENT_STATUS.DONE).length
    return Math.round((doneCount / agents.value.length) * 100)
  })

  const isRunning = computed(() => flowStatus.value === FLOW_STATUS.RUNNING)
  const isPaused = computed(() => flowStatus.value === FLOW_STATUS.PAUSED)
  const isDone = computed(() => flowStatus.value === FLOW_STATUS.DONE)
  const hasError = computed(() => flowStatus.value === FLOW_STATUS.ERROR)

  const duration = computed(() => {
    if (!startTime.value) return 0
    const end = endTime.value || Date.now()
    return end - startTime.value
  })

  const currentAgent = computed(() => {
    if (currentAgentIndex.value < 0 || currentAgentIndex.value >= agents.value.length) {
      return null
    }
    return agents.value[currentAgentIndex.value]
  })

  // ============================================================
  // 初始化
  // ============================================================

  /**
   * 从 DSL 初始化工作流
   * @param {object} dslConfig DSL 配置
   */
  function initFromDSL(dslConfig) {
    dsl.value = dslConfig
    agents.value = dslConfig.agents.map((agentConfig, idx) => ({
      id: agentConfig.id,
      name: agentConfig.name,
      role: agentConfig.role,
      description: agentConfig.description,
      status: idx === 0 ? AGENT_STATUS.PENDING : AGENT_STATUS.WAITING,
      result: null,
      error: null,
      retryCount: 0,
      progress: 0,
      startTime: null,
      endTime: null,
      config: agentConfig
    }))
    currentAgentIndex.value = 0
    flowStatus.value = FLOW_STATUS.IDLE
    startTime.value = null
    endTime.value = null
    error.value = null
    allArtifacts.value = []
  }

  // ============================================================
  // 执行控制
  // ============================================================

  /**
   * 开始执行工作流
   */
  async function start() {
    if (agents.value.length === 0) {
      throw new Error('工作流未初始化，请先调用 initFromDSL')
    }
    if (flowStatus.value === FLOW_STATUS.RUNNING) {
      return
    }

    flowStatus.value = FLOW_STATUS.RUNNING
    startTime.value = Date.now()
    error.value = null

    await runNextAgent()
  }

  /**
   * 暂停执行
   */
  function pause() {
    if (flowStatus.value === FLOW_STATUS.RUNNING) {
      flowStatus.value = FLOW_STATUS.PAUSED
    }
  }

  /**
   * 继续执行
   */
  async function resume() {
    if (flowStatus.value === FLOW_STATUS.PAUSED) {
      flowStatus.value = FLOW_STATUS.RUNNING
      await runNextAgent()
    }
  }

  /**
   * 停止执行
   */
  function stop() {
    flowStatus.value = FLOW_STATUS.IDLE
    currentAgentIndex.value = -1
  }

  /**
   * 重试当前出错的 Agent
   */
  async function retry() {
    if (flowStatus.value !== FLOW_STATUS.ERROR) return

    const agent = agents.value[currentAgentIndex.value]
    if (!agent) return

    agent.status = AGENT_STATUS.PENDING
    agent.error = null
    agent.retryCount++
    flowStatus.value = FLOW_STATUS.RUNNING

    await runAgent(agent)
  }

  // ============================================================
  // 核心执行逻辑
  // ============================================================

  /**
   * 执行下一个 Agent
   */
  async function runNextAgent() {
    if (flowStatus.value !== FLOW_STATUS.RUNNING) return

    // 检查是否全部完成
    if (currentAgentIndex.value >= agents.value.length) {
      flowStatus.value = FLOW_STATUS.DONE
      endTime.value = Date.now()
      await onFlowComplete()
      return
    }

    const agent = agents.value[currentAgentIndex.value]

    // 如果当前 Agent 已完成，继续下一个
    if (agent.status === AGENT_STATUS.DONE) {
      currentAgentIndex.value++
      if (currentAgentIndex.value < agents.value.length) {
        agents.value[currentAgentIndex.value].status = AGENT_STATUS.PENDING
      }
      setTimeout(() => runNextAgent(), 300)
      return
    }

    // 执行当前 Agent
    await runAgent(agent)
  }

  /**
   * 执行单个 Agent
   * @param {object} agent Agent 对象
   */
  async function runAgent(agent) {
    if (flowStatus.value !== FLOW_STATUS.RUNNING) return

    agent.status = AGENT_STATUS.RUNNING
    agent.startTime = Date.now()
    agent.progress = 0

    try {
      // 收集上游输入
      const inputs = collectInputs(agent)

      // 执行 Agent
      const result = await executeAgent(
        agent.config,
        { inputs, agentIndex: currentAgentIndex.value },
        (progress, message) => {
          agent.progress = progress
        }
      )

      // 保存结果
      agent.status = AGENT_STATUS.DONE
      agent.result = result
      agent.endTime = Date.now()
      results[agent.id] = result

      // 收集产物
      if (result.output?.artifacts) {
        allArtifacts.value.push(...result.output.artifacts.map(a => ({
          name: a,
          agentId: agent.id,
          agentName: agent.name,
          timestamp: new Date().toISOString()
        })))
      }

      // 继续下一个
      currentAgentIndex.value++
      if (currentAgentIndex.value < agents.value.length) {
        agents.value[currentAgentIndex.value].status = AGENT_STATUS.PENDING
      }

      // 短暂延迟后继续
      setTimeout(() => runNextAgent(), 500)

    } catch (err) {
      console.error(`Agent ${agent.name} 执行失败:`, err)
      agent.status = AGENT_STATUS.ERROR
      agent.error = err.message
      agent.endTime = Date.now()

      // 检查是否可以重试
      const maxRetries = agent.config.retry?.max_attempts || 1
      if (agent.retryCount < maxRetries) {
        agent.retryCount++
        console.log(`Agent ${agent.name} 重试中 (${agent.retryCount}/${maxRetries})`)
        setTimeout(() => {
          agent.status = AGENT_STATUS.PENDING
          agent.error = null
          runAgent(agent)
        }, 1000 * agent.retryCount)
      } else {
        // 重试耗尽，工作流出错
        flowStatus.value = FLOW_STATUS.ERROR
        error.value = err.message
        endTime.value = Date.now()
        onFlowError(err)
      }
    }
  }

  /**
   * 收集上游 Agent 的输出作为输入
   * @param {object} agent 当前 Agent
   * @returns {object} 输入数据
   */
  function collectInputs(agent) {
    const inputs = {}
    const agentIdx = agents.value.findIndex(a => a.id === agent.id)

    // 收集所有前置 Agent 的结果
    for (let i = 0; i < agentIdx; i++) {
      const prevAgent = agents.value[i]
      if (prevAgent.result) {
        inputs[prevAgent.id] = prevAgent.result
      }
    }

    return inputs
  }

  // ============================================================
  // 事件回调（可扩展）
  // ============================================================

  /**
   * 工作流完成回调
   */
  async function onFlowComplete() {
    console.log('工作流执行完成！')
    console.log('总产物数量:', allArtifacts.value.length)
    // 这里可以触发归档、通知等后续操作
  }

  /**
   * 工作流出错回调
   * @param {Error} err 错误对象
   */
  function onFlowError(err) {
    console.error('工作流执行出错:', err)
    // 这里可以触发错误通知、人工介入等
  }

  // ============================================================
  // 工具方法
  // ============================================================

  /**
   * 获取 Agent 状态颜色
   * @param {string} status 状态
   * @returns {string} 颜色值
   */
  function getStatusColor(status) {
    const colors = {
      [AGENT_STATUS.WAITING]: '#9ca3af',
      [AGENT_STATUS.PENDING]: '#f59e0b',
      [AGENT_STATUS.RUNNING]: '#3b82f6',
      [AGENT_STATUS.DONE]: '#10b981',
      [AGENT_STATUS.ERROR]: '#ef4444',
      [AGENT_STATUS.SKIPPED]: '#6b7280',
      [AGENT_STATUS.PAUSED]: '#f59e0b'
    }
    return colors[status] || '#9ca3af'
  }

  /**
   * 获取状态文本
   * @param {string} status 状态
   * @returns {string} 状态文本
   */
  function getStatusText(status) {
    const texts = {
      [AGENT_STATUS.WAITING]: '等待中',
      [AGENT_STATUS.PENDING]: '待执行',
      [AGENT_STATUS.RUNNING]: '执行中',
      [AGENT_STATUS.DONE]: '已完成',
      [AGENT_STATUS.ERROR]: '出错',
      [AGENT_STATUS.SKIPPED]: '已跳过',
      [AGENT_STATUS.PAUSED]: '已暂停'
    }
    return texts[status] || status
  }

  /**
   * 重置引擎状态
   */
  function reset() {
    flowStatus.value = FLOW_STATUS.IDLE
    dsl.value = null
    agents.value = []
    currentAgentIndex.value = -1
    startTime.value = null
    endTime.value = null
    error.value = null
    allArtifacts.value = []
    Object.keys(results).forEach(key => delete results[key])
  }

  return {
    // 状态
    flowStatus,
    dsl,
    agents,
    currentAgentIndex,
    startTime,
    endTime,
    error,
    results,
    allArtifacts,
    // 计算属性
    progress,
    isRunning,
    isPaused,
    isDone,
    hasError,
    duration,
    currentAgent,
    // 方法
    initFromDSL,
    start,
    pause,
    resume,
    stop,
    retry,
    getStatusColor,
    getStatusText,
    reset,
    // 常量
    AGENT_STATUS,
    FLOW_STATUS
  }
}

export default useFlowEngine

/**
 * 对话驱动项目生成器 - 主入口
 * 功能：整合意图解析、项目匹配、动态编排、工作流执行、项目归档
 *
 * 使用方式：
 * import { useChatProjectGenerator } from './src/index'
 * const generator = useChatProjectGenerator()
 * generator.run('帮我构建一个医疗知识图谱')
 */

import { ref, computed } from 'vue'
import { useProjectAwareDialogue } from './projectAwareDialogue'
import { useAutoOrchestrator } from './autoOrchestrator'
import { useFlowEngine } from './flowEngine'
import { useProjectArchiver } from './projectArchiver'

// ============================================================
// 生成器阶段枚举
// ============================================================
const GENERATOR_STAGES = {
  IDLE: 'idle',                 // 空闲
  INTENT_PARSING: 'intent',     // 意图解析中
  PROJECT_MATCHING: 'matching', // 项目匹配中
  ORCHESTRATING: 'orchestrating', // 编排中
  EXECUTING: 'executing',       // 执行中
  ARCHIVING: 'archiving',       // 归档中
  DONE: 'done',                 // 完成
  ERROR: 'error'                // 出错
}

// ============================================================
// 组合式函数：对话驱动项目生成器
// ============================================================

/**
 * 对话驱动项目生成器
 * 一站式：对话 → 项目匹配 → 编排 → 执行 → 归档
 */
export function useChatProjectGenerator() {
  // 子模块
  const dialogue = useProjectAwareDialogue()
  const orchestrator = useAutoOrchestrator()
  const flowEngine = useFlowEngine()
  const archiver = useProjectArchiver()

  // 状态
  const currentStage = ref(GENERATOR_STAGES.IDLE)
  const overallProgress = ref(0)
  const stageHistory = ref([])
  const lastResult = ref(null)
  const error = ref(null)

  // 计算属性
  const isRunning = computed(() => {
    return currentStage.value !== GENERATOR_STAGES.IDLE &&
           currentStage.value !== GENERATOR_STAGES.DONE &&
           currentStage.value !== GENERATOR_STAGES.ERROR
  })

  const isDone = computed(() => currentStage.value === GENERATOR_STAGES.DONE)
  const hasError = computed(() => currentStage.value === GENERATOR_STAGES.ERROR)

  const currentProject = computed(() => dialogue.matchedProject.value)
  const currentIntent = computed(() => dialogue.currentIntent.value)
  const currentWorkflow = computed(() => orchestrator.currentWorkflow.value)

  // 阶段进度映射（每个阶段占总进度的百分比）
  const stageProgressWeights = {
    [GENERATOR_STAGES.INTENT_PARSING]: 10,
    [GENERATOR_STAGES.PROJECT_MATCHING]: 15,
    [GENERATOR_STAGES.ORCHESTRATING]: 20,
    [GENERATOR_STAGES.EXECUTING]: 45,
    [GENERATOR_STAGES.ARCHIVING]: 10
  }

  // ============================================================
  // 阶段管理
  // ============================================================

  /**
   * 切换阶段
   * @param {string} stage 新阶段
   * @param {object} data 阶段数据
   */
  function enterStage(stage, data = {}) {
    currentStage.value = stage
    stageHistory.value.push({
      stage,
      timestamp: new Date().toISOString(),
      data
    })

    // 计算累计进度
    let progress = 0
    const stageOrder = [
      GENERATOR_STAGES.INTENT_PARSING,
      GENERATOR_STAGES.PROJECT_MATCHING,
      GENERATOR_STAGES.ORCHESTRATING,
      GENERATOR_STAGES.EXECUTING,
      GENERATOR_STAGES.ARCHIVING
    ]

    const currentIdx = stageOrder.indexOf(stage)
    for (let i = 0; i < currentIdx; i++) {
      progress += stageProgressWeights[stageOrder[i]] || 0
    }

    overallProgress.value = progress
  }

  /**
   * 更新当前阶段进度
   * @param {number} percent 阶段内进度 0-100
   */
  function updateStageProgress(percent) {
    const weight = stageProgressWeights[currentStage.value] || 0
    const baseProgress = overallProgress.value - (overallProgress.value % weight)
    overallProgress.value = Math.min(100, baseProgress + (weight * percent / 100))
  }

  // ============================================================
  // 主流程
  // ============================================================

  /**
   * 运行完整生成流程
   * @param {string} message 用户输入消息
   * @param {object} options 配置项
   * @returns {Promise<object>} 最终结果
   */
  async function run(message, options = {}) {
    const {
      autoCreate = true,
      autoExecute = true,
      autoArchive = true,
      matchThreshold = 0.75,
      complexity = 'auto'  // auto | simple | medium | complex
    } = options

    error.value = null
    lastResult.value = null
    stageHistory.value = []
    overallProgress.value = 0

    try {
      // ============================================================
      // Phase 1: 意图解析 + 项目匹配
      // ============================================================
      enterStage(GENERATOR_STAGES.INTENT_PARSING, { message })
      updateStageProgress(50)

      const { intent, project, isNew } = await dialogue.processMessage(message, {
        autoCreate,
        matchThreshold
      })

      updateStageProgress(100)

      if (!project && intent.intentType !== 'general_chat') {
        throw new Error('未能匹配或创建项目')
      }

      // ============================================================
      // Phase 2: 项目匹配完成（进入编排阶段）
      // ============================================================
      enterStage(GENERATOR_STAGES.PROJECT_MATCHING, { project, isNew })
      updateStageProgress(100)

      // 普通闲聊直接返回
      if (!project) {
        enterStage(GENERATOR_STAGES.DONE)
        overallProgress.value = 100
        return {
          type: 'chat',
          intent,
          message: '这是一个普通对话，无需生成项目内容'
        }
      }

      // ============================================================
      // Phase 3: 动态编排
      // ============================================================
      enterStage(GENERATOR_STAGES.ORCHESTRATING, { intentType: intent.intentType })
      updateStageProgress(30)

      // 确定复杂度
      const actualComplexity = complexity === 'auto'
        ? (intent.complexity || 'medium')
        : complexity

      updateStageProgress(60)

      const { workflow, dsl } = await orchestrator.orchestrate({
        intentType: intent.intentType,
        complexity: actualComplexity,
        project,
        intent
      })

      updateStageProgress(100)

      // 如果不自动执行，到这里就返回
      if (!autoExecute) {
        enterStage(GENERATOR_STAGES.DONE)
        overallProgress.value = 100
        return {
          type: 'orchestrated',
          project,
          intent,
          workflow,
          dsl,
          message: `编排完成！共 ${workflow.agents.length} 个 Agent，预计耗时 ${orchestrator.estimateTotalTime(workflow)}`
        }
      }

      // ============================================================
      // Phase 4: 工作流执行
      // ============================================================
      enterStage(GENERATOR_STAGES.EXECUTING, { agentCount: workflow.agents.length })

      // 初始化执行引擎
      flowEngine.initFromDSL(dsl)

      // 监听执行进度
      const progressWatcher = setInterval(() => {
        updateStageProgress(flowEngine.progress.value)
      }, 200)

      // 启动执行
      await flowEngine.start()

      clearInterval(progressWatcher)
      updateStageProgress(100)

      if (flowEngine.hasError.value) {
        throw new Error(`工作流执行失败: ${flowEngine.error.value}`)
      }

      // ============================================================
      // Phase 5: 项目归档
      // ============================================================
      if (autoArchive) {
        enterStage(GENERATOR_STAGES.ARCHIVING, { artifactCount: flowEngine.allArtifacts.value.length })
        updateStageProgress(30)

        const archiveResult = await archiver.archive({
          project,
          artifacts: flowEngine.allArtifacts.value,
          executionResult: {
            status: 'success',
            duration_ms: flowEngine.duration.value,
            agent_results: Object.keys(flowEngine.results).length
          },
          description: `自动执行生成 - ${intent.summary || intent.intentType}`
        })

        updateStageProgress(100)
      }

      // ============================================================
      // 完成
      // ============================================================
      enterStage(GENERATOR_STAGES.DONE)
      overallProgress.value = 100

      const result = {
        type: 'complete',
        project,
        intent,
        workflow: orchestrator.currentWorkflow.value,
        artifacts: flowEngine.allArtifacts.value,
        snapshot: archiver.currentSnapshot.value,
        duration_ms: flowEngine.duration.value,
        message: `任务完成！已生成 ${flowEngine.allArtifacts.value.length} 个产物，归档到项目「${project.name}」`
      }

      lastResult.value = result
      return result

    } catch (err) {
      console.error('生成流程失败:', err)
      error.value = err.message
      currentStage.value = GENERATOR_STAGES.ERROR
      throw err
    }
  }

  /**
   * 从编排阶段开始执行（已有编排结果时）
   * @param {object} dsl DSL 配置
   * @param {object} project 项目信息
   * @returns {Promise<object>} 执行结果
   */
  async function executeExisting(dsl, project) {
    try {
      enterStage(GENERATOR_STAGES.EXECUTING)

      flowEngine.initFromDSL(dsl)
      await flowEngine.start()

      if (flowEngine.hasError.value) {
        throw new Error(flowEngine.error.value)
      }

      // 归档
      enterStage(GENERATOR_STAGES.ARCHIVING)
      await archiver.archive({
        project,
        artifacts: flowEngine.allArtifacts.value,
        description: '手动执行生成'
      })

      enterStage(GENERATOR_STAGES.DONE)
      overallProgress.value = 100

      return {
        artifacts: flowEngine.allArtifacts.value,
        snapshot: archiver.currentSnapshot.value
      }

    } catch (err) {
      error.value = err.message
      currentStage.value = GENERATOR_STAGES.ERROR
      throw err
    }
  }

  /**
   * 暂停当前执行
   */
  function pause() {
    if (currentStage.value === GENERATOR_STAGES.EXECUTING) {
      flowEngine.pause()
    }
  }

  /**
   * 继续执行
   */
  function resume() {
    if (currentStage.value === GENERATOR_STAGES.EXECUTING) {
      flowEngine.resume()
    }
  }

  /**
   * 重试出错的步骤
   */
  async function retry() {
    if (currentStage.value === GENERATOR_STAGES.ERROR) {
      if (flowEngine.hasError.value) {
        currentStage.value = GENERATOR_STAGES.EXECUTING
        await flowEngine.retry()
      }
    }
  }

  /**
   * 重置生成器状态
   */
  function reset() {
    currentStage.value = GENERATOR_STAGES.IDLE
    overallProgress.value = 0
    stageHistory.value = []
    lastResult.value = null
    error.value = null
    dialogue.resetContext()
    orchestrator.reset()
    flowEngine.reset()
    archiver.reset()
  }

  // ============================================================
  // 快捷方法
  // ============================================================

  /**
   * 获取阶段信息
   * @param {string} stage 阶段
   * @returns {object} 阶段信息
   */
  function getStageInfo(stage) {
    const stageInfo = {
      [GENERATOR_STAGES.IDLE]: { name: '待命', icon: '⏸️', color: '#9ca3af' },
      [GENERATOR_STAGES.INTENT_PARSING]: { name: '解析意图', icon: '🧠', color: '#6366f1' },
      [GENERATOR_STAGES.PROJECT_MATCHING]: { name: '匹配项目', icon: '🔍', color: '#8b5cf6' },
      [GENERATOR_STAGES.ORCHESTRATING]: { name: '编排工作流', icon: '⚙️', color: '#f59e0b' },
      [GENERATOR_STAGES.EXECUTING]: { name: '执行中', icon: '🚀', color: '#3b82f6' },
      [GENERATOR_STAGES.ARCHIVING]: { name: '归档中', icon: '📦', color: '#10b981' },
      [GENERATOR_STAGES.DONE]: { name: '完成', icon: '✅', color: '#10b981' },
      [GENERATOR_STAGES.ERROR]: { name: '出错', icon: '❌', color: '#ef4444' }
    }
    return stageInfo[stage] || stageInfo[GENERATOR_STAGES.IDLE]
  }

  return {
    // 状态
    currentStage,
    overallProgress,
    stageHistory,
    lastResult,
    error,
    // 计算属性
    isRunning,
    isDone,
    hasError,
    currentProject,
    currentIntent,
    currentWorkflow,
    // 子模块
    dialogue,
    orchestrator,
    flowEngine,
    archiver,
    // 主方法
    run,
    executeExisting,
    pause,
    resume,
    retry,
    reset,
    // 工具方法
    getStageInfo,
    // 常量
    GENERATOR_STAGES
  }
}

export default useChatProjectGenerator

/**
 * 对话驱动项目生成器 - 使用示例
 *
 * 本文件演示如何使用对话驱动项目生成器的各个模块
 */

import { useChatProjectGenerator } from './src/index'
import { useProjectAwareDialogue } from './src/projectAwareDialogue'
import { useAutoOrchestrator } from './src/autoOrchestrator'
import { useFlowEngine } from './src/flowEngine'
import { useProjectArchiver } from './src/projectArchiver'

// ============================================================
// 示例 1: 完整流程（推荐）
// ============================================================

async function exampleFullFlow() {
  console.log('=== 示例 1: 完整流程 ===')

  const generator = useChatProjectGenerator()

  try {
    // 运行完整流程
    const result = await generator.run('帮我构建一个医疗知识图谱', {
      autoCreate: true,       // 未匹配到项目时自动创建
      autoExecute: true,      // 编排完成后自动执行
      autoArchive: true,      // 执行完成后自动归档
      matchThreshold: 0.75,   // 项目匹配阈值
      complexity: 'auto'      // 复杂度自动判断
    })

    console.log('生成完成:', result.message)
    console.log('项目:', result.project?.name)
    console.log('产物数量:', result.artifacts?.length)
    console.log('版本:', result.snapshot?.version)

  } catch (error) {
    console.error('生成失败:', error.message)
  }
}

// ============================================================
// 示例 2: 分步执行（精细控制）
// ============================================================

async function exampleStepByStep() {
  console.log('=== 示例 2: 分步执行 ===')

  // 1. 意图解析 + 项目匹配
  const dialogue = useProjectAwareDialogue()
  const { intent, project, isNew } = await dialogue.processMessage(
    '帮我分析销售数据，生成可视化报表'
  )

  console.log('意图类型:', intent.intentType)
  console.log('匹配项目:', project?.name, isNew ? '(新建)' : '(已有)')

  // 2. 动态编排
  const orchestrator = useAutoOrchestrator()
  const { workflow, dsl } = await orchestrator.orchestrate({
    intentType: intent.intentType,
    complexity: intent.complexity || 'medium',
    project,
    intent
  })

  console.log('编排完成:', workflow.name)
  console.log('Agent 数量:', workflow.agents.length)
  console.log('预计耗时:', orchestrator.estimateTotalTime(workflow))

  // 3. 工作流执行
  const flowEngine = useFlowEngine()
  flowEngine.initFromDSL(dsl)

  // 可以监听进度
  // flowEngine.agents 可以用来渲染 UI
  // flowEngine.progress 可以用来显示进度条

  await flowEngine.start()

  if (flowEngine.isDone.value) {
    console.log('执行完成!')
    console.log('产物:', flowEngine.allArtifacts.value)
  }

  // 4. 项目归档
  const archiver = useProjectArchiver()
  const archiveResult = await archiver.archive({
    project,
    artifacts: flowEngine.allArtifacts.value,
    description: '手动分步执行'
  })

  console.log('归档完成:', archiveResult.snapshot.version)
}

// ============================================================
// 示例 3: 仅使用意图解析
// ============================================================

async function exampleIntentOnly() {
  console.log('=== 示例 3: 仅意图解析 ===')

  const dialogue = useProjectAwareDialogue()
  const { intent, project } = await dialogue.processMessage('你好，今天天气怎么样？')

  console.log('意图:', intent.intentType)
  console.log('置信度:', intent.confidence)
  console.log('是否绑定项目:', !!project)
}

// ============================================================
// 示例 4: 仅使用编排器
// ============================================================

function exampleOrchestratorOnly() {
  console.log('=== 示例 4: 仅使用编排器 ===')

  const orchestrator = useAutoOrchestrator()

  // 查看所有可用模板
  console.log('可用模板:')
  orchestrator.availableTemplates.value.forEach(t => {
    console.log(`  - ${t.icon} ${t.name} (${t.agentCount}个Agent, 约${t.estimatedTotalTime})`)
  })

  // 获取特定模板
  const kgTemplate = orchestrator.getTemplate('kg-pipeline')
  console.log('知识图谱模板 Agent 列表:')
  kgTemplate.agents.forEach((a, i) => {
    console.log(`  ${i + 1}. ${a.name} - ${a.role}`)
  })
}

// ============================================================
// 示例 5: 执行引擎控制
// ============================================================

async function exampleFlowControl() {
  console.log('=== 示例 5: 执行引擎控制 ===')

  const flowEngine = useFlowEngine()

  // 假设有一个 DSL
  const sampleDSL = {
    agents: [
      { id: 'a1', name: '第一步', role: '角色1' },
      { id: 'a2', name: '第二步', role: '角色2' },
      { id: 'a3', name: '第三步', role: '角色3' }
    ]
  }

  flowEngine.initFromDSL(sampleDSL)

  // 开始执行
  flowEngine.start()

  // 500ms 后暂停
  setTimeout(() => {
    console.log('暂停执行...')
    flowEngine.pause()

    // 再过 1s 继续
    setTimeout(() => {
      console.log('继续执行...')
      flowEngine.resume()
    }, 1000)
  }, 500)

  // 等待完成
  await new Promise(resolve => {
    const check = setInterval(() => {
      if (flowEngine.isDone.value || flowEngine.hasError.value) {
        clearInterval(check)
        resolve()
      }
    }, 100)
  })

  console.log('最终状态:', flowEngine.flowStatus.value)
  console.log('总进度:', flowEngine.progress.value + '%')
}

// ============================================================
// 示例 6: 项目归档与版本管理
// ============================================================

async function exampleArchiver() {
  console.log('=== 示例 6: 项目归档 ===')

  const archiver = useProjectArchiver()

  const project = {
    id: 'proj_001',
    name: '测试项目',
    description: '这是一个测试项目',
    category: '测试',
    tags: ['测试', '示例']
  }

  // 第一次归档
  const result1 = await archiver.archive({
    project,
    artifacts: [
      { name: 'data.csv', agentName: '数据采集' },
      { name: 'report.md', agentName: '报告生成' }
    ],
    description: '第一次运行'
  })

  console.log('第一次归档版本:', result1.snapshot.version)

  // 第二次归档
  const result2 = await archiver.archive({
    project,
    artifacts: [
      { name: 'data_v2.csv', agentName: '数据采集' },
      { name: 'report_v2.md', agentName: '报告生成' },
      { name: 'chart.png', agentName: '可视化' }
    ],
    description: '第二次运行，增加了可视化'
  })

  console.log('第二次归档版本:', result2.snapshot.version)
  console.log('总快照数:', archiver.snapshotCount.value)

  // 版本对比
  const diff = archiver.compareVersions(
    result1.snapshot.version,
    result2.snapshot.version
  )
  console.log('版本差异:', diff.summary)
}

// ============================================================
// 导出所有示例
// ============================================================

export {
  exampleFullFlow,
  exampleStepByStep,
  exampleIntentOnly,
  exampleOrchestratorOnly,
  exampleFlowControl,
  exampleArchiver
}

// 默认运行完整流程示例
// exampleFullFlow()

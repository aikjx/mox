/**
 * 项目感知对话引擎
 * 功能：解析用户对话意图、语义匹配项目、自动创建新项目
 *
 * 核心能力：
 * 1. 意图分类与实体提取
 * 2. 项目语义匹配（基于向量相似度）
 * 3. 自动项目创建（基于意图模板）
 * 4. 对话上下文管理
 */

import { ref, computed } from 'vue'
import { useProjectStore } from '../stores/projectStore'
import { useLLM } from '../services/llmService'

// ============================================================
// 意图分类器
// ============================================================
const INTENT_TYPES = {
  KNOWLEDGE_GRAPH: 'knowledge_graph',
  CODE_GEN: 'code_gen',
  DOC_GEN: 'doc_gen',
  DATA_ANALYSIS: 'data_analysis',
  RESEARCH: 'research',
  PLANNING: 'planning',
  GENERAL_CHAT: 'general_chat'
}

// 意图关键词映射（用于快速匹配，LLM 作为高精度补充）
const INTENT_KEYWORDS = {
  [INTENT_TYPES.KNOWLEDGE_GRAPH]: ['知识图谱', '图谱', 'neo4j', '实体', '关系', 'ontology', '三元组', '图数据库'],
  [INTENT_TYPES.CODE_GEN]: ['代码', '开发', '函数', '组件', 'API', '编程', '实现', '写个', '生成代码'],
  [INTENT_TYPES.DOC_GEN]: ['文档', '报告', '方案', '设计', 'PRD', '说明书', '白皮书', '写文档'],
  [INTENT_TYPES.DATA_ANALYSIS]: ['分析', '统计', '报表', '可视化', '数据', '洞察', '趋势', '对比'],
  [INTENT_TYPES.RESEARCH]: ['研究', '调研', '竞品', '市场', '行业', '前沿', '技术趋势'],
  [INTENT_TYPES.PLANNING]: ['计划', '规划', '路线图', 'roadmap', '排期', '里程碑']
}

/**
 * 快速意图分类（基于关键词，低延迟）
 * @param {string} text 用户输入文本
 * @returns {{ intentType: string, confidence: number, keywords: string[] }}
 */
function fastIntentClassify(text) {
  const lowerText = text.toLowerCase()
  const scores = {}
  const matchedKeywords = {}

  for (const [intent, keywords] of Object.entries(INTENT_KEYWORDS)) {
    scores[intent] = 0
    matchedKeywords[intent] = []
    for (const kw of keywords) {
      if (lowerText.includes(kw.toLowerCase())) {
        scores[intent] += 1
        matchedKeywords[intent].push(kw)
      }
    }
  }

  // 找出最高分
  let maxIntent = INTENT_TYPES.GENERAL_CHAT
  let maxScore = 0
  for (const [intent, score] of Object.entries(scores)) {
    if (score > maxScore) {
      maxScore = score
      maxIntent = intent
    }
  }

  // 计算置信度（基于匹配关键词数量）
  const confidence = maxScore > 0 ? Math.min(0.9, 0.5 + maxScore * 0.1) : 0.3

  return {
    intentType: maxIntent,
    confidence,
    keywords: matchedKeywords[maxIntent] || [],
    method: 'keyword'
  }
}

/**
 * LLM 高精度意图分类（异步，用于低置信度场景）
 * @param {string} text 用户输入文本
 * @param {Array} conversationHistory 对话历史
 * @returns {Promise<{ intentType: string, confidence: number, entities: Array, summary: string }>}
 */
async function llmIntentClassify(text, conversationHistory = []) {
  const llm = useLLM()

  const prompt = `
你是一个意图分类器。请分析以下用户输入，判断其意图类型。

可选意图类型：
- knowledge_graph: 知识图谱构建相关（实体抽取、关系建模、图谱构建等）
- code_gen: 代码生成相关（编写函数、组件、API等）
- doc_gen: 文档生成相关（写报告、方案、PRD等）
- data_analysis: 数据分析相关（统计分析、可视化、报表等）
- research: 研究调研相关（竞品分析、市场调研、技术研究等）
- planning: 规划计划相关（制定计划、路线图、排期等）
- general_chat: 普通闲聊

用户输入："${text}"

请以 JSON 格式返回：
{
  "intentType": "意图类型",
  "confidence": 0.0-1.0,
  "entities": ["提取的关键实体列表"],
  "summary": "一句话总结用户意图",
  "complexity": "simple | medium | complex"
}
`

  try {
    const response = await llm.chat(prompt, {
      systemPrompt: '你是一个专业的意图分析助手，输出严格的 JSON 格式。',
      temperature: 0.1,
      responseFormat: { type: 'json_object' }
    })

    const result = JSON.parse(response.content)
    return {
      ...result,
      method: 'llm'
    }
  } catch (error) {
    console.error('LLM 意图分类失败:', error)
    return {
      intentType: INTENT_TYPES.GENERAL_CHAT,
      confidence: 0.5,
      entities: [],
      summary: text.slice(0, 50),
      complexity: 'simple',
      method: 'fallback'
    }
  }
}

// ============================================================
// 项目匹配器
// ============================================================

/**
 * 计算文本与项目的语义相似度
 * 简化版：基于关键词重合度，实际可用向量数据库
 * @param {string} text 用户输入文本
 * @param {object} project 项目对象
 * @returns {number} 相似度分数 0-1
 */
function calculateSimilarity(text, project) {
  const lowerText = text.toLowerCase()
  const projectText = `${project.name} ${project.description || ''} ${project.tags?.join(' ') || ''}`.toLowerCase()

  // 简单的词频重合度计算
  const textWords = lowerText.split(/[\s,，。！？、；：]+/).filter(w => w.length > 1)
  const projectWords = projectText.split(/[\s,，。！？、；：]+/).filter(w => w.length > 1)

  if (textWords.length === 0 || projectWords.length === 0) return 0

  let matches = 0
  for (const word of textWords) {
    if (projectWords.some(pw => pw.includes(word) || word.includes(pw))) {
      matches++
    }
  }

  return matches / Math.sqrt(textWords.length * projectWords.length)
}

/**
 * 语义匹配项目
 * @param {string} text 用户输入文本
 * @param {Array} projects 项目列表
 * @param {number} threshold 匹配阈值
 * @returns {{ matched: boolean, project: object|null, score: number, candidates: Array }}
 */
function matchProject(text, projects, threshold = 0.75) {
  if (!projects || projects.length === 0) {
    return { matched: false, project: null, score: 0, candidates: [] }
  }

  // 计算所有项目的相似度
  const scored = projects.map(project => ({
    project,
    score: calculateSimilarity(text, project)
  }))

  // 按分数排序
  scored.sort((a, b) => b.score - a.score)

  const topMatch = scored[0]
  const isMatched = topMatch.score >= threshold

  return {
    matched: isMatched,
    project: isMatched ? topMatch.project : null,
    score: topMatch.score,
    candidates: scored.slice(0, 3).map(s => ({
      id: s.project.id,
      name: s.project.name,
      score: s.score
    }))
  }
}

// ============================================================
// 项目模板
// ============================================================

const PROJECT_TEMPLATES = {
  [INTENT_TYPES.KNOWLEDGE_GRAPH]: {
    icon: '🧠',
    color: '#6366f1',
    category: '知识图谱',
    defaultDescription: '知识图谱构建项目，包含数据采集、清洗、建模、入库全流程',
    tags: ['知识图谱', 'NLP', '图数据库']
  },
  [INTENT_TYPES.CODE_GEN]: {
    icon: '💻',
    color: '#10b981',
    category: '代码开发',
    defaultDescription: '代码生成与开发项目',
    tags: ['开发', '代码']
  },
  [INTENT_TYPES.DOC_GEN]: {
    icon: '📄',
    color: '#f59e0b',
    category: '文档撰写',
    defaultDescription: '文档生成与撰写项目',
    tags: ['文档', '报告']
  },
  [INTENT_TYPES.DATA_ANALYSIS]: {
    icon: '📊',
    color: '#3b82f6',
    category: '数据分析',
    defaultDescription: '数据分析与可视化项目',
    tags: ['分析', '数据']
  },
  [INTENT_TYPES.RESEARCH]: {
    icon: '🔬',
    color: '#8b5cf6',
    category: '研究调研',
    defaultDescription: '研究与调研报告项目',
    tags: ['研究', '调研']
  },
  [INTENT_TYPES.PLANNING]: {
    icon: '🗺️',
    color: '#ec4899',
    category: '规划计划',
    defaultDescription: '规划与计划制定项目',
    tags: ['规划', '计划']
  },
  [INTENT_TYPES.GENERAL_CHAT]: {
    icon: '💬',
    color: '#6b7280',
    category: '对话交流',
    defaultDescription: '通用对话项目',
    tags: ['对话']
  }
}

/**
 * 从意图创建新项目
 * @param {object} intentResult 意图解析结果
 * @returns {object} 新项目配置
 */
function createProjectFromIntent(intentResult) {
  const template = PROJECT_TEMPLATES[intentResult.intentType] || PROJECT_TEMPLATES[INTENT_TYPES.GENERAL_CHAT]

  // 生成项目名称
  const keyword = intentResult.entities?.[0] || intentResult.keywords?.[0] || '新'
  const projectName = `${keyword}${template.category}`

  // 生成项目描述
  const description = intentResult.summary || template.defaultDescription

  return {
    name: projectName,
    description,
    icon: template.icon,
    color: template.color,
    category: template.category,
    tags: [...template.tags, ...(intentResult.entities?.slice(0, 3) || [])],
    autoCreated: true,
    createdAt: new Date().toISOString()
  }
}

// ============================================================
// 组合式函数：项目感知对话引擎
// ============================================================

/**
 * 项目感知对话引擎
 * 集成意图解析、项目匹配、自动创建功能
 */
export function useProjectAwareDialogue() {
  const projectStore = useProjectStore()

  // 状态
  const isProcessing = ref(false)
  const currentIntent = ref(null)
  const matchedProject = ref(null)
  const matchCandidates = ref([])
  const lastError = ref(null)

  // 计算属性
  const hasProjectContext = computed(() => !!matchedProject.value)
  const isNewProject = computed(() => matchedProject.value?.autoCreated || false)

  /**
   * 处理用户消息，完成意图解析 + 项目匹配
   * @param {string} message 用户消息
   * @param {object} options 配置项
   * @returns {Promise<{ intent: object, project: object, isNew: boolean }>}
   */
  async function processMessage(message, options = {}) {
    const {
      useLLM = true,
      autoCreate = true,
      matchThreshold = 0.75
    } = options

    isProcessing.value = true
    lastError.value = null

    try {
      // Step 1: 快速意图分类
      let intentResult = fastIntentClassify(message)

      // Step 2: 低置信度时使用 LLM
      if (useLLM && intentResult.confidence < 0.7) {
        const llmResult = await llmIntentClassify(message)
        // 合并结果，LLM 结果优先级更高
        if (llmResult.confidence > intentResult.confidence) {
          intentResult = { ...intentResult, ...llmResult }
        }
      }

      currentIntent.value = intentResult

      // Step 3: 项目匹配
      const projects = projectStore.projects
      const matchResult = matchProject(message, projects, matchThreshold)
      matchCandidates.value = matchResult.candidates

      if (matchResult.matched) {
        // 匹配到现有项目
        matchedProject.value = matchResult.project
        return {
          intent: intentResult,
          project: matchResult.project,
          isNew: false
        }
      }

      // Step 4: 未匹配到，自动创建新项目
      if (autoCreate && intentResult.intentType !== INTENT_TYPES.GENERAL_CHAT) {
        const newProjectConfig = createProjectFromIntent(intentResult)
        const newProject = await projectStore.createProject(newProjectConfig)
        matchedProject.value = newProject

        return {
          intent: intentResult,
          project: newProject,
          isNew: true
        }
      }

      // 普通闲聊，不绑定项目
      matchedProject.value = null
      return {
        intent: intentResult,
        project: null,
        isNew: false
      }

    } catch (error) {
      console.error('对话处理失败:', error)
      lastError.value = error.message
      throw error
    } finally {
      isProcessing.value = false
    }
  }

  /**
   * 手动选择项目
   * @param {string} projectId 项目ID
   */
  function selectProject(projectId) {
    const project = projectStore.projects.find(p => p.id === projectId)
    if (project) {
      matchedProject.value = project
    }
  }

  /**
   * 重置对话上下文
   */
  function resetContext() {
    currentIntent.value = null
    matchedProject.value = null
    matchCandidates.value = []
    lastError.value = null
  }

  return {
    // 状态
    isProcessing,
    currentIntent,
    matchedProject,
    matchCandidates,
    lastError,
    // 计算属性
    hasProjectContext,
    isNewProject,
    // 方法
    processMessage,
    selectProject,
    resetContext,
    // 常量
    INTENT_TYPES
  }
}

export default useProjectAwareDialogue

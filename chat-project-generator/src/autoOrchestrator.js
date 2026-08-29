/**
 * 自动编排器
 * 功能：根据意图类型和项目上下文，动态选择并组装 Agent 工作流
 *
 * 核心能力：
 * 1. 工作流模板管理
 * 2. 动态 DSL 生成
 * 3. Agent 依赖解析
 * 4. 资源分配与优化
 */

import { ref, computed } from 'vue'

// ============================================================
// 工作流模板定义
// ============================================================

const WORKFLOW_TEMPLATES = {
  // 知识图谱构建流水线
  knowledge_graph: {
    id: 'kg-pipeline',
    name: '知识图谱构建流水线',
    description: '从数据采集到图谱入库的完整知识图谱构建流程',
    icon: '🧠',
    agents: [
      {
        id: 'agent_kg_collect',
        name: '数据采集',
        role: '数据采集引擎',
        description: '从多源采集结构化与非结构化数据',
        tools: ['Apache NiFi', 'Airbyte', 'Scrapy'],
        estimatedTime: '10-30分钟'
      },
      {
        id: 'agent_kg_clean',
        name: '数据清洗',
        role: '数据清洗引擎',
        description: '去重、归一化、缺失值处理',
        tools: ['Pandas', 'OpenRefine', 'Great Expectations'],
        estimatedTime: '15-45分钟'
      },
      {
        id: 'agent_kg_extract',
        name: '实体关系抽取',
        role: 'NLP 抽取引擎',
        description: '命名实体识别、关系抽取、属性抽取',
        tools: ['spaCy', 'HanLP', 'LLM Fine-tuning'],
        estimatedTime: '20-60分钟'
      },
      {
        id: 'agent_kg_model',
        name: '本体建模',
        role: '知识建模引擎',
        description: '设计本体 schema、定义实体类型与关系',
        tools: ['Protégé', 'Neo4j Schema', 'OWL'],
        estimatedTime: '15-40分钟'
      },
      {
        id: 'agent_kg_integrate',
        name: '知识融合',
        role: '知识融合引擎',
        description: '实体对齐、属性融合、冲突消解',
        tools: ['Dedupe', 'Silk', 'LIMES'],
        estimatedTime: '20-50分钟'
      },
      {
        id: 'agent_kg_quality',
        name: '质量评估',
        role: '质量评估引擎',
        description: '准确性、完整性、一致性评估',
        tools: ['SHACL', '自定义规则', '抽样校验'],
        estimatedTime: '10-25分钟'
      },
      {
        id: 'agent_kg_store',
        name: '图谱入库',
        role: '存储引擎',
        description: '数据导入图数据库、建立索引',
        tools: ['Neo4j', 'NebulaGraph', 'Dgraph'],
        estimatedTime: '15-40分钟'
      },
      {
        id: 'agent_kg_visualize',
        name: '可视化呈现',
        role: '可视化引擎',
        description: '生成图谱可视化、统计报告',
        tools: ['ECharts', 'D3.js', 'Neo4j Bloom'],
        estimatedTime: '10-20分钟'
      }
    ],
    edges: [
      { from: 'agent_kg_collect', to: 'agent_kg_clean' },
      { from: 'agent_kg_clean', to: 'agent_kg_extract' },
      { from: 'agent_kg_extract', to: 'agent_kg_model' },
      { from: 'agent_kg_model', to: 'agent_kg_integrate' },
      { from: 'agent_kg_integrate', to: 'agent_kg_quality' },
      { from: 'agent_kg_quality', to: 'agent_kg_store' },
      { from: 'agent_kg_store', to: 'agent_kg_visualize' }
    ]
  },

  // 代码生成流水线
  code_gen: {
    id: 'code-gen-pipeline',
    name: '代码生成流水线',
    description: '从需求分析到代码实现的完整开发流程',
    icon: '💻',
    agents: [
      {
        id: 'agent_code_analyze',
        name: '需求分析',
        role: '需求分析师',
        description: '分析需求、明确功能边界',
        tools: ['需求模板', '用例分析'],
        estimatedTime: '5-15分钟'
      },
      {
        id: 'agent_code_design',
        name: '架构设计',
        role: '架构师',
        description: '技术选型、模块设计、接口定义',
        tools: ['设计模式库', '架构模板'],
        estimatedTime: '10-25分钟'
      },
      {
        id: 'agent_code_implement',
        name: '代码实现',
        role: '开发工程师',
        description: '编写核心代码、单元测试',
        tools: ['代码生成器', '代码审查'],
        estimatedTime: '15-60分钟'
      },
      {
        id: 'agent_code_test',
        name: '测试验证',
        role: '测试工程师',
        description: '功能测试、边界测试、性能测试',
        tools: ['测试框架', 'Mock 工具'],
        estimatedTime: '10-30分钟'
      },
      {
        id: 'agent_code_document',
        name: '文档生成',
        role: '技术文档工程师',
        description: '生成 API 文档、使用说明',
        tools: ['JSDoc', 'Swagger'],
        estimatedTime: '5-15分钟'
      }
    ],
    edges: [
      { from: 'agent_code_analyze', to: 'agent_code_design' },
      { from: 'agent_code_design', to: 'agent_code_implement' },
      { from: 'agent_code_implement', to: 'agent_code_test' },
      { from: 'agent_code_test', to: 'agent_code_document' }
    ]
  },

  // 文档生成流水线
  doc_gen: {
    id: 'doc-gen-pipeline',
    name: '文档生成流水线',
    description: '从大纲到成稿的完整文档撰写流程',
    icon: '📄',
    agents: [
      {
        id: 'agent_doc_outline',
        name: '大纲规划',
        role: '内容策划师',
        description: '确定文档结构、章节安排',
        tools: ['大纲模板', '结构优化'],
        estimatedTime: '5-15分钟'
      },
      {
        id: 'agent_doc_research',
        name: '资料收集',
        role: '研究分析师',
        description: '收集相关资料、数据支撑',
        tools: ['搜索引擎', '知识库'],
        estimatedTime: '10-30分钟'
      },
      {
        id: 'agent_doc_write',
        name: '内容撰写',
        role: '内容编辑',
        description: '撰写各章节内容',
        tools: ['写作助手', '风格指南'],
        estimatedTime: '20-60分钟'
      },
      {
        id: 'agent_doc_review',
        name: '审校优化',
        role: '审校编辑',
        description: '内容审核、润色优化',
        tools: ['语法检查', '风格统一'],
        estimatedTime: '10-25分钟'
      },
      {
        id: 'agent_doc_format',
        name: '排版输出',
        role: '排版设计师',
        description: '格式排版、图表美化、多格式导出',
        tools: ['排版引擎', '图表生成'],
        estimatedTime: '5-15分钟'
      }
    ],
    edges: [
      { from: 'agent_doc_outline', to: 'agent_doc_research' },
      { from: 'agent_doc_research', to: 'agent_doc_write' },
      { from: 'agent_doc_write', to: 'agent_doc_review' },
      { from: 'agent_doc_review', to: 'agent_doc_format' }
    ]
  },

  // 数据分析流水线
  data_analysis: {
    id: 'data-analysis-pipeline',
    name: '数据分析流水线',
    description: '从数据探索到洞察报告的完整分析流程',
    icon: '📊',
    agents: [
      {
        id: 'agent_data_explore',
        name: '数据探索',
        role: '数据分析师',
        description: '数据概览、分布分析、异常检测',
        tools: ['Pandas', 'NumPy', '统计方法'],
        estimatedTime: '10-25分钟'
      },
      {
        id: 'agent_data_clean',
        name: '数据预处理',
        role: '数据工程师',
        description: '数据清洗、特征工程',
        tools: ['Scikit-learn', '特征选择'],
        estimatedTime: '15-35分钟'
      },
      {
        id: 'agent_data_analyze',
        name: '深度分析',
        role: '高级分析师',
        description: '建模分析、关联分析、预测分析',
        tools: ['机器学习', '统计模型'],
        estimatedTime: '20-50分钟'
      },
      {
        id: 'agent_data_visualize',
        name: '可视化',
        role: '可视化工程师',
        description: '生成图表、仪表盘设计',
        tools: ['ECharts', 'Matplotlib', 'D3.js'],
        estimatedTime: '15-30分钟'
      },
      {
        id: 'agent_data_report',
        name: '报告生成',
        role: '报告撰写人',
        description: '撰写分析报告、提炼洞察',
        tools: ['报告模板', '文案优化'],
        estimatedTime: '10-25分钟'
      }
    ],
    edges: [
      { from: 'agent_data_explore', to: 'agent_data_clean' },
      { from: 'agent_data_clean', to: 'agent_data_analyze' },
      { from: 'agent_data_analyze', to: 'agent_data_visualize' },
      { from: 'agent_data_visualize', to: 'agent_data_report' }
    ]
  }
}

// ============================================================
// 工作流裁剪器（根据复杂度调整）
// ============================================================

/**
 * 根据任务复杂度裁剪工作流
 * @param {object} template 工作流模板
 * @param {string} complexity 任务复杂度 simple | medium | complex
 * @returns {object} 裁剪后的工作流
 */
function adaptWorkflowByComplexity(template, complexity = 'medium') {
  const workflow = JSON.parse(JSON.stringify(template)) // 深拷贝

  if (complexity === 'simple') {
    // 简单任务：合并步骤，只保留核心 Agent
    const coreAgents = workflow.agents.filter((_, idx) => idx % 2 === 0 || idx === workflow.agents.length - 1)
    workflow.agents = coreAgents

    // 重建边
    workflow.edges = []
    for (let i = 0; i < coreAgents.length - 1; i++) {
      workflow.edges.push({
        from: coreAgents[i].id,
        to: coreAgents[i + 1].id
      })
    }
  } else if (complexity === 'complex') {
    // 复杂任务：增加审查和优化节点
    const reviewAgent = {
      id: 'agent_review_' + Date.now(),
      name: '专家评审',
      role: '专家顾问',
      description: '对中间结果进行质量评审和优化建议',
      tools: ['质量检查清单', '最佳实践库'],
      estimatedTime: '10-20分钟',
      isReview: true
    }

    // 在关键节点后插入评审
    const midIndex = Math.floor(workflow.agents.length / 2)
    workflow.agents.splice(midIndex + 1, 0, reviewAgent)

    // 重建边
    workflow.edges = []
    for (let i = 0; i < workflow.agents.length - 1; i++) {
      workflow.edges.push({
        from: workflow.agents[i].id,
        to: workflow.agents[i + 1].id
      })
    }
  }

  // medium 复杂度使用原始模板

  return workflow
}

// ============================================================
// DSL 生成器
// ============================================================

/**
 * 从工作流配置生成 DSL
 * @param {object} workflow 工作流配置
 * @param {object} context 上下文（项目信息、用户信息等）
 * @returns {object} DSL 配置对象
 */
function generateDSL(workflow, context = {}) {
  const { project, intent } = context

  // 构建 Agent DSL 节点
  const agents = workflow.agents.map((agent, idx) => {
    const nextAgent = workflow.agents[idx + 1]
    return {
      id: agent.id,
      name: agent.name,
      role: agent.role,
      description: agent.description,
      input: idx === 0 ? [
        { type: 'project_context', project_id: project?.id },
        { type: 'intent', intent_type: intent?.intentType }
      ] : [{ type: 'upstream_result', from: workflow.agents[idx - 1]?.id }],
      output: [{ type: 'result', format: 'structured' }],
      tools: agent.tools || [],
      retry: {
        max_attempts: 2,
        backoff: 'exponential'
      },
      on: 'done',
      next: nextAgent?.id || null,
      on_error: 'retry'
    }
  })

  return {
    version: '1.0',
    name: `${workflow.id}-${Date.now()}`,
    description: workflow.description,
    meta: {
      template_id: workflow.id,
      template_name: workflow.name,
      created_at: new Date().toISOString(),
      project_id: project?.id,
      project_name: project?.name,
      intent_type: intent?.intentType
    },
    agents,
    policy: {
      human_in_the_loop: 'high_risk_only',
      audit: 'enabled',
      rollback_strategy: 'per_agent_snapshot',
      versioning: 'semantic'
    }
  }
}

// ============================================================
// 组合式函数：自动编排器
// ============================================================

/**
 * 自动编排器
 */
export function useAutoOrchestrator() {
  const templates = ref(WORKFLOW_TEMPLATES)
  const currentWorkflow = ref(null)
  const generatedDSL = ref(null)
  const isOrchestrating = ref(false)

  // 可用的模板列表
  const availableTemplates = computed(() => {
    return Object.values(templates.value).map(t => ({
      id: t.id,
      name: t.name,
      description: t.description,
      icon: t.icon,
      agentCount: t.agents.length,
      estimatedTotalTime: estimateTotalTime(t)
    }))
  })

  /**
   * 估算工作流总耗时
   * @param {object} workflow 工作流
   * @returns {string} 估算时间描述
   */
  function estimateTotalTime(workflow) {
    let minTotal = 0
    let maxTotal = 0
    workflow.agents.forEach(agent => {
      const match = agent.estimatedTime?.match(/(\d+)-(\d+)分钟/)
      if (match) {
        minTotal += parseInt(match[1])
        maxTotal += parseInt(match[2])
      }
    })
    return `${minTotal}-${maxTotal}分钟`
  }

  /**
   * 根据意图类型选择模板
   * @param {string} intentType 意图类型
   * @returns {object|null} 匹配的模板
   */
  function selectTemplateByIntent(intentType) {
    const template = templates.value[intentType]
    if (!template) {
      console.warn(`未找到意图类型 ${intentType} 对应的工作流模板，使用默认文档生成模板`)
      return templates.value.doc_gen
    }
    return template
  }

  /**
   * 编排工作流
   * @param {object} params 编排参数
   * @param {string} params.intentType 意图类型
   * @param {string} params.complexity 复杂度 simple|medium|complex
   * @param {object} params.project 项目上下文
   * @param {object} params.intent 意图解析结果
   * @returns {Promise<{ workflow: object, dsl: object }>}
   */
  async function orchestrate(params = {}) {
    const {
      intentType,
      complexity = 'medium',
      project = null,
      intent = null
    } = params

    isOrchestrating.value = true

    try {
      // 1. 选择模板
      const template = selectTemplateByIntent(intentType)
      if (!template) {
        throw new Error(`未找到可用的工作流模板: ${intentType}`)
      }

      // 2. 根据复杂度调整
      const adaptedWorkflow = adaptWorkflowByComplexity(template, complexity)

      // 3. 注入项目上下文
      const workflowWithContext = {
        ...adaptedWorkflow,
        projectId: project?.id,
        projectName: project?.name,
        createdAt: new Date().toISOString()
      }

      currentWorkflow.value = workflowWithContext

      // 4. 生成 DSL
      const dsl = generateDSL(workflowWithContext, { project, intent })
      generatedDSL.value = dsl

      return {
        workflow: workflowWithContext,
        dsl
      }

    } catch (error) {
      console.error('工作流编排失败:', error)
      throw error
    } finally {
      isOrchestrating.value = false
    }
  }

  /**
   * 获取模板详情
   * @param {string} templateId 模板ID
   * @returns {object|null} 模板详情
   */
  function getTemplate(templateId) {
    for (const template of Object.values(templates.value)) {
      if (template.id === templateId) {
        return template
      }
    }
    return null
  }

  /**
   * 注册自定义模板
   * @param {string} key 模板键
   * @param {object} template 模板定义
   */
  function registerTemplate(key, template) {
    templates.value[key] = template
  }

  /**
   * 重置编排器状态
   */
  function reset() {
    currentWorkflow.value = null
    generatedDSL.value = null
  }

  return {
    // 状态
    templates,
    currentWorkflow,
    generatedDSL,
    isOrchestrating,
    availableTemplates,
    // 方法
    orchestrate,
    selectTemplateByIntent,
    getTemplate,
    registerTemplate,
    estimateTotalTime,
    reset
  }
}

export default useAutoOrchestrator

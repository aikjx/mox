'use strict';

/**
 * 项目全息图谱 · 业务资产注册表（domain 层 · 静态值对象 · 零 IO）
 * ------------------------------------------------------------------
 * 24 个业务域 + 4 个可插拔模块的唯一权威定义。
 * 每条登记：身份 / 核心功能 / 代码路径 / 依赖引擎 / 数据资产 / 关联文档。
 * 全部功能自研（可借鉴业界架构思想，实现零外部框架依赖）。
 */

const DOMAINS = [
  {
    id: 'system', name: '系统与状态', codePath: 'src/routes/system.js',
    keyFeatures: ['健康检查与运行状态', '服务元信息自描述', '运行日志查询'],
    engines: ['knowledge-graph'], dataAssets: ['settings.json', 'logs.json'], docs: ['docs/architecture.md']
  },
  {
    id: 'graph', name: '知识图谱', codePath: 'src/routes/graph.js',
    keyFeatures: ['图谱节点/边 CRUD 与检索', '图算法分析（PageRank/中心性/社区/路径）', '图谱统计与结构洞察'],
    engines: ['knowledge-graph', 'ai-integration-engine'], dataAssets: ['graph_nodes.json', 'graph_edges.json'], docs: ['docs/modules/business-process-flowcharts.md']
  },
  {
    id: 'chat', name: 'AI 对话', codePath: 'src/routes/chat.js',
    keyFeatures: ['多会话对话管理', '会话记忆语义检索', '联网搜索上下文增强'],
    engines: ['llm-gateway', 'session-store', 'web-search-service'], dataAssets: ['dialogue_sessions.json'], docs: ['docs/对话开发系统-全维分析与业务流程图.md']
  },
  {
    id: 'web-search', name: '联网搜索', codePath: 'src/routes/web-search.js',
    keyFeatures: ['多搜索引擎配置管理', '统一搜索入口与引用结构化', '搜索连通性测试'],
    engines: ['web-search-service'], dataAssets: ['settings.json', 'web_search_config.json'], docs: ['docs/AI-UNIFIED-OPTIMIZATION-PLAN.md']
  },
  {
    id: 'artifacts', name: '本地制品', codePath: 'src/routes/artifacts.js',
    keyFeatures: ['制品注册表（按文件名去重）', '安全文件落盘（路径逃逸校验）', '制品预览与下载'],
    engines: ['auto-dev-engine'], dataAssets: ['artifacts.json'], docs: ['docs/modules/local-artifact-agent.md']
  },
  {
    id: 'optimizer', name: '无穷维度优化', codePath: 'src/routes/optimizer.js',
    keyFeatures: ['CEM 高维配置寻优', '多引擎对比验证矩阵', '收敛曲线与维度敏感度可视化'],
    engines: ['infinite-dimension-optimizer'], dataAssets: ['infinite_optimization_runs.json'], docs: ['docs/modules/infinite-dimension-optimization.md']
  },
  {
    id: 'ai-platform', name: 'AI 平台资源', codePath: 'src/routes/ai-platform.js',
    keyFeatures: ['工作流定义与管理', '算子注册与编排', '资源池管理'],
    engines: ['ai-engine'], dataAssets: ['workflows.json', 'operators.json', 'resources.json', 'flows.json'], docs: ['docs/modules/automation-module.md']
  },
  {
    id: 'browser-market', name: '浏览器与市场', codePath: 'src/routes/browser-market.js',
    keyFeatures: ['智能体市场', '流水线注册与交易', '插件生态管理'],
    engines: ['llm-gateway'], dataAssets: ['market.json', 'registered_agents.json', 'registered_pipelines.json', 'plugins.json'], docs: ['docs/modules/market-module.md']
  },
  {
    id: 'integration', name: '集成通道', codePath: 'src/routes/integration.js',
    keyFeatures: ['多 LLM 提供商管理', '路由权重配置', '用量统计'],
    engines: ['llm-gateway'], dataAssets: ['llm_config.json', 'llm_routing.json', 'llm_usage.json'], docs: ['docs/modules/ai-engine-master-analysis.md']
  },
  {
    id: 'expert-alliance', name: '专家联盟', codePath: 'src/routes/expert-alliance.js',
    keyFeatures: ['专家全生命周期与咨询编排', '多专家辩论与会话链', '智能路由与指标反馈'],
    engines: ['expert-alliance', 'expert-alliance-engine', 'expert-dispatcher'], dataAssets: ['experts.json', 'expert_sessions.json', 'expert_chat_history.json', 'alliance_intent_priors.json', 'alliance_traces.jsonl', 'dispatcher_config.json', 'learned_skills.json'], docs: ['docs/modules/专家联盟AI对话需求文档-V2.0-架构优化版.md', 'docs/modules/xuanji-expert-alliance-fusion-flows.md']
  },
  {
    id: 'expert-graph', name: '专家图谱', codePath: 'src/routes/expert-graph.js',
    keyFeatures: ['专家能力图三级建边', 'CNM 社区聚类', '协同增益计算'],
    engines: ['expert-graph'], dataAssets: ['expert_capability_graph.json'], docs: ['docs/modules/专家联盟V2.0-集成对齐分析报告.md']
  },
  {
    id: 'orchestration', name: '编排协作', codePath: 'src/routes/orchestration.js',
    keyFeatures: ['插件化编排流水线', '检查点与事务回放', '编排统计'],
    engines: ['orchestration-engine'], dataAssets: ['plugins.json'], docs: ['docs/modules/automation-module.md']
  },
  {
    id: 'ai-enhanced', name: '16 模块 AI 增强', codePath: 'src/routes/ai-enhanced.js',
    keyFeatures: ['16 个 AI 增强模块', '模板化内容生成', '算子级 AI 任务'],
    engines: ['llm-gateway'], dataAssets: ['caomei_templates.json', 'operators.json'], docs: ['docs/modules/algorithm-verification.md']
  },
  {
    id: 'tasks', name: '任务管理', codePath: 'src/routes/tasks.js',
    keyFeatures: ['任务 CRUD 与状态机', '任务分配与追踪', '任务关联会话'],
    engines: ['expert-alliance'], dataAssets: ['tasks.json'], docs: ['docs/enterprise/04-business-processing.md']
  },
  {
    id: 'kb', name: '知识库', codePath: 'src/routes/kb.js',
    keyFeatures: ['文档全生命周期与版本快照', 'AI 文档分析（实体/关键词/分类）', '文档实体与图谱互链'],
    engines: ['kb'], dataAssets: ['kb_documents.json', 'kb_categories.json', 'kb_versions.json', 'kb_history.json'], docs: ['docs/DOC-NORMALIZATION-REPORT.md']
  },
  {
    id: 'engine-universe', name: '引擎宇宙图谱', codePath: 'src/routes/engine-universe.js',
    keyFeatures: ['17 引擎节点化与关联边查询', '需求归一化链服务映射', '全链路 113 项机器验证'],
    engines: ['engine-universe'], dataAssets: [], docs: ['docs/standards/engine-universe.md']
  },
  {
    id: 'engine-kernel', name: '引擎内核', codePath: 'src/routes/engine-kernel.js',
    keyFeatures: ['槽位契约架构（一切皆可插件化，切换引擎零代码改动）', '瞬间切换与失败自动回滚（探活保障银行级不宕机）', '三层插件商城（系统内置/云端目录/本地清单）', 'AI 自动配置引擎组合（自然语言需求→绑定方案）'],
    engines: ['engine-kernel', 'llm-gateway', 'web-search-service'], dataAssets: ['engine_bindings.json', 'engine_plugins.json', 'engine_marketplace.json'], docs: ['docs/standards/engine-kernel.md']
  },
  {
    id: 'atlas', name: '项目全息图谱', codePath: 'src/routes/atlas.js',
    keyFeatures: ['全项目机器图谱化（域/引擎/算法/数据/文档统一关联）', '无破窗验证 145 项（动态比对真实代码库）', 'AI 架构师图谱增强对话'],
    engines: ['project-atlas', 'expert-alliance', 'engine-universe'], dataAssets: [], docs: ['docs/standards/project-atlas.md']
  },
  {
    id: 'auto-tasks', name: '自动任务', codePath: 'src/routes/auto-tasks.js',
    keyFeatures: ['自动化任务调度', '任务执行引擎', '执行历史与重试'],
    engines: ['orchestration-engine'], dataAssets: ['automation.json'], docs: ['docs/enterprise/08-全维自动化处理明确书.md']
  },
  {
    id: 'modules-admin', name: '模块与存储管理', codePath: 'src/routes/modules-admin.js',
    keyFeatures: ['可插拔模块管理', '存储提供方切换', '数据迁移'],
    engines: ['knowledge-graph'], dataAssets: ['settings.json'], docs: ['docs/specs/PT-Primi-架构规范-V1.0-完整版.md']
  },
  {
    id: 'security', name: '安全审计', codePath: 'src/routes/security.js',
    keyFeatures: ['操作审计日志', '安全状态检查', '密钥加密管理'],
    engines: ['llm-gateway'], dataAssets: ['logs.json'], docs: ['docs/enterprise/12-RBAC审计全链路闭环验收报告.md']
  },
  {
    id: 'ai-engine', name: 'AI 引擎核心', codePath: 'src/routes/ai-engine.js',
    keyFeatures: ['统一编排五步流水线', '意图识别（激活扩散）', '能力矩阵自描述'],
    engines: ['ai-engine-core', 'ai-engine'], dataAssets: ['graph_nodes.json', 'graph_edges.json'], docs: ['docs/modules/ai-engine-master-analysis.md']
  },
  {
    id: 'ai-integrated', name: '智能集成引擎', codePath: 'src/routes/ai-integrated.js',
    keyFeatures: ['个性化 PageRank 图计算', '符号图 LLM 交互', 'token 预算裁剪'],
    engines: ['ai-integration-engine'], dataAssets: ['llm_usage.json'], docs: ['docs/modules/mathematical-foundation.md']
  },
  {
    id: 'ai-ultimate', name: '终极 AI 引擎', codePath: 'src/routes/ai-ultimate.js',
    keyFeatures: ['向量记忆语义检索', '多步推理与置信度评估', '推理规则管理'],
    engines: ['ultimate-ai-engine'], dataAssets: ['ultimate_reasoning_rules.json'], docs: ['docs/modules/ai-engine-master-analysis.md']
  },
  {
    id: 'auto-dev', name: '自动开发引擎', codePath: 'src/routes/auto-dev.js',
    keyFeatures: ['需求→架构图谱→代码全自动流水线', '确定性代码渲染（无幻觉）', '制品预览与注册'],
    engines: ['auto-dev-engine', 'llm-gateway'], dataAssets: ['artifacts.json'], docs: ['docs/modules/PrimiFlow-设计蓝图.md']
  },
  {
    id: 'services', name: '服务管理', codePath: 'src/routes/services.js',
    keyFeatures: ['外部服务注册与探活', '服务依赖管理', '优雅启停'],
    engines: ['llm-gateway'], dataAssets: ['settings.json'], docs: ['docs/enterprise/02-architecture.md']
  }
];

const MODULES = [
  {
    id: 'mod-graph', name: '图谱模块', codePath: 'src/modules/graph.js',
    keyFeatures: ['图谱批量导入导出', '节点详情与邻居查询', '图谱检索'],
    engines: ['knowledge-graph'], dataAssets: ['graph_nodes.json', 'graph_edges.json'], docs: ['docs/graph/guantu.req.json']
  },
  {
    id: 'mod-task', name: '任务模块', codePath: 'src/modules/task.js',
    keyFeatures: ['任务 REST CRUD', '任务状态流转', '任务列表查询'],
    engines: ['expert-alliance'], dataAssets: ['tasks.json'], docs: ['docs/enterprise/04-business-processing.md']
  },
  {
    id: 'mod-storage', name: '存储模块', codePath: 'src/modules/storage.js',
    keyFeatures: ['SQLite + JSON 双写存储', '存储提供方切换', '历史数据迁移'],
    engines: ['knowledge-graph'], dataAssets: ['settings.json'], docs: ['docs/specs/OUS-业务功能规划与架构数据关系分析.md']
  },
  {
    id: 'mod-melody2score', name: '旋律转谱模块', codePath: 'src/modules/melody2score.js',
    keyFeatures: ['旋律→乐谱工业级转换（8/8 样本全对）', '多音高检测后端自动降级', 'MusicXML/简谱双输出'],
    engines: ['llm-gateway'], dataAssets: [], docs: ['docs/modules/algorithm-verification.md']
  }
];

module.exports = { DOMAINS, MODULES };

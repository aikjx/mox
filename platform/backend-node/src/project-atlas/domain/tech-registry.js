'use strict';

/**
 * 项目全息图谱 · 技术资产注册表（domain 层 · 静态值对象 · 零 IO）
 * ------------------------------------------------------------------
 * 核心算法（自研实现 + 单源标注）+ 数据资产 + 核心文档。
 */

// ============ 核心算法（全部自研实现，singleSource 标注唯一权威定义处） ============
const ALGORITHMS = [
  {
    id: 'algo-pagerank', name: 'PageRank 质量传播', codePath: 'src/ai-integration-engine.js',
    principle: '标准推模型：节点质量沿出边传播给目标，转置图处理保证方向正确',
    singleSource: true, category: '图算法', consumers: ['ai-engine', 'expert-graph', 'ai-flow-graph']
  },
  {
    id: 'algo-cnm', name: 'CNM 模块度贪心社区检测', codePath: 'src/ai-flow-graph.js',
    principle: '模块度贪心凝聚：每轮合并使 ΔQ 最大的社区对，避免 LPA 标签吞并',
    singleSource: true, category: '图算法', consumers: ['ai-engine', 'expert-graph']
  },
  {
    id: 'algo-brandes', name: 'Brandes 介数中心性', codePath: 'src/ai-flow-graph.js',
    principle: '单源最短路 DAG 累加，O(VE) 计算 all-pairs 介数',
    singleSource: true, category: '图算法', consumers: ['ai-engine', 'graph']
  },
  {
    id: 'algo-harmonic', name: 'Harmonic 紧密中心性', codePath: 'src/ai-flow-graph.js',
    principle: '调和平均处理不可达节点：Σ(1/d(v,u)) 归一化',
    singleSource: true, category: '图算法', consumers: ['ai-engine', 'graph']
  },
  {
    id: 'algo-degree', name: '度中心性（RAW 边展开）', codePath: 'src/ai-flow-graph.js',
    principle: '无向边统一 RAW 输入库内展开，度 = 出度 + 入度',
    singleSource: true, category: '图算法', consumers: ['ai-engine', 'graph']
  },
  {
    id: 'algo-density', name: '图密度', codePath: 'src/ai-flow-graph.js',
    principle: 'm / (n(n-1)) 有向归一，附人读解读（高度稠密/中等/稀疏）',
    singleSource: true, category: '图算法', consumers: ['ai-engine', 'graph']
  },
  {
    id: 'algo-spread', name: '激活扩散意图识别', codePath: 'src/ai-flow-graph.js',
    principle: '个性化 PageRank 特例（method=spread, d=0.85, 30 轮收敛）',
    singleSource: true, category: '意图算法', consumers: ['ai-engine-core', 'ai-flow-graph']
  },
  {
    id: 'algo-intent', name: '意图多标签分类', codePath: 'src/expert-alliance/domain/intent-classifier.js',
    principle: '关键词加权 + 15 意图域 + 置信度归一（多义性降权）',
    singleSource: true, category: '意图算法', consumers: ['expert-alliance', 'expert-alliance-engine']
  },
  {
    id: 'algo-match', name: '专家最优组队', codePath: 'src/expert-alliance/domain/expert-matcher.js',
    principle: '多目标：能力匹配 + 图谱协同增益 + Dispatcher 负载均衡加权',
    singleSource: true, category: '协作算法', consumers: ['expert-alliance', 'expert-alliance-engine']
  },
  {
    id: 'algo-debate', name: '辩论综合合成', codePath: 'src/expert-alliance/domain/debate-synthesis.js',
    principle: '共识提取 + 分歧保留 + 最终建议生成（真实内容驱动无硬编码）',
    singleSource: true, category: '协作算法', consumers: ['expert-alliance', 'expert-alliance-engine']
  },
  {
    id: 'algo-cem', name: 'CEM 交叉熵优化', codePath: 'src/infinite-dimension-optimizer.js',
    principle: '高维空间采样→评估→精英选择→分布更新，σ̄<0.06 收敛',
    singleSource: true, category: '优化算法', consumers: ['infinite-dimension-optimizer']
  },
  {
    id: 'algo-lcs', name: 'LCS 版本差异', codePath: 'src/kb/domain/version-differ.js',
    principle: '行级最长公共子序列，输出新增/删除/相似度',
    singleSource: true, category: '文档算法', consumers: ['kb']
  },
  {
    id: 'algo-docanalyze', name: '文档智能分析', codePath: 'src/kb/domain/document-analyzer.js',
    principle: '正则实体抽取 + 分类关键词打分 + 阅读指标',
    singleSource: true, category: '文档算法', consumers: ['kb']
  },
  {
    id: 'algo-bfs', name: 'BFS 最短路径', codePath: 'src/lib/graph-algos.js',
    principle: '队列式广度优先，前置节点回溯路径',
    singleSource: true, category: '图算法', consumers: ['graph', 'engine-universe', 'project-atlas']
  },
  {
    id: 'algo-median', name: '中值滤波音高平滑', codePath: 'melody2score/core/pipeline.py',
    principle: 'NaN 隔离 + 跨音符窗口保护 + 下中值消除半音伪影（8/8 样本全对）',
    singleSource: true, category: '信号算法', consumers: ['mod-melody2score']
  },
  {
    id: 'algo-slot-contract', name: '槽位契约路由', codePath: 'src/engine-kernel/domain/contract-registry.js',
    principle: '能力槽位化 + 方法签名/输入输出契约文档化，调用方只依赖契约不依赖具体引擎',
    singleSource: true, category: '架构算法', consumers: ['engine-kernel']
  },
  {
    id: 'algo-switch-rollback', name: '切换探活回滚', codePath: 'src/engine-kernel/application/switch-service.js',
    principle: '校验→切换→契约探活→失败自动回滚原绑定（银行级不宕机切换）',
    singleSource: true, category: '架构算法', consumers: ['engine-kernel']
  }
];

// ============ 数据资产（data/ 目录 34 个 JSON/JSONL，全域覆盖） ============
const DATA_ASSETS = [
  { file: 'settings.json', domain: 'system', desc: '系统全局配置' },
  { file: 'logs.json', domain: 'system', desc: '操作与审计日志' },
  { file: 'graph_nodes.json', domain: 'graph', desc: '知识图谱节点（55）' },
  { file: 'graph_edges.json', domain: 'graph', desc: '知识图谱边（71）' },
  { file: 'dialogue_sessions.json', domain: 'chat', desc: 'AI 对话会话' },
  { file: 'artifacts.json', domain: 'artifacts', desc: '本地制品注册表' },
  { file: 'infinite_optimization_runs.json', domain: 'optimizer', desc: '优化运行记录' },
  { file: 'workflows.json', domain: 'ai-platform', desc: '工作流定义' },
  { file: 'operators.json', domain: 'ai-platform', desc: '算子注册表' },
  { file: 'resources.json', domain: 'ai-platform', desc: '资源池' },
  { file: 'flows.json', domain: 'ai-platform', desc: '流程定义' },
  { file: 'market.json', domain: 'browser-market', desc: '智能体市场' },
  { file: 'registered_agents.json', domain: 'browser-market', desc: '注册智能体' },
  { file: 'registered_pipelines.json', domain: 'browser-market', desc: '注册流水线' },
  { file: 'plugins.json', domain: 'orchestration', desc: '编排插件' },
  { file: 'llm_config.json', domain: 'integration', desc: 'LLM 提供商配置（密钥加密）' },
  { file: 'web_search_config.json', domain: 'web-search', desc: '联网搜索引擎配置（槽位切换落点）' },
  { file: 'llm_routing.json', domain: 'integration', desc: 'LLM 路由权重' },
  { file: 'llm_usage.json', domain: 'integration', desc: 'LLM 用量统计' },
  { file: 'experts.json', domain: 'expert-alliance', desc: '专家注册表（15+1 专家）' },
  { file: 'expert_sessions.json', domain: 'expert-alliance', desc: '专家会话' },
  { file: 'expert_chat_history.json', domain: 'expert-alliance', desc: '专家对话历史' },
  { file: 'alliance_intent_priors.json', domain: 'expert-alliance', desc: '意图先验反馈' },
  { file: 'alliance_traces.jsonl', domain: 'expert-alliance', desc: '联盟处理轨迹' },
  { file: 'dispatcher_config.json', domain: 'expert-alliance', desc: '调度策略配置' },
  { file: 'learned_skills.json', domain: 'expert-alliance', desc: '学习技能' },
  { file: 'expert_capability_graph.json', domain: 'expert-graph', desc: '专家能力图' },
  { file: 'engine_bindings.json', domain: 'engine-kernel', desc: '引擎槽位绑定（切换引擎零代码改动）' },
  { file: 'engine_plugins.json', domain: 'engine-kernel', desc: '本地安装插件清单' },
  { file: 'engine_marketplace.json', domain: 'engine-kernel', desc: '云端商城注册表配置' },
  { file: 'caomei_templates.json', domain: 'ai-enhanced', desc: '内容模板' },
  { file: 'tasks.json', domain: 'tasks', desc: '任务数据' },
  { file: 'kb_documents.json', domain: 'kb', desc: '知识库文档' },
  { file: 'kb_categories.json', domain: 'kb', desc: '知识库分类' },
  { file: 'kb_versions.json', domain: 'kb', desc: '文档版本快照' },
  { file: 'kb_history.json', domain: 'kb', desc: '文档变更历史' },
  { file: 'automation.json', domain: 'auto-tasks', desc: '自动化任务' },
  { file: 'ultimate_reasoning_rules.json', domain: 'ai-ultimate', desc: '推理规则' }
];

// ============ 核心文档（项目智慧沉淀，关联业务域） ============
const DOCS = [
  { file: 'docs/standards/ai-native-architecture-standard.md', domain: 'engine-universe', desc: 'AINA-STD-001 架构规范（五公理+门禁）' },
  { file: 'docs/standards/engine-universe.md', domain: 'engine-universe', desc: '引擎宇宙图谱（17 引擎关联）' },
  { file: 'docs/standards/project-atlas.md', domain: 'atlas', desc: '项目全息图谱（无破窗验证 W1-W8）' },
  { file: 'docs/standards/engine-kernel.md', domain: 'engine-kernel', desc: '引擎内核（槽位契约+瞬间切换+三层商城+AI配置）' },
  { file: 'docs/architecture.md', domain: 'system', desc: '系统总体架构' },
  { file: 'docs/README.md', domain: 'system', desc: '文档索引' },
  { file: 'docs/GLOSSARY.md', domain: 'system', desc: '术语表' },
  { file: 'docs/modules/ai-engine-master-analysis.md', domain: 'ai-engine', desc: 'AI 引擎主分析（D 系列缺陷修复史）' },
  { file: 'docs/AI-UNIFIED-OPTIMIZATION-PLAN.md', domain: 'web-search', desc: 'AI 统一优化计划（含联网搜索设计）' },
  { file: 'docs/modules/ai-engine-master-analysis.md', domain: 'integration', desc: 'LLM 网关集成分析（多引擎接入）' },
  { file: 'docs/modules/ai-engine-master-analysis.md', domain: 'ai-ultimate', desc: '终极引擎分析（记忆与推理）' },
  { file: 'docs/modules/ai-flow-graph-design.md', domain: 'ai-engine', desc: '流程图谱设计（Rust/Node 对齐）' },
  { file: 'docs/modules/mathematical-foundation.md', domain: 'ai-integrated', desc: '数学基础（图公式推导）' },
  { file: 'docs/modules/infinite-dimension-optimization.md', domain: 'optimizer', desc: '无穷维度优化设计' },
  { file: 'docs/modules/algorithm-verification.md', domain: 'ai-enhanced', desc: '算法验证报告' },
  { file: 'docs/modules/automation-module.md', domain: 'orchestration', desc: '自动化模块设计' },
  { file: 'docs/modules/market-module.md', domain: 'browser-market', desc: '市场模块设计' },
  { file: 'docs/modules/local-artifact-agent.md', domain: 'artifacts', desc: '本地制品代理设计' },
  { file: 'docs/modules/PrimiFlow-设计蓝图.md', domain: 'auto-dev', desc: '自动开发引擎蓝图' },
  { file: 'docs/modules/business-process-flowcharts.md', domain: 'graph', desc: '业务流程图集（Mermaid）' },
  { file: 'docs/modules/business-process-flows.md', domain: 'graph', desc: '业务流程分析' },
  { file: 'docs/modules/专家联盟AI对话需求文档-V2.0-架构优化版.md', domain: 'expert-alliance', desc: '专家联盟 V2.0 需求' },
  { file: 'docs/modules/专家联盟V2.0-集成对齐分析报告.md', domain: 'expert-graph', desc: '集成对齐分析' },
  { file: 'docs/modules/xuanji-expert-alliance-fusion-flows.md', domain: 'expert-alliance', desc: '联盟融合流程' },
  { file: 'docs/modules/xuanji-expert-normalization.md', domain: 'expert-alliance', desc: '专家归一化设计' },
  { file: 'docs/对话开发系统-全维分析与业务流程图.md', domain: 'chat', desc: '对话系统全维分析' },
  { file: 'docs/DOC-NORMALIZATION-REPORT.md', domain: 'kb', desc: '文档归一化报告' },
  { file: 'docs/enterprise/01-requirements.md', domain: 'tasks', desc: '企业需求书' },
  { file: 'docs/enterprise/02-architecture.md', domain: 'services', desc: '企业架构' },
  { file: 'docs/enterprise/04-business-processing.md', domain: 'tasks', desc: '业务处理流程' },
  { file: 'docs/enterprise/08-全维自动化处理明确书.md', domain: 'auto-tasks', desc: '全维自动化明确书' },
  { file: 'docs/enterprise/12-RBAC审计全链路闭环验收报告.md', domain: 'security', desc: 'RBAC 审计验收' },
  { file: 'docs/enterprise/17-算子系统全维分析与归一化设计.md', domain: 'ai-platform', desc: '算子系统归一化设计' },
  { file: 'docs/specs/GR-STD-信息关联关系图开发规范-V1.0.md', domain: 'engine-universe', desc: '关联关系图开发规范' },
  { file: 'docs/specs/OUS-业务功能规划与架构数据关系分析.md', domain: 'modules-admin', desc: '架构数据关系分析' },
  { file: 'docs/specs/PT-Primi-架构规范-V1.0-完整版.md', domain: 'modules-admin', desc: 'Primi 架构规范' }
];

module.exports = { ALGORITHMS, DATA_ASSETS, DOCS };

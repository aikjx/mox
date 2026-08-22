'use strict';

/**
 * 意图模式表（单一真相源 · AINA A3）
 * ------------------------------------------------------------------
 * 全系统唯一 INTENT_PATTERNS 定义：
 *   - expert-alliance.js（编排层）意图路由
 *   - expert-alliance-engine.js 六阶段流水线 classifyIntent
 * 历史教训（A16）：两处各维护一份曾发生关键词漂移，故收口于此。
 * 本文件属 domain 层（R1）：零 IO、零引擎依赖，纯数据。
 */
const INTENT_PATTERNS = [
  { intent: 'algorithm', keywords: ['算法', '复杂度', '排序', '搜索', '动态规划', '贪心', '回溯', '分治', '递归', '时间复杂度', '空间复杂度', 'O(n)', 'O(log n)', '优化算法'] },
  { intent: 'architecture', keywords: ['架构', '系统设计', '微服务', '分布式', '高可用', '负载均衡', '服务治理', 'SOA', 'DDD', '分层架构', '组件图'] },
  { intent: 'data', keywords: ['数据建模', '数据库', 'ETL', '数据仓库', '数据治理', '数据质量', '主数据', 'OLAP', 'OLTP', 'Schema', '数据迁移'] },
  { intent: 'ai', keywords: ['机器学习', '深度学习', '神经网络', '大模型', 'LLM', 'RAG', 'Prompt', 'Transformer', 'CNN', 'RNN', '训练', '推理', '微调'] },
  { intent: 'workflow', keywords: ['BPMN', '工作流', '流程编排', '流程引擎', 'Activity', '网关', '服务任务', '用户任务', '定时器', '事件'] },
  { intent: 'operator', keywords: ['算子', '运算', '状态向量', '守恒律', '代数', '群论', '幺正', '组合算子', '算子代数'] },
  { intent: 'graph', keywords: ['图', '图谱', '节点', '边', '实体关系', '知识图谱', 'PageRank', '中心性', '社区发现', '最短路径', '图算法'] },
  { intent: 'security', keywords: ['安全', '加密', '认证', '授权', 'RBAC', 'OA', '审计', '合规', '渗透', '漏洞', '威胁', '等保'] },
  { intent: 'performance', keywords: ['性能', '优化', '瓶颈', '调优', '缓存', '索引', '并发', '吞吐量', '延迟', 'QPS', 'TPS'] },
  { intent: 'monitor', keywords: ['监控', '告警', '日志', '追踪', 'Metrics', 'Prometheus', 'Grafana', '链路', '可观测', 'SLA'] },
  { intent: 'market', keywords: ['商业', '市场', '用户画像', '推荐', '增长', '变现', '商业模式', '竞品', '用户行为'] },
  { intent: 'mcp', keywords: ['MCP', '协议', '工具调用', '上下文', 'Model Context', 'Server'] },
  { intent: 'automation', keywords: ['自动化', 'RPA', '智能体', 'Agent', '低代码', '无代码', '脚本', '机器人流程'] },
  { intent: 'requirement', keywords: ['需求', '用例', '用户故事', '需求分析', '需求追踪', '验收标准', '范围', ' stakeholders'] },
  { intent: 'fusion', keywords: ['融合', '璇玑', '治理', '全维', '双十四维', '归一化', '统一'] }
];

module.exports = { INTENT_PATTERNS };

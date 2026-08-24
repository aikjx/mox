'use strict';

/**
 * 意图模式表（单一真相源 · AINA A3）
 * ------------------------------------------------------------------
 * 全系统唯一 INTENT_PATTERNS 定义：
 *   - expert-alliance.js（编排层）意图路由
 *   - expert-alliance-engine.js 六阶段流水线 classifyIntent
 * 历史教训（A16）：两处各维护一份曾发生关键词漂移，故收口于此。
 * 本文件属 domain 层（R1）：零 IO、零引擎依赖，纯数据。
 * 关键词覆盖：中文 + 英文，支持双语意图识别（解决英文安全问题路由错误缺陷）。
 */
const INTENT_PATTERNS = [
  {
    intent: 'algorithm',
    keywords: [
      // 中文
      '算法', '复杂度', '排序', '搜索', '动态规划', '贪心', '回溯', '分治', '递归',
      '时间复杂度', '空间复杂度', 'O(n)', 'O(log n)', '优化算法', '哈希', 'hash',
      // 英文
      'algorithm', 'complexity', 'sort', 'search', 'dp', 'dynamic programming',
      'greedy', 'backtrack', 'divide and conquer', 'recursion', 'time complexity',
      'space complexity', 'big o', 'optimization', 'sorting', 'searching', 'hash'
    ]
  },
  {
    intent: 'architecture',
    keywords: [
      // 中文
      '架构', '系统设计', '微服务', '分布式', '高可用', '负载均衡', '服务治理',
      'SOA', 'DDD', '分层架构', '组件图',
      // 英文
      'architecture', 'system design', 'microservice', 'distributed', 'high availability',
      'load balance', 'service governance', 'soa', 'ddd', 'layered architecture',
      'component diagram', 'design pattern', 'scalability', 'availability'
    ]
  },
  {
    intent: 'data',
    keywords: [
      // 中文
      '数据建模', '数据库', 'ETL', '数据仓库', '数据治理', '数据质量', '主数据',
      'OLAP', 'OLTP', 'Schema', '数据迁移', 'SQL', 'NoSQL',
      // 英文
      'data modeling', 'database', 'etl', 'data warehouse', 'data governance',
      'data quality', 'master data', 'olap', 'oltp', 'schema', 'data migration',
      'sql', 'nosql', 'mysql', 'postgresql', 'mongodb', 'redis'
    ]
  },
  {
    intent: 'ai',
    keywords: [
      // 中文
      '机器学习', '深度学习', '神经网络', '大模型', 'LLM', 'RAG', 'Prompt',
      'Transformer', 'CNN', 'RNN', '训练', '推理', '微调', 'GPT', 'AI',
      // 英文
      'machine learning', 'deep learning', 'neural network', 'llm', 'large language model',
      'rag', 'retrieval augmented', 'prompt', 'transformer', 'cnn', 'rnn', 'training',
      'inference', 'fine-tuning', 'fine tuning', 'gpt', 'claude', 'gemini', 'ai', 'ml'
    ]
  },
  {
    intent: 'workflow',
    keywords: [
      // 中文
      'BPMN', '工作流', '流程编排', '流程引擎', 'Activity', '网关', '服务任务',
      '用户任务', '定时器', '事件',
      // 英文
      'bpmn', 'workflow', 'process orchestration', 'process engine', 'activity',
      'gateway', 'service task', 'user task', 'timer', 'event', 'pipeline', 'orchestration'
    ]
  },
  {
    intent: 'operator',
    keywords: [
      // 中文
      '算子', '运算', '状态向量', '守恒律', '代数', '群论', '幺正', '组合算子', '算子代数',
      // 英文
      'operator', 'operation', 'state vector', 'conservation law', 'algebra',
      'group theory', 'unitary', 'combinatorial operator', 'operator algebra', 'tensor'
    ]
  },
  {
    intent: 'graph',
    keywords: [
      // 中文
      '图', '图谱', '节点', '边', '实体关系', '知识图谱', 'PageRank', '中心性',
      '社区发现', '最短路径', '图算法',
      // 英文
      'graph', 'knowledge graph', 'node', 'edge', 'entity relation', 'pagerank',
      'centrality', 'community detection', 'shortest path', 'graph algorithm',
      'network', 'vertex', 'neural graph'
    ]
  },
  {
    intent: 'security',
    keywords: [
      // 中文
      '安全', '加密', '认证', '授权', 'RBAC', 'OA', '审计', '合规', '渗透', '漏洞',
      '威胁', '等保', '注入', 'XSS', 'CSRF', '攻击', '防护', '越权', '防火墙',
      'WAF', '脱敏', '密钥泄露', '防重放', '风控', '密码', '加盐', '哈希', '签名',
      // 中文复合词（与英文复合词对称，防止 SQL注入/XSS攻击 等被单字误平局判为 data/ai 意图）
      'SQL注入', 'XSS攻击', 'CSRF攻击', '注入攻击', '暴力破解', '重放攻击',
      // 英文
      'security', 'encryption', 'authentication', 'authorization', 'rbac', 'audit',
      'compliance', 'penetration', 'vulnerability', 'threat', 'injection', 'xss', 'csrf',
      'attack', 'protection', 'privilege', 'firewall', 'waf', 'masking', 'key leak',
      'replay protection', 'risk control', 'password', 'salt', 'hashing', 'hash',
      'signature', 'oauth', 'jwt', 'ssl', 'tls', 'cryptography', 'cipher',
      'sql injection', 'xss attack', 'csrf attack', 'ddos', 'mitm', 'man-in-the-middle',
      'brute force', 'rainbow table', 'bcrypt', 'scrypt', 'argon2', 'pbkdf2'
    ]
  },
  {
    intent: 'performance',
    keywords: [
      // 中文
      '性能', '优化', '瓶颈', '调优', '缓存', '索引', '并发', '吞吐量', '延迟', 'QPS', 'TPS',
      // 英文
      'performance', 'optimization', 'bottleneck', 'tuning', 'cache', 'index',
      'concurrency', 'throughput', 'latency', 'qps', 'tps', 'benchmark', 'profiling'
    ]
  },
  {
    intent: 'monitor',
    keywords: [
      // 中文
      '监控', '告警', '日志', '追踪', 'Metrics', 'Prometheus', 'Grafana', '链路', '可观测', 'SLA',
      // 英文
      'monitoring', 'alert', 'logging', 'tracing', 'metrics', 'prometheus', 'grafana',
      'trace', 'observability', 'sla', 'telemetry', 'apm'
    ]
  },
  {
    intent: 'market',
    keywords: [
      // 中文
      '商业', '市场', '用户画像', '推荐', '增长', '变现', '商业模式', '竞品', '用户行为',
      // 英文
      'business', 'market', 'user profile', 'recommendation', 'growth',
      'monetization', 'business model', 'competitor', 'user behavior', 'marketing'
    ]
  },
  {
    intent: 'mcp',
    keywords: [
      // 中文
      'MCP', '协议', '工具调用', '上下文', 'Model Context', 'Server',
      // 英文
      'mcp', 'model context protocol', 'protocol', 'tool call', 'tool use', 'context',
      'server', 'client', 'tool integration'
    ]
  },
  {
    intent: 'automation',
    keywords: [
      // 中文
      '自动化', 'RPA', '智能体', 'Agent', '低代码', '无代码', '脚本', '机器人流程',
      // 英文
      'automation', 'rpa', 'agent', 'ai agent', 'low code', 'no code', 'script',
      'robotic process', 'bot', 'auto'
    ]
  },
  {
    intent: 'requirement',
    keywords: [
      // 中文
      '需求', '用例', '用户故事', '需求分析', '需求追踪', '验收标准', '范围', ' stakeholders',
      // 英文
      'requirement', 'use case', 'user story', 'requirement analysis', 'traceability',
      'acceptance criteria', 'scope', 'stakeholder'
    ]
  },
  {
    intent: 'fusion',
    keywords: [
      // 中文（专家联盟域触发词：会诊/联盟/组队/会商 = expert capability 路由专用词）
      '融合', '璇玑', '治理', '全维', '双十四维', '归一化', '统一',
      '专家', '会诊', '联盟', '专家联盟', '多专家', '协作会商', '团队讨论', '组队',
      // 英文
      'fusion', 'xuanji', 'governance', 'full dimension', 'normalization', 'unified',
      'integration', 'unification',
      'expert alliance', 'multi-expert', 'joint consultation', 'panel discussion'
    ]
  }
];

module.exports = { INTENT_PATTERNS };

/**
 * 工作台 Mock 数据
 * 用于 API 不可用时的降级展示
 */

export function getMockExperts() {
  return [
    { id: 'exp-001', name: '林算法', type: 'algorithm', status: 'active', capabilities: ['动态规划', '图算法', '复杂度分析'], metrics: { total_consults: 1286, success_rate: 0.97 } },
    { id: 'exp-002', name: '陈架构', type: 'architecture', status: 'active', capabilities: ['微服务', 'DDD', '高可用设计'], metrics: { total_consults: 2103, success_rate: 0.95 } },
    { id: 'exp-003', name: '王数据', type: 'data', status: 'active', capabilities: ['数据建模', 'ETL', '数据治理'], metrics: { total_consults: 856, success_rate: 0.98 } },
    { id: 'exp-004', name: '张AI', type: 'ai', status: 'active', capabilities: ['LLM', 'RAG', 'Prompt工程'], metrics: { total_consults: 3241, success_rate: 0.94 } },
    { id: 'exp-005', name: '李工作流', type: 'workflow', status: 'busy', capabilities: ['流程编排', 'BPM', '自动化'], metrics: { total_consults: 678, success_rate: 0.96 } },
    { id: 'exp-006', name: '赵图谱', type: 'graph', status: 'active', capabilities: ['图数据库', 'Cypher', '图计算'], metrics: { total_consults: 945, success_rate: 0.93 } },
    { id: 'exp-007', name: '孙安全', type: 'security', status: 'active', capabilities: ['渗透测试', '安全审计', '合规'], metrics: { total_consults: 523, success_rate: 0.99 } },
    { id: 'exp-008', name: '周性能', type: 'performance', status: 'idle', capabilities: ['性能调优', '压测', '缓存策略'], metrics: { total_consults: 712, success_rate: 0.92 } }
  ]
}

export function getMockSessions() {
  return [
    { id: 'sess-001', title: '架构优化方案讨论', expert_count: 3, mode: 'debate', created_at: Date.now() - 600000, updated_at: Date.now() - 600000 },
    { id: 'sess-002', title: '知识图谱融合策略', expert_count: 2, mode: 'multi', created_at: Date.now() - 3600000, updated_at: Date.now() - 3600000 },
    { id: 'sess-003', title: '性能瓶颈分析', expert_count: 2, mode: 'single', created_at: Date.now() - 86400000, updated_at: Date.now() - 86400000 }
  ]
}

export function getMockDocs() {
  return [
    { id: 'doc-001', title: '专家联盟架构设计 V3.0', type: 'pdf', size: 2516582, category_id: 'cat-arch', updated_at: Date.now() - 600000, tags: ['架构设计', '专家系统', '微服务'], graph_linked: true },
    { id: 'doc-002', title: '知识图谱域架构规范', type: 'doc', size: 1887436, category_id: 'cat-arch', updated_at: Date.now() - 7200000, tags: ['知识图谱', '架构规范'], graph_linked: true },
    { id: 'doc-003', title: '算法归一化设计方案', type: 'doc', size: 978944, category_id: 'cat-arch', updated_at: Date.now() - 86400000, tags: ['算法', '归一化'], graph_linked: false },
    { id: 'doc-004', title: '云存储域接口定义', type: 'api', size: 634880, category_id: 'cat-arch', updated_at: Date.now() - 259200000, tags: ['云存储', 'API'], graph_linked: true },
    { id: 'doc-005', title: '中心性算法对比研究', type: 'pdf', size: 1258291, category_id: 'cat-algo', updated_at: Date.now() - 86400000, tags: ['图算法', '中心性', '研究'], graph_linked: true },
    { id: 'doc-006', title: '社区发现算法优化', type: 'doc', size: 911360, category_id: 'cat-algo', updated_at: Date.now() - 259200000, tags: ['图算法', '社区发现'], graph_linked: false },
    { id: 'doc-007', title: 'RAG 检索增强生成实践', type: 'pdf', size: 2097152, category_id: 'cat-ai', updated_at: Date.now() - 172800000, tags: ['RAG', 'LLM', 'AI'], graph_linked: true },
    { id: 'doc-008', title: '向量检索性能调优指南', type: 'doc', size: 734003, category_id: 'cat-algo', updated_at: Date.now() - 432000000, tags: ['向量检索', '性能优化'], graph_linked: false }
  ]
}

export function getMockCategories() {
  return [
    { id: 'cat-arch', name: '架构设计文档', count: 4 },
    { id: 'cat-algo', name: '算法研究', count: 3 },
    { id: 'cat-ai', name: 'AI 模型', count: 1 },
    { id: 'cat-data', name: '数据规范', count: 0 }
  ]
}

export function getMockTags() {
  const tags = [
    { name: '架构设计', count: 15 }, { name: '知识图谱', count: 12 }, { name: '图算法', count: 10 },
    { name: '专家系统', count: 14 }, { name: '微服务', count: 8 }, { name: 'RAG', count: 6 },
    { name: '向量检索', count: 5 }, { name: '性能优化', count: 9 }, { name: '模块化', count: 7 },
    { name: '归一化', count: 4 }, { name: '协作模式', count: 3 }, { name: '知识融合', count: 6 }
  ]
  return tags.map(t => ({ ...t, fontSize: 12 + Math.min(t.count, 20) * 0.4 }))
}

export function getMockVersions() {
  return [
    { id: 'v1', version: '3.0', created_at: Date.now() - 3600000, author: '陈架构', action: '重大版本更新' },
    { id: 'v2', version: '2.5', created_at: Date.now() - 86400000, author: '林算法', action: '新增算法章节' },
    { id: 'v3', version: '2.1', created_at: Date.now() - 172800000, author: '王数据', action: '修订数据模型' },
    { id: 'v4', version: '2.0', created_at: Date.now() - 604800000, author: '陈架构', action: '重构架构设计' }
  ]
}

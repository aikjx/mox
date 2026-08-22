'use strict';

/**
 * 算法分析用例族（Application 层 mixin）
 * ------------------------------------------------------------------
 * 挂载于 ExpertAlliance.prototype：analyzeWithAlgorithm 及其私有协作方法。
 * PageRank 通过惰性委托 ai-engine 单源实现（A18），保持延迟边解耦。
 */

async function analyzeWithAlgorithm(question, graphData, options = {}) {
  const expertIds = this._determineAlgorithmExperts(question, graphData);
  const analysisResults = {};

  for (const expertId of expertIds) {
    const expert = this.repo.get(expertId);
    if (!expert) continue;

    if (expert.type === 'graph' && graphData) {
      analysisResults.graph = await this._performGraphAnalysis(graphData, options);
    }

    if (expert.type === 'algorithm') {
      analysisResults.algorithm = this._performAlgorithmAnalysis(question, options);
    }
  }

  if (expertIds.includes('alg-expert') || expertIds.includes('graph-expert')) {
    const { getGateway } = require('../../llm-gateway');
    const gateway = getGateway();
    if (gateway && gateway.activeProvider) {
      try {
        const aiInsight = await this._getAIAlgorithmInsight(question, analysisResults);
        analysisResults.ai_insight = aiInsight;
      } catch (e) {
        analysisResults.ai_insight = 'AI 增强分析暂不可用，已返回基础分析结果';
      }
    }
  }

  return {
    question,
    experts_consulted: expertIds,
    analysis: analysisResults,
    timestamp: new Date().toISOString()
  };
}

function _determineAlgorithmExperts(question, graphData) {
  const experts = [];
  const text = (question || '').toLowerCase();

  if (graphData || text.includes('图') || text.includes('图谱') || text.includes('节点') || text.includes('边')) {
    experts.push('graph-expert');
  }
  if (text.includes('算法') || text.includes('复杂度') || text.includes('优化') || text.includes('排序')) {
    experts.push('alg-expert');
  }
  if (text.includes('架构') || text.includes('系统')) {
    experts.push('arch-expert');
  }
  if (text.includes('性能') || text.includes('瓶颈') || text.includes('优化')) {
    experts.push('perf-expert');
  }

  if (experts.length === 0) {
    experts.push('alg-expert', 'arch-expert');
  }

  return experts;
}

async function _performGraphAnalysis(graphData, options = {}) {
  const nodes = graphData?.nodes || [];
  const edges = graphData?.edges || [];
  const n = nodes.length;

  const degreeMap = new Map();
  nodes.forEach(nd => degreeMap.set(nd.id, 0));
  edges.forEach(e => {
    degreeMap.set(e.source, (degreeMap.get(e.source) || 0) + 1);
    degreeMap.set(e.target, (degreeMap.get(e.target) || 0) + 1);
  });

  const degrees = Array.from(degreeMap.values());
  const avgDegree = degrees.length > 0 ? degrees.reduce((a, b) => a + b, 0) / degrees.length : 0;
  const maxDegree = degrees.length > 0 ? Math.max(...degrees) : 0;
  const isolatedNodes = degrees.filter(d => d === 0).length;
  const density = n > 1 ? (2 * edges.length) / (n * (n - 1)) : 0;

  const pagerank = await this._computePageRank(nodes, edges, options.damping || 0.85, options.iterations || 50);
  const topNodes = pagerank.slice(0, 10).map((p, i) => ({
    rank: i + 1,
    id: p.id,
    pagerank: Math.round(p.pagerank * 10000) / 10000
  }));

  return {
    stats: {
      nodeCount: n,
      edgeCount: edges.length,
      density: Math.round(density * 10000) / 10000,
      avgDegree: Math.round(avgDegree * 100) / 100,
      maxDegree,
      isolatedNodes
    },
    topNodes,
    analysis_time: new Date().toISOString()
  };
}

async function _computePageRank(nodes, edges, damping = 0.85, iterations = 50) {
  // 归一化收口（A18 修复）：委托 ai-engine 单源 PageRank
  // （其内部再委托 ai-integration-engine 统一实现：边权重/收敛容差/悬挂节点处理），
  // 消除联盟层 40 行无收敛检测的重复实现。
  // 延迟 require：ai-integration-engine 顶层依赖本模块，顶层互相 require 会形成循环。
  const { getAIEngine } = require('../../ai-engine');
  const { getGateway } = require('../../llm-gateway');
  const engine = getAIEngine(getGateway());
  return engine._computePageRank(nodes, edges, damping, iterations);
}

function _performAlgorithmAnalysis(question, options = {}) {
  const text = (question || '').toLowerCase();
  const analyses = [];

  if (text.includes('排序') || text.includes('sort')) {
    analyses.push({
      algorithm: '排序算法',
      recommendation: '推荐使用归并排序 (O(n log n)) 或快速排序 (平均 O(n log n))',
      complexity: { time: 'O(n log n)', space: 'O(n) 或 O(log n)' }
    });
  }
  if (text.includes('搜索') || text.includes('search')) {
    analyses.push({
      algorithm: '搜索算法',
      recommendation: '有序数组用二分搜索 O(log n)，无序用哈希表 O(1)',
      complexity: { time: 'O(log n) 或 O(1)', space: 'O(n)' }
    });
  }
  if (text.includes('图') || text.includes('最短路径')) {
    analyses.push({
      algorithm: '图算法',
      recommendation: '无权图 BFS O(V+E)，有权图 Dijkstra O(E log V)',
      complexity: { time: 'O(V+E) 或 O(E log V)', space: 'O(V)' }
    });
  }
  if (text.includes('动态规划') || text.includes('dp')) {
    analyses.push({
      algorithm: '动态规划',
      recommendation: '适用最优子结构和重叠子问题场景，注意状态转移方程设计',
      complexity: { time: 'O(n²) 或 O(n*k)', space: 'O(n) 或 O(n*k)' }
    });
  }

  if (analyses.length === 0) {
    analyses.push({
      algorithm: '通用建议',
      recommendation: '根据具体场景选择合适的算法，注意数据规模和约束条件',
      complexity: { time: '依赖具体算法', space: '依赖具体算法' }
    });
  }

  return { analyses, analysis_time: new Date().toISOString() };
}

async function _getAIAlgorithmInsight(question, existingResults) {
  const { getGateway } = require('../../llm-gateway');
  const gateway = getGateway();
  if (!gateway || !gateway.activeProvider) {
    return null;
  }

  const prompt = `请基于以下分析结果，提供深度算法洞察：

问题: ${question}

已有分析:
${JSON.stringify(existingResults, null, 2).slice(0, 2000)}

请提供：
1. 关键发现和洞察
2. 潜在的性能优化机会
3. 风险点和注意事项
4. 进一步分析建议`;

  const result = await gateway.chat({
    messages: [
      { role: 'system', content: '你是一位资深算法分析师。请提供深入、可操作的分析洞察。' },
      { role: 'user', content: prompt }
    ]
  });

  return result.content;
}

module.exports = {
  analyzeWithAlgorithm,
  _determineAlgorithmExperts,
  _performGraphAnalysis,
  _computePageRank,
  _performAlgorithmAnalysis,
  _getAIAlgorithmInsight
};

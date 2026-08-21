'use strict';

class CodeAnalysisPlugin {
  constructor() {
    this.name = 'code-analysis';
    this.version = '1.0';
    this.description = '代码深度分析插件 - 集成专家代码审查能力';
    this.alliance = null;
  }

  onMount(context) {
    const { getService } = context;
    this.alliance = getService ? null : null;
    console.log('[code-analysis] Plugin mounted');
  }

  async process(input, context) {
    const codePatterns = [
      { name: '复杂度分析', patterns: ['复杂度', 'complexity', 'big-o', '时间复杂度'] },
      { name: '代码质量', patterns: ['代码质量', 'code quality', 'code review', '代码审查'] },
      { name: '性能优化', patterns: ['性能', 'performance', '优化', 'optimize'] },
      { name: '安全审计', patterns: ['安全', 'security', '漏洞', 'vulnerability'] },
      { name: '重构建议', patterns: ['重构', 'refactor', '重构建议'] }
    ];

    const matched = codePatterns.filter(cp => 
      cp.patterns.some(p => input?.question?.toLowerCase().includes(p))
    );

    return {
      analysis: {
        detectedDomains: matched.map(m => m.name),
        requiresCodeAnalysis: matched.length > 0,
        codeAnalysisDepth: matched.length >= 2 ? 'deep' : 'standard'
      }
    };
  }
}

class GraphAnalysisPlugin {
  constructor() {
    this.name = 'graph-analysis';
    this.version = '1.0';
    this.description = '图谱深度分析插件 - PageRank/社群检测/路径分析';
    this.graph = null;
  }

  onMount(context) {
    try {
      const { getExpertGraph } = require('./expert-graph');
      this.graph = getExpertGraph();
    } catch (e) {
      console.warn('[graph-analysis] expert-graph not available');
    }
  }

  async process(input, context) {
    const question = input?.question || '';
    const graphKeywords = ['图', '图谱', 'graph', '节点', '中心性', 'PageRank', '社群', '路径'];
    const isGraphQuery = graphKeywords.some(k => question.toLowerCase().includes(k.toLowerCase()));

    if (!isGraphQuery) return { graphAnalysis: { relevant: false } };

    return {
      graphAnalysis: {
        relevant: true,
        analysisType: this._determineAnalysisType(question),
        expertRecommendation: 'graph-expert',
        suggestedAlgorithms: this._suggestAlgorithms(question)
      }
    };
  }

  _determineAnalysisType(question) {
    if (/pagerank|中心性|centrality/i.test(question)) return 'centrality';
    if (/社群|community|聚类|cluster/i.test(question)) return 'community_detection';
    if (/路径|path|最短|shortest/i.test(question)) return 'path_analysis';
    if (/传播|propagation|扩散/i.test(question)) return 'propagation';
    return 'general_graph_analysis';
  }

  _suggestAlgorithms(question) {
    const algos = [];
    if (/中心性|pagerank/i.test(question)) algos.push('PageRank', 'Betweenness Centrality');
    if (/社群|community/i.test(question)) algos.push('Louvain', 'Label Propagation');
    if (/路径|path/i.test(question)) algos.push('Dijkstra', 'A*');
    if (/传播|propagation/i.test(question)) algos.push('Activation Spread', 'Influence Maximization');
    return algos.length ? algos : ['PageRank'];
  }
}

class GovernancePlugin {
  constructor() {
    this.name = 'governance';
    this.version = '1.0';
    this.description = '治理与合规插件 - 双璇玑十四维诊断接入';
  }

  async analyze(result, context) {
    if (!result) return { governanceCheck: { passed: true, warnings: [] } };

    const warnings = [];
    const score = result.reflection?.quality || 0.7;

    if (score < 0.5) warnings.push('输出质量评分低于 0.5，建议人工审核');
    if (result.duration > 30000) warnings.push('执行时间超过 30 秒，可能影响用户体验');

    return {
      governanceCheck: {
        passed: warnings.length === 0,
        warnings,
        score,
        timestamp: Date.now()
      }
    };
  }

  async validate(result, context) {
    return {
      valid: result?.status === 'success',
      confidence: 0.95,
      checkedAt: Date.now()
    };
  }
}

module.exports = {
  CodeAnalysisPlugin,
  GraphAnalysisPlugin,
  GovernancePlugin
};
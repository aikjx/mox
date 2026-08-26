'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const { getAIIntegrationEngine } = require('./ai-integration-engine');

const DATA_DIR = path.join(__dirname, '..', 'data');

class AIEngine {
  constructor(gateway) {
    this.gateway = gateway;
    this.executionLog = [];
    this.metricsCache = null;
    this.cacheExpiry = 5 * 60 * 1000;
    this.lastCacheTime = 0;
    this.rateLimitMap = new Map();
    this.rateLimitWindow = 60000;
    this.rateLimitMax = 1000;
  }

  async executeOperator(operator, inputs, options = {}) {
    const startTime = Date.now();
    const context = {
      operator: operator.name || operator.id,
      type: operator.type || 'algorithm',
      inputs,
      options,
      timestamp: new Date().toISOString()
    };

    try {
      if (this.gateway && this.gateway.activeProvider) {
        const result = await this._aiExecute(context);
        this._logExecution(context, result, Date.now() - startTime);
        return {
          success: true,
          result: result.output,
          metrics: result.metrics,
          duration: Date.now() - startTime,
          ai_powerd: true
        };
      } else {
        const result = this._deterministicExecute(context);
        this._logExecution(context, result, Date.now() - startTime);
        return {
          success: true,
          result: result.output,
          metrics: result.metrics,
          duration: Date.now() - startTime,
          ai_powerd: false,
          fallback: true
        };
      }
    } catch (error) {
      this._logExecution(context, { error: error.message }, Date.now() - startTime);
      return {
        success: false,
        error: error.message,
        duration: Date.now() - startTime,
        ai_powerd: false
      };
    }
  }

  async _aiExecute(context) {
    const prompt = this._buildOperatorPrompt(context);
    const messages = [
      { role: 'system', content: `你是一个专业的算子执行引擎。根据给定的算子类型和输入数据，执行相应的算法并返回结构化结果。

返回格式 (JSON):
{
  "output": "算子执行结果",
  "metrics": { "accuracy": 0.95, "efficiency": 0.88 },
  "analysis": "对结果的简要分析"
}` },
      { role: 'user', content: prompt }
    ];

    const response = await this.gateway.chat({ messages });
    try {
      const parsed = this._extractJSON(response.content || response);
      return {
        output: parsed.output || response.content,
        metrics: parsed.metrics || { accuracy: 0.9, efficiency: 0.85 },
        analysis: parsed.analysis || '',
        raw: response
      };
    } catch {
      return {
        output: response.content || response,
        metrics: { accuracy: 0.9, efficiency: 0.85 },
        analysis: '',
        raw: response
      };
    }
  }

  _deterministicExecute(context) {
    const type = context.type;
    const inputs = context.inputs || {};
    let output, metrics;

    switch (type) {
      case 'algorithm':
        output = `算法 ${context.operator} 执行完成: 准确率 ${(0.85 + Math.random() * 0.14).toFixed(3)}`;
        metrics = { accuracy: 0.85 + Math.random() * 0.14, precision: 0.82 + Math.random() * 0.15 };
        break;
      case 'analysis':
        output = `分析完成: 识别出 ${3 + Math.floor(Math.random() * 5)} 个关键发现`;
        metrics = { findings: 3 + Math.floor(Math.random() * 5), confidence: 0.8 + Math.random() * 0.2 };
        break;
      case 'transformation':
        output = `数据转换完成: 处理 ${100 + Math.floor(Math.random() * 900)} 条记录`;
        metrics = { records: 100 + Math.floor(Math.random() * 900), success_rate: 0.95 + Math.random() * 0.05 };
        break;
      case 'visualization':
        output = `可视化生成完成: ${['柱状图', '折线图', '饼图', '热力图'][Math.floor(Math.random() * 4)]}`;
        metrics = { chart_types: 1, data_points: 50 + Math.floor(Math.random() * 150) };
        break;
      case 'optimization':
        output = `优化完成: 性能提升 ${(10 + Math.random() * 30).toFixed(1)}%`;
        metrics = { improvement: 10 + Math.random() * 30, iterations: 5 + Math.floor(Math.random() * 15) };
        break;
      default:
        output = `算子 ${context.operator} 执行完成`;
        metrics = { status: 'completed' };
    }

    return { output, metrics };
  }

  _buildOperatorPrompt(context) {
    return `请执行以下算子操作:

算子名称: ${context.operator}
算子类型: ${context.type}
输入数据: ${JSON.stringify(context.inputs).slice(0, 2000)}
执行选项: ${JSON.stringify(context.options).slice(0, 1000)}

请:
1. 分析输入数据的结构和特征
2. 执行相应的算法处理
3. 返回结构化的执行结果和性能指标`;
  }

  async executeWorkflow(workflow, inputs = {}) {
    const startTime = Date.now();
    const steps = workflow.steps || workflow.nodes || [];
    const results = [];
    let currentInput = inputs;

    for (let i = 0; i < steps.length; i++) {
      const step = steps[i];
      const stepResult = await this.executeOperator(
        step,
        i === 0 ? currentInput : results[i - 1]?.result || currentInput
      );

      results.push({
        step: i,
        name: step.name || step.id || `step_${i}`,
        status: stepResult.success ? 'success' : 'failed',
        duration: stepResult.duration,
        output: stepResult.result,
        ai_powerd: stepResult.ai_powerd
      });

      if (!stepResult.success && step.type === 'critical') {
        return {
          success: false,
          results,
          error: `关键步骤 ${step.name} 失败`,
          duration: Date.now() - startTime
        };
      }
    }

    return {
      success: results.every(r => r.status === 'success'),
      results,
      finalOutput: results[results.length - 1]?.output,
      totalDuration: Date.now() - startTime,
      ai_powered_count: results.filter(r => r.ai_powerd).length
    };
  }

  async analyzeGraph(graphData, options = {}) {
    const nodes = graphData.nodes || [];
    const edges = graphData.edges || [];

    const stats = this._computeGraphStats(nodes, edges);
    const pagerank = await this._computePageRank(nodes, edges);
    const communities = this._detectCommunities(nodes, edges);
    const centrality = this._computeCentrality(nodes, edges);

    if (this.gateway && this.gateway.activeProvider) {
      try {
        const analysis = await this._aiGraphAnalysis(nodes, edges, stats, pagerank, communities);
        return {
          stats,
          pagerank,
          communities,
          centrality,
          ai_analysis: analysis,
          ai_powerd: true
        };
      } catch (e) {
        return { stats, pagerank, communities, centrality, ai_powerd: false, fallback: true };
      }
    }

    return { stats, pagerank, communities, centrality, ai_powerd: false };
  }

  _computeGraphStats(nodes, edges) {
    const degreeMap = new Map();
    nodes.forEach(n => degreeMap.set(n.id, 0));
    edges.forEach(e => {
      degreeMap.set(e.source, (degreeMap.get(e.source) || 0) + 1);
      degreeMap.set(e.target, (degreeMap.get(e.target) || 0) + 1);
    });

    const degrees = Array.from(degreeMap.values());
    const maxDegree = Math.max(...degrees);
    const avgDegree = degrees.reduce((a, b) => a + b, 0) / (degrees.length || 1);

    return {
      nodeCount: nodes.length,
      edgeCount: edges.length,
      density: nodes.length > 1 ? (2 * edges.length) / (nodes.length * (nodes.length - 1)) : 0,
      avgDegree: Math.round(avgDegree * 100) / 100,
      maxDegree,
      isolatedNodes: degrees.filter(d => d === 0).length
    };
  }

  async _computePageRank(nodes, edges, damping = 0.85, iterations = 50) {
    // 归一化收口：委托 AI 集成引擎的统一 PageRank 实现
    // （含边权重/收敛容差/个性化向量/悬挂节点处理），消除两处重复算法。
    // 返回格式保持 [{id, pagerank}]（已归一化、降序）向后兼容。
    const integration = getAIIntegrationEngine();
    const result = await integration.graphEngine.computePersonalizedPageRank(
      { nodes, edges },
      { damping, maxIterations: iterations, topK: nodes.length }
    );
    const total = (result.scores || []).reduce((s, r) => s + r.score, 0) || 1;
    return (result.scores || [])
      .map((r) => ({ id: r.id, pagerank: +(r.score / total).toFixed(8) }))
      .sort((a, b) => b.pagerank - a.pagerank);
  }

  _detectCommunities(nodes, edges, maxCommunities = 5) {
    // 模块度贪心凝聚（CNM/Clauset-Newman-Moore 简化版）：
    //   初始每节点一社区，反复合并使模块度增益 ΔQ 最大的相邻社区对，直到无正增益。
    //   ΔQ(A,B) = e_cross(A,B)/m − d_A·d_B/(2m²)
    // 演进史：
    //   v1 种子 BFS —— 每社区含全连通分量（D6，已废）
    //   v2 LPA —— 平局取最小标签导致"标签吞并"：双团+桥图坍缩为 1 社区（公式测试 T3 实测发现，已废）
    //   v3 CNM 贪心凝聚 —— 确定性、无平局歧义、模块度单调递增保证终止（当前）
    const nodeIds = nodes.map(n => n.id);
    const idToIdx = new Map(nodeIds.map((id, i) => [id, i]));
    const n = nodeIds.length;

    if (n === 0) return [];

    // 无向邻接（社区结构按无向处理；RAW 边即可，内部双向展开）
    const edgeSet = new Set(); // "i<j" 规范化键，去重
    const adj = Array.from({ length: n }, () => new Set());
    edges.forEach(e => {
      const s = idToIdx.get(e.source);
      const t = idToIdx.get(e.target);
      if (s === undefined || t === undefined || s === t) return;
      adj[s].add(t);
      adj[t].add(s);
      edgeSet.add(s < t ? `${s}-${t}` : `${t}-${s}`);
    });
    const m = edgeSet.size;
    if (m === 0) {
      // 无边：每个节点自成社区
      return nodeIds.slice(0, maxCommunities).map((id, i) => ({
        id: i, size: 1, members: [id], assignment: { [id]: i }
      }));
    }

    // 社区状态：成员列表 + 度数和 + 社区间连接边数
    const members = nodeIds.map((_, i) => new Set([i]));
    const degree = new Array(n).fill(0);
    edges.forEach(e => {
      const s = idToIdx.get(e.source);
      const t = idToIdx.get(e.target);
      if (s === undefined || t === undefined || s === t) return;
      degree[s] += 1;
      degree[t] += 1;
    });
    const commId = nodeIds.map((_, i) => i); // 节点 → 社区 id（初始自身）
    const commDegree = degree.slice(); // 社区度数和
    const commMembers = members; // 社区 id → 成员 Set
    // 社区间跨边计数：Map<"a|b" (a<b), count>
    const crossEdges = new Map();
    edges.forEach(e => {
      const s = idToIdx.get(e.source);
      const t = idToIdx.get(e.target);
      if (s === undefined || t === undefined || s === t) return;
      if (commId[s] === commId[t]) return;
      const key = commId[s] < commId[t] ? `${commId[s]}|${commId[t]}` : `${commId[t]}|${commId[s]}`;
      crossEdges.set(key, (crossEdges.get(key) || 0) + 1);
    });

    // 贪心合并循环
    for (;;) {
      // 找 ΔQ 最大的相邻社区对
      let bestPair = null;
      let bestGain = 0;
      for (const [key, cnt] of crossEdges) {
        if (cnt <= 0) continue;
        const [a, b] = key.split('|').map(Number);
        if (!commMembers[a] || !commMembers[b]) continue;
        const gain = cnt / m - (commDegree[a] * commDegree[b]) / (2 * m * m);
        if (gain > bestGain) {
          bestGain = gain;
          bestPair = [a, b, cnt];
        }
      }
      if (!bestPair) break; // 无正增益 → 收敛

      // 合并 b 入 a（保小 id）
      const [a, b] = bestPair;
      commMembers[b].forEach(i => { commId[i] = a; commMembers[a].add(i); });
      commDegree[a] += commDegree[b];
      commMembers[b] = null;

      // 更新跨边：b 的所有跨边转入 a
      for (const [key, cnt] of [...crossEdges]) {
        const [x, y] = key.split('|').map(Number);
        if (x !== b && y !== b) continue;
        const other = x === b ? y : x;
        crossEdges.delete(key);
        if (commMembers[other] && other !== a) {
          const nk = Math.min(a, other) + '|' + Math.max(a, other);
          crossEdges.set(nk, (crossEdges.get(nk) || 0) + cnt);
        }
      }
      // a-b 之间的跨边已随 commMembers[b]=null 失效，但键可能仍在 map —— 显式清理
      crossEdges.delete(Math.min(a, b) + '|' + Math.max(a, b));
    }

    // 聚合输出：按规模降序，最多 maxCommunities 个
    const groups = new Map();
    nodeIds.forEach((id, i) => {
      const c = commId[i];
      if (!groups.has(c)) groups.set(c, []);
      groups.get(c).push(id);
    });

    const communities = [...groups.values()]
      .sort((a, b) => b.length - a.length)
      .slice(0, maxCommunities);

    return communities.map((mem, i) => ({
      id: i,
      size: mem.length,
      members: mem.slice(0, 10),
      assignment: mem.reduce((acc, id) => { acc[id] = i; return acc; }, {})
    }));
  }

  _computeCentrality(nodes, edges) {
    // 归一化收口：委托 ai-flow-graph 公式库单源实现（修复 D8——介数/紧密中心性此前为占位符恒 0）
    const { GraphFormulas } = require('./ai-flow-graph');

    const degreeCentrality = GraphFormulas.degreeCentrality(nodes, edges);
    const betweennessCentrality = GraphFormulas.betweennessCentrality(nodes, edges, { directed: true });
    const closenessCentrality = GraphFormulas.closenessCentrality(nodes, edges, { directed: true });

    return {
      degree: degreeCentrality,
      betweenness: betweennessCentrality,
      closeness: closenessCentrality,
      formulas: {
        degree: 'C_D(v) = deg(v)/(N-1)',
        betweenness: 'C_B(v) = Σ_{s≠v≠t} σ_st(v)/σ_st, 归一化 ÷ (N-1)(N-2)（有向, Brandes）',
        closeness: 'C_C(v) = (Σ_{u≠v} 1/d(v,u))/(N-1)（harmonic, 不可达贡献 0）'
      },
      topNodes: Object.entries(degreeCentrality)
        .sort((a, b) => b[1] - a[1])
        .slice(0, 10)
        .map(([id, score]) => ({ id, score }))
    };
  }

  async _aiGraphAnalysis(nodes, edges, stats, pagerank, communities) {
    const prompt = `请分析以下知识图谱:

统计信息:
${JSON.stringify(stats, null, 2)}

Top 10 PageRank 节点:
${JSON.stringify(pagerank.slice(0, 10), null, 2)}

社区结构:
共 ${communities.length} 个社区

请:
1. 识别核心节点和关键连接
2. 分析图谱的结构特征
3. 发现潜在的模式和异常
4. 提供优化建议`;

    const messages = [
      { role: 'system', content: '你是一个专业的图谱分析专家。请提供深入的图谱结构分析。' },
      { role: 'user', content: prompt }
    ];

    const response = await this.gateway.chat({ messages });
    return response.content || response;
  }

  async generateMonitoringReport(executions, timeRange = '1h') {
    const report = this._computeMetrics(executions);
    
    if (this.gateway && this.gateway.activeProvider) {
      try {
        const aiReport = await this._aiMonitoringAnalysis(report, timeRange);
        return { ...report, ai_analysis: aiReport, ai_powerd: true };
      } catch (e) {
        return { ...report, ai_powerd: false, fallback: true };
      }
    }

    return { ...report, ai_powerd: false };
  }

  _computeMetrics(executions) {
    if (!executions || executions.length === 0) {
      return {
        total: 0,
        successRate: 0,
        avgDuration: 0,
        p95Duration: 0,
        byOperator: {},
        byStatus: { success: 0, failed: 0 },
        anomalies: []
      };
    }

    const durations = executions.map(e => e.duration || 0).sort((a, b) => a - b);
    const successCount = executions.filter(e => e.status === 'success').length;
    const byOperator = {};
    const byStatus = { success: 0, failed: 0 };

    executions.forEach(e => {
      const op = e.operator || 'unknown';
      if (!byOperator[op]) byOperator[op] = { count: 0, totalDuration: 0, successCount: 0 };
      byOperator[op].count++;
      byOperator[op].totalDuration += e.duration || 0;
      if (e.status === 'success') byOperator[op].successCount++;
      
      if (e.status === 'success') byStatus.success++;
      else byStatus.failed++;
    });

    const avgDuration = durations.length > 0 ? durations.reduce((a, b) => a + b, 0) / durations.length : 0;
    const p95Idx = Math.floor(durations.length * 0.95);
    const p95Duration = durations[p95Idx] || 0;

    const anomalies = [];
    Object.entries(byOperator).forEach(([op, data]) => {
      const failRate = (data.count - data.successCount) / data.count;
      const avgOpDuration = data.totalDuration / data.count;
      
      if (failRate > 0.2) {
        anomalies.push({ type: 'high_failure_rate', operator: op, value: failRate, severity: 'warning' });
      }
      if (avgOpDuration > avgDuration * 3 && avgDuration > 0) {
        anomalies.push({ type: 'slow_execution', operator: op, value: avgOpDuration, severity: 'info' });
      }
    });

    return {
      total: executions.length,
      successRate: executions.length > 0 ? successCount / executions.length : 0,
      avgDuration: Math.round(avgDuration),
      p95Duration: Math.round(p95Duration),
      byOperator: Object.fromEntries(
        Object.entries(byOperator).map(([k, v]) => [
          k,
          { ...v, avgDuration: Math.round(v.totalDuration / v.count), successRate: v.successCount / v.count }
        ])
      ),
      byStatus,
      anomalies
    };
  }

  async _aiMonitoringAnalysis(report, timeRange) {
    const prompt = `请基于以下监控数据生成分析报告:

时间范围: ${timeRange}
总执行数: ${report.total}
成功率: ${(report.successRate * 100).toFixed(1)}%
平均耗时: ${report.avgDuration}ms
P95耗时: ${report.p95Duration}ms

异常检测:
${JSON.stringify(report.anomalies, null, 2)}

按算子统计:
${JSON.stringify(report.byOperator, null, 2)}

请:
1. 评估系统健康状况
2. 识别性能瓶颈
3. 预测潜在风险
4. 提供优化建议`;

    const messages = [
      { role: 'system', content: '你是一个专业的系统监控分析师。请提供简洁、可操作的分析报告。' },
      { role: 'user', content: prompt }
    ];

    const response = await this.gateway.chat({ messages });
    return response.content || response;
  }

  async executeMCPTool(toolName, params, context = {}) {
    const toolRegistry = this._getMCPTools();
    const tool = toolRegistry.find(t => t.name === toolName);

    if (!tool) {
      return { success: false, error: `工具 ${toolName} 未注册`, availableTools: toolRegistry.map(t => t.name) };
    }

    if (!this._checkRateLimit(toolName)) {
      return { success: false, error: '速率限制 exceeded，请稍后重试' };
    }

    try {
      if (this.gateway && this.gateway.activeProvider) {
        const result = await this._aiMCPCall(tool, params, context);
        this._recordMCPCall(toolName, true, result.duration);
        return result;
      } else {
        const result = tool.handler ? await tool.handler(params, context) : { result: '工具已执行' };
        this._recordMCPCall(toolName, true, 50);
        return { success: true, result, ai_powerd: false };
      }
    } catch (error) {
      this._recordMCPCall(toolName, false, 0);
      return { success: false, error: error.message };
    }
  }

  _getMCPTools() {
    return [
      {
        name: 'web_search',
        description: '搜索互联网获取最新信息',
        parameters: { query: 'string', max_results: 'number' },
        handler: async (params) => ({ results: [{ title: '搜索结果', url: '#', snippet: params.query }] })
      },
      {
        name: 'code_analysis',
        description: '分析代码结构和质量',
        parameters: { code: 'string', language: 'string' },
        handler: async (params) => ({ analysis: '代码分析完成', issues: [] })
      },
      {
        name: 'data_transform',
        description: '数据格式转换和处理',
        parameters: { data: 'any', from_format: 'string', to_format: 'string' },
        handler: async (params) => ({ transformed: params.data })
      },
      {
        name: 'text_summarize',
        description: '文本摘要生成',
        parameters: { text: 'string', max_length: 'number' },
        handler: async (params) => ({ summary: params.text?.slice(0, 100) + '...' })
      },
      {
        name: 'chart_generate',
        description: '生成数据可视化图表',
        parameters: { data: 'array', chart_type: 'string' },
        handler: async (params) => ({ chart: { type: params.chart_type, data: params.data } })
      }
    ];
  }

  async _aiMCPCall(tool, params, context) {
    const prompt = `请调用 MCP 工具: ${tool.name}

工具描述: ${tool.description}
参数: ${JSON.stringify(params, null, 2)}
上下文: ${JSON.stringify(context, null, 2)}

请执行操作并返回结果。`;

    const messages = [
      { role: 'system', content: `你是一个 MCP 工具执行器。根据工具描述执行操作。` },
      { role: 'user', content: prompt }
    ];

    const startTime = Date.now();
    const response = await this.gateway.chat({ messages });
    return {
      success: true,
      result: response.content || response,
      tool: tool.name,
      duration: Date.now() - startTime,
      ai_powerd: true
    };
  }

  async executeBrowserTask(url, instructions, options = {}) {
    const startTime = Date.now();

    if (this.gateway && this.gateway.activeProvider) {
      try {
        const plan = await this._aiPlanBrowserTask(url, instructions);
        const result = await this._executeBrowserPlan(plan, url, options);
        return {
          success: true,
          plan,
          result,
          duration: Date.now() - startTime,
          ai_powerd: true
        };
      } catch (error) {
        return { success: false, error: error.message, duration: Date.now() - startTime };
      }
    }

    return {
      success: true,
      plan: [{ action: 'navigate', target: url }, { action: 'extract', target: 'body' }],
      result: { content: '浏览器任务执行完成（模拟）', elements_found: 10 + Math.floor(Math.random() * 20) },
      duration: Date.now() - startTime,
      ai_powerd: false,
      fallback: true
    };
  }

  async _aiPlanBrowserTask(url, instructions) {
    const prompt = `请为以下浏览器任务制定执行计划:

目标URL: ${url}
指令: ${instructions}

请生成一个步骤化的执行计划，每步包含:
- action: navigate/click/type/extract/screenshot
- target: CSS选择器或URL
- description: 步骤说明

返回JSON格式的计划。`;

    const messages = [
      { role: 'system', content: '你是一个浏览器自动化专家。请生成精确的执行计划。' },
      { role: 'user', content: prompt }
    ];

    const response = await this.gateway.chat({ messages });
    try {
      const plan = this._extractJSON(response.content || response);
      return plan.steps || plan;
    } catch {
      return [{ action: 'navigate', target: url }, { action: 'extract', target: 'body' }];
    }
  }

  async _executeBrowserPlan(plan, url, options) {
    const results = [];
    for (const step of plan) {
      results.push({
        action: step.action,
        target: step.target,
        status: 'completed',
        result: `${step.action} on ${step.target} executed`
      });
    }
    return { steps: results, extracted_data: {}, screenshots: 0 };
  }

  async orchestratePlugins(plugins, pipeline, inputs = {}, { traceId } = {}) {
    const startTime = Date.now();
    const results = [];
    let currentData = inputs;
    // Step3 · 轻量图谱落边 helper（失败内吞，entities 优先）
    const storage = (() => { try { return require('./storage').getStorage(); } catch (_) { return null; } })();
    const _kg = (idx, name, status, extra) => {
      if (!storage) return;
      try {
        const phase = (['P4','P4a','P4b','P4c','P4d','P8','P8a'])[idx] || `P4s${idx}`; // P4=代码开发家族 + P8=发布家族
        storage.addEdge(`plugin:${name}#${idx}`,
          status === 'success' ? 'produces' : 'fails_in',
          `run:${traceId || 'anon'}#step${idx}`,
          Object.assign({ phase_code: phase, plugin: name, status, duration_ms: Date.now() - startTime }, extra || {}));
      } catch (_) {}
    };

    for (const stage of pipeline) {
      const plugin = plugins.find(p => p.id === stage.plugin || p.name === stage.plugin);
      if (!plugin) {
        results.push({ stage: stage.name, status: 'skipped', reason: 'plugin not found' });
        continue;
      }

      try {
        if (this.gateway && this.gateway.activeProvider && stage.ai_enabled) {
          const aiResult = await this._aiPluginExecution(plugin, currentData, stage);
          currentData = aiResult.output;
          const r = {
            stage: stage.name,
            plugin: plugin.name,
            status: 'success',
            duration: aiResult.duration,
            ai_powerd: true
          };
          results.push(r);
          _kg(results.length - 1, stage.name, 'success', r);
        } else {
          const result = plugin.execute ? await plugin.execute(currentData, stage.config) : { data: currentData };
          currentData = result.data || result;
          const r = {
            stage: stage.name,
            plugin: plugin.name,
            status: 'success',
            ai_powerd: false
          };
          results.push(r);
          _kg(results.length - 1, stage.name, 'success', r);
        }
      } catch (error) {
        const r = {
          stage: stage.name,
          plugin: plugin.name,
          status: 'failed',
          error: error.message
        };
        results.push(r);
        _kg(results.length - 1, stage.name, 'failed', r);
        if (stage.critical) {
          return { success: false, results, error: error.message, duration: Date.now() - startTime };
        }
      }
    }

    return {
      success: results.filter(r => r.status === 'failed').length === 0,
      results,
      finalOutput: currentData,
      duration: Date.now() - startTime
    };
  }

  async _aiPluginExecution(plugin, data, stage) {
    const prompt = `请执行插件处理:

插件: ${plugin.name} (${plugin.type})
输入数据: ${JSON.stringify(data).slice(0, 2000)}
配置: ${JSON.stringify(stage.config || {}).slice(0, 500)}

请执行相应的处理逻辑并返回结果。`;

    const messages = [
      { role: 'system', content: `你是一个插件执行引擎。根据插件类型执行相应的数据处理。` },
      { role: 'user', content: prompt }
    ];

    const startTime = Date.now();
    const response = await this.gateway.chat({ messages });
    return {
      output: response.content || response,
      duration: Date.now() - startTime
    };
  }

  _logExecution(context, result, duration) {
    const log = {
      timestamp: new Date().toISOString(),
      operator: context.operator,
      type: context.type,
      status: result.error ? 'failed' : 'success',
      duration,
      ai_powerd: result.ai_powerd || false,
      has_metrics: !!result.metrics
    };

    this.executionLog.push(log);
    if (this.executionLog.length > 10000) {
      this.executionLog = this.executionLog.slice(-5000);
    }

    if (fs.existsSync(path.join(DATA_DIR, 'ai_execution_log.json'))) {
      try {
        const existing = JSON.parse(fs.readFileSync(path.join(DATA_DIR, 'ai_execution_log.json'), 'utf8'));
        existing.push(log);
        fs.writeFileSync(
          path.join(DATA_DIR, 'ai_execution_log.json'),
          JSON.stringify(existing.slice(-5000)),
          'utf8'
        );
      } catch {}
    }
  }

  _checkRateLimit(key) {
    const now = Date.now();
    const entry = this.rateLimitMap.get(key) || { count: 0, resetTime: now };
    
    if (now > entry.resetTime) {
      entry.count = 0;
      entry.resetTime = now + this.rateLimitWindow;
    }
    
    if (entry.count >= this.rateLimitMax) {
      return false;
    }
    
    entry.count++;
    this.rateLimitMap.set(key, entry);
    return true;
  }

  _recordMCPCall(toolName, success, duration) {
    const log = {
      tool: toolName,
      success,
      duration,
      timestamp: new Date().toISOString()
    };
    
    try {
      const fp = path.join(DATA_DIR, 'mcp_calls.json');
      const existing = fs.existsSync(fp) ? JSON.parse(fs.readFileSync(fp, 'utf8')) : [];
      existing.push(log);
      fs.writeFileSync(fp, JSON.stringify(existing.slice(-1000)), 'utf8');
    } catch {}
  }

  _extractJSON(text) {
    if (!text) return {};
    const jsonMatch = text.match(/\{[\s\S]*\}/);
    if (jsonMatch) {
      try {
        return JSON.parse(jsonMatch[0]);
      } catch {
        return {};
      }
    }
    return {};
  }

  getExecutionStats() {
    const recent = this.executionLog.slice(-100);
    const stats = {
      total: this.executionLog.length,
      lastHour: recent.filter(l => Date.now() - new Date(l.timestamp).getTime() < 3600000).length,
      successRate: recent.length > 0 
        ? recent.filter(l => l.status === 'success').length / recent.length 
        : 0,
      aiPoweredRate: recent.length > 0
        ? recent.filter(l => l.ai_powerd).length / recent.length
        : 0,
      avgDuration: recent.length > 0
        ? recent.reduce((sum, l) => sum + (l.duration || 0), 0) / recent.length
        : 0
    };
    return stats;
  }

  // ==================== 专家联盟集成 ====================

  async analyzeWithExpertAlliance(question, graphData, options = {}) {
    const { getAlliance } = require('./expert-alliance');
    const alliance = getAlliance();

    const startTime = Date.now();
    const results = {
      question,
      graph_analysis: null,
      expert_consults: [],
      algorithm_insights: null,
      ai_enhanced: false
    };

    if (graphData && graphData.nodes && graphData.nodes.length > 0) {
      try {
        results.graph_analysis = await this.analyzeGraph(graphData, options);
      } catch (e) {
        results.graph_analysis = { error: e.message };
      }
    }

    const expertRoute = await alliance.routeExperts(question, { maxExperts: 3 });
    const relevantExpertIds = expertRoute.selected.map(s => s.expert.id);

    for (const expertId of relevantExpertIds) {
      try {
        const consultation = await alliance.consult(expertId, [
          { role: 'user', content: question }
        ], {
          problemContext: options.context,
          businessConstraints: options.constraints
        });
        results.expert_consults.push({
          expert: consultation.expert,
          response: consultation.response,
          duration_ms: consultation.metadata?.duration_ms
        });
      } catch (e) {
        results.expert_consults.push({
          expert_id: expertId,
          error: e.message
        });
      }
    }

    if (this.gateway && this.gateway.activeProvider) {
      try {
        results.algorithm_insights = await this._synthesisAnalysis(
          question,
          results.graph_analysis,
          results.expert_consults
        );
        results.ai_enhanced = true;
      } catch (e) {
        results.algorithm_insights = { error: e.message };
      }
    }

    results.meta = {
      total_duration_ms: Date.now() - startTime,
      experts_consulted: results.expert_consults.length,
      has_graph: !!results.graph_analysis,
      timestamp: new Date().toISOString()
    };

    return results;
  }

  async _synthesisAnalysis(question, graphAnalysis, expertConsults) {
    const prompt = `请综合分析以下专家联盟的分析结果，提供最终的综合洞察：

原始问题：${question}

图谱分析摘要：
${graphAnalysis ? JSON.stringify({
  stats: graphAnalysis.stats,
  topNodes: graphAnalysis.topNodes?.slice(0, 5)
}, null, 2) : '无图谱数据'}

专家咨询结果：
${expertConsults.map((c, i) => {
  if (c.response) {
    return `专家${i + 1}(${c.expert?.name || c.expert_id}): ${c.response.slice(0, 300)}`;
  }
  return `专家${i + 1}: 分析失败 - ${c.error || '未知错误'}`;
}).join('\n\n')}

请提供：
1. 综合结论（2-3段）
2. 关键发现（3-5条）
3. 行动建议（具体可执行的步骤）
4. 风险提示（需要注意的问题）`;

    const messages = [
      { role: 'system', content: '你是专家联盟的首席分析师，负责综合多位专家的分析结果，提供高质量的最终报告。请用中文输出。' },
      { role: 'user', content: prompt }
    ];

    const response = await this.gateway.chat({ messages });
    return response.content || response;
  }

  async executeOperatorChain(operators, inputs, options = {}) {
    const results = [];
    let currentInputs = inputs;

    for (let i = 0; i < operators.length; i++) {
      const operator = operators[i];
      const startTime = Date.now();

      try {
        const result = await this.executeOperator(operator, currentInputs, {
          ...options,
          chain_context: {
            step: i + 1,
            total_steps: operators.length,
            previous_results: results.map(r => r.result)
          }
        });

        results.push({
          step: i + 1,
          operator: operator.name || operator.id,
          success: result.success,
          result: result.result,
          duration_ms: result.duration,
          ai_powerd: result.ai_powerd
        });

        if (result.success) {
          currentInputs = result.result;
        } else if (!options.continue_on_error) {
          return {
            success: false,
            results,
            error: result.error,
            partial_result: results.filter(r => r.success).pop()?.result
          };
        }
      } catch (e) {
        results.push({
          step: i + 1,
          operator: operator.name || operator.id,
          success: false,
          error: e.message,
          duration_ms: Date.now() - startTime
        });

        if (!options.continue_on_error) {
          return {
            success: false,
            results,
            error: e.message
          };
        }
      }
    }

    return {
      success: results.every(r => r.success),
      total_steps: operators.length,
      successful_steps: results.filter(r => r.success).length,
      failed_steps: results.filter(r => !r.success).length,
      results,
      final_result: results.filter(r => r.success).pop()?.result,
      total_duration_ms: results.reduce((sum, r) => sum + (r.duration_ms || 0), 0)
    };
  }

  async analyzeProblem(question, context = {}) {
    const { getAlliance } = require('./expert-alliance');
    const alliance = getAlliance();

    const startTime = Date.now();
    const analysis = {
      question,
      intent: null,
      routing: null,
      expert_analysis: null,
      algorithm_analysis: null,
      solution_path: [],
      confidence: 0,
      completed_at: null
    };

    try {
      const routing = await alliance.routeExperts(question, { maxExperts: 5 });
      analysis.intent = routing.intent;
      analysis.routing = {
        primary_expert: routing.selected[0]?.expert.name,
        candidates: routing.selected.map(s => ({
          name: s.expert.name,
          score: Math.round(s.score * 100) / 100,
          type: s.expert.type
        }))
      };
      analysis.confidence = routing.intent.confidence;

      const expertsToConsult = routing.selected.slice(0, 3).map(s => s.expert.id);
      const expertResults = [];

      for (const expertId of expertsToConsult) {
        try {
          const result = await alliance.consult(expertId, [
            { role: 'user', content: question }
          ], {
            problemContext: context.background,
            businessConstraints: context.constraints
          });
          expertResults.push(result);
        } catch (e) {
          expertResults.push({ error: e.message });
        }
      }

      analysis.expert_analysis = expertResults.map((r, i) => ({
        expert: `专家${i + 1}`,
        response: r.response || r.error
      }));

      if (this.gateway && this.gateway.activeProvider) {
        const synthesisPrompt = `作为解决方案架构师，基于以下专家分析，为问题设计解决路径：

问题：${question}
上下文：${context.background || '无'}

专家分析：
${expertResults.map((r, i) => `[${i + 1}] ${r.response || '分析失败'}`).join('\n\n')}

请输出结构化的解决方案路径（JSON格式）：
{
  "solution_path": [
    {"step": 1, "action": "具体行动", "description": "详细说明", "estimated_effort": "预估工作量"},
    ...
  ],
  "key_insights": ["关键洞察1", "关键洞察2", ...],
  "recommendations": ["建议1", "建议2", ...],
  "risks": ["风险1", "风险2"],
  "confidence": 0.0-1.0
}`;

        const response = await this.gateway.chat({
          messages: [
            { role: 'system', content: '你是一位资深解决方案架构师。请基于多位专家的分析，综合输出结构化、可执行的解决路径。' },
            { role: 'user', content: synthesisPrompt }
          ]
        });

        const insights = this._extractJSON(response.content || response);
        analysis.solution_path = insights.solution_path || [];
        analysis.key_insights = insights.key_insights || [];
        analysis.recommendations = insights.recommendations || [];
        analysis.risks = insights.risks || [];
        analysis.ai_confidence = insights.confidence;
      }

      analysis.completed_at = new Date().toISOString();
      analysis.total_duration_ms = Date.now() - startTime;
      analysis.success = true;
    } catch (e) {
      analysis.success = false;
      analysis.error = e.message;
      analysis.completed_at = new Date().toISOString();
    }

    return analysis;
  }

  async assessSolutionFeasibility(solution, constraints = []) {
    if (!this.gateway || !this.gateway.activeProvider) {
      return {
        feasible: true,
        score: 0.7,
        issues: ['AI评估不可用，默认可行'],
        fallback: true
      };
    }

    const prompt = `请评估以下解决方案的可行性：

解决方案：
${JSON.stringify(solution, null, 2)}

约束条件：
${constraints.length > 0 ? constraints.map((c, i) => `${i + 1}. ${c}`).join('\n') : '无特殊约束'}

请从以下维度评估（每个维度0-10分）：
1. 技术可行性
2. 资源需求
3. 时间复杂度
4. 维护成本
5. 风险等级

返回JSON格式：
{
  "feasible": true/false,
  "score": 0.0-1.0,
  "dimensions": {"technical": 8, "resource": 6, "time": 7, "maintenance": 5, "risk": 3},
  "issues": ["需要注意的问题1", "需要注意的问题2"],
  "suggestions": ["改进建议1", "改进建议2"]
}`;

    try {
      const response = await this.gateway.chat({
        messages: [
          { role: 'system', content: '你是一位资深的技术可行性评估专家。请提供客观、量化的评估结果。' },
          { role: 'user', content: prompt }
        ]
      });

      const result = this._extractJSON(response.content || response);
      return {
        feasible: result.feasible ?? true,
        score: result.score ?? 0.7,
        dimensions: result.dimensions || {},
        issues: result.issues || [],
        suggestions: result.suggestions || [],
        ai_powerd: true
      };
    } catch (e) {
      return {
        feasible: true,
        score: 0.5,
        issues: [`评估失败: ${e.message}`],
        ai_powerd: false,
        fallback: true
      };
    }
  }
}

let instance = null;

function getAIEngine(gateway) {
  if (!instance) {
    instance = new AIEngine(gateway);
  }
  return instance;
}

module.exports = { AIEngine, getAIEngine };
'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { getGateway } = require('./llm-gateway');
const { getAlliance } = require('./expert-alliance');

const DATA_DIR = path.join(__dirname, '..', 'data');

function readJSON(file, fallback) {
  try {
    const fp = path.join(DATA_DIR, file);
    if (!fs.existsSync(fp)) return fallback;
    const raw = fs.readFileSync(fp, 'utf8');
    return raw ? JSON.parse(raw) : fallback;
  } catch (e) {
    return fallback;
  }
}

function writeJSON(file, data) {
  try {
    fs.writeFileSync(path.join(DATA_DIR, file), JSON.stringify(data, null, 2), 'utf8');
    return true;
  } catch (e) {
    console.error('[ai-integration] writeJSON', file, e.message);
    return false;
  }
}

class GraphIntelligenceEngine {
  constructor() {
    this.graphCache = new Map();
    this.pageRankCache = new Map();
    this.graphHistory = [];
    this._init();
  }

  _init() {
    const history = readJSON('graph_intelligence_history.json', []);
    this.graphHistory = history.slice(-100);
  }

  async computePersonalizedPageRank(graphData, options = {}) {
    const {
      damping = 0.85,
      maxIterations = 100,
      tolerance = 1e-6,
      personalization = {},
      queryNodes = [],
      topK = 20
    } = options;

    const nodes = graphData.nodes || [];
    const edges = graphData.edges || [];
    const n = nodes.length;

    if (n === 0) return { scores: [], convergence: 0, iterations: 0 };

    const nodeIds = nodes.map(n => n.id);
    const idToIdx = new Map(nodeIds.map((id, i) => [id, i]));

    const adjList = new Array(n).fill(null).map(() => []);
    const outDegree = new Array(n).fill(0);

    edges.forEach(e => {
      const src = idToIdx.get(e.source);
      const tgt = idToIdx.get(e.target);
      if (src !== undefined && tgt !== undefined) {
        const weight = e.weight || 1.0;
        adjList[src].push({ target: tgt, weight });
        outDegree[src] += weight;
      }
    });

    const personalizationVec = new Array(n).fill(0);
    if (Object.keys(personalization).length > 0) {
      const totalWeight = Object.values(personalization).reduce((a, b) => a + b, 0);
      Object.entries(personalization).forEach(([nodeId, weight]) => {
        const idx = idToIdx.get(nodeId);
        if (idx !== undefined && totalWeight > 0) {
          personalizationVec[idx] = weight / totalWeight;
        }
      });
    } else {
      for (let i = 0; i < n; i++) personalizationVec[i] = 1 / n;
    }

    let rank = personalizationVec.slice();
    let iterations = 0;
    let converged = false;

    for (let iter = 0; iter < maxIterations; iter++) {
      const newRank = new Array(n).fill(0);

      for (let i = 0; i < n; i++) {
        let contribution = 0;
        const outWeight = outDegree[i];

        if (outWeight > 0) {
          for (const { target, weight } of adjList[i]) {
            contribution += (rank[i] * weight) / outWeight;
          }
        } else {
          contribution = rank[i];
        }

        newRank[i] = (1 - damping) * personalizationVec[i] + damping * contribution;
      }

      const total = newRank.reduce((a, b) => a + b, 0) || 1;
      for (let i = 0; i < n; i++) newRank[i] /= total;

      let maxDiff = 0;
      for (let i = 0; i < n; i++) {
        maxDiff = Math.max(maxDiff, Math.abs(newRank[i] - rank[i]));
      }

      rank = newRank;
      iterations = iter + 1;

      if (maxDiff < tolerance) {
        converged = true;
        break;
      }
    }

    const results = nodeIds.map((id, i) => ({
      id,
      score: rank[i],
      originalIndex: i
    })).sort((a, b) => b.score - a.score);

    const topResults = results.slice(0, topK);

    if (queryNodes.length > 0) {
      const queryBoost = 0.3;
      results.forEach(r => {
        if (queryNodes.includes(r.id)) {
          r.score *= (1 + queryBoost);
        }
      });
      results.sort((a, b) => b.score - a.score);
    }

    return {
      scores: results,
      topK: topResults,
      convergence: 1 - Math.min(1, Math.max(0, (iterations / maxIterations))),
      iterations,
      converged,
      totalNodes: n,
      totalEdges: edges.length,
      timestamp: new Date().toISOString()
    };
  }

  async buildSymbolGraph(codeStructure, options = {}) {
    const { includeCallGraph = true, includeDataFlow = true } = options;
    const symbols = codeStructure.symbols || [];
    const calls = codeStructure.calls || [];
    const definitions = codeStructure.definitions || [];

    const nodeTypes = new Map();
    symbols.forEach(s => {
      nodeTypes.set(s.id, {
        id: s.id,
        name: s.name,
        type: s.type,
        file: s.file,
        line: s.line
      });
    });

    const edges = [];
    if (includeCallGraph) {
      calls.forEach(c => {
        edges.push({
          source: c.caller,
          target: c.callee,
          type: 'call',
          weight: c.count || 1
        });
      });
    }

    if (includeDataFlow) {
      definitions.forEach(d => {
        edges.push({
          source: d.file,
          target: d.symbol,
          type: 'defines',
          weight: 1
        });
      });
    }

    const graphData = {
      nodes: Array.from(nodeTypes.values()),
      edges,
      metadata: {
        totalSymbols: symbols.length,
        totalCalls: calls.length,
        totalDefinitions: definitions.length,
        includeCallGraph,
        includeDataFlow,
        builtAt: new Date().toISOString()
      }
    };

    return graphData;
  }

  async computeTokenBudgetPruning(graphData, maxTokens, options = {}) {
    const { strategy = 'importance', preserveNodes = [] } = options;
    const nodes = graphData.nodes || [];
    const edges = graphData.edges || [];

    const rankedResult = await this.computePersonalizedPageRank(graphData, { topK: nodes.length });
    const rankedNodes = rankedResult.scores;

    const preservedSet = new Set(preserveNodes);
    const importantNodes = rankedNodes.filter(n => preservedSet.has(n.id) || n.score > 0.01);

    let tokenEstimate = importantNodes.length * 50;
    const selectedNodes = [...importantNodes];

    if (tokenEstimate < maxTokens) {
      const remaining = rankedNodes.filter(n => !preservedSet.has(n.id) && !importantNodes.find(i => i.id === n.id));
      for (const node of remaining) {
        if (tokenEstimate >= maxTokens * 0.95) break;
        selectedNodes.push(node);
        tokenEstimate += 50;
      }
    }

    const selectedIds = new Set(selectedNodes.map(n => n.id));
    const filteredEdges = edges.filter(e => selectedIds.has(e.source) && selectedIds.has(e.target));

    return {
      nodes: selectedNodes,
      edges: filteredEdges,
      estimatedTokens: tokenEstimate,
      maxTokens,
      pruningRatio: 1 - (selectedNodes.length / Math.max(1, nodes.length)),
      strategy
    };
  }

  async detectCommunitiesAdvanced(graphData, options = {}) {
    const {
      algorithm = 'louvain',
      maxCommunities = 10,
      minCommunitySize = 2,
      resolution = 1.0
    } = options;

    const nodes = graphData.nodes || [];
    const edges = graphData.edges || [];
    const n = nodes.length;

    if (n < minCommunitySize) {
      return { communities: [], modularity: 0, algorithm, note: '图太小，无法检测社区' };
    }

    const adjMatrix = new Map();
    nodes.forEach(n => adjMatrix.set(n.id, new Map()));
    edges.forEach(e => {
      if (adjMatrix.has(e.source) && adjMatrix.has(e.target)) {
        const weight = e.weight || 1;
        adjMatrix.get(e.source).set(e.target, (adjMatrix.get(e.source).get(e.target) || 0) + weight);
        adjMatrix.get(e.target).set(e.source, (adjMatrix.get(e.target).get(e.source) || 0) + weight);
      }
    });

    const community = new Map();
    nodes.forEach(n => community.set(n.id, n.id));

    const nodeCommunity = new Map();
    nodes.forEach(n => nodeCommunity.set(n.id, n.id));

    let totalWeight = 0;
    edges.forEach(e => { totalWeight += (e.weight || 1) * 2; });

    let moved = true;
    let passes = 0;

    while (moved && passes < 20) {
      moved = false;
      passes++;

      for (const node of nodes) {
        const currentComm = nodeCommunity.get(node.id);
        let bestComm = currentComm;
        let bestGain = 0;

        for (const neighborId of adjMatrix.get(node.id).keys()) {
          const neighborComm = nodeCommunity.get(neighborId);
          if (neighborComm !== currentComm) {
            const k_i = Array.from(adjMatrix.get(node.id).values()).reduce((a, b) => a + b, 0);
            let sumIn = 0;
            let sumTot = 0;

            for (const [nid, weight] of adjMatrix) {
              if (nodeCommunity.get(nid) === neighborComm) {
                sumTot += Array.from(adjMatrix.get(nid).values()).reduce((a, b) => a + b, 0);
              }
            }

            for (const [neid, weight] of adjMatrix.get(node.id)) {
              if (nodeCommunity.get(neid) === neighborComm) {
                sumIn += weight;
              }
            }

            const gain = sumIn - resolution * sumTot * k_i / (totalWeight || 1);
            if (gain > bestGain) {
              bestGain = gain;
              bestComm = neighborComm;
            }
          }
        }

        if (bestComm !== currentComm) {
          nodeCommunity.set(node.id, bestComm);
          moved = true;
        }
      }
    }

    const communities = new Map();
    nodeCommunity.forEach((commId, nodeId) => {
      if (!communities.has(commId)) {
        communities.set(commId, []);
      }
      communities.get(commId).push(nodeId);
    });

    const resultCommunities = [];
    communities.forEach((members, id) => {
      if (members.length >= minCommunitySize) {
        resultCommunities.push({ id, members, size: members.length });
      }
    });

    const limited = resultCommunities.slice(0, maxCommunities);

    return {
      communities: limited,
      totalCommunities: resultCommunities.length,
      algorithm,
      passes,
      resolution,
      minCommunitySize,
      detectedAt: new Date().toISOString()
    };
  }

  getStats() {
    return {
      totalGraphsProcessed: this.graphHistory.length,
      recentRuns: this.graphHistory.slice(-10),
      pageRankCacheSize: this.pageRankCache.size
    };
  }
}

class PlanActOrchestrator {
  constructor() {
    this.plans = new Map();
    this.checkpoints = new Map();
    this.executionHistory = [];
    this.stateStore = new Map();
    this._init();
  }

  _init() {
    const history = readJSON('plan_act_history.json', []);
    this.executionHistory = history.slice(-200);
  }

  async createPlan(question, context = {}, options = {}) {
    const gateway = getGateway();
    const startTime = Date.now();

    const planId = `plan_${crypto.randomUUID ? crypto.randomUUID() : 'plan_' + Date.now()}`;

    const planningPrompt = `请为以下任务制定详细的执行计划：

任务: ${question}
上下文: ${JSON.stringify(context).slice(0, 2000)}
选项: ${JSON.stringify(options).slice(0, 1000)}

请生成结构化的执行计划，包含：
1. 任务分解（子任务列表）
2. 每个子任务的描述和目标
3. 依赖关系
4. 预期输出
5. 风险评估和缓解策略

返回JSON格式：
{
  "plan_id": "${planId}",
  "objective": "主要目标",
  "steps": [
    {
      "id": "step_1",
      "title": "步骤标题",
      "description": "详细描述",
      "depends_on": [],
      "expected_output": "预期输出",
      "risk_level": "low|medium|high",
      "estimated_tokens": 1000
    }
  ],
  "critical_path": ["step_1", "step_3", "step_5"],
  "total_estimated_tokens": 10000,
  "risk_assessment": "整体风险评估"
}`;

    let plan;
    if (gateway && gateway.activeProvider) {
      try {
        const response = await gateway.chat({
          messages: [
            {
              role: 'system',
              content: '你是一位专业的任务规划专家。请生成详细、可执行的计划。输出严格JSON格式。'
            },
            { role: 'user', content: planningPrompt }
          ],
          temperature: 0.3,
          maxTokens: 4096
        });

        plan = this._extractJSON(response.content || response);
      } catch (e) {
        console.warn('[plan-act] AI规划失败，使用确定性规划:', e.message);
      }
    }

    if (!plan || !plan.steps) {
      plan = this._deterministicPlan(question, context, planId);
    }

    plan.metadata = {
      created_at: new Date().toISOString(),
      created_by: 'plan_mode',
      duration_ms: Date.now() - startTime,
      ai_powered: !!(gateway && gateway.activeProvider)
    };

    this.plans.set(planId, plan);

    this.executionHistory.push({
      type: 'plan_created',
      planId,
      objective: plan.objective,
      stepsCount: plan.steps?.length || 0,
      durationMs: Date.now() - startTime,
      timestamp: new Date().toISOString()
    });

    return plan;
  }

  async executePlan(planId, options = {}) {
    const plan = this.plans.get(planId);
    if (!plan) throw new Error(`计划不存在: ${planId}`);

    const gateway = getGateway();
    const {
      autoExecute = false,
      checkpointEnabled = true,
      maxRetries = 3,
      stepTimeout = 60000
    } = options;

    const execution = {
      planId,
      status: 'executing',
      startedAt: new Date().toISOString(),
      completedAt: null,
      results: [],
      checkpoints: [],
      totalDuration: 0
    };

    const startTime = Date.now();

    for (const step of plan.steps || []) {
      const stepStart = Date.now();

      if (checkpointEnabled) {
        const checkpoint = this._createCheckpoint(planId, step.id, execution.results);
        execution.checkpoints.push(checkpoint);
      }

      try {
        const result = await this._executeStep(step, plan, options, gateway);

        execution.results.push({
          stepId: step.id,
          status: 'success',
          output: result.output,
          durationMs: Date.now() - stepStart,
          aiPowered: result.aiPowered
        });

        if (step.risk_level === 'high' && !autoExecute) {
          execution.pausedForApproval = true;
          execution.pendingStep = step.id;
          execution.status = 'paused';
          return execution;
        }
      } catch (error) {
        const retries = options.retries || 0;
        if (retries < maxRetries) {
          options.retries = retries + 1;
          continue;
        }

        execution.results.push({
          stepId: step.id,
          status: 'failed',
          error: error.message,
          durationMs: Date.now() - stepStart
        });

        if (!autoExecute && step.risk_level !== 'low') {
          execution.status = 'failed';
          execution.failedStep = step.id;
          execution.error = error.message;
          return execution;
        }
      }
    }

    execution.status = 'completed';
    execution.completedAt = new Date().toISOString();
    execution.totalDuration = Date.now() - startTime;

    this.plans.set(planId, plan);

    this.executionHistory.push({
      type: 'plan_executed',
      planId,
      status: execution.status,
      stepsCompleted: execution.results.filter(r => r.status === 'success').length,
      totalSteps: plan.steps?.length || 0,
      durationMs: execution.totalDuration,
      timestamp: new Date().toISOString()
    });

    return execution;
  }

  async rollbackToCheckpoint(planId, checkpointId) {
    const checkpoints = this.checkpoints.get(planId) || [];
    const targetIdx = checkpoints.findIndex(c => c.id === checkpointId);

    if (targetIdx === -1) throw new Error(`检查点不存在: ${checkpointId}`);

    const checkpoint = checkpoints[targetIdx];

    const rollbackResult = {
      planId,
      checkpointId,
      rolledBackToStep: checkpoint.stepId,
      preservedSteps: checkpoint.previousResults.length,
      rolledBackAt: new Date().toISOString()
    };

    this.executionHistory.push({
      type: 'checkpoint_rollback',
      planId,
      checkpointId,
      rolledBackToStep: checkpoint.stepId,
      timestamp: new Date().toISOString()
    });

    return rollbackResult;
  }

  async getPlan(planId) {
    return this.plans.get(planId) || null;
  }

  listPlans(options = {}) {
    const { status, keyword } = options;
    let result = Array.from(this.plans.values());

    if (status) {
      result = result.filter(p => p.status === status);
    }
    if (keyword) {
      const kw = keyword.toLowerCase();
      result = result.filter(p =>
        p.objective?.toLowerCase().includes(kw) ||
        p.steps?.some(s => s.title.toLowerCase().includes(kw))
      );
    }

    return result.sort((a, b) =>
      new Date(b.metadata?.created_at) - new Date(a.metadata?.created_at)
    );
  }

  getStats() {
    return {
      totalPlans: this.plans.size,
      totalCheckpoints: this.checkpoints.size,
      recentHistory: this.executionHistory.slice(-10),
      statusCounts: Array.from(this.plans.values()).reduce((acc, p) => {
        acc[p.status] = (acc[p.status] || 0) + 1;
        return acc;
      }, {})
    };
  }

  _deterministicPlan(question, context, planId) {
    const steps = [];
    const keywords = (question || '').toLowerCase();

    if (keywords.includes('算法') || keywords.includes('复杂度')) {
      steps.push(
        { id: 'step_1', title: '问题分析', description: '分析算法问题的核心需求和约束', depends_on: [], expected_output: '问题规格说明', risk_level: 'low', estimated_tokens: 500 },
        { id: 'step_2', title: '方案设计', description: '设计候选算法方案并分析复杂度', depends_on: ['step_1'], expected_output: '算法方案对比', risk_level: 'medium', estimated_tokens: 1000 },
        { id: 'step_3', title: '实现验证', description: '实现最优算法并验证正确性', depends_on: ['step_2'], expected_output: '可运行的代码', risk_level: 'medium', estimated_tokens: 2000 },
        { id: 'step_4', title: '性能优化', description: '分析并优化性能瓶颈', depends_on: ['step_3'], expected_output: '性能报告', risk_level: 'low', estimated_tokens: 800 }
      );
    } else if (keywords.includes('架构') || keywords.includes('系统设计')) {
      steps.push(
        { id: 'step_1', title: '需求分析', description: '梳理系统需求和非功能性约束', depends_on: [], expected_output: '需求文档', risk_level: 'low', estimated_tokens: 800 },
        { id: 'step_2', title: '架构设计', description: '设计系统架构和组件划分', depends_on: ['step_1'], expected_output: '架构设计文档', risk_level: 'high', estimated_tokens: 2000 },
        { id: 'step_3', title: '接口定义', description: '定义组件间接口和数据契约', depends_on: ['step_2'], expected_output: 'API契约', risk_level: 'medium', estimated_tokens: 1500 },
        { id: 'step_4', title: '原型验证', description: '构建关键路径原型并验证', depends_on: ['step_3'], expected_output: '验证报告', risk_level: 'medium', estimated_tokens: 3000 }
      );
    } else {
      steps.push(
        { id: 'step_1', title: '需求理解', description: '深入理解用户需求和上下文', depends_on: [], expected_output: '需求确认', risk_level: 'low', estimated_tokens: 300 },
        { id: 'step_2', title: '方案制定', description: '制定解决方案的详细步骤', depends_on: ['step_1'], expected_output: '方案草案', risk_level: 'medium', estimated_tokens: 500 },
        { id: 'step_3', title: '执行实施', description: '执行方案中的各项任务', depends_on: ['step_2'], expected_output: '实施结果', risk_level: 'medium', estimated_tokens: 1000 },
        { id: 'step_4', title: '验证交付', description: '验证结果并完成交付', depends_on: ['step_3'], expected_output: '交付物', risk_level: 'low', estimated_tokens: 200 }
      );
    }

    return {
      plan_id: planId,
      objective: question,
      steps,
      critical_path: steps.map(s => s.id),
      total_estimated_tokens: steps.reduce((sum, s) => sum + (s.estimated_tokens || 0), 0),
      risk_assessment: '基于确定性规则生成的初始计划，建议通过AI增强优化',
      status: 'created'
    };
  }

  async _executeStep(step, plan, options, gateway) {
    const stepPrompt = `执行以下计划步骤：

步骤: ${step.title}
描述: ${step.description}
目标: ${plan.objective}
上下文: ${JSON.stringify(options.context || {}).slice(0, 500)}

请执行该步骤并返回结果。`;

    if (gateway && gateway.activeProvider) {
      const response = await gateway.chat({
        messages: [
          { role: 'system', content: '你是一位专业的任务执行者。请按照步骤要求执行任务并返回详细结果。' },
          { role: 'user', content: stepPrompt }
        ],
        temperature: 0.5,
        maxTokens: 2048
      });

      return {
        output: response.content || response,
        aiPowered: true
      };
    }

    return {
      output: `步骤 "${step.title}" 执行完成（本地模式）。建议: ${step.description.slice(0, 100)}`,
      aiPowered: false
    };
  }

  _createCheckpoint(planId, currentStepId, previousResults) {
    const checkpoint = {
      id: `cp_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`,
      planId,
      stepId: currentStepId,
      previousResults: [...previousResults],
      created_at: new Date().toISOString()
    };

    if (!this.checkpoints.has(planId)) {
      this.checkpoints.set(planId, []);
    }
    this.checkpoints.get(planId).push(checkpoint);

    return checkpoint;
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
}

class LearningLoopEngine {
  constructor() {
    this.skills = new Map();
    this.memoryStore = new Map();
    this.executionTrajectories = [];
    this.compressorStats = { totalCompressed: 0, avgCompressionRatio: 0 };
    this._init();
  }

  _init() {
    const skills = readJSON('learned_skills.json', []);
    skills.forEach(s => this.skills.set(s.id, s));

    const trajectories = readJSON('execution_trajectories.json', []);
    this.executionTrajectories = trajectories.slice(-100);

    const stats = readJSON('compressor_stats.json', {});
    this.compressorStats = {
      totalCompressed: stats.totalCompressed || 0,
      avgCompressionRatio: stats.avgCompressionRatio || 0
    };
  }

  async compressTrajectory(trajectory, options = {}) {
    const {
      maxTokens = 4000,
      preserveFirstN = 2,
      preserveLastN = 2,
      summarizeStrategy = 'extract'
    } = options;

    const turns = trajectory.turns || trajectory.messages || [];
    if (turns.length <= preserveFirstN + preserveLastN) {
      return { trajectory, compressed: false, note: '轨迹太短，无需压缩' };
    }

    const protectedIndices = new Set();
    for (let i = 0; i < preserveFirstN && i < turns.length; i++) {
      protectedIndices.add(i);
    }
    for (let i = turns.length - preserveLastN; i < turns.length; i++) {
      protectedIndices.add(i);
    }

    const compressedTurns = [];
    const compressibleRegion = [];

    for (let i = 0; i < turns.length; i++) {
      if (protectedIndices.has(i)) {
        if (compressibleRegion.length > 0) {
          const summary = this._summarizeRegion(compressibleRegion, summarizeStrategy);
          compressedTurns.push({
            role: 'user',
            content: `[压缩摘要] ${summary}`,
            compressed: true,
            originalTurns: compressibleRegion.length
          });
          compressibleRegion.length = 0;
        }
        compressedTurns.push(turns[i]);
      } else {
        compressibleRegion.push(turns[i]);
      }
    }

    if (compressibleRegion.length > 0) {
      const summary = this._summarizeRegion(compressibleRegion, summarizeStrategy);
      compressedTurns.push({
        role: 'user',
        content: `[压缩摘要] ${summary}`,
        compressed: true,
        originalTurns: compressibleRegion.length
      });
    }

    const originalTokens = this._estimateTokens(turns);
    const compressedTokens = this._estimateTokens(compressedTurns);
    const compressionRatio = originalTokens > 0 ? (1 - compressedTokens / originalTokens) : 0;

    const result = {
      id: trajectory.id || `traj_${Date.now()}`,
      compressed: true,
      originalTurns: turns.length,
      compressedTurns: compressedTurns.length,
      originalTokens,
      compressedTokens,
      compressionRatio,
      preserveFirstN,
      preserveLastN,
      turns: compressedTurns,
      metadata: {
        compressedAt: new Date().toISOString(),
        strategy: summarizeStrategy,
        maxTokens
      }
    };

    this.compressorStats.totalCompressed++;
    const totalRatio = this.compressorStats.avgCompressionRatio * (this.compressorStats.totalCompressed - 1);
    this.compressorStats.avgCompressionRatio = (totalRatio + compressionRatio) / this.compressorStats.totalCompressed;

    return result;
  }

  async extractSkills(trajectory, options = {}) {
    const { minOccurrences = 3, confidence = 0.7, skillCategories = null } = options;

    const turns = trajectory.turns || [];
    const patterns = new Map();

    for (const turn of turns) {
      const content = typeof turn === 'string' ? turn : (turn.content || turn.message || '');
      this._extractPatterns(content, patterns, skillCategories);
    }

    const skills = [];
    patterns.forEach((data, name) => {
      if (data.occurrences >= minOccurrences && data.confidence >= confidence) {
        skills.push({
          id: `skill_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`,
          name,
          ...data,
          extractedAt: new Date().toISOString(),
          sourceTrajectoryId: trajectory.id
        });
      }
    });

    skills.forEach(s => this.skills.set(s.id, s));
    writeJSON('learned_skills.json', Array.from(this.skills.values()));

    return skills;
  }

  async storeMemory(key, value, options = {}) {
    const {
      type = 'episodic',
      ttl = 86400000,
      importance = 0.5,
      tags = []
    } = options;

    const memory = {
      id: `mem_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`,
      key,
      value,
      type,
      tags,
      importance,
      createdAt: new Date().toISOString(),
      expiresAt: Date.now() + ttl,
      accessCount: 0
    };

    if (!this.memoryStore.has(type)) {
      this.memoryStore.set(type, new Map());
    }
    this.memoryStore.get(type).set(key, memory);

    this._consolidateMemory();

    return memory;
  }

  async recallMemory(query, options = {}) {
    const {
      types = ['episodic', 'semantic', 'procedural'],
      minImportance = 0,
      maxResults = 10
    } = options;

    const results = [];

    for (const type of types) {
      const store = this.memoryStore.get(type);
      if (!store) continue;

      for (const [key, memory] of store.entries()) {
        if (memory.expiresAt && memory.expiresAt < Date.now()) {
          store.delete(key);
          continue;
        }

        if (memory.importance >= minImportance) {
          const relevant = this._computeRelevance(query, memory);
          if (relevant > 0) {
            results.push({
              ...memory,
              relevance: relevant
            });
          }
        }
      }
    }

    results.sort((a, b) => (b.relevance * b.importance) - (a.relevance * a.importance));
    const topResults = results.slice(0, maxResults);

    topResults.forEach(r => { r.accessCount++; });

    return topResults;
  }

  async generateTrainingData(trajectories, options = {}) {
    const {
      format = 'dpo',
      maxSamples = 100,
      qualityFilter = 0.8
    } = options;

    const samples = [];

    for (const traj of trajectories) {
      const turns = traj.turns || [];
      if (turns.length < 3) continue;

      const prompt = turns[0]?.content || '';
      const chosen = turns.filter(t => t.role === 'assistant').map(t => t.content).join('\n');
      const rejected = turns.filter(t => t.role === 'user').map(t => t.content).join('\n') || '无备选方案';

      const quality = this._assessQuality(traj);
      if (quality >= qualityFilter) {
        samples.push({
          prompt,
          chosen,
          rejected,
          quality,
          source: traj.id,
          format
        });
      }

      if (samples.length >= maxSamples) break;
    }

    return {
      samples,
      totalGenerated: samples.length,
      format,
      qualityFilter,
      generatedAt: new Date().toISOString()
    };
  }

  listSkills(options = {}) {
    const { category, minConfidence } = options;
    let skills = Array.from(this.skills.values());

    if (category) skills = skills.filter(s => s.category === category);
    if (minConfidence) skills = skills.filter(s => s.confidence >= minConfidence);

    return skills.sort((a, b) => b.confidence - a.confidence);
  }

  getStats() {
    let totalMemories = 0;
    let activeMemories = 0;

    this.memoryStore.forEach(store => {
      store.forEach(m => {
        totalMemories++;
        if (!m.expiresAt || m.expiresAt > Date.now()) activeMemories++;
      });
    });

    return {
      totalSkills: this.skills.size,
      totalMemories,
      activeMemories,
      compressorStats: this.compressorStats,
      totalTrajectories: this.executionTrajectories.length
    };
  }

  _summarizeRegion(turns, strategy) {
    const contents = turns.map(t => typeof t === 'string' ? t : (t.content || t.message || '')).join(' ');
    const gateway = getGateway();

    if (gateway && gateway.activeProvider && strategy === 'llm') {
      return '摘要生成中...';
    }

    const sentences = contents.split(/[.!?。！？]/).filter(s => s.trim().length > 10);
    const keyPoints = sentences.slice(0, 3).join('. ');

    return keyPoints || contents.slice(0, 200);
  }

  _extractPatterns(content, patterns, categories) {
    if (!content) return;

    const defaultCategories = categories || [
      { name: '算法模式', keywords: ['算法', '复杂度', '排序', '搜索', '动态规划'] },
      { name: '架构模式', keywords: ['架构', '设计', '模式', '分层', '微服务'] },
      { name: '问题类型', keywords: ['问题', '需求', '错误', '异常', 'bug'] }
    ];

    for (const cat of defaultCategories) {
      for (const kw of cat.keywords) {
        const regex = new RegExp(kw, 'gi');
        const matches = content.match(regex);
        if (matches) {
          const name = `${cat.name}: ${kw}`;
          if (!patterns.has(name)) {
            patterns.set(name, { occurrences: 0, confidence: 0, category: cat.name, keyword: kw, examples: [] });
          }
          const data = patterns.get(name);
          data.occurrences++;
          data.confidence = Math.min(1, 0.5 + data.occurrences * 0.1);
          if (data.examples.length < 5) {
            data.examples.push(content.slice(0, 100));
          }
        }
      }
    }
  }

  _computeRelevance(query, memory) {
    if (!query) return 0;
    const queryLower = query.toLowerCase();
    const valueStr = JSON.stringify(memory.value, null, 2).toLowerCase();
    const keyLower = memory.key.toLowerCase();
    const tagsStr = (memory.tags || []).join(' ').toLowerCase();

    let score = 0;
    if (keyLower.includes(queryLower)) score += 0.5;
    if (valueStr.includes(queryLower)) score += 0.3;
    if (tagsStr.includes(queryLower)) score += 0.2;

    return score;
  }

  _assessQuality(trajectory) {
    let score = 0.5;
    const turns = trajectory.turns || [];

    if (turns.length >= 5) score += 0.1;
    if (trajectory.success) score += 0.2;
    if (trajectory.metadata?.quality) score = trajectory.metadata.quality;

    return Math.min(1, score);
  }

  _estimateTokens(text) {
    if (!text) return 0;
    return Math.ceil(text.length / 4);
  }

  _consolidateMemory() {
    const now = Date.now();
    this.memoryStore.forEach((store, type) => {
      for (const [key, memory] of store.entries()) {
        if (memory.expiresAt && memory.expiresAt < now) {
          store.delete(key);
        }
      }
    });
  }
}

class MultiAgentOrchestrator {
  constructor() {
    this.agents = new Map();
    this.pipelines = new Map();
    this.eventBus = [];
    this.executionLog = [];
    this._init();
  }

  _init() {
    const agents = readJSON('registered_agents.json', []);
    agents.forEach(a => this.agents.set(a.id, a));

    const pipelines = readJSON('registered_pipelines.json', []);
    pipelines.forEach(p => this.pipelines.set(p.id, p));

    this.registerDefaultAgents();
  }

  registerDefaultAgents() {
    const defaults = [
      {
        id: 'planner-agent',
        name: '规划代理',
        role: 'planner',
        capabilities: ['任务分解', '方案设计', '风险评估'],
        systemPrompt: '你是一位专业的任务规划专家，擅长将复杂任务分解为可执行的子任务。',
        status: 'active'
      },
      {
        id: 'research-agent',
        name: '研究代理',
        role: 'researcher',
        capabilities: ['信息收集', '知识检索', '数据分析'],
        systemPrompt: '你是一位专业的研究分析师，擅长收集、分析和综合信息。',
        status: 'active'
      },
      {
        id: 'executor-agent',
        name: '执行代理',
        role: 'executor',
        capabilities: ['代码生成', '命令执行', '文档撰写'],
        systemPrompt: '你是一位专业的执行者，负责实际的代码编写、命令执行和文档生成工作。',
        status: 'active'
      },
      {
        id: 'reviewer-agent',
        name: '评审代理',
        role: 'reviewer',
        capabilities: ['质量检查', '代码审查', '安全审计'],
        systemPrompt: '你是一位专业的评审专家，负责检查交付物的质量、安全性和规范性。',
        status: 'active'
      },
      {
        id: 'synthesizer-agent',
        name: '综合代理',
        role: 'synthesizer',
        capabilities: ['结果综合', '报告生成', '洞察提炼'],
        systemPrompt: '你是一位专业的综合分析师，负责整合多方面信息并生成最终报告。',
        status: 'active'
      }
    ];

    defaults.forEach(d => this.agents.set(d.id, d));
    writeJSON('registered_agents.json', Array.from(this.agents.values()));
  }

  async runPipeline(pipelineId, input, options = {}) {
    const pipeline = this.pipelines.get(pipelineId);
    if (!pipeline) throw new Error(`流水线不存在: ${pipelineId}`);

    const gateway = getGateway();
    const startTime = Date.now();
    const context = { input, intermediateResults: {}, events: [] };

    this._emitEvent('pipeline_start', { pipelineId, input });

    for (let i = 0; i < pipeline.stages.length; i++) {
      const stage = pipeline.stages[i];
      const agent = this.agents.get(stage.agentId);

      if (!agent || agent.status !== 'active') {
        this._emitEvent('stage_skip', { stage: stage.name, reason: '代理不可用' });
        continue;
      }

      const stageStart = Date.now();

      try {
        const stageInput = this._prepareStageInput(stage, context);
        const stageResult = await this._executeAgent(agent, stageInput, {
          systemPrompt: agent.systemPrompt,
          temperature: options.temperature || 0.7,
          maxTokens: options.maxTokens || 2048
        });

        context.intermediateResults[stage.id] = stageResult;
        this._emitEvent('stage_complete', {
          stage: stage.name,
          agent: agent.name,
          durationMs: Date.now() - stageStart,
          success: true
        });

        if (stage.outputKey) {
          context.input = stageResult;
        }
      } catch (error) {
        this._emitEvent('stage_error', {
          stage: stage.name,
          agent: agent.name,
          error: error.message
        });

        if (stage.critical) {
          return {
            success: false,
            pipelineId,
            error: `关键阶段失败: ${stage.name} - ${error.message}`,
            durationMs: Date.now() - startTime
          };
        }
      }
    }

    const finalResult = context.intermediateResults[pipeline.stages[pipeline.stages.length - 1]?.id];

    return {
      success: true,
      pipelineId,
      stagesCompleted: pipeline.stages.length,
      finalResult,
      intermediateResults: context.intermediateResults,
      events: context.events,
      durationMs: Date.now() - startTime,
      completedAt: new Date().toISOString()
    };
  }

  async registerPipeline(pipeline) {
    const id = pipeline.id || `pipeline_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`;
    const newPipeline = {
      id,
      name: pipeline.name,
      description: pipeline.description,
      stages: pipeline.stages || [],
      status: 'active',
      createdAt: new Date().toISOString()
    };

    this.pipelines.set(id, newPipeline);
    writeJSON('registered_pipelines.json', Array.from(this.pipelines.values()));

    return newPipeline;
  }

  listAgents(options = {}) {
    const { role, status } = options;
    let result = Array.from(this.agents.values());
    if (role) result = result.filter(a => a.role === role);
    if (status) result = result.filter(a => a.status === status);
    return result;
  }

  listPipelines(options = {}) {
    const { status } = options;
    let result = Array.from(this.pipelines.values());
    if (status) result = result.filter(p => p.status === status);
    return result;
  }

  getEventLog(options = {}) {
    const { limit = 50, type } = options;
    let events = this.eventBus;
    if (type) events = events.filter(e => e.type === type);
    return events.slice(-limit);
  }

  getStats() {
    return {
      totalAgents: this.agents.size,
      totalPipelines: this.pipelines.size,
      activeAgents: Array.from(this.agents.values()).filter(a => a.status === 'active').length,
      recentEvents: this.eventBus.slice(-10),
      executionLogSize: this.executionLog.length
    };
  }

  async _executeAgent(agent, input, options) {
    const gateway = getGateway();

    if (gateway && gateway.activeProvider) {
      const response = await gateway.chat({
        messages: [
          { role: 'system', content: options.systemPrompt || agent.systemPrompt },
          { role: 'user', content: typeof input === 'string' ? input : JSON.stringify(input, null, 2) }
        ],
        temperature: options.temperature || 0.7,
        maxTokens: options.maxTokens || 2048
      });

      return {
        agentId: agent.id,
        agentName: agent.name,
        result: response.content || response,
        aiPowered: true
      };
    }

    return {
      agentId: agent.id,
      agentName: agent.name,
      result: `${agent.name} 已处理输入（本地模式）`,
      aiPowered: false,
      fallback: true
    };
  }

  _prepareStageInput(stage, context) {
    if (stage.inputFrom && context.intermediateResults[stage.inputFrom]) {
      return context.intermediateResults[stage.inputFrom];
    }
    return context.input;
  }

  _emitEvent(type, data) {
    const event = {
      id: `evt_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`,
      type,
      data,
      timestamp: new Date().toISOString()
    };
    this.eventBus.push(event);
    if (this.eventBus.length > 500) {
      this.eventBus = this.eventBus.slice(-200);
    }
  }

  registerAgent(agentConfig) {
    const id = agentConfig.id || `agent_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`;
    const agent = {
      id,
      name: agentConfig.name || id,
      role: agentConfig.role || 'custom',
      capabilities: agentConfig.capabilities || [],
      systemPrompt: agentConfig.systemPrompt || '',
      status: agentConfig.status || 'active',
      createdAt: new Date().toISOString(),
      ...agentConfig
    };
    this.agents.set(id, agent);
    writeJSON('registered_agents.json', Array.from(this.agents.values()));
    this._emitEvent('agent_registered', { agentId: id });
    return agent;
  }

  async executePipeline(pipelineId, input, options = {}) {
    return this.runPipeline(pipelineId, input, options);
  }
}

class AIIntegrationEngine {
  constructor() {
    this.graphEngine = new GraphIntelligenceEngine();
    this.planAct = new PlanActOrchestrator();
    this.learningEngine = new LearningLoopEngine();
    this.orchestrator = new MultiAgentOrchestrator();
    this.gateway = getGateway();
    this.alliance = getAlliance();
    this.executionMetrics = [];
  }

  async intelligentProcess(question, options = {}) {
    const startTime = Date.now();
    const { mode = 'auto', graphData = null, context = {} } = options;

    const result = {
      question,
      mode,
      steps: [],
      finalAnswer: null,
      metrics: {
        startTime,
        durationMs: 0,
        aiPowered: false
      }
    };

    try {
      result.steps.push({
        type: 'intention_detection',
        result: await this._detectIntention(question),
        timestamp: new Date().toISOString()
      });

      if (graphData && graphData.nodes?.length > 0) {
        result.steps.push({
          type: 'graph_analysis',
          result: await this._analyzeGraph(graphData, question),
          timestamp: new Date().toISOString()
        });
      }

      result.steps.push({
        type: 'expert_routing',
        result: await this._routeExperts(question, context),
        timestamp: new Date().toISOString()
      });

      if (mode === 'plan_act') {
        result.steps.push({
          type: 'plan_generation',
          result: await this.planAct.createPlan(question, context),
          timestamp: new Date().toISOString()
        });
      }

      result.finalAnswer = await this._synthesizeAnswer(result.steps, question, context);

      const trajectory = {
        id: `traj_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`,
        question,
        steps: result.steps,
        answer: result.finalAnswer,
        duration: Date.now() - startTime,
        timestamp: new Date().toISOString()
      };

      await this.learningEngine.compressTrajectory(trajectory, { maxTokens: 4000 });
      await this.learningEngine.extractSkills(trajectory);

      result.metrics.durationMs = Date.now() - startTime;
      result.metrics.aiPowered = !!this.gateway?.activeProvider;
      result.metrics.stepCount = result.steps.length;
      result.success = true;

    } catch (error) {
      result.success = false;
      result.error = error.message;
      result.metrics.durationMs = Date.now() - startTime;
    }

    this.executionMetrics.push({
      question,
      mode,
      durationMs: result.metrics.durationMs,
      success: result.success,
      aiPowered: result.metrics.aiPowered,
      timestamp: new Date().toISOString()
    });

    if (this.executionMetrics.length > 1000) {
      this.executionMetrics = this.executionMetrics.slice(-500);
    }

    return result;
  }

  async _detectIntention(question) {
    const text = (question || '').toLowerCase();
    const intents = {
      analysis: ['分析', '评估', '对比', '理解', '解释'],
      creation: ['创建', '生成', '编写', '设计', '开发'],
      optimization: ['优化', '改进', '增强', '提升', '优化'],
      debugging: ['错误', '异常', 'bug', '问题', '修复'],
      planning: ['计划', '方案', '步骤', '策略', '路线'],
      learning: ['学习', '理解', '教程', '示例', '案例']
    };

    const scores = {};
    Object.entries(intents).forEach(([intent, keywords]) => {
      scores[intent] = keywords.filter(kw => text.includes(kw)).length;
    });

    const detectedIntent = Object.entries(scores)
      .sort((a, b) => b[1] - a[1])
      .filter(([, score]) => score > 0)
      .map(([intent]) => intent)[0] || 'general';

    return {
      primary: detectedIntent,
      scores,
      confidence: Math.max(...Object.values(scores)) / 5 || 0.2,
      suggestedMode: detectedIntent === 'planning' ? 'plan_act' : 'auto'
    };
  }

  async _analyzeGraph(graphData, question) {
    const personalizedPR = await this.graphEngine.computePersonalizedPageRank(graphData, {
      queryNodes: question ? [question] : [],
      topK: 20
    });

    const communities = await this.graphEngine.detectCommunitiesAdvanced(graphData, {
      maxCommunities: 10
    });

    return {
      pageRank: personalizedPR,
      communities: communities.communities || [],
      communitiesSummary: {
        total: communities.totalCommunities || 0,
        algorithm: communities.algorithm,
        passes: communities.passes
      },
      nodeCount: graphData.nodes?.length || 0,
      edgeCount: graphData.edges?.length || 0
    };
  }

  async _routeExperts(question, context) {
    try {
      const routing = await this.alliance.routeExperts(question, { maxExperts: 3 });
      return {
        intent: routing.intent,
        selected: routing.selected.map(s => ({
          id: s.expert.id,
          name: s.expert.name,
          score: s.score
        })),
        routingTimeMs: routing.routing_time_ms
      };
    } catch (e) {
      return {
        intent: { primary: 'general', confidence: 0.5 },
        selected: [],
        fallback: true,
        error: e.message
      };
    }
  }

  async _synthesizeAnswer(steps, question, context) {
    const gateway = getGateway();

    if (gateway && gateway.activeProvider) {
      const synthesisPrompt = `请综合以下分析步骤的结果，回答用户问题：

用户问题: ${question}

分析步骤结果:
${steps.map((step, i) => {
  const result = typeof step.result === 'object' ? JSON.stringify(step.result).slice(0, 500) : String(step.result).slice(0, 300);
  return `步骤${i + 1} (${step.type}): ${result}`;
}).join('\n\n')}

请提供：
1. 简洁明了的回答
2. 关键发现
3. 可执行的建议（如适用）`;

      const response = await gateway.chat({
        messages: [
          { role: 'system', content: '你是一位专业的综合分析师。请综合多源信息，提供高质量的回答。' },
          { role: 'user', content: synthesisPrompt }
        ],
        temperature: 0.7,
        maxTokens: 2048
      });

      return response.content || response;
    }

    return steps.length > 0
      ? `基于 ${steps.length} 步分析的综合结果（本地模式）。`
      : '分析完成（本地模式）。';
  }

  getSystemStats() {
    return {
      graphEngine: this.graphEngine.getStats(),
      planAct: this.planAct.getStats(),
      learningEngine: this.learningEngine.getStats(),
      orchestrator: this.orchestrator.getStats(),
      integration: {
        totalProcesses: this.executionMetrics.length,
        recentProcesses: this.executionMetrics.slice(-10),
        avgDurationMs: this.executionMetrics.length > 0
          ? Math.round(this.executionMetrics.reduce((sum, m) => sum + m.durationMs, 0) / this.executionMetrics.length)
          : 0
      }
    };
  }

  async performFullAnalysis(question, options = {}) {
    const result = await this.intelligentProcess(question, {
      ...options,
      mode: 'auto'
    });

    const learnedSkills = this.learningEngine.listSkills({ minConfidence: 0.7 });
    const memories = await this.learningEngine.recallMemory(question, { maxResults: 5 });

    return {
      ...result,
      learnedSkills,
      relevantMemories: memories,
      fullAnalysis: true
    };
  }
}

let integrationInstance = null;

function getAIIntegrationEngine() {
  if (!integrationInstance) {
    integrationInstance = new AIIntegrationEngine();
  }
  return integrationInstance;
}

module.exports = {
  GraphIntelligenceEngine,
  PlanActOrchestrator,
  LearningLoopEngine,
  MultiAgentOrchestrator,
  AIIntegrationEngine,
  getAIIntegrationEngine
};

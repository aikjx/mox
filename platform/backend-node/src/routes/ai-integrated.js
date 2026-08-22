'use strict';

/**
 * 路由域：智能集成引擎
 * /ai/integrated/* 计划、技能、记忆、智能体、流水线、一键全链
 */
module.exports = function registerAiIntegratedRoutes(ctx) {
  const { aiIntegration, config, ok, fail, readBody, reg, pagerank } = ctx;

  // ===== AI 智能集成引擎路由 =====
  reg('post', '/ai/integrated/process', async (req, res) => {
    const body = await readBody(req);
    const question = body.question || body.text || '';
    const mode = body.mode || 'auto';
    const options = body.options || {};
    try {
      const result = await aiIntegration.intelligentProcess(question, { ...options, mode });
      ok(res, result);
    } catch (e) {
      console.error('[ai-integrated-process]', e);
      fail(res, 500, 'AI 智能处理失败: ' + e.message);
    }
  });

  reg('post', '/ai/integrated/full-analysis', async (req, res) => {
    const body = await readBody(req);
    const question = body.question || body.text || '';
    const options = body.options || {};
    try {
      const result = await aiIntegration.performFullAnalysis(question, options);
      ok(res, result);
    } catch (e) {
      console.error('[ai-integrated-full-analysis]', e);
      fail(res, 500, '全维分析失败: ' + e.message);
    }
  });

  reg('get', '/ai/integrated/stats', (req, res) => {
    try {
      const stats = aiIntegration.getSystemStats();
      ok(res, stats);
    } catch (e) {
      console.error('[ai-integrated-stats]', e);
      fail(res, 500, '获取系统统计失败: ' + e.message);
    }
  });

  reg('post', '/ai/integrated/graph-intelligence', async (req, res) => {
    const body = await readBody(req);
    const graphData = body.graph || { nodes: [], edges: [] };
    const question = body.question || '';
    try {
      const [pagerank, communities] = await Promise.all([
        aiIntegration.graphEngine.computePersonalizedPageRank(graphData, { topK: 20 }),
        aiIntegration.graphEngine.detectCommunitiesAdvanced(graphData, { maxCommunities: 10 })
      ]);
      ok(res, {
        personalizedPageRank: pagerank,
        communities,
        analysisTime: Date.now()
      });
    } catch (e) {
      console.error('[ai-graph-intelligence]', e);
      fail(res, 500, '图计算失败: ' + e.message);
    }
  });

  reg('post', '/ai/integrated/plan-create', async (req, res) => {
    const body = await readBody(req);
    const goal = body.goal || body.question || '';
    const context = body.context || {};
    const options = body.options || {};
    try {
      const plan = await aiIntegration.planAct.createPlan(goal, context, options);
      ok(res, plan);
    } catch (e) {
      console.error('[ai-plan-create]', e);
      fail(res, 500, '创建计划失败: ' + e.message);
    }
  });

  reg('post', '/ai/integrated/plan-execute', async (req, res) => {
    const body = await readBody(req);
    const planId = body.plan_id || '';
    const options = body.options || {};
    try {
      const plan = await aiIntegration.planAct.executePlan(planId, options);
      ok(res, plan);
    } catch (e) {
      console.error('[ai-plan-execute]', e);
      fail(res, 500, '执行计划失败: ' + e.message);
    }
  });

  reg('get', '/ai/integrated/plans', (req, res) => {
    try {
      const plans = aiIntegration.planAct.listPlans();
      ok(res, plans);
    } catch (e) {
      console.error('[ai-plans]', e);
      fail(res, 500, '获取计划列表失败: ' + e.message);
    }
  });

  reg('post', '/ai/integrated/plan-rollback', async (req, res) => {
    const body = await readBody(req);
    const planId = body.plan_id || '';
    const checkpointId = body.checkpoint_id || '';
    try {
      const result = await aiIntegration.planAct.rollbackToCheckpoint(planId, checkpointId);
      ok(res, result);
    } catch (e) {
      console.error('[ai-plan-rollback]', e);
      fail(res, 500, '回滚计划失败: ' + e.message);
    }
  });

  reg('post', '/ai/integrated/skill-extract', async (req, res) => {
    const body = await readBody(req);
    const trajectory = body.trajectory || {};
    const options = body.options || {};
    try {
      const skills = await aiIntegration.learningEngine.extractSkills(trajectory, options);
      ok(res, skills);
    } catch (e) {
      console.error('[ai-skill-extract]', e);
      fail(res, 500, '技能提取失败: ' + e.message);
    }
  });

  reg('get', '/ai/integrated/skills', (req, res) => {
    try {
      const skills = aiIntegration.learningEngine.listSkills();
      ok(res, skills);
    } catch (e) {
      console.error('[ai-skills]', e);
      fail(res, 500, '获取技能列表失败: ' + e.message);
    }
  });

  reg('post', '/ai/integrated/memory-recall', async (req, res) => {
    const body = await readBody(req);
    const query = body.query || '';
    const options = body.options || {};
    try {
      const memories = await aiIntegration.learningEngine.recallMemory(query, options);
      ok(res, memories);
    } catch (e) {
      console.error('[ai-memory-recall]', e);
      fail(res, 500, '记忆召回失败: ' + e.message);
    }
  });

  reg('post', '/ai/integrated/agent-register', async (req, res) => {
    const body = await readBody(req);
    const agentConfig = body.agent || body.config || {};
    try {
      const agent = aiIntegration.orchestrator.registerAgent(agentConfig);
      ok(res, agent);
    } catch (e) {
      console.error('[ai-agent-register]', e);
      fail(res, 500, '注册智能体失败: ' + e.message);
    }
  });

  reg('post', '/ai/integrated/pipeline-execute', async (req, res) => {
    const body = await readBody(req);
    const pipelineId = body.pipeline_id || '';
    const input = body.input || {};
    const options = body.options || {};
    try {
      const result = await aiIntegration.orchestrator.executePipeline(pipelineId, input, options);
      ok(res, result);
    } catch (e) {
      console.error('[ai-pipeline-execute]', e);
      fail(res, 500, '执行流水线失败: ' + e.message);
    }
  });

  reg('post', '/ai/integrated/pipeline-register', async (req, res) => {
    const body = await readBody(req);
    const pipeline = body.pipeline || body.config || {};
    try {
      const result = await aiIntegration.orchestrator.registerPipeline(pipeline);
      ok(res, result);
    } catch (e) {
      console.error('[ai-pipeline-register]', e);
      fail(res, 500, '注册流水线失败: ' + e.message);
    }
  });

  reg('get', '/ai/integrated/pipelines', (req, res) => {
    try {
      const pipelines = aiIntegration.orchestrator.listPipelines();
      ok(res, pipelines);
    } catch (e) {
      console.error('[ai-pipelines]', e);
      fail(res, 500, '获取流水线列表失败: ' + e.message);
    }
  });

  reg('get', '/ai/integrated/agents', (req, res) => {
    try {
      const agents = aiIntegration.orchestrator.listAgents();
      ok(res, agents);
    } catch (e) {
      console.error('[ai-agents]', e);
      fail(res, 500, '获取智能体列表失败: ' + e.message);
    }
  });

  reg('post', '/ai/integrated/memory-store', async (req, res) => {
    const body = await readBody(req);
    const key = body.key || '';
    const value = body.value || {};
    const options = body.options || {};
    try {
      const memory = await aiIntegration.learningEngine.storeMemory(key, value, options);
      ok(res, memory);
    } catch (e) {
      console.error('[ai-memory-store]', e);
      fail(res, 500, '存储记忆失败: ' + e.message);
    }
  });

  reg('post', '/ai/integrated/trajectory-compress', async (req, res) => {
    const body = await readBody(req);
    const trajectory = body.trajectory || {};
    const options = body.options || {};
    try {
      const result = await aiIntegration.learningEngine.compressTrajectory(trajectory, options);
      ok(res, result);
    } catch (e) {
      console.error('[ai-trajectory-compress]', e);
      fail(res, 500, '轨迹压缩失败: ' + e.message);
    }
  });

  reg('post', '/ai/integrated/one-shot', async (req, res) => {
    const body = await readBody(req);
    const question = body.question || body.text || '';
    const graphData = body.graph || null;
    const context = body.context || {};
    try {
      const results = { start: Date.now(), question };
      const errors = [];

      if (graphData && graphData.nodes) {
        try {
          results.graphIntelligence = await aiIntegration._analyzeGraph(graphData, question);
        } catch (e) {
          errors.push('graph: ' + e.message);
        }
      }

      try {
        results.expertRouting = await aiIntegration._routeExperts(question, context);
      } catch (e) {
        errors.push('routing: ' + e.message);
      }

      try {
        const processResult = await aiIntegration.intelligentProcess(question, { mode: 'auto' });
        results.intelligentProcess = {
          answer: processResult.answer || processResult.result || '',
          steps: processResult.steps?.length || 0,
          mode: processResult.mode,
          durationMs: processResult.metrics?.durationMs
        };
      } catch (e) {
        errors.push('process: ' + e.message);
      }

      try {
        const memories = await aiIntegration.learningEngine.recallMemory(question, { maxResults: 3 });
        results.memories = memories;
      } catch (e) {
        errors.push('memory: ' + e.message);
      }

      try {
        const skills = aiIntegration.learningEngine.listSkills({ minConfidence: 0.5 });
        results.relevantSkills = skills.slice(0, 5);
      } catch (e) {
        errors.push('skills: ' + e.message);
      }

      results.completedAt = new Date().toISOString();
      results.totalDurationMs = Date.now() - results.start;
      results.success = errors.length === 0;
      results.errors = errors;

      delete results.start;

      ok(res, results);
    } catch (e) {
      console.error('[ai-one-shot]', e);
      fail(res, 500, '一键集成处理失败: ' + e.message);
    }
  });

  reg('get', '/ai/integrated/health', (req, res) => {
    try {
      const stats = aiIntegration.getSystemStats();
      const healthScore = Math.min(100,
        (stats.orchestrator.activeAgents / Math.max(stats.orchestrator.totalAgents, 1)) * 40 +
        (stats.integration.totalProcesses > 0 ? 30 : 10) +
        (stats.learningEngine.totalSkills > 0 ? 30 : 15)
      );
      ok(res, {
        status: 'healthy',
        healthScore: Math.round(healthScore),
        components: {
          graphEngine: stats.graphEngine.totalGraphsProcessed > 0 ? 'active' : 'idle',
          planAct: stats.planAct.totalPlans > 0 ? 'active' : 'idle',
          learningEngine: stats.learningEngine.totalSkills > 0 ? 'active' : 'idle',
          orchestrator: stats.orchestrator.activeAgents > 0 ? 'active' : 'warning'
        },
        activeAgents: stats.orchestrator.activeAgents,
        totalProcesses: stats.integration.totalProcesses,
        avgDurationMs: stats.integration.avgDurationMs,
        learnedSkills: stats.learningEngine.totalSkills,
        uptime: process.uptime()
      });
    } catch (e) {
      console.error('[ai-integrated-health]', e);
      fail(res, 500, '获取健康状态失败: ' + e.message);
    }
  });

};

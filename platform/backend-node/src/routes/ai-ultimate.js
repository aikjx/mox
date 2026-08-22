'use strict';

/**
 * 路由域：终极 AI 引擎
 * /ai/ultimate/* 推理、类比、熔断、规则与性能
 */
module.exports = function registerAiUltimateRoutes(ctx) {
  const { ultimateEngine, config, ok, fail, readBody, reg } = ctx;

  // ===== 终极AI引擎路由 =====
  reg('post', '/ai/ultimate/process', async (req, res) => {
    const body = await readBody(req);
    const question = body.question || body.text || '';
    const options = body.options || {};
    try {
      const result = await ultimateEngine.processWithDeepIntelligence(question, options);
      ok(res, result);
    } catch (e) {
      console.error('[ultimate-process]', e);
      fail(res, 500, '终极处理失败: ' + e.message);
    }
  });

  reg('post', '/ai/ultimate/full-analysis', async (req, res) => {
    const body = await readBody(req);
    const question = body.question || body.text || '';
    const options = body.options || {};
    try {
      const result = await ultimateEngine.performFullUltimateAnalysis(question, options);
      ok(res, result);
    } catch (e) {
      console.error('[ultimate-full-analysis]', e);
      fail(res, 500, '终极分析失败: ' + e.message);
    }
  });

  reg('get', '/ai/ultimate/stats', (req, res) => {
    try {
      const stats = ultimateEngine.getUltimateStats();
      ok(res, stats);
    } catch (e) {
      console.error('[ultimate-stats]', e);
      fail(res, 500, '获取终极统计失败: ' + e.message);
    }
  });

  reg('get', '/ai/ultimate/health', (req, res) => {
    try {
      const stats = ultimateEngine.getUltimateStats();
      const healthScore = Math.min(100,
        (stats.vectorStore.totalVectors > 0 ? 20 : 10) +
        (stats.processingHistory.total > 0 ? 25 : 5) +
        (stats.vectorStore.dimensions >= 128 ? 15 : 5) +
        (stats.performance.successRate > 0.8 ? 25 : stats.performance.successRate * 30) +
        (stats.graphReasoner.rulesCount >= 5 ? 15 : 5)
      );
      ok(res, {
        status: 'ultimate',
        healthScore: Math.round(healthScore),
        version: '2.0.0',
        engine: stats.engine,
        components: stats.integrations,
        performance: stats.performance,
        vectorStore: {
          vectors: stats.vectorStore.totalVectors,
          dimensions: stats.vectorStore.dimensions
        },
        processingHistory: stats.processingHistory.total,
        uptime: process.uptime()
      });
    } catch (e) {
      console.error('[ultimate-health]', e);
      fail(res, 500, '获取终极健康状态失败: ' + e.message);
    }
  });

  reg('post', '/ai/ultimate/reasoning', async (req, res) => {
    const body = await readBody(req);
    const question = body.question || body.text || '';
    const options = body.options || {};
    try {
      const reasoning = await ultimateEngine.reasoningEngine.multiStepReasoning(question, options);
      if (options.self_reflect !== false) {
        const reflected = await ultimateEngine.reasoningEngine.selfReflect(reasoning, question, options);
        ok(res, reflected);
      } else {
        ok(res, reasoning);
      }
    } catch (e) {
      console.error('[ultimate-reasoning]', e);
      fail(res, 500, '深度推理失败: ' + e.message);
    }
  });

  reg('post', '/ai/ultimate/analogical', async (req, res) => {
    const body = await readBody(req);
    const sourceDomain = body.source_domain || body.source || '';
    const targetDomain = body.target_domain || body.target || '';
    const question = body.question || '';
    try {
      const result = await ultimateEngine.reasonByAnalogy(sourceDomain, targetDomain, question);
      ok(res, result);
    } catch (e) {
      console.error('[ultimate-analogical]', e);
      fail(res, 500, '类比推理失败: ' + e.message);
    }
  });

  reg('post', '/ai/ultimate/store', async (req, res) => {
    const body = await readBody(req);
    const id = body.id || `kv_${Date.now()}`;
    const content = body.content || body.text || '';
    const metadata = body.metadata || {};
    try {
      const result = await ultimateEngine.storeKnowledge(id, content, metadata);
      ok(res, result);
    } catch (e) {
      console.error('[ultimate-store]', e);
      fail(res, 500, '存储知识失败: ' + e.message);
    }
  });

  reg('post', '/ai/ultimate/search', async (req, res) => {
    const body = await readBody(req);
    const query = body.query || body.question || '';
    const options = body.options || {};
    try {
      const results = await ultimateEngine.searchKnowledge(query, options);
      ok(res, { query, results, totalMatches: results.length });
    } catch (e) {
      console.error('[ultimate-search]', e);
      fail(res, 500, '搜索知识失败: ' + e.message);
    }
  });

  reg('post', '/ai/ultimate/optimize-prompt', async (req, res) => {
    const body = await readBody(req);
    const prompt = body.prompt || '';
    const target = body.target || 'concise';
    try {
      const optimized = ultimateEngine.optimizer.optimizePrompt(prompt, target);
      ok(res, { original: prompt, optimized, target });
    } catch (e) {
      console.error('[ultimate-optimize]', e);
      fail(res, 500, '优化Prompt失败: ' + e.message);
    }
  });

  reg('get', '/ai/ultimate/performance', (req, res) => {
    try {
      const report = ultimateEngine.optimizer.getPerformanceReport();
      ok(res, report);
    } catch (e) {
      console.error('[ultimate-performance]', e);
      fail(res, 500, '获取性能报告失败: ' + e.message);
    }
  });

  reg('get', '/ai/ultimate/circuit-breaker', (req, res) => {
    try {
      const status = ultimateEngine.optimizer.getCircuitStatus();
      ok(res, status);
    } catch (e) {
      console.error('[ultimate-circuit]', e);
      fail(res, 500, '获取熔断器状态失败: ' + e.message);
    }
  });

  reg('post', '/ai/ultimate/reasoning-rules', async (req, res) => {
    const body = await readBody(req);
    const rule = body.rule || body.config || {};
    try {
      ultimateEngine.addReasoningRule(rule);
      ok(res, { success: true, rule });
    } catch (e) {
      console.error('[ultimate-rule]', e);
      fail(res, 500, '添加推理规则失败: ' + e.message);
    }
  });

  reg('get', '/ai/ultimate/reasoning-rules', (req, res) => {
    try {
      const stats = ultimateEngine.getUltimateStats();
      ok(res, {
        rulesCount: stats.graphReasoner.rulesCount,
        engine: 'KnowledgeGraphReasoner'
      });
    } catch (e) {
      console.error('[ultimate-rules-list]', e);
      fail(res, 500, '获取规则列表失败: ' + e.message);
    }
  });

};

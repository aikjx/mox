'use strict';

/**
 * 路由域：16 模块 AI 增强
 * /workbench|operators|graph|resources|workflow|plugins|browser|monitor|docs|market|mcp|automation|caomei|algolab|fusion 16 模块 AI 端点
 */
module.exports = function registerAiEnhancedRoutes(ctx) {
  const { gateway, alliance, modules, ok, fail, readBody, reg } = ctx;

  // ===== 16模块 AI 增强端点 =====
  // 工作台 AI 概览
  reg('get', '/workbench/ai-overview', async (req, res) => {
    try {
      const status = await gateway.chat({
        messages: [{ role: 'user', content: '请分析当前系统状态并生成工作台概览' }],
        expertType: 'architecture'
      });
      ok(res, {
        timestamp: new Date().toISOString(),
        expert_analysis: status.content,
        system_metrics: {
          modules: 16,
          experts: alliance.listExperts().length,
          providers: gateway.listProviders().length
        }
      });
    } catch (e) {
      ok(res, { timestamp: new Date().toISOString(), error: e.message });
    }
  });

  // 算子中心 AI 推荐
  reg('post', '/operators/ai-recommend', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.requirement || '请推荐适合的算子' }],
        expertType: 'operator'
      });
      ok(res, {
        recommendations: result.metadata?.related_operators || [],
        analysis: result.content,
        confidence: result.metadata?.confidence
      });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 知识图谱 AI 洞察
  reg('post', '/graph/ai-insights', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请分析图谱数据：${JSON.stringify(body.graph_summary || {})}` }],
        expertType: 'graph'
      });
      ok(res, { insights: result.content, metadata: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // AI 助手 - 专家模式
  reg('post', '/ai/expert-chat', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: body.messages || [],
        expertType: body.expert_type,
        systemPrompt: body.system_prompt,
        sessionId: body.session_id,
        temperature: body.temperature,
        maxTokens: body.max_tokens
      });
      ok(res, {
        response: result.content,
        metadata: result.metadata,
        usage: result.usage
      });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 资源管理 AI 分析
  reg('post', '/resources/ai-analysis', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请分析资源状况：${JSON.stringify(body.resources || {})}` }],
        expertType: 'architecture'
      });
      ok(res, { analysis: result.content, recommendations: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 工作流编排 AI 生成
  reg('post', '/workflow/ai-generate', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.requirement || '请生成工作流' }],
        expertType: 'workflow'
      });
      ok(res, { workflow_design: result.content, metadata: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // AI 插件智能路由
  reg('post', '/plugins/ai-route', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.request || '请路由到合适的插件' }],
        expertType: 'automation'
      });
      ok(res, { routing_decision: result.content, target_plugins: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 浏览器自动化 AI 指令
  reg('post', '/browser/ai-instruct', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.instruction || '请执行浏览器操作' }],
        expertType: 'automation'
      });
      ok(res, { parsed_instruction: result.content, steps: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 系统监控 AI 诊断
  reg('post', '/monitor/ai-diagnose', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请诊断系统状态：${JSON.stringify(body.metrics || {})}` }],
        expertType: 'monitor'
      });
      ok(res, { diagnosis: result.content, severity: result.metadata?.confidence });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // API 文档 AI 解释
  reg('post', '/docs/ai-explain', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请解释 API：${body.endpoint || body.text || ''}` }],
        expertType: 'architecture'
      });
      ok(res, { explanation: result.content, examples: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 算子商城 AI 搜索
  reg('post', '/market/ai-search', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.requirement || body.query || '请搜索算子' }],
        expertType: 'market'
      });
      ok(res, { search_results: result.content, relevant_items: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // MCP 兼容 AI 映射
  reg('post', '/mcp/ai-map', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请映射 MCP 工具：${JSON.stringify(body.tools || {})}` }],
        expertType: 'mcp'
      });
      ok(res, { mapping: result.content, compatibility: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // AI 自动化智能执行
  reg('post', '/automation/ai-execute', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.task || '请执行自动化任务' }],
        expertType: 'automation'
      });
      ok(res, { execution_plan: result.content, steps: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 需求编译 AI 解析
  reg('post', '/caomei/ai-parse', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.requirement || body.text || '' }],
        expertType: 'requirement'
      });
      ok(res, { parsed_requirement: result.content, structure: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 算法实验室 AI 分析
  reg('post', '/algolab/ai-analyze', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请分析算法：${body.algorithm || body.code || ''}` }],
        expertType: 'algorithm'
      });
      ok(res, { analysis: result.content, complexity: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 全维融合 AI 治理
  reg('post', '/fusion/ai-govern', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请进行全维治理：${JSON.stringify(body.fusion_data || {})}` }],
        expertType: 'fusion'
      });
      ok(res, { governance_report: result.content, dimensions: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

};

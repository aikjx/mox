'use strict';

/**
 * 路由域：专家联盟
 * /experts/* 专家 CRUD、咨询、辩论、联盟引擎、会话、调度策略
 */
module.exports = function registerExpertAllianceRoutes(ctx) {
  const { url, alliance, sessionStore, dispatcher, config, ok, fail, readBody, appendLog, reg, getAllianceEngine } = ctx;

  // ===== 专家联盟路由 =====
  reg('get', '/experts', async (req, res) => {
    const q = url.parse(req.url, true).query;
    const experts = alliance.listExperts({
      type: q.type,
      status: q.status,
      keyword: q.q
    });
    ok(res, experts);
  });

  reg('get', '/ai/experts', async (req, res) => {
    const q = url.parse(req.url, true).query;
    const experts = alliance.listExperts({
      type: q.type,
      status: q.status,
      keyword: q.q
    });
    ok(res, experts);
  });

  reg('get', '/experts/:id', (req, res, params) => {
    const expert = alliance.getExpert(params.id);
    if (expert) {
      ok(res, expert);
    } else {
      fail(res, 404, 'Expert not found');
    }
  });

  reg('post', '/experts', async (req, res) => {
    const body = await readBody(req);
    const expert = alliance.registerExpert(body);
    appendLog({ type: 'expert', msg: 'register ' + expert.name, id: expert.id });
    ok(res, expert);
  });

  reg('put', '/experts/:id', async (req, res, params) => {
    const body = await readBody(req);
    const expert = alliance.updateExpert(params.id, body);
    if (expert) {
      ok(res, expert);
    } else {
      fail(res, 404, 'Expert not found');
    }
  });

  reg('delete', '/experts/:id', (req, res, params) => {
    const success = alliance.removeExpert(params.id);
    if (success) {
      ok(res, { success: true });
    } else {
      fail(res, 404, 'Expert not found');
    }
  });

  reg('post', '/experts/:id/consult', async (req, res, params) => {
    const body = await readBody(req);
    try {
      const result = await alliance.consult(params.id, body.messages || [], {
        sessionId: body.sessionId,
        useCustomPrompt: body.useCustomPrompt,
        systemPrompt: body.systemPrompt,
        temperature: body.temperature,
        maxTokens: body.maxTokens
      });
      appendLog({ type: 'expert', msg: 'consult ' + params.id, tokens: 1 });
      ok(res, result);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('post', '/experts/multi-consult', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await alliance.multiExpertConsult(
        body.question || body.message || '',
        body.expert_ids || [],
        {
          temperature: body.temperature,
          maxTokens: body.maxTokens
        }
      );
      appendLog({ type: 'expert', msg: 'multi-consult', experts: result.total });
      ok(res, result);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('post', '/experts/debate', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await alliance.debate(
        body.question || '',
        body.expert_ids || [],
        {
          rounds: body.rounds || 2,
          temperature: body.temperature
        }
      );
      appendLog({ type: 'expert', msg: 'debate', rounds: result.rounds });
      ok(res, result);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('get', '/experts/capabilities', (req, res) => {
    ok(res, {
      capabilities: alliance.getExpertCapabilities(),
      types: alliance.getExpertTypes()
    });
  });

  reg('post', '/experts/route', async (req, res) => {
    const body = await readBody(req);
    try {
      const routing = await alliance.routeExperts(body.question || body.message || '', {
        maxExperts: body.maxExperts || 3,
        strategy: body.strategy
      });
      ok(res, routing);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('post', '/experts/intelligent-consult', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await alliance.intelligentConsult(body.question || body.message || '', {
        mode: body.mode,
        maxExperts: body.maxExperts,
        temperature: body.temperature,
        problemContext: body.problemContext,
        businessConstraints: body.businessConstraints
      });
      appendLog({ type: 'expert', msg: 'intelligent-consult', mode: result.mode });
      ok(res, result);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('post', '/experts/algorithm-analysis', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await alliance.analyzeWithAlgorithm(
        body.question || '',
        body.graphData || body.graph || null,
        body.options || {}
      );
      appendLog({ type: 'expert', msg: 'algorithm-analysis' });
      ok(res, result);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // ===== 企业级专家联盟处理引擎 =====
  reg('post', '/experts/alliance/process', async (req, res) => {
    const body = await readBody(req);
    const question = String(body.question || body.message || '').trim();
    if (!question) return fail(res, 400, 'question 为必填（question 或 message），不能为空');
    try {
      const engine = getAllianceEngine();
      const result = await engine.process(question, {
        teamSize: body.teamSize,
        enableDebate: body.enableDebate,
        disableRetry: body.disableRetry,
        context: { background: body.background, constraints: body.constraints },
        feedback: body.feedback
      });
      appendLog({ type: 'alliance', msg: 'process', level: result.gate ? result.gate.level : '?' });
      ok(res, result);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // ===== G2 审计闭环：trace 查询（任何一次咨询可完整回溯） =====
  reg('get', '/experts/alliance/traces/stats', (req, res) => {
    ok(res, getAllianceEngine().traceStats());
  });

  reg('get', '/experts/alliance/traces', (req, res) => {
    const limit = Math.max(1, Math.min(parseInt(req.url.split('limit=')[1] || '20', 10) || 20, 200));
    ok(res, { traces: getAllianceEngine().queryTraces(limit) });
  });

  reg('get', '/experts/alliance/traces/:traceId', (req, res, params) => {
    const trace = getAllianceEngine().queryTrace(params.traceId);
    if (!trace) return fail(res, 404, `trace ${params.traceId} 不存在（窗口：最近 200 条）`);
    ok(res, trace);
  });

  // ===== G1 学习技能视图（沉淀成果可查） =====
  reg('get', '/experts/alliance/skills', (req, res) => {
    const engine = getAllianceEngine();
    const limit = Math.max(1, Math.min(parseInt(req.url.split('limit=')[1] || '20', 10) || 20, 200));
    ok(res, { skills: engine.getLearnedSkills(limit), stats: engine.getSkillStats() });
  });

  reg('post', '/experts/alliance/intent', async (req, res) => {
    const body = await readBody(req);
    try {
      const engine = getAllianceEngine();
      ok(res, engine.classifyIntent(body.question || body.message || ''));
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('post', '/experts/alliance/compose', async (req, res) => {
    const body = await readBody(req);
    try {
      const engine = getAllianceEngine();
      const intent = engine.classifyIntent(body.question || '');
      ok(res, engine.composeTeam(body.question || '', intent, { teamSize: body.teamSize }));
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('get', '/experts/metrics', (req, res) => {
    ok(res, { metrics: alliance.getAllMetrics() });
  });

  reg('get', '/experts/overview', (req, res) => {
    ok(res, alliance.getSystemOverview());
  });

  reg('get', '/experts/:id/metrics', (req, res, params) => {
    const metrics = alliance.getExpertMetrics(params.id);
    if (metrics) {
      ok(res, metrics);
    } else {
      fail(res, 404, 'Expert not found');
    }
  });

  reg('post', '/expert-sessions', (req, res) => {
    const body = readBody.sync ? readBody.sync(req) : null;
    try {
      const session = alliance.createSession(body || {});
      ok(res, session);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('get', '/expert-sessions', (req, res) => {
    ok(res, alliance.listSessions());
  });

  reg('get', '/expert-sessions/:id', (req, res, params) => {
    const session = alliance.getSession(params.id);
    if (session) {
      ok(res, session);
    } else {
      fail(res, 404, 'Session not found');
    }
  });

  reg('post', '/expert-sessions/:id/message', async (req, res, params) => {
    const body = await readBody(req);
    try {
      const result = await alliance.processSessionMessage(
        params.id,
        body.message || body.content || '',
        body.options || {}
      );
      ok(res, result);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('post', '/expert-chains', (req, res) => {
    const body = readBody.sync ? readBody.sync(req) : null;
    try {
      const chain = alliance.createSessionChain(
        body?.name || 'Expert Chain',
        body?.expert_ids || [],
        body?.options || {}
      );
      ok(res, chain);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('get', '/expert-chains', (req, res) => {
    ok(res, alliance.listSessionChains());
  });

  reg('post', '/expert-chains/:id/execute', async (req, res, params) => {
    const body = await readBody(req);
    try {
      const result = await alliance.executeChain(
        params.id,
        body.question || '',
        body.options || {}
      );
      ok(res, result);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // ===== 企业级会话持久化 =====
  reg('post', '/experts/sessions', async (req, res) => {
    const body = await readBody(req);
    try {
      const session = sessionStore.createSession(body);
      appendLog({ type: 'expert', msg: 'session-create', id: session.id });
      ok(res, session);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('get', '/experts/sessions', (req, res) => {
    const q = url.parse(req.url, true).query;
    ok(res, sessionStore.listSessions({
      status: q.status, mode: q.mode, expertId: q.expert, keyword: q.q
    }));
  });

  reg('get', '/experts/sessions/stats', (req, res) => {
    ok(res, sessionStore.getSessionStats());
  });

  reg('get', '/experts/sessions/:id', (req, res, params) => {
    const session = sessionStore.getSession(params.id);
    if (session) ok(res, session);
    else fail(res, 404, 'Session not found');
  });

  reg('put', '/experts/sessions/:id', async (req, res, params) => {
    const body = await readBody(req);
    try {
      const session = sessionStore.updateSession(params.id, body);
      if (session) ok(res, session);
      else fail(res, 404, 'Session not found');
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('delete', '/experts/sessions/:id', (req, res, params) => {
    const success = sessionStore.deleteSession(params.id);
    if (success) ok(res, { success: true });
    else fail(res, 404, 'Session not found');
  });

  reg('post', '/experts/sessions/:id/messages', async (req, res, params) => {
    const body = await readBody(req);
    try {
      const message = sessionStore.appendMessage(params.id, body);
      if (message) ok(res, message);
      else fail(res, 404, 'Session not found');
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('post', '/experts/sessions/:id/similar-search', async (req, res, params) => {
    const body = await readBody(req);
    try {
      const result = await sessionStore.findRelevantHistory(params.id, body.question || '', {
        threshold: body.threshold, limit: body.limit, recentCount: body.recentCount
      });
      ok(res, result);
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('post', '/experts/semantic-search', async (req, res) => {
    const body = await readBody(req);
    try {
      const results = await sessionStore.semanticSearch(body.query || body.question || '', {
        threshold: body.threshold, limit: body.limit
      });
      ok(res, results);
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('get', '/experts/sessions/:id/export', (req, res, params) => {
    const exported = sessionStore.exportSession(params.id);
    if (exported) ok(res, exported);
    else fail(res, 404, 'Session not found');
  });

  reg('post', '/experts/sessions/:id/archive', (req, res, params) => {
    const archived = sessionStore.archiveSession(params.id);
    if (archived) ok(res, archived);
    else fail(res, 404, 'Session not found');
  });

  // ===== 企业级调度策略引擎 =====
  reg('get', '/experts/dispatcher/config', (req, res) => {
    ok(res, dispatcher.getConfig());
  });

  reg('put', '/experts/dispatcher/config', async (req, res) => {
    const body = await readBody(req);
    try {
      if (body.strategy) dispatcher.setStrategy(body.strategy);
      ok(res, dispatcher.getConfig());
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('get', '/experts/dispatcher/status', (req, res) => {
    ok(res, dispatcher.getStatus());
  });

  reg('post', '/experts/dispatcher/dispatch', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await dispatcher.dispatch(body.question || body.message || '', {
        strategy: body.strategy, expertIds: body.expertIds, requester: body.requester
      });
      ok(res, result);
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('post', '/experts/dispatcher/consult', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await dispatcher.dispatchAndConsult(body.question || body.message || '', {
        strategy: body.strategy, expertIds: body.expertIds, requester: body.requester, ...(body.options || {})
      });
      appendLog({ type: 'expert', msg: 'dispatch-consult', success: result.success });
      ok(res, result);
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('post', '/experts/dispatcher/multi-consult', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await dispatcher.dispatchMultiExpert(body.question || body.message || '', {
        maxExperts: body.maxExperts, strategy: body.strategy, ...(body.options || {})
      });
      ok(res, result);
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('post', '/experts/dispatcher/reset/:expertId', (req, res, params) => {
    dispatcher.resetExpert(params.expertId);
    ok(res, { success: true, expert_id: params.expertId });
  });

  reg('post', '/experts/dispatcher/reset-all', () => {
    dispatcher.resetAll();
    ok(res, { success: true });
  });

};

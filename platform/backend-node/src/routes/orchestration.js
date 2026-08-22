'use strict';

/**
 * 路由域：编排协作
 * /experts/enterprise|orchestrate|plan/* V2 编排引擎与企业级协作
 */
module.exports = function registerOrchestrationRoutes(ctx) {
  const { url, alliance, sessionStore, dispatcher, expertGraph, ok, fail, readBody, reg } = ctx;

  // ===== 企业级协作端点 =====
  reg('post', '/experts/enterprise/consult', async (req, res) => {
    const body = await readBody(req);
    try {
      const question = body.question || body.message || '';
      const session = sessionStore.createSession({
        title: question.slice(0, 50),
        mode: body.mode || 'smart',
        createdBy: body.requester || 'enterprise',
        tags: body.tags || [],
        problemContext: body.problemContext,
        businessConstraints: body.businessConstraints
      });

      const related = await sessionStore.findRelevantHistory(session.id, question);
      const dispatchResult = await dispatcher.dispatchAndConsult(question, {
        strategy: body.strategy || STRATEGY_TYPES.CONTENT_AWARE,
        requester: body.requester
      });

      if (dispatchResult.success) {
        sessionStore.appendMessage(session.id, { role: 'user', content: question });
        if (dispatchResult.result?.response) {
          sessionStore.appendMessage(session.id, {
            role: 'assistant', content: dispatchResult.result.response,
            expert_id: dispatchResult.result.expert?.id
          });
        }
      }

      ok(res, {
        session, dispatch: dispatchResult,
        context_used: related.context_messages.length > 0,
        similar_history_found: related.similar_history.length,
        similar_history: related.similar_history.slice(0, 3)
      });
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('post', '/experts/enterprise/analyze', async (req, res) => {
    const body = await readBody(req);
    try {
      const question = body.question || '';
      const optimalTeam = expertGraph.findOptimalTeam(question, body.teamSize || 3);
      const session = sessionStore.createSession({
        title: question.slice(0, 50), mode: 'multi_expert',
        tags: ['enterprise', 'analysis'], problemContext: body.problemContext
      });

      const multiResult = await dispatcher.dispatchMultiExpert(question, {
        maxExperts: body.teamSize || 3
      });

      for (const r of multiResult.results || []) {
        if (r?.response) {
          sessionStore.appendMessage(session.id, { role: 'assistant', content: r.response, expert_id: r.expert?.id });
        }
      }

      ok(res, { session, optimal_team: optimalTeam, multi_result: multiResult, graph_insights: expertGraph.getGraphStats() });
    } catch (e) { fail(res, 500, e.message); }
  });

  // ===== V2 编排引擎路由 =====
  reg('post', '/experts/orchestrate', async (req, res) => {
    const body = await readBody(req);
    try {
      const question = body.question || body.message || '';
      const result = await alliance.orchestrate(question, {
        pipeline: body.pipeline || body.mode,
        maxSteps: body.maxSteps,
        sessionId: body.sessionId,
        context: body.context,
        constraints: body.constraints,
        enableCheckpoints: body.enableCheckpoints,
        enableLearning: body.enableLearning
      });
      ok(res, result);
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('post', '/experts/plan/generate', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await alliance.generatePlan(body.question || '', body);
      ok(res, result);
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('post', '/experts/plan/execute', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await alliance.runPlanExecution(body.plan || body, body);
      ok(res, result);
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('get', '/experts/orchestration/stats', async (req, res) => {
    try {
      const stats = alliance.getOrchestrationStats();
      ok(res, stats);
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('get', '/experts/orchestration/plugins', async (req, res) => {
    try {
      const plugins = alliance.listPlugins();
      ok(res, plugins);
    } catch (e) { fail(res, 500, e.message); }
  });

  reg('get', '/experts/orchestration/history', async (req, res) => {
    try {
      const engine = alliance.getOrchestrationEngine();
      if (!engine) { ok(res, { history: [], total: 0 }); return; }
      const history = engine.getHistory({
        mode: req.url.searchParams?.get('mode') || undefined,
        status: req.url.searchParams?.get('status') || undefined,
        limit: parseInt(req.url.searchParams?.get('limit') || '100')
      });
      ok(res, { history, total: history.length });
    } catch (e) { fail(res, 500, e.message); }
  });

};

'use strict';

/**
 * 会话与专家链用例族（Application 层 mixin）
 * ------------------------------------------------------------------
 * 挂载于 ExpertAlliance.prototype：会话链执行、会话消息处理。
 * 存储委托 SessionChainStore，咨询委托本类 consult/multiExpertConsult/debate。
 */

async function executeChain(chainId, initialQuestion, options = {}) {
  const chain = this.store.getChain(chainId);
  if (!chain) throw new Error(`链不存在: ${chainId}`);

  let context = initialQuestion;
  const results = [];
  const startTime = Date.now();

  if (chain.mode === 'parallel') {
    const parallelResults = await this.multiExpertConsult(initialQuestion, chain.experts, options);
    for (const r of parallelResults.results) {
      results.push({
        expert_id: r.expert.id,
        expert_name: r.expert.name,
        output: r.response || r.error,
        status: r.success ? 'success' : 'failed'
      });
      chain.interactions.push({
        expert_id: r.expert.id,
        input: context,
        output: r.response || r.error,
        timestamp: new Date().toISOString(),
        status: r.success ? 'success' : 'failed'
      });
    }
  } else {
    for (const expertId of chain.experts) {
      const expert = this.repo.get(expertId);
      if (!expert || expert.status !== 'active') continue;

      try {
        const result = await this.consult(expertId, [
          { role: 'user', content: context }
        ], options);

        results.push({
          expert_id: expertId,
          expert_name: expert.name,
          output: result.response,
          status: 'success'
        });

        chain.interactions.push({
          expert_id: expertId,
          expert_name: expert.name,
          input: context,
          output: result.response,
          timestamp: new Date().toISOString(),
          status: 'success'
        });

        context = `基于 ${expert.name} 的分析：\n${result.response}\n\n请继续处理以下问题：${initialQuestion}`;
      } catch (e) {
        results.push({
          expert_id: expertId,
          expert_name: expert.name,
          output: e.message,
          status: 'failed'
        });

        chain.interactions.push({
          expert_id: expertId,
          expert_name: expert.name,
          input: context,
          output: e.message,
          timestamp: new Date().toISOString(),
          status: 'failed',
          error: e.message
        });
      }
    }
  }

  chain.status = 'completed';
  chain.completed_at = new Date().toISOString();

  return {
    chain_id: chainId,
    mode: chain.mode,
    experts_consulted: results.length,
    results,
    total_duration_ms: Date.now() - startTime,
    final_response: results[results.length - 1]?.output || '暂无结果'
  };
}

async function processSessionMessage(sessionId, message, options = {}) {
  const session = this.store.getSession(sessionId);
  if (!session) throw new Error(`会话不存在: ${sessionId}`);

  this.appendMessage(sessionId, { role: 'user', content: message });

  const routing = await this.routeExperts(message, options);
  let response;

  if (session.mode === 'debate' && routing.selected.length >= 2) {
    const debateResult = await this.debate(
      message,
      routing.selected.map(s => s.expert.id),
      options
    );
    response = debateResult.final_synthesis;
  } else if (session.mode === 'multi' && routing.selected.length >= 2) {
    const multiResult = await this.multiExpertConsult(
      message,
      routing.selected.map(s => s.expert.id),
      options
    );
    response = multiResult.results.filter(r => r.success)
      .map(r => `【${r.expert.name}】\n${r.response}`).join('\n\n');
  } else {
    const expertId = session.current_expert || routing.selected[0]?.expert.id || 'alg-expert';
    const result = await this.consult(expertId, [{ role: 'user', content: message }], options);
    response = result.response;
  }

  this.appendMessage(sessionId, {
    role: 'assistant',
    content: response,
    expert_id: routing.selected[0]?.expert.id,
    routing_info: {
      intent: routing.intent.primary,
      experts_considered: routing.selected.length
    }
  });

  return {
    session_id: sessionId,
    response,
    routing: {
      intent: routing.intent.primary,
      confidence: Math.round(routing.intent.confidence * 100) / 100,
      experts: routing.selected.map(s => ({
        id: s.expert.id,
        name: s.expert.name,
        score: Math.round(s.score * 100) / 100
      }))
    }
  };
}

module.exports = { executeChain, processSessionMessage };

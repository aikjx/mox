'use strict';

/**
 * V2 编排引擎代理用例族（Application 层 mixin）
 * ------------------------------------------------------------------
 * 挂载于 ExpertAlliance.prototype：orchestration-engine 单例的转发方法。
 * 引擎不可用时智能降级回 intelligentConsult（A1 degrades_to 边的编排层体现）。
 */

async function orchestrate(question, options = {}) {
  if (!this.orchestrationEngine) {
    return this.intelligentConsult(question, options);
  }

  const input = {
    question,
    mode: options.pipeline || options.mode || 'standard',
    sessionId: options.sessionId,
    context: options.context,
    constraints: options.constraints,
    user: options.user
  };

  const engineOptions = {
    mode: input.mode,
    maxSteps: options.maxSteps || 10,
    enableCheckpoints: options.enableCheckpoints,
    enableLearning: options.enableLearning
  };

  const result = await this.orchestrationEngine.runTurn(input, engineOptions);

  if (result.status === 'success') {
    const expertRoute = result.state?.execution?.expertsConsulted || [];
    const finalOutput = result.finalOutput || result.state?.reflection || result.state?.execution;

    return {
      success: true,
      response: typeof finalOutput === 'object' ? JSON.stringify(finalOutput, null, 2) : finalOutput,
      expert: expertRoute[0]?.id ? { id: expertRoute[0].id, name: expertRoute[0].id } : null,
      metadata: {
        orchestrated: true,
        pipeline: input.mode,
        turnId: result.turnId,
        duration_ms: result.duration,
        checkpoints: result.checkpoints,
        status: result.status
      },
      v2: true,
      orchestration: result
    };
  }

  return {
    success: false,
    response: result.error || '编排执行失败',
    error: result.error,
    metadata: {
      orchestrated: true,
      pipeline: input.mode,
      turnId: result.turnId,
      duration_ms: result.duration,
      status: result.status
    },
    v2: true
  };
}

async function generatePlan(question, options = {}) {
  if (!this.orchestrationEngine) {
    return { success: false, error: '编排引擎未初始化' };
  }

  const planner = this.orchestrationEngine.getPlugin('planner');
  if (!planner) {
    return { success: false, error: 'Planner 插件不可用' };
  }

  const input = { question, mode: 'plan_act' };
  const context = this.orchestrationEngine.createPluginContext();
  const planResult = await planner.createPlan(input, {}, context);

  return {
    success: true,
    plan: planResult.plan,
    generatedAt: new Date().toISOString(),
    v2: true
  };
}

function getOrchestrationStats() {
  if (!this.orchestrationEngine) {
    return { error: '编排引擎未初始化' };
  }
  return this.orchestrationEngine.getStats();
}

function listPlugins() {
  if (!this.orchestrationEngine) {
    return [];
  }
  return this.orchestrationEngine.listPlugins();
}

async function runPlanExecution(plan, options = {}) {
  if (!this.orchestrationEngine) {
    return { success: false, error: '编排引擎未初始化' };
  }

  const result = await this.orchestrationEngine.runTurn(
    { ...plan, mode: options.pipeline || 'plan_act' },
    options
  );

  return { success: result.status === 'success', result, v2: true };
}

module.exports = {
  orchestrate,
  generatePlan,
  getOrchestrationStats,
  listPlugins,
  runPlanExecution
};

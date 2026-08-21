'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const DATA_DIR = path.join(__dirname, '..', 'data');
const PLUGINS_DIR = path.join(__dirname, '..', 'plugins');
const ORCHESTRATION_CONFIG = 'orchestration_config.json';
const MAX_HISTORY = 1000;

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
    console.error('[orchestration] writeJSON', file, e.message);
    return false;
  }
}

class OrchestrationEngine {
  constructor(options = {}) {
    this.plugins = new Map();
    this.eventBus = new EventBus();
    this.stateManager = new StateManager();
    this.checkpoints = new Map();
    this.history = [];
    this.config = this._loadConfig();
    this._initPlugins();
  }

  _loadConfig() {
    const saved = readJSON(ORCHESTRATION_CONFIG, null);
    if (saved) return saved;
    return {
      version: '2.0',
      defaultPipeline: 'standard',
      maxTurns: 50,
      timeout: 30000,
      enableCheckpoints: true,
      enableLearning: true
    };
  }

  _initPlugins() {
    const builtInPlugins = [
      new PerceptionPlugin(),
      new MemoryPlugin(),
      new PlannerPlugin(),
      new ExecutorPlugin(),
      new ReflectorPlugin(),
      new LearnerPlugin()
    ];
    builtInPlugins.forEach(plugin => this.registerPlugin(plugin));
    this._loadCustomPlugins();
  }

  _loadCustomPlugins() {
    if (!fs.existsSync(PLUGINS_DIR)) {
      fs.mkdirSync(PLUGINS_DIR, { recursive: true });
      return;
    }
    const files = fs.readdirSync(PLUGINS_DIR).filter(f => f.endsWith('.js'));
    files.forEach(file => {
      try {
        const PluginClass = require(path.join(PLUGINS_DIR, file));
        if (typeof PluginClass === 'function') {
          const plugin = new PluginClass();
          this.registerPlugin(plugin);
        }
      } catch (e) {
        console.warn('[orchestration] Failed to load plugin', file, e.message);
      }
    });
  }

  registerPlugin(plugin) {
    if (!plugin.name) throw new Error('Plugin must have a name');
    this.plugins.set(plugin.name, plugin);
    plugin.onMount?.(this.createPluginContext());
    return plugin;
  }

  unregisterPlugin(name) {
    const plugin = this.plugins.get(name);
    if (plugin) {
      plugin.onUnmount?.();
      this.plugins.delete(name);
    }
  }

  getPlugin(name) {
    return this.plugins.get(name);
  }

  listPlugins() {
    return Array.from(this.plugins.entries()).map(([name, plugin]) => ({
      name,
      description: plugin.description,
      version: plugin.version,
      hooks: Object.keys(plugin).filter(k => k.startsWith('on') || k.startsWith('before') || k.startsWith('after'))
    }));
  }

  createPluginContext() {
    return {
      config: this.config,
      eventBus: this.eventBus,
      state: this.stateManager,
      history: this.history,
      createCheckpoint: (state) => this.createCheckpoint(state),
      getService: (name) => this.getPlugin(name)
    };
  }

  async runTurn(input, options = {}) {
    const turnId = `turn_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`;
    const context = this.createTurnContext(turnId, input, options);

    try {
      this.eventBus.publish({ type: 'orchestration.start', turnId, timestamp: Date.now() });

      const result = await this.executePipeline(context);

      this.eventBus.publish({
        type: 'orchestration.end',
        turnId,
        status: result.status,
        duration: Date.now() - context.startTime,
        timestamp: Date.now()
      });

      this._recordHistory(context, result);
      return result;

    } catch (error) {
      this.eventBus.publish({
        type: 'orchestration.error',
        turnId,
        error: error.message,
        timestamp: Date.now()
      });

      return {
        turnId,
        status: 'error',
        error: error.message,
        duration: Date.now() - context.startTime,
        checkpoints: this.getCheckpointChain(turnId)
      };
    }
  }

  createTurnContext(turnId, input, options) {
    return {
      turnId,
      input,
      options: {
        mode: options.mode || 'standard',
        maxSteps: options.maxSteps || 10,
        enableCheckpoints: options.enableCheckpoints ?? this.config.enableCheckpoints,
        enableLearning: options.enableLearning ?? this.config.enableLearning,
        ...options
      },
      startTime: Date.now(),
      currentStep: 0,
      state: {},
      checkpoints: [],
      results: [],
      interrupted: false
    };
  }

  async executePipeline(context) {
    const pipeline = this.resolvePipeline(context.options.mode);
    const steps = pipeline || this.getDefaultPipeline();

    for (let i = 0; i < steps.length; i++) {
      if (context.interrupted) {
        return this.createInterruptedResult(context);
      }

      context.currentStep = i;
      const step = steps[i];

      this.eventBus.publish({
        type: 'orchestration.step',
        turnId: context.turnId,
        step: { name: step.name, index: i, total: steps.length },
        timestamp: Date.now()
      });

      try {
        if (context.options.enableCheckpoints && step.savesCheckpoint) {
          const checkpoint = this.createCheckpoint(context);
          context.checkpoints.push(checkpoint);
        }

        const result = await step.execute(context, this.createPluginContext());
        context.results.push({ step: step.name, result, success: true });
        Object.assign(context.state, result.state || {});

        if (step.validates) {
          const isValid = await step.validates(result, context);
          if (!isValid) {
            return this.createValidationErrorResult(context, step.name);
          }
        }

      } catch (stepError) {
        context.results.push({ step: step.name, error: stepError.message, success: false });

        if (step.recoverable) {
          const recovered = await this.attemptRecovery(context, step, stepError);
          if (recovered) continue;
        }

        return this.createErrorResult(context, step.name, stepError);
      }
    }

    return this.createSuccessResult(context);
  }

  resolvePipeline(mode) {
    const pipelines = {
      'standard': this.getDefaultPipeline(),
      'plan_act': this.getPlanActPipeline(),
      'fast_path': this.getFastPathPipeline(),
      'deep_analysis': this.getDeepAnalysisPipeline()
    };
    return pipelines[mode] || this.getDefaultPipeline();
  }

  getDefaultPipeline() {
    return [
      { name: 'perception', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('perception');
        return plugin ? plugin.process(ctx.input, pc) : { perception: ctx.input };
      }, savesCheckpoint: true },
      { name: 'memory', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('memory');
        return plugin ? plugin.recall(ctx.state.perception, pc) : { memories: [] };
      }},
      { name: 'planner', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('planner');
        return plugin ? plugin.createPlan(ctx.input, ctx.state, pc) : { plan: null };
      }, validates: (result) => result.plan !== null },
      { name: 'executor', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('executor');
        return plugin ? plugin.execute(ctx.state.plan, pc) : { execution: null };
      }, savesCheckpoint: true, recoverable: true },
      { name: 'reflector', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('reflector');
        return plugin ? plugin.analyze(ctx.state.execution, pc) : { reflection: null };
      }},
      { name: 'learner', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('learner');
        return plugin && ctx.options.enableLearning
          ? plugin.consolidate(ctx.state, pc)
          : { learned: false };
      }}
    ];
  }

  getPlanActPipeline() {
    return [
      { name: 'perception', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('perception');
        return plugin ? plugin.process(ctx.input, pc) : { perception: ctx.input };
      }},
      { name: 'planner', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('planner');
        const plan = plugin ? plugin.createPlan(ctx.input, ctx.state, pc) : { plan: null };
        ctx.state._planMode = true;
        return plan;
      }, savesCheckpoint: true },
      { name: 'memory', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('memory');
        return plugin ? plugin.recall(ctx.input, pc) : { memories: [] };
      }},
      { name: 'executor', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('executor');
        ctx.state._actMode = true;
        return plugin ? plugin.execute(ctx.state.plan, pc) : { execution: null };
      }, savesCheckpoint: true, recoverable: true },
      { name: 'reflector', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('reflector');
        return plugin ? plugin.analyze(ctx.state.execution, pc) : { reflection: null };
      }},
      { name: 'learner', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('learner');
        return plugin && ctx.options.enableLearning
          ? plugin.consolidate(ctx.state, pc)
          : { learned: false };
      }}
    ];
  }

  getFastPathPipeline() {
    return [
      { name: 'perception', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('perception');
        return plugin ? plugin.process(ctx.input, pc) : { perception: ctx.input };
      }},
      { name: 'executor', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('executor');
        return plugin ? plugin.execute(ctx.input, pc) : { execution: null };
      }, savesCheckpoint: true },
      { name: 'reflector', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('reflector');
        return plugin ? plugin.analyze(ctx.state.execution, pc) : { reflection: null };
      }}
    ];
  }

  getDeepAnalysisPipeline() {
    return [
      { name: 'perception', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('perception');
        return plugin ? plugin.process(ctx.input, pc) : { perception: ctx.input };
      }, savesCheckpoint: true },
      { name: 'memory', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('memory');
        return plugin ? plugin.recall(ctx.state.perception, pc) : { memories: [] };
      }, savesCheckpoint: true },
      { name: 'planner', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('planner');
        return plugin ? plugin.createPlan(ctx.input, ctx.state, pc) : { plan: null };
      }, savesCheckpoint: true },
      { name: 'executor', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('executor');
        return plugin ? plugin.execute(ctx.state.plan, pc) : { execution: null };
      }, savesCheckpoint: true, recoverable: true },
      { name: 'reflector', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('reflector');
        const reflection1 = plugin ? plugin.analyze(ctx.state.execution, pc) : {};
        if (ctx.state._iteration < 3) {
          ctx.state._iteration = (ctx.state._iteration || 0) + 1;
          ctx.input._reflection = reflection1;
          return { ...reflection1, iterate: true };
        }
        return { ...reflection1, iterate: false };
      }, savesCheckpoint: true },
      { name: 'learner', execute: async (ctx, pc) => {
        const plugin = this.getPlugin('learner');
        return plugin && ctx.options.enableLearning
          ? plugin.consolidate(ctx.state, pc)
          : { learned: false };
      }}
    ];
  }

  async attemptRecovery(context, step, error) {
    const lastCheckpoint = context.checkpoints[context.checkpoints.length - 1];
    if (!lastCheckpoint) return false;

    context.state = { ...lastCheckpoint.state };
    context.results = context.results.slice(0, lastCheckpoint.stepIndex);
    context.checkpoints = context.checkpoints.slice(0, -1);

    this.eventBus.publish({
      type: 'orchestration.recovery',
      turnId: context.turnId,
      recoveredFrom: step.name,
      checkpointId: lastCheckpoint.id,
      timestamp: Date.now()
    });

    return true;
  }

  createCheckpoint(context) {
    const checkpoint = {
      id: `ckpt_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`,
      turnId: context.turnId,
      stepIndex: context.currentStep,
      stepName: context.options.mode,
      state: JSON.parse(JSON.stringify(context.state)),
      results: JSON.parse(JSON.stringify(context.results)),
      createdAt: Date.now()
    };
    this.checkpoints.set(checkpoint.id, checkpoint);
    return checkpoint;
  }

  rollbackToCheckpoint(checkpointId) {
    const checkpoint = this.checkpoints.get(checkpointId);
    if (!checkpoint) return false;
    return checkpoint;
  }

  getCheckpointChain(turnId) {
    return Array.from(this.checkpoints.values())
      .filter(c => c.turnId === turnId)
      .sort((a, b) => a.stepIndex - b.stepIndex);
  }

  createSuccessResult(context) {
    return {
      turnId: context.turnId,
      status: 'success',
      mode: context.options.mode,
      state: context.state,
      results: context.results.map(r => ({
        step: r.step,
        success: r.success,
        output: r.result?.output || r.result
      })),
      finalOutput: context.state.reflection || context.state.execution || context.state.plan,
      duration: Date.now() - context.startTime,
      checkpoints: context.checkpoints.length,
      timestamp: Date.now()
    };
  }

  createErrorResult(context, failedStep, error) {
    return {
      turnId: context.turnId,
      status: 'error',
      failedStep,
      error: error.message,
      state: context.state,
      partialResults: context.results,
      duration: Date.now() - context.startTime,
      recoverable: !!context.checkpoints.length,
      checkpoints: this.getCheckpointChain(context.turnId),
      timestamp: Date.now()
    };
  }

  createValidationErrorResult(context, stepName) {
    return {
      turnId: context.turnId,
      status: 'validation_error',
      failedStep: stepName,
      state: context.state,
      partialResults: context.results,
      duration: Date.now() - context.startTime,
      timestamp: Date.now()
    };
  }

  createInterruptedResult(context) {
    return {
      turnId: context.turnId,
      status: 'interrupted',
      state: context.state,
      partialResults: context.results,
      duration: Date.now() - context.startTime,
      timestamp: Date.now()
    };
  }

  _recordHistory(context, result) {
    this.history.push({
      turnId: context.turnId,
      input: context.input,
      result: {
        status: result.status,
        duration: result.duration,
        finalOutput: result.finalOutput || result.error
      },
      timestamp: Date.now()
    });

    if (this.history.length > MAX_HISTORY) {
      this.history = this.history.slice(-MAX_HISTORY);
    }
  }

  getHistory(options = {}) {
    let result = this.history;
    if (options.mode) result = result.filter(h => h.input?.mode === options.mode);
    if (options.status) result = result.filter(h => h.result.status === options.status);
    if (options.limit) result = result.slice(-options.limit);
    return result;
  }

  getStats() {
    const total = this.history.length;
    const byStatus = {};
    const byMode = {};
    let totalDuration = 0;

    this.history.forEach(h => {
      const status = h.result.status || 'unknown';
      byStatus[status] = (byStatus[status] || 0) + 1;
      byMode[h.input?.mode || 'unknown'] = (byMode[h.input?.mode || 'unknown'] || 0) + 1;
      totalDuration += h.result.duration || 0;
    });

    return {
      totalTurns: total,
      byStatus,
      byMode,
      avgDuration: total > 0 ? Math.round(totalDuration / total) : 0,
      activePlugins: this.plugins.size,
      lastUpdate: Date.now()
    };
  }

  updateConfig(partialConfig) {
    this.config = { ...this.config, ...partialConfig };
    writeJSON(ORCHESTRATION_CONFIG, this.config);
    return this.config;
  }

  interruptTurn(turnId) {
    this.eventBus.publish({
      type: 'orchestration.interrupt',
      turnId,
      timestamp: Date.now()
    });
    return true;
  }
}

class EventBus {
  constructor() {
    this.subscribers = new Map();
    this.eventLog = [];
    this.maxLogSize = 10000;
  }

  publish(event) {
    const enrichedEvent = {
      id: crypto.randomUUID(),
      timestamp: event.timestamp || Date.now(),
      ...event
    };

    this.eventLog.push(enrichedEvent);
    if (this.eventLog.length > this.maxLogSize) {
      this.eventLog = this.eventLog.slice(-this.maxLogSize);
    }

    const handlers = this.subscribers.get(event.type) || [];
    handlers.forEach(h => {
      try { h.handler(enrichedEvent); }
      catch (e) { console.error('[event-bus] handler error:', e.message); }
    });

    const globalHandlers = this.subscribers.get('*') || [];
    globalHandlers.forEach(h => {
      try { h.handler(enrichedEvent); }
      catch (e) { console.error('[event-bus] global handler error:', e.message); }
    });

    return enrichedEvent;
  }

  subscribe(eventType, handler, options = {}) {
    const subscription = {
      id: crypto.randomUUID(),
      eventType,
      handler,
      filter: options.filter || (() => true),
      priority: options.priority || 0
    };

    if (!this.subscribers.has(eventType)) {
      this.subscribers.set(eventType, []);
    }
    this.subscribers.get(eventType).push(subscription);
    this.subscribers.get(eventType).sort((a, b) => b.priority - a.priority);

    return subscription.id;
  }

  unsubscribe(subscriptionId) {
    for (const [type, subs] of this.subscribers.entries()) {
      const idx = subs.findIndex(s => s.id === subscriptionId);
      if (idx !== -1) {
        subs.splice(idx, 1);
        return true;
      }
    }
    return false;
  }

  getEventLog(options = {}) {
    let result = this.eventLog;
    if (options.type) result = result.filter(e => e.type === options.type);
    if (options.turnId) result = result.filter(e => e.turnId === options.turnId);
    if (options.limit) result = result.slice(-options.limit);
    return result;
  }

  clearEventLog() {
    this.eventLog = [];
  }
}

class StateManager {
  constructor() {
    this.states = new Map();
    this.transitions = [];
  }

  createState(id, initialData = {}) {
    this.states.set(id, {
      id,
      data: { ...initialData },
      history: [],
      createdAt: Date.now(),
      updatedAt: Date.now()
    });
    return this.states.get(id);
  }

  getState(id) {
    return this.states.get(id);
  }

  updateState(id, updates) {
    const state = this.states.get(id);
    if (!state) return null;
    state.data = { ...state.data, ...updates };
    state.updatedAt = Date.now();
    state.history.push({ update: updates, timestamp: Date.now() });
    if (state.history.length > 100) state.history = state.history.slice(-100);
    return state;
  }

  deleteState(id) {
    return this.states.delete(id);
  }

  getStatesByPrefix(prefix) {
    return Array.from(this.states.entries())
      .filter(([id]) => id.startsWith(prefix))
      .map(([id, state]) => state);
  }
}

class PerceptionPlugin {
  constructor() {
    this.name = 'perception';
    this.version = '1.0';
    this.description = '感知与输入处理插件';
  }

  process(input, context) {
    const perception = {
      raw: input,
      processed: input,
      timestamp: Date.now()
    };

    if (input?.question || input?.message) {
      perception.processed = input.question || input.message;
      perception.type = 'question';
    } else if (input?.mode) {
      perception.processed = input;
      perception.type = 'structured';
    }

    if (context?.config?.enableEntityExtraction) {
      perception.entities = this._extractEntities(perception.processed);
    }

    return { perception };
  }

  _extractEntities(text) {
    const entities = [];
    if (!text) return entities;
    const patterns = [
      { type: 'algorithm', regex: /(算法|algorithm|复杂度|复杂度分析)/i },
      { type: 'architecture', regex: /(架构|architecture|系统设计|微服务)/i },
      { type: 'data', regex: /(数据|database|数据建模|ETL)/i },
      { type: 'ai', regex: /(AI|人工智能|LLM|大模型|机器学习)/i },
      { type: 'graph', regex: /(图|图谱|graph|节点|实体关系)/i },
      { type: 'security', regex: /(安全|security|加密|认证|RBAC)/i }
    ];

    patterns.forEach(p => {
      if (p.regex.test(text)) {
        entities.push({ type: p.type, matched: text.match(p.regex)[0] });
      }
    });

    return entities;
  }
}

class MemoryPlugin {
  constructor() {
    this.name = 'memory';
    this.version = '1.0';
    this.description = '记忆与上下文检索插件';
    this.memoryStore = new Map();
  }

  recall(query, context) {
    const memories = [];

    if (context?.history) {
      const recentHistory = context.history.slice(-10);
      memories.push({
        type: 'recent',
        content: recentHistory,
        relevance: 0.9
      });
    }

    const sessionId = query?.sessionId || context?.sessionId;
    if (sessionId && this.memoryStore.has(sessionId)) {
      const sessionMemory = this.memoryStore.get(sessionId);
      memories.push({
        type: 'session',
        content: sessionMemory,
        relevance: 0.85
      });
    }

    return { memories, memoryCount: memories.length };
  }

  store(sessionId, data) {
    this.memoryStore.set(sessionId, {
      data,
      updatedAt: Date.now()
    });
  }

  clear(sessionId) {
    this.memoryStore.delete(sessionId);
  }
}

class PlannerPlugin {
  constructor() {
    this.name = 'planner';
    this.version = '1.0';
    this.description = '规划与策略选择插件';
  }

  createPlan(input, state, context) {
    const perception = state?.perception || input;
    const memories = state?.memories || [];

    const plan = {
      id: `plan_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`,
      strategy: this._selectStrategy(perception, memories),
      steps: [],
      estimatedExperts: [],
      risks: [],
      createdAt: Date.now()
    };

    plan.steps.push({
      id: 'analyze_intent',
      action: 'analyze',
      description: '分析用户意图与问题类型',
      estimatedDuration: 1000
    });

    plan.steps.push({
      id: 'route_experts',
      action: 'route',
      description: '匹配并选择合适的专家',
      estimatedDuration: 500,
      dependsOn: ['analyze_intent']
    });

    plan.steps.push({
      id: 'execute_strategy',
      action: 'execute',
      description: `执行策略: ${plan.strategy}`,
      estimatedDuration: 8000,
      dependsOn: ['route_experts']
    });

    plan.steps.push({
      id: 'validate_result',
      action: 'validate',
      description: '验证执行结果质量',
      estimatedDuration: 500,
      dependsOn: ['execute_strategy']
    });

    plan.steps.push({
      id: 'generate_output',
      action: 'generate',
      description: '生成最终输出与建议',
      estimatedDuration: 1000,
      dependsOn: ['validate_result']
    });

    return { plan };
  }

  _selectStrategy(perception, memories) {
    const text = typeof perception === 'string' ? perception : JSON.stringify(perception || {});
    const riskScore = this._assessRisk(text);

    if (riskScore >= 0.8) return 'deep_analysis';
    if (riskScore >= 0.5) return 'plan_act';
    if (memories?.length > 3) return 'plan_act';
    return 'standard';
  }

  _assessRisk(text) {
    let score = 0;
    const highRiskPatterns = [
      /(复杂|complex|multi.*step)/i,
      /(系统|架构|framework)/i,
      /(多个|multi.*expert|协作)/i
    ];
    const mediumRiskPatterns = [
      /(分析|analyze|review)/i,
      /(优化|optimize|improve)/i,
      /(设计|design|plan)/i
    ];

    highRiskPatterns.forEach(p => { if (p.test(text)) score += 0.3; });
    mediumRiskPatterns.forEach(p => { if (p.test(text)) score += 0.15; });

    return Math.min(score, 1.0);
  }
}

class ExecutorPlugin {
  constructor() {
    this.name = 'executor';
    this.version = '1.0';
    this.description = '执行与专家调用插件';
  }

  async execute(plan, context) {
    const execution = {
      planId: plan?.id,
      steps: [],
      expertsConsulted: [],
      startTime: Date.now()
    };

    if (plan?.strategy === 'fast_path') {
      execution.result = this._executeFastPath(plan, context);
    } else {
      const expertRoute = this._routeExperts(plan, context);
      execution.expertsConsulted = expertRoute.experts;
      execution.result = {
        strategy: plan?.strategy || 'standard',
        experts: expertRoute.experts,
        executionPlan: plan?.steps?.length || 0,
        status: 'ready_for_dispatch'
      };
    }

    execution.endTime = Date.now();
    execution.duration = execution.endTime - execution.startTime;

    return { execution };
  }

  _routeExperts(plan, context) {
    const experts = [];
    const intentType = this._detectIntent(plan);

    const expertMapping = {
      'algorithm': 'alg-expert',
      'architecture': 'arch-expert',
      'data': 'data-expert',
      'ai': 'ai-expert',
      'graph': 'graph-expert',
      'security': 'sec-expert',
      'performance': 'perf-expert',
      'workflow': 'wf-expert',
      'requirement': 'req-expert'
    };

    if (expertMapping[intentType]) {
      experts.push({
        id: expertMapping[intentType],
        role: 'lead',
        score: 0.9,
        reason: `主专家匹配: ${intentType}`
      });
    }

    if (plan?.strategy === 'deep_analysis') {
      experts.push({ id: 'arch-expert', role: 'support', score: 0.7 });
      experts.push({ id: 'graph-expert', role: 'support', score: 0.65 });
    } else if (plan?.strategy === 'plan_act') {
      experts.push({ id: 'graph-expert', role: 'support', score: 0.75 });
    }

    return { experts, intentType };
  }

  _detectIntent(plan) {
    const text = typeof plan === 'string' ? plan : JSON.stringify(plan || {});
    if (/(算法|algorithm|复杂度)/i.test(text)) return 'algorithm';
    if (/(架构|architecture|系统设计)/i.test(text)) return 'architecture';
    if (/(数据|database|ETL)/i.test(text)) return 'data';
    if (/(AI|LLM|大模型)/i.test(text)) return 'ai';
    if (/(图|图谱|graph)/i.test(text)) return 'graph';
    if (/(安全|security|加密)/i.test(text)) return 'security';
    return 'general';
  }

  _executeFastPath(plan, context) {
    return {
      strategy: 'fast_path',
      status: 'completed',
      result: 'Fast path execution completed',
      duration: Date.now() - (plan?._startTime || Date.now())
    };
  }
}

class ReflectorPlugin {
  constructor() {
    this.name = 'reflector';
    this.version = '1.0';
    this.description = '反思与评估插件';
  }

  analyze(execution, context) {
    const reflection = {
      quality: this._assessQuality(execution),
      insights: this._generateInsights(execution),
      suggestions: this._generateSuggestions(execution),
      timestamp: Date.now()
    };

    return { reflection };
  }

  _assessQuality(execution) {
    const base = 0.7;
    const adjustments = [];

    if (execution?.duration && execution.duration < 2000) adjustments.push(0.1);
    if (execution?.expertsConsulted?.length >= 3) adjustments.push(0.05);
    if (execution?.result?.status === 'ready_for_dispatch') adjustments.push(0.05);

    return Math.min(base + adjustments.reduce((a, b) => a + b, 0), 1.0);
  }

  _generateInsights(execution) {
    const insights = [];
    if (execution?.expertsConsulted?.length > 0) {
      insights.push({
        type: 'expert_coverage',
        message: `选中 ${execution.expertsConsulted.length} 位专家协作`,
        level: 'info'
      });
    }
    if (execution?.duration && execution.duration > 5000) {
      insights.push({
        type: 'performance',
        message: `执行耗时 ${execution.duration}ms，建议优化`,
        level: 'warning'
      });
    }
    return insights;
  }

  _generateSuggestions(execution) {
    const suggestions = [];
    if (execution?.strategy === 'standard') {
      suggestions.push({
        type: 'strategy',
        message: '考虑使用 plan_act 模式进行更深入的分析',
        priority: 'medium'
      });
    }
    return suggestions;
  }
}

class LearnerPlugin {
  constructor() {
    this.name = 'learner';
    this.version = '1.0';
    this.description = '学习与知识沉淀插件';
    this.learnedPatterns = [];
  }

  consolidate(state, context) {
    const learningResult = {
      patternsExtracted: 0,
      skillsUpdated: 0,
      memoryEnhanced: false,
      timestamp: Date.now()
    };

    if (state?.reflection?.quality >= 0.8) {
      const pattern = this._extractPattern(state);
      if (pattern) {
        this.learnedPatterns.push(pattern);
        learningResult.patternsExtracted = 1;
        learningResult.skillsUpdated = 1;
      }
    }

    learningResult.learnedPatterns = this.learnedPatterns.length;
    return learningResult;
  }

  _extractPattern(state) {
    if (!state?.reflection?.insights?.length) return null;
    return {
      id: `pattern_${Date.now()}`,
      trigger: state.perception?.type || 'unknown',
      strategy: state.plan?.strategy || 'standard',
      quality: state.reflection.quality,
      insights: state.reflection.insights.map(i => i.type),
      createdAt: Date.now()
    };
  }

  getLearnedPatterns() {
    return this.learnedPatterns;
  }
}

let _instance = null;

function getOrchestrationEngine(options) {
  if (!_instance) {
    _instance = new OrchestrationEngine(options);
  }
  return _instance;
}

module.exports = {
  OrchestrationEngine,
  EventBus,
  StateManager,
  PerceptionPlugin,
  MemoryPlugin,
  PlannerPlugin,
  ExecutorPlugin,
  ReflectorPlugin,
  LearnerPlugin,
  getOrchestrationEngine
};
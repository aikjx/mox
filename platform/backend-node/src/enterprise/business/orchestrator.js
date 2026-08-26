/**
 * ============================================================
 *  璇玑 RelGraph · 宇宙级业务流程架构
 * ============================================================
 *
 *  架构层级：L3Orchestration + L4Services · 业务流程处理核心
 *  设计模式：
 *    ┌───────────────────────────────────────────────────────┐
 *    │  业务模块(BizModule) → 业务管道(Pipeline) → 服务层    │
 *    │       ↑ 注册/发现           │ 中间件链        ↑ 实现    │
 *    │  行业包(Industry)     编排器(Orchestrator)   │ 调用    │
 *    │       ↓ 动态加载           │ 统一入口        │ 调用    │
 *    │  领域服务(DomainSvc) ← 规则引擎(RuleEng) ←──┘         │
 *    └───────────────────────────────────────────────────────┘
 *
 *  核心特性：
 *    1. 业务模块化注册：每个行业包=1个模块，可动态装卸
 *    2. Pipeline中间件链：鉴权→数据权限→校验→规则→持久化→事件→通知
 *    3. 服务层统一门面：所有业务CRUD + 自定义操作走同一编排器
 *    4. 事件驱动解耦：业务动作产事件，异步处理器订阅（审计/通知/报表/数据湖）
 *    5. 行业融合：相同实体在不同行业包中可叠加规则/视图/流程
 *
 *  与现有架构对齐：
 *    - Rust framework: TenantMode / Claims ←→ 本编排器的 ctx.tenant / ctx.user
 *    - mox-expert: 14专家评估 ←→ Orchestrator.evaluate() 作为后处理钩子
 *    - kg-hub: 图治理 ←→ 业务实体变更后自动同步入图 (OnSaved hook)
 *    - engine-kernel: 算子市场 ←→ 可将业务动作注册为算子 Capability
 * ============================================================
 */

'use strict';

const { EventEmitter } = require('events');

// ============================================================
// 一、核心类型定义
// ============================================================

/**
 * 业务操作类型（Pipeline各阶段一致的action）
 */
const BizAction = Object.freeze({
  // CRUD 基操
  CREATE:   'create',
  BATCH_CREATE: 'batch_create',
  GET:      'get',
  LIST:     'list',
  UPDATE:   'update',
  BATCH_UPDATE: 'batch_update',
  DELETE:   'delete',
  BATCH_DELETE: 'batch_delete',
  UPSERT:   'upsert',
  EXPORT:   'export',
  IMPORT:   'import',
  COUNT:    'count',
  AGGREGATE:'aggregate',
  // 业务操
  APPROVE:  'approve',
  REJECT:   'reject',
  TRANSFER: 'transfer',
  ASSIGN:   'assign',
  START_WF: 'start_workflow',
  CANCEL:   'cancel',
  ARCHIVE:  'archive',
  RESTORE:  'restore',
  LOCK:     'lock',
  UNLOCK:   'unlock',
  COPY:     'copy',
  MERGE:    'merge',
  SPLIT:    'split',
  REORDER:  'reorder',
  // 自定义
  CUSTOM:   'custom',
});

/**
 * 管道阶段（中间件挂载点）
 */
const PipeStage = Object.freeze({
  // 入口阶段
  GLOBAL_BEFORE:   'global:before',       // 最前置（全局）
  AUTHZ:           'authz',                // 鉴权：功能权限 + 数据权限
  VALIDATE:        'validate',             // 入参校验 + 规则引擎校验
  // 业务阶段
  BEFORE:          'before',               // 业务前钩子
  TRANSACTION:     'transaction',          // 事务执行
  MAIN:            'main',                 // 主逻辑
  AFTER:           'after',                // 业务后钩子
  // 出口阶段
  ENRICH:          'enrich',               // 结果补全（关联信息/显示名称）
  NOTIFY:          'notify',               // 通知：站内信/短信/邮件/钉钉
  EVENT:           'event',                // 事件发布（异步解耦）
  AUDIT:           'audit',                // 审计记录（不可篡改）
  GLOBAL_AFTER:    'global:after',         // 最后出口（全局）
  // 异常
  ERROR:           'error',                // 错误处理
});

/**
 * 事件类型（跨模块解耦）
 */
const BizEvent = Object.freeze({
  // CRUD 事件
  CREATED:     'biz:created',
  UPDATED:     'biz:updated',
  DELETED:     'biz:deleted',
  BATCH_DONE:  'biz:batch_done',
  IMPORTED:    'biz:imported',
  EXPORTED:    'biz:exported',
  // 状态事件
  STATUS_CHANGED: 'biz:status_changed',
  OWNER_CHANGED:  'biz:owner_changed',
  LOCKED:      'biz:locked',
  UNLOCKED:    'biz:unlocked',
  // 流程事件
  WF_STARTED:  'wf:started',
  WF_APPROVED: 'wf:task_approved',
  WF_REJECTED: 'wf:task_rejected',
  WF_COMPLETED:'wf:completed',
  // 集成事件
  SYNC_OUT:    'sync:out',       // 数据同步出
  SYNC_IN:     'sync:in',        // 数据同步入
  MQ_SEND:     'mq:send',        // 消息队列发送
  // 通知事件
  NOTIFY:      'notify:send',
});

// ============================================================
// 二、业务上下文 (BizContext) —— 贯穿整个Pipeline
// ============================================================

class BizContext {
  constructor({ tenant, user, request }) {
    // 身份
    this.tenantId  = tenant?.tenantId  || tenant?.id;
    this.tenant    = tenant;                  // TenantContext
    this.userId    = user?.userId    || user?.sub;
    this.user      = user;                    // Claims
    this.roles     = user?.roles     || [];
    this.perms     = user?.permissions || [];
    // 请求
    this.requestId = request?.requestId;
    this.traceId   = request?.traceId;
    this.clientIp  = request?.clientIp;
    this.userAgent = request?.userAgent;
    // 业务
    this.entityCode = null;
    this.entityId    = null;
    this.action      = null;
    // 数据
    this.input  = null;         // 原始入参
    this.params = null;         // 解析后参数（含分页/过滤/排序）
    this.data   = null;         // 当前业务数据（主对象）
    this.dataBefore = null;     // 变更前快照
    this.result = null;         // 输出结果
    // 审计
    this.auditLog = {
      action: null,
      result: 'success',
      error: null,
      changedFields: null,
      durationMs: 0,
    };
    // 流程控制
    this.stop = false;          // 中途停止管道
    this.skipStages = new Set(); // 跳过的阶段
    this.errors = [];           // 累计错误（软错误）
    this.warnings = [];         // 累计警告
    this.extra = new Map();     // 中间件之间传数据
    // 时间
    this.startAt = Date.now();
    this.stageTimings = {};     // 各阶段耗时
  }

  markStage(stage) {
    this.stageTimings[stage] = (this.stageTimings[stage] || 0) + 1;
  }

  setError(err, stage = 'unknown') {
    this.auditLog.result = 'fail';
    this.auditLog.error = { message: err.message, stack: err.stack, stage };
    this.errors.push(err);
  }

  addWarning(msg, meta) {
    this.warnings.push({ msg, meta, at: Date.now() });
  }

  finish() {
    this.auditLog.durationMs = Date.now() - this.startAt;
  }

  get(key)  { return this.extra.get(key); }
  set(key, v) { this.extra.set(key, v); return v; }
}

// ============================================================
// 三、管道中间件系统 (Pipeline + Middleware)
// ============================================================

/**
 * 管道：一组按阶段顺序执行的中间件
 * 每个中间件签名: async (ctx, next) => void
 *  - 调用 next() 进入下一中间件
 *  - 不调用 next() 则短路
 *  - 抛出异常则进入 ERROR 阶段
 */
class Pipeline {
  constructor(name) {
    this.name = name;
    this._handlers = {}; // stage → [handler]
    Object.values(PipeStage).forEach(s => { this._handlers[s] = []; });
  }

  /**
   * 注册中间件到指定阶段
   */
  use(stage, handler, priority = 500) {
    if (!this._handlers[stage]) throw new Error(`Unknown stage: ${stage}`);
    const entry = { handler, priority };
    const arr = this._handlers[stage];
    // 按优先级插入（小的在前，默认500）
    let i = arr.length;
    while (i > 0 && arr[i - 1].priority > priority) i--;
    arr.splice(i, 0, entry);
    return this;
  }

  /**
   * 便捷方法：前置中间件（高优先级）
   */
  beforeStage(stage, handler) { return this.use(stage, handler, 100); }
  /**
   * 便捷方法：后置中间件（低优先级）
   */
  afterStage(stage, handler)  { return this.use(stage, handler, 900); }

  /**
   * 执行管道
   */
  async run(ctx) {
    const orderedStages = [
      PipeStage.GLOBAL_BEFORE,
      PipeStage.AUTHZ,
      PipeStage.VALIDATE,
      PipeStage.BEFORE,
      PipeStage.TRANSACTION,
      PipeStage.MAIN,
      PipeStage.AFTER,
      PipeStage.ENRICH,
      PipeStage.NOTIFY,
      PipeStage.EVENT,
      PipeStage.AUDIT,
      PipeStage.GLOBAL_AFTER,
    ];
    try {
      for (const stage of orderedStages) {
        if (ctx.stop) break;
        if (ctx.skipStages.has(stage)) continue;
        ctx.markStage(stage);
        await this._runStage(ctx, stage);
      }
    } catch (err) {
      ctx.setError(err, 'pipeline');
      try { await this._runStage(ctx, PipeStage.ERROR); } catch (_) {}
      throw err;
    } finally {
      ctx.finish();
    }
    return ctx.result;
  }

  async _runStage(ctx, stage) {
    const handlers = this._handlers[stage] || [];
    if (handlers.length === 0) return;
    let idx = 0;
    const next = async () => {
      if (idx >= handlers.length) return;
      const { handler } = handlers[idx++];
      await handler(ctx, next);
    };
    await next();
  }
}

// ============================================================
// 四、业务模块 (BizModule) —— 可插拔的行业/领域包
// ============================================================

/**
 * 业务模块：一组相关实体 + 服务 + 规则 + 视图 + 流程的打包单元
 * 对应 meta_industry_package 的运行时实例
 */
class BizModule {
  constructor({ code, name, version = '1.0.0', description = '' }) {
    this.code = code;
    this.name = name;
    this.version = version;
    this.description = description;
    // 注册项
    this._entities  = new Map();   // entity_code → EntityDef
    this._services  = new Map();   // service_name → BizService
    this._rules     = [];          // RuleDef[]
    this._views     = [];          // ViewDef[]
    this._workflows = new Map();   // wf_code → WorkflowDef
    this._hooks     = new Map();   // entity_code:action → [fn]
    this._listeners = new Map();   // event_type → [fn]
    this._middlewares = [];        // {stage, handler, priority}
    // 状态
    this._installed  = false;
    this._enabled    = true;
    this._order      = 100;        // 加载顺序（小的先）
  }

  // ── 注册 API ──────────────────────────────────────────

  entity(def) { this._entities.set(def.code, def); return this; }
  service(name, svc) { this._services.set(name, svc); return this; }
  rule(def) { this._rules.push(def); return this; }
  view(def) { this._views.push(def); return this; }
  workflow(def) { this._workflows.set(def.code, def); return this; }
  hook(entityCode, action, fn) {
    const k = `${entityCode}:${action}`;
    const arr = this._hooks.get(k) || [];
    arr.push(fn);
    this._hooks.set(k, arr);
    return this;
  }
  on(event, fn) {
    const arr = this._listeners.get(event) || [];
    arr.push(fn);
    this._listeners.set(event, arr);
    return this;
  }
  middleware(stage, handler, priority = 500) {
    this._middlewares.push({ stage, handler, priority });
    return this;
  }

  // ── 查询 API ──────────────────────────────────────────

  getEntities()  { return Array.from(this._entities.values()); }
  getEntity(c)   { return this._entities.get(c); }
  getService(n)  { return this._services.get(n); }
  getServices()  { return Array.from(this._services.values()); }
  getRules()     { return this._rules.slice(); }
  getViews()     { return this._views.slice(); }
  getWorkflows() { return Array.from(this._workflows.values()); }
  getHooks(entityCode, action) {
    return this._hooks.get(`${entityCode}:${action}`) || [];
  }
  getListeners(event) { return this._listeners.get(event) || []; }
  getMiddlewares() { return this._middlewares.slice(); }

  // ── 生命周期 ──────────────────────────────────────────

  async onInstall(orchestrator)  { this._installed = true;  }
  async onUninstall(orchestrator){ this._installed = false; }
  async onEnable()  { this._enabled = true;  }
  async onDisable() { this._enabled = false; }
  get enabled() { return this._enabled; }
}

// ============================================================
// 五、业务服务基类 (BizService) —— 所有业务服务的统一基类
//     提供开箱即用的CRUD + 高级查询 + 聚合 + 工作流联动
// ============================================================

class BizService {
  constructor({ entityCode, entityDef, orchRef }) {
    this.entityCode = entityCode;
    this.entityDef  = entityDef;
    this._orch      = orchRef;     // Orchestrator 引用
    // 子类可覆盖的钩子（也可通过 BizModule.hook 注册）
    this.hooks = {
      beforeCreate:   null,    // async (ctx, data) => data
      beforeUpdate:   null,    // async (ctx, data, before) => data
      beforeDelete:   null,    // async (ctx, before) => void
      afterCreate:    null,    // async (ctx, result) => void
      afterUpdate:    null,    // async (ctx, result, before) => void
      afterDelete:    null,    // async (ctx, before) => void
      afterGet:       null,    // async (ctx, result) => result
      afterList:      null,    // async (ctx, result) => result
    };
  }

  get orch() {
    if (!this._orch) throw new Error('BizService not bound to Orchestrator');
    return this._orch;
  }

  // ── 基础 CRUD（全部走 Orchestrator 管道）───────────────

  async create(ctx, data) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.CREATE;
    ctx.input = data;
    return this.orch.execute(ctx);
  }

  async batchCreate(ctx, items) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.BATCH_CREATE;
    ctx.input = items;
    return this.orch.execute(ctx);
  }

  async get(ctx, id, options = {}) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.GET;
    ctx.input = { id, ...options };
    return this.orch.execute(ctx);
  }

  async list(ctx, params = {}) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.LIST;
    ctx.input = params;
    return this.orch.execute(ctx);
  }

  async update(ctx, id, updates, options = {}) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.UPDATE;
    ctx.input = { id, updates, ...options };
    return this.orch.execute(ctx);
  }

  async batchUpdate(ctx, filters, updates) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.BATCH_UPDATE;
    ctx.input = { filters, updates };
    return this.orch.execute(ctx);
  }

  async delete(ctx, id, options = {}) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.DELETE;
    ctx.input = { id, ...options };
    return this.orch.execute(ctx);
  }

  async batchDelete(ctx, filters) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.BATCH_DELETE;
    ctx.input = { filters };
    return this.orch.execute(ctx);
  }

  async upsert(ctx, uniqueBy, data) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.UPSERT;
    ctx.input = { uniqueBy, data };
    return this.orch.execute(ctx);
  }

  async count(ctx, params = {}) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.COUNT;
    ctx.input = params;
    return this.orch.execute(ctx);
  }

  async export(ctx, params = {}) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.EXPORT;
    ctx.input = params;
    return this.orch.execute(ctx);
  }

  async import(ctx, rows, params = {}) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.IMPORT;
    ctx.input = { rows, params };
    return this.orch.execute(ctx);
  }

  // ── 工作流联动 ──────────────────────────────────────────

  async startWorkflow(ctx, workflowCode, bizId, formData) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.START_WF;
    ctx.input = { workflowCode, bizId, formData };
    return this.orch.execute(ctx);
  }

  // ── 自定义操作 ──────────────────────────────────────────

  /**
   * 执行自定义操作（子类可重写）
   * custom(ctx, params) 可在内部直接调 dataStore，不走管道
   * 或调用 orch.execute(ctx, custom: true) 仍走管道
   */
  async custom(ctx, opName, params = {}) {
    ctx.entityCode = this.entityCode;
    ctx.action = BizAction.CUSTOM;
    ctx.input = { opName, params };
    ctx.set('custom_op', opName);
    return this.orch.execute(ctx);
  }
}

// ============================================================
// 六、编排器 (Orchestrator) —— 业务流程总调度
//     唯一入口：orchestrator.execute(ctx)
//     所有业务操作必须通过此入口，保证鉴权/审计/规则/事件全闭环
// ============================================================

class Orchestrator extends EventEmitter {
  constructor({ dataStore, ruleEngine, workflowEngine, eventBus, auditLogger, moduleRegistry }) {
    super();
    this.setMaxListeners(200);
    // 外部依赖（可替换实现）
    this.dataStore      = dataStore;      // 数据持久化
    this.ruleEngine     = ruleEngine;     // 规则引擎
    this.workflowEngine = workflowEngine; // 工作流引擎
    this.eventBus       = eventBus || this;
    this.auditLogger    = auditLogger;
    this.moduleRegistry = moduleRegistry;

    // 管道
    this.pipeline = new Pipeline('orchestrator');
    this._installDefaultMiddlewares();

    // 扩展点
    this._customActions = new Map(); // entityCode:opName → fn(ctx)

    // 统计
    this._metrics = {
      totalCalls: 0,
      successCalls: 0,
      failCalls: 0,
      actionHistogram: {},
      durationBuckets: [0, 0, 0, 0, 0], // <10ms <100ms <1s <10s >10s
    };
  }

  // ── 模块管理 ────────────────────────────────────────────

  registerModule(module) {
    if (!this.moduleRegistry) this.moduleRegistry = new Map();
    this.moduleRegistry.set(module.code, module);
    // 挂载模块中间件
    for (const { stage, handler, priority } of module.getMiddlewares()) {
      this.pipeline.use(stage, handler, priority);
    }
    // 挂载模块服务
    for (const svc of module.getServices()) {
      svc._orch = this;
    }
    // 挂载模块监听器到事件总线
    for (const [ev, fns] of module._listeners || []) {
      for (const fn of fns) this.eventBus.on(ev, fn);
    }
    return this;
  }

  getModule(code) {
    return this.moduleRegistry?.get(code);
  }

  listModules() {
    return this.moduleRegistry
      ? Array.from(this.moduleRegistry.values())
      : [];
  }

  // ── 自定义操作注册 ─────────────────────────────────────

  registerCustomAction(entityCode, opName, handler) {
    this._customActions.set(`${entityCode}:${opName}`, handler);
    return this;
  }

  // ── 核心执行：唯一入口 ─────────────────────────────────

  async execute(ctx) {
    if (!(ctx instanceof BizContext)) {
      ctx = new BizContext(ctx);
    }
    this._metrics.totalCalls++;
    ctx.params = this._parseParams(ctx);

    try {
      const result = await this.pipeline.run(ctx);
      this._metrics.successCalls++;
      this._recordDuration(ctx.auditLog.durationMs);
      this._metrics.actionHistogram[ctx.action] =
        (this._metrics.actionHistogram[ctx.action] || 0) + 1;
      return {
        success: true,
        data: result,
        warnings: ctx.warnings.length ? ctx.warnings : undefined,
        meta: { requestId: ctx.requestId, durationMs: ctx.auditLog.durationMs },
      };
    } catch (err) {
      this._metrics.failCalls++;
      this._recordDuration(ctx.auditLog.durationMs);
      return {
        success: false,
        error: { code: err.code || 'E_UNKNOWN', message: err.message, details: err.details },
        warnings: ctx.warnings.length ? ctx.warnings : undefined,
        meta: { requestId: ctx.requestId, durationMs: ctx.auditLog.durationMs },
      };
    }
  }

  // ── 指标 ───────────────────────────────────────────────

  getMetrics() {
    return {
      ...this._metrics,
      failRate: this._metrics.totalCalls
        ? (this._metrics.failCalls / this._metrics.totalCalls).toFixed(4)
        : '0',
    };
  }

  // ============================================================
  // 内部：默认中间件
  // ============================================================

  _installDefaultMiddlewares() {
    // 0. GLOBAL_BEFORE: 打点
    this.pipeline.use(PipeStage.GLOBAL_BEFORE, async (ctx, next) => {
      ctx.set('start_stage_ms', Date.now());
      await next();
    });

    // 1. AUTHZ: 鉴权 + 数据权限
    this.pipeline.use(PipeStage.AUTHZ, async (ctx, next) => {
      await this._authz(ctx);
      await next();
    });

    // 2. VALIDATE: 入参校验 + 规则引擎
    this.pipeline.use(PipeStage.VALIDATE, async (ctx, next) => {
      await this._validate(ctx);
      await next();
    });

    // 3. BEFORE: 模块钩子 beforeXxx
    this.pipeline.use(PipeStage.BEFORE, async (ctx, next) => {
      await this._runModuleHooks(ctx, 'before');
      await next();
    });

    // 4. TRANSACTION: 事务包裹
    this.pipeline.use(PipeStage.TRANSACTION, async (ctx, next) => {
      // dataStore 负责提供事务；若不支持则直接执行
      if (this.dataStore?.withTransaction) {
        ctx.result = await this.dataStore.withTransaction(async (tx) => {
          ctx.set('tx', tx);
          await next();
          return ctx.result;
        });
      } else {
        await next();
      }
    });

    // 5. MAIN: 主业务逻辑（核心CRUD / 自定义操作）
    this.pipeline.use(PipeStage.MAIN, async (ctx, next) => {
      await this._mainLogic(ctx);
      await next();
    });

    // 6. AFTER: 模块钩子 afterXxx
    this.pipeline.use(PipeStage.AFTER, async (ctx, next) => {
      await this._runModuleHooks(ctx, 'after');
      await next();
    });

    // 7. ENRICH: 结果补全（关联字段、字典翻译、权限标记）
    this.pipeline.use(PipeStage.ENRICH, async (ctx, next) => {
      await this._enrich(ctx);
      await next();
    });

    // 8. NOTIFY: 通知
    this.pipeline.use(PipeStage.NOTIFY, async (ctx, next) => {
      await next();
      await this._notify(ctx);
    });

    // 9. EVENT: 事件发布（异步，不阻塞）
    this.pipeline.use(PipeStage.EVENT, async (ctx, next) => {
      await next();
      this._publishEvents(ctx); // 非阻塞
    });

    // 10. AUDIT: 审计
    this.pipeline.use(PipeStage.AUDIT, async (ctx, next) => {
      await next();
      await this._audit(ctx);
    });

    // 11. GLOBAL_AFTER: 清理
    this.pipeline.use(PipeStage.GLOBAL_AFTER, async (ctx, next) => {
      await next();
      // 可放熔断/降级统计
    });

    // ERROR: 错误处理
    this.pipeline.use(PipeStage.ERROR, async (ctx, next) => {
      // 错误审计
      if (this.auditLogger) {
        try { await this.auditLogger.write({ ctx, isError: true }); } catch (_) {}
      }
      await next();
    });
  }

  // ── 阶段实现 ───────────────────────────────────────────

  async _authz(ctx) {
    // 功能权限：查 iam_permission → iam_user_role
    if (!this.dataStore?.checkPermission) return;
    const permCode = `${ctx.entityCode}:${ctx.action}`;
    const hasPerm = await this.dataStore.checkPermission({
      tenantId: ctx.tenantId,
      userId: ctx.userId,
      roles: ctx.roles,
      permission: permCode,
      entityCode: ctx.entityCode,
    });
    if (!hasPerm) {
      const err = new Error(`无权限: ${permCode}`);
      err.code = 'E_PERMISSION_DENIED';
      throw err;
    }
    // 数据权限：注入 scope 过滤条件到 ctx.params.filter
    const dataScope = await this.dataStore.resolveDataScope({
      tenantId: ctx.tenantId,
      userId: ctx.userId,
      roles: ctx.roles,
      entityCode: ctx.entityCode,
    });
    if (dataScope && ctx.params) {
      ctx.params.dataScope = dataScope;
    }
  }

  async _validate(ctx) {
    // 1. 实体字段级校验（meta_field.validations）
    if (this.dataStore?.validateEntity && (ctx.action === BizAction.CREATE || ctx.action === BizAction.UPDATE)) {
      const errors = await this.dataStore.validateEntity({
        tenantId: ctx.tenantId,
        entityCode: ctx.entityCode,
        data: ctx.action === BizAction.CREATE ? ctx.input : ctx.input.updates,
        action: ctx.action,
      });
      if (errors && errors.length) {
        const err = new Error('数据校验失败');
        err.code = 'E_VALIDATION';
        err.details = errors;
        throw err;
      }
    }
    // 2. 规则引擎：validation/calculation/linkage 类规则
    if (this.ruleEngine) {
      const ruleResult = await this.ruleEngine.run({
        tenantId: ctx.tenantId,
        entityCode: ctx.entityCode,
        action: ctx.action,
        data: ctx.input,
        event: `ACTION_${ctx.action.toUpperCase()}`,
      });
      if (ruleResult?.blocked) {
        const err = new Error(ruleResult.message || '规则拦截');
        err.code = 'E_RULE_BLOCK';
        err.details = ruleResult.violations;
        throw err;
      }
      // calculation规则的结果回写到input
      if (ruleResult?.calculated) {
        ctx.input = { ...ctx.input, ...ruleResult.calculated };
      }
    }
  }

  async _runModuleHooks(ctx, phase) {
    const hooks = [];
    for (const mod of this.listModules()) {
      if (!mod.enabled) continue;
      hooks.push(...mod.getHooks(ctx.entityCode, ctx.action));
    }
    // BizService 实例钩子
    const svc = this._findService(ctx.entityCode);
    if (svc) {
      const fn = svc.hooks[`${phase}${this._hookSuffix(ctx.action)}`];
      if (typeof fn === 'function') hooks.push(fn);
    }
    for (const hook of hooks) {
      try {
        await hook(ctx, phase === 'before' ? ctx.input : ctx.result, ctx.dataBefore);
      } catch (err) {
        ctx.addWarning(`hook failed: ${err.message}`, { phase, action: ctx.action });
      }
    }
  }

  _hookSuffix(action) {
    const map = {
      [BizAction.CREATE]:   'Create',
      [BizAction.UPDATE]:   'Update',
      [BizAction.DELETE]:   'Delete',
      [BizAction.GET]:      'Get',
      [BizAction.LIST]:     'List',
    };
    return map[action] || '';
  }

  async _mainLogic(ctx) {
    // 自定义操作优先
    if (ctx.action === BizAction.CUSTOM) {
      const opName = ctx.get('custom_op');
      const customHandler = this._customActions.get(`${ctx.entityCode}:${opName}`);
      const svc = this._findService(ctx.entityCode);
      if (customHandler) {
        ctx.result = await customHandler(ctx, ctx.input?.params);
        return;
      }
      if (svc && typeof svc[opName] === 'function') {
        ctx.result = await svc[opName](ctx, ctx.input?.params);
        return;
      }
      const err = new Error(`未知自定义操作: ${ctx.entityCode}.${opName}`);
      err.code = 'E_OP_NOT_FOUND';
      throw err;
    }

    if (!this.dataStore) {
      throw new Error('Orchestrator 缺少 dataStore 配置');
    }
    const ds = this.dataStore;
    const scope = ctx.params?.dataScope;
    const tx = ctx.get('tx');

    switch (ctx.action) {
      case BizAction.CREATE:
        ctx.dataBefore = null;
        ctx.result = await ds.create({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, data: ctx.input, userId: ctx.userId, tx });
        ctx.data = ctx.result;
        break;
      case BizAction.BATCH_CREATE:
        ctx.result = await ds.batchCreate({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, items: ctx.input, userId: ctx.userId, tx });
        break;
      case BizAction.GET:
        ctx.result = await ds.get({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, id: ctx.input.id, options: ctx.input, scope, tx });
        ctx.data = ctx.result;
        break;
      case BizAction.LIST:
        ctx.result = await ds.list({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, params: ctx.params, scope, tx });
        break;
      case BizAction.UPDATE: {
        ctx.dataBefore = await ds.get({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, id: ctx.input.id, tx });
        ctx.result = await ds.update({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, id: ctx.input.id, updates: ctx.input.updates, userId: ctx.userId, tx });
        ctx.data = ctx.result;
        // 变更字段审计
        if (ctx.dataBefore && ctx.result) {
          ctx.auditLog.changedFields = this._diffKeys(ctx.dataBefore, ctx.result);
        }
        break;
      }
      case BizAction.BATCH_UPDATE:
        ctx.result = await ds.batchUpdate({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, filters: ctx.input.filters, updates: ctx.input.updates, scope, userId: ctx.userId, tx });
        break;
      case BizAction.DELETE:
        ctx.dataBefore = await ds.get({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, id: ctx.input.id, tx });
        ctx.result = await ds.delete({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, id: ctx.input.id, userId: ctx.userId, soft: ctx.input.soft !== false, tx });
        break;
      case BizAction.BATCH_DELETE:
        ctx.result = await ds.batchDelete({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, filters: ctx.input.filters, scope, userId: ctx.userId, tx });
        break;
      case BizAction.UPSERT:
        ctx.result = await ds.upsert({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, uniqueBy: ctx.input.uniqueBy, data: ctx.input.data, userId: ctx.userId, tx });
        ctx.data = ctx.result;
        break;
      case BizAction.COUNT:
        ctx.result = await ds.count({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, params: ctx.params, scope, tx });
        break;
      case BizAction.EXPORT:
        ctx.result = await ds.export({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, params: ctx.params, scope, userId: ctx.userId });
        break;
      case BizAction.IMPORT:
        ctx.result = await ds.import({ tenantId: ctx.tenantId, entityCode: ctx.entityCode, rows: ctx.input.rows, params: ctx.input.params, userId: ctx.userId, tx });
        break;
      case BizAction.START_WF:
        if (!this.workflowEngine) throw new Error('缺少 workflowEngine');
        ctx.result = await this.workflowEngine.start({
          tenantId: ctx.tenantId, userId: ctx.userId,
          workflowCode: ctx.input.workflowCode,
          bizId: ctx.input.bizId, entityCode: ctx.entityCode,
          formData: ctx.input.formData,
        });
        break;
      default:
        // 交给 service 自定义
        const svc = this._findService(ctx.entityCode);
        if (svc && typeof svc[ctx.action] === 'function') {
          ctx.result = await svc[ctx.action](ctx, ctx.input);
        } else {
          const err = new Error(`不支持的操作: ${ctx.action}`);
          err.code = 'E_ACTION_UNSUPPORTED';
          throw err;
        }
    }
  }

  async _enrich(ctx) {
    if (!ctx.result) return;
    // 字典翻译 + 关联字段补全
    if (this.dataStore?.enrich) {
      ctx.result = await this.dataStore.enrich({
        tenantId: ctx.tenantId,
        entityCode: ctx.entityCode,
        data: ctx.result,
        action: ctx.action,
      });
    }
  }

  async _notify(ctx) {
    const silent = ctx.input?.__silent;
    if (silent) return;
    const notifications = this._buildNotifications(ctx);
    if (!notifications.length) return;
    // 事件总线异步分发（由通知模块订阅）
    for (const n of notifications) {
      this.eventBus.emit(BizEvent.NOTIFY, n);
    }
  }

  _buildNotifications(ctx) {
    // 根据动作类型组装通知（简单实现：创建/删除/审批触发）
    const triggerMap = {
      [BizAction.CREATE]:   BizEvent.CREATED,
      [BizAction.UPDATE]:   BizEvent.UPDATED,
      [BizAction.DELETE]:   BizEvent.DELETED,
    };
    const ev = triggerMap[ctx.action];
    if (!ev) return [];
    return [{
      tenantId: ctx.tenantId,
      event: ev,
      entityCode: ctx.entityCode,
      bizId: ctx.result?.biz_id || ctx.input?.id,
      operatorId: ctx.userId,
      data: ctx.result,
    }];
  }

  _publishEvents(ctx) {
    const triggerMap = {
      [BizAction.CREATE]:   BizEvent.CREATED,
      [BizAction.UPDATE]:   BizEvent.UPDATED,
      [BizAction.DELETE]:   BizEvent.DELETED,
      [BizAction.IMPORT]:   BizEvent.IMPORTED,
      [BizAction.EXPORT]:   BizEvent.EXPORTED,
    };
    const ev = triggerMap[ctx.action];
    if (!ev) return;
    const payload = {
      tenantId: ctx.tenantId,
      entityCode: ctx.entityCode,
      bizId: ctx.result?.biz_id || ctx.input?.id,
      userId: ctx.userId,
      data: ctx.result,
      dataBefore: ctx.dataBefore,
      changedFields: ctx.auditLog.changedFields,
      traceId: ctx.traceId,
      requestId: ctx.requestId,
    };
    try {
      this.eventBus.emit(ev, payload);
      // 同步入 kg-hub（如果可用）
      this.eventBus.emit(BizEvent.SYNC_OUT, { graph: true, ...payload });
    } catch (err) {
      ctx.addWarning('事件发布失败', err.message);
    }
  }

  async _audit(ctx) {
    if (!this.auditLogger) return;
    try {
      await this.auditLogger.write({
        ctx,
        tenantId: ctx.tenantId,
        userId: ctx.userId,
        actionDomain: 'biz',
        actionModule: ctx.entityCode,
        actionName: ctx.action,
        targetId: ctx.result?.biz_id || ctx.input?.id,
        result: ctx.auditLog.result,
        durationMs: ctx.auditLog.durationMs,
        snapshotBefore: ctx.dataBefore,
        snapshotAfter: ctx.result,
        changedFields: ctx.auditLog.changedFields,
        clientIp: ctx.clientIp,
        userAgent: ctx.userAgent,
        requestId: ctx.requestId,
        traceId: ctx.traceId,
      });
    } catch (_) { /* 审计失败不影响主流程 */ }
  }

  // ── 工具方法 ─────────────────────────────────────────

  _parseParams(ctx) {
    const input = ctx.input || {};
    const params = {};
    // 分页
    params.page     = Math.max(1, Number(input.page) || 1);
    params.pageSize = Math.min(1000, Number(input.pageSize) || 20);
    // 过滤
    params.filters  = input.filters || input.where || {};
    params.search   = input.search || input.keyword;
    // 排序
    params.sorts    = input.sorts || input.orderBy || [];
    // 字段选择
    params.fields   = input.fields || input.select;
    // 关联
    params.includes = input.includes || input.with;
    // 去重/分组
    params.distinct = input.distinct;
    params.groupBy  = input.groupBy;
    params.aggregations = input.aggregations;
    return params;
  }

  _findService(entityCode) {
    for (const mod of this.listModules()) {
      const svc = mod.getService(entityCode);
      if (svc) return svc;
    }
    return null;
  }

  _diffKeys(a, b) {
    if (!a || !b) return null;
    const changed = [];
    const allKeys = new Set([...Object.keys(a), ...Object.keys(b)]);
    for (const k of allKeys) {
      const va = a[k], vb = b[k];
      if (JSON.stringify(va) !== JSON.stringify(vb)) changed.push(k);
      if (changed.length > 100) break;
    }
    return changed;
  }

  _recordDuration(ms) {
    const b = this._metrics.durationBuckets;
    if (ms < 10)        b[0]++;
    else if (ms < 100)  b[1]++;
    else if (ms < 1000) b[2]++;
    else if (ms < 10000)b[3]++;
    else                b[4]++;
  }
}

// ============================================================
// 七、业务模块注册表示例（行业融合架构演示）
//    —— 展示如何让6大行业包无缝融合到同一编排器中
// ============================================================

function createIndustryModules() {
  const common = new BizModule({ code: 'common', name: '通用基础包', version: '1.0.0' });
  const gov    = new BizModule({ code: 'gov',    name: '政务服务包', version: '1.0.0' });
  const fin    = new BizModule({ code: 'finance',name: '金融业务包', version: '1.0.0' });
  const med    = new BizModule({ code: 'medical',name: '医疗健康包', version: '1.0.0' });
  const mfg    = new BizModule({ code: 'manufacturing', name: '智能制造包', version: '1.0.0' });
  const edu    = new BizModule({ code: 'education', name: '智慧教育包', version: '1.0.0' });
  const rtl    = new BizModule({ code: 'retail', name: '智慧零售包', version: '1.0.0' });

  // 示例：通用包的项目审批联动
  common.hook('project', BizAction.CREATE, async (ctx) => {
    ctx.set('need_wf', (ctx.input?.amount || 0) > 10000);
  });
  common.hook('project', BizAction.CREATE, async (ctx, result) => {
    if (ctx.get('need_wf') && ctx.result?.biz_id) {
      // 异步启动大额项目审批流
      setImmediate(async () => {
        try {
          await ctx.orchRef?.workflowEngine?.start({
            tenantId: ctx.tenantId, userId: ctx.userId,
            workflowCode: 'project_budget_approval',
            bizId: ctx.result.biz_id, entityCode: 'project',
          });
        } catch (_) {}
      });
    }
  });

  // 示例：金融包的客户信用分联动计算
  fin.hook('fin_customer', BizAction.UPDATE, async (ctx, result, before) => {
    if (ctx.auditLog.changedFields?.includes('total_assets')) {
      ctx.addWarning('资产变更，信用分将异步重算');
    }
  });

  // 示例：制造包的工单质量自动触发
  mfg.hook('mfg_workorder', BizAction.UPDATE, async (ctx, result, before) => {
    if (before?.status !== 'completed' && result?.status === 'completed' && result?.defect_rate > 0.05) {
      // 高缺陷率自动生成QC单（通过事件解耦）
      ctx.orchRef?.eventBus?.emit('qc:create_from_workorder', { source: result });
    }
  });

  // 示例：医疗包的就诊完成自动结算
  med.hook('med_visit', BizAction.UPDATE, async (ctx, result) => {
    if (result?.status === 'finished' && !result?.settled) {
      ctx.orchRef?.eventBus?.emit('insurance:settle', { visitId: result.biz_id });
    }
  });

  return { common, gov, fin, med, mfg, edu, rtl };
}

// ============================================================
// 导出
// ============================================================

module.exports = {
  // 枚举
  BizAction,
  PipeStage,
  BizEvent,
  // 核心类
  BizContext,
  Pipeline,
  BizModule,
  BizService,
  Orchestrator,
  // 行业包工厂
  createIndustryModules,
};

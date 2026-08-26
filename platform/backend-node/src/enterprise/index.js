/**
 * ============================================================
 *  璇玑 RelGraph · 宇宙级企业架构入口
 * ============================================================
 *
 *  本文件将：
 *    ① IAM 身份体系（租户/部门/用户/角色/权限/菜单/审计）
 *    ② 元数据引擎（实体/动态字段/视图/工作流/规则/编号/字典/行业包）
 *    ③ 业务流程编排器（Pipeline / BizModule / BizService / Orchestrator）
 *    ④ 数据存储层（DataStore + 通用表 biz_data + 版本历史）
 *  统一组装为一个 EnterpriseBootstrap，一行代码完成企业级系统初始化。
 *
 *  使用方式（最简）：
 *    const { EnterpriseBootstrap } = require('./src/enterprise');
 *    const app = await EnterpriseBootstrap.start({
 *      db: betterSqlite3Instance,       // 可替换为 Postgres/MySQL 驱动
 *      installIndustries: ['common','finance','medical'],
 *    });
 *    const result = await app.orchestrator.execute({ tenant, user, request, action, entityCode, input });
 *
 *  使用方式（专家联盟/图谱联动）：
 *    app.on('biz:created', payload => app.kgHub.syncEntity(payload));
 *    app.orchestrator.use('after', async (ctx, next) => {
 *      await app.moxExpert.evaluate(ctx);
 *      await next();
 *    });
 * ============================================================
 */

'use strict';

const { Enums: IamEnums, DDL: IamDDL, SeedData: IamSeed } = require('./iam/schema');
const { MetaDDL, IndustryPackages }                   = require('./meta/schema');
const {
  BizAction, PipeStage, BizEvent,
  BizContext, Pipeline, BizModule, BizService, Orchestrator, createIndustryModules,
} = require('./business/orchestrator');
const { DataStore, MetaCache, FieldSlotAllocator, UniversalBizDAO, uuidv7 } = require('./business/data-store');
const { EventEmitter } = require('events');

/**
 * 顶层启动器：DDL建表 + 种子数据 + 模块安装 + 服务装配
 */
class EnterpriseBootstrap extends EventEmitter {
  constructor(options = {}) {
    super();
    this.options = {
      // 默认选项
      journalMode:         'WAL',
      foreignKeys:         true,
      synchronous:         'NORMAL',
      installIndustries:   ['common'],
      seedIamBuiltins:     true,
      enableAuditChain:    true,       // 审计链式签名防篡改
      // 外部依赖注入（可选，默认全自举）
      db:                  null,       // better-sqlite3 / 兼容对象
      dbPath:              null,       // 若未提供db，则用better-sqlite3从此路径打开
      ruleEngine:          null,       // 可注入，否则用空实现
      workflowEngine:      null,       // 可注入，否则用空实现
      auditLogger:         null,       // 可注入，否则用内置实现
      logger:              console,
      ...options,
    };
    this.db               = null;
    this.metaCache        = null;
    this.dataStore        = null;
    this.orchestrator     = null;
    this.moduleRegistry   = new Map();
    this.industryRegistry = new Map(); // code → IndustryPackage
    this._started         = false;
  }

  // ============================================================
  // 启动流程
  // ============================================================
  async start() {
    if (this._started) return this;
    const o = this.options;

    // 1. 初始化 DB
    this._initDB();

    // 2. 建表（IAM + META + FLOW + BIZ）
    this._runDDL();

    // 3. 元数据缓存
    this.metaCache = new MetaCache(this.db);

    // 4. 审计记录器
    if (!o.auditLogger) {
      o.auditLogger = this._createBuiltinAuditLogger();
    }
    this.auditLogger = o.auditLogger;

    // 5. 规则引擎 / 工作流引擎（空实现占位，生产需具体实现对接）
    this.ruleEngine     = o.ruleEngine     || new DummyRuleEngine();
    this.workflowEngine = o.workflowEngine || new DummyWorkflowEngine();

    // 6. DataStore
    this.dataStore = new DataStore({
      db: this.db,
      metaCache: this.metaCache,
      auditLogger: this.auditLogger,
    });

    // 7. Orchestrator
    this.orchestrator = new Orchestrator({
      dataStore:      this.dataStore,
      ruleEngine:     this.ruleEngine,
      workflowEngine: this.workflowEngine,
      auditLogger:    this.auditLogger,
      moduleRegistry: this.moduleRegistry,
    });
    // 事件转发
    this.orchestrator.onAny?.() || this._bridgeOrchEvents();

    // 8. IAM 种子数据（内置角色/权限）
    if (o.seedIamBuiltins) {
      this._seedIamBuiltins();
    }

    // 9. 行业包安装
    this._installIndustryPackages();

    // 10. 开发者模式：注册 6 个行业模块（演示行业融合）
    this._registerDemoIndustryModules();

    this._started = true;
    o.logger.info?.(`[Enterprise] 启动完成. 已安装行业包: ${o.installIndustries.join(',')}`);
    return this;
  }

  // ============================================================
  // 便捷入口：动态定义实体/字段（写入 DB + 刷新缓存）
  // 用于：页面配置、行业包安装、测试、二次开发融合
  // ============================================================
  defineEntity({ tenantId = 'system', entityCode, entityName, entityCategory = 'master',
                 storageMode = 'universal', fields = [] }) {
    if (!entityCode) throw new Error('entityCode required');
    const now = new Date().toISOString();
    // 1. 写 meta_entity（存在则更新版本号）
    const existing = this.db.prepare(
      `SELECT entity_id, version FROM meta_entity WHERE tenant_id=? AND entity_code=? LIMIT 1`
    ).get(tenantId, entityCode);
    let entityId;
    if (existing) {
      entityId = existing.entity_id;
      this.db.prepare(`UPDATE meta_entity SET entity_name=?, entity_category=?, storage_mode=?, updated_at=?, version=version+1
                       WHERE entity_id=?`)
        .run(entityName || entityCode, entityCategory, storageMode, now, entityId);
    } else {
      entityId = uuidv7();
      this.db.prepare(`
        INSERT INTO meta_entity
        (entity_id, tenant_id, entity_code, entity_name, entity_category, storage_mode, is_system, status, created_at, updated_at, version)
        VALUES (?,?,?,?,?,?, 0, 'active',?,?, 1)
      `).run(entityId, tenantId, entityCode, entityName || entityCode, entityCategory, storageMode, now, now);
    }
    // 2. 分配字段槽位并写 meta_field
    const allocator = new FieldSlotAllocator();
    const slotMap = allocator.allocate(entityCode, fields.map(f => ({
      field_code: f.field_code, field_type: f.field_type || 'string',
      is_required: !!f.required, is_searchable: !!f.searchable, is_sortable: !!f.sortable,
      is_filterable: !!f.filterable, is_indexed: !!f.indexed,
    })));
    const insF = this.db.prepare(`
      INSERT OR REPLACE INTO meta_field
      (field_id, tenant_id, entity_id, field_code, field_name, field_type,
       is_required, is_indexed, is_searchable, is_sortable, is_filterable,
       default_value, options_source, options_inline,
       ui_component, ui_sort_order, status, created_at, updated_at, version)
      VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?, 1)
    `);
    for (let i = 0; i < fields.length; i++) {
      const f = fields[i];
      insF.run(
        uuidv7(), tenantId, entityId, f.field_code, f.field_name || f.field_code,
        f.field_type || 'string',
        f.required ? 1 : 0,
        f.indexed ? 1 : (f.required ? 1 : 0),
        f.searchable ? 1 : (f.required ? 1 : 0),
        f.sortable ? 1 : 0,
        f.filterable ? 1 : 0,
        f.default_value == null ? null : (typeof f.default_value === 'object' ? JSON.stringify(f.default_value) : String(f.default_value)),
        f.options_inline ? 'inline' : (f.options_sql ? 'sql' : (f.options_api ? 'api' : (f.options_dict_code ? 'dictionary' : 'inline'))),
        f.options_inline ? JSON.stringify(f.options_inline) : null,
        f.ui_component || f.ui_widget || 'input',
        typeof f.ui_sort_order === 'number' ? f.ui_sort_order : (i + 1) * 10,
        'active', now, now,
      );
    }
    // 3. 缓存失效
    this.metaCache.invalidate(tenantId, entityCode);
    return { entityId, tenantId, entityCode, fieldsCount: fields.length };
  }

  // ============================================================
  // 便捷入口：业务执行
  // ============================================================
  async execute({ tenant, user, request, entityCode, action, input }) {
    const ctx = new BizContext({ tenant, user, request });
    ctx.entityCode = entityCode;
    ctx.action = action;
    ctx.input = input;
    return this.orchestrator.execute(ctx);
  }

  service(entityCode) {
    return new BizService({ entityCode, orchRef: this.orchestrator });
  }

  // ============================================================
  // 内部：DB
  // ============================================================
  _initDB() {
    const o = this.options;
    if (o.db) { this.db = o.db; return; }
    // 未提供db实例 → 尝试 better-sqlite3
    try {
      const Database = require('better-sqlite3');
      const fs = require('fs');
      const dir = o.dbPath ? require('path').dirname(o.dbPath) : null;
      if (dir && !fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
      this.db = new Database(o.dbPath || ':memory:');
    } catch (e) {
      throw new Error(`无法初始化数据库: ${e.message}，请传入 options.db 或 options.dbPath`);
    }
    if (typeof this.db.pragma === 'function') {
      this.db.pragma(`journal_mode = ${o.journalMode}`);
      this.db.pragma(`foreign_keys = ${o.foreignKeys ? 'ON' : 'OFF'}`);
      this.db.pragma(`synchronous = ${o.synchronous}`);
    }
  }

  _runDDL() {
    let ddl = IamDDL + '\n' + MetaDDL;
    ddl = this._normalizeDDLForSQLite(ddl);
    if (typeof this.db.exec === 'function') {
      const stmts = this._splitStatements(ddl);
      for (let i = 0; i < stmts.length; i++) {
        const s = stmts[i];
        try {
          this.db.exec(s);
        } catch (e) {
          if (/already exists/i.test(e.message)) continue;
          this.options.logger.error?.(
            '[DDL ERROR] stmt #' + i,
            e.message,
            '\nPrev SQL: ', (stmts[i-1]||'').slice(0,200),
            '\nThis SQL: ', s.slice(0, 600),
          );
          throw e;
        }
      }
    }
  }

  /**
   * 将 MySQL 风格 DDL 规范化为 SQLite 可执行形式
   */
  _normalizeDDLForSQLite(sql) {
    // Step 0: 全局安全剥注释（先保护字符串字面量，再剥 /* */、--、// 风格）
    const strs = [];
    sql = sql.replace(/'([^'\\]|\\.)*'/g, s => { strs.push(s); return `'__ST${strs.length-1}__'`; });
    sql = sql.replace(/\/\*[\s\S]*?\*\//g, ' ');       // 块注释 /* ... */
    sql = sql.replace(/(^|\s)\-\-.*?$/gm, '$1');        // 行注释 --
    sql = sql.replace(/(^|\s)\/\/.*?$/gm, '$1');        // 行注释 // （来源：DDL内嵌代码框伪注释）
    sql = sql.replace(/'__ST(\d+)__'/g, (_, i) => strs[Number(i)]);

    // Step 1: 抽离 CREATE TABLE 内部的 UNIQUE KEY/KEY 定义，后置为独立 CREATE INDEX
    const extraIndexes = [];
    sql = sql.replace(/CREATE\s+TABLE\s+(IF\s+NOT\s+EXISTS\s+)?([`"\w]+)\s*\(([\s\S]*?)\)\s*;/gi,
      (m, ifne, tbl, body) => {
        // 临时保存字符串字面量，避免误伤里面的 --
        const strings = [];
        let tmpBody = body.replace(/'([^'\\]|\\.)*'/g, (s) => {
          strings.push(s); return `'__STR${strings.length-1}__'`;
        });
        // 去掉每行 "-- xxx" 注释（SQLite 行注释）
        tmpBody = tmpBody.replace(/\-\-.*/g, '');
        // 恢复字符串字面量
        tmpBody = tmpBody.replace(/'__STR(\d+)__'/g, (_, i) => strings[Number(i)]);

        // 去掉 UNIQUE KEY name(cols) / UNIQUE KEY `name`(cols)
        tmpBody = tmpBody.replace(/UNIQUE\s+KEY\s+`?([\w]+)`?\s*\(([^)]+)\)\s*,?/gi, (_, name, cols) => {
          extraIndexes.push(`CREATE UNIQUE INDEX IF NOT EXISTS ${name} ON ${tbl} (${cols});`);
          return '';
        });
        // 去掉 KEY name(cols) / KEY `name`(cols)（普通索引）
        tmpBody = tmpBody.replace(/(?<!UNIQUE\s)\bKEY\s+`?([\w]+)`?\s*\(([^)]+)\)\s*,?/gi, (_, name, cols) => {
          extraIndexes.push(`CREATE INDEX IF NOT EXISTS ${name} ON ${tbl} (${cols});`);
          return '';
        });
        // 去掉 CONSTRAINT name UNIQUE(cols) 内嵌定义
        tmpBody = tmpBody.replace(/CONSTRAINT\s+`?([\w]+)`?\s+UNIQUE\s*\(([^)]+)\)\s*,?/gi, (_, name, cols) => {
          extraIndexes.push(`CREATE UNIQUE INDEX IF NOT EXISTS ${name} ON ${tbl} (${cols});`);
          return '';
        });
        // 去掉尾部逗号：反复清理，直到没有
        while (/\,\s*$/.test(tmpBody)) { tmpBody = tmpBody.replace(/\,\s*$/, ''); }
        tmpBody = tmpBody.replace(/,(\s*\))$/, '$1');
        return `CREATE TABLE ${ifne||''}${tbl} (${tmpBody});`;
      });

    // Step 2: 常规类型/关键字映射
    sql = sql
      .replace(/\bDATETIME\(\d+\)/gi, 'TEXT')
      .replace(/\bDATETIME\b/gi, 'TEXT')
      .replace(/\bTIMESTAMP\b/gi, 'TEXT')
      .replace(/\b(MEDIUMTEXT|LONGTEXT|TINYTEXT)\b/gi, 'TEXT')
      .replace(/\bBOOL(EAN)?\b/gi, 'INTEGER')
      .replace(/\bTINYINT\s*\(\d+\)/gi, 'INTEGER')
      .replace(/\bSMALLINT\s*\(\d+\)/gi, 'INTEGER')
      .replace(/\bBIGINT\s*\(\d+\)/gi, 'INTEGER')
      .replace(/\bINT(EGER)?\s*\(\d+\)/gi, 'INTEGER')
      .replace(/\b(DECIMAL|NUMERIC|FLOAT|DOUBLE|REAL)\s*\([^)]*\)/gi, (m, t) => t.toUpperCase())
      .replace(/\bCHAR\s*\(\d+\)/gi, 'TEXT')
      .replace(/\s*ON\s+UPDATE\s+CURRENT_TIMESTAMP\s*\(\d*\)/gi, '')
      .replace(/\s*ON\s+UPDATE\s+CURRENT_TIMESTAMP/gi, '')
      .replace(/\bCURRENT_TIMESTAMP\s*\(\d*\)/gi, 'CURRENT_TIMESTAMP')
      .replace(/\s+COMMENT\s+'[^']*'/gi, '')
      .replace(/\)\s*ENGINE\s*=\s*\w+[^;]*;/gi, ');')
      .replace(/\)\s*DEFAULT\s+CHARSET\s*=\s*\w+[^;]*;/gi, ');')
      .replace(/\)\s*COLLATE\s*=\s*\w+[^;]*;/gi, ');')
      .replace(/\bCREATE\s+(UNIQUE\s+)?INDEX\s+(IF\s+NOT\s+EXISTS\s+)?(\w+)\s+ON\s+(\w+)\s*\(((?:[^()]|\([^()]*\))*)\)\s*;?/gi,
        (m, unique, ifne, name, tbl, cols) => {
          const normCols = cols.replace(/\((\d+)\)/g, '');
          return `CREATE ${unique||''} INDEX ${ifne||''}${name} ON ${tbl} (${normCols});`;
        });

    // Step 3: 追加上抽离出的索引
    sql += '\n' + extraIndexes.join('\n') + '\n';
    return sql;
  }

  _splitStatements(sql) {
    const out = [];
    let depth = 0, cur = '', inStr = false, strCh = '';
    for (let i = 0; i < sql.length; i++) {
      const c = sql[i];
      if (inStr) {
        cur += c;
        if (c === strCh && sql[i-1] !== '\\') inStr = false;
        continue;
      }
      if (c === "'" || c === '"' || c === '`') { inStr = true; strCh = c; cur += c; continue; }
      if (c === '(' || c === '[' || c === '{') depth++;
      if (c === ')' || c === ']' || c === '}') depth--;
      if (c === ';' && depth === 0) {
        const s = cur.trim();
        if (s) out.push(s + ';');
        cur = '';
        continue;
      }
      cur += c;
    }
    const tail = cur.trim();
    if (tail) out.push(tail.endsWith(';') ? tail : tail + ';');
    return out;
  }

  // ============================================================
  // 内部：IAM 种子
  // ============================================================
  _seedIamBuiltins() {
    const tenantId = 'system';
    const now = new Date().toISOString();
    const db = this.db;
    // 系统租户
    db.prepare(`
      INSERT OR IGNORE INTO iam_tenant
      (tenant_id, tenant_code, tenant_name, tenant_mode, tenant_status, tenant_plan,
       created_at, updated_at, version)
      VALUES (?,?,?,?,?,?,?,?,?)
    `).run(tenantId, 'system', 'System Tenant', 'logical', 'active', 'ultimate', now, now, 1);

    // 内置角色
    const insRole = db.prepare(`
      INSERT OR IGNORE INTO iam_role
      (role_id, tenant_id, role_code, role_name, role_type, is_builtin, description,
       status, created_at, updated_at, version)
      VALUES (?,?,?,?,?,?,?,?,?,?,?)
    `);
    for (const r of IamSeed.BuiltinRoles) {
      insRole.run(
        uuidv7(), tenantId, r.role_code, r.role_name, r.role_type, r.is_builtin ? 1 : 0,
        r.description || '', 'active', now, now, 1,
      );
    }
    // 内置权限（基于权限矩阵 × 资源 × 动作）
    const permCodes = [];
    const matrix = IamSeed.BuiltinPermissionMatrix;
    const insertPerm = db.prepare(`
      INSERT OR IGNORE INTO iam_permission
      (perm_id, tenant_id, perm_code, resource_id, perm_action, status, created_at)
      VALUES (?,?,?,?,?, 'active', ?)
    `);
    for (const [resCode, actions] of Object.entries(matrix)) {
      for (const act of actions) {
        permCodes.push(`${resCode}:${act}`);
        insertPerm.run(uuidv7(), tenantId, `${resCode}:${act}`, resCode, act, now);
      }
    }
    // sys_admin 绑定全部权限
    const adminRole = db.prepare(
      `SELECT role_id FROM iam_role WHERE tenant_id = ? AND role_code = 'sys_admin' LIMIT 1`
    ).get(tenantId);
    if (adminRole) {
      const insRP = db.prepare(`
        INSERT OR IGNORE INTO iam_role_permission (rp_id, tenant_id, role_id, perm_id, created_at, created_by)
        SELECT ?, ?, ?, perm_id, ?, 'system'
        FROM iam_permission WHERE tenant_id = ? AND perm_code = ?
      `);
      for (const pc of permCodes) {
        insRP.run(uuidv7(), tenantId, adminRole.role_id, now, tenantId, pc);
      }
    }
  }

  // ============================================================
  // 内部：行业包安装
  // ============================================================
  _installIndustryPackages() {
    const toInstall = this.options.installIndustries || ['common'];
    for (const code of toInstall) {
      const pkg = IndustryPackages[code];
      if (!pkg) continue;
      this.industryRegistry.set(code, pkg);
      // 写 meta_industry_package + meta_tenant_industry
      const now = new Date().toISOString();
      this.db.prepare(`
        INSERT OR IGNORE INTO meta_industry_package
        (package_id, package_code, package_name, package_version, description, status, is_official, created_at, updated_at)
        VALUES (?,?,?,?,?, 'active', 1,?,?)
      `).run(uuidv7(), pkg.package_code, pkg.package_name, pkg.package_version || '1.0.0',
             pkg.description || `行业包 ${pkg.package_name}`, now, now);

      // 写实体占位（若 meta_entity 已存在则跳过）
      for (const e of pkg.entities || []) {
        this.db.prepare(`
          INSERT OR IGNORE INTO meta_entity
          (entity_id, tenant_id, entity_code, entity_name, entity_category, storage_mode,
           is_system, status, created_at, updated_at, version)
          VALUES (?, 'system', ?,?,?, 'universal', 0, 'active',?,?,?)
        `).run(uuidv7(), e.code, e.name, e.category || 'master', now, now, 1);
      }
    }
  }

  _registerDemoIndustryModules() {
    const { common, gov, fin, med, mfg, edu, rtl } = createIndustryModules();
    const toInstall = this.options.installIndustries || ['common'];
    const map = { common, gov, finance: fin, medical: med, manufacturing: mfg, education: edu, retail: rtl };
    for (const code of toInstall) {
      const mod = map[code];
      if (mod) this.orchestrator.registerModule(mod);
    }
  }

  // ============================================================
  // 内部：事件桥
  // ============================================================
  _bridgeOrchEvents() {
    const self = this;
    const bus = this.orchestrator.eventBus;
    if (!bus) return;
    for (const ev of Object.values(BizEvent)) {
      bus.on(ev, (p) => process.nextTick(() => self.emit(ev, p)));
    }
  }

  // ============================================================
  // 内置：审计记录器
  // ============================================================
  _createBuiltinAuditLogger() {
    const db = this;
    return {
      write: async (info) => {
        try {
          const ctx = info.ctx || {};
          const tenantId = info.tenantId || ctx.tenantId || 'system';
          // 取前一 hash 形成链式签名
          const prev = (this.db?.prepare?.(
            `SELECT curr_hash FROM audit_log WHERE tenant_id = ? ORDER BY created_at DESC, audit_id DESC LIMIT 1`
          ).get(tenantId));
          const auditId = uuidv7();
          const payload = JSON.stringify({
            actionDomain: info.actionDomain,
            actionModule: info.actionModule,
            actionName:   info.actionName,
            targetId:     info.targetId,
            result:       info.result,
            durationMs:   info.durationMs,
            userId:       info.userId,
            clientIp:     info.clientIp,
          });
          const h = cryptoHash(`${prev?.curr_hash || ''}|${payload}`);
          this.db?.prepare?.(`
            INSERT INTO audit_log
            (audit_id, tenant_id, user_id, action_domain, action_module, action_name, target_type, target_id,
             request_id, trace_id, result, response_time_ms, client_ip, user_agent,
             snapshot_before, snapshot_after, changed_fields, prev_hash, curr_hash, created_at)
            VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
          `).run(
            auditId, tenantId, info.userId || null,
            info.actionDomain || 'system',
            info.actionModule || 'unknown',
            info.actionName   || 'unknown',
            info.targetType   || null,
            info.targetId     || null,
            info.requestId    || ctx.requestId || null,
            info.traceId      || ctx.traceId   || null,
            info.result       || 'success',
            info.durationMs   || 0,
            info.clientIp     || ctx.clientIp   || null,
            info.userAgent    || ctx.userAgent  || null,
            info.snapshotBefore ? JSON.stringify(info.snapshotBefore) : null,
            info.snapshotAfter  ? JSON.stringify(info.snapshotAfter)  : null,
            info.changedFields  ? JSON.stringify(info.changedFields)  : null,
            prev?.curr_hash || null, h, new Date().toISOString(),
          );
        } catch (_) { /* 审计失败必须吞 */ }
      },
    };
  }
}

// ============================================================
// 空实现占位（真实环境应替换）
// ============================================================

class DummyRuleEngine {
  async run({ tenantId, entityCode, action, data, event }) {
    // 返回空结果：不拦截、不计算、不触发
    return { blocked: false, violations: [], calculated: null };
  }
}

class DummyWorkflowEngine {
  async start({ tenantId, userId, workflowCode, bizId, entityCode, formData }) {
    return { workflowInstanceId: uuidv7(), status: 'running', workflowCode };
  }
  async approve() { return { ok: true }; }
  async reject()  { return { ok: true }; }
}

function cryptoHash(s) {
  return require('crypto').createHash('sha256').update(String(s)).digest('hex');
}

// ============================================================
// 导出
// ============================================================

module.exports = {
  // 启动
  EnterpriseBootstrap,
  // 枚举
  IamEnums,
  BizAction,
  PipeStage,
  BizEvent,
  // 核心类
  BizContext,
  Pipeline,
  BizModule,
  BizService,
  Orchestrator,
  DataStore,
  MetaCache,
  FieldSlotAllocator,
  UniversalBizDAO,
  // 工具
  uuidv7,
  // 数据库定义
  IamDDL,
  IamSeed,
  MetaDDL,
  IndustryPackages,
  createIndustryModules,
};

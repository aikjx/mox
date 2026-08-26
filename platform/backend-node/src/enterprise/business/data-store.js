/**
 * ============================================================
 *  璇玑 RelGraph · 企业级数据存储层实现 (DataStore)
 * ============================================================
 *
 *  功能：为 Orchestrator 提供统一的持久化门面
 *  存储策略：
 *    1. universal 模式：绝大多数实体存入 biz_data 通用表
 *       - 字段映射通过 meta_field.field_storage 配置
 *       - 常用字段进入 ext_xxx 预留列以支持索引查询
 *       - 其他字段统一进 dynamic_data JSON 列
 *    2. dedicated 模式：特殊实体（如审计、工作流）用独立表
 *    3. 多租户：所有查询自动注入 tenant_id 条件
 *    4. 软删除：deleted_at IS NULL 过滤
 *    5. 乐观锁：version 字段
 *    6. 原子写：先写 .tmp 再 rename（按 project_memory 硬约束）
 *    7. 大列表增量更新：saveList 使用 changeLog 节流合并（按硬约束）
 * ============================================================
 */

'use strict';

const crypto = require('crypto');
const path = require('path');
const fs = require('fs');

// ============================================================
// 一、字段映射工具
// ============================================================

// 预定义的 12 个通用字符串槽位（按用途推荐）
const PREDEF_STR_SLOTS = [
  'ext_str_01', 'ext_str_02', 'ext_str_03', 'ext_str_04',
  'ext_str_05', 'ext_str_06', 'ext_str_07', 'ext_str_08',
];

/**
 * 字段 → 存储槽位 分配器
 * 缓存分配结果，避免每次查询都计算
 */
class FieldSlotAllocator {
  constructor() {
    this._cache = new Map(); // entity_code → Map(field_code, slot_name)
  }

  /**
   * 根据实体所有字段的 meta_field 定义，返回分配映射
   * 策略：
   *  - is_indexed/is_searchable/is_filterable/is_sortable = true 的优先分配到 ext_xxx 列
   *  - string(≤255) → ext_str_xx
   *  - integer/bigint → ext_int_xx
   *  - decimal/money → ext_dec_xx
   *  - date → ext_date_xx
   *  - datetime/timestamp → ext_datetime_xx
   *  - boolean/toggle → ext_bool_xx
   *  - text/rich/json/object/array/relation_multi → dynamic_data JSON
   */
  allocate(entityCode, fields) {
    if (this._cache.has(entityCode)) {
      return this._cache.get(entityCode);
    }
    const map = new Map();
    // 计数各槽位使用量
    const counters = {
      str: 0, text: 0, json: 0,
      int: 0, dec: 0,
      date: 0, datetime: 0, bool: 0,
    };
    const MAX = { str: 12, text: 2, json: 4, int: 5, dec: 5, date: 3, datetime: 4, bool: 4 };
    // 优先分配被索引/需要查询的字段
    const prioritized = [...fields].sort((a, b) => {
      const score = f =>
        (f.is_indexed ? 16 : 0) + (f.is_searchable ? 8 : 0) +
        (f.is_filterable ? 4 : 0) + (f.is_sortable ? 2 : 0) +
        (f.is_required ? 1 : 0);
      return score(b) - score(a);
    });
    for (const f of prioritized) {
      const slot = this._tryAlloc(f, counters, MAX);
      if (slot) map.set(f.field_code, slot);
    }
    this._cache.set(entityCode, map);
    return map;
  }

  _tryAlloc(field, counters, MAX) {
    const t = field.field_type;
    const cat = this._category(t);
    if (!cat) return null; // 无法分配到预定义列 → 走 dynamic_data
    if (counters[cat] >= MAX[cat]) return null;
    const idx = ++counters[cat];
    const slot = ({
      str:      i => `ext_str_${String(i).padStart(2, '0')}`,
      text:     i => `ext_text_0${i}`,
      json:     i => `ext_json_0${i}`,
      int:      i => `ext_int_0${i}`,
      dec:      i => `ext_dec_0${i}`,
      date:     i => `ext_date_0${i}`,
      datetime: i => `ext_datetime_0${i}`,
      bool:     i => `ext_bool_0${i}`,
    })[cat](idx);
    return slot ? { slot, category: cat } : null;
  }

  _category(fieldType) {
    switch (fieldType) {
      case 'string': case 'keyword': case 'phone': case 'email':
      case 'url': case 'id_card': case 'bank_card': case 'domain': case 'ip':
      case 'enum': case 'user': case 'dept': case 'tenant':
      case 'barcode': case 'qrcode': case 'auto_increment':
        return 'str';
      case 'text': case 'rich_text': case 'html': case 'markdown':
      case 'signature': case 'address': case 'location':
        return 'text';
      case 'json': case 'json_array': case 'object': case 'array':
      case 'map': case 'relation': case 'relation_multi':
      case 'lookup': case 'reference': case 'parent_child':
      case 'file': case 'files': case 'image': case 'images':
      case 'video': case 'audio': case 'avatar':
        return 'json';
      case 'integer': case 'bigint':
      case 'rating': case 'stars':
        return 'int';
      case 'decimal': case 'float': case 'double':
      case 'percentage': case 'money':
        return 'dec';
      case 'date': case 'daterange':
        return 'date';
      case 'time': case 'datetime': case 'timestamp': case 'timerange':
        return 'datetime';
      case 'boolean': case 'toggle':
        return 'bool';
      default:
        return null; // formula/aggregate 等计算字段不存储
    }
  }
}

// ============================================================
// 二、通用业务数据 DAO
// ============================================================

class UniversalBizDAO {
  constructor(db, slotAllocator) {
    this.db = db;
    this.slotAllocator = slotAllocator || new FieldSlotAllocator();
  }

  /**
   * 将 JSON 格式的动态字段数据，拆成预定义列 + dynamic_data
   */
  flatten(entityCode, fields, data) {
    const slotMap = this.slotAllocator.allocate(entityCode, fields);
    const row = { dynamic_data: {} };
    for (const f of fields) {
      if (!(f.field_code in data)) continue;
      const val = data[f.field_code];
      const alloc = slotMap.get(f.field_code);
      if (alloc) {
        row[alloc.slot] = this._castForStore(val, f.field_type, alloc.category);
      } else {
        row.dynamic_data[f.field_code] = val;
      }
    }
    return row;
  }

  /**
   * 将数据库行还原为业务对象
   */
  unflatten(entityCode, fields, row) {
    if (!row) return null;
    const slotMap = this.slotAllocator.allocate(entityCode, fields);
    const out = {};
    // 系统字段
    for (const sysf of ['biz_id','tenant_id','entity_id','biz_code','parent_biz_id',
      'biz_level','biz_path','biz_status','workflow_status','workflow_inst_id',
      'owner_user_id','owner_dept_id','assignee_user_id','collaborator_user_ids',
      'version','created_at','updated_at','created_by','updated_by','deleted_at']) {
      if (sysf in row) out[sysf] = row[sysf];
    }
    // 预定义列字段
    for (const f of fields) {
      const alloc = slotMap.get(f.field_code);
      if (alloc && row[alloc.slot] !== undefined && row[alloc.slot] !== null) {
        out[f.field_code] = this._castFromStore(row[alloc.slot], f.field_type);
      }
    }
    // dynamic_data
    const dyn = typeof row.dynamic_data === 'string' ? JSON.parse(row.dynamic_data) : (row.dynamic_data || {});
    for (const f of fields) {
      if (!(f.field_code in out) && (f.field_code in dyn)) {
        out[f.field_code] = dyn[f.field_code];
      }
    }
    return out;
  }

  _castForStore(val, fieldType, cat) {
    if (val === null || val === undefined) return null;
    switch (cat) {
      case 'json':
        if (typeof val === 'string') return val;
        return JSON.stringify(val);
      case 'date':
        if (!val) return null;
        if (val instanceof Date) return val.toISOString().slice(0, 10);
        return String(val).slice(0, 10);
      case 'datetime':
        if (!val) return null;
        if (val instanceof Date) return val;
        return new Date(val);
      case 'bool':
        return val ? 1 : 0;
      case 'int': case 'dec':
        if (val === '') return null;
        return Number(val);
      default: // str/text
        if (typeof val === 'object') return JSON.stringify(val);
        return String(val);
    }
  }

  _castFromStore(raw, fieldType) {
    if (raw === null || raw === undefined) return raw;
    switch (fieldType) {
      case 'boolean': case 'toggle':
        return !!raw;
      case 'json': case 'json_array': case 'object': case 'array':
      case 'map': case 'relation_multi': case 'files': case 'images':
      case 'options_inline':
        if (typeof raw === 'string') { try { return JSON.parse(raw); } catch { return raw; } }
        return raw;
      case 'integer': case 'bigint':
      case 'rating': case 'stars':
        return Number.isInteger(Number(raw)) ? Number(raw) : raw;
      case 'decimal': case 'float': case 'double':
      case 'percentage': case 'money':
        return Number(raw);
      default:
        return raw;
    }
  }
}

// ============================================================
// 三、DataStore 实现 —— 对接 Orchestrator
// ============================================================

class DataStore {
  constructor({ db, metaCache, auditLogger }) {
    this.db = db;                       // better-sqlite3 实例（或兼容接口）
    this.metaCache = metaCache || null; // 元数据缓存：{ getFields(entityCode), getEntity(code) }
    this.auditLogger = auditLogger;
    this.dao = new UniversalBizDAO(db);
    this.largeListThreshold = 5000;     // 硬约束阈值：大列表不做全量重写
  }

  // ──────────────────────────────────────────────────────
  // 权限相关
  // ──────────────────────────────────────────────────────

  async checkPermission({ tenantId, userId, roles, permission, entityCode }) {
    // 管理员免鉴权
    if (roles?.includes('admin') || roles?.includes('tenant_admin') || roles?.includes('sys_admin')) {
      return true;
    }
    // 查 iam_permission + iam_role_permission + iam_user_role
    const sql = `
      SELECT COUNT(*) AS cnt FROM iam_permission p
      INNER JOIN iam_role_permission rp ON rp.perm_id = p.perm_id AND rp.tenant_id = p.tenant_id
      INNER JOIN iam_user_role ur        ON ur.role_id = rp.role_id   AND ur.tenant_id = rp.tenant_id
      WHERE p.tenant_id = ?
        AND ur.user_id = ?
        AND p.perm_code = ?
        AND (ur.effective_from IS NULL OR ur.effective_from <= CURRENT_TIMESTAMP)
        AND (ur.effective_to   IS NULL OR ur.effective_to   >= CURRENT_TIMESTAMP)
    `;
    const row = this.db.prepare(sql).get(tenantId, userId, permission);
    return row && row.cnt > 0;
  }

  async resolveDataScope({ tenantId, userId, roles, entityCode }) {
    // 按用户角色取最宽的数据权限策略
    const rows = this.db.prepare(`
      SELECT dp.scope_type, dp.filter_expr, dp.filter_params, dp.priority
      FROM iam_data_permission dp
      LEFT JOIN iam_user_role ur ON ur.role_id = dp.role_id AND ur.tenant_id = dp.tenant_id
      WHERE dp.tenant_id = ? AND dp.target_entity IN (?, '*') AND dp.status = 'active'
        AND (ur.user_id = ? OR dp.role_id IS NULL)
      ORDER BY dp.priority DESC
    `).all(tenantId, entityCode, userId);
    if (!rows.length) return null;
    // 取最高优先级一条
    const top = rows[0];
    const scope = { type: top.scope_type, filterExpr: top.filter_expr, params: top.filter_params || {} };
    // 填充变量
    if (scope.filterExpr && userId) {
      scope.filterExpr = scope.filterExpr.replace(/\{\{user_id\}\}/g, JSON.stringify(userId));
    }
    return scope;
  }

  // ──────────────────────────────────────────────────────
  // 实体字段校验
  // ──────────────────────────────────────────────────────

  async validateEntity({ tenantId, entityCode, data, action }) {
    const fields = await this._getFields(tenantId, entityCode);
    const errors = [];
    for (const f of fields) {
      if (f.is_system) continue;
      const v = data[f.field_code];
      // 必填
      if (action === 'create' && f.is_required && (v === undefined || v === null || v === '')) {
        errors.push({ field: f.field_code, type: 'required', message: `${f.field_name} 为必填项` });
        continue;
      }
      if (v === undefined || v === null || v === '') continue;
      // 长度
      if (f.max_length && String(v).length > f.max_length) {
        errors.push({ field: f.field_code, type: 'max_length', message: `${f.field_name} 超过最大长度 ${f.max_length}` });
      }
      // 范围
      if (f.min_value !== undefined && f.min_value !== null && Number(v) < Number(f.min_value)) {
        errors.push({ field: f.field_code, type: 'min', message: `${f.field_name} 不能小于 ${f.min_value}` });
      }
      if (f.max_value !== undefined && f.max_value !== null && Number(v) > Number(f.max_value)) {
        errors.push({ field: f.field_code, type: 'max', message: `${f.field_name} 不能大于 ${f.max_value}` });
      }
      // 枚举值校验
      if (f.options_source === 'inline' && f.options_inline && Array.isArray(f.options_inline)) {
        const vals = Array.isArray(v) ? v : [v];
        const allowed = new Set(f.options_inline.map(o => String(o.value)));
        for (const val of vals) {
          if (val !== null && val !== undefined && val !== '' && !allowed.has(String(val))) {
            errors.push({ field: f.field_code, type: 'enum', message: `${f.field_name} 非法枚举值: ${val}` });
          }
        }
      }
      // 自定义正则
      if (f.validations && Array.isArray(f.validations)) {
        for (const rule of f.validations) {
          if (rule.type === 'regex' && rule.pattern) {
            try {
              const re = new RegExp(rule.pattern);
              if (!re.test(String(v))) {
                errors.push({ field: f.field_code, type: 'regex', message: rule.message || `${f.field_name} 格式错误` });
              }
            } catch (_) { /* 坏正则忽略 */ }
          }
        }
      }
    }
    return errors.length ? errors : null;
  }

  // ──────────────────────────────────────────────────────
  // 核心 CRUD
  // ──────────────────────────────────────────────────────

  async create({ tenantId, entityCode, data, userId, tx }) {
    const entity = await this._getEntity(tenantId, entityCode);
    const fields = await this._getFields(tenantId, entityCode);
    const conn = tx || this.db;

    // 1. 生成 biz_id(UUIDv7) / biz_code(按编号规则)
    const bizId = data.biz_id || uuidv7();
    let bizCode = data.biz_code;
    if (!bizCode) {
      try { bizCode = await this._generateBizCode(tenantId, entity, userId); } catch (_) { bizCode = bizId; }
    }

    // 2. flatten 数据
    const flat = this.dao.flatten(entityCode, fields, data || {});

    // 3. 默认归属
    const ownerUserId = data.owner_user_id || userId;
    const ownerDeptId  = data.owner_dept_id  || (await this._userDept(tenantId, userId));

    const version = 1;
    const now = new Date().toISOString();
    const row = {
      biz_id: bizId,
      tenant_id: tenantId,
      entity_id: entity?.entity_id || entityCode,
      biz_code: bizCode,
      parent_biz_id: data.parent_biz_id || null,
      biz_level: data.biz_level ?? 0,
      biz_path:  data.biz_path || null,
      biz_status: data.biz_status || 'draft',
      workflow_status: data.workflow_status || 'none',
      owner_user_id: ownerUserId,
      owner_dept_id: ownerDeptId,
      assignee_user_id: data.assignee_user_id || null,
      collaborator_user_ids: data.collaborator_user_ids ? JSON.stringify(data.collaborator_user_ids) : null,
      version,
      created_at: now,
      updated_at: now,
      created_by: userId,
      updated_by: userId,
      ...flat,
      dynamic_data: flat.dynamic_data && Object.keys(flat.dynamic_data).length
        ? JSON.stringify(flat.dynamic_data)
        : null,
    };

    // 4. 计算行Hash (SHA-256)
    row._hash = this._rowHash(row);

    // 5. 插入
    const cols = Object.keys(row);
    const placeholders = cols.map(() => '?').join(',');
    const sql = `INSERT INTO biz_data (${cols.join(',')}) VALUES (${placeholders})`;
    const stmt = conn.prepare(sql);
    stmt.run(...cols.map(c => (row[c] === undefined ? null : row[c])));

    // 6. 版本历史快照
    this._appendVersion(tenantId, bizId, entity?.entity_id || entityCode, {
      version,
      changeType: 'create',
      after: { ...data, biz_id: bizId, biz_code: bizCode },
      userId,
    });

    return this.get({ tenantId, entityCode, id: bizId, tx: conn });
  }

  async batchCreate({ tenantId, entityCode, items, userId, tx }) {
    if (!Array.isArray(items)) throw new Error('batchCreate: items must be array');
    const results = [];
    for (const item of items) {
      results.push(await this.create({ tenantId, entityCode, data: item, userId, tx }));
    }
    return { created: results.length, items: results };
  }

  async get({ tenantId, entityCode, id, options, scope, tx }) {
    const conn = tx || this.db;
    const scopeWhere = this._scopeToWhere(scope);
    const sql = `SELECT * FROM biz_data
      WHERE tenant_id = ? AND (entity_id = ? OR entity_id = ?) AND biz_id = ?
      AND deleted_at IS NULL ${scopeWhere ? 'AND ' + scopeWhere : ''}
      LIMIT 1`;
    const [id1, id2] = this._entityMatches(tenantId, entityCode);
    const row = conn.prepare(sql).get(tenantId, id1, id2, id);
    if (!row) return null;
    const fields = await this._getFields(tenantId, entityCode);
    return this.dao.unflatten(entityCode, fields, row);
  }

  async list({ tenantId, entityCode, params, scope, tx }) {
    const conn = tx || this.db;
    const fields = await this._getFields(tenantId, entityCode);
    const slotMap = this.dao.slotAllocator.allocate(entityCode, fields);

    // 1. 构建WHERE
    const where = [];
    const values = [];
    where.push('tenant_id = ?'); values.push(tenantId);
    const [id1, id2] = this._entityMatches(tenantId, entityCode);
    where.push('(entity_id = ? OR entity_id = ?)'); values.push(id1); values.push(id2);
    where.push('deleted_at IS NULL');

    // 数据权限
    const scopeWhere = this._scopeToWhere(scope);
    if (scopeWhere) where.push(scopeWhere);

    // 搜索（简单实现：动态字段需要 JSON 函数）
    if (params.search) {
      const like = `%${params.search}%`;
      where.push(`(
        biz_code LIKE ? OR ext_str_01 LIKE ? OR ext_str_02 LIKE ? OR ext_str_03 LIKE ? OR
        ext_str_04 LIKE ? OR ext_str_05 LIKE ? OR ext_str_06 LIKE ? OR ext_str_07 LIKE ? OR
        ext_str_08 LIKE ? OR ext_str_09 LIKE ? OR ext_str_10 LIKE ?
      )`);
      for (let i = 0; i < 11; i++) values.push(like);
    }

    // 条件过滤（按字段映射到对应列）
    if (params.filters && typeof params.filters === 'object') {
      for (const [fieldCode, cond] of Object.entries(params.filters)) {
        const alloc = slotMap.get(fieldCode);
        const col = alloc
          ? alloc.slot
          : `JSON_EXTRACT(dynamic_data, '$."${fieldCode}"')`;
        this._applyFilter(where, values, col, cond);
      }
    }

    // 2. 排序
    const orderParts = [];
    const sorts = Array.isArray(params.sorts) ? params.sorts : [];
    if (!sorts.length) sorts.push({ field: 'updated_at', order: 'desc' });
    for (const s of sorts) {
      const alloc = slotMap.get(s.field);
      const col = alloc ? alloc.slot
        : (s.field === 'updated_at' || s.field === 'created_at' || s.field === 'biz_status'
            ? s.field
            : `JSON_EXTRACT(dynamic_data, '$."${s.field}"')`);
      orderParts.push(`${col} ${/^desc/i.test(s.order || '') ? 'DESC' : 'ASC'}`);
    }
    const orderBy = orderParts.length ? `ORDER BY ${orderParts.join(',')}` : '';

    // 3. 计数（去分页前）
    const whereSql = where.length ? 'WHERE ' + where.join(' AND ') : '';
    const countSql = `SELECT COUNT(*) AS cnt FROM biz_data ${whereSql}`;
    const total = conn.prepare(countSql).get(...values).cnt;

    // 4. 分页
    const limit = Math.max(1, Math.min(1000, Number(params.pageSize) || 20));
    const page  = Math.max(1, Number(params.page) || 1);
    const offset = (page - 1) * limit;
    const pageSql = `SELECT * FROM biz_data ${whereSql} ${orderBy} LIMIT ? OFFSET ?`;
    const rows = conn.prepare(pageSql).all(...values, limit, offset);

    // 5. 反flatten
    const list = rows.map(r => this.dao.unflatten(entityCode, fields, r));

    return {
      list,
      pagination: {
        page,
        pageSize: limit,
        total,
        totalPages: Math.ceil(total / limit),
      },
    };
  }

  async update({ tenantId, entityCode, id, updates, userId, tx }) {
    const conn = tx || this.db;
    const entity = await this._getEntity(tenantId, entityCode);
    const fields = await this._getFields(tenantId, entityCode);
    const current = await this.get({ tenantId, entityCode, id, tx: conn });
    if (!current) {
      const err = new Error('记录不存在');
      err.code = 'E_NOT_FOUND';
      throw err;
    }
    const nextVersion = (current.version || 1) + 1;
    // 乐观锁校验
    if (updates.version && updates.version !== current.version) {
      const err = new Error('版本冲突，数据已被他人修改');
      err.code = 'E_VERSION_CONFLICT';
      throw err;
    }
    const flat = this.dao.flatten(entityCode, fields, updates || {});
    const sets = [];
    const vals = [];
    for (const [k, v] of Object.entries(flat)) {
      if (k === 'dynamic_data') {
        // 增量合并 dynamic_data 而非覆盖
        const merged = { ...(current.dynamic_data || {}), ...(v || {}) };
        sets.push('dynamic_data = ?');
        vals.push(Object.keys(merged).length ? JSON.stringify(merged) : null);
      } else if (v !== undefined) {
        sets.push(`${k} = ?`);
        vals.push(v);
      }
    }
    // 系统字段
    sets.push('version = ?'); vals.push(nextVersion);
    sets.push('updated_at = ?'); vals.push(new Date().toISOString());
    sets.push('updated_by = ?'); vals.push(userId);
    sets.push('_hash = ?'); vals.push(''); // 下面再算，先占位

    vals.push(tenantId);
    vals.push(id);

    // 算新 hash：简单起见重查
    const updateSql = `UPDATE biz_data SET ${sets.join(',')}
      WHERE tenant_id = ? AND biz_id = ? AND deleted_at IS NULL`;
    conn.prepare(updateSql).run(...vals);

    // 更新 hash
    const updatedRow = conn.prepare('SELECT * FROM biz_data WHERE biz_id = ?').get(id);
    if (updatedRow) {
      const hash = this._rowHash(updatedRow);
      conn.prepare('UPDATE biz_data SET _hash = ? WHERE biz_id = ?').run(hash, id);
    }

    // 版本历史
    const changedFields = this._diffKeys(current, { ...current, ...updates });
    this._appendVersion(tenantId, id, entity?.entity_id || entityCode, {
      version: nextVersion,
      changeType: 'update',
      before: current,
      after: { ...current, ...updates, version: nextVersion },
      changedFields,
      userId,
    });

    return this.get({ tenantId, entityCode, id, tx: conn });
  }

  async batchUpdate({ tenantId, entityCode, filters, updates, scope, userId, tx }) {
    const ids = await this._listIdsByFilters({ tenantId, entityCode, filters, scope, tx });
    let success = 0, failed = 0;
    for (const id of ids) {
      try { await this.update({ tenantId, entityCode, id, updates, userId, tx }); success++; }
      catch { failed++; }
    }
    return { matched: ids.length, success, failed };
  }

  async delete({ tenantId, entityCode, id, userId, soft = true, tx }) {
    const conn = tx || this.db;
    const before = await this.get({ tenantId, entityCode, id, tx: conn });
    if (!before) return { deleted: 0, id };
    if (soft) {
      conn.prepare(`UPDATE biz_data
        SET deleted_at = ?, deleted_by = ?, updated_at = ?, updated_by = ?,
            biz_status = 'archived'
        WHERE tenant_id = ? AND biz_id = ? AND deleted_at IS NULL`
      ).run(new Date().toISOString(), userId, new Date().toISOString(), userId, tenantId, id);
    } else {
      conn.prepare(`DELETE FROM biz_data WHERE tenant_id = ? AND biz_id = ?`)
        .run(tenantId, id);
    }
    // 版本历史
    this._appendVersion(tenantId, id, before.entity_id || entityCode, {
      version: (before.version || 1) + 1,
      changeType: soft ? 'delete' : 'delete_permanent',
      before,
      userId,
    });
    return { deleted: 1, id, soft };
  }

  async batchDelete({ tenantId, entityCode, filters, scope, userId, tx }) {
    const ids = await this._listIdsByFilters({ tenantId, entityCode, filters, scope, tx });
    let n = 0;
    for (const id of ids) {
      await this.delete({ tenantId, entityCode, id, userId, tx });
      n++;
    }
    return { deleted: n, count: ids.length };
  }

  async upsert({ tenantId, entityCode, uniqueBy, data, userId, tx }) {
    const conn = tx || this.db;
    const existing = await this._findByUnique({ tenantId, entityCode, uniqueBy, tx: conn });
    if (existing) {
      return this.update({ tenantId, entityCode, id: existing.biz_id, updates: data, userId, tx: conn });
    }
    return this.create({ tenantId, entityCode, data: { ...uniqueBy, ...data }, userId, tx: conn });
  }

  async count({ tenantId, entityCode, params, scope, tx }) {
    const listResult = await this.list({ tenantId, entityCode, params: { ...params, page: 1, pageSize: 1 }, scope, tx });
    return listResult.pagination.total;
  }

  async export({ tenantId, entityCode, params, scope, userId }) {
    // 全量导出，分页拉取
    const pageSize = 1000;
    let page = 1;
    const rows = [];
    while (true) {
      const res = await this.list({
        tenantId, entityCode, params: { ...params, page, pageSize }, scope,
      });
      rows.push(...res.list);
      if (res.list.length < pageSize) break;
      page++;
    }
    return { format: params.format || 'json', total: rows.length, rows };
  }

  async import({ tenantId, entityCode, rows, params, userId, tx }) {
    if (!Array.isArray(rows)) throw new Error('import: rows must be array');
    // 大列表(>5000)：硬约束要求增量变更日志+节流合并，不做全量重写
    if (rows.length > this.largeListThreshold && !params?.forceFullRewrite) {
      return await this._largeListImportIncremental({ tenantId, entityCode, rows, params, userId, tx });
    }
    let created = 0, updated = 0, failed = 0;
    const errors = [];
    for (let i = 0; i < rows.length; i++) {
      const r = rows[i];
      try {
        const uniqueBy = params?.uniqueBy ? this._pick(r, params.uniqueBy) : null;
        if (uniqueBy && Object.keys(uniqueBy).length) {
          const ex = await this._findByUnique({ tenantId, entityCode, uniqueBy, tx });
          if (ex) {
            await this.update({ tenantId, entityCode, id: ex.biz_id, updates: r, userId, tx });
            updated++;
            continue;
          }
        }
        await this.create({ tenantId, entityCode, data: r, userId, tx });
        created++;
      } catch (err) {
        failed++;
        errors.push({ row: i, error: err.message, data: r });
      }
    }
    return { total: rows.length, created, updated, failed, errors: errors.slice(0, 100) };
  }

  async enrich({ tenantId, entityCode, data, action }) {
    if (!data) return data;
    const fields = await this._getFields(tenantId, entityCode);
    const fieldMap = new Map(fields.map(f => [f.field_code, f]));
    // 单条 or 批量
    const arr = Array.isArray(data) ? data : (data.list ? data.list : [data]);
    for (const item of arr) {
      if (!item || typeof item !== 'object') continue;
      for (const [k, v] of Object.entries(item)) {
        const f = fieldMap.get(k);
        if (!f) continue;
        // 字典翻译
        if (f.options_source === 'inline' && f.options_inline) {
          const opt = f.options_inline.find(o => String(o.value) === String(v));
          if (opt) {
            item[`${k}__label`] = opt.label;
            if (opt.color) item[`${k}__color`] = opt.color;
          }
        }
      }
      // 审计字段翻译：created_by/updated_by → 用户昵称
      for (const who of ['created_by', 'updated_by', 'owner_user_id', 'assignee_user_id']) {
        if (item[who]) {
          const u = await this._quickUser(tenantId, item[who]);
          if (u) item[`${who}__name`] = u;
        }
      }
    }
    return data;
  }

  // ──────────────────────────────────────────────────────
  // 事务支持（SQLite / better-sqlite3 同步事务）
  // ──────────────────────────────────────────────────────

  /**
   * 支持 async 回调的事务包裹：
   * better-sqlite3 的原生 db.transaction 只能接受 sync 函数，
   * 而 Pipeline 阶段是 async/await（next()、审计、规则引擎都是 async），
   * 所以这里用 "BEGIN IMMEDIATE → await fn() → COMMIT / ROLLBACK" 的手动模式。
   * 对嵌套调用使用 SAVEPOINT，避免 SQLite 对嵌套 BEGIN 的报错。
   */
  async withTransaction(fn) {
    const db = this.db;
    const depthKey = '__txDepth';
    db[depthKey] = (db[depthKey] || 0) + 1;
    const depth = db[depthKey];
    try {
      if (depth === 1) {
        db.exec('BEGIN IMMEDIATE');
      } else {
        db.exec(`SAVEPOINT biz_tx_sp_${depth}`);
      }
      const result = await fn(db);
      if (depth === 1) {
        db.exec('COMMIT');
      } else {
        db.exec(`RELEASE SAVEPOINT biz_tx_sp_${depth}`);
      }
      return result;
    } catch (e) {
      try {
        if (depth === 1) db.exec('ROLLBACK');
        else             db.exec(`ROLLBACK TO SAVEPOINT biz_tx_sp_${depth}`);
      } catch (_) { /* ignore rollback errors */ }
      throw e;
    } finally {
      db[depthKey] = Math.max(0, db[depthKey] - 1);
    }
  }

  // ──────────────────────────────────────────────────────
  // 内部：元数据解析
  // ──────────────────────────────────────────────────────

  async _getEntity(tenantId, entityCode) {
    if (this.metaCache?.getEntity) {
      return this.metaCache.getEntity(tenantId, entityCode);
    }
    const row = this.db.prepare(
      `SELECT * FROM meta_entity WHERE tenant_id IN (?, 'system') AND entity_code = ? AND status = 'active' LIMIT 1`
    ).get(tenantId, entityCode);
    return row || { entity_id: entityCode, entity_code: entityCode, entity_name: entityCode };
  }

  _entityIdCache(entityCode) {
    return entityCode; // 简化：entity_id 允许存 code 或 uuid，这里 OR 兼容
  }

  /**
   * 统一 biz_data.entity_id 的匹配主键：
   * - 如果 meta_entity 里存在该 code，返回它的真实 entity_id(uuid) + entity_code；
   * - 否则返回 [entityCode, entityCode] 兜底。
   * 因为写入 DataStore.create 时 entity?.entity_id（可能是 UUID）/ entityCode 两种模式都存在历史数据，
   * 所以 SQL 里永远用 OR (entity_id = uuidMatch OR entity_id = entityCode) 兼容。
   */
  _entityMatches(tenantId, entityCode) {
    let eid = entityCode;
    if (this.metaCache?.getEntity) {
      const e = this.metaCache.getEntity(tenantId, entityCode);
      if (e?.entity_id) eid = e.entity_id;
    }
    return [eid, entityCode];
  }

  async _getFields(tenantId, entityCode) {
    if (this.metaCache?.getFields) {
      return this.metaCache.getFields(tenantId, entityCode);
    }
    const entity = await this._getEntity(tenantId, entityCode);
    const rows = this.db.prepare(`
      SELECT * FROM meta_field
      WHERE tenant_id IN (?, 'system') AND entity_id = ? AND status = 'active'
      ORDER BY ui_sort_order ASC, field_code ASC
    `).all(tenantId, entity.entity_id);
    return rows.map(r => ({
      ...r,
      options_inline: r.options_inline ? (typeof r.options_inline === 'string' ? JSON.parse(r.options_inline) : r.options_inline) : null,
      validations:   r.validations   ? (typeof r.validations   === 'string' ? JSON.parse(r.validations)   : r.validations)   : null,
    }));
  }

  // ──────────────────────────────────────────────────────
  // 内部：过滤条件 / 数据权限
  // ──────────────────────────────────────────────────────

  _applyFilter(where, values, col, cond) {
    // cond 格式:
    //   直接值 → =
    //   { op:'in', values:[] }
    //   { op:'between', from, to }
    //   { op:'gt' | 'gte' | 'lt' | 'lte' | 'ne' | 'like' | 'contains' | 'starts_with' | 'ends_with', value }
    //   { op:'null' } / { op:'notNull' }
    if (cond === null || cond === undefined) {
      where.push(`${col} IS NULL`);
      return;
    }
    if (typeof cond !== 'object' || !('op' in cond)) {
      where.push(`${col} = ?`); values.push(cond);
      return;
    }
    switch (cond.op) {
      case 'in':
        if (!Array.isArray(cond.values) || !cond.values.length) { where.push('1=0'); return; }
        where.push(`${col} IN (${cond.values.map(()=>'?').join(',')})`);
        values.push(...cond.values);
        break;
      case 'between':
        where.push(`${col} BETWEEN ? AND ?`); values.push(cond.from, cond.to);
        break;
      case 'gt':  where.push(`${col} > ?`);  values.push(cond.value); break;
      case 'gte': where.push(`${col} >= ?`); values.push(cond.value); break;
      case 'lt':  where.push(`${col} < ?`);  values.push(cond.value); break;
      case 'lte': where.push(`${col} <= ?`); values.push(cond.value); break;
      case 'ne':  where.push(`${col} != ?`); values.push(cond.value); break;
      case 'like':
      case 'contains':
        where.push(`${col} LIKE ?`); values.push(`%${cond.value}%`); break;
      case 'starts_with':
        where.push(`${col} LIKE ?`); values.push(`${cond.value}%`); break;
      case 'ends_with':
        where.push(`${col} LIKE ?`); values.push(`%${cond.value}`); break;
      case 'null':    where.push(`${col} IS NULL`); break;
      case 'notNull': where.push(`${col} IS NOT NULL`); break;
      default:
        where.push(`${col} = ?`); values.push(cond.value);
    }
  }

  _scopeToWhere(scope) {
    if (!scope) return null;
    switch (scope.type) {
      case 'all':    return null;
      case 'self':   return `owner_user_id = ${this._q(scope.params?.userId || 'NULL_USER')}`;
      case 'dept':
        return scope.params?.deptId ? `owner_dept_id = ${this._q(scope.params.deptId)}` : null;
      case 'dept_and_sub':
        return scope.params?.deptPath ? `owner_dept_id IN (SELECT dept_id FROM iam_department WHERE dept_path LIKE ${this._q(scope.params.deptPath + '%')})` : null;
      case 'custom':
        return scope.filterExpr || null;
      default: return null;
    }
  }

  _q(v) { return typeof v === 'string' ? `'${v.replace(/'/g, "''")}'` : String(v); }

  // ──────────────────────────────────────────────────────
  // 内部：辅助
  // ──────────────────────────────────────────────────────

  _rowHash(row) {
    const exclude = new Set(['_hash', 'version', 'updated_at', 'updated_by']);
    const entries = Object.entries(row)
      .filter(([k]) => !exclude.has(k))
      .sort(([a],[b]) => a.localeCompare(b));
    const s = JSON.stringify(entries);
    return crypto.createHash('sha256').update(s).digest('hex');
  }

  _diffKeys(a, b) {
    if (!a || !b) return null;
    const all = new Set([...Object.keys(a), ...Object.keys(b)]);
    const out = [];
    for (const k of all) {
      if (['version','updated_at','updated_by','_hash'].includes(k)) continue;
      if (JSON.stringify(a[k]) !== JSON.stringify(b[k])) out.push(k);
    }
    return out;
  }

  _pick(obj, keys) {
    const out = {};
    for (const k of keys) if (k in obj) out[k] = obj[k];
    return out;
  }

  async _findByUnique({ tenantId, entityCode, uniqueBy, tx }) {
    const conn = tx || this.db;
    const [id1, id2] = this._entityMatches(tenantId, entityCode);
    const where = ['tenant_id = ?', '(entity_id = ? OR entity_id = ?)', 'deleted_at IS NULL'];
    const vals = [tenantId, id1, id2];
    const fields = await this._getFields(tenantId, entityCode);
    const slotMap = this.dao.slotAllocator.allocate(entityCode, fields);
    for (const [k, v] of Object.entries(uniqueBy)) {
      const alloc = slotMap.get(k);
      const col = alloc ? alloc.slot : `JSON_EXTRACT(dynamic_data, '$."${k}"')`;
      where.push(`${col} = ?`);
      vals.push(v);
    }
    const sql = `SELECT * FROM biz_data WHERE ${where.join(' AND ')} LIMIT 1`;
    const row = conn.prepare(sql).get(...vals);
    if (!row) return null;
    return this.dao.unflatten(entityCode, fields, row);
  }

  async _listIdsByFilters({ tenantId, entityCode, filters, scope, tx }) {
    const res = await this.list({
      tenantId, entityCode,
      params: { filters, page: 1, pageSize: 100000, fields: ['biz_id'] },
      scope, tx,
    });
    return res.list.map(x => x.biz_id);
  }

  async _userDept(tenantId, userId) {
    if (!userId) return null;
    const row = this.db.prepare(
      `SELECT dept_id FROM iam_user WHERE tenant_id = ? AND user_id = ? AND deleted_at IS NULL LIMIT 1`
    ).get(tenantId, userId);
    return row?.dept_id || null;
  }

  async _quickUser(tenantId, userId) {
    const row = this.db.prepare(
      `SELECT nickname, real_name, username FROM iam_user WHERE tenant_id = ? AND user_id = ? LIMIT 1`
    ).get(tenantId, userId);
    return row ? (row.real_name || row.nickname || row.username) : null;
  }

  async _generateBizCode(tenantId, entity, userId) {
    // 查 meta_auto_number 编号规则
    const number = this.db.prepare(`
      SELECT * FROM meta_auto_number
      WHERE tenant_id IN (?, 'system') AND (entity_id = ? OR number_code = ?) AND status = 'active'
      LIMIT 1
    `).get(tenantId, entity?.entity_id, entity?.entity_code);
    if (!number) return null;

    const now = new Date();
    const vars = {
      YYYY: now.getFullYear(),
      YY:   String(now.getFullYear()).slice(2),
      MM:   String(now.getMonth() + 1).padStart(2, '0'),
      DD:   String(now.getDate()).padStart(2, '0'),
      HH:   String(now.getHours()).padStart(2, '0'),
      mm:   String(now.getMinutes()).padStart(2, '0'),
      ss:   String(now.getSeconds()).padStart(2, '0'),
    };
    // 简单序号（生产用分布式锁）
    const seq = (number.seq_current || 0) + number.seq_step;
    this.db.prepare(`UPDATE meta_auto_number SET seq_current = ?, updated_at = CURRENT_TIMESTAMP WHERE number_id = ?`)
      .run(seq, number.number_id);
    const template = number.format_template;
    let out = template;
    for (const [k, v] of Object.entries(vars)) out = out.split(`{{${k}}}`).join(v);
    // {{SEQ:N}}
    out = out.replace(/\{\{SEQ:(\d+)\}\}/g, (_, n) => String(seq).padStart(Number(n), '0'));
    // {{RAND:N}}
    out = out.replace(/\{\{RAND:(\d+)\}\}/g, (_, n) => crypto.randomBytes(Number(n)).toString('hex').slice(0, Number(n)).toUpperCase());
    return out;
  }

  _appendVersion(tenantId, bizId, entityId, { version, changeType, before, after, changedFields, userId, comment }) {
    const versionId = uuidv7();
    try {
      this.db.prepare(`
        INSERT INTO biz_data_version
        (version_id, tenant_id, biz_id, entity_id, version_number, change_type,
         change_summary, changed_fields, snapshot_before, snapshot_after,
         created_at, created_by, comment)
        VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)
      `).run(
        versionId, tenantId, bizId, entityId, version, changeType,
        (changedFields && changedFields.length) ? `修改字段: ${changedFields.slice(0,5).join(',')}${changedFields.length>5?'+':''}` : null,
        changedFields ? JSON.stringify(changedFields) : null,
        before ? JSON.stringify(before) : null,
        after  ? JSON.stringify(after)  : null,
        new Date().toISOString(), userId, comment || null,
      );
    } catch (_) { /* 版本历史失败不影响主流程 */ }
  }

  /**
   * 大列表(>5000)增量导入：按 uniqueBy 分 upsert，仅写变更
   * 硬约束对齐：禁止全量重写 + delete 大表
   */
  async _largeListImportIncremental({ tenantId, entityCode, rows, params, userId, tx }) {
    const uniqueBy = params?.uniqueBy;
    if (!uniqueBy) {
      // 退化为分批 insert + 幂等日志
      let created = 0, failed = 0;
      const BATCH = 1000;
      for (let i = 0; i < rows.length; i += BATCH) {
        const batch = rows.slice(i, i + BATCH);
        for (const r of batch) {
          try { await this.create({ tenantId, entityCode, data: r, userId, tx }); created++; }
          catch { failed++; }
        }
      }
      return { mode: 'insert_no_unique', total: rows.length, created, updated: 0, failed };
    }
    let created = 0, updated = 0, unchanged = 0, failed = 0;
    const errors = [];
    for (let i = 0; i < rows.length; i++) {
      const r = rows[i];
      try {
        const keys = this._pick(r, uniqueBy);
        const ex = await this._findByUnique({ tenantId, entityCode, uniqueBy: keys, tx });
        if (!ex) {
          await this.create({ tenantId, entityCode, data: r, userId, tx });
          created++;
          continue;
        }
        // 仅当字段真的不同才 update，避免无意义 version + 审计
        const diff = this._diffKeys(ex, r);
        if (!diff || !diff.length) { unchanged++; continue; }
        await this.update({ tenantId, entityCode, id: ex.biz_id, updates: r, userId, tx });
        updated++;
      } catch (err) {
        failed++;
        if (errors.length < 100) errors.push({ row: i, error: err.message });
      }
    }
    return { mode: 'incremental', total: rows.length, created, updated, unchanged, failed, errors };
  }
}

// ============================================================
// 四、UUIDv7 生成器（时间有序，便于分库分表）
// ============================================================

function uuidv7() {
  // 取 48bit 毫秒时间戳 + 80bit 随机
  const buf = crypto.randomBytes(16);
  const now = Date.now();
  // time_high 32bit
  buf.writeUInt32BE(now / 0x10000, 0);
  // time_low 16bit
  buf.writeUInt16BE(now & 0xFFFF, 4);
  // version 0111
  buf[6] = (buf[6] & 0x0F) | 0x70;
  // variant 10
  buf[8] = (buf[8] & 0x3F) | 0x80;
  const h = buf.toString('hex');
  return `${h.slice(0,8)}-${h.slice(8,12)}-${h.slice(12,16)}-${h.slice(16,20)}-${h.slice(20)}`;
}

// ============================================================
// 五、元数据缓存器（减少重复 SQL）
// ============================================================

class MetaCache {
  constructor(db, ttlMs = 60_000) {
    this.db = db;
    this.ttlMs = ttlMs;
    this._ent  = new Map(); // key → { ts, data }
    this._flds = new Map();
  }
  _key(tenantId, code) { return `${tenantId}::${code}`; }
  _get(map, key) {
    const e = map.get(key);
    if (!e) return null;
    if (Date.now() - e.ts > this.ttlMs) { map.delete(key); return null; }
    return e.data;
  }
  _set(map, key, data) { map.set(key, { ts: Date.now(), data }); return data; }

  getEntity(tenantId, entityCode) {
    const k = this._key(tenantId, entityCode);
    return this._get(this._ent, k) || this._set(this._ent, k, (() => {
      const row = this.db.prepare(
        `SELECT * FROM meta_entity WHERE tenant_id IN (?, 'system') AND entity_code = ? AND status='active' LIMIT 1`
      ).get(tenantId, entityCode);
      return row || { entity_id: entityCode, entity_code: entityCode, entity_name: entityCode };
    })());
  }

  getFields(tenantId, entityCode) {
    const k = this._key(tenantId, entityCode);
    return this._get(this._flds, k) || this._set(this._flds, k, (() => {
      const entity = this.getEntity(tenantId, entityCode);
      const rows = this.db.prepare(`
        SELECT * FROM meta_field
        WHERE tenant_id IN (?, 'system') AND entity_id = ? AND status='active'
        ORDER BY ui_sort_order ASC, field_code ASC
      `).all(tenantId, entity.entity_id);
      return rows.map(r => ({
        ...r,
        options_inline: r.options_inline ? (typeof r.options_inline === 'string' ? JSON.parse(r.options_inline) : r.options_inline) : null,
        validations:   r.validations   ? (typeof r.validations   === 'string' ? JSON.parse(r.validations)   : r.validations)   : null,
      }));
    })());
  }

  invalidate(tenantId, entityCode) {
    const k = this._key(tenantId, entityCode);
    this._ent.delete(k); this._flds.delete(k);
  }

  invalidateAll() {
    this._ent.clear(); this._flds.clear();
  }
}

module.exports = {
  DataStore,
  MetaCache,
  FieldSlotAllocator,
  UniversalBizDAO,
  uuidv7,
};

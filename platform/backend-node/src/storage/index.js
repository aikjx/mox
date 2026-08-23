'use strict';

const { config, getStorageConfig, switchProvider, listProviders, DATA_DIR } = require('../config');

class StorageProvider {
  constructor() { this.name = 'base'; }
  connect() { throw new Error('not implemented'); }
  disconnect() {}
  insertEntity(type, id, data) { throw new Error('not implemented'); }
  upsertEntity(type, id, data) { throw new Error('not implemented'); }
  updateEntity(type, id, data) { throw new Error('not implemented'); }
  deleteEntity(id) { throw new Error('not implemented'); }
  deleteByType(type) { throw new Error('not implemented'); }
  getEntity(type, id) { throw new Error('not implemented'); }
  getEntityData(type, id) { throw new Error('not implemented'); }
  listEntities(type) { throw new Error('not implemented'); }
  listAllEntities() { throw new Error('not implemented'); }
  countByType(type) { throw new Error('not implemented'); }
  saveList(type, items, idField) { throw new Error('not implemented'); }
  getList(type) { throw new Error('not implemented'); }
  searchEntities(type, query) { throw new Error('not implemented'); }
  kvGet(key, fallback) { throw new Error('not implemented'); }
  kvSet(key, value) { throw new Error('not implemented'); }
  kvDelete(key) { throw new Error('not implemented'); }
  addLog(type, message, data) { throw new Error('not implemented'); }
  getLogs(type, limit) { throw new Error('not implemented'); }
  clearLogs() { throw new Error('not implemented'); }
  migrateFromJSON(jsonDir) { throw new Error('not implemented'); }
}

class SQLiteProvider extends StorageProvider {
  constructor(dbConfig) {
    super();
    this.name = 'sqlite';
    this.dbConfig = dbConfig;
    this.db = null;
    this.stmts = null;
  }

  connect() {
    const Database = require('better-sqlite3');
    const fs = require('fs');
    const path = require('path');
    const dir = path.dirname(this.dbConfig.path);
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

    this.db = new Database(this.dbConfig.path);
    this.db.pragma('journal_mode = ' + (this.dbConfig.options?.journal_mode || 'WAL'));
    this.db.pragma('foreign_keys = ON');
    this.db.pragma('synchronous = ' + (this.dbConfig.options?.synchronous || 'NORMAL'));

    this.db.exec(`
      CREATE TABLE IF NOT EXISTS entities (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        entity_type TEXT NOT NULL,
        entity_id TEXT NOT NULL UNIQUE,
        data TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
      );
      CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
      CREATE INDEX IF NOT EXISTS idx_entities_id ON entities(entity_id);

      CREATE TABLE IF NOT EXISTS kv_store (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL,
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
      );

      CREATE TABLE IF NOT EXISTS logs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        log_type TEXT,
        message TEXT,
        data TEXT,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
      );
      CREATE INDEX IF NOT EXISTS idx_logs_type ON logs(log_type);
      CREATE INDEX IF NOT EXISTS idx_logs_time ON logs(created_at);
    `);

    this.stmts = {
      insert: this.db.prepare('INSERT INTO entities (entity_type, entity_id, data) VALUES (@entity_type, @entity_id, @data)'),
      upsert: this.db.prepare('INSERT INTO entities (entity_type, entity_id, data) VALUES (@entity_type, @entity_id, @data) ON CONFLICT(entity_id) DO UPDATE SET data = excluded.data, updated_at = datetime(\'now\')'),
      update: this.db.prepare('UPDATE entities SET data = @data, updated_at = datetime(\'now\') WHERE entity_id = @entity_id'),
      delete: this.db.prepare('DELETE FROM entities WHERE entity_id = ?'),
      deleteByType: this.db.prepare('DELETE FROM entities WHERE entity_type = ?'),
      getById: this.db.prepare('SELECT * FROM entities WHERE entity_id = ?'),
      getByType: this.db.prepare('SELECT * FROM entities WHERE entity_type = ? ORDER BY updated_at DESC'),
      listAll: this.db.prepare('SELECT * FROM entities ORDER BY updated_at DESC'),
      countByType: this.db.prepare('SELECT COUNT(*) as cnt FROM entities WHERE entity_type = ?'),
      search: this.db.prepare('SELECT * FROM entities WHERE entity_type = ? AND (data LIKE ? OR entity_id LIKE ?) ORDER BY updated_at DESC'),
      kvGet: this.db.prepare('SELECT * FROM kv_store WHERE key = ?'),
      kvUpsert: this.db.prepare('INSERT INTO kv_store (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime(\'now\')'),
      kvDelete: this.db.prepare('DELETE FROM kv_store WHERE key = ?'),
      insertLog: this.db.prepare('INSERT INTO logs (log_type, message, data) VALUES (?, ?, ?)'),
      getLogs: this.db.prepare('SELECT * FROM logs ORDER BY id DESC LIMIT ?'),
      getLogsByType: this.db.prepare('SELECT * FROM logs WHERE log_type = ? ORDER BY id DESC LIMIT ?'),
      clearLogs: this.db.prepare('DELETE FROM logs')
    };

    console.log(`[storage] SQLite 已连接: ${this.dbConfig.path}`);
  }

  disconnect() {
    if (this.db) { this.db.close(); this.db = null; }
    console.log('[storage] SQLite 已断开');
  }

  _rowToEntity(row) {
    return {
      id: row.entity_id,
      type: row.entity_type,
      data: JSON.parse(row.data),
      created_at: row.created_at,
      updated_at: row.updated_at
    };
  }

  insertEntity(type, id, data) {
    this.stmts.insert.run({ entity_type: type, entity_id: id, data: JSON.stringify(data) });
    return this.getEntity(type, id);
  }

  upsertEntity(type, id, data) {
    this.stmts.upsert.run({ entity_type: type, entity_id: id, data: JSON.stringify(data) });
    return this.getEntity(type, id);
  }

  updateEntity(type, id, data) {
    this.stmts.update.run({ entity_id: id, data: JSON.stringify(data) });
    return this.getEntity(type, id);
  }

  deleteEntity(id) { return this.stmts.delete.run(id); }
  deleteByType(type) { return this.stmts.deleteByType.run(type); }

  getEntity(type, id) {
    const row = this.stmts.getById.get(id);
    return row ? this._rowToEntity(row) : null;
  }

  getEntityData(type, id) {
    const row = this.stmts.getById.get(id);
    return row ? JSON.parse(row.data) : null;
  }

  listEntities(type) {
    return this.stmts.getByType.all(type).map(r => this._rowToEntity(r));
  }

  listAllEntities() {
    return this.stmts.listAll.all().map(r => this._rowToEntity(r));
  }

  countByType(type) {
    const row = this.stmts.countByType.get(type);
    return row.cnt;
  }

  saveList(type, items, idField = 'id') {
    const tx = this.db.transaction((items) => {
      this.stmts.deleteByType.run(type);
      for (const item of items) {
        const id = item[idField] || `${type}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
        this.stmts.insert.run({ entity_type: type, entity_id: String(id), data: JSON.stringify(item) });
      }
    });
    tx(items);
    return items;
  }

  getList(type) {
    return this.listEntities(type).map(e => e.data);
  }

  searchEntities(type, query) {
    const p = `%${query}%`;
    return this.stmts.search.all(type, p, p).map(r => this._rowToEntity(r));
  }

  kvGet(key, fallback = null) {
    const row = this.stmts.kvGet.get(key);
    if (!row) return fallback;
    try { return JSON.parse(row.value); } catch { return row.value; }
  }

  kvSet(key, value) {
    const v = typeof value === 'object' ? JSON.stringify(value) : String(value);
    this.stmts.kvUpsert.run(key, v);
  }

  kvDelete(key) { return this.stmts.kvDelete.run(key); }

  addLog(type, message, data = null) {
    this.stmts.insertLog.run(type, message, data ? JSON.stringify(data) : null);
  }

  getLogs(type = null, limit = 200) {
    const rows = type ? this.stmts.getLogsByType.all(type, limit) : this.stmts.getLogs.all(limit);
    return rows.map(r => ({
      id: r.id, type: r.log_type, message: r.message,
      data: r.data ? JSON.parse(r.data) : null, created_at: r.created_at
    }));
  }

  clearLogs() { this.stmts.clearLogs.run(); }

  migrateFromJSON(jsonDir) {
    const fs = require('fs');
    const path = require('path');
    if (!fs.existsSync(jsonDir)) return 0;

    const jsonFiles = fs.readdirSync(jsonDir).filter(f => f.endsWith('.json'));
    let migrated = 0;

    const tx = this.db.transaction(() => {
      for (const file of jsonFiles) {
        const fp = path.join(jsonDir, file);
        try {
          const raw = fs.readFileSync(fp, 'utf-8');
          if (!raw.trim()) continue;
          const data = JSON.parse(raw);
          const entityType = file.replace(/\.json$/, '');
          // 迁移幂等护栏：表已有数据说明已迁移/由存储层管理，跳过该类型，
          // 防止每次进程启动把 JSON 镜像整份追加进 SQLite 造成行膨胀
          if (this.countByType(entityType) > 0) continue;
          if (Array.isArray(data)) {
            for (const item of data) {
              const id = item.id || `${entityType}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
              this.stmts.upsert.run({ entity_type: entityType, entity_id: String(id), data: JSON.stringify(item) });
              migrated++;
            }
          } else if (typeof data === 'object' && data !== null) {
            const id = data.id || entityType;
            this.stmts.upsert.run({ entity_type: entityType, entity_id: String(id), data: JSON.stringify(data) });
            migrated++;
          }
          console.log(`[storage] 迁移 ${file}: ${Array.isArray(data) ? data.length + ' 条' : '1 条'}`);
        } catch (e) {
          console.warn(`[storage] 迁移 ${file} 失败: ${e.message}`);
        }
      }
    });
    tx();
    console.log(`[storage] 迁移完成: ${migrated} 条记录`);
    return migrated;
  }
}

class MemoryProvider extends StorageProvider {
  constructor() {
    super();
    this.name = 'memory';
    this.entities = new Map();
    this.kv = new Map();
    this.logs = [];
  }

  connect() { console.log('[storage] 内存存储已连接'); }
  disconnect() {}

  _saveEntity(type, id, data) {
    const key = `${type}:${id}`;
    this.entities.set(key, {
      id, type, data,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    });
  }

  insertEntity(type, id, data) { this._saveEntity(type, id, data); return this.getEntity(type, id); }
  upsertEntity(type, id, data) { return this.insertEntity(type, id, data); }
  updateEntity(type, id, data) { return this.insertEntity(type, id, data); }
  deleteEntity(id) {
    for (const [k] of this.entities) { if (k.endsWith(`:${id}`)) { this.entities.delete(k); return; } }
  }
  deleteByType(type) {
    for (const [k] of this.entities) { if (k.startsWith(`${type}:`)) this.entities.delete(k); }
  }

  getEntity(type, id) { return this.entities.get(`${type}:${id}`) || null; }
  getEntityData(type, id) { const e = this.getEntity(type, id); return e ? e.data : null; }

  listEntities(type) {
    return Array.from(this.entities.values())
      .filter(e => e.type === type)
      .sort((a, b) => new Date(b.updated_at) - new Date(a.updated_at));
  }

  listAllEntities() {
    return Array.from(this.entities.values())
      .sort((a, b) => new Date(b.updated_at) - new Date(a.updated_at));
  }

  countByType(type) { return this.listEntities(type).length; }

  saveList(type, items, idField = 'id') {
    this.deleteByType(type);
    for (const item of items) {
      const id = item[idField] || `${type}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
      this._saveEntity(type, id, item);
    }
    return items;
  }

  getList(type) { return this.listEntities(type).map(e => e.data); }

  searchEntities(type, query) {
    const q = query.toLowerCase();
    return this.listEntities(type)
      .filter(e => JSON.stringify(e.data).toLowerCase().includes(q) || e.id.toLowerCase().includes(q));
  }

  kvGet(key, fallback = null) {
    if (!this.kv.has(key)) return fallback;
    try { return JSON.parse(this.kv.get(key)); } catch { return this.kv.get(key); }
  }

  kvSet(key, value) { this.kv.set(key, typeof value === 'object' ? JSON.stringify(value) : String(value)); }
  kvDelete(key) { this.kv.delete(key); }

  addLog(type, message, data = null) {
    this.logs.unshift({ id: Date.now() + Math.random(), type, message, data, created_at: new Date().toISOString() });
    if (this.logs.length > 1000) this.logs.length = 1000;
  }

  getLogs(type = null, limit = 200) {
    const filtered = type ? this.logs.filter(l => l.type === type) : this.logs;
    return filtered.slice(0, limit);
  }

  clearLogs() { this.logs = []; }

  migrateFromJSON(jsonDir) {
    const fs = require('fs');
    const path = require('path');
    if (!fs.existsSync(jsonDir)) return 0;
    const jsonFiles = fs.readdirSync(jsonDir).filter(f => f.endsWith('.json'));
    let migrated = 0;
    for (const file of jsonFiles) {
      try {
        const raw = fs.readFileSync(path.join(jsonDir, file), 'utf-8');
        if (!raw.trim()) continue;
        const data = JSON.parse(raw);
        const entityType = file.replace(/\.json$/, '');
        // 迁移幂等护栏：非空类型跳过（与 SQLiteProvider 一致，防重复追加）
        if (this.countByType(entityType) > 0) continue;
        if (Array.isArray(data)) {
          for (const item of data) {
            const id = item.id || `${entityType}_${Date.now()}`;
            this._saveEntity(entityType, id, item);
            migrated++;
          }
        } else if (typeof data === 'object') {
          const id = data.id || entityType;
          this._saveEntity(entityType, id, data);
          migrated++;
        }
      } catch {}
    }
    console.log(`[storage] 内存迁移: ${migrated} 条`);
    return migrated;
  }
}

class StorageFactory {
  static create(name, cfg) {
    const providers = {
      sqlite: () => new SQLiteProvider(cfg),
      memory: () => new MemoryProvider(cfg),
      mysql: () => { throw new Error('MySQL provider 需要安装 mysql2'); },
      postgresql: () => { throw new Error('PostgreSQL provider 需要安装 pg'); }
    };
    const factory = providers[name];
    if (!factory) throw new Error(`未知的数据库提供商: ${name}`);
    return factory();
  }
}

let _instance = null;

function getStorage() {
  if (_instance && _instance.name === config.storage.provider) return _instance;
  if (_instance) { _instance.disconnect(); _instance = null; }
  const provider = StorageFactory.create(config.storage.provider, getStorageConfig());
  provider.connect();
  if (config.features.autoMigrate) {
    try { provider.migrateFromJSON(DATA_DIR); } catch (e) { console.warn('[storage] 自动迁移失败:', e.message); }
  }
  _instance = provider;
  return provider;
}

function resetStorage() {
  if (_instance) { _instance.disconnect(); _instance = null; }
}

function switchDatabase(providerName) {
  switchProvider(providerName);
  resetStorage();
  return getStorage();
}

module.exports = {
  getStorage,
  resetStorage,
  switchDatabase,
  StorageFactory,
  StorageProvider,
  SQLiteProvider,
  MemoryProvider,
  listProviders: () => listProviders(),
  currentProvider: () => config.storage.provider
};

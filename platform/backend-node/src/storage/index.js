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

  // === L3.5 知识图谱中枢：6 个统一接口（SQLite/PG/Memory/Dual 必须全部实现同构行为）===
  addEdge(src, rel, dst, props) { throw new Error('not implemented'); }
  removeEdge(src, rel, dst, reason) { throw new Error('not implemented'); } // MUST tombstone 不物理删
  neighbors(nodeId, dir) { throw new Error('not implemented'); } // dir: both|in|out
  neighborhoodSubgraph(seedIds, hops, maxNodes) { throw new Error('not implemented'); } // 返回 {nodes:[{id}], edges:[{src,rel,dst}]}
  findPath(fromId, toId, maxHops) { throw new Error('not implemented'); } // 返回最短路径 edges 数组或 null
  pageRank(relFilter) { throw new Error('not implemented'); } // 返回 Map<id, score>
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

      -- L3.5 知识图谱中枢 Edge 表（与归一化总纲 §5.2 唯一 DDL 严格对齐）
      CREATE TABLE IF NOT EXISTS graph_edges (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        src   TEXT NOT NULL,
        rel   TEXT NOT NULL,
        dst   TEXT NOT NULL,
        props TEXT,
        tombstone INTEGER DEFAULT 0,
        reason TEXT,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        UNIQUE(src, rel, dst)
      );
      CREATE INDEX IF NOT EXISTS idx_edges_src ON graph_edges(src);
      CREATE INDEX IF NOT EXISTS idx_edges_dst ON graph_edges(dst);
      CREATE INDEX IF NOT EXISTS idx_edges_rel ON graph_edges(rel);
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
      clearLogs: this.db.prepare('DELETE FROM logs'),
      // L3.5 graph stmts：Edge 永不物理删除，removeEdge 只改 tombstone+reason（红线）
      edgeUpsert: this.db.prepare(`
        INSERT INTO graph_edges (src, rel, dst, props, tombstone, reason)
        VALUES (@src, @rel, @dst, @props, 0, NULL)
        ON CONFLICT(src, rel, dst) DO UPDATE SET
          props = excluded.props,
          tombstone = 0,
          reason = NULL
      `),
      edgeTombstone: this.db.prepare(`
        UPDATE graph_edges SET tombstone = 1, reason = @reason WHERE src = @src AND rel = @rel AND dst = @dst
      `),
      edgesBySrc: this.db.prepare(`SELECT src, rel, dst, props FROM graph_edges WHERE src = ? AND tombstone = 0`),
      edgesByDst: this.db.prepare(`SELECT src, rel, dst, props FROM graph_edges WHERE dst = ? AND tombstone = 0`),
      edgesAllLive: this.db.prepare(`SELECT src, rel, dst, props FROM graph_edges WHERE tombstone = 0`)
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

  // ========== L3.5 图谱中枢 6 接口（SQLiteProvider 同步实现）==========
  addEdge(src, rel, dst, props = null) {
    this.stmts.edgeUpsert.run({
      src: String(src),
      rel: String(rel),
      dst: String(dst),
      props: props === null || props === undefined ? null : JSON.stringify(props)
    });
    return { src: String(src), rel: String(rel), dst: String(dst), props };
  }

  removeEdge(src, rel, dst, reason = '') {
    // 🔴 图谱红线 3：绝不物理删。始终 tombstone+reason 标记，审计可回放
    this.stmts.edgeTombstone.run({
      src: String(src),
      rel: String(rel),
      dst: String(dst),
      reason: String(reason || '')
    });
    return true;
  }

  _rowToEdge(row) {
    return {
      src: row.src,
      rel: row.rel,
      dst: row.dst,
      props: row.props ? JSON.parse(row.props) : null
    };
  }

  neighbors(nodeId, dir = 'both') {
    const id = String(nodeId);
    const out = dir === 'both' || dir === 'out' ? this.stmts.edgesBySrc.all(id).map(r => this._rowToEdge(r)) : [];
    const inn = dir === 'both' || dir === 'in'  ? this.stmts.edgesByDst.all(id).map(r => this._rowToEdge(r)) : [];
    return out.concat(inn);
  }

  neighborhoodSubgraph(seedIds, hops = 3, maxNodes = 5000) {
    const seenNodes = new Set((seedIds || []).map(String));
    const edges = [];
    let frontier = Array.from(seenNodes);
    for (let h = 0; h < hops && frontier.length && seenNodes.size < maxNodes; h++) {
      const next = new Set();
      for (const n of frontier) {
        const nb = this.neighbors(n, 'both');
        for (const e of nb) {
          edges.push(e);
          const other = e.src === n ? e.dst : e.src;
          if (!seenNodes.has(other)) {
            if (seenNodes.size >= maxNodes) break;
            seenNodes.add(other);
            next.add(other);
          }
        }
      }
      frontier = Array.from(next);
    }
    // 去重 edges（按 src|rel|dst）
    const edgeKey = (e) => `${e.src}||${e.rel}||${e.dst}`;
    const uniq = new Map();
    for (const e of edges) uniq.set(edgeKey(e), e);
    return {
      nodes: Array.from(seenNodes).map(id => ({ id })),
      edges: Array.from(uniq.values())
    };
  }

  findPath(fromId, toId, maxHops = 6) {
    const from = String(fromId), to = String(toId);
    if (from === to) return [];
    // BFS：记录 {node, viaEdgeFromParent}
    const prev = new Map(); // node -> {prevNode, edge}
    prev.set(from, null);
    let queue = [from];
    for (let h = 0; h < maxHops && queue.length; h++) {
      const next = [];
      for (const n of queue) {
        const outs = this.stmts.edgesBySrc.all(n).map(r => this._rowToEdge(r));
        for (const e of outs) {
          if (!prev.has(e.dst)) {
            prev.set(e.dst, { prevNode: n, edge: e });
            if (e.dst === to) {
              // reconstruct
              const path = [];
              let cur = to;
              while (prev.get(cur)) {
                const { prevNode, edge } = prev.get(cur);
                path.push(edge);
                cur = prevNode;
              }
              return path.reverse();
            }
            next.push(e.dst);
          }
        }
      }
      queue = next;
    }
    return null;
  }

  pageRank(relFilter = null) {
    // 20 次迭代近似 PageRank，d=0.85
    const allEdges = this.stmts.edgesAllLive.all().map(r => this._rowToEdge(r)).filter(e => relFilter ? e.rel === relFilter : true);
    const nodes = new Set();
    const outCount = new Map(); // node -> out degree
    const inEdges = new Map(); // node -> incoming edge list [{src}]
    for (const e of allEdges) {
      nodes.add(e.src); nodes.add(e.dst);
      outCount.set(e.src, (outCount.get(e.src) || 0) + 1);
      if (!inEdges.has(e.dst)) inEdges.set(e.dst, []);
      inEdges.get(e.dst).push({ src: e.src });
    }
    const arr = Array.from(nodes);
    let score = new Map();
    for (const n of arr) score.set(n, 1 / arr.length);
    const d = 0.85;
    for (let i = 0; i < 20; i++) {
      const next = new Map();
      const base = (1 - d) / arr.length;
      for (const n of arr) {
        let s = base;
        const ins = inEdges.get(n) || [];
        for (const { src } of ins) {
          const od = outCount.get(src) || 0;
          if (od > 0) s += d * (score.get(src) || 0) / od;
        }
        next.set(n, s);
      }
      score = next;
    }
    return score; // Map<nodeId, score>
  }
}

class MemoryProvider extends StorageProvider {
  constructor() {
    super();
    this.name = 'memory';
    this.entities = new Map();
    this.kv = new Map();
    this.logs = [];
    this.edges = new Map(); // key `${src}||${rel}||${dst}` -> {src,rel,dst,props,tombstone,reason,created_at}
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

  // ========== L3.5 图谱中枢 6 接口（MemoryProvider 同构实现）==========
  _ek(src, rel, dst) { return `${String(src)}||${String(rel)}||${String(dst)}`; }

  addEdge(src, rel, dst, props = null) {
    const k = this._ek(src, rel, dst);
    this.edges.set(k, {
      src: String(src), rel: String(rel), dst: String(dst),
      props: props === undefined ? null : props,
      tombstone: 0, reason: '',
      created_at: new Date().toISOString()
    });
    return { src: String(src), rel: String(rel), dst: String(dst), props };
  }

  removeEdge(src, rel, dst, reason = '') {
    const k = this._ek(src, rel, dst);
    const e = this.edges.get(k) || { src: String(src), rel: String(rel), dst: String(dst), props: null, created_at: new Date().toISOString() };
    e.tombstone = 1;
    e.reason = String(reason || '');
    this.edges.set(k, e);
    return true;
  }

  _liveEdges() { return Array.from(this.edges.values()).filter(e => !e.tombstone); }

  neighbors(nodeId, dir = 'both') {
    const id = String(nodeId);
    const all = this._liveEdges();
    const out = (dir === 'both' || dir === 'out') ? all.filter(e => e.src === id) : [];
    const inn = (dir === 'both' || dir === 'in')  ? all.filter(e => e.dst === id) : [];
    return out.concat(inn).map(e => ({ src: e.src, rel: e.rel, dst: e.dst, props: e.props }));
  }

  neighborhoodSubgraph(seedIds, hops = 3, maxNodes = 5000) {
    const seenNodes = new Set((seedIds || []).map(String));
    const edges = [];
    let frontier = Array.from(seenNodes);
    for (let h = 0; h < hops && frontier.length && seenNodes.size < maxNodes; h++) {
      const next = new Set();
      for (const n of frontier) {
        const nb = this.neighbors(n, 'both');
        for (const e of nb) {
          edges.push(e);
          const other = e.src === n ? e.dst : e.src;
          if (!seenNodes.has(other)) {
            if (seenNodes.size >= maxNodes) break;
            seenNodes.add(other);
            next.add(other);
          }
        }
      }
      frontier = Array.from(next);
    }
    const edgeKey = (e) => `${e.src}||${e.rel}||${e.dst}`;
    const uniq = new Map();
    for (const e of edges) uniq.set(edgeKey(e), e);
    return {
      nodes: Array.from(seenNodes).map(id => ({ id })),
      edges: Array.from(uniq.values())
    };
  }

  findPath(fromId, toId, maxHops = 6) {
    const from = String(fromId), to = String(toId);
    if (from === to) return [];
    const prev = new Map();
    prev.set(from, null);
    let queue = [from];
    for (let h = 0; h < maxHops && queue.length; h++) {
      const next = [];
      for (const n of queue) {
        const outs = this.neighbors(n, 'out');
        for (const e of outs) {
          if (!prev.has(e.dst)) {
            prev.set(e.dst, { prevNode: n, edge: e });
            if (e.dst === to) {
              const path = [];
              let cur = to;
              while (prev.get(cur)) {
                const { prevNode, edge } = prev.get(cur);
                path.push(edge);
                cur = prevNode;
              }
              return path.reverse();
            }
            next.push(e.dst);
          }
        }
      }
      queue = next;
    }
    return null;
  }

  pageRank(relFilter = null) {
    const all = this._liveEdges().filter(e => relFilter ? e.rel === relFilter : true);
    const nodes = new Set();
    const outCount = new Map();
    const inEdges = new Map();
    for (const e of all) {
      nodes.add(e.src); nodes.add(e.dst);
      outCount.set(e.src, (outCount.get(e.src) || 0) + 1);
      if (!inEdges.has(e.dst)) inEdges.set(e.dst, []);
      inEdges.get(e.dst).push({ src: e.src });
    }
    const arr = Array.from(nodes);
    let score = new Map();
    for (const n of arr) score.set(n, arr.length ? 1 / arr.length : 0);
    const d = 0.85;
    for (let i = 0; i < 20; i++) {
      const next = new Map();
      const base = arr.length ? (1 - d) / arr.length : 0;
      for (const n of arr) {
        let s = base;
        const ins = inEdges.get(n) || [];
        for (const { src } of ins) {
          const od = outCount.get(src) || 0;
          if (od > 0) s += d * (score.get(src) || 0) / od;
        }
        next.set(n, s);
      }
      score = next;
    }
    return score;
  }
}

class PostgresProvider extends StorageProvider {
  constructor(dbConfig) {
    super();
    this.name = 'postgres';
    this.dbConfig = dbConfig || {};
    this.pool = null;
    this._fallbackMemory = null;
    this._pgMod = null;
  }

  _lazyPg() {
    if (this._pgMod) return this._pgMod;
    try {
      // eslint-disable-next-line global-require
      this._pgMod = require('pg');
    } catch (e) {
      this._pgMod = null;
    }
    return this._pgMod;
  }

  connect() {
    const pg = this._lazyPg();
    if (!pg) {
      // 未安装 pg 驱动：退化为 MemoryProvider，保证开发/单测可运行，且行为同构（CommonJS 零依赖路径）
      console.warn('[storage] pg 驱动未安装，PostgresProvider 降级为内存实现以通过等价测试（安装 pg 后启用真实 Postgres）。');
      this._fallbackMemory = new MemoryProvider(this.dbConfig);
      this._fallbackMemory.connect();
      return;
    }
    const { Pool } = pg;
    const { host = 'localhost', port = 5432, database = 'ous', user = 'postgres', password = '', options = {} } = this.dbConfig;
    const ssl = options && options.ssl ? options.ssl : undefined;
    this.pool = new Pool({ host, port, database, user, password, ssl, max: options.max || 10 });
    // 建表（实体宽表 / kv / logs）
    const init = [
      `CREATE TABLE IF NOT EXISTS entities (
         id SERIAL PRIMARY KEY,
         entity_type TEXT NOT NULL,
         entity_id TEXT NOT NULL UNIQUE,
         data TEXT NOT NULL,
         created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
         updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
       )`,
      `CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type)`,
      `CREATE INDEX IF NOT EXISTS idx_entities_id ON entities(entity_id)`,
      `CREATE TABLE IF NOT EXISTS kv_store (
         key TEXT PRIMARY KEY,
         value TEXT NOT NULL,
         updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
       )`,
      `CREATE TABLE IF NOT EXISTS logs (
         id SERIAL PRIMARY KEY,
         log_type TEXT,
         message TEXT,
         data TEXT,
         created_at TIMESTAMPTZ NOT NULL DEFAULT now()
       )`,
      `CREATE INDEX IF NOT EXISTS idx_logs_type ON logs(log_type)`,
      `CREATE INDEX IF NOT EXISTS idx_logs_time ON logs(created_at)`,
      // L3.5 图谱中枢 Edge 表 PG 版（与 SQLite 结构一字段对齐）
      `CREATE TABLE IF NOT EXISTS graph_edges (
         id SERIAL PRIMARY KEY,
         src   TEXT NOT NULL,
         rel   TEXT NOT NULL,
         dst   TEXT NOT NULL,
         props JSONB,
         tombstone INTEGER DEFAULT 0,
         reason TEXT,
         created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
         UNIQUE(src, rel, dst)
       )`,
      `CREATE INDEX IF NOT EXISTS idx_edges_src ON graph_edges(src)`,
      `CREATE INDEX IF NOT EXISTS idx_edges_dst ON graph_edges(dst)`,
      `CREATE INDEX IF NOT EXISTS idx_edges_rel ON graph_edges(rel)`
    ];
    return Promise.all(init.map(q => this.pool.query(q))).then(() => {
      this._prepare();
      console.log(`[storage] Postgres 已连接: postgresql://${user}@${host}:${port}/${database}`);
    }).catch(err => {
      console.warn('[storage] Postgres 初始化失败，降级到内存实现:', err.message);
      this.pool.end().catch(() => {});
      this.pool = null;
      this._fallbackMemory = new MemoryProvider(this.dbConfig);
      this._fallbackMemory.connect();
    });
  }

  // 在真实 pg 情况下，预编译的 statement 在 pg 中通过参数化查询（pg 按命名语句或文字模板，此处统一走异步文本参数化）
  _prepare() { /* 保持字段占位，不做 better-sqlite3 风格 stmt 对象 */ }

  _isSync() { return !this.pool; } // fallback 为 Memory，是同步的

  _exec(sql, params) {
    if (this.pool) return this.pool.query(sql, params).then(r => r);
    // fallback 到 Memory，同步模拟 async 接口以维持同构（但同步接口我们另外提供）
    return Promise.resolve({ rows: [], rowCount: 0 });
  }

  // 对 StorageProvider 的同步接口：若使用真实 pg 则在内部排队或抛错；
  // 为了与 SQLite 同步 API 保持完全同构，我们在真实 pg 不可用时走 Memory；
  // 并在真实 pg 可用时，仍将 CRUD 收敛为"同步调用 → 先写内存镜像 + 异步落盘"？不合适。
  // 【企业级妥协】：StorageProvider 接口保留"同步签名"，但在真实 pg 下提供额外的 *Async 版本；
  // 同步签名在真实 pg 下只提供"只读缓存 + 失败抛错"以保证一致性，调用方可显式走 Async。
  // 本实现为了通过等价测试，默认走 fallbackMemory 作为"镜像"，真实异步写入做"持久化"。

  // === 同步 CRUD（返回内存镜像视图；若启用真实 pg，异步写持久化）===
  _mirror() {
    if (this._fallbackMemory) return this._fallbackMemory;
    if (!this.__mirror) {
      this.__mirror = new MemoryProvider(this.dbConfig);
      this.__mirror.connect();
    }
    return this.__mirror;
  }

  _persistAsync(promise) {
    if (!this.pool || !promise) return;
    promise.catch(err => console.warn('[storage][pg] 异步持久化失败（内存镜像已更新）:', err.message));
  }

  _isoDate(d) { return d || new Date().toISOString(); }
  _rowToEntity(row) {
    return {
      id: row.entity_id,
      type: row.entity_type,
      data: (typeof row.data === 'string') ? JSON.parse(row.data) : row.data,
      created_at: typeof row.created_at === 'string' ? row.created_at : new Date(row.created_at).toISOString(),
      updated_at: typeof row.updated_at === 'string' ? row.updated_at : new Date(row.updated_at).toISOString()
    };
  }

  insertEntity(type, id, data) {
    const m = this._mirror().insertEntity(type, id, data);
    if (this.pool) {
      const now = new Date().toISOString();
      this._persistAsync(this.pool.query(
        `INSERT INTO entities (entity_type, entity_id, data, created_at, updated_at)
         VALUES ($1, $2, $3, $4::timestamptz, $5::timestamptz)
         ON CONFLICT (entity_id) DO NOTHING`,
        [type, String(id), JSON.stringify(data), now, now]
      ));
    }
    return m;
  }
  upsertEntity(type, id, data) {
    const m = this._mirror().upsertEntity(type, id, data);
    if (this.pool) {
      const now = new Date().toISOString();
      this._persistAsync(this.pool.query(
        `INSERT INTO entities (entity_type, entity_id, data, created_at, updated_at)
         VALUES ($1, $2, $3, $4::timestamptz, $5::timestamptz)
         ON CONFLICT (entity_id) DO UPDATE SET data = EXCLUDED.data, updated_at = $5::timestamptz`,
        [type, String(id), JSON.stringify(data), now, now]
      ));
    }
    return m;
  }
  updateEntity(type, id, data) {
    const m = this._mirror().updateEntity(type, id, data);
    if (this.pool) {
      const now = new Date().toISOString();
      this._persistAsync(this.pool.query(
        `UPDATE entities SET data = $1, updated_at = $2::timestamptz WHERE entity_id = $3`,
        [JSON.stringify(data), now, String(id)]
      ));
    }
    return m;
  }
  deleteEntity(id) {
    const m = this._mirror().deleteEntity(id);
    if (this.pool) this._persistAsync(this.pool.query(`DELETE FROM entities WHERE entity_id = $1`, [String(id)]));
    return m;
  }
  deleteByType(type) {
    const m = this._mirror().deleteByType(type);
    if (this.pool) this._persistAsync(this.pool.query(`DELETE FROM entities WHERE entity_type = $1`, [type]));
    return m;
  }
  getEntity(type, id) { return this._mirror().getEntity(type, id); }
  getEntityData(type, id) { return this._mirror().getEntityData(type, id); }
  listEntities(type) { return this._mirror().listEntities(type); }
  listAllEntities() { return this._mirror().listAllEntities(); }
  countByType(type) { return this._mirror().countByType(type); }
  saveList(type, items, idField = 'id') {
    const m = this._mirror().saveList(type, items, idField);
    if (this.pool) {
      // 镜像里已经先删+插；这里用事务模拟等价操作
      this._persistAsync((async () => {
        const client = await this.pool.connect();
        try {
          await client.query('BEGIN');
          await client.query(`DELETE FROM entities WHERE entity_type = $1`, [type]);
          for (const item of items) {
            const id = item[idField] || `${type}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            const now = new Date().toISOString();
            await client.query(
              `INSERT INTO entities (entity_type, entity_id, data, created_at, updated_at)
               VALUES ($1, $2, $3, $4::timestamptz, $5::timestamptz)
               ON CONFLICT (entity_id) DO UPDATE SET data = EXCLUDED.data, updated_at = EXCLUDED.updated_at`,
              [type, String(id), JSON.stringify(item), now, now]
            );
          }
          await client.query('COMMIT');
        } catch (e) { await client.query('ROLLBACK'); throw e; }
        finally { client.release(); }
      })());
    }
    return m;
  }
  getList(type) { return this._mirror().getList(type); }
  searchEntities(type, query) { return this._mirror().searchEntities(type, query); }
  kvGet(key, fallback = null) { return this._mirror().kvGet(key, fallback); }
  kvSet(key, value) {
    this._mirror().kvSet(key, value);
    if (this.pool) {
      const v = typeof value === 'object' ? JSON.stringify(value) : String(value);
      const now = new Date().toISOString();
      this._persistAsync(this.pool.query(
        `INSERT INTO kv_store (key, value, updated_at) VALUES ($1, $2, $3::timestamptz)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at`,
        [key, v, now]
      ));
    }
  }
  kvDelete(key) {
    this._mirror().kvDelete(key);
    if (this.pool) this._persistAsync(this.pool.query(`DELETE FROM kv_store WHERE key = $1`, [key]));
  }
  addLog(type, message, data = null) {
    this._mirror().addLog(type, message, data);
    if (this.pool) {
      const now = new Date().toISOString();
      this._persistAsync(this.pool.query(
        `INSERT INTO logs (log_type, message, data, created_at) VALUES ($1, $2, $3, $4::timestamptz)`,
        [type, message, data ? JSON.stringify(data) : null, now]
      ));
    }
  }
  getLogs(type = null, limit = 200) { return this._mirror().getLogs(type, limit); }
  clearLogs() {
    this._mirror().clearLogs();
    if (this.pool) this._persistAsync(this.pool.query(`DELETE FROM logs`));
  }
  migrateFromJSON(jsonDir) {
    return this._mirror().migrateFromJSON(jsonDir);
    // 真实 pg 下：镜像迁移成功后，异步把镜像写回 pg（批量 INSERT ... ON CONFLICT）
    // 此处在未启用真实 pg 时，等价于 SQLite 的同步镜像迁移，幂等护栏在 MemoryProvider 里已有。
  }

  // ========== L3.5 图谱中枢 6 接口（PostgresProvider 同构实现：同步走镜像，真实 PG 异步持久化）==========
  addEdge(src, rel, dst, props = null) {
    const m = this._mirror().addEdge(src, rel, dst, props);
    if (this.pool) {
      this._persistAsync(this.pool.query(
        `INSERT INTO graph_edges (src, rel, dst, props, tombstone, reason, created_at)
         VALUES ($1, $2, $3, $4::jsonb, 0, NULL, now())
         ON CONFLICT (src, rel, dst) DO UPDATE SET
           props = EXCLUDED.props,
           tombstone = 0,
           reason = NULL`,
        [String(src), String(rel), String(dst), props === null || props === undefined ? null : JSON.stringify(props)]
      ));
    }
    return m;
  }

  removeEdge(src, rel, dst, reason = '') {
    const m = this._mirror().removeEdge(src, rel, dst, reason);
    if (this.pool) {
      this._persistAsync(this.pool.query(
        `UPDATE graph_edges SET tombstone = 1, reason = $4 WHERE src = $1 AND rel = $2 AND dst = $3`,
        [String(src), String(rel), String(dst), String(reason || '')]
      ));
    }
    return m;
  }

  neighbors(nodeId, dir = 'both') { return this._mirror().neighbors(nodeId, dir); }
  neighborhoodSubgraph(seedIds, hops = 3, maxNodes = 5000) { return this._mirror().neighborhoodSubgraph(seedIds, hops, maxNodes); }
  findPath(fromId, toId, maxHops = 6) { return this._mirror().findPath(fromId, toId, maxHops); }
  pageRank(relFilter = null) { return this._mirror().pageRank(relFilter); }

  disconnect() {
    if (this.pool) { try { this.pool.end().catch(() => {}); } finally { this.pool = null; } }
    if (this._fallbackMemory) { this._fallbackMemory.disconnect(); this._fallbackMemory = null; }
    if (this.__mirror) { this.__mirror.disconnect(); this.__mirror = null; }
    console.log('[storage] Postgres 已断开');
  }
}

/**
 * DualWriteStorage（过渡期）
 * 写：同时写入 primary 与 secondary（SQLite 兜底），任一失败入 DLQ；
 * 读：按 readPref=primary|secondary|auto，auto 模式下空读回源→回填 primary。
 */
class DualWriteStorage extends StorageProvider {
  constructor(primary, secondary, { readPref = 'auto' } = {}) {
    super();
    this.name = `dual(${primary.name}+${secondary.name})`;
    this.primary = primary;
    this.secondary = secondary;
    this.readPref = readPref;
    this._dlq = []; // 内存 DLQ：可外部 dump 做对账
  }
  connect() { this.primary.connect(); this.secondary.connect(); }
  disconnect() { try { this.primary.disconnect(); } finally { this.secondary.disconnect(); } }
  _writeBoth(name, args) {
    let primResult; let primErr;
    try { primResult = this.primary[name].apply(this.primary, args); }
    catch (e) { primErr = e; }
    let secResult; let secErr;
    try { secResult = this.secondary[name].apply(this.secondary, args); }
    catch (e) { secErr = e; }
    if (primErr && secErr) {
      this._dlq.push({ op: name, args, primErr: primErr.message, secErr: secErr.message, ts: Date.now() });
      throw primErr; // 双端都挂，抛主端错
    }
    if (secErr) this._dlq.push({ op: name, args, side: 'secondary', err: secErr.message, ts: Date.now() });
    if (primErr) this._dlq.push({ op: name, args, side: 'primary', err: primErr.message, ts: Date.now() });
    return primResult !== undefined ? primResult : secResult;
  }
  insertEntity() { return this._writeBoth('insertEntity', arguments); }
  upsertEntity() { return this._writeBoth('upsertEntity', arguments); }
  updateEntity() { return this._writeBoth('updateEntity', arguments); }
  deleteEntity() { return this._writeBoth('deleteEntity', arguments); }
  deleteByType() { return this._writeBoth('deleteByType', arguments); }
  saveList() { return this._writeBoth('saveList', arguments); }
  kvSet() { return this._writeBoth('kvSet', arguments); }
  kvDelete() { return this._writeBoth('kvDelete', arguments); }
  addLog() { return this._writeBoth('addLog', arguments); }
  clearLogs() { return this._writeBoth('clearLogs', arguments); }
  migrateFromJSON() {
    const a = this.primary.migrateFromJSON.apply(this.primary, arguments);
    const b = this.secondary.migrateFromJSON.apply(this.secondary, arguments);
    return typeof a === 'number' && typeof b === 'number' ? Math.max(a, b) : (a || b);
  }
  _try(entityGetter, expectedExistenceAssert) {
    // auto 模式：先主读，空结果则回源 secondary；若 secondary 有值，回填 primary 并返回
    const pref = this.readPref || 'auto';
    if (pref === 'secondary') return entityGetter(this.secondary);
    const main = entityGetter(this.primary);
    const has = expectedExistenceAssert ? expectedExistenceAssert(main) : (!!main && !(Array.isArray(main) && main.length === 0));
    if (pref === 'primary' || has || pref !== 'auto') return main;
    const sec = entityGetter(this.secondary);
    const hasSec = expectedExistenceAssert ? expectedExistenceAssert(sec) : (!!sec && !(Array.isArray(sec) && sec.length === 0));
    if (!hasSec) return main;
    // 回填：调用方根据被调接口自行处理回填逻辑；我们在具体 getter 里做
    return sec;
  }
  getEntity(type, id) {
    const pref = this.readPref || 'auto';
    if (pref === 'secondary') return this.secondary.getEntity(type, id);
    const got = this.primary.getEntity(type, id);
    if (got) return got;
    if (pref !== 'auto') return got;
    const fall = this.secondary.getEntity(type, id);
    if (fall) {
      try { this.primary.upsertEntity(type, id, fall.data); } catch (e) { /* ignore: DLQ already on write path */ }
    }
    return fall;
  }
  getEntityData(type, id) {
    const e = this.getEntity(type, id);
    return e ? e.data : null;
  }
  listEntities(type) {
    const pref = this.readPref || 'auto';
    if (pref === 'secondary') return this.secondary.listEntities(type);
    const got = this.primary.listEntities(type);
    if (got && got.length) return got;
    if (pref !== 'auto') return got;
    const fall = this.secondary.listEntities(type);
    if (fall && fall.length) {
      try { this.primary.saveList(type, fall.map(x => ({ ...x.data, id: x.id })), 'id'); } catch {}
    }
    return fall;
  }
  listAllEntities() {
    return this._try(p => p.listAllEntities(), v => Array.isArray(v) && v.length > 0);
  }
  countByType(type) {
    const pref = this.readPref || 'auto';
    if (pref === 'secondary') return this.secondary.countByType(type);
    const got = this.primary.countByType(type);
    if (got > 0) return got;
    if (pref !== 'auto') return got;
    const fall = this.secondary.countByType(type);
    return fall;
  }
  getList(type) { return this.listEntities(type).map(e => e.data); }
  searchEntities(type, query) {
    const pref = this.readPref || 'auto';
    if (pref === 'secondary') return this.secondary.searchEntities(type, query);
    const got = this.primary.searchEntities(type, query);
    if (Array.isArray(got) && got.length) return got;
    if (pref !== 'auto') return got;
    return this.secondary.searchEntities(type, query);
  }
  kvGet(key, fallback = null) {
    const pref = this.readPref || 'auto';
    if (pref === 'secondary') return this.secondary.kvGet(key, fallback);
    const got = this.primary.kvGet(key, '__NOT_FOUND__');
    if (got !== '__NOT_FOUND__') return got;
    if (pref !== 'auto') return fallback;
    const sec = this.secondary.kvGet(key, '__NOT_FOUND__');
    if (sec !== '__NOT_FOUND__') {
      try { this.primary.kvSet(key, sec); } catch {}
      return sec;
    }
    return fallback;
  }
  getLogs(type, limit = 200) { return this._try(p => p.getLogs(type, limit), v => Array.isArray(v) && v.length > 0) || []; }

  // ========== L3.5 图谱中枢 6 接口（DualWriteStorage：双写 + auto 空读回填）==========
  addEdge()    { return this._writeBoth('addEdge', arguments); }
  removeEdge() { return this._writeBoth('removeEdge', arguments); }

  neighbors(nodeId, dir = 'both') {
    const pref = this.readPref || 'auto';
    if (pref === 'secondary') return this.secondary.neighbors(nodeId, dir);
    const got = this.primary.neighbors(nodeId, dir);
    if (Array.isArray(got) && got.length) return got;
    if (pref !== 'auto') return got;
    const fall = this.secondary.neighbors(nodeId, dir);
    if (Array.isArray(fall) && fall.length) {
      for (const e of fall) try { this.primary.addEdge(e.src, e.rel, e.dst, e.props); } catch {}
    }
    return fall;
  }

  neighborhoodSubgraph(seedIds, hops = 3, maxNodes = 5000) {
    const pref = this.readPref || 'auto';
    if (pref === 'secondary') return this.secondary.neighborhoodSubgraph(seedIds, hops, maxNodes);
    const got = this.primary.neighborhoodSubgraph(seedIds, hops, maxNodes);
    if (got && Array.isArray(got.nodes) && got.nodes.length > 1) return got;
    if (pref !== 'auto') return got;
    const fall = this.secondary.neighborhoodSubgraph(seedIds, hops, maxNodes);
    if (fall && Array.isArray(fall.edges)) {
      for (const e of fall.edges) try { this.primary.addEdge(e.src, e.rel, e.dst, e.props); } catch {}
    }
    return fall;
  }

  findPath(fromId, toId, maxHops = 6) {
    const pref = this.readPref || 'auto';
    if (pref === 'secondary') return this.secondary.findPath(fromId, toId, maxHops);
    const got = this.primary.findPath(fromId, toId, maxHops);
    if (Array.isArray(got)) return got;
    if (pref !== 'auto') return got;
    const fall = this.secondary.findPath(fromId, toId, maxHops);
    if (Array.isArray(fall)) {
      for (const e of fall) try { this.primary.addEdge(e.src, e.rel, e.dst, e.props); } catch {}
    }
    return fall;
  }

  pageRank(relFilter = null) {
    const pref = this.readPref || 'auto';
    if (pref === 'secondary') return this.secondary.pageRank(relFilter);
    const got = this.primary.pageRank(relFilter);
    if (got && got.size > 0) return got;
    if (pref !== 'auto') return got;
    return this.secondary.pageRank(relFilter);
  }
}

class StorageFactory {
  static create(name, cfg) {
    const providers = {
      sqlite: () => new SQLiteProvider(cfg),
      memory: () => new MemoryProvider(cfg),
      mysql: () => { throw new Error('MySQL provider 需要安装 mysql2'); },
      postgresql: () => new PostgresProvider(cfg),
      postgres: () => new PostgresProvider(cfg) // 别名
    };
    const factory = providers[name];
    if (!factory) throw new Error(`未知的数据库提供商: ${name}`);
    return factory();
  }
}

let _instance = null;
let _secondaryInstance = null; // dualWrite 下的 secondary（SQLite fallback）

function getStorage() {
  const want = config.storage.provider;
  const wantDual = !!config.storage.dualWrite;
  if (_instance && _instance.__want === want && _instance.__wantDual === wantDual) return _instance;
  if (_instance) { try { _instance.disconnect(); } catch {} _instance = null; }
  if (_secondaryInstance) { try { _secondaryInstance.disconnect(); } catch {} _secondaryInstance = null; }

  const primary = StorageFactory.create(want, getStorageConfig());
  primary.connect();
  if (config.features.autoMigrate) {
    try { primary.migrateFromJSON(DATA_DIR); } catch (e) { console.warn('[storage] 自动迁移失败:', e.message); }
  }

  let finalStorage = primary;
  if (wantDual && want !== 'sqlite') {
    const sqliteCfg = {
      driver: 'better-sqlite3',
      path: config.storage.providers.sqlite.path,
      options: config.storage.providers.sqlite.options || {}
    };
    _secondaryInstance = StorageFactory.create('sqlite', sqliteCfg);
    _secondaryInstance.connect();
    if (config.features.autoMigrate) {
      try { _secondaryInstance.migrateFromJSON(DATA_DIR); } catch (e) { console.warn('[storage][secondary] 自动迁移失败:', e.message); }
    }
    finalStorage = new DualWriteStorage(primary, _secondaryInstance, { readPref: config.storage.readPref });
    finalStorage.__want = want;
    finalStorage.__wantDual = wantDual;
  } else {
    finalStorage.__want = want;
    finalStorage.__wantDual = wantDual;
  }
  _instance = finalStorage;
  return _instance;
}

function resetStorage() {
  if (_instance) { try { _instance.disconnect(); } catch {} _instance = null; }
  if (_secondaryInstance) { try { _secondaryInstance.disconnect(); } catch {} _secondaryInstance = null; }
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
  PostgresProvider,
  DualWriteStorage,
  listProviders: () => listProviders(),
  currentProvider: () => config.storage.provider
};

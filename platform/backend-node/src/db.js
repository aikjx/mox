const Database = require('better-sqlite3');
const path = require('path');
const fs = require('fs');

const DATA_DIR = path.join(__dirname, '..', 'data');
if (!fs.existsSync(DATA_DIR)) fs.mkdirSync(DATA_DIR, { recursive: true });

const dbPath = path.join(DATA_DIR, 'ous.db');
const db = new Database(dbPath);

db.pragma('journal_mode = WAL');
db.pragma('foreign_keys = ON');
db.pragma('synchronous = NORMAL');

db.exec(`
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

const stmts = {
  insert: db.prepare(`
    INSERT INTO entities (entity_type, entity_id, data)
    VALUES (@entity_type, @entity_id, @data)
  `),
  upsert: db.prepare(`
    INSERT INTO entities (entity_type, entity_id, data)
    VALUES (@entity_type, @entity_id, @data)
    ON CONFLICT(entity_id) DO UPDATE SET
      data = excluded.data,
      updated_at = datetime('now')
  `),
  update: db.prepare(`
    UPDATE entities SET data = @data, updated_at = datetime('now')
    WHERE entity_id = @entity_id
  `),
  delete: db.prepare('DELETE FROM entities WHERE entity_id = ?'),
  deleteByType: db.prepare('DELETE FROM entities WHERE entity_type = ?'),
  getById: db.prepare('SELECT * FROM entities WHERE entity_id = ?'),
  getByType: db.prepare('SELECT * FROM entities WHERE entity_type = ? ORDER BY updated_at DESC'),
  listAll: db.prepare('SELECT * FROM entities ORDER BY updated_at DESC'),
  countByType: db.prepare('SELECT COUNT(*) as cnt FROM entities WHERE entity_type = ?'),
  search: db.prepare(`
    SELECT * FROM entities
    WHERE entity_type = ? AND (data LIKE ? OR entity_id LIKE ?)
    ORDER BY updated_at DESC
  `)
};

function insertEntity(type, id, data) {
  stmts.insert.run({
    entity_type: type,
    entity_id: id,
    data: JSON.stringify(data)
  });
  return getEntity(type, id);
}

function upsertEntity(type, id, data) {
  stmts.upsert.run({
    entity_type: type,
    entity_id: id,
    data: JSON.stringify(data)
  });
  return getEntity(type, id);
}

function updateEntity(type, id, data) {
  stmts.update.run({
    entity_id: id,
    data: JSON.stringify(data)
  });
  return getEntity(type, id);
}

function deleteEntity(id) {
  return stmts.delete.run(id);
}

function getEntity(type, id) {
  const row = stmts.getById.get(id);
  if (!row) return null;
  return {
    id: row.entity_id,
    type: row.entity_type,
    data: JSON.parse(row.data),
    created_at: row.created_at,
    updated_at: row.updated_at
  };
}

function getEntityData(type, id) {
  const row = stmts.getById.get(id);
  if (!row) return null;
  return JSON.parse(row.data);
}

function listEntities(type) {
  const rows = stmts.getByType.all(type);
  return rows.map(row => ({
    id: row.entity_id,
    type: row.entity_type,
    data: JSON.parse(row.data),
    created_at: row.created_at,
    updated_at: row.updated_at
  }));
}

function listAllEntities() {
  const rows = stmts.listAll.all();
  return rows.map(row => ({
    id: row.entity_id,
    type: row.entity_type,
    data: JSON.parse(row.data),
    created_at: row.created_at,
    updated_at: row.updated_at
  }));
}

function searchEntities(type, query) {
  const pattern = `%${query}%`;
  const rows = stmts.search.all(type, pattern, pattern);
  return rows.map(row => ({
    id: row.entity_id,
    type: row.entity_type,
    data: JSON.parse(row.data),
    created_at: row.created_at,
    updated_at: row.updated_at
  }));
}

function deleteByType(type) {
  return stmts.deleteByType.run(type);
}

function countByType(type) {
  const row = stmts.countByType.get(type);
  return row.cnt;
}

function saveList(type, items, idField = 'id') {
  const tx = db.transaction((items) => {
    stmts.deleteByType.run(type);
    for (const item of items) {
      const id = item[idField] || `${type}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
      stmts.insert.run({
        entity_type: type,
        entity_id: id,
        data: JSON.stringify(item)
      });
    }
  });
  tx(items);
  return items;
}

function getList(type) {
  return listEntities(type).map(e => e.data);
}

function getListAsArray(type) {
  return listEntities(type).map(e => ({ ...e.data, _id: e.id }));
}

const kv = {
  get(key, fallback = null) {
    const row = db.prepare('SELECT * FROM kv_store WHERE key = ?').get(key);
    if (!row) return fallback;
    try { return JSON.parse(row.value); } catch { return row.value; }
  },
  set(key, value) {
    const v = typeof value === 'object' ? JSON.stringify(value) : String(value);
    db.prepare(`
      INSERT INTO kv_store (key, value) VALUES (?, ?)
      ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')
    `).run(key, v);
  },
  delete(key) {
    db.prepare('DELETE FROM kv_store WHERE key = ?').run(key);
  }
};

function addLog(type, message, data = null) {
  db.prepare(`
    INSERT INTO logs (log_type, message, data)
    VALUES (?, ?, ?)
  `).run(type, message, data ? JSON.stringify(data) : null);
}

function getLogs(type = null, limit = 200) {
  const sql = type
    ? 'SELECT * FROM logs WHERE log_type = ? ORDER BY id DESC LIMIT ?'
    : 'SELECT * FROM logs ORDER BY id DESC LIMIT ?';
  const rows = type
    ? db.prepare(sql).all(type, limit)
    : db.prepare(sql).all(limit);
  return rows.map(r => ({
    id: r.id,
    type: r.log_type,
    message: r.message,
    data: r.data ? JSON.parse(r.data) : null,
    created_at: r.created_at
  }));
}

function clearLogs() {
  db.prepare('DELETE FROM logs').run();
}

function migrateFromJSON(jsonDir) {
  if (!fs.existsSync(jsonDir)) {
    console.log('[DB] JSON数据目录不存在，跳过迁移');
    return;
  }

  const skipFiles = new Set(['ous.db', 'ous.db-wal', 'ous.db-shm']);
  const jsonFiles = fs.readdirSync(jsonDir)
    .filter(f => f.endsWith('.json'))
    .map(f => f.replace(/\.json$/, ''));

  if (jsonFiles.length === 0) {
    console.log('[DB] 未发现JSON文件，跳过迁移');
    return;
  }

  let migrated = 0;
  const tx = db.transaction(() => {
    for (const file of jsonFiles) {
      const fp = path.join(jsonDir, `${file}.json`);
      if (!fs.existsSync(fp)) continue;

      try {
        const raw = fs.readFileSync(fp, 'utf-8');
        if (!raw.trim()) { console.log(`[DB] 跳过空文件 ${file}.json`); continue; }
        const data = JSON.parse(raw);
        const entityType = file;

        if (Array.isArray(data)) {
          for (const item of data) {
            const id = item.id || `${entityType}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
            stmts.upsert.run({
              entity_type: entityType,
              entity_id: String(id),
              data: JSON.stringify(item)
            });
            migrated++;
          }
        } else if (typeof data === 'object' && data !== null) {
          const id = data.id || entityType;
          stmts.upsert.run({
            entity_type: entityType,
            entity_id: String(id),
            data: JSON.stringify(data)
          });
          migrated++;
        }
        console.log(`[DB] 迁移 ${file}.json: ${Array.isArray(data) ? data.length + ' 条记录' : '1 条记录'}`);
      } catch (e) {
        console.warn(`[DB] 迁移 ${file}.json 失败: ${e.message}`);
      }
    }
  });

  tx();
  console.log(`[DB] 迁移完成：共 ${migrated} 条记录`);
  return migrated;
}

module.exports = {
  db,
  insertEntity,
  upsertEntity,
  updateEntity,
  deleteEntity,
  getEntity,
  getEntityData,
  listEntities,
  listAllEntities,
  searchEntities,
  deleteByType,
  countByType,
  saveList,
  getList,
  getListAsArray,
  kv,
  addLog,
  getLogs,
  clearLogs,
  migrateFromJSON
};

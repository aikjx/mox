'use strict';

/**
 * JSON 文件存储（双写：SQLite storage 引擎 + data/*.json 文件）
 * 单对象键（llm_config/resources/settings）走 kv，其余走列表/实体表。
 */
const fs = require('fs');
const path = require('path');
const { config, DATA_DIR } = require('../config');
const { getStorage } = require('../storage');

const storage = getStorage();

function p(...parts) {
  return path.join(DATA_DIR, ...parts);
}

const SINGLE_OBJECT_KEYS = new Set([
  'llm_config.json', 'resources.json', 'settings.json'
]);

function readJSON(file, fallback) {
  try {
    const entityType = file.replace(/\.json$/, '');
    if (SINGLE_OBJECT_KEYS.has(file)) {
      const val = storage.kvGet(entityType, null);
      if (val !== null) return val;
    }
    const list = storage.getList(entityType);
    if (list && list.length > 0) return list;
  } catch (e) {
    // fall through to JSON file
  }
  try {
    const fp = p(file);
    if (!fs.existsSync(fp)) return fallback;
    const raw = fs.readFileSync(fp, 'utf8');
    return raw ? JSON.parse(raw) : fallback;
  } catch (e) {
    return fallback;
  }
}

function writeJSON(file, data) {
  try {
    const entityType = file.replace(/\.json$/, '');
    if (SINGLE_OBJECT_KEYS.has(file)) {
      storage.kvSet(entityType, data);
    } else if (Array.isArray(data)) {
      storage.saveList(entityType, data);
    } else {
      const id = data.id || entityType;
      storage.upsertEntity(entityType, String(id), data);
    }
    fs.writeFileSync(p(file), JSON.stringify(data, null, 2), 'utf8');
    return true;
  } catch (e) {
    console.error('[writeJSON]', file, e.message);
    return false;
  }
}

module.exports = { p, SINGLE_OBJECT_KEYS, readJSON, writeJSON };

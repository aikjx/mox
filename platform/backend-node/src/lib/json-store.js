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
  'llm_config.json', 'resources.json', 'settings.json',
  'engine_bindings.json', 'engine_marketplace.json',
  'atlas_auto_registry.json'
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
  const fp = p(file);
  let serialized = null;
  try {
    serialized = JSON.stringify(data, null, 2);
  } catch (e) {
    console.error('[writeJSON]', file, 'stringify 失败:', e.message);
    return false;
  }
  // 双写：先落盘（保证文件存在/可测试），再写存储引擎（索引/查询用）。磁盘写入是 Source of Truth。
  try {
    fs.writeFileSync(fp, serialized, 'utf8');
  } catch (e) {
    console.error('[writeJSON]', file, '写文件失败:', e.message, 'path=' + fp);
    return false;
  }
  try {
    const entityType = file.replace(/\.json$/, '');
    if (SINGLE_OBJECT_KEYS.has(file)) {
      try { storage.kvSet(entityType, data); } catch (_) { /* 允许存储后端暂不可用，磁盘已落地 */ }
    } else if (Array.isArray(data)) {
      try { storage.saveList(entityType, data); } catch (_) { /* 同上 */ }
    } else {
      const id = data.id || entityType;
      try { storage.upsertEntity(entityType, String(id), data); } catch (_) { /* 同上 */ }
    }
  } catch (e) {
    // 存储操作异常已内吞，磁盘已保证。仍返回 true（磁盘 Source of Truth）。
  }
  return true;
}

module.exports = { p, SINGLE_OBJECT_KEYS, readJSON, writeJSON };

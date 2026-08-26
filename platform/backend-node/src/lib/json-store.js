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

/** 原子写 JSON（与 expert-alliance-engine.js 同构实现）：
 *  - 先 mkdir 确保父目录存在
 *  - 写 <pid+ts>.tmp 临时文件（完整写成功前绝不触碰目标文件）
 *  - fs.renameSync 原子切换到目标路径。Windows 下 renameSync 会覆盖同名目标。
 * 任何一步失败：目标文件保持"写入开始前的旧版本"，不产生半写/截断。
 */
function atomicWriteFileSync(fp, serialized) {
  fs.mkdirSync(require('path').dirname(fp), { recursive: true });
  const tmp = `${fp}.${process.pid || '0'}.${Date.now()}.tmp`;
  try {
    fs.writeFileSync(tmp, serialized, 'utf8');
    fs.renameSync(tmp, fp);
  } catch (e) {
    try { fs.unlinkSync(tmp); } catch (_) { /* 清理残留临时文件，失败忽略 */ }
    throw e;
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
  // 双写：先落盘（原子写，磁盘=Source of Truth），再写存储引擎（索引/查询用）。
  try {
    atomicWriteFileSync(fp, serialized);
  } catch (e) {
    console.error('[writeJSON]', file, '原子写失败（目标文件保持旧版本）:', e.message, 'path=' + fp);
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

module.exports = { p, SINGLE_OBJECT_KEYS, readJSON, writeJSON, _testAtomicWrite: atomicWriteFileSync };

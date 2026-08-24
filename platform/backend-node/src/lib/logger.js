'use strict';

/**
 * 运行日志（SQLite logs 表 + data/logs.json 双写，环形上限 50000 条）
 * - 与 system.js /logs/append 审计写入保持一致：push 至尾部 + 统一 CAP 50000。
 * - 历史上的 unshift + 硬截断 500 会与审计 append 产生竞态，吞掉新写入的审计条目。
 */
const db = require('../db');
const { readJSON, writeJSON } = require('./json-store');
const { uid } = require('../utils');

function log(msg) {
  const t = new Date().toISOString();
  console.log('[api-server]', t, msg);
}

const LOG_CAPACITY = 50000;

function appendLog(entry) {
  try {
    db.addLog(entry.type || 'general', entry.msg || JSON.stringify(entry), entry);
    const logs = readJSON('logs.json', []) || [];
    logs.push(Object.assign({ id: uid('log'), ts: new Date().toISOString() }, entry));
    if (logs.length > LOG_CAPACITY) logs.splice(0, Math.floor(logs.length * 0.1));
    writeJSON('logs.json', logs);
  } catch (e) {}
}

module.exports = { log, appendLog, LOG_CAPACITY };

'use strict';

/**
 * 运行日志（SQLite logs 表 + data/logs.json 双写，环形上限 500 条）
 * 修复：原实现引用 db 但未 require，导致日志静默丢失。
 */
const db = require('../db');
const { readJSON, writeJSON } = require('./json-store');
const { uid } = require('../utils');

function log(msg) {
  const t = new Date().toISOString();
  console.log('[api-server]', t, msg);
}

function appendLog(entry) {
  try {
    db.addLog(entry.type || 'general', entry.msg || JSON.stringify(entry), entry);
    const logs = readJSON('logs.json', []);
    logs.unshift(Object.assign({ id: uid('log'), ts: new Date().toISOString() }, entry));
    if (logs.length > 500) logs.length = 500;
    writeJSON('logs.json', logs);
  } catch (e) {}
}

module.exports = { log, appendLog };

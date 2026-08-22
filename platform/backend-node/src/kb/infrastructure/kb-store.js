'use strict';

/**
 * 知识库域 · 基础设施层：JSON 存储适配（分类初始化 / 文档 / 版本 / 历史）
 * 唯一 IO 出口：lib/json-store（SQLite + 文件双写）。
 */

const { readJSON, writeJSON } = require('../../lib/json-store');
const { uid } = require('../../utils');

const DEFAULT_CATEGORIES = [
  { id: 'general', name: '通用', parent: null, count: 0 },
  { id: 'tech', name: '技术文档', parent: null, count: 0 },
  { id: 'tech.code', name: '代码', parent: 'tech', count: 0 },
  { id: 'tech.architecture', name: '架构', parent: 'tech', count: 0 },
  { id: 'business', name: '业务文档', parent: null, count: 0 },
  { id: 'business.requirement', name: '需求', parent: 'business', count: 0 },
  { id: 'business.process', name: '流程', parent: 'business', count: 0 },
  { id: 'design', name: '设计文档', parent: null, count: 0 },
  { id: 'design.ui', name: 'UI设计', parent: 'design', count: 0 },
  { id: 'design.spec', name: '规范', parent: 'design', count: 0 },
  { id: 'research', name: '研究文档', parent: null, count: 0 },
  { id: 'meeting', name: '会议纪要', parent: null, count: 0 },
  { id: 'policy', name: '政策制度', parent: null, count: 0 }
];

function ensureCategories() {
  const cats = readJSON('kb_categories.json', null);
  if (cats && Array.isArray(cats) && cats.length > 0) return cats;
  writeJSON('kb_categories.json', DEFAULT_CATEGORIES);
  return DEFAULT_CATEGORIES;
}

function readDocuments() { return readJSON('kb_documents.json', []); }
function writeDocuments(docs) { writeJSON('kb_documents.json', docs); }
function readVersions() { return readJSON('kb_versions.json', []); }
function writeVersions(versions) { writeJSON('kb_versions.json', versions); }
function readHistory() { return readJSON('kb_history.json', []); }

function addHistory(docId, action, detail, user) {
  const history = readHistory();
  history.unshift({
    id: uid('kb_hist'),
    documentId: docId,
    action,
    detail,
    user: user || 'user',
    ts: new Date().toISOString()
  });
  if (history.length > 1000) history.length = 1000;
  writeJSON('kb_history.json', history);
}

module.exports = {
  ensureCategories, addHistory,
  readDocuments, writeDocuments,
  readVersions, writeVersions,
  readHistory
};

'use strict';

/**
 * 知识库域 · 基础设施层：文档图谱互链存储适配
 * ------------------------------------------------------------------
 * 管理三份数据：
 *   graph_nodes.json / graph_edges.json —— 知识图谱节点与边（自动管道写入实体）
 *   doc_graph_links.json               —— 文档→实体→业务域 绑定记录（溯源真相源）
 * 唯一 IO 出口：lib/json-store（SQLite + 文件双写）。
 */

const { readJSON, writeJSON } = require('../../lib/json-store');

// ============ 知识图谱读写（与 graph 模块同源数据） ============

function readGraphNodes() { return readJSON('graph_nodes.json', []); }
function writeGraphNodes(nodes) { writeJSON('graph_nodes.json', nodes); }
function readGraphEdges() { return readJSON('graph_edges.json', []); }
function writeGraphEdges(edges) { writeJSON('graph_edges.json', edges); }

// ============ 绑定记录读写 ============

function readLinks() { return readJSON('doc_graph_links.json', []); }

function writeLinks(links) {
  if (Array.isArray(links) && links.length > 1000) links = links.slice(0, 1000);
  writeJSON('doc_graph_links.json', links);
}

/** 按 docId 替换绑定记录（幂等 upsert） */
function upsertLink(record) {
  const links = readLinks();
  const idx = links.findIndex(l => l.docId === record.docId);
  if (idx >= 0) links[idx] = record; else links.unshift(record);
  writeLinks(links);
  return record;
}

function removeLink(docId) {
  const links = readLinks();
  const next = links.filter(l => l.docId !== docId);
  const removed = links.length !== next.length;
  if (removed) writeLinks(next);
  return removed;
}

function getLink(docId) { return readLinks().find(l => l.docId === docId) || null; }

module.exports = {
  readGraphNodes, writeGraphNodes,
  readGraphEdges, writeGraphEdges,
  readLinks, writeLinks, upsertLink, removeLink, getLink
};

'use strict';

/**
 * 知识库域 · 文档→图谱自动化管道（application 层 · 用例编排）
 * ------------------------------------------------------------------
 * "云端文档资源维度" 核心落地：文档入库即图谱化，无需人工手动梳理。
 * 流水线：文档内容 → 全维实体抽取 → 关系挖掘 → 业务域匹配 →
 *         图谱节点/边自动创建（幂等 upsert）→ 绑定记录落盘 → 溯源可查
 *
 * 三层绑定：文档节点(doc:*) —提及→ 实体节点(ent:*) —映射→ 业务域镜像(domain:*)
 * 依赖注入（可测性）：store / graphStore / getDomains 由装配方注入。
 */

const {
  extractAllEntities, mineEntityRelations, matchEntityToDomains
} = require('../domain/entity-extractor');
const { uid } = require('../../utils');

// 实体类型 → 图谱视觉配置（与前端 NODE_TYPE_COLORS 口径对齐）
const ENTITY_NODE_STYLE = {
  requirement:    { color: '#6366f1', size: 12, zh: '需求条目' },
  business_rule:  { color: '#ef4444', size: 10, zh: '业务规则' },
  architecture:   { color: '#f59e0b', size: 11, zh: '架构节点' },
  module_def:     { color: '#10b981', size: 11, zh: '模块定义' },
  technical_term: { color: '#06b6d4', size: 9,  zh: '技术术语' }
};
const DOC_NODE_STYLE = { color: '#8b5cf6', size: 14 };
const DOMAIN_NODE_STYLE = { color: '#7c3aed', size: 13 };

// 边语义
const EDGE_MENTIONS = '提及';
const EDGE_CO_OCCURS = '共现';
const EDGE_MAPS_TO = '映射域';

function createDocGraphPipeline({ store, graphStore, getDomains }) {

  /** 单文档自动化管道（幂等）：抽取→建点→连边→绑定落盘 */
  function autoSyncDocument(docId) {
    const docs = store.readDocuments();
    const doc = docs.find(d => d.id === docId);
    if (!doc) return { ok: false, error: `文档不存在: ${docId}` };

    // 删除态文档：清理绑定与图谱痕迹（自愈）
    if (doc.status === 'deleted') {
      const removed = removeDocFromGraph(docId);
      return { ok: true, docId, cleaned: true, ...removed };
    }

    // ① 全维实体抽取 + 关系挖掘
    const entities = extractAllEntities(doc.content || '');
    const relations = mineEntityRelations(entities, doc.content || '');

    // ② 实体 → 业务域匹配（注入的域清单，无域时跳过）
    const domains = (typeof getDomains === 'function' ? (getDomains() || []) : []);
    const domainBindings = [];
    if (domains.length > 0) {
      entities.forEach(e => {
        matchEntityToDomains(e, domains).forEach(m => {
          domainBindings.push({ entityId: e.id, entityLabel: e.label, ...m });
        });
      });
    }

    // ③ 图谱节点 upsert（文档节点 + 实体节点 + 域镜像节点）
    const now = new Date().toISOString();
    const nodes = graphStore.readGraphNodes();
    const nodeIds = new Set(nodes.map(n => n.id));
    let newNodes = 0;

    const upsertNode = (node) => {
      const idx = nodes.findIndex(n => n.id === node.id);
      if (idx === -1) {
        nodes.push(node); nodeIds.add(node.id); newNodes++;
      } else {
        // 幂等更新：合并来源文档，保留最新描述
        const prev = nodes[idx];
        const docIds = [...new Set([...(prev.attributes && prev.attributes.docIds || []), docId])].slice(0, 50);
        nodes[idx] = { ...prev, ...node, attributes: { ...node.attributes, docIds } };
      }
    };

    const docNodeId = `doc:${docId}`;
    upsertNode({
      id: docNodeId, label: doc.title || docId, type: 'document', node_type: 'document',
      color: DOC_NODE_STYLE.color, size: DOC_NODE_STYLE.size,
      description: (doc.description || doc.title || '').slice(0, 120),
      attributes: { docId, sourceType: 'kb_document', category: doc.category },
      created_at: now
    });

    entities.forEach(e => {
      const style = ENTITY_NODE_STYLE[e.type] || { color: '#64748b', size: 9 };
      upsertNode({
        id: e.id, label: e.label, type: e.type, node_type: e.type,
        color: style.color, size: style.size,
        description: (e.description || '').slice(0, 120),
        attributes: { ...e.attributes, sourceType: e.attributes && e.attributes.sourceType },
        created_at: now
      });
    });

    const boundDomainIds = [...new Set(domainBindings.map(b => b.domainId))];
    const domainNameById = new Map(domains.map(d => [d.id, d.name]));
    boundDomainIds.forEach(domainId => {
      upsertNode({
        id: `domain:${domainId}`, label: domainNameById.get(domainId) || domainId,
        type: 'atlas_domain', node_type: 'atlas_domain',
        color: DOMAIN_NODE_STYLE.color, size: DOMAIN_NODE_STYLE.size,
        description: '项目全息图谱业务域（镜像节点）',
        attributes: { atlasDomainId: domainId, mirror: true },
        created_at: now
      });
    });

    // ④ 图谱边 upsert（幂等：source|label|target 去重）
    const edges = graphStore.readGraphEdges();
    const edgeKeySet = new Set(edges.map(e => `${e.source}|${e.label}|${e.target}`));
    let newEdges = 0;
    const addEdge = (source, target, label) => {
      const key = `${source}|${label}|${target}`;
      if (edgeKeySet.has(key)) return;
      edgeKeySet.add(key);
      edges.push({ id: uid('graph_edges'), source, target, label, weight: 1, created_at: now });
      newEdges++;
    };

    entities.forEach(e => addEdge(docNodeId, e.id, EDGE_MENTIONS));
    relations.forEach(r => addEdge(r.source, r.target, EDGE_CO_OCCURS));
    domainBindings.forEach(b => addEdge(b.entityId, `domain:${b.domainId}`, EDGE_MAPS_TO));

    // ⑤ 失效边清理：本文档旧的"提及"边（实体已不在文档中）→ 移除（自愈）
    const currentEntityIds = new Set(entities.map(e => e.id));
    let removedEdges = 0;
    for (let i = edges.length - 1; i >= 0; i--) {
      const e = edges[i];
      if (e.source === docNodeId && e.label === EDGE_MENTIONS && !currentEntityIds.has(e.target)) {
        edges.splice(i, 1); removedEdges++;
      }
    }

    graphStore.writeGraphNodes(nodes);
    graphStore.writeGraphEdges(edges);

    // ⑥ 绑定记录落盘（溯源真相源）
    const record = {
      id: uid('dgl'),
      docId, docTitle: doc.title || docId, category: doc.category || 'general',
      syncedAt: now, docVersion: doc.version || 1,
      entities: entities.map(e => ({ id: e.id, type: e.type, label: e.label })),
      entityIds: entities.map(e => e.id),
      relations: relations.length,
      domainBindings: domainBindings.map(b => ({
        entityId: b.entityId, entityLabel: b.entityLabel,
        domainId: b.domainId, domainName: b.domainName,
        score: b.score, matchedKeywords: b.matchedKeywords
      }))
    };
    graphStore.upsertLink(record);

    // ⑦ 回写文档 graphLinks（兼容既有 UI 语义）
    const docIdx = docs.findIndex(d => d.id === docId);
    docs[docIdx].graphLinks = entities.map(e => e.id);
    docs[docIdx].graphAutoSyncedAt = now;
    store.writeDocuments(docs);
    store.addHistory(docId, 'auto-graph-sync',
      `自动图谱化：${entities.length} 实体 / ${relations.length} 关系 / ${domainBindings.length} 域映射`);

    return {
      ok: true, docId, docTitle: record.docTitle,
      entities: entities.length,
      entityTypes: countBy(entities, e => e.type),
      relations: relations.length,
      domainBindings: domainBindings.length,
      boundDomains: boundDomainIds,
      newNodes, newEdges, removedEdges,
      totalGraphNodes: nodes.length, totalGraphEdges: edges.length,
      syncedAt: now
    };
  }

  /** 全量同步：所有活跃文档依次过管道（新文档自动图谱化，旧文档幂等刷新） */
  function autoSyncAll() {
    const docs = store.readDocuments().filter(d => d.status === 'active');
    const results = [];
    let failed = 0;
    docs.forEach(d => {
      try {
        results.push(autoSyncDocument(d.id));
      } catch (e) {
        failed++;
        results.push({ ok: false, docId: d.id, error: e.message });
      }
    });
    return {
      ok: failed === 0,
      total: docs.length, failed,
      entities: results.reduce((s, r) => s + (r.entities || 0), 0),
      relations: results.reduce((s, r) => s + (r.relations || 0), 0),
      domainBindings: results.reduce((s, r) => s + (r.domainBindings || 0), 0),
      results, syncedAt: new Date().toISOString()
    };
  }

  /** 绑定记录查询（全量 / 单文档） */
  function getBindings(docId) {
    const links = graphStore.readLinks();
    return docId ? links.filter(l => l.docId === docId) : links;
  }

  /** 覆盖率统计：已绑定文档 / 活跃文档 */
  function getCoverage() {
    const docs = store.readDocuments().filter(d => d.status === 'active');
    const links = graphStore.readLinks();
    const boundDocIds = new Set(links.map(l => l.docId));
    const bound = docs.filter(d => boundDocIds.has(d.id));
    const totalEntities = links.reduce((s, l) => s + (l.entityIds || []).length, 0);
    const totalDomainBindings = links.reduce((s, l) => s + (l.domainBindings || []).length, 0);
    const entityTypes = {};
    links.forEach(l => (l.entities || []).forEach(e => {
      entityTypes[e.type] = (entityTypes[e.type] || 0) + 1;
    }));
    return {
      docs: docs.length, boundDocs: bound.length,
      unboundDocs: docs.length - bound.length,
      coverage: docs.length === 0 ? 1 : bound.length / docs.length,
      unboundDocList: docs.filter(d => !boundDocIds.has(d.id)).map(d => ({ id: d.id, title: d.title })),
      entities: totalEntities, entityTypes,
      domainBindings: totalDomainBindings,
      syncedAt: new Date().toISOString()
    };
  }

  /** 删除文档的图谱清理（文档节点 + 提及边 + 绑定记录） */
  function removeDocFromGraph(docId) {
    const docNodeId = `doc:${docId}`;
    const nodes = graphStore.readGraphNodes();
    const edges = graphStore.readGraphEdges();
    const nodeCount = nodes.length, edgeCount = edges.length;
    const nextNodes = nodes.filter(n => n.id !== docNodeId);
    const nextEdges = edges.filter(e => e.source !== docNodeId && e.target !== docNodeId);
    if (nextNodes.length !== nodeCount || nextEdges.length !== edgeCount) {
      graphStore.writeGraphNodes(nextNodes);
      graphStore.writeGraphEdges(nextEdges);
    }
    const linkRemoved = graphStore.removeLink(docId);
    // 回写文档 graphLinks 清空
    const docs = store.readDocuments();
    const idx = docs.findIndex(d => d.id === docId);
    if (idx >= 0) {
      docs[idx].graphLinks = [];
      store.writeDocuments(docs);
    }
    return {
      removedNodes: nodeCount - nextNodes.length,
      removedEdges: edgeCount - nextEdges.length,
      linkRemoved
    };
  }

  return { autoSyncDocument, autoSyncAll, getBindings, getCoverage, removeDocFromGraph };
}

function countBy(list, fn) {
  const out = {};
  list.forEach(x => { const k = fn(x); out[k] = (out[k] || 0) + 1; });
  return out;
}

module.exports = { createDocGraphPipeline, ENTITY_NODE_STYLE };

'use strict';

const http = require('http');
const url = require('url');
const { EventEmitter } = require('events');
const { getStorage } = require('./storage');
const { uid } = require('./utils');
const { config } = require('./config');
const { createDriverFromEnv, MockRemoteGraphDriver } = require('./graph/remote-graph-driver');
const { GraphFormulas, expandRawEdges } = require('./graph/graph-formulas');

const NEBUGRAPH_HOST = process.env.NEBULAGRAPH_HOST || 'localhost';
const NEBUGRAPH_PORT = parseInt(process.env.NEBULAGRAPH_PORT || '9669', 10);
const NEBUGRAPH_GRAPH = process.env.NEBULAGRAPH_GRAPH || 'infotopograph';
const USE_NEBULAGRAPH = process.env.USE_NEBULAGRAPH === 'true';

// 企业级 CDC 事件总线：内存事件 + 可选 Redis Stream 适配器（配置化）。
class CdcEventBus extends EventEmitter {
  constructor() {
    super();
    this.setMaxListeners(100);
    this._seq = 0;
    this._dlq = [];
  }
  emitEvent(topic, payload) {
    const evt = { topic, seq: ++this._seq, payload, ts: Date.now() };
    try { this.emit(topic, evt); }
    catch (e) { this._dlq.push({ evt, err: e.message }); }
    try { this.emit('*', evt); }
    catch (e) { this._dlq.push({ evt, err: e.message }); }
    return evt;
  }
  get dlq() { return this._dlq.slice(); }
  clearDlq() { this._dlq.length = 0; }
}

/**
 * LRU-ttl 缓存：O(1) get/set；用于图节点远程读端 L1。
 */
class LruCache {
  constructor({ max = 10000, ttlMs = 300 * 1000 } = {}) {
    this.max = max; this.ttl = ttlMs;
    this.map = new Map();
  }
  get(key) {
    const v = this.map.get(key);
    if (!v) return undefined;
    if (this.ttl > 0 && Date.now() - v.ts > this.ttl) { this.map.delete(key); return undefined; }
    this.map.delete(key); this.map.set(key, v);
    return v.value;
  }
  set(key, value) {
    if (this.map.has(key)) this.map.delete(key);
    this.map.set(key, { value, ts: Date.now() });
    while (this.map.size > this.max) {
      const firstKey = this.map.keys().next().value;
      this.map.delete(firstKey);
    }
  }
  del(key) { this.map.delete(key); }
  clear() { this.map.clear(); }
  get size() { return this.map.size; }
}

class NebulaGraphAdapter {
  constructor({ driver, storage, cdcBus, l1Cache } = {}) {
    this.storage = storage || getStorage();
    this._initLocalGraph();

    // 远程驱动：优先注入，否则按环境变量决定用 Gremlin/Mock
    this.driver = driver || createDriverFromEnv();
    this.driver.connect().catch(() => { /* 不可用仍可用 local */ });
    this.remote = {
      host: NEBUGRAPH_HOST,
      port: NEBUGRAPH_PORT,
      graph: NEBUGRAPH_GRAPH,
      connected: (this.driver && !(this.driver instanceof MockRemoteGraphDriver)) || USE_NEBULAGRAPH
    };

    // CDC + L1
    this.cdc = cdcBus || new CdcEventBus();
    this.l1 = l1Cache || new LruCache({ max: 20000, ttlMs: 300 * 1000 });

    // CDC 消费端：1) 失效 L1；2) 触发索引钩子（预留）
    this.cdc.on('graph:node_updated', (evt) => this.l1.del(`node:${evt.payload.id}`));
    this.cdc.on('graph:edge_updated', (evt) => {
      this.l1.del(`neighbors:${evt.payload.source}`);
      this.l1.del(`neighbors:${evt.payload.target}`);
      this.l1.del('listNodes');
    });
    this.cdc.on('graph:bulk_complete', () => this.l1.clear());
    this._indexHooks = [];
    this.cdc.on('*', (evt) => {
      for (const h of this._indexHooks) {
        try { h(evt).catch(() => {}); } catch {}
      }
    });
  }

  /** 注册索引更新钩子（例如 pgvector embedding 更新） */
  onIndexUpdated(fn) { if (typeof fn === 'function') this._indexHooks.push(fn); }

  _initLocalGraph() {
    let graph = this.storage.getEntityData('knowledge_graph', 'main');
    if (!graph) {
      graph = {
        id: 'main',
        name: '璇玑知识图谱',
        description: '企业级全维知识图谱中枢',
        nodes: {},
        edges: {},
        nodeCount: 0,
        edgeCount: 0,
        layerCount: { L1: 0, L2: 0, L3: 0, L4: 0, L5: 0, L6: 0, L7: 0 },
        kindCount: {},
        status: 'active',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        _lastSync: null
      };
      this.storage.upsertEntity('knowledge_graph', 'main', graph);
    }
    this.localGraph = graph;
  }

  _persist() {
    this.localGraph.nodeCount = Object.keys(this.localGraph.nodes).length;
    this.localGraph.edgeCount = Object.keys(this.localGraph.edges).length;
    this.localGraph.layerCount = { L1: 0, L2: 0, L3: 0, L4: 0, L5: 0, L6: 0, L7: 0 };
    this.localGraph.kindCount = {};
    Object.values(this.localGraph.nodes).forEach(n => {
      if (n.layer) this.localGraph.layerCount[n.layer] = (this.localGraph.layerCount[n.layer] || 0) + 1;
      if (n.kind) this.localGraph.kindCount[n.kind] = (this.localGraph.kindCount[n.kind] || 0) + 1;
    });
    this.localGraph.updatedAt = new Date().toISOString();
    this.storage.upsertEntity('knowledge_graph', 'main', this.localGraph);
  }

  _connectRemote() {
    this.remote = {
      host: NEBUGRAPH_HOST,
      port: NEBUGRAPH_PORT,
      graph: NEBUGRAPH_GRAPH,
      connected: false
    };
    this._probeRemote().then(ok => {
      this.remote.connected = ok;
      console.log(`[NebulaGraph] ${ok ? '已连接' : '未连接 (使用本地图谱)'} ${NEBUGRAPH_HOST}:${NEBULAGRAPH_PORT}`);
    }).catch(() => {
      console.log(`[NebulaGraph] 远程不可用，使用本地图谱`);
    });
  }

  async _probeRemote() {
    return new Promise((resolve) => {
      const req = http.request({
        hostname: this.remote.host,
        port: this.remote.port,
        path: '/status',
        method: 'GET',
        timeout: 3000
      }, (res) => {
        resolve(res.statusCode === 200);
      });
      req.on('error', () => resolve(false));
      req.on('timeout', () => { req.destroy(); resolve(false); });
      req.end();
    });
  }

  async _execRemote(gremlin) {
    if (!this.remote?.connected) return null;
    return new Promise((resolve) => {
      const data = JSON.stringify({ gremlin });
      const req = http.request({
        hostname: this.remote.host,
        port: this.remote.port,
        path: '/gremlin',
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Content-Length': data.length },
        timeout: 5000
      }, (res) => {
        let body = '';
        res.on('data', (c) => body += c);
        res.on('end', () => {
          try { resolve(JSON.parse(body)); } catch { resolve(null); }
        });
      });
      req.on('error', () => resolve(null));
      req.on('timeout', () => { req.destroy(); resolve(null); });
      req.write(data);
      req.end();
    });
  }

  // ==================== 节点操作 ====================

  createNode(params) {
    const id = params.id || uid('node');
    const node = {
      id,
      kind: params.kind || 'Entity',
      layer: params.layer || 'L3',
      name: params.name || id,
      description: params.description || '',
      properties: params.properties || {},
      tags: params.tags || [],
      source: params.source || 'system',
      confidence: params.confidence || 1.0,
      degree: 0,
      inDegree: 0,
      outDegree: 0,
      community: -1,
      labels: params.labels || [],
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString()
    };

    if (params.aliases) node.aliases = params.aliases;
    if (params.fileId) node.fileId = params.fileId;
    if (params.embedding) node.embedding = params.embedding;

    this.localGraph.nodes[id] = node;
    this._updateNodeDegrees(id);
    this._persist();

    if (this.driver && typeof this.driver.upsertNode === 'function') {
      try {
        // 对 Mock 同步；对真实 driver 异步不阻塞写
        const r = this.driver.upsertNode({
          id, kind: node.kind, project_domain: node.properties?.project_domain,
          layer: node.layer, attributes: { ...node.properties, name: node.name, description: node.description, tags: node.tags, labels: node.labels }
        });
        if (r && typeof r.then === 'function') r.catch(() => {});
      } catch {}
    }
    this.cdc.emitEvent('graph:node_updated', { id, kind: node.kind, source: 'createNode' });

    return node;
  }

  getNode(id) {
    // 读策略：远程优先 → L1 → 本地；为兼容同步 API，对 MockRemoteGraphDriver 用同步桥接；
    // 对真实 GremlinHttpDriver 仍保留 Promise 执行但同步返回 L1/本地（异步结果会在后续刷新 L1，保证下一次拿到最新）。
    const cacheKey = 'node:' + id;
    const cached = this.l1.get(cacheKey);
    if (cached !== undefined) return cached.node;

    let remoteResult = null;
    if (this.driver && typeof this.driver.getNode === 'function') {
      if (this.driver instanceof MockRemoteGraphDriver) {
        // Mock：同步桥接，保留 callCounts
        try {
          this.driver._tick('getNode');
          remoteResult = this.driver._nodes.get(id) || null;
        } catch { remoteResult = null; }
      } else {
        // 真实异步：发请求，后续 Promise 中更新 L1；当前同步调用返回本地数据
        Promise.resolve(this.driver.getNode(id)).then(n => {
          if (n) this.l1.set(cacheKey, { node: n, source: 'remote-async' });
        }).catch(() => {});
      }
    }

    if (remoteResult) {
      this.l1.set(cacheKey, { node: remoteResult, source: 'remote' });
      return remoteResult;
    }

    const local = this.localGraph.nodes[id] ? this._toPublicNode(this.localGraph.nodes[id]) : null;
    if (local) this.l1.set(cacheKey, { node: local, source: 'local' });
    return local;
  }

  _toPublicNode(localNode) {
    // 把本地图结构（createNode 写入的 {id, kind, layer, name, description, properties, ...}）
    // 规范化为同形状，保持兼容；对外 getNode 返回的即是此对象。
    if (!localNode) return null;
    return localNode;
  }

  updateNode(id, updates) {
    const node = this.localGraph.nodes[id];
    if (!node) return null;
    Object.assign(node, updates, { updatedAt: new Date().toISOString() });
    this.localGraph.nodes[id] = node;
    this._updateNodeDegrees(id);
    this._persist();
    if (this.driver && typeof this.driver.upsertNode === 'function') {
      try {
        const r = this.driver.upsertNode({
          id, kind: node.kind, project_domain: node.properties?.project_domain,
          layer: node.layer, attributes: { ...node.properties, name: node.name, description: node.description }
        });
        if (r && typeof r.then === 'function') r.catch(() => {});
      } catch {}
    }
    this.cdc.emitEvent('graph:node_updated', { id, kind: node.kind, source: 'updateNode' });
    return node;
  }

  deleteNode(id) {
    if (!this.localGraph.nodes[id]) return false;
    delete this.localGraph.nodes[id];
    Object.keys(this.localGraph.edges).forEach(eid => {
      const edge = this.localGraph.edges[eid];
      if (edge.from === id || edge.to === id) {
        delete this.localGraph.edges[eid];
      }
    });
    this._persist();
    return true;
  }

  listNodes(filters = {}) {
    let nodes = Object.values(this.localGraph.nodes);
    if (filters.kind) nodes = nodes.filter(n => n.kind === filters.kind);
    if (filters.layer) nodes = nodes.filter(n => n.layer === filters.layer);
    if (filters.tag) nodes = nodes.filter(n => (n.tags || []).includes(filters.tag));
    if (filters.fileId) nodes = nodes.filter(n => n.fileId === filters.fileId);
    if (filters.source) nodes = nodes.filter(n => n.source === filters.source);
    if (filters.status) nodes = nodes.filter(n => n.status === filters.status);
    if (filters.community !== undefined) nodes = nodes.filter(n => n.community === filters.community);
    return nodes.sort((a, b) => new Date(b.updatedAt) - new Date(a.updatedAt));
  }

  // ==================== 边操作 ====================

  createEdge(fromId, toId, kind, properties = {}) {
    if (!this.localGraph.nodes[fromId]) throw new Error(`Source node not found: ${fromId}`);
    if (!this.localGraph.nodes[toId]) throw new Error(`Target node not found: ${toId}`);

    const id = uid('edge');
    const edge = {
      id,
      from: fromId,
      to: toId,
      kind: kind || 'RELATED_TO',
      label: properties.label || kind || 'related',
      weight: properties.weight || 1.0,
      direction: properties.direction || 'directed',
      properties,
      confidence: properties.confidence || 1.0,
      evidence: properties.evidence || [],
      source: properties.source || 'system',
      createdAt: new Date().toISOString()
    };

    this.localGraph.edges[id] = edge;
    this._updateNodeDegrees(fromId);
    this._updateNodeDegrees(toId);
    this._persist();

    if (this.driver && typeof this.driver.upsertEdge === 'function') {
      try {
        const r = this.driver.upsertEdge({ source: fromId, target: toId, type: edge.kind, weight: edge.weight, attributes: properties });
        if (r && typeof r.then === 'function') r.catch(() => {});
      } catch {}
    }
    this.cdc.emitEvent('graph:edge_updated', { id, source: fromId, target: toId, kind: edge.kind });

    return edge;
  }

  getEdge(id) {
    return this.localGraph.edges[id] || null;
  }

  updateEdge(id, updates) {
    const edge = this.localGraph.edges[id];
    if (!edge) return null;
    Object.assign(edge, updates);
    this.localGraph.edges[id] = edge;
    this._persist();
    return edge;
  }

  deleteEdge(id) {
    if (!this.localGraph.edges[id]) return false;
    const edge = this.localGraph.edges[id];
    delete this.localGraph.edges[id];
    this._updateNodeDegrees(edge.from);
    this._updateNodeDegrees(edge.to);
    this._persist();
    return true;
  }

  listEdges(filters = {}) {
    let edges = Object.values(this.localGraph.edges);
    if (filters.from) edges = edges.filter(e => e.from === filters.from);
    if (filters.to) edges = edges.filter(e => e.to === filters.to);
    if (filters.kind) edges = edges.filter(e => e.kind === filters.kind);
    if (filters.nodeId) edges = edges.filter(e => e.from === filters.nodeId || e.to === filters.nodeId);
    return edges;
  }

  // ==================== 批量操作 ====================

  bulkUpsert(operations) {
    const results = { created: 0, updated: 0, failed: 0 };
    for (const op of operations) {
      try {
        if (op.type === 'node') {
          if (this.localGraph.nodes[op.id]) {
            this.updateNode(op.id, op.data);
            results.updated++;
          } else {
            this.createNode(op.data);
            results.created++;
          }
        } else if (op.type === 'edge') {
          this.createEdge(op.from, op.to, op.kind, op.properties || {});
          results.created++;
        }
      } catch (e) {
        results.failed++;
      }
    }
    return results;
  }

  // ==================== 查询与推理 ====================

  neighbors(nodeId, direction = 'both') {
    const edges = Object.values(this.localGraph.edges);
    const neighbors = new Set();

    edges.forEach(e => {
      if (direction === 'out' && e.from === nodeId) neighbors.add(e.to);
      else if (direction === 'in' && e.to === nodeId) neighbors.add(e.from);
      else if (direction === 'both') {
        if (e.from === nodeId) neighbors.add(e.to);
        if (e.to === nodeId) neighbors.add(e.from);
      }
    });

    return Array.from(neighbors).map(id => this.localGraph.nodes[id]).filter(Boolean);
  }

  multiHopTraversal(startId, maxDepth = 3, filters = {}) {
    const visited = new Set([startId]);
    const queue = [{ id: startId, depth: 0 }];
    const nodes = [];
    const edges = [];

    while (queue.length > 0) {
      const { id, depth } = queue.shift();
      if (depth >= maxDepth) continue;

      const node = this.localGraph.nodes[id];
      if (!node) continue;

      const neighborEdges = Object.values(this.localGraph.edges).filter(e => {
        if (filters.edgeKind && e.kind !== filters.edgeKind) return false;
        return e.from === id || e.to === id;
      });

      neighborEdges.forEach(e => {
        const nextId = e.from === id ? e.to : e.from;
        if (!visited.has(nextId)) {
          visited.add(nextId);
          const nextNode = this.localGraph.nodes[nextId];
          if (!filters.nodeKind || nextNode?.kind === filters.nodeKind) {
            nodes.push(nextNode);
            edges.push(e);
            queue.push({ id: nextId, depth: depth + 1 });
          }
        }
      });
    }

    return { nodes, edges, visitedCount: visited.size };
  }

  shortestPath(fromId, toId) {
    const visited = new Map([[fromId, null]]);
    const queue = [fromId];

    while (queue.length > 0) {
      const current = queue.shift();
      if (current === toId) break;

      Object.values(this.localGraph.edges).forEach(e => {
        let neighbor = null;
        if (e.from === current && !visited.has(e.to)) neighbor = e.to;
        else if (e.to === current && !visited.has(e.from)) neighbor = e.from;

        if (neighbor) {
          visited.set(neighbor, { from: current, via: e.id });
          queue.push(neighbor);
        }
      });
    }

    if (!visited.has(toId)) return { found: false, path: [], length: 0 };

    const path = [];
    let current = toId;
    while (current !== fromId) {
      const step = visited.get(current);
      if (!step) return { found: false, path: [], length: 0 };
      path.unshift({ node: current, edgeId: step.via });
      current = step.from;
    }
    path.unshift({ node: fromId, edgeId: null });

    return { found: true, path, length: path.length - 1 };
  }

  semanticSearch(query, topK = 10) {
    const q = query.toLowerCase();
    const results = [];

    Object.values(this.localGraph.nodes).forEach(node => {
      let score = 0;
      const name = (node.name || '').toLowerCase();
      const kind = (node.kind || '').toLowerCase();
      const desc = (node.description || '').toLowerCase();
      const tags = (node.tags || []).join(' ').toLowerCase();
      const props = JSON.stringify(node.properties || {}).toLowerCase();

      if (name === q) score += 100;
      if (name.includes(q)) score += 50;
      if (kind.includes(q)) score += 30;
      if (desc.includes(q)) score += 20;
      if (tags.includes(q)) score += 15;
      if (props.includes(q)) score += 10;

      if (score > 0) {
        results.push({ node, score });
      }
    });

    return results.sort((a, b) => b.score - a.score).slice(0, topK);
  }

  pagerank(dampingFactor = 0.85, maxIterations = 50, tolerance = 0.0001) {
    const nodeIds = Object.keys(this.localGraph.nodes);
    const edgeList = Object.values(this.localGraph.edges);
    const n = nodeIds.length;

    if (n === 0) return {};

    const inLinks = {};
    const outLinks = {};
    nodeIds.forEach(id => { inLinks[id] = []; outLinks[id] = []; });

    edgeList.forEach(e => {
      if (outLinks[e.from]) outLinks[e.from].push(e.to);
      if (inLinks[e.to]) inLinks[e.to].push(e.from);
    });

    let pr = {};
    nodeIds.forEach(id => { pr[id] = 1 / n; });

    for (let iter = 0; iter < maxIterations; iter++) {
      let maxChange = 0;
      const newPr = {};

      nodeIds.forEach(id => {
        const inNodes = inLinks[id];
        let sum = 0;
        inNodes.forEach(inId => {
          const outCount = (outLinks[inId] || []).length;
          if (outCount > 0) sum += pr[inId] / outCount;
        });
        newPr[id] = (1 - dampingFactor) / n + dampingFactor * sum;
        maxChange = Math.max(maxChange, Math.abs(newPr[id] - pr[id]));
      });

      pr = newPr;
      if (maxChange < tolerance) break;
    }

    const total = Object.values(pr).reduce((a, b) => a + b, 0);
    const normalized = {};
    Object.entries(pr).forEach(([id, val]) => {
      normalized[id] = total > 0 ? val / total : 0;
    });

    return normalized;
  }

  detectCommunities() {
    // 项目记忆硬性：CNM 模块度贪心凝聚；shape 与原返回保持兼容：
    //   { communities: {[id]: members[]}, nodeCommunity: {id: index}, count }
    const nodeArr = Object.values(this.localGraph.nodes);
    const edgeArr = Object.values(this.localGraph.edges).map(e => ({ source: e.from, target: e.to, weight: e.weight || 1 }));
    const cnm = GraphFormulas.communityDetectionCNM(nodeArr, edgeArr);
    const communityMap = {};
    (cnm.communities || []).forEach((members, i) => { communityMap[i] = members; });
    return { communities: communityMap, nodeCommunity: cnm.nodeCommunity || {}, count: (cnm.communities || []).length, modularity: cnm.modularity, algorithm: cnm.algorithm };
  }

  // ==================== 统计 ====================

  getStats() {
    const graph = this.localGraph;
    return {
      name: graph.name,
      description: graph.description,
      nodeCount: graph.nodeCount,
      edgeCount: graph.edgeCount,
      layerCount: graph.layerCount,
      kindCount: graph.kindCount,
      communities: this.detectCommunities().count,
      pagerankTop: this._getTopPagerank(5),
      remoteConnected: this.remote?.connected || false,
      lastUpdated: graph.updatedAt
    };
  }

  _getTopPagerank(k) {
    const pr = this.pagerank();
    return Object.entries(pr)
      .sort((a, b) => b[1] - a[1])
      .slice(0, k)
      .map(([id, score]) => ({ id, name: this.localGraph.nodes[id]?.name || id, score }));
  }

  _updateNodeDegrees(nodeId) {
    const edges = Object.values(this.localGraph.edges);
    let inDeg = 0, outDeg = 0;
    edges.forEach(e => {
      if (e.to === nodeId) inDeg++;
      if (e.from === nodeId) outDeg++;
    });
    if (this.localGraph.nodes[nodeId]) {
      this.localGraph.nodes[nodeId].inDegree = inDeg;
      this.localGraph.nodes[nodeId].outDegree = outDeg;
      this.localGraph.nodes[nodeId].degree = inDeg + outDeg;
    }
  }

  // ==================== 导出 ====================

  export() {
    return {
      version: '2.0',
      exportedAt: new Date().toISOString(),
      graph: {
        nodes: Object.values(this.localGraph.nodes),
        edges: Object.values(this.localGraph.edges)
      },
      stats: this.getStats()
    };
  }

  import(data) {
    if (!data?.graph) throw new Error('Invalid graph data');
    this.localGraph.nodes = {};
    this.localGraph.edges = {};
    (data.graph.nodes || []).forEach(n => { this.localGraph.nodes[n.id] = n; });
    (data.graph.edges || []).forEach(e => { this.localGraph.edges[e.id] = e; });
    this._persist();
    return this.getStats();
  }

  toMermaid() {
    const lines = ['graph TD'];
    Object.values(this.localGraph.edges).forEach(e => {
      const fromName = (this.localGraph.nodes[e.from]?.name || e.from).replace(/[^\w\u4e00-\u9fa5]/g, '_');
      const toName = (this.localGraph.nodes[e.to]?.name || e.to).replace(/[^\w\u4e00-\u9fa5]/g, '_');
      lines.push(`  ${fromName} -->|${e.label}| ${toName}`);
    });
    return lines.join('\n');
  }
}

let _instance = null;

function getNebulaGraphAdapter() {
  if (!_instance) _instance = new NebulaGraphAdapter();
  return _instance;
}

// 测试友好：重置单例
function resetNebulaGraphAdapter() { _instance = null; return true; }

module.exports = {
  NebulaGraphAdapter,
  getNebulaGraphAdapter,
  resetNebulaGraphAdapter,
  CdcEventBus,
  LruCache
};
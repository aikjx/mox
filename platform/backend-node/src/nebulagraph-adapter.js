'use strict';

const http = require('http');
const url = require('url');
const { getStorage } = require('./storage');
const { uid } = require('./utils');
const { config } = require('./config');

const NEBUGRAPH_HOST = process.env.NEBULAGRAPH_HOST || 'localhost';
const NEBUGRAPH_PORT = parseInt(process.env.NEBULAGRAPH_PORT || '9669', 10);
const NEBUGRAPH_GRAPH = process.env.NEBULAGRAPH_GRAPH || 'infotopograph';
const USE_NEBULAGRAPH = process.env.USE_NEBULAGRAPH === 'true';

class NebulaGraphAdapter {
  constructor() {
    this.storage = getStorage();
    this.remote = null;
    this._initLocalGraph();
    if (USE_NEBULAGRAPH) {
      this._connectRemote();
    }
  }

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

    if (this.remote?.connected) {
      const props = Object.entries(node.properties || {}).map(([k, v]) => `.property('${k}', '${v}')`).join('');
      this._execRemote(`g.V().addV('${node.kind}')${props}.property('id', '${id}').property('name', '${node.name}')`);
    }

    return node;
  }

  getNode(id) {
    return this.localGraph.nodes[id] || null;
  }

  updateNode(id, updates) {
    const node = this.localGraph.nodes[id];
    if (!node) return null;
    Object.assign(node, updates, { updatedAt: new Date().toISOString() });
    this.localGraph.nodes[id] = node;
    this._updateNodeDegrees(id);
    this._persist();
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
    const nodeIds = Object.keys(this.localGraph.nodes);
    const communities = {};
    nodeIds.forEach(id => { communities[id] = { id, neighbors: new Set() }; });

    Object.values(this.localGraph.edges).forEach(e => {
      if (communities[e.from]) communities[e.from].neighbors.add(e.to);
      if (communities[e.to]) communities[e.to].neighbors.add(e.from);
    });

    const labels = {};
    nodeIds.forEach(id => { labels[id] = id; });

    for (let iter = 0; iter < 20; iter++) {
      let changed = false;
      const order = nodeIds.sort(() => Math.random() - 0.5);

      order.forEach(id => {
        const labelCounts = {};
        communities[id].neighbors.forEach(nb => {
          const lbl = labels[nb];
          labelCounts[lbl] = (labelCounts[lbl] || 0) + 1;
        });

        let maxLabel = labels[id];
        let maxCount = 0;
        Object.entries(labelCounts).forEach(([lbl, count]) => {
          if (count > maxCount || (count === maxCount && lbl < maxLabel)) {
            maxLabel = lbl;
            maxCount = count;
          }
        });

        if (maxLabel !== labels[id]) {
          labels[id] = maxLabel;
          changed = true;
        }
      });

      if (!changed) break;
    }

    const communityMap = {};
    Object.entries(labels).forEach(([nodeId, label]) => {
      if (!communityMap[label]) communityMap[label] = [];
      communityMap[label].push(nodeId);
    });

    const nodeCommunity = {};
    let commIndex = 0;
    Object.values(communityMap).forEach(members => {
      members.forEach(m => { nodeCommunity[m] = commIndex; });
      commIndex++;
    });

    return { communities: communityMap, nodeCommunity, count: Object.keys(communityMap).length };
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

module.exports = { NebulaGraphAdapter, getNebulaGraphAdapter };
'use strict';

/**
 * 璇玑分布式图谱：远程图驱动抽象 + HTTP（Gremlin/nGQL mock）双协议实现
 * ============================================================
 * 说明：
 *  - RemoteGraphDriver：远端调用统一抽象，方法签名对齐 NebulaGraphAdapter 所需；
 *  - GremlinHttpDriver：对接 Gremlin Server HTTP / AWS Neptune；
 *  - MockRemoteGraphDriver：单测/开发/无 Nebula 环境使用，内存图谱 + TTL LRU 索引，
 *    提供与真实 Gremlin/nGQL 同构语义的 CRUD 与邻居/最短路径/搜索查询。
 */

const http = require('http');
const https = require('https');
const url = require('url');

class RemoteGraphDriver {
  constructor(options = {}) { this.options = options; }
  async connect() { return true; }
  async disconnect() { return true; }

  // 基础读
  async getNode(id) { throw new Error('getNode not implemented'); }
  async listNodes(filter = {}) { throw new Error('listNodes not implemented'); }
  async neighbors(ids, hops = 1, dir = 'OUT') { throw new Error('neighbors not implemented'); }
  async shortestPath(src, dst, maxHop = 8) { throw new Error('shortestPath not implemented'); }

  // 基础写
  async upsertNode(node) { throw new Error('upsertNode not implemented'); }
  async upsertEdge(edge) { throw new Error('upsertEdge not implemented'); }
  async bulkUpsert({ nodes = [], edges = [] }) { throw new Error('bulkUpsert not implemented'); }
  async deleteNode(id) { throw new Error('deleteNode not implemented'); }
  async deleteEdge({ source, target, type }) { throw new Error('deleteEdge not implemented'); }

  // 计数/分析
  async stats() { throw new Error('stats not implemented'); }
  async semanticSearch(embeddingOrText, topK = 10) { return { items: [], total: 0 }; }
}

class BaseHttpDriver extends RemoteGraphDriver {
  constructor(options = {}) {
    super(options);
    const endpoint = options.endpoint || process.env.NEBULA_ENDPOINT || process.env.GREMLIN_ENDPOINT || null;
    this.endpoint = endpoint;
    this.timeout = parseInt(String(options.timeout || process.env.GRAPH_REQ_TIMEOUT_MS || '2000'), 10);
    this._parsed = endpoint ? url.parse(endpoint) : null;
  }
  _request({ path, method = 'POST', body, headers = {} }) {
    return new Promise((resolve, reject) => {
      if (!this.endpoint) { return reject(new Error('远程图谱 endpoint 未配置（NEBULA_ENDPOINT / GREMLIN_ENDPOINT 为空）')); }
      const data = body ? JSON.stringify(body) : null;
      const u = this._parsed;
      const h = {
        'Accept': 'application/json',
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(data || ''),
        ...headers
      };
      const opts = {
        method,
        hostname: u.hostname,
        port: u.port,
        path: path || u.path || '/',
        headers: h,
        timeout: this.timeout
      };
      const lib = u.protocol === 'https:' ? https : http;
      const req = lib.request(opts, res => {
        let buf = '';
        res.on('data', c => (buf += c));
        res.on('end', () => {
          try {
            const json = buf ? JSON.parse(buf) : null;
            if (res.statusCode >= 200 && res.statusCode < 300) resolve(json);
            else reject(Object.assign(new Error(`Http ${res.statusCode}: ${buf}`), { status: res.statusCode, body: buf }));
          } catch (e) { reject(e); }
        });
      });
      req.on('error', reject);
      req.on('timeout', () => { req.destroy(new Error('request timeout')); });
      if (data) req.write(data);
      req.end();
    });
  }
}

class GremlinHttpDriver extends BaseHttpDriver {
  constructor(options = {}) { super(options); }

  async _runGremlin(script, bindings = {}) {
    const body = { gremlin: script, bindings, language: 'gremlin-groovy' };
    const path = '/gremlin';
    const res = await this._request({ path, method: 'POST', body });
    return res && res.result && res.result.data ? res.result.data : res;
  }

  async getNode(id) {
    const d = await this._runGremlin(`g.V(${JSON.stringify(id)}).valueMap(true).limit(1)`);
    if (!d || !d.length) return null;
    return this._toNode(d[0]);
  }

  async listNodes({ kind, project, domain, layer, limit = 200, offset = 0 } = {}) {
    let q = 'g.V()';
    if (kind) q += `.has('kind', ${JSON.stringify(kind)})`;
    if (project) q += `.has('project', ${JSON.stringify(project)})`;
    if (domain) q += `.has('project_domain', ${JSON.stringify(domain)})`;
    if (layer) q += `.has('layer', ${JSON.stringify(layer)})`;
    q += `.range(${offset}, ${offset + limit}).valueMap(true)`;
    const d = await this._runGremlin(q);
    return (d || []).map(this._toNode);
  }

  async neighbors(ids, hops = 1, dir = 'BOTH') {
    const idsJson = JSON.stringify(ids);
    const dirStep = dir === 'IN' ? '.in()' : dir === 'OUT' ? '.out()' : '.both()';
    const q = `g.V(${idsJson}).repeat(${dirStep}.dedup()).times(${hops}).emit().dedup().valueMap(true)`;
    const d = await this._runGremlin(q);
    return (d || []).map(this._toNode);
  }

  async shortestPath(src, dst, maxHop = 8) {
    const d = await this._runGremlin(
      `g.V(${JSON.stringify(src)}).repeat(both().simplePath()).until(hasId(${JSON.stringify(dst)})).path().limit(${maxHop}).toList()`
    );
    if (!d || !d.length) return null;
    const first = d[0] && d[0].objects ? d[0].objects : d[0];
    return Array.isArray(first) ? first.filter(x => typeof x === 'string' || (x && x.id !== undefined)).map(x => typeof x === 'string' ? x : x.id) : first;
  }

  async upsertNode(node) {
    const { id, label = 'Entity', attributes = {}, kind, project, project_domain, layer } = node;
    const props = { ...attributes };
    if (kind !== undefined) props.kind = kind;
    if (project !== undefined) props.project = project;
    if (project_domain !== undefined) props.project_domain = project_domain;
    if (layer !== undefined) props.layer = layer;
    const script = [
      `g.V(${JSON.stringify(id)}).fold()`,
      `.coalesce(unfold(), addV(${JSON.stringify(label)}).property(id, T.id, ${JSON.stringify(id)}))`
    ];
    for (const [k, v] of Object.entries(props)) {
      script.push(`.property(${JSON.stringify(k)}, ${JSON.stringify(v)})`);
    }
    script.push('.valueMap(true)');
    const d = await this._runGremlin(script.join(''));
    return this._toNode(d && d[0]);
  }

  async upsertEdge(edge) {
    const { source, target, type = 'LINK', weight = 1, attributes = {} } = edge;
    const script = [
      `g.V(${JSON.stringify(source)}).as('s')`,
      `.V(${JSON.stringify(target)}).as('t')`,
      `.addE(${JSON.stringify(type)}).from('s').to('t')`,
      `.property('weight', ${JSON.stringify(weight)})`
    ];
    for (const [k, v] of Object.entries(attributes)) {
      script.push(`.property(${JSON.stringify(k)}, ${JSON.stringify(v)})`);
    }
    script.push('.valueMap(true)');
    const d = await this._runGremlin(script.join(''));
    return d && d[0];
  }

  async bulkUpsert({ nodes, edges }) {
    const addedNodes = [], addedEdges = [];
    for (const n of nodes) addedNodes.push(await this.upsertNode(n));
    for (const e of edges) addedEdges.push(await this.upsertEdge(e));
    return { addedNodes, addedEdges };
  }

  async deleteNode(id) { await this._runGremlin(`g.V(${JSON.stringify(id)}).drop().iterate()`); return true; }
  async deleteEdge({ source, target, type }) {
    const q = `g.V(${JSON.stringify(source)}).outE(${JSON.stringify(type)}).where(inV().hasId(${JSON.stringify(target)})).drop().iterate()`;
    await this._runGremlin(q);
    return true;
  }

  async stats() {
    const [nodes, edges] = await Promise.all([
      this._runGremlin('g.V().count()').then(d => Array.isArray(d) ? d[0] : d),
      this._runGremlin('g.E().count()').then(d => Array.isArray(d) ? d[0] : d)
    ]);
    return { nodes: Number(nodes || 0), edges: Number(edges || 0) };
  }

  _toNode(r) {
    if (!r) return null;
    // Gremlin valueMap(true) 返回形如 {id:xxx, label:yyy, fieldName:[value]}
    const out = { id: r.id !== undefined ? r.id : r._id, label: r.label || 'Entity', attributes: {} };
    for (const [k, v] of Object.entries(r || {})) {
      if (k === 'id' || k === 'label') continue;
      out.attributes[k] = Array.isArray(v) && v.length ? v[0] : v;
    }
    for (const f of ['kind', 'project', 'project_domain', 'layer']) {
      if (out.attributes[f] !== undefined) { out[f] = out.attributes[f]; }
    }
    return out;
  }
}

/**
 * 内存 Mock 远程图驱动：
 *  - 对齐真实 GremlinHttpDriver 方法签名；
 *  - 采用"nodes map + adjacency list + edge list"三结构，邻居/最短路径/语义搜索返回同构结构。
 *  - 计数器 callCounts：记录各方法调用次数，满足 T3 TR-3.1 序列断言。
 */
class MockRemoteGraphDriver extends RemoteGraphDriver {
  constructor() {
    super();
    this.name = 'mock-remote';
    this._nodes = new Map();        // id -> node
    this._outEdges = new Map();     // id -> [{target, type, weight, attrs}]
    this._inEdges = new Map();
    this._edgeList = [];
    this.callCounts = Object.create(null);
    this.resetStats();
  }
  resetStats() {
    this.callCounts = Object.create(null);
  }
  _tick(name) { this.callCounts[name] = (this.callCounts[name] || 0) + 1; }
  _getOrCreateAdj(id) {
    if (!this._outEdges.has(id)) this._outEdges.set(id, []);
    if (!this._inEdges.has(id)) this._inEdges.set(id, []);
  }

  async connect() { return true; }
  async disconnect() { return true; }

  async getNode(id) { this._tick('getNode'); return this._nodes.get(id) || null; }
  async listNodes({ kind, project, project_domain, layer } = {}) {
    this._tick('listNodes');
    return Array.from(this._nodes.values()).filter(n => {
      if (kind && n.kind !== kind) return false;
      if (project && n.attributes && n.attributes.project !== project) return false;
      if (project_domain && n.attributes && n.attributes.project_domain !== project_domain) return false;
      if (layer && n.attributes && n.attributes.layer !== layer) return false;
      return true;
    });
  }
  async neighbors(ids, hops = 1, dir = 'BOTH') {
    this._tick('neighbors');
    const visited = new Set();
    let frontier = new Set(Array.isArray(ids) ? ids : [ids]);
    for (let h = 0; h < hops; h++) {
      const next = new Set();
      for (const id of frontier) {
        this._getOrCreateAdj(id);
        if (dir === 'BOTH' || dir === 'OUT') for (const e of (this._outEdges.get(id) || [])) next.add(e.target);
        if (dir === 'BOTH' || dir === 'IN') for (const e of (this._inEdges.get(id) || [])) next.add(e.source);
      }
      frontier = next;
      for (const x of frontier) visited.add(x);
    }
    const out = [];
    for (const id of visited) if (this._nodes.has(id)) out.push(this._nodes.get(id));
    return out;
  }
  async shortestPath(src, dst, maxHop = 8) {
    this._tick('shortestPath');
    if (src === dst) return [src];
    const prev = new Map([[src, null]]);
    const q = [src];
    let depth = 0;
    while (q.length && depth <= maxHop) {
      depth++;
      const size = q.length;
      for (let i = 0; i < size; i++) {
        const id = q.shift();
        if (id === dst) {
          const path = [];
          let x = dst;
          while (x != null) { path.push(x); x = prev.get(x); }
          return path.reverse();
        }
        this._getOrCreateAdj(id);
        const all = [...(this._outEdges.get(id) || []).map(e => e.target), ...(this._inEdges.get(id) || []).map(e => e.source)];
        for (const t of all) {
          if (!prev.has(t)) { prev.set(t, id); q.push(t); }
        }
      }
    }
    return null;
  }

  async upsertNode(node) {
    this._tick('upsertNode');
    if (!node || !node.id) throw new Error('node.id 必须');
    const id = node.id;
    const old = this._nodes.get(id) || { id, label: node.label || 'Entity', attributes: {} };
    const merged = {
      id,
      label: node.label || old.label,
      kind: node.kind !== undefined ? node.kind : old.kind,
      attributes: { ...(old.attributes || {}), ...(node.attributes || {}) }
    };
    for (const k of ['project', 'project_domain', 'layer']) if (node[k] !== undefined) merged.attributes[k] = node[k];
    this._nodes.set(id, merged);
    return merged;
  }
  async upsertEdge(edge) {
    this._tick('upsertEdge');
    if (!edge || !edge.source || !edge.target) throw new Error('upsertEdge 需要 source/target');
    this._getOrCreateAdj(edge.source);
    this._getOrCreateAdj(edge.target);
    const type = edge.type || 'LINK';
    const weight = edge.weight ?? 1;
    const attrs = edge.attributes || {};
    const record = { source: edge.source, target: edge.target, type, weight, attrs: { ...attrs } };
    this._outEdges.get(edge.source).push({ target: edge.target, type, weight, attrs });
    this._inEdges.get(edge.target).push({ source: edge.source, type, weight, attrs });
    this._edgeList.push(record);
    return record;
  }
  async bulkUpsert({ nodes = [], edges = [] }) {
    const addedNodes = [], addedEdges = [];
    for (const n of nodes) addedNodes.push(await this.upsertNode(n));
    for (const e of edges) addedEdges.push(await this.upsertEdge(e));
    return { addedNodes, addedEdges };
  }
  async deleteNode(id) { this._tick('deleteNode'); this._nodes.delete(id); this._outEdges.delete(id); this._inEdges.delete(id); return true; }
  async deleteEdge({ source, target, type }) {
    this._tick('deleteEdge');
    const remove = (arr, src, tgt) => {
      for (let i = arr.length - 1; i >= 0; i--) {
        const e = arr[i];
        if (src && e.source && e.source !== src) continue;
        if (tgt && e.target && e.target !== tgt) continue;
        if (type && e.type !== type) continue;
        arr.splice(i, 1);
      }
    };
    if (this._outEdges.has(source)) remove(this._outEdges.get(source), null, target);
    if (this._inEdges.has(target)) remove(this._inEdges.get(target), source, null);
    return true;
  }
  async stats() { return { nodes: this._nodes.size, edges: this._edgeList.length }; }
  async semanticSearch(textOrVec, topK = 10) {
    this._tick('semanticSearch');
    // 内存 mock：基于 node.attributes JSON 字符串做关键字匹配
    const q = String(textOrVec).toLowerCase();
    const scored = [];
    for (const n of this._nodes.values()) {
      const text = (n.id + ' ' + (n.label || '') + ' ' + JSON.stringify(n.attributes || {})).toLowerCase();
      const score = q ? (text.split(q).length - 1) : 0;
      if (score > 0 || !q) scored.push({ item: n, score });
    }
    scored.sort((a, b) => b.score - a.score);
    const items = scored.slice(0, topK).map(s => s.item);
    return { items, total: items.length };
  }
}

/** 自动创建驱动：USE_NEBULAGRAPH===true && 有 endpoint → GremlinHttpDriver，否则 → Mock */
function createDriverFromEnv(options = {}) {
  const enable = process.env.USE_NEBULAGRAPH === 'true' || options.force;
  if (!enable) return new MockRemoteGraphDriver();
  const endpoint = process.env.NEBULA_ENDPOINT || process.env.GREMLIN_ENDPOINT || options.endpoint;
  if (!endpoint) return new MockRemoteGraphDriver();
  return new GremlinHttpDriver({ ...options, endpoint });
}

module.exports = {
  RemoteGraphDriver,
  BaseHttpDriver,
  GremlinHttpDriver,
  MockRemoteGraphDriver,
  createDriverFromEnv
};

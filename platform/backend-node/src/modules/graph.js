'use strict';

const { registerModule, BaseModule } = require('./index');
const { getStorage } = require('../storage');
const { uid } = require('../utils');

const routes = [
  {
    method: 'get', path: '/graph',
    handler: (req, res) => {
      const s = getStorage();
      const nodes = s.getList('graph_nodes');
      const edges = s.getList('graph_edges');
      BaseModule.ok(res, { nodes, edges });
    }
  },
  {
    method: 'get', path: '/graph/stats',
    handler: (req, res) => {
      const s = getStorage();
      const n = s.countByType('graph_nodes');
      const e = s.countByType('graph_edges');
      BaseModule.ok(res, { nodes: n, edges: e, communities: 5 });
    }
  },
  {
    method: 'get', path: '/graph/nodes',
    handler: (req, res) => {
      BaseModule.ok(res, getStorage().getList('graph_nodes'));
    }
  },
  {
    method: 'get', path: '/graph/edges',
    handler: (req, res) => {
      BaseModule.ok(res, getStorage().getList('graph_edges'));
    }
  },
  {
    method: 'post', path: '/graph/node',
    handler: async (req, res) => {
      const s = getStorage();
      const body = await BaseModule.readBody(req);
      if (!body.id || !body.label) return BaseModule.fail(res, 400, 'id and label required');
      const nodes = s.getList('graph_nodes');
      if (nodes.find(n => n.id === body.id)) return BaseModule.fail(res, 409, 'id exists');
      const node = { id: body.id, label: body.label, type: body.type || 'concept', description: body.description || '', attributes: body.attributes || {}, community: 0, degree: 0, created_at: new Date().toISOString(), ...body };
      nodes.push(node);
      s.saveList('graph_nodes', nodes);
      BaseModule.ok(res, node);
    }
  },
  {
    method: 'post', path: '/graph/edge',
    handler: async (req, res) => {
      const s = getStorage();
      const body = await BaseModule.readBody(req);
      if (!body.source || !body.target) return BaseModule.fail(res, 400, 'source and target required');
      const edges = s.getList('graph_edges');
      const edge = { id: 'graph_edge_' + Date.now(), source: body.source, target: body.target, label: body.label || 'related', weight: body.weight || 1.0, created_at: new Date().toISOString(), ...body };
      edges.push(edge);
      s.saveList('graph_edges', edges);
      BaseModule.ok(res, edge);
    }
  },
  {
    method: 'post', path: '/graph/bulk',
    handler: async (req, res) => {
      const s = getStorage();
      const body = await BaseModule.readBody(req);
      const { nodes = [], edges = [] } = body;
      const curNodes = s.getList('graph_nodes');
      const curEdges = s.getList('graph_edges');
      const existingIds = new Set(curNodes.map(n => n.id));
      const existingKeys = new Set(curEdges.map(e => `${e.source}_${e.target}`));
      const addedNodes = [], addedEdges = [];
      for (const n of nodes) {
        if (!existingIds.has(n.id)) {
          const enriched = { id: n.id, label: n.label || n.id, type: n.type || 'concept', description: n.description || '', attributes: n.attributes || {}, community: 0, degree: 0, created_at: new Date().toISOString(), ...n };
          curNodes.push(enriched); addedNodes.push(enriched); existingIds.add(n.id);
        }
      }
      for (const e of edges) {
        const key = `${e.source}_${e.target}`;
        if (!existingKeys.has(key)) {
          const enriched = { id: 'graph_edge_' + Date.now() + '_' + Math.random().toString(36).slice(2, 6), source: e.source, target: e.target, label: e.label || 'related', weight: e.weight || 1.0, created_at: new Date().toISOString(), ...e };
          curEdges.push(enriched); addedEdges.push(enriched); existingKeys.add(key);
        }
      }
      s.saveList('graph_nodes', curNodes);
      s.saveList('graph_edges', curEdges);
      BaseModule.ok(res, { added: { nodes: addedNodes.length, edges: addedEdges.length }, total: { nodes: curNodes.length, edges: curEdges.length } });
    }
  },
  {
    method: 'delete', path: '/graph/node/:id',
    handler: async (req, res, params) => {
      const s = getStorage();
      const nodes = s.getList('graph_nodes');
      const idx = nodes.findIndex(n => n.id === params.id);
      if (idx === -1) return BaseModule.fail(res, 404, 'node not found');
      nodes.splice(idx, 1);
      s.saveList('graph_nodes', nodes);
      s.deleteEntity(params.id);
      BaseModule.ok(res, { deleted: true });
    }
  },
  {
    method: 'get', path: '/graph/export',
    handler: (req, res) => {
      const s = getStorage();
      const nodes = s.getList('graph_nodes');
      const edges = s.getList('graph_edges');
      BaseModule.ok(res, { version: '2.0', exportedAt: new Date().toISOString(), graph: { nodes, edges } });
    }
  },
  {
    method: 'get', path: '/graph/search',
    handler: (req, res) => {
      const parsed = require('url').parse(req.url, true);
      const q = parsed.query.q || '';
      if (!q) return BaseModule.fail(res, 400, 'q required');
      const nodes = getStorage().searchEntities('graph_nodes', q);
      BaseModule.ok(res, { query: q, results: nodes, total: nodes.length });
    }
  },
  // Step4 · 开放给前端 Cytoscape 的 2 个 L3.5 中枢 API
  //   GET /kg/neighborhood?ids=eq:a,eq:b&depth=2&limit=200
  //   GET /kg/path?src=proj:A&dst=incident:B&maxDepth=6
  {
    method: 'get', path: '/kg/neighborhood',
    handler: (req, res) => {
      try {
        const { query } = require('url').parse(req.url, true);
        const ids = (query.ids || '').split(',').map(s => s.trim()).filter(Boolean);
        if (!ids.length) return BaseModule.fail(res, 400, 'ids required (comma separated)');
        const depth = Math.max(1, Math.min(5, parseInt(query.depth || '2', 10)));
        const limit = Math.max(10, Math.min(1000, parseInt(query.limit || '200', 10)));
        const s = getStorage();
        const sub = typeof s.neighborhoodSubgraph === 'function'
          ? s.neighborhoodSubgraph(ids, depth, limit)
          : { nodes: [], edges: [], note: 'provider 暂未实现 neighborhoodSubgraph（升级 SQLite/PG 后可用）' };
        // Cytoscape 直接消费格式：{ elements: { nodes, edges } }
        BaseModule.ok(res, {
          ids, depth, limit,
          elements: {
            nodes: (sub.nodes || []).map(n => ({ data: typeof n === 'object' ? (n.id ? n : { id: String(n) }) : { id: String(n) } })),
            edges: (sub.edges || []).map(e => ({ data: { id: `${e.src}-${e.rel}-${e.dst}`, source: e.src, target: e.dst, rel: e.rel, props: e.props || null } }))
          },
          meta: { nodes: (sub.nodes || []).length, edges: (sub.edges || []).length }
        });
      } catch (e) {
        return BaseModule.fail(res, 500, 'neighborhood error: ' + e.message);
      }
    }
  },
  {
    method: 'get', path: '/kg/path',
    handler: (req, res) => {
      try {
        const { query } = require('url').parse(req.url, true);
        const src = String(query.src || '').trim();
        const dst = String(query.dst || '').trim();
        if (!src || !dst) return BaseModule.fail(res, 400, 'src & dst required');
        const maxDepth = Math.max(1, Math.min(8, parseInt(query.maxDepth || '6', 10)));
        const s = getStorage();
        const path = typeof s.findPath === 'function'
          ? s.findPath(src, dst, maxDepth) || []
          : [];
        // 金链覆盖校验（§5.5 红线 2）：任何 Project → Incident 必须非空，否则返回 warning 字段
        const threeChainsWarn = path.length === 0 ? `空路径：若 src/dst 对应 §4.2 三条金链节点对（需求/根因/切换审计/组织进化），则 Stage 禁止推进，CR-003 需先补边` : null;
        BaseModule.ok(res, {
          src, dst, maxDepth,
          hops: path.length,
          edges: path.map(e => ({ source: e.src, target: e.dst, rel: e.rel, props: e.props || null })),
          cytoscape_elements: {
            nodes: Array.from(new Set([src, dst, ...path.map(e => [e.src, e.dst]).flat()])).map(id => ({ data: { id } })),
            edges: path.map((e, i) => ({ data: { id: `hop${i}-${e.rel}`, source: e.src, target: e.dst, rel: e.rel } }))
          },
          warning: threeChainsWarn
        });
      } catch (e) {
        return BaseModule.fail(res, 500, 'path error: ' + e.message);
      }
    }
  }
];

registerModule('graph', routes, { description: '知识图谱模块', version: '2.0' });

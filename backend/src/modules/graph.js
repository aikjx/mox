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
  }
];

registerModule('graph', routes, { description: '知识图谱模块', version: '2.0' });

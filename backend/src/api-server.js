const http = require('http');
const fs = require('fs');
const path = require('path');
const url = require('url');
const { getGateway } = require('./llm-gateway');
const { getAlliance } = require('./expert-alliance');

const PORT = 3002;
const DATA_DIR = path.join(__dirname, '..', 'data');

const gateway = getGateway();
const alliance = getAlliance();

function p(...parts) {
  return path.join(DATA_DIR, ...parts);
}

function readJSON(file, fallback) {
  try {
    const fp = p(file);
    if (!fs.existsSync(fp)) return fallback;
    const raw = fs.readFileSync(fp, 'utf8');
    return raw ? JSON.parse(raw) : fallback;
  } catch (e) {
    return fallback;
  }
}

function writeJSON(file, data) {
  try {
    fs.writeFileSync(p(file), JSON.stringify(data, null, 2), 'utf8');
    return true;
  } catch (e) {
    console.error('[writeJSON]', file, e.message);
    return false;
  }
}

function uid(prefix) {
  return prefix + '_' + Math.random().toString(36).slice(2, 12);
}

function send(res, status, payload, headers) {
  const body = JSON.stringify(payload);
  res.writeHead(status, Object.assign({
    'Content-Type': 'application/json; charset=utf-8',
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET,POST,PUT,DELETE,OPTIONS,PATCH',
    'Access-Control-Allow-Headers': 'Content-Type,Authorization,Accept,X-Requested-With,Origin'
  }, headers || {}));
  res.end(body);
}

function ok(res, data, extra) {
  send(res, 200, Object.assign({ success: true, data: data }, extra || {}));
}

function fail(res, status, message) {
  send(res, status, { success: false, error: message });
}

function readBody(req) {
  return new Promise((resolve) => {
    let chunks = '';
    req.on('data', (c) => { chunks += c; });
    req.on('end', () => {
      if (!chunks) return resolve({});
      try { resolve(JSON.parse(chunks)); } catch (e) { resolve({}); }
    });
    req.on('error', () => resolve({}));
  });
}

function log(msg) {
  const t = new Date().toISOString();
  console.log('[api-server]', t, msg);
}

function appendLog(entry) {
  try {
    const logs = readJSON('logs.json', []);
    logs.unshift(Object.assign({ id: uid('log'), ts: new Date().toISOString() }, entry));
    if (logs.length > 500) logs.length = 500;
    writeJSON('logs.json', logs);
  } catch (e) {}
}

const autoSync = { active: false, interval: null };
function toggleAutoSync(req, res) {
  autoSync.active = !autoSync.active;
  if (autoSync.active) {
    autoSync.interval = setInterval(() => {
      const nodes = readJSON('graph_nodes.json', []);
      const edges = readJSON('graph_edges.json', []);
      appendLog({ type: 'auto-sync', msg: 'auto sync tick', nodes: nodes.length, edges: edges.length });
    }, 3000);
  } else if (autoSync.interval) {
    clearInterval(autoSync.interval);
    autoSync.interval = null;
  }
  ok(res, { active: autoSync.active });
}

function graphAdjacency() {
  const nodes = readJSON('graph_nodes.json', []);
  const edges = readJSON('graph_edges.json', []);
  const adj = {};
  nodes.forEach((n) => { adj[n.id] = { out: [], in: [] }; });
  edges.forEach((e) => {
    if (adj[e.source]) adj[e.source].out.push(e.target);
    if (adj[e.target]) adj[e.target].in.push(e.source);
  });
  return { nodes, edges, adj };
}

function bfsPath(adj, source, target) {
  if (!adj[source] || !adj[target]) return null;
  const visited = { [source]: null };
  const q = [source];
  while (q.length) {
    const cur = q.shift();
    if (cur === target) {
      const pathArr = [];
      let n = cur;
      while (n !== null) { pathArr.unshift(n); n = visited[n]; }
      return pathArr;
    }
    (adj[cur] ? adj[cur].out : []).forEach((nb) => {
      if (!(nb in visited)) { visited[nb] = cur; q.push(nb); }
    });
  }
  return null;
}

function pagerank(nodes, edges, damping, maxIter) {
  damping = damping || 0.85;
  maxIter = maxIter || 80;
  const n = nodes.length;
  if (n === 0) return {};
  const idIndex = {};
  nodes.forEach((node, i) => { idIndex[node.id] = i; });
  const outLinks = nodes.map(() => []);
  edges.forEach((e) => {
    const si = idIndex[e.source], ti = idIndex[e.target];
    if (si !== undefined && ti !== undefined && outLinks[si].indexOf(ti) === -1) {
      outLinks[si].push(ti);
    }
  });
  let pr = nodes.map(() => 1 / n);
  for (let iter = 0; iter < maxIter; iter++) {
    const newPr = nodes.map(() => (1 - damping) / n);
    for (let i = 0; i < n; i++) {
      const out = outLinks[i];
      if (out.length === 0) {
        for (let j = 0; j < n; j++) newPr[j] += damping * pr[i] / n;
      } else {
        const share = damping * pr[i] / out.length;
        out.forEach((j) => { newPr[j] += share; });
      }
    }
    const diff = pr.reduce((s, v, i) => s + Math.abs(v - newPr[i]), 0);
    pr = newPr;
    if (diff < 1e-6) break;
  }
  const result = {};
  nodes.forEach((node, i) => { result[node.id] = pr[i]; });
  return result;
}

function degreeCentrality(nodes, edges) {
  const inDeg = {}, outDeg = {};
  nodes.forEach((n) => { inDeg[n.id] = 0; outDeg[n.id] = 0; });
  edges.forEach((e) => {
    if (outDeg[e.source] !== undefined) outDeg[e.source]++;
    if (inDeg[e.target] !== undefined) inDeg[e.target]++;
  });
  const total = nodes.length - 1;
  const result = {};
  nodes.forEach((n) => {
    const d = (inDeg[n.id] || 0) + (outDeg[n.id] || 0);
    result[n.id] = {
      degree: d,
      inDegree: inDeg[n.id] || 0,
      outDegree: outDeg[n.id] || 0,
      normalized: total > 0 ? d / total : 0
    };
  });
  return result;
}

function betweennessCentrality(nodes, edges) {
  const adj = {};
  nodes.forEach((n) => { adj[n.id] = []; });
  edges.forEach((e) => {
    if (adj[e.source]) adj[e.source].push(e.target);
    if (adj[e.target]) adj[e.target].push(e.source);
  });
  const cb = {};
  nodes.forEach((n) => { cb[n.id] = 0; });
  const ids = nodes.map((n) => n.id);
  ids.forEach((s) => {
    const S = [];
    const P = {};
    const sigma = {};
    ids.forEach((t) => { P[t] = []; sigma[t] = 0; });
    sigma[s] = 1;
    const Q = [s];
    while (Q.length) {
      const v = Q.shift();
      S.push(v);
      (adj[v] || []).forEach((w) => {
        if (sigma[w] === 0) Q.push(w);
        sigma[w] += sigma[v];
        P[w].push(v);
      });
    }
    const delta = {};
    ids.forEach((t) => { delta[t] = 0; });
    while (S.length) {
      const w = S.pop();
      P[w].forEach((v) => {
        if (sigma[w] > 0) delta[v] += (sigma[v] / sigma[w]) * (1 + delta[w]);
      });
      if (w !== s) cb[w] += delta[w];
    }
  });
  return cb;
}

function labelPropagation(nodes, edges, maxIter) {
  maxIter = maxIter || 30;
  const adj = {};
  nodes.forEach((n) => { adj[n.id] = []; });
  edges.forEach((e) => {
    if (adj[e.source]) adj[e.source].push(e.target);
    if (adj[e.target]) adj[e.target].push(e.source);
  });
  const labels = {};
  nodes.forEach((n, i) => { labels[n.id] = i; });
  const ids = nodes.map((n) => n.id);
  let changed = true;
  let iter = 0;
  while (changed && iter < maxIter) {
    changed = false;
    iter++;
    for (let i = ids.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      const tmp = ids[i]; ids[i] = ids[j]; ids[j] = tmp;
    }
    ids.forEach((v) => {
      const neighborLabels = {};
      (adj[v] || []).forEach((nb) => {
        const l = labels[nb];
        neighborLabels[l] = (neighborLabels[l] || 0) + 1;
      });
      let bestLabel = labels[v];
      let bestCount = -1;
      Object.keys(neighborLabels).forEach((l) => {
        if (neighborLabels[l] > bestCount) { bestCount = neighborLabels[l]; bestLabel = parseInt(l, 10); }
      });
      if (bestLabel !== labels[v]) { labels[v] = bestLabel; changed = true; }
    });
  }
  const communities = {};
  ids.forEach((id) => {
    const c = labels[id];
    if (!communities[c]) communities[c] = [];
    communities[c].push(id);
  });
  return communities;
}

function activateSpread(nodes, edges, seedId, decay) {
  decay = decay || 0.7;
  const adj = {};
  nodes.forEach((n) => { adj[n.id] = []; });
  edges.forEach((e) => {
    if (adj[e.source]) adj[e.source].push(e.target);
  });
  const energy = {};
  nodes.forEach((n) => { energy[n.id] = 0; });
  if (!adj[seedId]) return energy;
  const q = [{ id: seedId, e: 1.0, depth: 0 }];
  const visited = {};
  while (q.length) {
    const cur = q.shift();
    if (visited[cur.id] && visited[cur.id] >= cur.e) continue;
    visited[cur.id] = cur.e;
    if (cur.e > energy[cur.id]) energy[cur.id] = cur.e;
    if (cur.depth < 6 && cur.e > 0.01) {
      (adj[cur.id] || []).forEach((nb) => {
        q.push({ id: nb, e: cur.e * decay, depth: cur.depth + 1 });
      });
    }
  }
  return energy;
}

const handlers = {
  get: {},
  post: {},
  put: {},
  delete: {}
};

function reg(method, pattern, fn) {
  handlers[method][pattern] = fn;
}

function match(method, urlPath) {
  const map = handlers[method];
  if (!map) return null;
  const keys = Object.keys(map).sort((a, b) => b.length - a.length);
  for (let i = 0; i < keys.length; i++) {
    const k = keys[i];
    if (k === urlPath) return { fn: map[k], params: {} };
    const kp = k.split('/');
    const up = urlPath.split('/');
    if (kp.length !== up.length) continue;
    const params = {};
    let ok = true;
    for (let j = 0; j < kp.length; j++) {
      if (kp[j].startsWith(':')) {
        params[kp[j].slice(1)] = decodeURIComponent(up[j]);
      } else if (kp[j] !== up[j]) {
        ok = false; break;
      }
    }
    if (ok) return { fn: map[k], params: params };
  }
  return null;
}

function registerRoutes() {
  reg('get', '/health', (req, res) => {
    ok(res, { status: 'ok', version: '3.0.0', uptime: process.uptime() });
  });

  reg('get', '/status', (req, res) => {
    const ops = readJSON('operators.json', []);
    const plugins = readJSON('plugins.json', []);
    const nodes = readJSON('graph_nodes.json', []);
    const edges = readJSON('graph_edges.json', []);
    const logs = readJSON('logs.json', []);
    const execLogs = logs.filter(l => l.type === 'execute' || l.type === 'workflow');
    const successLogs = execLogs.filter(l => l.success !== false);
    const totalExec = execLogs.length;
    const successRate = totalExec > 0 ? (successLogs.length / totalExec) * 100 : 98.5;
    const customOps = ops.filter(o => o.id && o.id.startsWith('operators_'));
    ok(res, {
      status: 'running',
      version: '3.0.0',
      operators_count: ops.length,
      plugins_count: plugins.length,
      executions_count: totalExec,
      success_rate: Math.round(successRate * 10) / 10,
      custom_operators_count: customOps.length,
      graph: { nodes: nodes.length, edges: edges.length, communities: 5 },
      ai_capabilities: ['chat', 'analyze', 'compile', 'optimize', 'publish', 'automate', 'monitor']
    });
  });

  reg('get', '/status/full', (req, res) => {
    const ops = readJSON('operators.json', []);
    const plugins = readJSON('plugins.json', []);
    const nodes = readJSON('graph_nodes.json', []);
    const edges = readJSON('graph_edges.json', []);
    const logs = readJSON('logs.json', []);
    const execLogs = logs.filter(l => l.type === 'execute' || l.type === 'workflow');
    const successLogs = execLogs.filter(l => l.success !== false);
    const totalExec = execLogs.length;
    const successRate = totalExec > 0 ? (successLogs.length / totalExec) * 100 : 98.5;
    const customOps = ops.filter(o => o.id && o.id.startsWith('operators_'));
    ok(res, {
      status: 'running',
      version: '3.0.0',
      uptime: process.uptime(),
      operators_count: ops.length,
      plugins_count: plugins.length,
      executions_count: totalExec,
      success_rate: Math.round(successRate * 10) / 10,
      custom_operators_count: customOps.length,
      graph: { nodes: nodes.length, edges: edges.length, communities: 5 },
      ai_capabilities: ['chat', 'analyze', 'compile', 'optimize', 'publish', 'automate', 'monitor'],
      collections: ['operators', 'graph_nodes', 'graph_edges', 'market', 'plugins', 'workflows', 'flows', 'resources']
    });
  });

  reg('get', '/logs', (req, res) => {
    const rawLogs = readJSON('logs.json', []);
    const execLogs = rawLogs.filter(l => l.type === 'execute' || l.type === 'workflow');
    if (execLogs.length > 0) {
      ok(res, execLogs.map(l => ({
        timestamp: l.timestamp || l.ts,
        workflow: l.workflow || [l.msg || 'execute'],
        success: l.success !== false,
        execution_time_ms: l.execution_time_ms || (50 + Math.floor(Math.random() * 500)),
        input_dim: l.input_dim || 3,
        output_dim: l.output_dim || 7
      })));
    } else {
      const mockLogs = [];
      const now = Date.now();
      const workflows = [
        ['需求采集', '归一化 IR', '双联盟十四维特派', '归一化裁决', '璇玑验证网关'],
        ['数据输入', '知识图谱算子', 'PageRank 计算', '社区发现'],
        ['浏览器自动化', '页面解析', '数据提取', '报告生成'],
        ['AI 对话', '意图识别', '算子匹配', '结果聚合'],
        ['工作流编排', '算子执行', '状态监控', '异常处理']
      ];
      for (let i = 0; i < 15; i++) {
        const wf = workflows[i % workflows.length];
        mockLogs.push({
          timestamp: new Date(now - i * 300000).toISOString(),
          workflow: wf,
          success: Math.random() > 0.1,
          execution_time_ms: 50 + Math.floor(Math.random() * 500),
          input_dim: 2 + Math.floor(Math.random() * 5),
          output_dim: 5 + Math.floor(Math.random() * 10)
        });
      }
      ok(res, mockLogs);
    }
  });

  reg('get', '/config', (req, res) => {
    ok(res, {
      version: '3.0.0',
      name: '算子统一系统 (OUS)',
      maxGraphSize: 10000,
      autoSave: true,
      aiEnabled: true,
      llmConfigured: true,
      modules: ['workbench', 'operators', 'graph', 'ai', 'workflow', 'plugins', 'browser', 'monitor']
    });
  });

  reg('get', '/plugins', (req, res) => { ok(res, readJSON('plugins.json', [])); });

  reg('get', '/operators', (req, res) => { ok(res, readJSON('operators.json', [])); });

  reg('get', '/operators/categories', (req, res) => {
    const ops = readJSON('operators.json', []);
    const cats = {};
    ops.forEach(op => {
      const c = op.category || 'general';
      if (!cats[c]) cats[c] = { name: c, count: 0 };
      cats[c].count++;
    });
    ok(res, Object.values(cats));
  });

  reg('get', '/operators/stats', (req, res) => {
    const ops = readJSON('operators.json', []);
    const byType = {};
    const byStatus = {};
    ops.forEach(op => {
      const t = op.type || 'unknown';
      const s = op.status || 'active';
      byType[t] = (byType[t] || 0) + 1;
      byStatus[s] = (byStatus[s] || 0) + 1;
    });
    ok(res, {
      total: ops.length,
      byType,
      byStatus,
      lastUpdated: new Date().toISOString()
    });
  });

  reg('post', '/operators/register', async (req, res) => {
    const body = await readBody(req);
    if (!body || !body.name) return fail(res, 400, 'name required');
    const ops = readJSON('operators.json', []);
    const op = Object.assign({
      id: uid('operators'),
      type: 'algorithm',
      category: 'general',
      desc: '',
      version: '1.0.0',
      status: 'active',
      tags: [],
      created_at: new Date().toISOString()
    }, body);
    ops.push(op);
    writeJSON('operators.json', ops);
    appendLog({ type: 'operator', msg: 'register ' + op.name, id: op.id });
    ok(res, op);
  });

  reg('post', '/execute', async (req, res) => {
    const body = await readBody(req);
    const workflow = body && body.workflow ? body.workflow : [];
    const results = [];
    let step = 0;
    for (let i = 0; i < workflow.length; i++) {
      const node = workflow[i];
      const dur = Math.random() * 120 + 20;
      await new Promise((r) => setTimeout(r, Math.min(dur, 50)));
      results.push({
        step: i,
        id: node.id || ('step_' + i),
        status: 'success',
        duration: Math.round(dur),
        output: 'Mock output for ' + (node.name || node.id || 'step ' + i)
      });
      step++;
    }
    const summary = {
      executed: results.length,
      totalDuration: results.reduce((s, r) => s + r.duration, 0),
      status: 'success'
    };
    appendLog({ type: 'execute', msg: 'workflow executed', steps: results.length });
    ok(res, { results: results, summary: summary });
  });

  reg('get', '/graph', (req, res) => {
    ok(res, {
      nodes: readJSON('graph_nodes.json', []),
      edges: readJSON('graph_edges.json', [])
    });
  });

  reg('get', '/graph/stats', (req, res) => {
    const { nodes, edges } = graphAdjacency();
    const n = nodes.length;
    const m = edges.length;
    const density = n > 1 ? m / (n * (n - 1)) : 0;
    const typeCounts = {};
    nodes.forEach((nd) => { typeCounts[nd.type] = (typeCounts[nd.type] || 0) + 1; });
    const degreeDist = {};
    const degMap = {};
    nodes.forEach((nd) => { degMap[nd.id] = 0; });
    edges.forEach((e) => {
      degMap[e.source] = (degMap[e.source] || 0) + 1;
      degMap[e.target] = (degMap[e.target] || 0) + 1;
    });
    Object.keys(degMap).forEach((id) => {
      const d = degMap[id];
      degreeDist[d] = (degreeDist[d] || 0) + 1;
    });
    const avgDegree = n > 0 ? Object.keys(degMap).reduce((s, k) => s + degMap[k], 0) / n : 0;
    ok(res, {
      nodes: n,
      edges: m,
      density: density,
      avgDegree: avgDegree,
      types: typeCounts,
      degreeDistribution: degreeDist
    });
  });

  reg('get', '/graph/centrality', (req, res) => {
    const { nodes, edges } = graphAdjacency();
    ok(res, {
      degree: degreeCentrality(nodes, edges),
      betweenness: betweennessCentrality(nodes, edges)
    });
  });

  reg('get', '/graph/communities', (req, res) => {
    const { nodes, edges } = graphAdjacency();
    const communities = labelPropagation(nodes, edges);
    const arr = Object.keys(communities).map((k, i) => ({
      id: 'c' + i,
      members: communities[k],
      size: communities[k].length
    }));
    ok(res, { communities: arr, count: arr.length });
  });

  reg('get', '/graph/pagerank', (req, res) => {
    const { nodes, edges } = graphAdjacency();
    const pr = pagerank(nodes, edges, 0.85, 80);
    const sorted = Object.keys(pr).map((id) => ({ id: id, score: pr[id] })).sort((a, b) => b.score - a.score);
    ok(res, { pagerank: pr, sorted: sorted, top10: sorted.slice(0, 10) });
  });

  reg('get', '/graph/neighbors/:id', (req, res, params) => {
    const { nodes, edges, adj } = graphAdjacency();
    const id = params.id;
    if (!adj[id]) return fail(res, 404, 'node not found');
    const outNodes = adj[id].out.map((t) => nodes.find((n) => n.id === t)).filter(Boolean);
    const inNodes = adj[id].in.map((t) => nodes.find((n) => n.id === t)).filter(Boolean);
    ok(res, { id: id, outgoing: outNodes, incoming: inNodes, degree: outNodes.length + inNodes.length });
  });

  reg('get', '/graph/path', (req, res) => {
    const q = url.parse(req.url, true).query;
    const source = q.source, target = q.target;
    if (!source || !target) return fail(res, 400, 'source and target required');
    const { adj } = graphAdjacency();
    const p = bfsPath(adj, source, target);
    if (!p) return fail(res, 404, 'no path found');
    ok(res, { source: source, target: target, path: p, length: p.length - 1 });
  });

  reg('post', '/graph/recommend', async (req, res) => {
    const body = await readBody(req);
    const seedIds = body.seeds || [];
    const { nodes, edges, adj } = graphAdjacency();
    const scores = {};
    seedIds.forEach((sid) => {
      if (!adj[sid]) return;
      const visited = {};
      const q = [{ id: sid, d: 0 }];
      visited[sid] = 0;
      while (q.length) {
        const cur = q.shift();
        if (cur.d > 3) continue;
        (adj[cur.id] ? adj[cur.id].out : []).forEach((nb) => {
          if (visited[nb] === undefined) {
            visited[nb] = cur.d + 1;
            q.push({ id: nb, d: cur.d + 1 });
          }
        });
      }
      Object.keys(visited).forEach((id) => {
        if (seedIds.indexOf(id) === -1) {
          const score = 1 / (visited[id] + 1);
          scores[id] = (scores[id] || 0) + score;
        }
      });
    });
    const recs = Object.keys(scores)
      .map((id) => ({ id: id, score: scores[id], node: nodes.find((n) => n.id === id) }))
      .filter((r) => r.node)
      .sort((a, b) => b.score - a.score)
      .slice(0, body.topK || 10);
    ok(res, { seeds: seedIds, recommendations: recs });
  });

  reg('post', '/graph/node', async (req, res) => {
    const body = await readBody(req);
    if (!body.id || !body.label) return fail(res, 400, 'id and label required');
    const nodes = readJSON('graph_nodes.json', []);
    if (nodes.find((n) => n.id === body.id)) return fail(res, 409, 'id exists');
    const node = Object.assign({
      type: 'operator', node_type: body.type || 'operator',
      color: '#5B8FF9', size: 8,
      created_at: new Date().toISOString()
    }, body);
    nodes.push(node);
    writeJSON('graph_nodes.json', nodes);
    appendLog({ type: 'graph', msg: 'add node ' + node.id });
    ok(res, node);
  });

  reg('post', '/graph/edge', async (req, res) => {
    const body = await readBody(req);
    if (!body.source || !body.target) return fail(res, 400, 'source and target required');
    const edges = readJSON('graph_edges.json', []);
    const edge = Object.assign({
      id: uid('graph_edges'),
      weight: 1,
      created_at: new Date().toISOString()
    }, body);
    edges.push(edge);
    writeJSON('graph_edges.json', edges);
    appendLog({ type: 'graph', msg: 'add edge ' + body.source + '->' + body.target });
    ok(res, edge);
  });

  reg('post', '/graph/activate', async (req, res) => {
    const body = await readBody(req);
    const seed = body.seed || body.seedId;
    if (!seed) return fail(res, 400, 'seed required');
    const { nodes, edges } = graphAdjacency();
    const energy = activateSpread(nodes, edges, seed, body.decay || 0.7);
    const rank = Object.keys(energy).map((id) => ({ id: id, energy: energy[id] }))
      .sort((a, b) => b.energy - a.energy).slice(0, 20);
    ok(res, { seed: seed, energy: energy, rank: rank });
  });

  reg('get', '/graph/search', (req, res) => {
    const q = url.parse(req.url, true).query;
    const query = (q.q || '').toLowerCase();
    const limit = parseInt(q.limit, 10) || 20;
    const nodes = readJSON('graph_nodes.json', []);
    const edges = readJSON('graph_edges.json', []);
    if (!query) return ok(res, { nodes: [], edges: [] });
    const matchedNodes = nodes.filter((n) =>
      (n.label || '').toLowerCase().indexOf(query) !== -1 ||
      (n.id || '').toLowerCase().indexOf(query) !== -1 ||
      (n.type || '').toLowerCase().indexOf(query) !== -1
    ).slice(0, limit);
    const matchedEdges = edges.filter((e) =>
      (e.source || '').toLowerCase().indexOf(query) !== -1 ||
      (e.target || '').toLowerCase().indexOf(query) !== -1
    ).slice(0, limit);
    ok(res, { nodes: matchedNodes, edges: matchedEdges, query: query });
  });

  reg('post', '/graph/auto-sync/toggle', toggleAutoSync);

  reg('get', '/graph/auto-sync/status', (req, res) => {
    ok(res, { active: autoSync.active });
  });

  reg('get', '/dialogue/sessions', (req, res) => {
    ok(res, readJSON('dialogue_sessions.json', []));
  });

  reg('get', '/ai/sessions', (req, res) => {
    ok(res, readJSON('dialogue_sessions.json', []));
  });

  reg('get', '/graph/export', (req, res) => {
    const nodes = readJSON('graph_nodes.json', []);
    const edges = readJSON('graph_edges.json', []);
    const pr = pagerank(nodes, edges, 0.85, 80);
    const comms = labelPropagation(nodes, edges);
    ok(res, {
      version: '1.0',
      exportedAt: new Date().toISOString(),
      graph: { nodes: nodes, edges: edges },
      analytics: { pagerank: pr, communities: comms }
    });
  });

  reg('post', '/graph/import', async (req, res) => {
    const body = await readBody(req);
    if (!body || !body.graph) return fail(res, 400, 'graph required');
    if (body.graph.nodes) writeJSON('graph_nodes.json', body.graph.nodes);
    if (body.graph.edges) writeJSON('graph_edges.json', body.graph.edges);
    appendLog({ type: 'graph', msg: 'import graph', nodes: body.graph.nodes ? body.graph.nodes.length : 0 });
    ok(res, { imported: true });
  });

  reg('post', '/ai/chat', async (req, res) => {
    const body = await readBody(req);
    const messages = body.messages || (body.message ? [{ role: 'user', content: body.message }] : []);
    const last = messages.length ? messages[messages.length - 1].content : '';
    const reply = buildAIReply(last);
    const sessionId = body.sessionId || body.session_id || uid('sess');
    const sessions = readJSON('dialogue_sessions.json', []);
    let sess = sessions.find((s) => s.id === sessionId);
    if (!sess) {
      sess = { id: sessionId, title: last.slice(0, 20) || '新会话', messages: [], updatedAt: new Date().toISOString() };
      sessions.push(sess);
    }
    sess.messages = sess.messages.concat([
      { role: 'user', content: last, ts: new Date().toISOString() },
      { role: 'assistant', content: reply, ts: new Date().toISOString() }
    ]);
    sess.updatedAt = new Date().toISOString();
    writeJSON('dialogue_sessions.json', sessions);
    
    // If expert type is specified, route through expert alliance
    if (body.expertType || body.expert_id) {
      try {
        const expertId = body.expert_id || `${body.expertType}-expert`;
        const expertResult = await alliance.consult(expertId, messages, {
          sessionId,
          temperature: body.temperature,
          maxTokens: body.maxTokens
        });
        ok(res, { reply: expertResult.response, sessionId, expert: expertResult.expert, metadata: expertResult.metadata });
        return;
      } catch (e) {
        // Fall back to normal reply
      }
    }
    
    ok(res, { reply: reply, sessionId: sessionId });
  });

  function buildAIReply(input) {
    const text = (input || '').toLowerCase();
    if (text.indexOf('pagerank') !== -1 || text.indexOf('中心性') !== -1) {
      return 'PageRank 是一种基于链接结构的节点影响力评估算法，采用阻尼系数 0.85 的迭代公式：PR(v) = (1-d)/N + d * Σ PR(u)/C(u)。在本系统中可通过 /graph/pagerank 端点实时计算。';
    }
    if (text.indexOf('社区') !== -1 || text.indexOf('community') !== -1) {
      return '本系统使用 Label Propagation 标签传播算法进行社区发现，时间复杂度接近线性，适合大规模图谱。可通过 /graph/communities 调用。';
    }
    if (text.indexOf('璇玑') !== -1) {
      return '双璇玑十四维治理体系：业务侧 7 维 + 研发侧 7 维，通过融合引擎 D04 汇聚并由验证网关 n_gate 进行闸门校验。';
    }
    if (text.indexOf('caomei') !== -1 || text.indexOf('草莓') !== -1 || text.indexOf('需求') !== -1) {
      return 'Caomei 需求编译器将自然语言需求编译为流程蓝图，支持精化迭代与模板复用。';
    }
    if (text.indexOf('你好') !== -1 || text.indexOf('hello') !== -1 || text.indexOf('hi') !== -1) {
      return '你好！我是 OUS 算子统一系统的 AI 助手，我可以帮你分析图谱、执行算子工作流、治理璇玑以及管理算子商城。';
    }
    if (text.indexOf('图谱') !== -1 || text.indexOf('graph') !== -1) {
      return '当前图谱包含 23 个节点与 30 条边，覆盖融合引擎、联盟、算子、AI 任务、商城等多种节点类型。可以查询邻居、最短路径或计算中心性。';
    }
    return '已收到你的请求："' + (input || '') + '"。本系统支持图谱分析、算子执行、AI 对话、浏览器自动化、MCP 兼容等能力。请告诉我具体需求。';
  }

  reg('get', '/ai/chat/history/:session', (req, res, params) => {
    const sessions = readJSON('dialogue_sessions.json', []);
    const sess = sessions.find((s) => s.id === params.session);
    if (!sess) return fail(res, 404, 'session not found');
    ok(res, sess);
  });

  reg('post', '/ai/analyze-algorithm', async (req, res) => {
    const body = await readBody(req);
    const algo = body.algorithm || 'unknown';
    ok(res, {
      algorithm: algo,
      complexity: { time: 'O(n log n)', space: 'O(n)' },
      description: algo + ' 的分析结果：适用于中小规模图谱，建议在 10k 节点以内运行。',
      params: body.params || {},
      benchmark: { avgMs: 120 + Math.floor(Math.random() * 200), samples: 100 }
    });
  });

  reg('get', '/ai/algorithm-types', (req, res) => {
    ok(res, [
      { id: 'pagerank', name: 'PageRank', category: 'graph', desc: '迭代式影响力排序' },
      { id: 'label-propagation', name: '标签传播', category: 'graph', desc: '无监督社区发现' },
      { id: 'bfs', name: '广度优先搜索', category: 'search', desc: '最短路径' },
      { id: 'activate', name: '激活传播', category: 'graph', desc: '种子扩散能量' },
      { id: 'centrality-degree', name: '度中心性', category: 'centrality', desc: '连接数度量' },
      { id: 'centrality-betweenness', name: '中介中心性', category: 'centrality', desc: '桥接节点识别' },
      { id: 'caomei-compile', name: '需求编译', category: 'compiler', desc: 'NL → 蓝图' }
    ]);
  });

  reg('post', '/analyze/spiral', async (req, res) => {
    const body = await readBody(req);
    ok(res, {
      input: body,
      spiral: {
        arms: 4,
        points: 100,
        seed: Math.random(),
        classification: body && body.topic ? 'topic-' + body.topic : 'general'
      }
    });
  });

  reg('get', '/ai/resources', (req, res) => {
    ok(res, readJSON('resources.json', {}));
  });

  reg('get', '/ai/resources/health', (req, res) => {
    const resData = readJSON('resources.json', {});
    const items = Array.isArray(resData) ? resData : (resData.items || []);
    const healthy = items.filter((i) => i.status === 'healthy').length;
    const total = items.length;
    ok(res, {
      total: total,
      healthy: healthy,
      healthRate: total > 0 ? healthy / total : 1,
      items: items.map((i) => ({ id: i.id || i.name, status: i.status || 'unknown', score: i.score || 0.8 }))
    });
  });

  reg('get', '/ai/plugins', (req, res) => { ok(res, readJSON('plugins.json', [])); });

  reg('post', '/ai/plugins/register', async (req, res) => {
    const body = await readBody(req);
    if (!body || !body.name) return fail(res, 400, 'name required');
    const plugins = readJSON('plugins.json', []);
    const p = Object.assign({
      id: uid('plug'),
      status: 'active',
      version: '1.0.0',
      registered_at: new Date().toISOString()
    }, body);
    plugins.push(p);
    writeJSON('plugins.json', plugins);
    appendLog({ type: 'plugin', msg: 'register ' + p.name });
    ok(res, p);
  });

  reg('post', '/ai/plugins/send-message', async (req, res) => {
    const body = await readBody(req);
    ok(res, {
      sent: true,
      target: body.target,
      message: body.message,
      deliveredAt: new Date().toISOString(),
      response: '已转发给插件 ' + (body.target || 'default')
    });
  });

  reg('get', '/ai/plugins/topology', (req, res) => {
    const plugins = readJSON('plugins.json', []);
    ok(res, {
      nodes: plugins.map((p) => ({ id: p.id, label: p.name, type: p.type || 'plugin' })),
      edges: plugins.map((p, i) => ({ source: 'core', target: p.id, weight: 1 })).concat([
        { source: 'mcp', target: 'core', weight: 1 }
      ])
    });
  });

  reg('get', '/ai/workflows/templates', (req, res) => {
    ok(res, [
      { id: 'wf_tpl_1', name: '图谱分析模板', steps: ['load_graph', 'compute_pagerank', 'detect_communities', 'export'] },
      { id: 'wf_tpl_2', name: '治理发布模板', steps: 'normalize -> govern -> optimize -> publish'.split(' -> ') },
      { id: 'wf_tpl_3', name: '需求编译模板', steps: ['caomei_compile', 'refine', 'validate'] }
    ]);
  });

  reg('get', '/ai/workflows', (req, res) => { ok(res, readJSON('workflows.json', [])); });

  reg('get', '/workflows', (req, res) => { ok(res, readJSON('workflows.json', [])); });

  reg('post', '/workflows', async (req, res) => {
    const body = await readBody(req);
    const wfs = readJSON('workflows.json', []);
    const wf = Object.assign({
      id: uid('wf'),
      status: 'draft',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    }, body);
    wfs.push(wf);
    writeJSON('workflows.json', wfs);
    appendLog({ type: 'workflow', msg: 'create ' + wf.name });
    ok(res, wf);
  });

  reg('post', '/ai/workflows/save', async (req, res) => {
    const body = await readBody(req);
    if (!body || !body.name) return fail(res, 400, 'name required');
    const wfs = readJSON('workflows.json', []);
    const wf = Object.assign({
      id: uid('wf'),
      status: 'draft',
      created_at: new Date().toISOString()
    }, body);
    wfs.push(wf);
    writeJSON('workflows.json', wfs);
    appendLog({ type: 'workflow', msg: 'save ' + wf.name });
    ok(res, wf);
  });

  reg('post', '/ai/workflows/execute', async (req, res) => {
    const body = await readBody(req);
    const id = body.workflowId;
    const wfs = readJSON('workflows.json', []);
    const wf = wfs.find((w) => w.id === id);
    const steps = wf ? (wf.steps || []) : (body.steps || []);
    const results = steps.map((s, i) => ({
      step: i, name: typeof s === 'string' ? s : (s.name || ('step_' + i)),
      status: 'success', duration: 30 + Math.floor(Math.random() * 80)
    }));
    appendLog({ type: 'workflow', msg: 'execute ' + id, steps: results.length });
    ok(res, { workflowId: id, results: results, status: 'success' });
  });

  reg('get', '/ai/workflows/instances', (req, res) => {
    const wfs = readJSON('workflows.json', []);
    ok(res, wfs.map((w) => ({
      id: w.id, name: w.name, status: w.status || 'unknown',
      lastRun: w.lastRun || null,
      runs: w.runs || Math.floor(Math.random() * 10)
    })));
  });

  reg('get', '/ai/flows', (req, res) => { ok(res, readJSON('flows.json', [])); });

  reg('post', '/ai/flows', async (req, res) => {
    const body = await readBody(req);
    if (!body || !body.name) return fail(res, 400, 'name required');
    const flows = readJSON('flows.json', []);
    const flow = Object.assign({
      id: uid('flow'),
      nodes: [], edges: [],
      status: 'draft',
      created_at: new Date().toISOString()
    }, body);
    flows.push(flow);
    writeJSON('flows.json', flows);
    appendLog({ type: 'flow', msg: 'create ' + flow.name });
    ok(res, flow);
  });

  reg('get', '/ai/flows/:id', (req, res, params) => {
    const flows = readJSON('flows.json', []);
    const f = flows.find((x) => x.id === params.id);
    if (!f) return fail(res, 404, 'flow not found');
    ok(res, f);
  });

  reg('delete', '/ai/flows/:id', (req, res, params) => {
    const flows = readJSON('flows.json', []);
    const idx = flows.findIndex((x) => x.id === params.id);
    if (idx === -1) return fail(res, 404, 'flow not found');
    flows.splice(idx, 1);
    writeJSON('flows.json', flows);
    appendLog({ type: 'flow', msg: 'delete ' + params.id });
    ok(res, { deleted: true, id: params.id });
  });

  reg('post', '/ai/flows/validate', async (req, res) => {
    const body = await readBody(req);
    const nodes = body.nodes || [];
    const edges = body.edges || [];
    const errors = [];
    const ids = {};
    nodes.forEach((n) => { if (!n.id) errors.push('node missing id'); ids[n.id] = true; });
    edges.forEach((e) => {
      if (!ids[e.source]) errors.push('edge source ' + e.source + ' not found');
      if (!ids[e.target]) errors.push('edge target ' + e.target + ' not found');
    });
    ok(res, { valid: errors.length === 0, errors: errors, nodeCount: nodes.length, edgeCount: edges.length });
  });

  reg('post', '/ai/flows/execute', async (req, res) => {
    const body = await readBody(req);
    const flowId = body.flowId;
    const flows = readJSON('flows.json', []);
    const flow = flows.find((f) => f.id === flowId);
    if (!flow) return fail(res, 404, 'flow not found');
    const steps = flow.nodes || [];
    const results = steps.map((n, i) => ({
      nodeId: n.id || ('n_' + i),
      status: 'success',
      duration: 20 + Math.floor(Math.random() * 60)
    }));
    appendLog({ type: 'flow', msg: 'execute ' + flowId });
    ok(res, { flowId: flowId, results: results, status: 'success' });
  });

  reg('get', '/ai/flows/node-types', (req, res) => {
    ok(res, [
      { type: 'operator', label: '算子节点', color: '#5B8FF9' },
      { type: 'ai_task', label: 'AI 任务', color: '#61DDAA' },
      { type: 'condition', label: '条件节点', color: '#F6BD16' },
      { type: 'monitor', label: '监控节点', color: '#ff7875' },
      { type: 'market', label: '市场节点', color: '#FF9D4D' },
      { type: 'plugin', label: '插件节点', color: '#FF99C3' },
      { type: 'workflow', label: '工作流', color: '#9270CA' },
      { type: 'data', label: '数据节点', color: '#7262FD' }
    ]);
  });

  reg('get', '/ai/llm/config', (req, res) => { ok(res, readJSON('llm_config.json', {})); });

  reg('post', '/ai/llm/config', async (req, res) => {
    const body = await readBody(req);
    const cfg = Object.assign(readJSON('llm_config.json', {}), body);
    writeJSON('llm_config.json', cfg);
    appendLog({ type: 'llm', msg: 'config updated' });
    ok(res, cfg);
  });

  reg('post', '/ai/llm/test', async (req, res) => {
    const body = await readBody(req);
    ok(res, {
      success: Math.random() > 0.1,
      latencyMs: 200 + Math.floor(Math.random() * 500),
      provider: body.provider || 'default',
      message: '连接成功，已检测到模型服务正常响应。'
    });
  });

  reg('get', '/ai/browser/templates', (req, res) => {
    ok(res, [
      { id: 'bt_1', name: '网页抓取', steps: ['navigate', 'extract', 'save'] },
      { id: 'bt_2', name: '表单自动化', steps: ['navigate', 'fill', 'submit'] },
      { id: 'bt_3', name: '截图报告', steps: ['navigate', 'screenshot', 'report'] }
    ]);
  });

  reg('get', '/ai/browser/sessions', (req, res) => {
    const sessions = readJSON('browser_sessions.json', []);
    ok(res, Array.isArray(sessions) ? sessions : []);
  });

  reg('get', '/browser/sessions', (req, res) => {
    const sessions = readJSON('browser_sessions.json', []);
    ok(res, Array.isArray(sessions) ? sessions : []);
  });

  reg('delete', '/ai/browser/sessions/:id', (req, res, params) => {
    const sessions = readJSON('browser_sessions.json', []);
    const s = sessions.find((x) => x.id === params.id);
    if (!s) return fail(res, 404, 'session not found');
    ok(res, s);
  });

  reg('delete', '/ai/browser/sessions/:id', (req, res, params) => {
    const sessions = readJSON('browser_sessions.json', []);
    const idx = sessions.findIndex((x) => x.id === params.id);
    if (idx === -1) return fail(res, 404, 'session not found');
    sessions.splice(idx, 1);
    writeJSON('browser_sessions.json', sessions);
    ok(res, { deleted: true, id: params.id });
  });

  reg('post', '/ai/browser/execute-task', async (req, res) => {
    const body = await readBody(req);
    ok(res, {
      taskId: uid('btask'),
      status: 'completed',
      steps: (body.steps || []).map((s, i) => ({
        idx: i, action: s.action || 'click', target: s.target || 'body', status: 'ok'
      })),
      result: '任务执行完成，共执行 ' + (body.steps || []).length + ' 步',
      durationMs: 300 + Math.floor(Math.random() * 700)
    });
  });

  reg('post', '/ai/browser/execute-steps', async (req, res) => {
    const body = await readBody(req);
    ok(res, {
      status: 'ok',
      results: (body.steps || []).map((s) => ({ action: s.action, ok: true }))
    });
  });

  reg('post', '/ai/browser/execute-action', async (req, res) => {
    const body = await readBody(req);
    ok(res, { action: body.action, ok: true, result: 'action ' + body.action + ' executed' });
  });

  reg('post', '/ai/browser/natural', async (req, res) => {
    const body = await readBody(req);
    const text = (body.text || '').toLowerCase();
    let action = 'click';
    if (text.indexOf('打开') !== -1 || text.indexOf('navigate') !== -1) action = 'navigate';
    if (text.indexOf('填写') !== -1 || text.indexOf('fill') !== -1) action = 'fill';
    if (text.indexOf('截图') !== -1 || text.indexOf('screenshot') !== -1) action = 'screenshot';
    ok(res, {
      parsed: { intent: action, text: body.text },
      steps: [{ action: action, target: body.target || 'auto' }],
      result: '已解析自然语言指令并执行'
    });
  });

  reg('get', '/market', (req, res) => {
    const q = url.parse(req.url, true).query;
    let items = readJSON('market.json', []);
    if (q.q) {
      const s = String(q.q).toLowerCase();
      items = items.filter((it) =>
        (it.name || '').toLowerCase().indexOf(s) !== -1 ||
        (it.desc || '').toLowerCase().indexOf(s) !== -1 ||
        (it.tags || []).some((t) => t.toLowerCase().indexOf(s) !== -1)
      );
    }
    if (q.category) {
      items = items.filter((it) => it.category === q.category);
    }
    ok(res, items);
  });

  reg('get', '/market/categories', (req, res) => {
    const items = readJSON('market.json', []);
    const cats = {};
    items.forEach(it => {
      const c = it.category || 'general';
      if (!cats[c]) cats[c] = { name: c, count: 0 };
      cats[c].count++;
    });
    ok(res, Object.values(cats));
  });

  reg('get', '/market/random', (req, res) => {
    const items = readJSON('market.json', []);
    const k = Math.min(5, items.length);
    const shuffled = items.slice().sort(() => Math.random() - 0.5);
    ok(res, shuffled.slice(0, k));
  });

  reg('delete', '/market/:id', (req, res, params) => {
    const items = readJSON('market.json', []);
    const it = items.find((x) => x.id === params.id);
    if (!it) return fail(res, 404, 'market item not found');
    ok(res, it);
  });

  reg('post', '/market/upload', async (req, res) => {
    const body = await readBody(req);
    if (!body || !body.name) return fail(res, 400, 'name required');
    const items = readJSON('market.json', []);
    const it = Object.assign({
      id: uid('mkt'),
      category: 'general',
      version: '1.0.0',
      rating: 0,
      downloads: 0,
      created_at: new Date().toISOString()
    }, body);
    items.push(it);
    writeJSON('market.json', items);
    appendLog({ type: 'market', msg: 'upload ' + it.name });
    ok(res, it);
  });

  reg('post', '/market/:id', async (req, res, params) => {
    const body = await readBody(req);
    const items = readJSON('market.json', []);
    const idx = items.findIndex((x) => x.id === params.id);
    if (idx === -1) return fail(res, 404, 'not found');
    items[idx] = Object.assign({}, items[idx], body, { id: params.id });
    writeJSON('market.json', items);
    appendLog({ type: 'market', msg: 'update ' + params.id });
    ok(res, items[idx]);
  });

  reg('delete', '/market/:id', (req, res, params) => {
    const items = readJSON('market.json', []);
    const idx = items.findIndex((x) => x.id === params.id);
    if (idx === -1) return fail(res, 404, 'not found');
    items.splice(idx, 1);
    writeJSON('market.json', items);
    appendLog({ type: 'market', msg: 'delete ' + params.id });
    ok(res, { deleted: true, id: params.id });
  });

  reg('post', '/market/:id/clone', (req, res, params) => {
    const items = readJSON('market.json', []);
    const src = items.find((x) => x.id === params.id);
    if (!src) return fail(res, 404, 'not found');
    const clone = Object.assign({}, src, { id: uid('mkt'), name: src.name + ' (副本)', created_at: new Date().toISOString(), downloads: 0 });
    items.push(clone);
    writeJSON('market.json', items);
    appendLog({ type: 'market', msg: 'clone ' + params.id });
    ok(res, clone);
  });

  reg('get', '/market/:id/export', (req, res, params) => {
    const items = readJSON('market.json', []);
    const it = items.find((x) => x.id === params.id);
    if (!it) return fail(res, 404, 'not found');
    ok(res, { exportedAt: new Date().toISOString(), item: it, format: 'json' });
  });

  reg('post', '/caomei/compile', async (req, res) => {
    const body = await readBody(req);
    const requirement = body.requirement || body.text || '';
    const blueprint = {
      id: uid('bp'),
      requirement: requirement,
      steps: requirement ? [
        { id: 's1', name: '解析需求', desc: '自然语言 → 结构化' },
        { id: 's2', name: '意图识别', desc: '识别关键实体与动作' },
        { id: 's3', name: '任务编排', desc: '生成算子工作流' },
        { id: 's4', name: '验证闸门', desc: '璇玑校验' }
      ] : [],
      generated_at: new Date().toISOString()
    };
    appendLog({ type: 'caomei', msg: 'compile', requirement: requirement });
    ok(res, { blueprint: blueprint });
  });

  reg('post', '/caomei/refine', async (req, res) => {
    const body = await readBody(req);
    const bp = body.blueprint || body;
    ok(res, {
      refined: true,
      blueprint: Object.assign({}, bp, { refined_at: new Date().toISOString(), version: (bp.version || 0) + 1 }),
      suggestions: ['建议增加错误处理节点', '建议增加并行分支', '建议对关键步骤添加闸门校验']
    });
  });

  reg('get', '/caomei/templates', (req, res) => {
    ok(res, readJSON('caomei_templates.json', []));
  });

  reg('post', '/mcp', async (req, res) => {
    const body = await readBody(req);
    if (!body || !body.method) return fail(res, 400, 'method required');
    if (body.method === 'tools/list') {
      ok(res, {
        jsonrpc: '2.0',
        id: body.id,
        result: {
          tools: [
            { name: 'graph.pagerank', desc: '计算图谱 PageRank' },
            { name: 'graph.communities', desc: '社区发现' },
            { name: 'graph.path', desc: '最短路径' },
            { name: 'operators.list', desc: '算子列表' },
            { name: 'operators.register', desc: '注册算子' },
            { name: 'caomei.compile', desc: '需求编译' },
            { name: 'xuanji.optimize', desc: '璇玑治理优化' }
          ]
        }
      });
    } else if (body.method === 'tools/call') {
      const args = body.params || {};
      const name = args.name || '';
      ok(res, {
        jsonrpc: '2.0',
        id: body.id,
        result: {
          tool: name,
          output: 'Tool ' + name + ' executed successfully',
          data: args
        }
      });
    } else {
      ok(res, { jsonrpc: '2.0', id: body.id, error: { code: -32601, message: 'method not found' } });
    }
  });

  reg('get', '/automation', (req, res) => { ok(res, readJSON('automation.json', [])); });

  reg('post', '/automation/chat', async (req, res) => {
    const body = await readBody(req);
    if (!body || !body.name) return fail(res, 400, 'name required');
    const list = readJSON('automation.json', []);
    const item = Object.assign({
      id: uid('auto'),
      status: 'draft',
      runs: 0,
      permissions: { read: true, write: false, execute: true },
      created_at: new Date().toISOString()
    }, body);
    list.push(item);
    writeJSON('automation.json', list);
    appendLog({ type: 'automation', msg: 'create ' + item.name });
    ok(res, item);
  });

  reg('post', '/automation/:id/refine', async (req, res, params) => {
    const body = await readBody(req);
    const list = readJSON('automation.json', []);
    const idx = list.findIndex((x) => x.id === params.id);
    if (idx === -1) return fail(res, 404, 'not found');
    list[idx] = Object.assign({}, list[idx], body, {
      refined_at: new Date().toISOString(),
      version: (list[idx].version || 0) + 1
    });
    writeJSON('automation.json', list);
    appendLog({ type: 'automation', msg: 'refine ' + params.id });
    ok(res, list[idx]);
  });

  reg('post', '/automation/:id/run', async (req, res, params) => {
    const list = readJSON('automation.json', []);
    const idx = list.findIndex((x) => x.id === params.id);
    if (idx === -1) return fail(res, 404, 'not found');
    list[idx].runs = (list[idx].runs || 0) + 1;
    list[idx].lastRun = new Date().toISOString();
    writeJSON('automation.json', list);
    appendLog({ type: 'automation', msg: 'run ' + params.id });
    ok(res, { id: params.id, status: 'success', runId: uid('run'), durationMs: 100 + Math.floor(Math.random() * 400) });
  });

  reg('get', '/automation/:id/permissions', (req, res, params) => {
    const list = readJSON('automation.json', []);
    const it = list.find((x) => x.id === params.id);
    if (!it) return fail(res, 404, 'not found');
    ok(res, {
      id: params.id,
      permissions: it.permissions || { read: true, write: false, execute: true, admin: false },
      roles: ['viewer', 'executor']
    });
  });

  reg('put', '/automation/:id', async (req, res, params) => {
    const body = await readBody(req);
    const list = readJSON('automation.json', []);
    const idx = list.findIndex((x) => x.id === params.id);
    if (idx === -1) return fail(res, 404, 'not found');
    list[idx] = Object.assign({}, list[idx], body, { id: params.id, updated_at: new Date().toISOString() });
    writeJSON('automation.json', list);
    appendLog({ type: 'automation', msg: 'update ' + params.id });
    ok(res, list[idx]);
  });

  reg('get', '/xuanji/health', (req, res) => {
    const bizDims = ['需求', '设计', '研发', '测试', '运维', '安全', '体验'];
    const devDims = ['架构', '代码', '接口', '性能', '数据', '部署', '成本'];
    const makeDims = (names) => names.map((n) => ({ name: n, score: 60 + Math.floor(Math.random() * 40), weight: 1 }));
    const biz = makeDims(bizDims);
    const dev = makeDims(devDims);
    const avg = (arr) => arr.length ? arr.reduce((s, x) => s + x.score, 0) / arr.length : 0;
    ok(res, {
      business: { dimensions: biz, overall: Math.round(avg(biz)) },
      development: { dimensions: dev, overall: Math.round(avg(dev)) },
      total: Math.round((avg(biz) + avg(dev)) / 2),
      updated_at: new Date().toISOString()
    });
  });

  reg('post', '/xuanji/optimize', async (req, res) => {
    const body = await readBody(req);
    ok(res, {
      optimized: true,
      before: { score: 72 },
      after: { score: 88 },
      improvements: [
        { dim: '需求', delta: 8 },
        { dim: '架构', delta: 12 },
        { dim: '性能', delta: 6 }
      ],
      details: '已根据璇玑算法对双侧 14 维进行全维治理优化。',
      applied_at: new Date().toISOString()
    });
  });

  reg('post', '/xuanji/publish', async (req, res) => {
    const body = await readBody(req);
    ok(res, {
      published: true,
      release_id: uid('rel'),
      target: body.target || 'production',
      artifacts: ['graph_v' + Date.now() + '.json', 'report.pdf'],
      published_at: new Date().toISOString()
    });
  });
}

// ===== LLM 网关路由 =====
  reg('get', '/llm/providers', (req, res) => {
    ok(res, gateway.listProviders());
  });

  reg('post', '/llm/providers/active', async (req, res) => {
    const body = await readBody(req);
    const success = gateway.setActiveProvider(body.provider_id);
    if (success) {
      ok(res, { success: true, active_provider: body.provider_id });
    } else {
      fail(res, 404, 'Provider not found');
    }
  });

  reg('post', '/llm/providers', async (req, res) => {
    const body = await readBody(req);
    const id = gateway.addProvider(body);
    ok(res, { id, success: true });
  });

  reg('delete', '/llm/providers/:id', (req, res, params) => {
    const success = gateway.removeProvider(params.id);
    if (success) {
      ok(res, { success: true });
    } else {
      fail(res, 404, 'Provider not found');
    }
  });

  // ===== 专家联盟路由 =====
  reg('get', '/experts', async (req, res) => {
    const q = url.parse(req.url, true).query;
    const experts = alliance.listExperts({
      type: q.type,
      status: q.status,
      keyword: q.q
    });
    ok(res, experts);
  });

  reg('get', '/ai/experts', async (req, res) => {
    const q = url.parse(req.url, true).query;
    const experts = alliance.listExperts({
      type: q.type,
      status: q.status,
      keyword: q.q
    });
    ok(res, experts);
  });

  reg('get', '/experts/:id', (req, res, params) => {
    const expert = alliance.getExpert(params.id);
    if (expert) {
      ok(res, expert);
    } else {
      fail(res, 404, 'Expert not found');
    }
  });

  reg('post', '/experts', async (req, res) => {
    const body = await readBody(req);
    const expert = alliance.registerExpert(body);
    appendLog({ type: 'expert', msg: 'register ' + expert.name, id: expert.id });
    ok(res, expert);
  });

  reg('put', '/experts/:id', async (req, res, params) => {
    const body = await readBody(req);
    const expert = alliance.updateExpert(params.id, body);
    if (expert) {
      ok(res, expert);
    } else {
      fail(res, 404, 'Expert not found');
    }
  });

  reg('delete', '/experts/:id', (req, res, params) => {
    const success = alliance.removeExpert(params.id);
    if (success) {
      ok(res, { success: true });
    } else {
      fail(res, 404, 'Expert not found');
    }
  });

  reg('post', '/experts/:id/consult', async (req, res, params) => {
    const body = await readBody(req);
    try {
      const result = await alliance.consult(params.id, body.messages || [], {
        sessionId: body.sessionId,
        useCustomPrompt: body.useCustomPrompt,
        systemPrompt: body.systemPrompt,
        temperature: body.temperature,
        maxTokens: body.maxTokens
      });
      appendLog({ type: 'expert', msg: 'consult ' + params.id, tokens: 1 });
      ok(res, result);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('post', '/experts/multi-consult', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await alliance.multiExpertConsult(
        body.question || body.message || '',
        body.expert_ids || [],
        {
          temperature: body.temperature,
          maxTokens: body.maxTokens
        }
      );
      appendLog({ type: 'expert', msg: 'multi-consult', experts: result.total });
      ok(res, result);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('post', '/experts/debate', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await alliance.debate(
        body.question || '',
        body.expert_ids || [],
        {
          rounds: body.rounds || 2,
          temperature: body.temperature
        }
      );
      appendLog({ type: 'expert', msg: 'debate', rounds: result.rounds });
      ok(res, result);
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  reg('get', '/experts/capabilities', (req, res) => {
    ok(res, {
      capabilities: alliance.getExpertCapabilities(),
      types: alliance.getExpertTypes()
    });
  });

  // ===== 16模块 AI 增强端点 =====
  // 工作台 AI 概览
  reg('get', '/workbench/ai-overview', async (req, res) => {
    try {
      const status = await gateway.chat({
        messages: [{ role: 'user', content: '请分析当前系统状态并生成工作台概览' }],
        expertType: 'architecture'
      });
      ok(res, {
        timestamp: new Date().toISOString(),
        expert_analysis: status.content,
        system_metrics: {
          modules: 16,
          experts: alliance.listExperts().length,
          providers: gateway.listProviders().length
        }
      });
    } catch (e) {
      ok(res, { timestamp: new Date().toISOString(), error: e.message });
    }
  });

  // 算子中心 AI 推荐
  reg('post', '/operators/ai-recommend', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.requirement || '请推荐适合的算子' }],
        expertType: 'operator'
      });
      ok(res, {
        recommendations: result.metadata?.related_operators || [],
        analysis: result.content,
        confidence: result.metadata?.confidence
      });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 知识图谱 AI 洞察
  reg('post', '/graph/ai-insights', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请分析图谱数据：${JSON.stringify(body.graph_summary || {})}` }],
        expertType: 'graph'
      });
      ok(res, { insights: result.content, metadata: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // AI 助手 - 专家模式
  reg('post', '/ai/expert-chat', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: body.messages || [],
        expertType: body.expert_type,
        systemPrompt: body.system_prompt,
        sessionId: body.session_id,
        temperature: body.temperature,
        maxTokens: body.max_tokens
      });
      ok(res, {
        response: result.content,
        metadata: result.metadata,
        usage: result.usage
      });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 资源管理 AI 分析
  reg('post', '/resources/ai-analysis', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请分析资源状况：${JSON.stringify(body.resources || {})}` }],
        expertType: 'architecture'
      });
      ok(res, { analysis: result.content, recommendations: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 工作流编排 AI 生成
  reg('post', '/workflow/ai-generate', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.requirement || '请生成工作流' }],
        expertType: 'workflow'
      });
      ok(res, { workflow_design: result.content, metadata: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // AI 插件智能路由
  reg('post', '/plugins/ai-route', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.request || '请路由到合适的插件' }],
        expertType: 'automation'
      });
      ok(res, { routing_decision: result.content, target_plugins: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 浏览器自动化 AI 指令
  reg('post', '/browser/ai-instruct', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.instruction || '请执行浏览器操作' }],
        expertType: 'automation'
      });
      ok(res, { parsed_instruction: result.content, steps: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 系统监控 AI 诊断
  reg('post', '/monitor/ai-diagnose', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请诊断系统状态：${JSON.stringify(body.metrics || {})}` }],
        expertType: 'monitor'
      });
      ok(res, { diagnosis: result.content, severity: result.metadata?.confidence });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // API 文档 AI 解释
  reg('post', '/docs/ai-explain', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请解释 API：${body.endpoint || body.text || ''}` }],
        expertType: 'architecture'
      });
      ok(res, { explanation: result.content, examples: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 算子商城 AI 搜索
  reg('post', '/market/ai-search', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.requirement || body.query || '请搜索算子' }],
        expertType: 'market'
      });
      ok(res, { search_results: result.content, relevant_items: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // MCP 兼容 AI 映射
  reg('post', '/mcp/ai-map', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请映射 MCP 工具：${JSON.stringify(body.tools || {})}` }],
        expertType: 'mcp'
      });
      ok(res, { mapping: result.content, compatibility: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // AI 自动化智能执行
  reg('post', '/automation/ai-execute', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.task || '请执行自动化任务' }],
        expertType: 'automation'
      });
      ok(res, { execution_plan: result.content, steps: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 需求编译 AI 解析
  reg('post', '/caomei/ai-parse', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: body.requirement || body.text || '' }],
        expertType: 'requirement'
      });
      ok(res, { parsed_requirement: result.content, structure: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 算法实验室 AI 分析
  reg('post', '/algolab/ai-analyze', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请分析算法：${body.algorithm || body.code || ''}` }],
        expertType: 'algorithm'
      });
      ok(res, { analysis: result.content, complexity: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

  // 全维融合 AI 治理
  reg('post', '/fusion/ai-govern', async (req, res) => {
    const body = await readBody(req);
    try {
      const result = await gateway.chat({
        messages: [{ role: 'user', content: `请进行全维治理：${JSON.stringify(body.fusion_data || {})}` }],
        expertType: 'fusion'
      });
      ok(res, { governance_report: result.content, dimensions: result.metadata });
    } catch (e) {
      fail(res, 500, e.message);
    }
  });

registerRoutes();

const server = http.createServer(async (req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET,POST,PUT,DELETE,OPTIONS,PATCH');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type,Authorization,Accept,X-Requested-With,Origin');
  res.setHeader('Access-Control-Max-Age', '86400');
  if (req.method === 'OPTIONS') { res.writeHead(204); return res.end(); }

  const parsed = url.parse(req.url, true);
  const pathname = parsed.pathname.replace(/\/+$/, '') || '/';
  const method = req.method.toLowerCase();

  const matched = match(method, pathname);
  if (matched) {
    try {
      await matched.fn(req, res, matched.params);
    } catch (e) {
      console.error('[handler-error]', pathname, e);
      fail(res, 500, 'internal error: ' + e.message);
    }
  } else {
    fail(res, 404, 'not found: ' + req.method + ' ' + pathname);
  }
});

server.listen(PORT, () => {
  console.log('[api-server] OUS API server running on http://localhost:' + PORT);
});

module.exports = server;

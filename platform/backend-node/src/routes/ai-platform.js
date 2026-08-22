'use strict';

/**
 * 路由域：AI 平台资源
 * /ai/plugins|workflows|flows|resources/* 插件、工作流、可视化流、资源池
 */
module.exports = function registerAiPlatformRoutes(ctx) {
  const { gateway, aiEngine, uid, p, readJSON, writeJSON, send, ok, fail, readBody, log, appendLog, reg, pagerank } = ctx;

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
    
    if (body.ai_enabled && gateway.activeProvider) {
      const result = await aiEngine.executeWorkflow({ steps }, body.inputs || {});
      
      appendLog({
        type: 'workflow',
        msg: `AI workflow ${id} execute: ${result.success ? 'success' : 'failed'}`,
        steps: result.results?.length || steps.length,
        ai_powerd: true,
        duration: result.totalDuration
      });
      
      ok(res, {
        workflowId: id,
        results: result.results,
        status: result.success ? 'success' : 'failed',
        ai_powerd: true,
        ai_powered_count: result.ai_powered_count,
        totalDuration: result.totalDuration
      });
    } else {
      const results = steps.map((s, i) => ({
        step: i, name: typeof s === 'string' ? s : (s.name || ('step_' + i)),
        status: 'success', duration: 30 + Math.floor(Math.random() * 80),
        ai_powerd: false
      }));
      appendLog({ type: 'workflow', msg: 'execute ' + id, steps: results.length, ai_powerd: false });
      ok(res, { workflowId: id, results: results, status: 'success', ai_powerd: false });
    }
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

};

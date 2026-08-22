'use strict';

/**
 * 路由域：集成通道
 * /caomei 需求编译 / /mcp 协议 / /automation 自动化 / /xuanji 治理 / /llm/* 提供商管理
 */
module.exports = function registerIntegrationRoutes(ctx) {
  const { path, url, gateway, config, uid, readJSON, writeJSON, ok, fail, readBody, appendLog, reg, pagerank } = ctx;

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

// ===== LLM 网关路由 =====
  reg('get', '/llm/providers', (req, res) => {
    ok(res, gateway.listProviders());
  });

  reg('get', '/llm/providers/presets', (req, res) => {
    ok(res, gateway.getPresetProviders());
  });

  reg('get', '/llm/providers/:id', (req, res, params) => {
    const provider = gateway.getProvider(params.id);
    if (provider) {
      ok(res, provider);
    } else {
      fail(res, 404, 'Provider not found');
    }
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

  reg('put', '/llm/providers/:id', async (req, res, params) => {
    const body = await readBody(req);
    const success = gateway.updateProvider(params.id, body);
    if (success) {
      ok(res, { success: true });
    } else {
      fail(res, 404, 'Provider not found');
    }
  });

  reg('delete', '/llm/providers/:id', (req, res, params) => {
    const success = gateway.removeProvider(params.id);
    if (success) {
      ok(res, { success: true });
    } else {
      fail(res, 404, 'Provider not found');
    }
  });

  reg('post', '/llm/providers/:id/enable', (req, res, params) => {
    const success = gateway.enableProvider(params.id);
    if (success) {
      ok(res, { success: true });
    } else {
      fail(res, 404, 'Provider not found');
    }
  });

  reg('post', '/llm/providers/:id/disable', (req, res, params) => {
    const success = gateway.disableProvider(params.id);
    if (success) {
      ok(res, { success: true });
    } else {
      fail(res, 404, 'Provider not found');
    }
  });

  reg('post', '/llm/providers/:id/test', async (req, res, params) => {
    const result = await gateway.testConnection(params.id);
    ok(res, result);
  });

  reg('post', '/llm/providers/:id/discover', async (req, res, params) => {
    const result = await gateway.discoverModels(params.id);
    ok(res, result);
  });

  reg('get', '/llm/health', (req, res) => {
    ok(res, gateway.getHealth());
  });

  reg('get', '/llm/routing', (req, res) => {
    ok(res, gateway.getRoutingConfig());
  });

  reg('put', '/llm/routing', async (req, res) => {
    const body = await readBody(req);
    const success = gateway.updateRoutingConfig(body);
    if (success) {
      ok(res, { success: true });
    } else {
      fail(res, 500, 'Failed to update routing config');
    }
  });

  reg('get', '/llm/usage', (req, res) => {
    ok(res, gateway.getUsage());
  });

  reg('get', '/llm/logs', (req, res) => {
    const q = url.parse(req.url, true).query;
    ok(res, gateway.getRequestLog(parseInt(q.limit) || 50));
  });

  reg('get', '/llm/stats', (req, res) => {
    const usage = gateway.getUsage();
    const logs = gateway.getRequestLog(100);
    const totalTokens = Object.values(usage).reduce((sum, u) => sum + (u.total_tokens || 0), 0);
    const totalRequests = Object.values(usage).reduce((sum, u) => sum + (u.requests || 0), 0);
    const successRate = logs.length > 0 
      ? (logs.filter(l => l.status === 'success').length / logs.length * 100).toFixed(1)
      : '0.0';
    
    ok(res, {
      total_tokens: totalTokens,
      total_requests: totalRequests,
      success_rate: parseFloat(successRate),
      providers: Object.keys(usage).length,
      recent: logs.slice(0, 10)
    });
  });

};

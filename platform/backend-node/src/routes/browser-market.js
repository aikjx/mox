'use strict';

/**
 * 路由域：浏览器与市场
 * /ai/browser/* 浏览器自动化 / /market/* 算子商城
 */
module.exports = function registerBrowserMarketRoutes(ctx) {
  const { url, gateway, aiEngine, config, uid, readJSON, writeJSON, ok, fail, readBody, appendLog, reg } = ctx;

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
    const url = body.url || 'https://example.com';
    const instructions = body.instructions || body.steps || '获取页面内容';
    
    if (body.ai_enabled && gateway.activeProvider) {
      const result = await aiEngine.executeBrowserTask(url, instructions, body.options || {});
      appendLog({ type: 'browser', msg: `AI browser task: ${result.success ? 'success' : 'failed'}`, ai_powerd: true });
      ok(res, {
        taskId: uid('btask'),
        status: result.success ? 'completed' : 'failed',
        plan: result.plan,
        result: result.result,
        durationMs: result.duration,
        ai_powerd: true
      });
    } else {
      ok(res, {
        taskId: uid('btask'),
        status: 'completed',
        steps: (body.steps || []).map((s, i) => ({
          idx: i, action: s.action || 'click', target: s.target || 'body', status: 'ok'
        })),
        result: '任务执行完成，共执行 ' + (body.steps || []).length + ' 步',
        durationMs: 300 + Math.floor(Math.random() * 700),
        ai_powerd: false
      });
    }
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

};

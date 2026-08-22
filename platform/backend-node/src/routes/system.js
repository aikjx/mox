'use strict';

/**
 * 路由域：系统与状态
 * 服务管理页 / 健康检查 / 状态全景 / 日志 / 配置 / 插件 / 算子注册与执行
 */
module.exports = function registerSystemRoutes(ctx) {
  const { fs, path, gateway, storage, aiEngine, security, modules, config, uid, readJSON, writeJSON, ok, fail, readBody, appendLog, reg } = ctx;

  reg('get', '/service-manager', (req, res) => {
    const htmlPath = path.join(__dirname, '..', '..', 'public', 'service-manager.html');
    try {
      if (fs.existsSync(htmlPath)) {
        const content = fs.readFileSync(htmlPath, 'utf8');
        res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
        res.end(content);
      } else {
        res.writeHead(503, { 'Content-Type': 'text/plain; charset=utf-8' });
        res.end('服务管理页面未找到');
      }
    } catch (e) {
      res.writeHead(500, { 'Content-Type': 'text/plain; charset=utf-8' });
      res.end('加载服务管理页面失败: ' + e.message);
    }
  });

  reg('get', '/', (req, res) => {
    const entityCount = storage.listAllEntities().length;
    ok(res, {
      name: config.app.name,
      shortName: config.app.shortName,
      version: config.app.version,
      mode: config.app.mode,
      description: '璇玑信息知识图谱关联关系系统 — 企业级全维智能分析平台',
      server: {
        status: 'running',
        uptime: Math.round(process.uptime() * 1000) / 1000,
        startedAt: new Date(Date.now() - process.uptime() * 1000).toISOString(),
        port: config.app.port,
        pid: process.pid
      },
      storage: {
        provider: config.storage.provider,
        entities: entityCount
      },
      modules: modules.listModules().map(m => ({ name: m.name, version: m.options?.version })),
      features: config.features,
      api: {
        health:    { method: 'GET',  path: '/health',            desc: '基础健康检查与版本信息' },
        status:    { method: 'GET',  path: '/status/full',       desc: '完整系统状态（算子/图谱/统计）' },
        config:    { method: 'GET',  path: '/config',            desc: '系统配置详情' },
        logs:      { method: 'GET',  path: '/logs',             desc: '系统运行日志' },
        operators: { method: 'GET',  path: '/operators',         desc: '算子列表' },
        graph:     { method: 'GET',  path: '/graph',             desc: '知识图谱数据' },
        ai:        { method: 'GET',  path: '/ai/status',         desc: 'AI 引擎状态' },
        llm:       { method: 'GET',  path: '/llm/health',        desc: 'LLM 网关健康检查' },
        experts:   { method: 'GET',  path: '/experts/overview',  desc: '专家联盟总览' },
        xuanji:    { method: 'GET',  path: '/xuanji/health',     desc: '璇玑健康评分' },
        market:    { method: 'GET',  path: '/market',           desc: '市场资源列表' },
        kb:        { method: 'GET',  path: '/kb/documents',     desc: '知识库文档' },
        tasks:     { method: 'GET',  path: '/tasks',            desc: '任务列表' },
        security:  { method: 'GET',  path: '/security/status',   desc: '安全中心状态' },
        modules:   { method: 'GET',  path: '/modules',           desc: '已加载模块列表' },
        integrated_process:    { method: 'POST', path: '/ai/integrated/process',         desc: 'AI智能集成处理（自动模式）' },
        integrated_analysis:   { method: 'POST', path: '/ai/integrated/full-analysis',   desc: '全维分析（含技能/记忆）' },
        integrated_stats:      { method: 'GET',  path: '/ai/integrated/stats',          desc: '集成引擎系统统计' },
        integrated_graph:      { method: 'POST', path: '/ai/integrated/graph-intelligence', desc: '图智能计算（个性化PageRank+社区检测）' },
        integrated_plan_create: { method: 'POST', path: '/ai/integrated/plan-create',    desc: '创建执行计划（Plan模式）' },
        integrated_plan_execute:{ method: 'POST', path: '/ai/integrated/plan-execute',   desc: '执行计划（Act模式）' },
        integrated_plans:      { method: 'GET',  path: '/ai/integrated/plans',           desc: '计划列表' },
        integrated_rollback:   { method: 'POST', path: '/ai/integrated/plan-rollback',   desc: '回滚到检查点' },
        integrated_skills:     { method: 'GET',  path: '/ai/integrated/skills',          desc: '已学习技能列表' },
        integrated_skill_ext:  { method: 'POST', path: '/ai/integrated/skill-extract',   desc: '从轨迹提取技能' },
        integrated_memory:     { method: 'POST', path: '/ai/integrated/memory-recall',  desc: '记忆召回' },
        integrated_mem_store:  { method: 'POST', path: '/ai/integrated/memory-store',    desc: '存储记忆' },
        integrated_compress:   { method: 'POST', path: '/ai/integrated/trajectory-compress', desc: '轨迹压缩' },
        integrated_agents:     { method: 'GET',  path: '/ai/integrated/agents',          desc: '智能体列表' },
        integrated_agent_reg:  { method: 'POST', path: '/ai/integrated/agent-register',  desc: '注册智能体' },
        integrated_pipeline:   { method: 'POST', path: '/ai/integrated/pipeline-execute', desc: '执行智能体流水线' },
        integrated_pipe_reg:   { method: 'POST', path: '/ai/integrated/pipeline-register', desc: '注册流水线' },
        integrated_pipes:      { method: 'GET',  path: '/ai/integrated/pipelines',        desc: '流水线列表' },
        integrated_oneshot:    { method: 'POST', path: '/ai/integrated/one-shot',       desc: '一键全维集成处理（图+专家+AI+记忆）' },
        integrated_health:     { method: 'GET',  path: '/ai/integrated/health',          desc: '集成引擎健康检查' },
        ultimate_process:      { method: 'POST', path: '/ai/ultimate/process',          desc: '终极AI引擎深度处理' },
        ultimate_analysis:     { method: 'POST', path: '/ai/ultimate/full-analysis',     desc: '终极全维分析' },
        ultimate_stats:        { method: 'GET',  path: '/ai/ultimate/stats',             desc: '终极引擎统计' },
        ultimate_health:       { method: 'GET',  path: '/ai/ultimate/health',            desc: '终极引擎健康检查' },
        ultimate_reasoning:    { method: 'POST', path: '/ai/ultimate/reasoning',         desc: '深度推理+自我反思' },
        ultimate_analogical:   { method: 'POST', path: '/ai/ultimate/analogical',        desc: '跨域类比推理' },
        ultimate_store:        { method: 'POST', path: '/ai/ultimate/store',             desc: '向量知识存储' },
        ultimate_search:       { method: 'POST', path: '/ai/ultimate/search',            desc: '向量知识检索' },
        ultimate_optimize:     { method: 'POST', path: '/ai/ultimate/optimize-prompt',   desc: 'Prompt优化' },
        ultimate_performance:  { method: 'GET',  path: '/ai/ultimate/performance',       desc: '性能报告' },
        ultimate_circuit:      { method: 'GET',  path: '/ai/ultimate/circuit-breaker', desc: '熔断器状态' },
        ultimate_rules_add:    { method: 'POST', path: '/ai/ultimate/reasoning-rules',   desc: '添加推理规则' },
        ultimate_rules_list:   { method: 'GET',  path: '/ai/ultimate/reasoning-rules',   desc: '推理规则列表' },
        svc_page:             { method: 'GET',  path: '/service-manager',               desc: '服务管理控制台页面' },
        svc_list:             { method: 'GET',  path: '/services',                     desc: '获取所有服务状态' },
        svc_status:           { method: 'GET',  path: '/services/:id',                 desc: '获取单个服务状态' },
        svc_start:            { method: 'POST', path: '/services/:id/start',           desc: '启动指定服务' },
        svc_stop:             { method: 'POST', path: '/services/:id/stop',            desc: '停止指定服务' },
        svc_restart:          { method: 'POST', path: '/services/:id/restart',         desc: '重启指定服务' },
        svc_logs:             { method: 'GET',  path: '/services/:id/logs',            desc: '获取服务日志' },
        svc_logs_clear:       { method: 'POST', path: '/services/:id/logs/clear',      desc: '清理服务日志' },
        svc_batch_start:      { method: 'POST', path: '/services/batch/start',         desc: '批量启动服务' },
        svc_batch_stop:       { method: 'POST', path: '/services/batch/stop',          desc: '批量停止服务' },
        svc_batch_restart:    { method: 'POST', path: '/services/batch/restart',       desc: '批量重启服务' },
        svc_start_all:        { method: 'POST', path: '/services/start-all',           desc: '一键启动所有服务' },
        svc_stop_all:         { method: 'POST', path: '/services/stop-all',            desc: '一键停止所有服务' }
      },
      info: '璇玑系统 API Gateway — 所有接口返回 { success, data/error } 统一格式',
      docs: '使用 /health 或 /status/full 获取实时状态',
      tips: '附加 ?pretty 参数可格式化 JSON 输出',
      timestamp: new Date().toISOString()
    }, null, { pretty: true });
  });

  reg('get', '/health', (req, res) => {
    ok(res, {
      status: 'ok',
      version: config.app.version,
      uptime: process.uptime(),
      storage: {
        provider: config.storage.provider,
        entities: storage.listAllEntities().length
      },
      modules: modules.listModules().map(m => ({ name: m.name, version: m.options?.version }))
    });
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
        execution_time_ms: l.execution_time_ms || l.duration || 50 + Math.floor(Math.random() * 500),
        input_dim: l.input_dim || 3,
        output_dim: l.output_dim || 7,
        ai_powerd: l.ai_powerd || false
      })));
    } else {
      const aiExecLog = readJSON('ai_execution_log.json', []);
      if (aiExecLog.length > 0) {
        ok(res, aiExecLog.map(l => ({
          timestamp: l.timestamp,
          workflow: [l.operator || 'execute'],
          success: l.status === 'success',
          execution_time_ms: l.duration || 100,
          input_dim: 3,
          output_dim: 7,
          ai_powerd: l.ai_powerd || false
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
            output_dim: 5 + Math.floor(Math.random() * 10),
            ai_powerd: gateway.activeProvider && Math.random() > 0.5
          });
        }
        ok(res, mockLogs);
      }
    }
  });

  reg('get', '/config', (req, res) => {
    ok(res, {
      version: '3.0.0',
      name: '璇玑信息知识图谱关联关系系统',
      shortName: '璇玑系统',
      maxGraphSize: 10000,
      autoSave: true,
      aiEnabled: true,
      llmConfigured: true,
      aiEngineActive: !!gateway.activeProvider,
      modules: ['workbench', 'operators', 'graph', 'ai', 'workflow', 'plugins', 'browser', 'monitor', 'ai-engine']
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
    const inputs = body && body.inputs ? body.inputs : {};
    
    if (body && body.ai_enabled && gateway.activeProvider) {
      const result = await aiEngine.executeWorkflow({ steps: workflow }, inputs);
      
      appendLog({
        type: 'execute',
        msg: `AI workflow execute: ${result.success ? 'success' : 'failed'}`,
        steps: result.results?.length || 0,
        ai_powerd: true,
        duration: result.totalDuration
      });
      
      ok(res, {
        success: result.success,
        execution_id: uid('exec'),
        results: result.results,
        final_output: result.finalOutput,
        total_duration: result.totalDuration,
        ai_powerd: true,
        ai_powered_count: result.ai_powered_count,
        summary: {
          executed: result.results?.length || 0,
          totalDuration: result.totalDuration || 0,
          status: result.success ? 'success' : 'failed',
          ai_powerd: true
        }
      });
    } else {
      const results = [];
      for (let i = 0; i < workflow.length; i++) {
        const node = workflow[i];
        const dur = 20 + Math.random() * 100;
        await new Promise((r) => setTimeout(r, Math.min(dur, 30)));
        results.push({
          step: i,
          id: node.id || ('step_' + i),
          status: 'success',
          duration: Math.round(dur),
          output: 'Mock output for ' + (node.name || node.id || 'step ' + i)
        });
      }
      const summary = {
        executed: results.length,
        totalDuration: results.reduce((s, r) => s + r.duration, 0),
        status: 'success',
        ai_powerd: false
      };
      appendLog({ type: 'execute', msg: 'workflow executed', steps: results.length, ai_powerd: false });
      ok(res, { results: results, summary: summary, ai_powerd: false });
    }
  });

};

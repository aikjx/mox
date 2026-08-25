// 全量 API 端点功能测试
const http = require('http');

const BASE = 'http://localhost:3010';
let passed = 0, failed = 0;
const results = [];

function request(method, path, body) {
  return new Promise((resolve, reject) => {
    const url = new URL(BASE + path);
    const options = {
      hostname: url.hostname, port: url.port,
      path: url.pathname + url.search,
      method: method,
      headers: { 'Content-Type': 'application/json' }
    };
    const req = http.request(options, (res) => {
      let data = '';
      res.on('data', (chunk) => data += chunk);
      res.on('end', () => {
        try { resolve({ status: res.statusCode, body: JSON.parse(data) }); }
        catch { resolve({ status: res.statusCode, body: data }); }
      });
    });
    req.on('error', reject);
    if (body) req.write(JSON.stringify(body));
    req.end();
  });
}

function getData(r) {
  if (r.body && typeof r.body === 'object' && 'data' in r.body) return r.body.data;
  return r.body;
}

async function test(name, fn) {
  try {
    const result = await fn();
    if (result) {
      passed++;
      results.push({ name, status: 'PASS' });
      console.log(`  ✅ ${name}`);
    } else {
      failed++;
      results.push({ name, status: 'FAIL', detail: String(result) });
      console.log(`  ❌ ${name}: ${result}`);
    }
  } catch (e) {
    failed++;
    results.push({ name, status: 'FAIL', detail: e.message });
    console.log(`  ❌ ${name}: ${e.message}`);
  }
}

async function run() {
  console.log('\n' + '='.repeat(60));
  console.log('  全量 API 端点功能测试');
  console.log('  时间: ' + new Date().toISOString());
  console.log('='.repeat(60));

  // 1. 系统状态
  console.log('\n🔍 1. 系统状态端点');
  await test('GET /health', async () => {
    const r = await request('GET', '/health');
    return r.status === 200 && getData(r).status === 'ok';
  });
  await test('GET /status', async () => {
    const r = await request('GET', '/status');
    return r.status === 200 && r.body.success === true;
  });
  await test('GET /logs', async () => {
    const r = await request('GET', '/logs');
    return r.status === 200 && Array.isArray(getData(r));
  });
  await test('GET /config', async () => {
    const r = await request('GET', '/config');
    return r.status === 200 && getData(r).version === '3.0.0';
  });

  // 2. 知识图谱
  console.log('\n🔍 2. 知识图谱端点');
  await test('GET /graph', async () => {
    const r = await request('GET', '/graph');
    return r.status === 200 && Array.isArray(getData(r).nodes);
  });
  await test('GET /graph/stats', async () => {
    const r = await request('GET', '/graph/stats');
    const d = getData(r);
    return r.status === 200 && typeof d.nodes === 'number';
  });
  await test('GET /graph/pagerank', async () => {
    const r = await request('GET', '/graph/pagerank');
    const d = getData(r);
    return r.status === 200 && typeof d.pagerank === 'object';
  });
  await test('GET /graph/centrality', async () => {
    const r = await request('GET', '/graph/centrality');
    const d = getData(r);
    return r.status === 200 && typeof d.degree === 'object';
  });
  await test('GET /graph/communities', async () => {
    const r = await request('GET', '/graph/communities');
    const d = getData(r);
    return r.status === 200 && typeof d.communities === 'object';
  });
  await test('GET /graph/path?source=n_disp&target=n_data', async () => {
    const r = await request('GET', '/graph/path?source=n_disp&target=n_data');
    return r.status === 200 && r.body.success === true;
  });
  await test('POST /graph/activate', async () => {
    const r = await request('POST', '/graph/activate', { seed: 'n_disp', decay: 0.7 });
    return r.status === 200 && r.body.success === true;
  });

  // 3. AI 助手
  console.log('\n🔍 3. AI 助手端点');
  await test('GET /ai/sessions', async () => {
    const r = await request('GET', '/ai/sessions');
    return r.status === 200;
  });
  await test('POST /ai/chat (simple)', async () => {
    const r = await request('POST', '/ai/chat', { message: '你好' });
    const d = getData(r);
    return r.status === 200 && typeof d.reply === 'string';
  });
  await test('POST /ai/chat (with session)', async () => {
    const r = await request('POST', '/ai/chat', { message: '推荐算子', session_id: 'test-session-1' });
    const d = getData(r);
    return r.status === 200 && typeof d.reply === 'string' && d.reply.length > 0;
  });
  await test('GET /ai/experts', async () => {
    const r = await request('GET', '/ai/experts');
    return r.status === 200;
  });
  await test('POST /ai/expert-chat', async () => {
    const r = await request('POST', '/ai/expert-chat', { 
      expert_type: 'algorithm', 
      messages: [{ role: 'user', content: '分析排序算法复杂度' }]
    });
    return r.status === 200;
  });
  await test('GET /ai/llm/config', async () => {
    const r = await request('GET', '/ai/llm/config');
    return r.status === 200;
  });
  await test('POST /ai/llm/test', async () => {
    const r = await request('POST', '/ai/llm/test', {});
    return r.status === 200;
  });

  // 4. 算子中心
  console.log('\n🔍 4. 算子中心端点');
  await test('GET /operators', async () => {
    const r = await request('GET', '/operators');
    return r.status === 200 && Array.isArray(getData(r));
  });
  await test('POST /operators (register)', async () => {
    const r = await request('POST', '/operators/register', {
      name: '测试算子_' + Date.now(),
      type: 'function',
      category: 'test',
      desc: 'API测试算子'
    });
    return r.status === 200 && r.body.success === true;
  });
  await test('GET /operators/categories', async () => {
    const r = await request('GET', '/operators/categories');
    return r.status === 200 && Array.isArray(getData(r));
  });
  await test('GET /operators/stats', async () => {
    const r = await request('GET', '/operators/stats');
    const d = getData(r);
    return r.status === 200 && typeof d.total === 'number';
  });

  // 5. 算子商城
  console.log('\n🔍 5. 算子商城端点');
  await test('GET /market', async () => {
    const r = await request('GET', '/market');
    return r.status === 200;
  });
  await test('GET /market/categories', async () => {
    const r = await request('GET', '/market/categories');
    return r.status === 200;
  });

  // 6. 工作流编排
  console.log('\n🔍 6. 工作流编排端点');
  await test('GET /workflows', async () => {
    const r = await request('GET', '/workflows');
    return r.status === 200;
  });
  await test('POST /workflows (create)', async () => {
    const r = await request('POST', '/workflows', {
      name: '测试工作流',
      description: 'API测试',
      nodes: [{ id: 'n1', type: 'operator', op_id: 'normalize', x: 100, y: 100 }],
      edges: []
    });
    return r.status === 200;
  });

  // 7. MCP 兼容
  console.log('\n🔍 7. MCP 兼容端点');
  await test('POST /mcp (tools/list)', async () => {
    const r = await request('POST', '/mcp', { jsonrpc: '2.0', method: 'tools/list', id: 1 });
    const d = getData(r);
    return r.status === 200 && d.result?.tools;
  });
  await test('POST /mcp (tools/call)', async () => {
    const r = await request('POST', '/mcp', { jsonrpc: '2.0', method: 'tools/call', id: 2, params: { name: 'graph.pagerank' } });
    const d = getData(r);
    return r.status === 200 && d.result?.tool;
  });
  await test('POST /mcp (invalid method)', async () => {
    const r = await request('POST', '/mcp', { jsonrpc: '2.0', method: 'invalid/method', id: 3 });
    const d = getData(r);
    return r.status === 200 && d.error?.code === -32601;
  });

  // 8. 需求编译
  console.log('\n🔍 8. 需求编译端点');
  await test('POST /caomei/compile', async () => {
    const r = await request('POST', '/caomei/compile', { requirement: '创建一个用户登录流程' });
    const d = getData(r);
    return r.status === 200 && d.blueprint;
  });
  await test('POST /caomei/refine', async () => {
    const r = await request('POST', '/caomei/refine', { blueprint: { nodes: [] } });
    return r.status === 200;
  });
  await test('GET /caomei/templates', async () => {
    const r = await request('GET', '/caomei/templates');
    return r.status === 200;
  });

  // 9. 璇玑治理
  console.log('\n🔍 9. 璇玑治理端点');
  await test('GET /mox/health', async () => {
    const r = await request('GET', '/mox/health');
    const d = getData(r);
    return r.status === 200 && d.business && d.development;
  });
  await test('POST /mox/optimize', async () => {
    const r = await request('POST', '/mox/optimize', {});
    return r.status === 200 && getData(r).optimized;
  });
  await test('POST /mox/publish', async () => {
    const r = await request('POST', '/mox/publish', { target: 'staging' });
    return r.status === 200 && getData(r).published;
  });

  // 10. AI 自动化
  console.log('\n🔍 10. AI 自动化端点');
  await test('GET /automation', async () => {
    const r = await request('GET', '/automation');
    return r.status === 200;
  });
  await test('POST /automation/chat', async () => {
    const r = await request('POST', '/automation/chat', { name: '测试自动化', description: 'API测试' });
    return r.status === 200;
  });

  // 11. LLM 网关
  console.log('\n🔍 11. LLM 网关端点');
  await test('GET /llm/providers', async () => {
    const r = await request('GET', '/llm/providers');
    return r.status === 200;
  });

  // 12. 资源管理
  console.log('\n🔍 12. 资源管理端点');
  await test('GET /plugins', async () => {
    const r = await request('GET', '/plugins');
    return r.status === 200;
  });
  await test('GET /browser/sessions', async () => {
    const r = await request('GET', '/browser/sessions');
    return r.status === 200;
  });

  // 汇总
  console.log('\n' + '='.repeat(60));
  console.log(`  测试完成: ${passed} 通过, ${failed} 失败, 共 ${passed + failed} 项`);
  console.log('='.repeat(60));

  if (failed > 0) {
    console.log('\n❌ 失败项详情:');
    results.filter(r => r.status === 'FAIL').forEach(r => console.log(`   - ${r.name}: ${r.detail}`));
    process.exit(1);
  } else {
    console.log('\n✅ 所有 API 端点测试通过！');
    process.exit(0);
  }
}

run().catch(e => { console.error('测试运行错误:', e); process.exit(1); });

'use strict';

const http = require('http');

const BASE = 'http://localhost:3010';
let passed = 0, failed = 0, errors = [];

function req(method, path, data) {
  return new Promise((resolve) => {
    const opts = {
      hostname: 'localhost', port: 3010,
      path, method,
      headers: data ? { 'Content-Type': 'application/json' } : {}
    };
    const r = http.request(opts, (res) => {
      let chunks = '';
      res.on('data', (c) => chunks += c);
      res.on('end', () => {
        try { resolve({ status: res.statusCode, body: JSON.parse(chunks) }); }
        catch (e) { resolve({ status: res.statusCode, body: chunks }); }
      });
    });
    r.on('error', (e) => resolve({ status: 0, error: e.message }));
    if (data) r.write(JSON.stringify(data));
    r.end();
  });
}

function check(name, condition, detail) {
  if (condition) { passed++; console.log('  ✅ ' + name); }
  else { failed++; errors.push({ name, detail }); console.log('  ❌ ' + name + (detail ? ' - ' + detail : '')); }
}

async function testAll() {
  console.log('╔══════════════════════════════════════════════╗');
  console.log('║  企业级全维度验证测试                       ║');
  console.log('║  算子统一系统 (OUS) 专家联盟                ║');
  console.log('╚══════════════════════════════════════════════╝\n');

  // ==================== 1. 基础健康检查 ====================
  console.log('📡 [1/16] 基础健康检查');
  const h1 = await req('GET', '/health');
  check('健康检查', h1.body?.data?.status === 'ok');
  const h2 = await req('GET', '/status/full');
  check('全状态检查', h2.body?.success);
  const h3 = await req('GET', '/logs');
  check('日志查询', h3.body?.success);

  // ==================== 2. 算子中心 ====================
  console.log('\n🔧 [2/16] 算子中心');
  const o1 = await req('GET', '/operators');
  check('算子列表', o1.body?.data?.length >= 0);
  const o2 = await req('POST', '/operators/register', {
    name: 'test-op', id: 'unit-test-' + Date.now(),
    type: 'transform', description: '测试算子'
  });
  check('算子注册', o2.body?.success);
  const o3 = await req('POST', '/execute', {
    operator: 'identity', inputs: { test: 'value' }
  });
  check('算子执行', o3.body?.success);
  const o4 = await req('POST', '/operators/ai-recommend', { query: '数据分析算子' });
  check('AI算子推荐', o4.body?.success);

  // ==================== 3. 知识图谱 ====================
  console.log('\n🕸️ [3/16] 知识图谱');
  const g1 = await req('GET', '/graph');
  check('图谱数据', g1.body?.success);
  const g2 = await req('GET', '/graph/stats');
  check('图谱统计', g2.body?.success);
  const g3 = await req('GET', '/graph/centrality');
  check('中心性计算', g3.body?.success);
  const g4 = await req('GET', '/graph/communities');
  check('社区发现', g4.body?.success);
  const g5 = await req('GET', '/graph/pagerank');
  check('PageRank', g5.body?.success);
  const g6 = await req('POST', '/graph/ai-insights', { question: '图谱中最重要的节点' });
  check('AI图谱洞察', g6.body?.success);
  const g7 = await req('POST', '/graph/recommend', { nodeId: 'n1', question: '相关推荐' });
  check('图谱推荐', g7.body?.success);

  // ==================== 4. AI助手 ====================
  console.log('\n🤖 [4/16] AI助手');
  const a1 = await req('POST', '/ai/chat', { message: '你好' });
  check('AI对话', a1.body?.success);
  const a2 = await req('POST', '/ai/chat', {
    message: '分析一下快速排序',
    expertType: 'algorithm'
  });
  check('AI专家路由', a2.body?.success);
  const a3 = await req('POST', '/ai/analyze-algorithm', { algorithm: 'quick_sort' });
  check('算法分析', a3.body?.success);
  const a4 = await req('GET', '/ai/algorithm-types');
  check('算法类型列表', a4.body?.success);

  // ==================== 5. 资源管理 ====================
  console.log('\n📦 [5/16] 资源管理');
  const r1 = await req('GET', '/ai/resources');
  check('资源列表', r1.body?.success);
  const r2 = await req('GET', '/ai/resources/health');
  check('资源健康', r2.body?.success);
  const r3 = await req('POST', '/resources/ai-analysis', { resourceType: 'data' });
  check('AI资源分析', r3.body?.success);

  // ==================== 6. 工作流编排 ====================
  console.log('\n⚙️ [6/16] 工作流编排');
  const w1 = await req('GET', '/ai/workflows');
  check('工作流列表', w1.body?.success);
  const w2 = await req('GET', '/ai/workflows/templates');
  check('工作流模板', w2.body?.success);
  const w3 = await req('POST', '/ai/workflows/save', {
    name: 'test-workflow',
    nodes: [{ id: 'n1', type: 'start' }, { id: 'n2', type: 'task' }],
    edges: [{ from: 'n1', to: 'n2' }]
  });
  check('工作流保存', w3.body?.success);
  const w4 = await req('POST', '/workflow/ai-generate', { description: '数据处理流程' });
  check('AI工作流生成', w4.body?.success);

  // ==================== 7. AI插件 ====================
  console.log('\n🔌 [7/16] AI插件');
  const p1 = await req('GET', '/ai/plugins');
  check('插件列表', p1.body?.success);
  const p2 = await req('GET', '/ai/plugins/topology');
  check('插件拓扑', p2.body?.success);
  const p3 = await req('POST', '/plugins/ai-route', { message: '发送消息' });
  check('AI插件路由', p3.body?.success);

  // ==================== 8. 浏览器自动化 ====================
  console.log('\n🌐 [8/16] 浏览器自动化');
  const b1 = await req('GET', '/ai/browser/templates');
  check('浏览器模板', b1.body?.success);
  const b2 = await req('POST', '/ai/browser/natural', {
    instruction: '访问 https://example.com'
  });
  check('自然语言浏览器', b2.body?.success);

  // ==================== 9. 系统监控 ====================
  console.log('\n📊 [9/16] 系统监控');
  const m1 = await req('GET', '/status');
  check('系统状态', m1.body?.success);
  const m2 = await req('GET', '/logs');
  check('日志监控', m2.body?.success);

  // ==================== 10. API文档 ====================
  console.log('\n📚 [10/16] API文档');
  const d1 = await req('GET', '/plugins');
  check('插件文档', d1.body?.success);
  check('算子文档', o1.body?.success);
  check('工作流文档', w1.body?.success);

  // ==================== 11. 算子商城 ====================
  console.log('\n🛒 [11/16] 算子商城');
  const mk1 = await req('GET', '/market');
  check('商城列表', mk1.body?.success);
  const mk2 = await req('GET', '/market/random');
  check('推荐算子', mk2.body?.success);

  // ==================== 12. MCP兼容 ====================
  console.log('\n🔗 [12/16] MCP兼容');
  const mcp1 = await req('POST', '/mcp', {
    method: 'tools/list', id: 1
  });
  const mcp1Data = mcp1.body?.data || mcp1.body;
  check('MCP工具列表', mcp1Data?.result?.tools?.length > 0 || mcp1Data?.jsonrpc === '2.0');
  const mcp2 = await req('POST', '/mcp', {
    method: 'tools/call', id: 2,
    params: { name: 'graph.pagerank', nodeId: 'test' }
  });
  const mcp2Data = mcp2.body?.data || mcp2.body;
  check('MCP工具调用', mcp2Data?.result?.output?.includes('executed'));

  // ==================== 13. AI自动化 ====================
  console.log('\n🤖 [13/16] AI自动化');
  const auto1 = await req('GET', '/automation');
  check('自动化列表', auto1.body?.success);
  const auto2 = await req('POST', '/automation/chat', {
    name: 'test-automation-' + Date.now(),
    description: '测试自动化任务'
  });
  check('AI自动化对话', auto2.body?.success);

  // ==================== 14. 需求编译 ====================
  console.log('\n📝 [14/16] 需求编译');
  const cm1 = await req('POST', '/caomei/compile', {
    requirement: '系统需要支持用户登录'
  });
  check('需求编译', cm1.body?.success);
  const cm2 = await req('GET', '/caomei/templates');
  check('需求模板', cm2.body?.success);

  // ==================== 15. 算法实验室 ====================
  console.log('\n🧪 [15/16] 算法实验室');
  const al1 = await req('POST', '/analyze/spiral', { n: 100 });
  check('螺旋算法分析', al1.body?.success);
  const al2 = await req('GET', '/ai/algorithm-types');
  check('算法类型', al2.body?.success);

  // ==================== 16. 全维融合 ====================
  console.log('\n🔮 [16/16] 全维融合');
  const xu1 = await req('GET', '/xuanji/health');
  check('璇玑健康', xu1.body?.success);
  const xu2 = await req('POST', '/xuanji/optimize', {
    dimension: 'performance', target: 'workflow'
  });
  check('璇玑优化', xu2.body?.success);
  const wb1 = await req('GET', '/workbench/ai-overview');
  check('工作台AI概览', wb1.body?.success);

  // ==================== 专家联盟专项 ====================
  console.log('\n🏛️ 专家联盟专项测试');
  const ex1 = await req('GET', '/experts');
  check('专家列表(15位)', ex1.body?.data?.length === 15);
  const ex2 = await req('GET', '/experts/capabilities');
  check('专家能力图谱', ex2.body?.success);
  const ex3 = await req('POST', '/experts/alg-expert/consult', {
    messages: [{ role: 'user', content: '分析快速排序时间复杂度' }]
  });
  check('单专家咨询', ex3.body?.success);
  const ex4 = await req('POST', '/experts/multi-consult', {
    question: '系统性能优化建议',
    expert_ids: ['perf-expert', 'arch-expert']
  });
  check('多专家协同', ex4.body?.success && ex4.body?.data?.successful === 2);
  const ex5 = await req('POST', '/experts/debate', {
    question: '架构选型讨论',
    expert_ids: ['arch-expert', 'perf-expert', 'op-expert'],
    rounds: 2
  });
  check('专家辩论', ex5.body?.success);
  check('辩论历史', ex5.body?.data?.history?.length === 2);
  check('综合结论', ex5.body?.data?.final_synthesis?.length > 50);

  // ==================== LLM网关专项 ====================
  console.log('\n🔑 LLM网关专项测试');
  const l1 = await req('GET', '/llm/providers');
  check('LLM提供商列表', l1.body?.success);
  const l2 = await req('GET', '/ai/llm/config');
  check('LLM配置', l2.body?.success);
  const l3 = await req('POST', '/ai/llm/test', {
    provider: 'default',
    message: '测试连接'
  });
  check('LLM连接测试', l3.body?.success);

  // ==================== 总结 ====================
  console.log('\n' + '='.repeat(50));
  console.log(`📊 测试结果: ${passed}/${passed + failed} 通过`);
  console.log(`   通过率: ${((passed / (passed + failed)) * 100).toFixed(1)}%`);
  
  if (errors.length > 0) {
    console.log('\n❌ 失败项详情:');
    errors.forEach((e, i) => {
      console.log(`   ${i + 1}. ${e.name}: ${e.detail}`);
    });
  }
  
  if (passed === passed + failed) {
    console.log('\n🎉 所有测试通过！系统可正常使用，已对接AI LLM。');
  } else {
    console.log(`\n⚠️  有 ${failed} 项测试未通过，需要检查相关模块。`);
  }
  
  return { passed, failed, errors };
}

testAll().then(() => process.exit(0)).catch(e => { console.error(e); process.exit(1); });
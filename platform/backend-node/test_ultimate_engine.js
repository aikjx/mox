const http = require('http');
const PORT = 3010;

function makeRequest(path, method, data) {
  return new Promise((resolve, reject) => {
    const opts = {
      hostname: 'localhost', port: PORT, path: path, method: method,
      headers: data ? { 'Content-Type': 'application/json' } : {}
    };
    const req = http.request(opts, (res) => {
      let chunks = '';
      res.on('data', (c) => chunks += c);
      res.on('end', () => {
        try { resolve(JSON.parse(chunks)); }
        catch (e) { resolve(chunks); }
      });
    });
    req.on('error', reject);
    if (data) req.write(JSON.stringify(data));
    req.end();
  });
}

async function test() {
  let passed = 0;
  let failed = 0;
  const results = [];

  function check(name, condition, detail = '') {
    const status = condition ? 'PASS' : 'FAIL';
    const msg = `[${status}] ${name}${detail ? ' (' + detail + ')' : ''}`;
    console.log(msg);
    results.push({ name, status, detail });
    if (condition) passed++; else failed++;
  }

  console.log('╔══════════════════════════════════════════════════╗');
  console.log('║   终极AI引擎 - 全链路综合测试报告                ║');
  console.log('╚══════════════════════════════════════════════════╝\n');

  // Group 1: Health & Stats
  console.log('─ 组1: 健康检查与系统统计 ─');
  const ultimateHealth = await makeRequest('/ai/ultimate/health', 'GET');
  check('终极引擎健康检查', ultimateHealth.success);
  check('健康分数返回', ultimateHealth.data?.healthScore !== undefined);
  check('版本号', ultimateHealth.data?.version === '2.0.0');
  check('组件状态', ultimateHealth.data?.components !== undefined);
  
  const ultimateStats = await makeRequest('/ai/ultimate/stats', 'GET');
  check('终极统计获取', ultimateStats.success);
  check('向量存储统计', ultimateStats.data?.vectorStore !== undefined);
  check('图推理统计', ultimateStats.data?.graphReasoner !== undefined);
  check('性能报告', ultimateStats.data?.performance !== undefined);

  // Group 2: Deep Processing
  console.log('\n─ 组2: 深度智能处理 ─');
  const processed = await makeRequest('/ai/ultimate/process', 'POST', {
    question: '如何构建一个高可用的分布式系统？',
    options: { mode: 'intelligent' }
  });
  check('深度处理成功', processed.success);
  check('处理层数', processed.data?.processingLayers?.length >= 2);
  check('有最终答案', processed.data?.finalAnswer !== null);
  check('包含基础智能层', processed.data?.processingLayers?.some(l => l.layer === 'base_intelligence'));
  check('包含深度推理层', processed.data?.processingLayers?.some(l => l.layer === 'deep_reasoning'));
  check('包含记忆召回层', processed.data?.processingLayers?.some(l => l.layer === 'memory_recall'));
  check('处理时间<10s', processed.data?.processingTimeMs < 10000);

  // Group 3: Reasoning
  console.log('\n─ 组3: 深度推理与自我反思 ─');
  const reasoning = await makeRequest('/ai/ultimate/reasoning', 'POST', {
    question: '分析微服务架构的优缺点',
    options: { maxSteps: 3, self_reflect: true }
  });
  check('深度推理成功', reasoning.success);
  check('推理步骤数', reasoning.data?.steps?.length >= 2);
  check('包含最终答案', reasoning.data?.finalAnswer !== null);
  check('有置信度', reasoning.data?.overallConfidence !== undefined);
  check('推理质量评估', reasoning.data?.reasoningQuality !== undefined);

  const reasoningNoReflect = await makeRequest('/ai/ultimate/reasoning', 'POST', {
    question: '什么是向量空间模型？',
    options: { maxSteps: 2, self_reflect: false }
  });
  check('无反思模式成功', reasoningNoReflect.success);

  // Group 4: Analogical Reasoning
  console.log('\n─ 组4: 跨域类比推理 ─');
  const analogical = await makeRequest('/ai/ultimate/analogical', 'POST', {
    source_domain: '生物进化',
    target_domain: '软件架构演进',
    question: '如何借鉴生物进化来优化软件架构？'
  });
  check('类比推理成功', analogical.success);
  check('包含类比', analogical.data?.analogies !== undefined || analogical.data?.rawResponse !== undefined);
  check('源域信息', analogical.data?.sourceDomain === '生物进化');
  check('目标域信息', analogical.data?.targetDomain === '软件架构演进');

  // Group 5: Vector Memory
  console.log('\n─ 组5: 向量知识存储与检索 ─');
  const store1 = await makeRequest('/ai/ultimate/store', 'POST', {
    id: 'doc_1',
    content: '专家联盟系统采用微服务架构，支持水平扩展。',
    metadata: { type: 'architecture', module: 'expert-alliance', version: '1.0' }
  });
  check('知识存储成功', store1.success);

  const store2 = await makeRequest('/ai/ultimate/store', 'POST', {
    id: 'doc_2',
    content: '图谱分析使用PageRank算法计算节点重要性，社区检测基于Louvain算法。',
    metadata: { type: 'algorithm', module: 'graph', version: '1.0' }
  });
  check('第二条知识存储', store2.success);

  const store3 = await makeRequest('/ai/ultimate/store', 'POST', {
    id: 'doc_3',
    content: 'LLM网关支持多供应商切换，包括DeepSeek、千问、豆包等，支持负载均衡。',
    metadata: { type: 'integration', module: 'llm-gateway', version: '2.0' }
  });
  check('第三条知识存储', store3.success);

  const search = await makeRequest('/ai/ultimate/search', 'POST', {
    query: '图谱算法',
    options: { topK: 3, threshold: 0.1 }
  });
  check('知识检索成功', search.success);
  check('返回结果', search.data?.results !== undefined);

  // Group 6: Prompt Optimization
  console.log('\n─ 组6: Prompt优化 ─');
  const optimized = await makeRequest('/ai/ultimate/optimize-prompt', 'POST', {
    prompt: '请帮我分析一下系统的性能瓶颈',
    target: 'analytical'
  });
  check('Prompt优化成功', optimized.success);
  check('返回优化后', optimized.data?.optimized !== undefined);

  const optimizedCreative = await makeRequest('/ai/ultimate/optimize-prompt', 'POST', {
    prompt: '如何解决这个技术问题',
    target: 'creative'
  });
  check('创意优化成功', optimizedCreative.success);

  // Group 7: Performance & Circuit Breaker
  console.log('\n─ 组7: 性能监控与熔断器 ─');
  const perfReport = await makeRequest('/ai/ultimate/performance', 'GET');
  check('性能报告获取', perfReport.success);
  check('成功率', perfReport.data?.successRate !== undefined);
  check('熔断器状态', perfReport.data?.circuitBreaker !== undefined);

  const circuitStatus = await makeRequest('/ai/ultimate/circuit-breaker', 'GET');
  check('熔断器状态获取', circuitStatus.success);
  check('状态为closed或half-open', ['closed', 'half-open', 'open'].includes(circuitStatus.data?.state));

  // Group 8: Reasoning Rules
  console.log('\n─ 组8: 推理规则引擎 ─');
  const rulesList = await makeRequest('/ai/ultimate/reasoning-rules', 'GET');
  check('规则列表获取', rulesList.success);
  check('规则数量', rulesList.data?.rulesCount >= 5);

  const addRule = await makeRequest('/ai/ultimate/reasoning-rules', 'POST', {
    rule: {
      name: '自定义传递规则',
      pattern: { relation: 'implements' },
      action: 'if A implements B then A has all behaviors of B',
      confidence: 0.85
    }
  });
  check('添加规则成功', addRule.success);

  // Group 9: Full Analysis
  console.log('\n─ 组9: 终极全维分析 ─');
  const fullAnalysis = await makeRequest('/ai/ultimate/full-analysis', 'POST', {
    question: '设计一个智能客服系统的架构方案',
    options: { mode: 'intelligent' }
  });
  check('全维分析成功', fullAnalysis.success);
  check('包含基础分析', fullAnalysis.data?.processingLayers !== undefined);
  check('包含备选推理', fullAnalysis.data?.alternateReasoning !== undefined);
  check('包含相关记忆', fullAnalysis.data?.relevantMemories !== undefined);

  // Group 10: Integration with existing system
  console.log('\n─ 组10: 系统集成性验证 ─');
  const home = await makeRequest('/', 'GET');
  const apiCount = Object.keys(home.data?.api || {}).length;
  const ultimateApiCount = Object.keys(home.data?.api || {}).filter(k => k.startsWith('ultimate')).length;
  check('API总路由数', apiCount >= 40);
  check('终极引擎API数量', ultimateApiCount >= 13);

  const integratedHealth = await makeRequest('/ai/integrated/health', 'GET');
  check('集成引擎健康', integratedHealth.success);

  const aiStatus = await makeRequest('/ai/status', 'GET');
  check('AI引擎状态', aiStatus.success);

  console.log('\n╔══════════════════════════════════════════════════╗');
  console.log('║                  测试结果汇总                      ║');
  console.log('╠══════════════════════════════════════════════════╣');
  console.log(`║  总测试: ${passed + failed} 项`);
  console.log(`║  通过:   ${passed} 项`);
  console.log(`║  失败:   ${failed} 项`);
  console.log(`║  通过率: ${(passed / (passed + failed) * 100).toFixed(1)}%`);
  console.log('╚══════════════════════════════════════════════════╝\n');

  if (failed > 0) {
    console.log('失败项:');
    results.filter(r => r.status === 'FAIL').forEach(r => {
      console.log(`  - ${r.name}`);
    });
  }

  if (passed === passed + failed) {
    console.log('🎉 终极AI引擎全链路测试全部通过！');
  } else {
    console.log('⚠️  部分测试未通过，请检查。');
  }
}

test().catch(console.error);

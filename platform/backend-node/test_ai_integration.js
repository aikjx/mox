const http = require('http');

const PORT = 3002;

function makeRequest(path, method, data) {
  return new Promise((resolve, reject) => {
    const opts = {
      hostname: 'localhost',
      port: PORT,
      path: path,
      method: method,
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

  function check(name, condition) {
    if (condition) { console.log('  ✅ ' + name); passed++; }
    else { console.log('  ❌ ' + name); failed++; }
  }

  console.log('=== AI 集成引擎全链路测试 ===\n');

  // Test 1: 系统统计
  console.log('1. 集成引擎系统统计:');
  try {
    const stats = await makeRequest('/ai/integrated/stats', 'GET');
    check('获取统计成功', stats.success);
    check('返回图引擎统计', stats.data?.graphEngine !== undefined);
    check('返回PlanAct统计', stats.data?.planAct !== undefined);
    check('返回学习引擎统计', stats.data?.learningEngine !== undefined);
    check('返回编排器统计', stats.data?.orchestrator !== undefined);
  } catch (e) {
    console.log('  ⚠️ 服务器可能未启动: ' + e.message);
    return;
  }

  // Test 2: 智能处理
  console.log('\n2. AI智能集成处理:');
  const process = await makeRequest('/ai/integrated/process', 'POST', {
    question: '分析一下当前系统的架构',
    mode: 'auto'
  });
  check('智能处理成功', process.success);
  check('返回处理结果', process.data !== undefined);

  // Test 3: 全维分析
  console.log('\n3. 全维分析:');
  const analysis = await makeRequest('/ai/integrated/full-analysis', 'POST', {
    question: '如何优化系统性能'
  });
  check('全维分析成功', analysis.success);
  check('返回分析结果', analysis.data !== undefined);

  // Test 4: 图智能计算
  console.log('\n4. 图智能计算:');
  const graphData = {
    nodes: [
      { id: 'A', label: '节点A' },
      { id: 'B', label: '节点B' },
      { id: 'C', label: '节点C' },
      { id: 'D', label: '节点D' },
      { id: 'E', label: '节点E' }
    ],
    edges: [
      { source: 'A', target: 'B', weight: 1 },
      { source: 'A', target: 'C', weight: 2 },
      { source: 'B', target: 'D', weight: 1 },
      { source: 'C', target: 'D', weight: 1 },
      { source: 'D', target: 'E', weight: 2 },
      { source: 'E', target: 'A', weight: 1 }
    ]
  };
  const graphInt = await makeRequest('/ai/integrated/graph-intelligence', 'POST', {
    graph: graphData,
    question: '节点A的重要性'
  });
  check('图智能计算成功', graphInt.success);
  check('返回PageRank', graphInt.data?.personalizedPageRank !== undefined);
  check('返回社区检测', graphInt.data?.communities !== undefined);

  // Test 5: 创建计划
  console.log('\n5. 创建执行计划:');
  const plan = await makeRequest('/ai/integrated/plan-create', 'POST', {
    goal: '开发一个新功能',
    context: { module: 'user', feature: 'auth' }
  });
  check('创建计划成功', plan.success);
  check('返回计划ID', plan.data?.plan_id !== undefined || plan.data?.id !== undefined);

  // Test 6: 计划列表
  console.log('\n6. 计划列表:');
  const plans = await makeRequest('/ai/integrated/plans', 'GET');
  check('获取计划列表成功', plans.success);
  check('返回计划数组', Array.isArray(plans.data));

  // Test 7: 技能列表
  console.log('\n7. 技能列表:');
  const skills = await makeRequest('/ai/integrated/skills', 'GET');
  check('获取技能列表成功', skills.success);
  check('返回技能数组', Array.isArray(skills.data));

  // Test 8: 技能提取
  console.log('\n8. 技能提取:');
  const trajectory = {
    turns: [
      { role: 'user', content: '用户登录功能' },
      { role: 'assistant', content: '创建LoginForm组件' },
      { role: 'user', content: '添加表单验证' },
      { role: 'assistant', content: '实现validateForm函数' },
      { role: 'user', content: '处理错误提示' },
      { role: 'assistant', content: '添加ErrorDisplay组件' },
      { role: 'user', content: '提交代码' },
      { role: 'assistant', content: '完成git commit' }
    ]
  };
  const extracted = await makeRequest('/ai/integrated/skill-extract', 'POST', {
    trajectory: trajectory
  });
  check('技能提取成功', extracted.success);

  // Test 9: 智能体列表
  console.log('\n9. 智能体列表:');
  const agents = await makeRequest('/ai/integrated/agents', 'GET');
  check('获取智能体列表成功', agents.success);
  check('返回智能体数组', Array.isArray(agents.data));
  check('默认智能体存在', agents.data?.length >= 5);

  // Test 10: 注册智能体
  console.log('\n10. 注册智能体:');
  const newAgent = await makeRequest('/ai/integrated/agent-register', 'POST', {
    agent: {
      id: 'test-agent',
      name: '测试智能体',
      role: 'tester',
      capabilities: ['测试', 'QA'],
      systemPrompt: '你是一位专业的测试工程师。'
    }
  });
  check('注册智能体成功', newAgent.success);
  check('返回智能体ID', newAgent.data?.id === 'test-agent');

  // Test 11: 记忆存储
  console.log('\n11. 记忆存储:');
  const memory = await makeRequest('/ai/integrated/memory-store', 'POST', {
    key: 'test_key',
    value: { content: '测试记忆内容' },
    options: { type: 'episodic', importance: 0.8 }
  });
  check('记忆存储成功', memory.success);

  // Test 12: 记忆召回
  console.log('\n12. 记忆召回:');
  const recalled = await makeRequest('/ai/integrated/memory-recall', 'POST', {
    query: '测试'
  });
  check('记忆召回成功', recalled.success);

  // Test 13: 流水线列表
  console.log('\n13. 流水线列表:');
  const pipelines = await makeRequest('/ai/integrated/pipelines', 'GET');
  check('获取流水线列表成功', pipelines.success);

  // Test 14: 轨迹压缩
  console.log('\n14. 轨迹压缩:');
  const compressed = await makeRequest('/ai/integrated/trajectory-compress', 'POST', {
    trajectory: trajectory,
    options: { maxTokens: 2000 }
  });
  check('轨迹压缩成功', compressed.success);

  // Test 15: 回滚检查点
  console.log('\n15. 计划回滚测试（预期可能失败）:');
  const rollback = await makeRequest('/ai/integrated/plan-rollback', 'POST', {
    plan_id: 'nonexistent',
    checkpoint_id: 'nonexistent'
  });
  check('回滚返回结果', rollback.success !== undefined);

  console.log('\n' + '='.repeat(40));
  console.log(`测试结果: ${passed}/${passed + failed} 通过`);
  if (failed > 0) {
    console.log(`失败率: ${(failed / (passed + failed) * 100).toFixed(1)}%`);
  } else {
    console.log('🎉 所有测试通过！');
  }
}

test().catch(console.error);

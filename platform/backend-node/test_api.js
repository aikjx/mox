const http = require('http');

function makeRequest(path, method, data) {
  return new Promise((resolve, reject) => {
    const opts = { hostname: 'localhost', port: 3010, path: path, method: method, headers: data ? {'Content-Type': 'application/json'} : {} };
    const req = http.request(opts, (res) => {
      let chunks = '';
      res.on('data', (c) => chunks += c);
      res.on('end', () => { try { resolve(JSON.parse(chunks)); } catch (e) { resolve(chunks); } });
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
  
  console.log('=== 专家联盟系统测试 ===\n');
  
  // Test 1: AI Chat with message field
  console.log('1. AI Chat (message字段):');
  const chat1 = await makeRequest('/ai/chat', 'POST', { message: '你好' });
  check('消息正确传递', chat1.data?.reply?.includes('你好') || chat1.data?.reply?.includes('已收到'));
  
  // Test 2: AI Chat with messages array
  console.log('\n2. AI Chat (messages数组):');
  const chat2 = await makeRequest('/ai/chat', 'POST', { messages: [{ role: 'user', content: '介绍一下算子系统' }] });
  check('消息数组正确传递', chat2.data?.reply?.includes('算子'));
  
  // Test 3: Expert list
  console.log('\n3. 专家列表:');
  const experts = await makeRequest('/experts', 'GET');
  check('返回15个专家', experts.data?.length === 15);
  check('专家类型包含algorithm', experts.data?.some(e => e.type === 'algorithm'));
  
  // Test 4: Expert capabilities and types
  console.log('\n4. 专家能力与类型:');
  const caps = await makeRequest('/experts/capabilities', 'GET');
  check('类型数量正确', caps.data?.types?.length === 15);
  check('能力映射存在', Object.keys(caps.data?.capabilities || {}).length > 0);
  
  // Test 5: Single expert consult
  console.log('\n5. 单专家咨询:');
  const consult = await makeRequest('/experts/alg-expert/consult', 'POST', { messages: [{ role: 'user', content: '分析快速排序' }] });
  check('咨询成功', consult.success);
  check('返回专家信息', consult.data?.expert?.id === 'alg-expert');
  check('响应包含内容', consult.data?.response?.length > 50);
  
  // Test 6: Multi-expert consult
  console.log('\n6. 多专家协同咨询:');
  const multi = await makeRequest('/experts/multi-consult', 'POST', { question: '如何优化系统性能？', expert_ids: ['perf-expert', 'arch-expert'] });
  check('多专家咨询成功', multi.success);
  check('返回2个结果', multi.data?.successful === 2);
  
  // Test 7: Expert debate
  console.log('\n7. 专家辩论:');
  const debate = await makeRequest('/experts/debate', 'POST', { question: '单体vs微服务', expert_ids: ['arch-expert', 'perf-expert'], rounds: 2 });
  check('辩论成功', debate.success);
  check('返回历史记录', debate.data?.history?.length === 2);
  check('返回综合结论', debate.data?.final_synthesis?.length > 50);
  
  // Test 8: Expert registration
  console.log('\n8. 专家注册:');
  const register = await makeRequest('/experts', 'POST', { id: 'test-expert', name: '测试专家', type: 'test', capabilities: ['测试能力'], description: '用于测试' });
  check('注册成功', register.success);
  
  // Test 9: Expert update
  console.log('\n9. 专家更新:');
  const update = await makeRequest('/experts/test-expert', 'PUT', { name: '更新专家' });
  check('更新成功', update.success);
  
  // Test 10: Expert deletion
  console.log('\n10. 专家删除:');
  const remove = await makeRequest('/experts/test-expert', 'DELETE');
  check('删除成功', remove.success);
  
  // Test 11: AI Chat with expert routing
  console.log('\n11. AI对话路由到专家:');
  const expertChat = await makeRequest('/ai/chat', 'POST', { message: '如何设计高可用架构？', expertType: 'architecture' });
  check('专家路由成功', expertChat.success);
  check('返回专家响应', expertChat.data?.reply?.length > 50);
  
  // Test 12: LLM providers
  console.log('\n12. LLM提供商:');
  const providers = await makeRequest('/llm/providers', 'GET');
  check('获取提供商列表', providers.success);
  
  console.log('\n' + '='.repeat(40));
  console.log(`测试结果: ${passed}/${passed + failed} 通过`);
}

test().catch(console.error);
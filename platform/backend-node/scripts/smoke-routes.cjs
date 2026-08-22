'use strict';
/**
 * 架构拆分冒烟测试：跨 23 个业务域抽取代表性端点验证可达性
 */
const http = require('http');

const CASES = [
  ['GET', '/health', 'system'],
  ['GET', '/status', 'system'],
  ['GET', '/graph/stats', 'graph'],
  ['GET', '/graph/pagerank', 'graph+算法库'],
  ['GET', '/ai/sessions', 'graph'],
  ['GET', '/web-search/config', 'web-search'],
  ['GET', '/ai/artifact/config', 'artifacts'],
  ['GET', '/ai/infinite-optimize/status', 'optimizer'],
  ['GET', '/ai/workflows', 'ai-platform'],
  ['GET', '/llm/providers', 'integration'],
  ['GET', '/experts', 'expert-alliance'],
  ['GET', '/experts/metrics', 'expert-alliance'],
  ['GET', '/expert-graph/stats', 'expert-graph'],
  ['GET', '/experts/orchestration/stats', 'orchestration'],
  ['GET', '/tasks', 'tasks'],
  ['GET', '/kb/stats', 'kb'],
  ['GET', '/modules', 'modules-admin'],
  ['GET', '/storage/providers', 'modules-admin'],
  ['GET', '/security/status', 'security'],
  ['GET', '/ai/status', 'ai-engine'],
  ['GET', '/ai/integrated/stats', 'ai-integrated'],
  ['GET', '/ai/ultimate/stats', 'ai-ultimate'],
  ['GET', '/ai/engine/capabilities', 'ai-engine-core'],
  ['GET', '/ai/engine/flow-graph', 'flow-graph'],
  ['GET', '/ai/engine/auto-dev/projects', 'auto-dev'],
  ['GET', '/services', 'services']
];

function call(method, path) {
  return new Promise((resolve) => {
    const req = http.request({
      host: 'localhost', port: 3002, method, path,
      headers: { Authorization: 'Bearer dev-secret-token' }
    }, (res) => {
      let body = '';
      res.on('data', (c) => { body += c; });
      res.on('end', () => {
        let okBody = false;
        try { okBody = JSON.parse(body).success === true; } catch (e) {}
        resolve({ status: res.statusCode, okBody, body: body.slice(0, 120) });
      });
    });
    req.on('error', (e) => resolve({ status: 0, okBody: false, body: e.message }));
    req.end();
  });
}

(async () => {
  let pass = 0, fail = 0;
  for (const [method, path, domain] of CASES) {
    const r = await call(method, path);
    const okk = r.status === 200 && r.okBody;
    if (okk) { pass++; console.log(`  [PASS] ${domain.padEnd(16)} ${method} ${path}`); }
    else { fail++; console.log(`  [FAIL] ${domain.padEnd(16)} ${method} ${path} → ${r.status} ${r.body}`); }
  }
  console.log(`\n冒烟结果: ${pass}/${pass + fail} 通过`);
  process.exit(fail ? 1 : 0);
})();

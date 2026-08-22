#!/usr/bin/env node
'use strict';

/**
 * MCP 协议真实合规验证：模拟标准 MCP 客户端全生命周期
 * initialize → notifications/initialized → tools/list → tools/call（含真实 LLM 工具）
 * → 批量请求 → 协议错误语义（parse error / method not found / 通知无响应）
 */

const BASE = 'http://127.0.0.1:3002';
let pass = 0, total = 0;

function check(name, ok, detail = '') {
  total++;
  console.log('  %s %s%s', ok ? 'PASS' : 'FAIL', name, detail ? ' — ' + detail : '');
  if (ok) pass++;
}

async function rpc(method, params, id) {
  const res = await fetch(`${BASE}/mcp`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: id === undefined ? 1 : id, method, params })
  });
  const status = res.status;
  const text = await res.text();
  return { status, body: text ? JSON.parse(text) : null };
}

async function main() {
  // ===== 1. initialize =====
  console.log('[1] initialize（MCP 握手）');
  const init = await rpc('initialize', { protocolVersion: '2025-06-18', capabilities: {}, clientInfo: { name: 'test-client', version: '1.0' } });
  check('返回 200', init.status === 200);
  check('JSON-RPC 2.0 响应结构', init.body.jsonrpc === '2.0' && init.body.id === 1);
  check('协议版本协商', init.body.result.protocolVersion === '2025-06-18', init.body.result.protocolVersion);
  check('声明 tools 能力', !!init.body.result.capabilities.tools);
  check('serverInfo 正确', init.body.result.serverInfo.name === 'xuanji-expert-alliance');

  // ===== 2. notifications/initialized（通知：应 202 无 body）=====
  console.log('[2] notifications/initialized（通知语义）');
  const nres = await fetch(`${BASE}/mcp`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' })
  });
  const ntext = await nres.text();
  check('通知返回 202 无响应体', nres.status === 202 && ntext === '', `status=${nres.status} body='${ntext}'`);

  // ===== 3. tools/list =====
  console.log('[3] tools/list（工具清单）');
  const tl = await rpc('tools/list', {});
  const tools = tl.body.result.tools;
  check('返回 200', tl.status === 200);
  check('7 个工具', tools.length === 7, `count=${tools.length}`);
  check('工具含 JSON Schema', tools.every(t => t.inputSchema && t.inputSchema.type === 'object'));
  const names = tools.map(t => t.name);
  check('关键工具齐备', ['list_experts', 'consult_expert', 'alliance_process', 'compose_team'].every(n => names.includes(n)), names.join(','));

  // ===== 4. tools/call：classify_intent（本地算法，快）=====
  console.log('[4] tools/call：classify_intent');
  const ci = await rpc('tools/call', { name: 'classify_intent', arguments: { question: '数据库索引优化与慢查询治理' } }, 2);
  const ciText = ci.body.result.content[0].text;
  check('返回 200', ci.status === 200);
  check('content 结构合规（type=text）', ci.body.result.content[0].type === 'text');
  check('意图识别正确', ciText.includes('performance'), ciText.replace(/\s+/g, ' ').substring(0, 80));

  // ===== 5. tools/call：list_experts =====
  console.log('[5] tools/call：list_experts');
  const le = await rpc('tools/call', { name: 'list_experts', arguments: {} }, 3);
  const leText = le.body.result.content[0].text;
  check('返回 200', le.status === 200);
  check('专家清单非空', leText.includes('expert'), `len=${leText.length}`);

  // ===== 6. tools/call：consult_expert（真实 LLM）=====
  console.log('[6] tools/call：consult_expert（真实 LLM）');
  const ce = await rpc('tools/call', { name: 'consult_expert', arguments: { expert_id: 'sec-expert', question: '一句话：SQL 注入的核心防御原则' } }, 4);
  const ceText = ce.body.result.content[0].text;
  check('返回 200', ce.status === 200);
  check('真实 LLM 响应', ceText.includes('response') && ceText.length > 100, `len=${ceText.length}`);
  check('模型标注真实', ceText.includes('deepseek'), '');

  // ===== 7. tools/call：alliance_traces_stats =====
  console.log('[7] tools/call：alliance_traces_stats');
  const ts = await rpc('tools/call', { name: 'alliance_traces_stats', arguments: {} }, 5);
  check('返回 200', ts.status === 200);
  check('审计统计结构', ts.body.result.content[0].text.includes('success_rate'));

  // ===== 8. 批量请求 =====
  console.log('[8] 批量请求（JSON-RPC 2.0 batch）');
  const bres = await fetch(`${BASE}/mcp`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify([
      { jsonrpc: '2.0', id: 10, method: 'ping' },
      { jsonrpc: '2.0', id: 11, method: 'tools/list', params: {} }
    ])
  });
  const bbody = await bres.json();
  check('批量响应是数组', Array.isArray(bbody) && bbody.length === 2);
  check('批量 id 对应', bbody[0].id === 10 && bbody[1].id === 11);
  check('ping 返回空 result', JSON.stringify(bbody[0].result) === '{}');

  // ===== 9. 错误语义 =====
  console.log('[9] 协议错误语义');
  const eres = await fetch(`${BASE}/mcp`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' }, body: 'not-json{{'
  });
  const ebody = await eres.json();
  check('解析错误 -32700', ebody.error.code === -32700);

  const nf = await rpc('resources/list', {}, 99);
  check('未知方法 -32601', nf.body.error.code === -32601, nf.body.error.message);

  const iv = await rpc('tools/call', { name: 'nonexistent_tool', arguments: {} }, 98);
  check('未知工具返回 isError', iv.body.result.isError === true);

  const mp = await rpc('tools/call', { name: 'consult_expert', arguments: {} }, 97);
  check('缺必填参数提示', mp.body.result.isError === true && mp.body.result.content[0].text.includes('必填'));

  // ===== 10. GET /mcp/tools 便捷端点 =====
  console.log('[10] GET /mcp/tools（便捷端点）');
  const gt = await fetch(`${BASE}/mcp/tools`);
  const gtj = await gt.json();
  check('返回 200 且 success', gt.status === 200 && gtj.success === true);
  check('工具清单一致', gtj.data.tools.length === 7);

  console.log('\n结果: %s/%s 通过', pass, total);
  process.exit(pass === total ? 0 : 1);
}

main().catch(e => { console.error('测试失败:', e.message); process.exit(1); });

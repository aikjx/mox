#!/usr/bin/env node
'use strict';

/**
 * SSE 流式真实验证脚本：测首 token 延迟、分片数、内容完整性、usage 真实性
 * 用法：node test/test-sse-stream.js
 */

const BASE = 'http://127.0.0.1:3010';

async function main() {
  const t0 = Date.now();
  const res = await fetch(`${BASE}/ai/chat/stream`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      messages: [{ role: 'user', content: '用三句话介绍分布式共识算法的核心思想' }],
      temperature: 0.5
    })
  });

  console.log('[1] 响应头: status=%s content-type=%s', res.status, res.headers.get('content-type'));
  if (res.status !== 200 || !res.headers.get('content-type').includes('text/event-stream')) {
    throw new Error('SSE 响应头不符合预期');
  }

  const reader = res.body.getReader();
  const decoder = new TextDecoder('utf-8');
  let buffer = '';
  let chunks = 0;
  let firstTokenMs = -1;
  let full = '';
  let done = null;
  let sawStart = false;
  let sawDoneMarker = false;

  while (true) {
    const { done: rd, value } = await reader.read();
    if (rd) break;
    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split('\n');
    buffer = lines.pop() || '';
    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed.startsWith('data:')) continue;
      const data = trimmed.slice(5).trim();
      if (data === '[DONE]') { sawDoneMarker = true; continue; }
      const obj = JSON.parse(data);
      if (obj.event === 'start') { sawStart = true; console.log('[2] start 事件: sessionId=%s', obj.sessionId); }
      else if (obj.event === 'delta') {
        chunks++;
        if (firstTokenMs < 0) firstTokenMs = Date.now() - t0;
        full += obj.content;
      }
      else if (obj.event === 'done') { done = obj; }
      else if (obj.event === 'error') { throw new Error('服务端 error 事件: ' + obj.message); }
    }
  }

  const totalMs = Date.now() - t0;
  console.log('[3] 首 token 延迟: %sms（非流式通常为全量耗时）', firstTokenMs);
  console.log('[4] 分片数: %s（>10 即为真实逐 token 推送）', chunks);
  console.log('[5] 总耗时: %sms，完整内容长度: %s', totalMs, full.length);
  console.log('[6] done 事件: model=%s ai_powered=%s tokens=%s', done.model, done.ai_powered, done.usage.total_tokens);
  console.log('[7] 完整回答: %s', full.substring(0, 150) + (full.length > 150 ? '...' : ''));

  const checks = [
    ['start 事件到达', sawStart],
    ['done 事件到达', !!done],
    ['[DONE] 结束标记', sawDoneMarker],
    ['分片数 > 10（真实流式）', chunks > 10],
    ['首 token < 总耗时（提前可见）', firstTokenMs > 0 && firstTokenMs < totalMs],
    ['内容非空且完整', full.length > 50],
    ['真实 AI（usage 有 token）', done && done.usage.total_tokens > 0]
  ];

  let pass = 0;
  for (const [name, ok] of checks) {
    console.log('  %s %s', ok ? 'PASS' : 'FAIL', name);
    if (ok) pass++;
  }
  console.log('结果: %s/%s 通过', pass, checks.length);
  if (pass !== checks.length) process.exit(1);
}

main().catch(e => { console.error('测试失败:', e.message); process.exit(1); });

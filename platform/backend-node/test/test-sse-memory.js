#!/usr/bin/env node
'use strict';

/** SSE 流式 ↔ 非流式会话记忆互通验证：stream 轮写入的记忆，chat 轮可读 */

const BASE = 'http://127.0.0.1:3010';

async function streamTurn(messages) {
  const res = await fetch(`${BASE}/ai/chat/stream`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ messages, temperature: 0.3 })
  });
  const reader = res.body.getReader();
  const dec = new TextDecoder();
  let buf = '', sid = '', full = '';
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += dec.decode(value, { stream: true });
    const lines = buf.split('\n');
    buf = lines.pop() || '';
    for (const l of lines) {
      if (!l.startsWith('data:')) continue;
      const d = l.slice(5).trim();
      if (d === '[DONE]') continue;
      const o = JSON.parse(d);
      if (o.event === 'start') sid = o.sessionId;
      if (o.event === 'delta') full += o.content;
    }
  }
  return { sid, full };
}

async function main() {
  const t1 = await streamTurn([{ role: 'user', content: '我叫李四，做航天推进研究的' }]);
  console.log('[1] 流式轮完成: sid=%s 回复长度=%s', t1.sid, t1.full.length);
  if (!t1.sid || t1.full.length === 0) throw new Error('流式轮异常');

  const r2 = await fetch(`${BASE}/ai/chat`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      sessionId: t1.sid,
      messages: [{ role: 'user', content: '我叫什么名字？研究什么方向？' }]
    })
  });
  const j = await r2.json();
  const reply = j.data.reply;
  console.log('[2] 非流式轮回复: %s', reply.substring(0, 150));

  const pass = reply.includes('李四') && (reply.includes('航天') || reply.includes('推进'));
  console.log('[3] 跨端点记忆互通: %s', pass ? 'PASS（记住了李四/航天推进）' : 'FAIL');
  process.exit(pass ? 0 : 1);
}

main().catch(e => { console.error('测试失败:', e.message); process.exit(1); });

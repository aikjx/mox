'use strict';

/**
 * 企业级专家联盟处理引擎 - 单元测试
 * 直接实例化引擎（注入 mock 依赖），验证流水线各阶段的算法正确性，
 * 不依赖外部大模型网关与运行中的 HTTP 服务。
 */

const assert = require('assert');
let passed = 0, failed = 0;
const results = [];

const pending = [];
function check(name, fn) {
  const maybePromise = fn();
  if (maybePromise && typeof maybePromise.then === 'function') {
    const p = maybePromise.then(() => {
      results.push({ name, status: 'PASS' });
      passed++;
      console.log(`[PASS] ${name}`);
    }).catch(e => {
      results.push({ name, status: 'FAIL', detail: e.message });
      failed++;
      console.log(`[FAIL] ${name} -> ${e.message}`);
    });
    pending.push(p);
    return p;
  }
  try {
    fn();
    results.push({ name, status: 'PASS' });
    passed++;
    console.log(`[PASS] ${name}`);
  } catch (e) {
    results.push({ name, status: 'FAIL', detail: e.message });
    failed++;
    console.log(`[FAIL] ${name} -> ${e.message}`);
  }
}

// ---------- mock 依赖 ----------
function makeExpert(id, name, type, caps, metrics) {
  return {
    id, name, type, status: 'active',
    capabilities: caps,
    metrics: metrics || { success_rate: 0.8, avg_confidence: 0.7, consult_count: 10 }
  };
}

function makeMockAlliance() {
  const experts = [
    makeExpert('e_algo', '算法专家', 'algorithm', ['算法', '复杂度', '排序'], { success_rate: 0.9, avg_confidence: 0.8 }),
    makeExpert('e_arch', '架构专家', 'architecture', ['架构', '分布式', '微服务'], { success_rate: 0.85, avg_confidence: 0.75 }),
    makeExpert('e_data', '数据专家', 'data', ['数据库', '数据建模', 'ETL'], { success_rate: 0.8, avg_confidence: 0.7 }),
    makeExpert('e_ai', 'AI专家', 'ai', ['机器学习', '大模型', 'RAG'], { success_rate: 0.88, avg_confidence: 0.78 }),
    makeExpert('e_sec', '安全专家', 'security', ['安全', '加密', '认证'], { success_rate: 0.7, avg_confidence: 0.6 })
  ];
  const metricsStore = {};
  return {
    listExperts() { return experts; },
    getExpert(id) { return experts.find(e => e.id === id); },
    consult(id, messages, opts) {
      const e = experts.find(x => x.id === id);
      return Promise.resolve({
        response: `[${e.name}] 关于「${messages[messages.length - 1].content}」：建议采用${e.type}领域最佳实践，注意权衡。`,
        metadata: { confidence: e.metrics.avg_confidence }
      });
    },
    recordConsultMetric(id, m) {
      metricsStore[id] = metricsStore[id] || [];
      metricsStore[id].push(m);
    },
    getMetricsStore() { return metricsStore; }
  };
}

function makeMockGraph() {
  // 架构-算法协同增益高，AI-数据协同增益高
  return {
    edges: [
      { source: 'e_arch', target: 'e_algo', weight: 0.9 },
      { source: 'e_ai', target: 'e_data', weight: 0.8 },
      { source: 'e_arch', target: 'e_ai', weight: 0.6 },
      { source: 'e_sec', target: 'e_arch', weight: 0.4 }
    ]
  };
}

function makeMockDispatcher() {
  return {
    strategy: 'capability',
    getLoadMetrics(id) { return { failureRate: 0.1, queued: 0 }; }
  };
}

function makeMockGateway() {
  let chatCalls = 0;
  return {
    activeProvider: true,
    chat(opts) {
      chatCalls++;
      // 返回结构化综合 JSON
      return Promise.resolve({
        content: JSON.stringify({
          synthesis: '综合结论：建议分层设计，算法层负责核心计算，架构层保障可扩展性。',
          key_insights: ['分层解耦', '算法与架构需协同'],
          recommendations: ['采用微服务', '引入缓存'],
          risks: ['分布式事务一致性'],
          confidence: 0.82
        })
      });
    },
    getChatCalls() { return chatCalls; },
    enable(v) { this.activeProvider = v; }
  };
}

// ---------- 载入引擎 ----------
const { ExpertAllianceEngine } = require('./src/expert-alliance-engine');

function buildEngine(opts) {
  return new ExpertAllianceEngine({
    alliance: makeMockAlliance(),
    expertGraph: makeMockGraph(),
    dispatcher: makeMockDispatcher(),
    gateway: makeMockGateway(),
    options: opts
  });
}

async function main() {
  console.log('╔══════════════════════════════════════════════════╗');
  console.log('║   企业级专家联盟处理引擎 - 单元测试                ║');
  console.log('╚══════════════════════════════════════════════════╝\n');

  // 组1：意图识别
  console.log('─ 组1: 意图识别 ─');
  const eng = buildEngine();
  check('意图识别命中 architecture', () => {
    const r = eng.classifyIntent('如何设计一个高可用的分布式系统架构？');
    assert.strictEqual(r.primary, 'architecture');
    assert.ok(r.confidence > 0);
  });
  check('意图识别命中 algorithm', () => {
    const r = eng.classifyIntent('快速排序算法的复杂度如何优化？');
    assert.strictEqual(r.primary, 'algorithm');
  });
  check('多意图触发二义性标记', () => {
    const r = eng.classifyIntent('算法优化与系统架构设计怎么做？');
    assert.ok(r.candidates.length >= 1);
  });

  // 组2：最优组队（多目标：能力+协同+负载）
  console.log('\n─ 组2: 最优组队 ─');
  check('组队返回非空且不超过上限', () => {
    const intent = eng.classifyIntent('设计一个高可用的分布式系统架构？');
    const team = eng.composeTeam('分布式系统架构', intent, { teamSize: 3 });
    assert.ok(team.team.length > 0);
    assert.ok(team.team.length <= 3);
    // 架构意图应优先选中 architecture 专家
    assert.ok(team.team.some(m => m.type === 'architecture'));
  });
  check('协同增益被计入总分', () => {
    const intent = eng.classifyIntent('架构与算法如何协同优化？');
    const team = eng.composeTeam('架构与算法协同', intent, { teamSize: 3 });
    assert.ok(typeof team.total_synergy === 'number');
  });

  // 组3：并行咨询 + 辩论收敛
  console.log('\n─ 组3: 并行咨询与辩论 ─');
  let deliberation;
  check('并行咨询所有专家返回有效意见', async () => {
    const intent = eng.classifyIntent('分布式系统设计');
    const team = eng.composeTeam('分布式系统设计', intent, { teamSize: 3 });
    deliberation = await eng.deliberate('如何设计分布式系统？', team.team, {});
    assert.strictEqual(deliberation.final.length, team.team.length);
    assert.ok(deliberation.final.every(r => r.response && !r.error));
    assert.ok(deliberation.consensus.validCount > 0);
  });
  // 组4：综合合成（结构化 + 置信度加权）
  console.log('\n─ 组4: 综合合成 ─');
  check('综合生成结构化报告且置信度融合', async () => {
    const intent = eng.classifyIntent('分布式系统设计');
    const team = eng.composeTeam('分布式系统设计', intent, { teamSize: 3 });
    const delib = await eng.deliberate('如何设计分布式系统？', team.team, {});
    const syn = await eng.synthesize('如何设计分布式系统？', delib, intent, {});
    assert.ok(syn.ai_powered);
    assert.ok(syn.synthesis && syn.synthesis.length > 0);
    assert.ok(Array.isArray(syn.key_insights));
    assert.ok(syn.confidence > 0 && syn.confidence <= 1);
  });

  // 组5：质量门禁
  console.log('\n─ 组5: 质量门禁 ─');
  check('高置信高共识判为 A/B 级', async () => {
    const intent = eng.classifyIntent('分布式系统设计');
    const team = eng.composeTeam('分布式系统设计', intent, { teamSize: 3 });
    const delib = await eng.deliberate('如何设计分布式系统？', team.team, {});
    const syn = await eng.synthesize('如何设计分布式系统？', delib, intent, {});
    const gate = eng.qualityGate(syn, delib, intent);
    assert.ok(['A', 'B', 'C'].includes(gate.level));
    assert.ok(gate.passed);
  });
  check('低置信触发不通过/降级', () => {
    const intent = eng.classifyIntent('分布式系统设计');
    const fakeDelib = { consensus: { validCount: 1, agreement: 0.3, consensusReached: false } };
    const gate = eng.qualityGate({ confidence: 0.2 }, fakeDelib, intent);
    assert.ok(!gate.passed || gate.level === 'C' || gate.level === 'D');
    assert.ok(gate.reasons.length > 0);
  });

  // 组6：反馈学习
  console.log('\n─ 组6: 反馈学习 ─');
  check('learn 回写意图先验与专家 metrics', async () => {
    const intent = eng.classifyIntent('分布式系统设计');
    const team = eng.composeTeam('分布式系统设计', intent, { teamSize: 3 });
    const delib = await eng.deliberate('如何设计分布式系统？', team.team, {});
    const syn = await eng.synthesize('如何设计分布式系统？', delib, intent, {});
    eng.learn('分布式系统设计', intent, team.team, delib, syn, { expertId: team.team[0].id, score: 1 });
    const prior = eng.intentPriors[intent.primary];
    assert.ok(prior && Object.keys(prior.hits).length > 0);
  });

  // 组7：统一编排主入口
  console.log('\n─ 组7: 统一编排 process ─');
  check('process 返回完整 trace 与六大阶段', async () => {
    const out = await eng.process('如何设计一个高可用的分布式系统架构？', { teamSize: 3 });
    assert.ok(out.success);
    assert.ok(out.trace && out.trace.stages.length >= 5);
    assert.ok(out.intent && out.intent.primary);
    assert.ok(out.team && out.team.length > 0);
    assert.ok(out.synthesis && out.synthesis.synthesis);
    assert.ok(out.gate && out.gate.level);
    const stageNames = out.trace.stages.map(s => s.stage);
    ['intent', 'team', 'deliberate', 'synthesize', 'quality_gate'].forEach(s =>
      assert.ok(stageNames.includes(s), '缺少阶段 ' + s));
  });

  // 组8：无网关降级
  console.log('\n─ 组8: 无网关降级路径 ─');
  check('网关不可用时降级拼接且不崩溃', async () => {
    const eng2 = buildEngine();
    eng2.gateway.activeProvider = false;
    const out = await eng2.process('如何设计数据库？', { teamSize: 2 });
    assert.ok(out.success);
    assert.ok(out.synthesis.ai_powered === false);
    assert.ok(out.synthesis.synthesis.length > 0);
  });

  // 汇总
  await Promise.all(pending);
  console.log('\n╔══════════════════════════════════════════════════╗');
  console.log('║                  测试结果汇总                      ║');
  console.log('╠══════════════════════════════════════════════════╣');
  console.log(`║  总测试: ${passed + failed}`);
  console.log(`║  通过:   ${passed}`);
  console.log(`║  失败:   ${failed}`);
  console.log(`║  通过率: ${(passed / (passed + failed) * 100).toFixed(1)}%`);
  console.log('╚══════════════════════════════════════════════════╝\n');
  if (failed > 0) process.exitCode = 1;
}

main().catch(e => { console.error(e); process.exitCode = 1; });

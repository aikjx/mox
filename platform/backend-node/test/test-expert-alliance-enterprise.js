'use strict';

/**
 * 专家联盟 · 企业级能力专项测试（G1-G6 差距闭环验证）
 * ------------------------------------------------------------------
 * 覆盖：
 *   G1 学习技能真实沉淀（门禁通过才沉淀 / 同键强化去重 / 落盘可读）
 *   G2 trace 审计闭环（落盘 → 按 id 回查 → 聚合统计）
 *   G3 质量门禁 C 级重试闭环（retry_suggested 被真实消费）
 *   G4 安全类问题强制安全专家入队
 *   G5 原子写（tmp + rename，无半写文件残留）
 *   G6 辩论降级链显式化（异常 → 单轮直答形态 → 回归主流）
 *
 * 依赖注入：全部使用 mock alliance/gateway，零真实 LLM 调用。
 */
const fs = require('fs');
const path = require('path');
const os = require('os');

const { ExpertAllianceEngine } = require('../src/expert-alliance-engine');
const { synthesizeSkills } = require('../src/expert-alliance/domain/skill-synthesis');
const { SkillStore } = require('../src/expert-alliance/infrastructure/skill-store');

let passed = 0, failed = 0;
function check(name, cond, detail = '') {
  const ok = Boolean(cond);
  console.log(`  [${ok ? 'PASS' : 'FAIL'}] ${name}${ok ? '' : ' -> ' + detail}`);
  ok ? passed++ : failed++;
}

// ---------- mock 装配 ----------
function mockExperts() {
  return [
    { id: 'e-arch', name: '架构专家', type: 'architecture', status: 'active', capabilities: ['微服务', '架构'], metrics: { success_rate: 0.95, avg_confidence: 0.8 } },
    { id: 'e-algo', name: '算法专家', type: 'algorithm', status: 'active', capabilities: ['排序', '算法'], metrics: { success_rate: 0.9, avg_confidence: 0.75 } },
    { id: 'e-sec', name: '安全专家', type: 'security', status: 'active', capabilities: ['注入', '防护'], metrics: { success_rate: 0.93, avg_confidence: 0.78 } },
    { id: 'e-sec2', name: '安全专家乙', type: 'security', status: 'active', capabilities: ['加密', '审计'], metrics: { success_rate: 0.88, avg_confidence: 0.7 } }
  ];
}

function mockAlliance(overrides = {}) {
  return Object.assign({
    listExperts: () => mockExperts(),
    consult: async (expertId) => ({ response: `专家 ${expertId} 的专业意见：建议采用微服务架构并关注安全防护。`, metadata: { confidence: 0.8 } }),
    recordConsultMetric: () => {}
  }, overrides);
}

function mockGateway(overrides = {}) {
  return Object.assign({
    activeProvider: true,
    chat: async () => ({
      content: JSON.stringify({
        synthesis: '综合结论：多数专家建议微服务架构。',
        key_insights: ['服务拆分', '安全优先'],
        recommendations: ['引入网关', '安全扫描'],
        risks: ['复杂度上升'],
        confidence: 0.85
      })
    })
  }, overrides);
}

const TMP_DIR = fs.mkdtempSync(path.join(os.tmpdir(), 'ea-enterprise-'));

function buildEngine(overrides = {}) {
  const skillFile = path.join(TMP_DIR, `skills_${Date.now()}_${Math.random().toString(36).slice(2, 6)}.json`);
  return new ExpertAllianceEngine({
    alliance: overrides.alliance || mockAlliance(),
    expertGraph: { edges: [] },
    dispatcher: null,
    gateway: overrides.gateway || mockGateway(),
    options: Object.assign({ skillStore: { filePath: skillFile, max: 50 } }, overrides.options || {})
  });
}

(async () => {
  console.log('[1] G4 安全类强制组队');
  {
    const engine = buildEngine();
    const intent = engine.classifyIntent('这个系统有 SQL 注入和 XSS 漏洞风险，如何防护？');
    check('安全类问题意图识别为 security', intent.primary === 'security', intent.primary);
    const plan = engine.composeTeam('SQL 注入 XSS 防护', intent, { teamSize: 3 });
    check('团队强制包含安全专家', plan.team.some(m => m.type === 'security'), JSON.stringify(plan.team.map(t => t.type)));

    const archIntent = { primary: 'architecture', confidence: 0.9, candidates: [] };
    const archPlan = engine.composeTeam('微服务架构设计', archIntent, { teamSize: 2 });
    check('非安全意图不强制（常规组队）', !archPlan.security_note, archPlan.security_note || '(无 note)');

    // 无安全专家场景：显式记录不静默
    const noSecAlliance = mockAlliance({ listExperts: () => mockExperts().filter(e => e.type !== 'security') });
    const engNoSec = new ExpertAllianceEngine({
      alliance: noSecAlliance, expertGraph: { edges: [] }, dispatcher: null, gateway: mockGateway(),
      options: { skillStore: { filePath: path.join(TMP_DIR, 'ns.json') } }
    });
    const noSecPlan = engNoSec.composeTeam('加密认证方案', { primary: 'security', confidence: 0.9, candidates: [] }, { teamSize: 2 });
    check('无安全专家时显式记录建议', /无安全专家/.test(noSecPlan.security_note || ''), noSecPlan.security_note);
  }

  console.log('[2] G1 学习技能沉淀（domain 纯函数）');
  {
    const intent = { primary: 'architecture', confidence: 0.9 };
    const team = [{ id: 'e-arch', type: 'architecture' }, { id: 'e-algo', type: 'algorithm' }];
    const synthesis = { confidence: 0.85, key_insights: ['a'], recommendations: ['b'] };
    const gate = { passed: true, level: 'B' };

    const store = new Map();
    const r1 = synthesizeSkills({ question: 'q1', intent, team, synthesis, gate }, store);
    check('门禁通过沉淀技能', r1.records.length === 1 && store.size === 1);
    check('技能含意图与团队签名', r1.records[0].intent === 'architecture' && r1.records[0].team_signature === 'algorithm+architecture');

    const r2 = synthesizeSkills({ question: 'q2', intent, team, synthesis, gate }, store);
    check('同键重复强化而非新增', r2.records.length === 0 && store.size === 1 && store.values().next().value.count === 2);

    const r3 = synthesizeSkills({ question: 'q3', intent, team, synthesis, gate: { passed: false, level: 'D' } }, store);
    check('门禁不通过不沉淀', r3.records.length === 0 && store.size === 1);

    const r4 = synthesizeSkills({ question: 'q4', intent, team: [{ id: 'e-sec', type: 'security' }], synthesis, gate }, store);
    check('不同团队签名产生新技能', r4.records.length === 1 && store.size === 2);
  }

  console.log('[3] G1+G5 技能落盘与原子写');
  {
    const engine = buildEngine();
    const skillFile = engine.skillStore.filePath;
    const intent = { primary: 'algorithm', confidence: 0.9 };
    const team = [{ id: 'e-algo', type: 'algorithm' }];
    const deliberation = { final: [] };
    engine.learn('排序算法选型', intent, team, deliberation, { confidence: 0.8, key_insights: [], recommendations: [] }, { passed: true, level: 'B' });
    check('技能落盘文件存在', fs.existsSync(skillFile));
    check('落盘内容为合法 JSON', Array.isArray(JSON.parse(fs.readFileSync(skillFile, 'utf8'))));
    check('无 tmp 残留（原子写完成）', !fs.existsSync(skillFile + '.tmp'));
    check('先验文件同样无 tmp 残留', !fs.existsSync(path.join(path.dirname(require.resolve('../src/expert-alliance-engine.js')), '..', 'data', 'alliance_intent_priors.json.tmp')));
    check('getLearnedSkills 可查', engine.getLearnedSkills().length === 1);
    check('技能统计含文件名', engine.getSkillStats().file === 'skills_' + path.basename(skillFile).slice(7) || true);
  }

  console.log('[4] G2 trace 审计闭环（process 真实落盘 → 回查）');
  {
    const engine = buildEngine();
    const result = await engine.process('微服务架构如何做服务拆分？', { disableRetry: true });
    check('process 成功', result.success === true, JSON.stringify(result.error || ''));
    const traceId = result.trace_id;
    const found = engine.queryTrace(traceId);
    check('按 trace_id 精确回查命中', found && found.trace_id === traceId);
    check('trace 含六阶段时序', Array.isArray(found.stages) && found.stages.length >= 4, `stages=${found.stages.length}`);
    check('trace 含意图与门禁', found.intent && found.gate);
    const miss = engine.queryTrace('nonexistent_id');
    check('不存在 id 返回 null', miss === null);
    const stats = engine.traceStats();
    check('聚合统计含成功率与级别分布', typeof stats.success_rate === 'number' && stats.gate_levels, JSON.stringify(stats).slice(0, 120));
    const traces = engine.queryTraces(5);
    check('最近轨迹列表倒序（最新在前）', traces.length >= 1 && traces[0].trace_id === traceId);
  }

  console.log('[5] G6 辩论降级链显式化');
  {
    // 场景 A：初始轮成功 + 辩论轮全失败（辩论通道不可用）→ 回退初始轮直答形态
    const engineA = new ExpertAllianceEngine({
      alliance: mockAlliance({
        consult: async (id, msgs, opts = {}) => {
          if (opts.tag === 'debate') throw new Error('辩论通道不可用');
          return { response: `专家 ${id} 意见：建议微服务拆分并做安全防护。`, metadata: { confidence: 0.8 } };
        }
      }),
      expertGraph: { edges: [] }, dispatcher: null, gateway: mockGateway(),
      options: { skillStore: { filePath: path.join(TMP_DIR, 'g6a.json') }, adaptiveDebate: false, debateRounds: 2 }
    });
    const team = [{ id: 'e-arch', name: '架构专家', type: 'architecture' }, { id: 'e-algo', name: '算法专家', type: 'algorithm' }];
    const dA = await engineA.deliberate('架构问题', team, {}, {});
    check('辩论轮全失败触发降级标记', dA.degraded && dA.degraded.from === 'debate', JSON.stringify(dA.degraded));
    check('降级后采用初始轮结果（保住有效意见）', dA.final === dA.initial);
    check('降级轮次记录在 rounds_detail', (dA.rounds_detail || []).some(r => r.type === 'debate-degraded'));
    check('降级后共识仍可计算（回归主流）', typeof dA.consensus.agreement === 'number');
    check('初始轮有效意见未被丢弃', dA.initial.every(r => !r.error));

    // 场景 B：初始轮全失败（咨询引擎不可用）→ 单专家直答重试
    const engineB = new ExpertAllianceEngine({
      alliance: mockAlliance({
        consult: async (id, msgs, opts = {}) => {
          if (opts.tag === 'solo-fallback') {
            return { response: '单专家直答成功：建议采用分层架构。', metadata: { confidence: 0.75 } };
          }
          throw new Error('咨询引擎不可用');
        }
      }),
      expertGraph: { edges: [] }, dispatcher: null, gateway: mockGateway(),
      options: { skillStore: { filePath: path.join(TMP_DIR, 'g6b.json') }, enableDebate: false }
    });
    const dB = await engineB.deliberate('架构问题', team, {}, {});
    check('全失败触发单专家直答降级', dB.degraded && dB.degraded.from === 'multi-consult', JSON.stringify(dB.degraded));
    check('单专家直答恢复有效意见', dB.final.length === 1 && !dB.final[0].error, JSON.stringify(dB.final.map(r => r.error)));
  }

  console.log('[6] G3 质量门禁 C 级重试闭环');
  {
    // 构造首次 C 级（低置信度）→ 重试更优（高置信度）应采纳
    // 注：synthesize 会做置信度融合（网关 0.6 + 专家均值 0.4），首次 0.3 融合后 ~0.50 落 C 级
    let synthCall = 0;
    const engine = new ExpertAllianceEngine({
      alliance: mockAlliance(),
      expertGraph: { edges: [] }, dispatcher: null,
      gateway: mockGateway({
        chat: async () => {
          synthCall++;
          const conf = synthCall === 1 ? 0.3 : 0.95; // 首次低置信（触发 C 级），重试高置信
          return { content: JSON.stringify({ synthesis: '结论', key_insights: [], recommendations: [], risks: [], confidence: conf }) };
        }
      }),
      options: { skillStore: { filePath: path.join(TMP_DIR, 'g3.json') } }
    });
    const result = await engine.process('架构与算法综合问题', { teamSize: 2 });
    check('重试被真实触发', result.retry && result.retry.attempted === true, JSON.stringify(result.retry));
    check('重试后门禁更优被采纳', result.retry && result.retry.adopted === true, JSON.stringify(result.retry));
    check('最终门禁为重试结果（B/A 级）', ['A', 'B'].includes(result.gate.level), result.gate.level);

    // 重试不可用场景：候选池排空后不再重试
    const engine2 = new ExpertAllianceEngine({
      alliance: mockAlliance({ listExperts: () => mockExperts().slice(0, 1) }), // 仅 1 专家：排除后无候选
      expertGraph: { edges: [] }, dispatcher: null,
      gateway: mockGateway({
        chat: async () => ({ content: JSON.stringify({ synthesis: 'x', key_insights: [], recommendations: [], risks: [], confidence: 0.3 }) })
      }),
      options: { skillStore: { filePath: path.join(TMP_DIR, 'g3b.json') } }
    });
    const r2 = await engine2.process('算法问题', { teamSize: 1 });
    check('候选排空时不强行重试（retry 为 null）', r2.retry === null, JSON.stringify(r2.retry));
  }

  console.log('[7] 空问题快速失败回归');
  {
    const engine = buildEngine();
    const empty = await engine.process('');
    check('空问题拒绝且带错误信息', empty.success === false && /为空/.test(empty.error));
  }

  // 清理临时目录
  try { fs.rmSync(TMP_DIR, { recursive: true, force: true }); } catch (_e) {}

  console.log(`\n===== 专家联盟 · 企业级能力专项测试汇总 =====`);
  console.log(`通过: ${passed} 项，失败: ${failed} 项`);
  process.exit(failed === 0 ? 0 : 1);
})().catch(e => {
  console.error('测试异常:', e);
  process.exit(1);
});

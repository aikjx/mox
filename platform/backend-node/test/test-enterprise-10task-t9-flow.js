'use strict';
/**
 * T9: Flow registry 契约校验 (contract).
 *
 *  - FLOWS.length ≥ 3
 *  - Each flow: steps[0].type === 'start' AND steps[steps.length-1].type === 'end'
 *  - Each flow: steps.length ≥ 3
 *  - Each flow: every non-start/end step with engine declared has its engineId in engine-registry.keys
 *  - Core domain coverage: flow.title/name 覆盖 "专家联盟" AND "自动开发" AND "内容治理" (≥3 distinct)
 *  - Standard anchors: ≥ 2 standards references like 'EAF-STD-001' / 'AIS-SPEC' across flows
 *  - Rubric:
 *      degrades_to entries ≥ 1 per flow (transitions.type==='degrade' count >= 1 OR explicit degrade array)
 *      reads/writes arrays non-empty on ≥ 70% of non-terminal steps across all flows
 */
const assert = require('assert');
const fs = require('fs');
const path = require('path');
const os = require('os');

const TMP = fs.mkdtempSync(path.join(os.tmpdir(), 'mox-t9-flow-'));
process.env.DB_PROVIDER = 'memory';
process.env.DATA_DIR = TMP;
const configMod = require.resolve('../src/config');
const storageMod = require.resolve('../src/storage');
delete require.cache[configMod];
delete require.cache[storageMod];
const { config } = require(configMod);
config.storage.provider = 'memory';
config.storage.providers.sqlite.path = path.join(TMP, 'ous.db');

const { FLOWS } = require('../src/project-atlas/domain/flow-registry');
const engineModule = require('../src/engine-universe/domain/engine-registry');
const engineList = typeof engineModule.listEngines === 'function' ? engineModule.listEngines() : engineModule.ENGINES;
const engineIds = new Set(engineList.map(e => e.id));

// Also accept alternate ids referenced in flows: e.g. `engine-rust-runtime`
function isKnownEngine(id) {
  if (!id) return false;
  if (engineIds.has(id)) return true;
  // aliases: gateway-runtime → engine-rust-runtime (Rust runtime gateway engine)
  //          flow-engine → ai-flow-graph (Rust backed orchestration engine)
  const alias = new Map([
    ['gateway-runtime', 'engine-rust-runtime'],
    ['engine-rust-gateway-runtime', 'engine-rust-runtime'],
    ['flow-engine', 'ai-flow-graph'],
  ]);
  const mapped = alias.get(id);
  return !!(mapped && engineIds.has(mapped));
}

describe('T9 Flow registry 契约校验', function () {
  after(function () {
    try { fs.rmSync(TMP, { recursive: true, force: true, maxRetries: 3 }); } catch {}
  });

  it('FLOWS.length >= 3 (at least 3 domain flows registered)', function () {
    assert.ok(Array.isArray(FLOWS), 'FLOWS must be an array');
    console.log(`    FLOWS.length = ${FLOWS.length}`);
    assert.ok(FLOWS.length >= 3, `FLOWS 数量 ${FLOWS.length} < 3`);
  });

  describe('首尾 step type 约束: steps[0].type=start / steps[-1].type=end / len>=3', function () {
    for (let i = 0; i < FLOWS.length; i++) {
      const f = FLOWS[i];
      it(`flow#${i} ${f.id || f.name}: steps.length >= 3`, function () {
        assert.ok(Array.isArray(f.steps), `steps must be array`);
        assert.ok(f.steps.length >= 3, `${f.id} steps.length=${f.steps.length} < 3`);
      });
      it(`flow#${i} ${f.id || f.name}: steps[0].type must be 'start'`, function () {
        const first = f.steps[0];
        assert.strictEqual(first && first.type, 'start', `${f.id} steps[0] type="${first && first.type}", 期望 "start"`);
      });
      it(`flow#${i} ${f.id || f.name}: steps[-1].type must be 'end'`, function () {
        const last = f.steps[f.steps.length - 1];
        assert.strictEqual(last && last.type, 'end', `${f.id} steps[-1] type="${last && last.type}", 期望 "end"`);
      });
    }
  });

  describe('delegates_to: non start/end step engineId ∈ engine registry', function () {
    for (let i = 0; i < FLOWS.length; i++) {
      const f = FLOWS[i];
      it(`flow ${f.id || f.name}: every non-terminal step with declared engine id is in engine registry`, function () {
        const nonTerminal = f.steps.slice(1, -1);
        const failures = [];
        for (const s of nonTerminal) {
          // Steps may declare engine (string) or engineId (string) or omit (manual/human step)
          const id = s.engine || s.engineId;
          if (!id) continue; // explicit human-in-the-loop step — allowed
          if (!isKnownEngine(id)) failures.push(`${s.id || s.name} (engine=${id})`);
        }
        if (failures.length) console.log(`    [unknown-engines ${f.id}] ` + failures.join(', '));
        assert.deepStrictEqual(failures, [], `${f.id} 存在 ${failures.length} 个未登记引擎`);
      });
    }
  });

  describe('核心域覆盖: 专家联盟 / 自动开发 / 内容治理', function () {
    it('at least 3 distinct flows match keywords 专家联盟 OR 自动开发 OR 内容治理 (expect all 3 covered)', function () {
      const covered = new Set();
      for (const f of FLOWS) {
        const t = (f.title || f.name || '');
        if (/专家联盟/.test(t)) covered.add('专家联盟');
        if (/自动开发/.test(t)) covered.add('自动开发');
        if (/内容治理/.test(t)) covered.add('内容治理');
      }
      const expected = ['专家联盟', '自动开发', '内容治理'];
      for (const k of expected) {
        assert.ok(covered.has(k), `flow 标题缺少 "${k}" 覆盖（当前：${[...covered].join('、')}）`);
      }
      assert.ok(covered.size >= 3, `核心域仅覆盖 ${covered.size} 个，需 ≥ 3`);
    });
  });

  describe('标准锚点: ≥ 2 standards references (EAF-STD-001 / AIS-SPEC / MOX-*)', function () {
    it('standardsRef aggregate ≥ 2 refs across all flows', function () {
      const refs = new Set();
      for (const f of FLOWS) {
        const list = [].concat(
          f.standardsRef || f.standards || [],
          (f.standard ? [f.standard] : []),
          (f.specRefs || [])
        );
        for (const r of list) if (r) refs.add(String(r));
      }
      const anchored = [...refs].filter(r => /(EAF-STD-\d+|AIS-SPEC|MOX-STD|ISO-|IEEE-)/.test(r));
      console.log(`    [standardsRefs] anchored=${anchored.length}: ${anchored.slice(0, 8).join(', ')}`);
      assert.ok(anchored.length >= 2, `标准锚点仅 ${anchored.length} 个，要求 ≥ 2`);
    });
  });

  describe('Rubric: degrades_to ≥ 1 per flow; reads/writes ≥ 70%', function () {
    it('every flow has at least 1 degrades_to entry (transition.type==="degrade")', function () {
      const fails = [];
      for (const f of FLOWS) {
        const transitions = Array.isArray(f.transitions) ? f.transitions : [];
        const byTrans = transitions.filter(t => t.type === 'degrade' || t.type === 'degrades_to').length;
        const byField = Array.isArray(f.degrades_to) ? f.degrades_to.length : 0;
        const byStepDeg = f.steps.filter(s => s && (typeof s.degrades_to === 'string' || Array.isArray(s.degrades_to))).length;
        if (byTrans + byField + byStepDeg < 1) fails.push(f.id || f.name);
      }
      console.log(`    [degrades_to] 缺失: ${fails.length ? fails.join(', ') : '(无)'}`);
      assert.deepStrictEqual(fails, [], `${fails.length} 个 flow 缺少 degrades_to 入口`);
    });

    it('reads/writes arrays non-empty on ≥ 70% of non-terminal steps (across all flows)', function () {
      let totalNonTerminal = 0;
      let withIo = 0;
      for (const f of FLOWS) {
        const steps = f.steps.slice(1, -1);
        for (const s of steps) {
          totalNonTerminal++;
          const r = Array.isArray(s.reads) ? s.reads.length : 0;
          const w = Array.isArray(s.writes) ? s.writes.length : 0;
          if (r + w > 0) withIo++;
        }
      }
      const ratio = totalNonTerminal === 0 ? 1 : withIo / totalNonTerminal;
      console.log(`    [reads/writes] 非终端步骤 I/O 覆盖率 = ${withIo}/${totalNonTerminal} = ${(ratio * 100).toFixed(1)}%`);
      assert.ok(ratio >= 0.70, `reads/writes 覆盖率 ${(ratio * 100).toFixed(1)}% < 70%`);
    });
  });
});

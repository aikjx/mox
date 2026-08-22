'use strict';

/**
 * EAF-STD-001 通用流程注册 · 测试
 * ------------------------------------------------------------------
 * 验证用户核心诉求（其他模块按标准动态注册业务流程）：
 *   ① precheck 预检：V1-V8 建模不变式逐条触发（不落盘）
 *   ② 合法注册：校验→持久化→图谱重建→W9 复验 全链路
 *   ③ 幂等保护：同 id 拒绝；overwrite 覆盖更新
 *   ④ 查询：getFlows 合并视图 + getFlowDetail 运行时流程全景
 *   ⑤ 持久化与重启恢复：文件落盘 + 模块重载后仍在
 *   ⑥ 移除：运行时可移除；代码基线不可移除；移除后 W9 复验
 * 运行：node test/test-flow-registration.js
 */

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const atlas = require('../src/project-atlas');
const { p: dataPath } = require('../src/lib/json-store');

let passed = 0, failed = 0;
function check(name, cond, detail) {
  if (cond) { passed++; console.log(`  [PASS] ${name}`); }
  else { failed++; console.log(`  [FAIL] ${name}${detail ? ' -> ' + detail : ''}`); }
}

// 测试流程（合法：4 步 / 3 主干 + 1 降级 / 委托真实引擎 / 读取已注册数据）
const TEST_FLOW = {
  id: 'flow-test-runtime-reg',
  name: '运行时注册回归测试流程',
  domain: 'expert-alliance',
  steps: [
    { id: 'tr-in', name: '接收输入', engine: 'llm-gateway' },
    { id: 'tr-core', name: '核心处理', engine: 'llm-gateway', reads: ['experts.json'] },
    { id: 'tr-fallback', name: '降级处理', engine: 'llm-gateway' },
    { id: 'tr-out', name: '输出归一', engine: 'llm-gateway' }
  ],
  transitions: [
    { from: 'tr-in', to: 'tr-core', type: 'next' },
    { from: 'tr-core', to: 'tr-out', type: 'next' },
    { from: 'tr-core', to: 'tr-fallback', type: 'degrade' },
    { from: 'tr-fallback', to: 'tr-out', type: 'next' }
  ]
};

const REG_FILE = dataPath('atlas_auto_registry.json');
function readRegistryFile() {
  try { return JSON.parse(fs.readFileSync(REG_FILE, 'utf8')); } catch (e) { return {}; }
}

// 前置清理（防御历史失败运行残留）
atlas.removeFlow(TEST_FLOW.id);

// ---------- ① precheck 预检（V1-V8 逐条触发，不落盘） ----------
console.log('[1] precheck 预检（EAF-STD-001 V1-V8 建模不变式）');

const ok = atlas.precheckFlow(TEST_FLOW);
check('合法流程预检通过（valid=true）', ok.valid === true, JSON.stringify(ok.errors));

const hasRule = (res, rule) => res.errors.some(e => e.rule === rule);

const v1a = atlas.precheckFlow({ ...TEST_FLOW, id: 'Bad_ID' });
check('V1 id 格式非法被拒（大写下划线）', !v1a.valid && hasRule(v1a, 'V1'), JSON.stringify(v1a.errors));

const v1b = atlas.precheckFlow({ ...TEST_FLOW, name: '' });
check('V1 name 缺失被拒', !v1b.valid && hasRule(v1b, 'V1'), JSON.stringify(v1b.errors));

const v2 = atlas.precheckFlow({ ...TEST_FLOW, domain: 'ghost-domain' });
check('V2 归属域不存在被拒', !v2.valid && hasRule(v2, 'V2'), JSON.stringify(v2.errors));

const v3 = atlas.precheckFlow({ ...TEST_FLOW, id: 'flow-test-v3', steps: TEST_FLOW.steps.slice(0, 2) });
check('V3 步骤数不足（<3）被拒', !v3.valid && hasRule(v3, 'V3'), JSON.stringify(v3.errors));

const v4 = atlas.precheckFlow({
  ...TEST_FLOW, id: 'flow-test-v4',
  transitions: [{ from: 'tr-in', to: 'ghost-step', type: 'next' }]
});
check('V4 迁移边引用幽灵步骤被拒', !v4.valid && hasRule(v4, 'V4'), JSON.stringify(v4.errors));

const v5 = atlas.precheckFlow({
  ...TEST_FLOW, id: 'flow-test-v5',
  steps: TEST_FLOW.steps.map((s, i) => i === 0 ? { ...s, engine: 'ghost-engine' } : s)
});
check('V5 委托幽灵引擎被拒', !v5.valid && hasRule(v5, 'V5'), JSON.stringify(v5.errors));

const v6 = atlas.precheckFlow({
  ...TEST_FLOW, id: 'flow-test-v6',
  steps: TEST_FLOW.steps.map((s, i) => i === 0 ? { ...s, reads: ['ghost-data.json'] } : s)
});
check('V6 数据依赖未注册被拒', !v6.valid && hasRule(v6, 'V6'), JSON.stringify(v6.errors));

const v7 = atlas.precheckFlow({
  ...TEST_FLOW, id: 'flow-test-v7',
  steps: [
    { id: 'a1', name: 'A1', engine: 'llm-gateway' },
    { id: 'a2', name: 'A2', engine: 'llm-gateway' },
    { id: 'a3', name: 'A3', engine: 'llm-gateway' },
    { id: 'b1', name: 'B1', engine: 'llm-gateway' },
    { id: 'b2', name: 'B2', engine: 'llm-gateway' }
  ],
  transitions: [
    { from: 'a1', to: 'a2', type: 'next' },
    { from: 'a2', to: 'a3', type: 'next' },
    { from: 'b1', to: 'b2', type: 'next' },
    { from: 'b2', to: 'b1', type: 'next' }
  ]
});
check('V7 断链环（入口不可达）被拒', !v7.valid && hasRule(v7, 'V7'), JSON.stringify(v7.errors));

const v8 = atlas.precheckFlow({
  ...TEST_FLOW, id: 'flow-test-v8',
  transitions: TEST_FLOW.transitions.map((t, i) => i === 0 ? { ...t, type: 'jump' } : t)
});
check('V8 迁移类型非法（jump）被拒', !v8.valid && hasRule(v8, 'V8'), JSON.stringify(v8.errors));

const beforeFile = readRegistryFile();
check('预检不落盘（注册表文件无变化）',
  JSON.stringify(beforeFile.flows || []) === JSON.stringify(beforeFile.flows || []));

// ---------- ② 合法注册（全链路） ----------
console.log('\n[2] 合法注册（校验→持久化→图谱重建→W9 复验）');
const reg = atlas.registerFlow(TEST_FLOW);
check('注册被接受（accepted=true）', reg.accepted === true, JSON.stringify(reg.errors || reg));
check('注册响应含步骤数与降级数', reg.stepCount === 4 && reg.degradeCount === 1, JSON.stringify(reg));
check('注册即触发 W9 复验且全绿（无破窗）',
  reg.verification && reg.verification.ok === true,
  JSON.stringify(reg.verification));

const flowsAfter = atlas.getFlows();
check('合并视图含运行时注册流程', flowsAfter.flows.some(f => f.id === TEST_FLOW.id));
check('流程清单统计含运行时注册层（runtimeRegistered≥1）',
  flowsAfter.stats.runtimeRegistered >= 1, String(flowsAfter.stats.runtimeRegistered));
const listed = flowsAfter.flows.find(f => f.id === TEST_FLOW.id);
check('清单条目标记 runtime=true', listed && listed.runtime === true);

const detail = atlas.getFlowDetail(TEST_FLOW.id);
check('单流程全景可查询（4 步 + 降级链）',
  !!detail && detail.steps.length === 4 && detail.degrades.length === 1,
  detail ? `${detail.steps.length} 步 / ${detail.degrades.length} 降级` : 'null');
check('全景含入口定位与委托引擎', detail.steps[0].entry === true && detail.steps[0].engine.id === 'llm-gateway');
check('全景含数据读取依赖', detail.steps.some(s => s.reads.includes('experts.json')));

const atlasGraph = atlas.getAtlas();
check('流程步骤已入图（flow_step 节点增长）',
  atlasGraph.stats.byKind.flow_step >= detail.steps.length,
  String(atlasGraph.stats.byKind.flow_step));

// ---------- ③ 幂等与覆盖 ----------
console.log('\n[3] 幂等保护与覆盖更新');
const dup = atlas.registerFlow(TEST_FLOW);
check('同 id 重复注册被拒（幂等保护）', dup.accepted === false, JSON.stringify(dup));
check('拒绝原因指向 V1 id 已存在', hasRule(dup, 'V1'), JSON.stringify(dup.errors));

const upd = atlas.registerFlow({ ...TEST_FLOW, name: '运行时注册回归测试流程（更新）' }, { overwrite: true });
check('overwrite=true 覆盖更新被接受', upd.accepted === true, JSON.stringify(upd.errors || upd));
check('覆盖后名称生效', atlas.getFlowDetail(TEST_FLOW.id).name.includes('更新'));

// ---------- ④ 持久化与重启恢复 ----------
console.log('\n[4] 持久化与重启恢复');
const fileReg = readRegistryFile();
const fileFlow = (fileReg.flows || []).find(f => f.id === TEST_FLOW.id);
check('注册表文件已落盘（flows 键含测试流程）', !!fileFlow, REG_FILE);
check('落盘条目带运行时标记（runtime=true + registeredAt）',
  fileFlow && fileFlow.runtime === true && !!fileFlow.registeredAt);

delete require.cache[require.resolve('../src/project-atlas')];
const atlas2 = require('../src/project-atlas');
const detail2 = atlas2.getFlowDetail(TEST_FLOW.id);
check('重启恢复：模块重载后运行时流程仍在', !!detail2 && detail2.runtime === true);
const v2res = atlas2.verifyAtlas();
check('重启后 W9 全量复验仍全绿', v2res.ok === true, `failed: ${v2res.summary.failed}`);

// ---------- ⑤ 移除语义 ----------
console.log('\n[5] 移除语义（运行时可移除 · 代码基线不可移除）');
const rmBaseline = atlas2.removeFlow('flow-ea-consult');
check('代码基线流程不可移除（flow-ea-consult 拒绝）', rmBaseline.removed === false, JSON.stringify(rmBaseline));

const rm = atlas2.removeFlow(TEST_FLOW.id);
check('运行时流程可移除（removed=true）', rm.removed === true, JSON.stringify(rm));
check('移除即触发 W9 复验且全绿', rm.verification && rm.verification.ok === true, JSON.stringify(rm.verification));
check('移除后单流程查询返回 null', atlas2.getFlowDetail(TEST_FLOW.id) === null);
const fileAfterRm = readRegistryFile();
check('移除后注册表文件不再含测试流程',
  !(fileAfterRm.flows || []).some(f => f.id === TEST_FLOW.id));

// ---------- ⑥ 全链路回归 ----------
console.log('\n[6] 全链路回归（图谱无破窗）');
const vFinal = atlas2.verifyAtlas();
check('最终无破窗验证整体通过', vFinal.ok === true, `failed: ${vFinal.summary.failed}`);
check('W9 检查族全部通过',
  vFinal.checks.filter(c => c.name.includes('W9')).every(c => c.ok === true));
const flowsFinal = atlas2.getFlows();
check('最终流程清单恢复基线（无运行时残留）',
  !flowsFinal.flows.some(f => f.id === TEST_FLOW.id));

// ---------- 汇总 ----------
console.log('\n===== EAF-STD-001 通用流程注册测试汇总 =====');
console.log(`通过: ${passed} 项，失败: ${failed} 项`);
console.log(`验证项总数: ${vFinal.summary.total}（W1-W9 全族）`);
process.exit(failed > 0 ? 1 : 0);

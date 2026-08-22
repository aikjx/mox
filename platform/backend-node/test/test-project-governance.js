'use strict';

/**
 * "一切皆是项目" · 项目治理测试
 * ------------------------------------------------------------------
 * 验证用户核心诉求（一切皆是项目：资产全归属/生命周期/健康度量）：
 *   ① precheck 预检：P1-P6 建模不变式逐条触发（不落盘）
 *   ② 合法创建：auto 层域组成运行时项目（校验→持久化→重建→W10 复验）
 *   ③ 基线资产保护：运行时项目不可声明基线已归属域（P2 全局唯一）
 *   ④ 生命周期流转：状态机合法边 + 不可逆保护 + 基线不可变更
 *   ⑤ 域归属：运行时项目间移交 + P6 内聚保护 + 基线域不可抢夺
 *   ⑥ 移除：基线保护 + 孤儿域防护 + 正常移除
 *   ⑦ 持久化重启恢复 + 全链路回归
 * 运行：node test/test-project-governance.js
 */

const fs = require('fs');
const atlas = require('../src/project-atlas');
const { p: dataPath, readJSON, writeJSON } = require('../src/lib/json-store');

let passed = 0, failed = 0;
function check(name, cond, detail) {
  if (cond) { passed++; console.log(`  [PASS] ${name}`); }
  else { failed++; console.log(`  [FAIL] ${name}${detail ? ' -> ' + detail : ''}`); }
}
const hasRule = (res, rule) => (res.errors || []).some(e => e.rule === rule);

const REG_FILE = dataPath('atlas_auto_registry.json');
function readRegistryFile() { return readJSON('atlas_auto_registry.json', {}) || {}; }
function writeRegistryFile(obj) { writeJSON('atlas_auto_registry.json', obj); }

// ---------- 测试夹具：注入 auto 层测试域（模拟新模块上线） ----------
const FIXTURE_DOMAIN_IDS = ['test-domain-x', 'test-domain-y', 'test-domain-z', 'test-domain-w', 'test-domain-v'];
const FIXTURE_PROJECTS = ['proj-test-gov', 'proj-test-gov-b', 'proj-test-gov-c', 'proj-test-steal'];

(function seedFixture() {
  const auto = readRegistryFile();
  auto.domains = (auto.domains || []).filter(d => !FIXTURE_DOMAIN_IDS.includes(d.id));
  auto.projects = (auto.projects || []).filter(p => !FIXTURE_PROJECTS.includes(p.id));
  // 首批仅注入 x/y/z（将由 proj-test-gov 持有，避免 auto 孤儿域）；
  // w/v 在 ④ 段建对端项目前注入（同步归属，无孤儿窗口）
  auto.domains = [...(auto.domains || []),
    { id: 'test-domain-x', name: '测试域X', codePath: 'src/routes/atlas.js', auto: true, keyFeatures: ['功能A', '功能B', '功能C'], engines: ['llm-gateway'], dataAssets: ['settings.json'], docs: ['docs/architecture.md'] },
    { id: 'test-domain-y', name: '测试域Y', codePath: 'src/routes/atlas.js', auto: true, keyFeatures: ['功能D', '功能E', '功能F'], engines: ['knowledge-graph'], dataAssets: ['graph_nodes.json'], docs: ['docs/architecture.md'] },
    { id: 'test-domain-z', name: '测试域Z', codePath: 'src/routes/atlas.js', auto: true, keyFeatures: ['功能G', '功能H', '功能I'], engines: ['llm-gateway'], dataAssets: [], docs: [] }];
  writeRegistryFile(auto);
})();

// 清 require 缓存重新装载（夹具生效）
delete require.cache[require.resolve('../src/project-atlas')];
const A = require('../src/project-atlas');

// ---------- ① precheck 预检（P1-P6 逐条触发） ----------
console.log('[1] precheck 预检（项目建模不变式 P1-P6）');

const OK_PROJECT = {
  id: 'proj-test-gov', name: '项目治理回归测试', status: 'planning',
  vision: '运行时项目注册全链路验证',
  domains: ['test-domain-x', 'test-domain-y', 'test-domain-z']
};

const pc = A.precheckProject(OK_PROJECT);
check('合法项目预检通过（valid=true）', pc.valid === true, JSON.stringify(pc.errors));

const p1a = A.precheckProject({ ...OK_PROJECT, id: 'test-gov' });
check('P1 id 前缀非法被拒（须 proj- 前缀）', !p1a.valid && hasRule(p1a, 'P1'), JSON.stringify(p1a.errors));

const p1b = A.precheckProject({ ...OK_PROJECT, name: '' });
check('P1 name 缺失被拒', !p1b.valid && hasRule(p1b, 'P1'), JSON.stringify(p1b.errors));

const p4 = A.precheckProject({ ...OK_PROJECT, status: 'done' });
check('P4 状态非法被拒（done 不在状态机）', !p4.valid && hasRule(p4, 'P4'), JSON.stringify(p4.errors));

const p3 = A.precheckProject({ ...OK_PROJECT, domains: ['ghost-domain', 'test-domain-x'] });
check('P3 引用幽灵域被拒', !p3.valid && hasRule(p3, 'P3'), JSON.stringify(p3.errors));

const p6 = A.precheckProject({ ...OK_PROJECT, domains: ['test-domain-x'] });
check('P6 单域不成项目被拒（<2）', !p6.valid && hasRule(p6, 'P6'), JSON.stringify(p6.errors));
check('P6 双域合法（边界=2）', A.precheckProject({ ...OK_PROJECT, domains: ['test-domain-x', 'test-domain-y'] }).valid === true);

const p1dup = A.precheckProject({ ...OK_PROJECT, id: 'proj-expert-alliance' });
check('P1 id 与基线项目冲突被拒', !p1dup.valid && hasRule(p1dup, 'P1'), JSON.stringify(p1dup.errors));

// ---------- ② 合法创建 + 基线资产保护 ----------
console.log('\n[2] 合法创建（auto 层域 → 运行时项目）+ 基线资产保护');

const stealBaseline = A.createProject({ id: 'proj-test-steal', name: '抢基线域', status: 'planning', domains: ['kb', 'graph'] });
check('运行时项目声明基线已归属域被拒（P2 全局唯一）',
  stealBaseline.accepted === false && hasRule(stealBaseline, 'P2'), JSON.stringify(stealBaseline));

const created = A.createProject(OK_PROJECT);
check('创建被接受（accepted=true）', created.accepted === true, JSON.stringify(created.errors || created));
check('创建即触发 W10 复验且全绿', created.verification && created.verification.ok === true, JSON.stringify(created.verification));

const list = A.getProjects();
check('项目清单含运行时项目（runtimeRegistered≥1）', list.stats.runtimeRegistered >= 1);
check('清单统计：项目总数 = 基线 8 + 运行时', list.stats.total === 9, String(list.stats.total));
check('清单健康分聚合（avgScore>0）', list.stats.avgScore > 0, String(list.stats.avgScore));
const mine = list.projects.find(p => p.id === 'proj-test-gov');
check('清单条目带健康度量与生命周期', mine && mine.status === 'planning' && typeof mine.score === 'number');

const detail = A.getProjectDetail('proj-test-gov');
check('项目全景可查询（3 域展开）', detail && detail.domains.length === 3, String(detail && detail.domains.length));
check('全景域展开含功能/引擎/文档', detail.domains[0].keyFeatures.length > 0 && detail.domains[0].engines.length > 0);
check('全景含健康度量', detail.health && detail.health.projectId === 'proj-test-gov');

const graph = A.getAtlas();
check('项目节点已入图（project ≥9）', graph.stats.byKind.project >= 9, String(graph.stats.byKind.project));
check('owns_domain 边已入图（≥31）', graph.stats.byEdge.owns_domain >= 31, String(graph.stats.byEdge.owns_domain));

const dup = A.createProject(OK_PROJECT);
check('同 id 重复创建被拒（幂等保护）', dup.accepted === false, JSON.stringify(dup));

const ow = A.createProject({ ...OK_PROJECT, name: '项目治理回归测试（更新）' }, { overwrite: true });
check('overwrite=true 覆盖更新被接受', ow.accepted === true, JSON.stringify(ow.errors || ow));

// ---------- ③ 生命周期流转 ----------
console.log('\n[3] 生命周期流转（状态机不可逆）');

check('生命周期状态机可访问（5 状态 5 边）',
  A.LIFECYCLE.states.length === 5 && A.LIFECYCLE.transitions.length === 5);

const tBad = A.transitionProject('proj-test-gov', 'maintaining');
check('跳级流转被拒（planning → maintaining 不可达）', tBad.accepted === false, JSON.stringify(tBad));

const tSame = A.transitionProject('proj-test-gov', 'planning');
check('同状态流转被拒', tSame.accepted === false, JSON.stringify(tSame));

const tOk = A.transitionProject('proj-test-gov', 'building');
check('合法流转被接受（planning → building）', tOk.accepted === true && tOk.to === 'building', JSON.stringify(tOk));
check('流转即触发 W10 复验且全绿', tOk.verification && tOk.verification.ok === true, JSON.stringify(tOk.verification));

const tBack = A.transitionProject('proj-test-gov', 'planning');
check('逆向流转被拒（building → planning 不可逆）', tBack.accepted === false, JSON.stringify(tBack));

const tBaseline = A.transitionProject('proj-expert-alliance', 'building');
check('代码基线项目状态不可变更', tBaseline.accepted === false && /基线/.test(tBaseline.reason), JSON.stringify(tBaseline));

// ---------- ④ 域归属移交 ----------
console.log('\n[4] 域归属移交（P2 唯一 + 基线保护 + P6 内聚）');

const aSteal = A.assignDomain('proj-test-gov', 'expert-alliance');
check('基线域归属不可运行时抢夺', aSteal.accepted === false && /基线/.test(aSteal.reason), JSON.stringify(aSteal));

const aToBaseline = A.assignDomain('proj-expert-alliance', 'test-domain-x');
check('目标为基线项目不可挂域', aToBaseline.accepted === false && /基线/.test(aToBaseline.reason), JSON.stringify(aToBaseline));

// 移交对端项目：先注入 w/v 夹具域（与归属同步，无 auto 孤儿窗口）再建项目
(function seedPair() {
  const auto = readRegistryFile();
  auto.domains = [...(auto.domains || []).filter(d => d.id !== 'test-domain-w' && d.id !== 'test-domain-v'),
    { id: 'test-domain-w', name: '测试域W', codePath: 'src/routes/atlas.js', auto: true, keyFeatures: ['功能J', '功能K', '功能L'], engines: ['llm-gateway'], dataAssets: [], docs: [] },
    { id: 'test-domain-v', name: '测试域V', codePath: 'src/routes/atlas.js', auto: true, keyFeatures: ['功能M', '功能N', '功能O'], engines: ['llm-gateway'], dataAssets: [], docs: [] }];
  writeRegistryFile(auto);
})();
delete require.cache[require.resolve('../src/project-atlas')];
const A2 = require('../src/project-atlas');
A2.createProject({ id: 'proj-test-gov-b', name: '移交对端项目', status: 'planning', domains: ['test-domain-w', 'test-domain-v'] });

const aMove = A2.assignDomain('proj-test-gov-b', 'test-domain-x');
check('运行时项目间域移交成功', aMove.accepted === true, JSON.stringify(aMove));
check('移交即触发 W10 复验且全绿', aMove.verification && aMove.verification.ok === true, JSON.stringify(aMove.verification));
const detailB = A2.getProjectDetail('proj-test-gov-b');
check('移交后目标项目持有 3 域', detailB.domains.length === 3, String(detailB.domains.length));
const detailA = A2.getProjectDetail('proj-test-gov');
check('移交后源项目剩 2 域（P6 合规）', detailA.domains.length === 2, String(detailA.domains.length));

// P6 内聚保护：2 域源项目再移出 1 域将被拒
const aInner = A2.assignDomain('proj-test-gov-b', 'test-domain-y');
check('移交致源项目域数 <2 被拒（P6 内聚保护）', aInner.accepted === false && /P6/.test(aInner.reason), JSON.stringify(aInner));

// W10 守门能力：外部篡改制造 P6 破坏（绕过 service 前置校验），验证复验立即暴露
(function sabotage() {
  const auto = readRegistryFile();
  const gov = (auto.projects || []).find(p => p.id === 'proj-test-gov');
  gov.domains = ['test-domain-y']; // 篡改：单域项目
  writeRegistryFile(auto);
})();
delete require.cache[require.resolve('../src/project-atlas')];
const S = require('../src/project-atlas');
const vSab = S.verifyAtlas();
check('外部篡改 P6 被 W10 立即暴露（无破窗守门）', vSab.ok === false, 'expected FAIL but got ok');
check('W10 失败项指名项目建模', vSab.checks.some(c => !c.ok && /W10 项目建模/.test(c.name)));

// 自愈：恢复正常归属
(function heal() {
  const auto = readRegistryFile();
  const gov = (auto.projects || []).find(p => p.id === 'proj-test-gov');
  gov.domains = ['test-domain-y', 'test-domain-z'];
  writeRegistryFile(auto);
})();
delete require.cache[require.resolve('../src/project-atlas')];
const B = require('../src/project-atlas');
const vHealed = B.verifyAtlas();
check('自愈后 W10 重新全绿', vHealed.ok === true, `failed=${vHealed.summary.failed}`);

// ---------- ⑤ 持久化与重启恢复 ----------
console.log('\n[5] 持久化与重启恢复');
const fileReg = readRegistryFile();
const fileProj = (fileReg.projects || []).find(p => p.id === 'proj-test-gov');
check('注册表文件已落盘（projects 键含测试项目）', !!fileProj, REG_FILE);
check('落盘条目带运行时标记（runtime=true + registeredAt）',
  fileProj && fileProj.runtime === true && !!fileProj.registeredAt);

delete require.cache[require.resolve('../src/project-atlas')];
const C = require('../src/project-atlas');
const detail2 = C.getProjectDetail('proj-test-gov');
check('重启恢复：模块重载后运行时项目仍在（含流转后状态）',
  !!detail2 && detail2.runtime === true && detail2.status === 'building', detail2 ? detail2.status : 'null');
const v2res = C.verifyAtlas();
check('重启后 W10 全量复验全绿', v2res.ok === true, `failed: ${v2res.summary.failed}`);

// ---------- ⑥ 移除语义（基线保护 + 孤儿域防护 + 级联移交） ----------
console.log('\n[6] 移除语义（基线保护 + 孤儿域防护 + 级联移交）');
const rmBaseline = C.removeProject('proj-expert-alliance');
check('代码基线项目不可移除', rmBaseline.removed === false, JSON.stringify(rmBaseline));

const rmOrphan = C.removeProject('proj-test-gov');
check('移除将造成孤儿域被拒（先移交）', rmOrphan.removed === false && /孤儿/.test(rmOrphan.reason), JSON.stringify(rmOrphan));

// 级联移除：域整体移交承接项目后删除（项目解散/合并场景）
const rmCascade = C.removeProject('proj-test-gov', { reassignTo: 'proj-test-gov-b' });
check('级联移除被接受（域整体移交承接方）', rmCascade.removed === true, JSON.stringify(rmCascade));
check('级联移交 2 域', rmCascade.movedDomains === 2, String(rmCascade.movedDomains));
check('移除即触发 W10 复验且全绿', rmCascade.verification && rmCascade.verification.ok === true, JSON.stringify(rmCascade.verification));
check('移除后项目查询返回 null', C.getProjectDetail('proj-test-gov') === null);
const receiverDetail = C.getProjectDetail('proj-test-gov-b');
check('承接项目现持有全部 5 域', receiverDetail.domains.length === 5, String(receiverDetail.domains.length));
const fileAfterRm = readRegistryFile();
check('移除后注册表文件不再含测试项目',
  !(fileAfterRm.projects || []).some(p => p.id === 'proj-test-gov'));

// 级联移除：承接方不存在被拒
const rmBadReceiver = C.removeProject('proj-test-gov-b', { reassignTo: 'proj-ghost' });
check('承接项目不存在被拒', rmBadReceiver.removed === false, JSON.stringify(rmBadReceiver));

// ---------- ⑦ 全链路回归 ----------
console.log('\n[7] 全链路回归（"一切皆是项目"无破窗）');
(function cleanup() {
  const auto = readRegistryFile();
  auto.projects = (auto.projects || []).filter(p => !FIXTURE_PROJECTS.includes(p.id));
  auto.domains = (auto.domains || []).filter(d => !FIXTURE_DOMAIN_IDS.includes(d.id));
  writeRegistryFile(auto);
})();
delete require.cache[require.resolve('../src/project-atlas')];
const D = require('../src/project-atlas');

const vFinal = D.verifyAtlas();
check('最终无破窗验证整体通过', vFinal.ok === true, `failed: ${vFinal.summary.failed}`);
check('W10 检查族全部通过',
  vFinal.checks.filter(c => c.name.includes('W10')).every(c => c.ok === true));
const listFinal = D.getProjects();
check('最终项目清单恢复基线（8 项目）',
  listFinal.stats.total === 8 && listFinal.stats.runtimeRegistered === 0,
  `total=${listFinal.stats.total} runtime=${listFinal.stats.runtimeRegistered}`);
check('全部业务域与模块归属项目（32 治理单元：28 域含 studio 与 mcp + 4 模块）',
  listFinal.stats.totalDomains === 32, String(listFinal.stats.totalDomains));
const domCheck = D.getDomainDetail('expert-alliance');
check('域详情回查所属项目', domCheck.ownedBy.length === 1 && domCheck.ownedBy[0].id === 'proj-expert-alliance');
const modCheck = D.getProjectDetail('proj-ai-platform');
check('模块亦归属项目（mod-task 在平台生态项目）',
  modCheck.domains.some(d => d.id === 'mod-task'));

console.log('\n===== "一切皆是项目"项目治理测试汇总 =====');
console.log(`通过: ${passed} 项，失败: ${failed} 项`);
console.log(`验证项总数: ${vFinal.summary.total}（W1-W10 全族）`);
process.exit(failed > 0 ? 1 : 0);

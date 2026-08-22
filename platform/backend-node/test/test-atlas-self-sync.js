'use strict';

/**
 * 图谱自管理（self-sync）测试——自己管理自己
 * ------------------------------------------------------------------
 * 验证闭环：
 *   [1] 幂等性：干净状态同步无变更
 *   [2] 自发现：新增 data/docs 文件 → dryRun 发现 → 执行登记 → 图谱节点增长
 *   [3] 无破窗维持：自动登记后 W1-W8 依然全绿
 *   [4] 自愈清理：资产删除 → 下次同步自动移除登记（无幽灵）
 *   [5] 自愈验证：selfHealVerify 修复后 ok
 *   [6] 域级规则：diffRegistry 纯函数（stub 注入验证自动域构造）
 * 运行：node test/test-atlas-self-sync.js
 */

const assert = require('assert');
const fs = require('fs');
const path = require('path');

const atlas = require('../src/project-atlas');
const { diffRegistry, buildAutoDomain, buildAutoContainerDomain, pruneAutoRegistry } = require('../src/project-atlas/domain/self-sync-rules');

let passed = 0, failed = 0;
function check(name, cond, detail) {
  if (cond) { passed++; console.log(`  [PASS] ${name}`); }
  else { failed++; console.log(`  [FAIL] ${name}${detail !== undefined ? ' -> ' + String(detail) : ''}`); }
}

const ROOT = path.join(__dirname, '..');
const DATA_DIR = path.join(ROOT, 'data');
const DOCS_DIR = path.join(ROOT, '..', '..', 'docs');

(async () => {
  console.log('[1] 幂等性与基线');
  const v0 = atlas.verifyAtlas();
  check('基线无破窗全绿（W1-W8）', v0.ok, `failed: ${v0.summary.failed}`);
  const nodesBefore = atlas.getAtlas().nodes.length;

  const sync0 = atlas.selfSync({ dryRun: false });
  check('干净状态同步：无变更（幂等）', sync0.changed === false && sync0.applied === false);
  check('同步后图谱节点数不变', atlas.getAtlas().nodes.length === nodesBefore);

  console.log('\n[2] 自发现：模拟新资产（自适应自开发产出）');
  // 模拟自动开发引擎产出新数据文件 + 新文档
  const newDataFile = path.join(DATA_DIR, 'test_selfsync_asset.json');
  const newDocFile = path.join(DOCS_DIR, 'test-selfsync-doc.md');
  fs.writeFileSync(newDataFile, JSON.stringify({ producedBy: 'auto-dev', at: new Date().toISOString() }), 'utf8');
  fs.writeFileSync(newDocFile, '# 自开发测试文档\nself-sync 应自动发现并登记本文件。', 'utf8');

  // dryRun 预览：发现 2 项
  const pending = atlas.discoverPending();
  check('dryRun 发现新 data 文件', pending.pending.dataFiles.includes('test_selfsync_asset.json'), JSON.stringify(pending.pending.dataFiles));
  check('dryRun 发现新 doc 文件', pending.pending.docs.includes('docs/test-selfsync-doc.md'), JSON.stringify(pending.pending.docs));
  check('dryRun 后图谱未变（不落盘）', atlas.getAtlas().nodes.length === nodesBefore);

  // 执行登记
  const sync1 = atlas.selfSync({ dryRun: false });
  check('执行同步：报告已变更', sync1.changed === true && sync1.applied === true);
  check('图谱节点增长（+1 data +1 doc）', atlas.getAtlas().nodes.length === nodesBefore + 2, `${nodesBefore} -> ${atlas.getAtlas().nodes.length}`);

  console.log('\n[3] 无破窗维持（自动登记后）');
  const v1 = atlas.verifyAtlas();
  check('自动登记后 W1-W8 仍全绿', v1.ok, v1.checks.filter(c => !c.ok).map(c => c.name).join(';'));
  check('容器域 atlas-auto 可查详情', atlas.getDomainDetail('atlas-auto') !== null);
  const container = atlas.getDomainDetail('atlas-auto');
  check('容器域挂载新资产', container.dataAssets.some(x => x.file === 'test_selfsync_asset.json') && container.docs.some(d => d.path === 'docs/test-selfsync-doc.md'));
  check('容器域关联自管理引擎（连通无孤岛）', container.engines.some(e => e.id === 'project-atlas'));

  console.log('\n[4] 自愈清理：资产删除');
  fs.unlinkSync(newDataFile);
  fs.unlinkSync(newDocFile);
  const sync2 = atlas.selfSync({ dryRun: false });
  check('删除后同步：清理失效登记', sync2.pruned.dataAssets >= 1 || sync2.pruned.docs >= 1, JSON.stringify(sync2.pruned));
  check('图谱节点回落（无幽灵资产）', atlas.getAtlas().nodes.length === nodesBefore, String(atlas.getAtlas().nodes.length));
  const v2 = atlas.verifyAtlas();
  check('清理后无破窗仍全绿', v2.ok);

  console.log('\n[5] 自愈验证（selfHealVerify）');
  // 再造一个缺口 → selfHealVerify 应自动修复
  fs.writeFileSync(newDataFile, '{}', 'utf8');
  const heal = atlas.selfHealVerify();
  check('自愈验证：发现缺口并自动修复', heal.ok === true && heal.healed === true);
  fs.unlinkSync(newDataFile);
  atlas.selfSync({ dryRun: false }); // 清理测试残留
  const vFinal = atlas.verifyAtlas();
  check('测试残留清理后全绿', vFinal.ok);

  console.log('\n[6] 域级规则纯函数（stub 验证）');
  const scanned = {
    routeDomains: [{ id: 'new-domain', name: '新业务域', codePath: 'src/routes/new-domain.js' }],
    dataFiles: ['a.json', 'b.json'],
    docs: ['docs/x.md']
  };
  const view = { domains: [{ id: 'existing' }], dataAssets: [{ file: 'a.json' }], docs: [] };
  const diff = diffRegistry(scanned, view);
  check('diff：未登记域被发现', diff.pendingDomains.length === 1 && diff.pendingDomains[0].id === 'new-domain');
  check('diff：未登记 data 文件被发现（b.json）', diff.pendingDataFiles.length === 1 && diff.pendingDataFiles[0] === 'b.json');
  check('diff：已登记 data 不重复发现（a.json 排除）', !diff.pendingDataFiles.includes('a.json'));
  check('diff：未登记 doc 被发现', diff.pendingDocs.length === 1);

  const autoDomain = buildAutoDomain(scanned.routeDomains[0]);
  check('自动域：3 条 keyFeatures（W6 内聚达标）', autoDomain.keyFeatures.length >= 3);
  check('自动域：engines 挂自管理引擎（W5/W8 连通）', autoDomain.engines.includes('project-atlas'));
  check('自动域：auto 标记（W1/W6 豁免 ghost）', autoDomain.auto === true);

  const container2 = buildAutoContainerDomain(['b.json'], ['docs/x.md']);
  check('容器域：聚合 data+docs', container2.dataAssets.length === 1 && container2.docs.length === 1);

  const pruned = pruneAutoRegistry(
    { domains: [buildAutoContainerDomain(['gone.json'], [])], dataAssets: [{ file: 'gone.json', domain: 'atlas-auto' }], docs: [] },
    { domains: new Set(), dataFiles: new Set(), docs: new Set() }
  );
  check('prune：资产消失后容器域整体移除（无空壳）', pruned.pruned.domains.length === 0 && pruned.removed.dataAssets === 1);

  console.log('\n===== 图谱自管理测试汇总 =====');
  console.log(`通过: ${passed} 项，失败: ${failed} 项`);
  process.exit(failed > 0 ? 1 : 0);
})().catch(e => { console.error('测试异常:', e); process.exit(1); });

'use strict';

/**
 * 图谱自管理用例（application 层 · mixin 用例族）
 * ------------------------------------------------------------------
 * 自己管理自己：扫描 → 差量 → 自动登记 → 图谱重建 → 复验，全流程无人值守。
 * 自适应自开发闭环：自动开发引擎产出新文件 → self-sync 自动图谱化 →
 * 无破窗验证保持全绿。资产删除 → 下次同步自动清理登记（自愈）。
 *
 * 依赖注入（可测性）：scanner / registryIO / rebuild / getRegisteredView
 * 由 index.js 装配时注入；测试可注入 stub。
 */

const { diffRegistry, buildAutoDomain, buildAutoContainerDomain, pruneAutoRegistry } = require('../domain/self-sync-rules');

function createSelfSyncService({ scanner, registryIO, rebuild, getRegisteredView }) {
  /** 预览：发现待登记/待清理项（不落盘） */
  function discoverPending() {
    const scanned = scanner.scanAll();
    const view = getRegisteredView();
    const { pendingDomains, pendingDataFiles, pendingDocs, scannedSet } = diffRegistry(scanned, view);

    const autoRegistry = registryIO.read();
    const prune = pruneAutoRegistry(autoRegistry, scannedSet, baselineDomainIds(view));

    return {
      scannedAt: scanned.scannedAt,
      pending: {
        domains: pendingDomains.map(d => ({ id: d.id, name: d.name, codePath: d.codePath })),
        dataFiles: pendingDataFiles,
        docs: pendingDocs,
        autoDevArtifacts: scanned.autoDevArtifacts
      },
      stale: prune.removed,
      summary: {
        toRegister: pendingDomains.length + pendingDataFiles.length + pendingDocs.length,
        toPrune: prune.removed.domains + prune.removed.dataAssets + prune.removed.docs
      }
    };
  }

  /** 执行自管理同步（幂等）：登记新资产 + 清理失效登记 + 重建图谱 + 复验 */
  function selfSync(options = {}) {
    const dryRun = options.dryRun !== false;
    const scanned = scanner.scanAll();
    const view = getRegisteredView();
    const { pendingDomains, pendingDataFiles, pendingDocs, scannedSet } = diffRegistry(scanned, view);

    const autoRegistry = registryIO.read();
    const prune = pruneAutoRegistry(autoRegistry, scannedSet, baselineDomainIds(view));

    // 构造新的 auto 层：净化后 + 新发现项
    const nextDomains = [...prune.pruned.domains];
    const nextDataAssets = [...prune.pruned.dataAssets];
    const nextDocs = [...prune.pruned.docs];

    const registered = new Set([
      ...view.domains.map(d => d.id),
      ...nextDomains.map(d => d.id)
    ]);
    for (const rd of pendingDomains) {
      if (!registered.has(rd.id)) nextDomains.push(buildAutoDomain(rd));
    }

    // 容器域：聚合未归属 data/docs（新发现 + 既有容器内容已在 nextDomains 中）
    const hasNewAssets = pendingDataFiles.length > 0 || pendingDocs.length > 0;
    let container = nextDomains.find(d => d.id === 'atlas-auto');
    if (hasNewAssets) {
      if (!container) {
        container = buildAutoContainerDomain(pendingDataFiles, pendingDocs);
        nextDomains.push(container);
      } else {
        container.dataAssets = [...new Set([...(container.dataAssets || []), ...pendingDataFiles])];
        container.docs = [...new Set([...(container.docs || []), ...pendingDocs])];
      }
      for (const f of pendingDataFiles) nextDataAssets.push({ file: f, domain: 'atlas-auto', desc: '自动发现（self-sync）' });
      for (const f of pendingDocs) nextDocs.push({ file: f, domain: 'atlas-auto', desc: '自动发现（self-sync）' });
    }

    const changed = dryRun === false && (
      pendingDomains.length > 0 || hasNewAssets || prune.changed
    );

    const report = {
      dryRun,
      discovered: {
        domains: pendingDomains.map(d => ({ id: d.id, name: d.name, codePath: d.codePath })),
        dataFiles: pendingDataFiles,
        docs: pendingDocs
      },
      pruned: prune.removed,
      autoDevArtifacts: scanned.autoDevArtifacts,
      changed,
      syncedAt: new Date().toISOString()
    };

    if (dryRun) return { ...report, applied: false };

    if (changed) {
      // flows/projects 键原样保留（运行时注册的业务流程与项目由各自服务管理，
      // self-sync 只治理 domains/dataAssets/docs 三类扫描资产）
      registryIO.write({
        domains: nextDomains, dataAssets: nextDataAssets, docs: nextDocs,
        flows: autoRegistry.flows || [], projects: autoRegistry.projects || []
      });
      rebuild(); // 图谱重建（合并新 auto 层）
      report.applied = true;
    } else {
      report.applied = false;
    }

    return report;
  }

  /** 自愈验证：先尝试 self-sync 修复，再复验（给启动钩子/巡检用） */
  function selfHealVerify() {
    const before = getRegisteredView().verify();
    if (before.ok) return { ok: true, healed: false, verification: before };

    const sync = selfSync({ dryRun: false });
    const after = getRegisteredView().verify();
    return {
      ok: after.ok,
      healed: after.ok && !before.ok,
      sync,
      verification: after,
      beforeFailed: before.summary.failed
    };
  }

  return { discoverPending, selfSync, selfHealVerify };
}

/** 代码基线域 id 集合（非 auto 登记的合并视图域 = 基线域，用于毕业退役判定） */
function baselineDomainIds(view) {
  return new Set((view.domains || []).filter(d => d.auto !== true).map(d => d.id));
}

module.exports = { createSelfSyncService };

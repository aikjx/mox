'use strict';

/**
 * 自登记差量规则（domain 层 · 纯函数零 IO）
 * ------------------------------------------------------------------
 * 输入扫描结果与已登记视图，输出待登记/待清理清单。
 * 规则：
 *   R1 未登记路由域 → 自动登记为 auto 域（engines 挂 project-atlas 自管理引擎）
 *   R2 未登记 data 文件 → 归属 atlas-auto 容器域（无法推断业务归属）
 *   R3 未登记 docs    → 归属 atlas-auto 容器域
 *   R4 auto 层中已消失的文件/域 → 清理（自愈：资产删除后图谱不留幽灵）
 *   R5 auto-dev 制品 → 归属 auto-dev 域的 dataAssets 维度（制品项目化）
 *   R6 auto 层中已毕业到代码基线的域 → 退役（资产正式登记后临时登记自动撤销）
 */

/** 差量计算：扫描结果 vs 当前合并登记视图 */
function diffRegistry(scanned, registeredView) {
  const { domains = [], dataAssets = [], docs = [] } = registeredView;
  const regDomainIds = new Set(domains.map(d => d.id));
  const regDataFiles = new Set(dataAssets.map(x => x.file));
  const regDocFiles = new Set(docs.map(x => x.file));

  const pendingDomains = scanned.routeDomains.filter(r => !regDomainIds.has(r.id));
  const pendingDataFiles = scanned.dataFiles.filter(f => !regDataFiles.has(f));
  const pendingDocs = scanned.docs.filter(f => !regDocFiles.has(f));

  // 自愈清理：auto 层登记过但文件已消失的项
  const scannedSet = {
    domains: new Set(scanned.routeDomains.map(r => r.id)),
    dataFiles: new Set(scanned.dataFiles),
    docs: new Set(scanned.docs)
  };

  return { pendingDomains, pendingDataFiles, pendingDocs, scannedSet };
}

/** 构造自动域登记条目（满足 W3/W5/W6：codePath 真实、引擎有效、内聚达标） */
function buildAutoDomain(routeDomain) {
  return {
    id: routeDomain.id,
    name: routeDomain.name,
    codePath: routeDomain.codePath,
    keyFeatures: [
      '自动发现并登记（图谱自管理 self-sync）',
      `路由域 ${routeDomain.id} 由扫描器发现，零人工登记`,
      '待业务 owner 补充关键功能描述后可覆盖此条目'
    ],
    engines: ['project-atlas'],
    dataAssets: [],
    docs: [],
    auto: true,
    autoRegisteredAt: new Date().toISOString()
  };
}

/** atlas-auto 容器域：承载无法推断归属的 data/docs 资产，保证 W8 连通 */
function buildAutoContainerDomain(pendingDataFiles, pendingDocs) {
  return {
    id: 'atlas-auto',
    name: '图谱自管理容器',
    codePath: 'src/project-atlas/infrastructure/atlas-scanner.js',
    keyFeatures: [
      '自动发现的未归属资产容器（self-sync）',
      '新 data 文件与新 docs 文档自动挂载',
      '资产删除后自动清理登记（自愈）'
    ],
    engines: ['project-atlas'],
    dataAssets: pendingDataFiles,
    docs: pendingDocs,
    auto: true,
    autoRegisteredAt: new Date().toISOString()
  };
}

/** auto 层自愈：清理登记后已消失的资产 + 已毕业到代码基线的域（输入 auto 层，输出净化后的 auto 层）
 *  baselineDomainIds：代码基线域 id 集合——资产正式登记进基线后，auto 层临时登记自动退役（毕业语义） */
function pruneAutoRegistry(autoRegistry, scannedSet, baselineDomainIds = new Set()) {
  // 变更检测只覆盖 self-sync 治理的三类资产（domains/dataAssets/docs）；
  // flows 键由 flow-registration-service 管理，不参与本差量（否则永远误报 changed）
  const before = JSON.stringify({
    domains: autoRegistry.domains || [],
    dataAssets: autoRegistry.dataAssets || [],
    docs: autoRegistry.docs || []
  });
  const pruned = {
    domains: (autoRegistry.domains || []).filter(d =>
      (scannedSet.domains.has(d.id) && !baselineDomainIds.has(d.id)) || d.id === 'atlas-auto'),
    dataAssets: (autoRegistry.dataAssets || []).filter(x => scannedSet.dataFiles.has(x.file)),
    docs: (autoRegistry.docs || []).filter(x => scannedSet.docs.has(x.file))
  };
  // atlas-auto 容器域的 dataAssets/docs 同步净化
  const container = pruned.domains.find(d => d.id === 'atlas-auto');
  if (container) {
    container.dataAssets = (container.dataAssets || []).filter(f => scannedSet.dataFiles.has(f));
    container.docs = (container.docs || []).filter(f => scannedSet.docs.has(f));
    // 容器空了就整体移除（无孤岛且无空壳域）
    if (container.dataAssets.length === 0 && container.docs.length === 0) {
      pruned.domains = pruned.domains.filter(d => d.id !== 'atlas-auto');
    }
  }
  const removed = {
    domains: (autoRegistry.domains || []).length - pruned.domains.length,
    dataAssets: (autoRegistry.dataAssets || []).length - pruned.dataAssets.length,
    docs: (autoRegistry.docs || []).length - pruned.docs.length
  };
  return { pruned, removed, changed: JSON.stringify(pruned) !== before };
}

module.exports = { diffRegistry, buildAutoDomain, buildAutoContainerDomain, pruneAutoRegistry };

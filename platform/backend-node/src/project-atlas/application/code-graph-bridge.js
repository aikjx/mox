'use strict';

/**
 * 项目全息图谱 · 代码图谱桥接（application 层 · 用例编排）
 * ------------------------------------------------------------------
 * "本地代码工程维度" 核心落地：图谱节点 ↔ 本地代码实体双向映射。
 *   scanAndBind     扫描全部图谱单元 codePath → 代码实体 → 绑定落盘（幂等）
 *   verifyConsistency  一致性校验（文件存在 / 实体存活 / 幽灵绑定检测）
 *   traceCode       图谱节点 → 代码实体全景（file:line 定位）
 *   suggestChanges  图谱变更 → 代码变更建议（影响面 × 代码实体交叉）
 * 绑定真相源：code_graph_bindings.json（unitId 一记录，幂等替换）。
 *
 * 依赖注入（可测性）：extractor / bindingsIO / getUnits / resolvePath / impact
 */

function createCodeGraphBridge({ extractor, bindingsIO, getUnits, resolvePath, impact }) {

  /** 全量扫描绑定：图谱单元（域/模块/引擎/算法）→ 代码实体 */
  function scanAndBind(options = {}) {
    const units = (typeof getUnits === 'function' ? getUnits() : [])
      .filter(u => u && u.codePath);
    const results = [];
    let bound = 0, missing = 0;

    units.forEach(u => {
      const absPath = resolvePath(u.codePath);
      if (!absPath) { missing++; results.push({ unitId: u.id, kind: u.kind, codePath: u.codePath, ok: false, error: '路径不可解析' }); return; }
      const scan = extractor.scanPath(absPath);
      if (!scan.exists) {
        missing++;
        results.push({ unitId: u.id, kind: u.kind, codePath: u.codePath, ok: false, error: '文件/目录不存在' });
        return;
      }
      const binding = {
        unitId: u.id, kind: u.kind, name: u.name, codePath: u.codePath,
        files: scan.totals.files, entityCount: scan.totals.entities,
        functions: scan.totals.functions, classes: scan.totals.classes,
        exports: scan.totals.exports, routes: scan.totals.routes,
        topEntities: collectTopEntities(scan.files),
        scannedAt: scan.scannedAt
      };
      upsertBinding(binding);
      bound++;
      results.push({ unitId: u.id, kind: u.kind, codePath: u.codePath, ok: true, entities: scan.totals.entities, files: scan.totals.files });
    });

    return {
      ok: true, units: units.length, bound, missing,
      totalEntities: results.reduce((s, r) => s + (r.entities || 0), 0),
      results, dryRun: options.dryRun === true,
      scannedAt: new Date().toISOString()
    };
  }

  /** 绑定记录 upsert（unitId 幂等键：一单元一记录）
   *  先剔除全部同 unitId 旧记录（含存储层历史重复）再写入，
   *  既保证契约，又对存量脏数据自愈归一。 */
  function upsertBinding(binding) {
    const list = bindingsIO.read().filter(b => b.unitId !== binding.unitId);
    list.push(binding);
    bindingsIO.write(list);
    return binding;
  }

  /** 代码实体全景收集（每文件 top 函数/类/路由，总量护栏） */
  function collectTopEntities(files) {
    const out = [];
    for (const f of files.slice(0, 30)) {
      f.functions.slice(0, 8).forEach(fn => out.push({ name: fn.name, kind: fn.kind, location: `${f.file}:${fn.line}` }));
      f.classes.slice(0, 3).forEach(c => out.push({ name: c.name, kind: 'class', location: `${f.file}:${c.line}` }));
      f.routes.slice(0, 5).forEach(r => out.push({ name: `${r.method} ${r.path}`, kind: 'route', location: `${f.file}:${r.line}` }));
    }
    return out.slice(0, 60);
  }

  /** 绑定查询（?unitId= / ?kind= 过滤） */
  function getBindings(filter = {}) {
    let list = bindingsIO.read();
    if (filter.unitId) list = list.filter(b => b.unitId === filter.unitId);
    if (filter.kind) list = list.filter(b => b.kind === filter.kind);
    return list;
  }

  /** 单节点代码溯源：绑定 → 实体定位全景 */
  function traceCode(unitId) {
    const binding = bindingsIO.read().find(b => b.unitId === unitId);
    if (!binding) return null;
    const scan = extractor.scanPath(resolvePath(binding.codePath) || binding.codePath);
    return {
      unitId: binding.unitId, kind: binding.kind, name: binding.name,
      codePath: binding.codePath,
      exists: scan.exists,
      totals: scan.totals,
      files: scan.files.map(f => ({
        file: f.file, language: f.language, entities: f.total,
        functions: f.functions, classes: f.classes, routes: f.routes
      })),
      lastScannedAt: binding.scannedAt
    };
  }

  /**
   * 一致性校验：绑定记录 ↔ 磁盘 ↔ 图谱三方对账
   * 检查项：① 文件/目录存在 ② 实体存活（复扫 entityCount>0 或等量）
   *         ③ 绑定单元仍存在于图谱（幽灵绑定检测） ④ 实体数漂移（stale 标记）
   */
  function verifyConsistency() {
    const bindings = bindingsIO.read();
    const unitIds = new Set((typeof getUnits === 'function' ? getUnits() : []).map(u => u.id));
    const checks = [];
    let okCount = 0;

    bindings.forEach(b => {
      const issues = [];
      if (!unitIds.has(b.unitId)) issues.push('幽灵绑定：图谱单元已不存在');
      const absPath = resolvePath(b.codePath);
      if (!absPath || !existsPath(absPath)) issues.push('代码路径不存在');
      else {
        const scan = extractor.scanPath(absPath);
        if (!scan.exists) issues.push('复扫失败');
        else if (scan.totals.entities !== b.entityCount) issues.push(`实体漂移：绑定 ${b.entityCount} → 当前 ${scan.totals.entities}`);
      }
      const ok = issues.length === 0;
      if (ok) okCount++;
      checks.push({ unitId: b.unitId, kind: b.kind, codePath: b.codePath, ok, issues });
    });

    return {
      ok: okCount === bindings.length,
      total: bindings.length, passed: okCount, failed: bindings.length - okCount,
      checks, verifiedAt: new Date().toISOString()
    };
  }

  /** 路径存在性（文件或目录） */
  function existsPath(absPath) {
    try { return require('fs').existsSync(absPath); } catch (e) { return false; }
  }

  /**
   * 代码变更建议：图谱节点变更 → 影响面节点 × 各自代码绑定 → 建议清单
   * 输出 [{ nodeId, kind, name, codePath, codeEntities:[{name,location}] }]
   */
  function suggestChanges(nodeId) {
    const impacted = typeof impact === 'function' ? impact(nodeId) : { seed: nodeId, reachableNodes: [] };
    const targets = [impacted.seed, ...(impacted.reachableNodes || [])];
    const bindingByUnit = new Map(bindingsIO.read().map(b => [b.unitId, b]));
    const suggestions = [];
    const unitIndex = new Map((typeof getUnits === 'function' ? getUnits() : []).map(u => [u.id, u]));

    targets.forEach(id => {
      const b = bindingByUnit.get(id);
      if (!b) return;
      const u = unitIndex.get(id) || {};
      suggestions.push({
        nodeId: id, kind: b.kind, name: u.name || b.name,
        codePath: b.codePath,
        action: b.kind === 'algorithm' ? '算法实现回归验证'
          : b.kind === 'engine' ? '引擎契约探活回归'
          : b.kind === 'module' || b.kind === 'domain' ? '模块功能与测试范围复核'
          : '关联代码复核',
        codeEntities: b.topEntities || []
      });
    });

    return {
      seed: nodeId, impactedCount: (impacted.reachableNodes || []).length,
      codeUnits: suggestions.length, suggestions,
      generatedAt: new Date().toISOString()
    };
  }

  /** 桥接统计（治理看板数据源）
   *  覆盖率按"单元视角"统计：域/引擎同 id 单元（如 kb）共享绑定记录属合法形态，
   *  bound = 已有绑定的单元数（而非绑定条目数），保证 coverage 真实反映绑定覆盖。 */
  function getStats() {
    const bindings = bindingsIO.read();
    const units = (typeof getUnits === 'function' ? getUnits() : []).filter(u => u && u.codePath);
    const boundUnitIds = new Set(bindings.map(b => b.unitId));
    const coveredUnits = units.filter(u => boundUnitIds.has(u.id)).length;
    const consistency = verifyConsistency();
    return {
      units: units.length, bound: coveredUnits,
      coverage: units.length === 0 ? 0 : coveredUnits / units.length,
      codeEntities: bindings.reduce((s, b) => s + b.entityCount, 0),
      functions: bindings.reduce((s, b) => s + b.functions, 0),
      classes: bindings.reduce((s, b) => s + b.classes, 0),
      routes: bindings.reduce((s, b) => s + b.routes, 0),
      consistent: consistency.passed, inconsistent: consistency.failed,
      lastScanAt: bindings.length > 0
        ? bindings.map(b => b.scannedAt).sort().reverse()[0] : null
    };
  }

  return { scanAndBind, getBindings, traceCode, verifyConsistency, suggestChanges, getStats };
}

module.exports = { createCodeGraphBridge };

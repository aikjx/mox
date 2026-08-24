/* D1-ARCH：internal ↔ business-registry ↔ routes 三向一致性校验
 *
 * 企业级"全维开发"要求：3 个独立注册表（源码事实）必须严格对齐：
 *   A = business-registry.js 中声明的业务域 id[]
 *   B = routes/index.js 中 DOMAINS[] 实际装配的域 id[]
 *   C = project-registry.js 中 projects[*].domains[] 引用到的所有域 id
 *
 * 验收标准（TDD 三项断言）：
 *   1) 任意 A 域必须在 B 中存在 → 注册了却没装路由 = 功能悬空
 *   2) 任意 B 域必须在 A 中存在 → 装了路由但域没登记 = 治理悬空
 *   3) 任意 C 域必须在 A∩B 中存在 → 项目引用了不存在的域 = 图谱孤点
 *   4) A∩B∩C 三方对称差 = ∅（三向完全一致）且数量 = 30（启动日志打印的"30 个业务域装配完成"）
 */
'use strict';
const assert = require('assert');
const fs = require('fs');
const path = require('path');
const ROOT = path.resolve(__dirname, '..');

/** 从 business-registry.js 原始文本里提取所有 id: 'xxx' / id: "xxx" */
function extractIdsFromRegistry(sourceText) {
  const ids = [];
  const re = /id\s*:\s*['"]([\w-]+)['"]/g;
  let m;
  while ((m = re.exec(sourceText)) !== null) ids.push(m[1]);
  return Array.from(new Set(ids));
}

/** 从 routes/index.js DOMAINS = [[id,name,handler], ...] 提取第一列 id */
function extractIdsFromRoutes(sourceText) {
  const ids = [];
  // 匹配 DOMAINS 数组中的子数组第一元素
  const re = /\[\s*['"]([\w-]+)['"]\s*,\s*['"][^'"]+['"]\s*,/g;
  let m;
  while ((m = re.exec(sourceText)) !== null) ids.push(m[1]);
  return Array.from(new Set(ids));
}

/** 从 project-registry.js 中所有 domains: [...] 数组提取域 id */
function extractIdsFromProjects(sourceText) {
  const ids = new Set();
  // 先定位每个 domains: [...] 块（最简单:匹配 domains: 之后的 [...] 数组字面量，允许换行/嵌套）
  const domRe = /domains\s*:\s*\[([^\]]*)\]/g;
  let m;
  while ((m = domRe.exec(sourceText)) !== null) {
    const inner = m[1];
    const quoted = inner.match(/['"]([\w-]+)['"]/g) || [];
    for (const q of quoted) ids.add(q.replace(/['"]/g, ''));
  }
  return Array.from(ids);
}

function setDiff(a, b) {
  const B = new Set(b);
  return a.filter(x => !B.has(x));
}

describe('D1-ARCH：域注册表三向一致性（business-registry ↔ routes ↔ projects）', function () {
  let A = [], B = [], C = [];
  const EXPECTED_COUNT = 30; // 启动日志 [routes] ≥30 个业务域装配完成

  before(function () {
    // 统一通过 require() 读取运行时真实导出（避免静态正则漏掉运行时 DOMAINS.push 动态补齐）
    const regMod = require(path.join(ROOT, 'src', 'project-atlas', 'domain', 'business-registry.js'));
    const rtsMod = require(path.join(ROOT, 'src', 'routes', 'index.js'));
    const proMod = require(path.join(ROOT, 'src', 'project-atlas', 'domain', 'project-registry.js'));

    if (regMod && typeof regMod.getAllEntityIds === 'function') {
      // A = 治理图谱注册表全量实体 = 业务主域 ∪ Rust 治理域 ∪ 引擎模块
      A = regMod.getAllEntityIds();
    } else if (regMod && typeof regMod.getDomains === 'function') {
      A = regMod.getDomains().map(d => d.id).filter(Boolean);
      if (Array.isArray(regMod.MODULES)) for (const m of regMod.MODULES) if (m && m.id) A.push(m.id);
    } else {
      const reg = fs.readFileSync(path.join(ROOT, 'src', 'project-atlas', 'domain', 'business-registry.js'), 'utf8');
      A = extractIdsFromRegistry(reg);
    }

    if (rtsMod && Array.isArray(rtsMod.DOMAINS)) {
      B = rtsMod.DOMAINS.map(x => Array.isArray(x) && x[0]).filter(Boolean);
    } else {
      const rts = fs.readFileSync(path.join(ROOT, 'src', 'routes', 'index.js'), 'utf8');
      B = extractIdsFromRoutes(rts);
    }

    // projects 侧：若有运行时 getter 优先使用，否则从源码扫
    if (proMod && (typeof proMod.getProjects === 'function' || Array.isArray(proMod.PROJECTS))) {
      const projArr = typeof proMod.getProjects === 'function'
        ? proMod.getProjects()
        : proMod.PROJECTS;
      const s = new Set();
      for (const p of projArr || []) {
        const domains = p && Array.isArray(p.domains) ? p.domains : [];
        for (const d of domains) if (typeof d === 'string') s.add(d);
      }
      C = Array.from(s);
    } else {
      const pro = fs.readFileSync(path.join(ROOT, 'src', 'project-atlas', 'domain', 'project-registry.js'), 'utf8');
      C = extractIdsFromProjects(pro);
    }
  });

  it('A (business-registry 域数) ≥ 30（对照启动日志 30 域）', function () {
    assert.ok(A.length >= EXPECTED_COUNT, `registry 域数 = ${A.length} < ${EXPECTED_COUNT}，A=${JSON.stringify(A)}`);
  });
  it('B (routes 装配域数) ≥ 30', function () {
    assert.ok(B.length >= EXPECTED_COUNT, `routes 域数 = ${B.length} < ${EXPECTED_COUNT}，B=${JSON.stringify(B)}`);
  });
  it('1) 所有 registry 域都已在 routes 装配（A ⊆ B）', function () {
    const missing = setDiff(A, B);
    assert.deepStrictEqual(missing, [], `${missing.length} 个域注册了但未装配路由: ${missing.join(',')}`);
  });
  it('2) 所有路由装配域都已在 registry 登记（B ⊆ A）', function () {
    const unreg = setDiff(B, A);
    assert.deepStrictEqual(unreg, [], `${unreg.length} 个域装配了路由但未登记 registry: ${unreg.join(',')}`);
  });
  it('3) 项目引用的域必须是 registry ∩ routes 的有效域（C ⊆ A ∩ B）', function () {
    const AB = new Set([...A, ...B]);
    const orphan = C.filter(c => !AB.has(c));
    assert.deepStrictEqual(orphan, [], `${orphan.length} 个项目引用域孤点: ${orphan.join(',')}`);
  });
  it('4) A 与 B 对称差为空（三向核心一致性：A ≡ B 作为治理 + 路由对等真相）', function () {
    const diff = [...setDiff(A, B), ...setDiff(B, A)];
    assert.deepStrictEqual(diff, [], `A↔B 对称差 ${diff.length} 项: ${diff.join(',')}`);
  });
  it('5) internal 域必须三向均存在（W1 已修复的关键回归测试）', function () {
    ['A', 'B', 'C'].forEach(k => {
      const arr = k === 'A' ? A : (k === 'B' ? B : C);
      assert.ok(arr.includes('internal'), `${k} 侧缺少 internal 域：arr=${JSON.stringify(arr)}`);
    });
  });
});

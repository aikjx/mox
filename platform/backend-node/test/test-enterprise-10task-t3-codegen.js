'use strict';
/**
 * T3: 代码生成质量 & AIS 审计
 *
 * 针对 10 份真实生产 Rust 源文件：
 *  (a) AIS 头标签存在性（/// AIS- 或 //! AIS-，文件头部 10 行内）
 *  (b) 注释密度：注释行 / 总行数 ≥ 15%（所有文件平均值）
 *  (c) 零 TODO / stub / unimplemented! 残留物
 *  (d) Rubric 维度：公共函数文档注释覆盖率 ≥ 80%（可选：如果有 pub fn 则 //!/pub doc）
 *
 * 说明：RED 阶段本测试应 FAIL（因为当前源文件尚无 AIS- 标签），
 *       GREEN 阶段通过为源文件补写 AIS 头标签后即可 PASS。
 */
const assert = require('assert');
const fs = require('fs');
const path = require('path');

const BACKEND_NODE = path.join(__dirname, '..');
const INFOTOPOGRAPH_ROOT = path.resolve(BACKEND_NODE, '..', '..');

const TARGET_FILES = [
  'platform/services/operator-core/src/resource.rs',
  'platform/services/operator-core/src/kernel_ext.rs',
  'platform/services/mox-system/src/orchestrator.rs',
  'platform/services/flow-ai/src/dataflow.rs',
  'platform/services/primiflow-core/examples/generate.rs',
  'platform/services/ai-agent/src/engine/tools.rs',
  'platform/services/ai-agent/src/flow_engine.rs',
  'platform/gateway/runtime/src/handlers/ai_engine.rs',
  'platform/services/hermes-flow-bridge/src/bridge.rs',
  'platform/services/hermes-flow-bridge/src/live.rs',
];

function resolveFile(rel) {
  const candidates = [
    path.join(INFOTOPOGRAPH_ROOT, rel),
    path.join(BACKEND_NODE, rel),
  ];
  for (const c of candidates) if (fs.existsSync(c)) return c;
  return null;
}

/**
 * Rust line classification — returns { total, doc, slashComment, code, blank, braces, pubFn, pubFnDocced, hasAisHeader }
 */
function analyzeRust(filePath) {
  const raw = fs.readFileSync(filePath, 'utf8');
  const lines = raw.split(/\r?\n/);
  let total = lines.length;
  let doc = 0;      // /// or //!  (outer/inner doc comments)
  let slash = 0;    // plain // comments
  let code = 0;
  let blank = 0;
  let braces = 0;
  let pubFn = 0;
  let pubFnDocced = 0;
  let hasAisHeader = false;
  let todoCount = 0;
  let stubCount = 0;

  // Lines for "top 10 header window"
  const headerWindow = lines.slice(0, Math.min(10, lines.length));

  // State: block comments /* ... */ — naive multi-line strip
  let inBlock = false;

  for (let i = 0; i < lines.length; i++) {
    const orig = lines[i];
    let line = orig.trim();

    // Block-comment handling (naive; good enough for density estimates)
    if (inBlock) {
      const close = line.indexOf('*/');
      if (close === -1) continue;
      inBlock = false;
      line = line.slice(close + 2).trim();
      if (!line) continue;
    }
    // Open block comment
    while (true) {
      const open = line.indexOf('/*');
      if (open === -1) break;
      const close = line.indexOf('*/', open + 2);
      if (close === -1) { inBlock = true; line = line.slice(0, open).trim(); break; }
      line = (line.slice(0, open) + ' ' + line.slice(close + 2)).trim();
    }
    if (!line) { blank++; continue; }

    // AIS tag scan in header (documentary comment starting with /// or //!)
    const headerLine = headerWindow[i];
    if (headerLine !== undefined) {
      const h = headerLine.trim();
      if ((h.startsWith('///') || h.startsWith('//!')) && /AIS-/.test(h)) {
        hasAisHeader = true;
      }
    }

    if (/^\{+\s*$/.test(line) || /^\}+\s*$/.test(line) || /^\}[,;]?\s*$/.test(line)) {
      // pure brace / trailing comma brace line
      braces++;
      continue;
    }

    if (line.startsWith('//!') || line.startsWith('///')) {
      doc++;
      // Count TODO/stub/docblock markers inside comments too
      if (/TODO|FIXME|todo!\(\)|unimplemented!\(\)|todo!|stub/i.test(line)) todoCount++;
      continue;
    }
    if (line.startsWith('//')) {
      slash++;
      if (/TODO|FIXME|todo!\(\)|unimplemented!\(\)|stub/i.test(line)) todoCount++;
      continue;
    }

    // Code-ish lines from here on
    code++;
    // Scan for inline unimplemented!/todo! macros even inside code
    if (/unimplemented!\s*\(/.test(line) || /\btodo!\s*\(/.test(line)) stubCount++;
    if (/\bstub\b/i.test(line)) stubCount++;

    // pub fn / pub(crate) fn / pub(super) fn
    const pubFnMatch = line.match(/\bpub(\([^)]*\))?\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)/);
    if (pubFnMatch) {
      pubFn++;
      // Walk previous non-blank, non-brace lines looking for preceding /// or //! doc comment
      let j = i - 1;
      let foundDoc = false;
      while (j >= 0) {
        const prev = lines[j].trim();
        if (!prev) { j--; continue; }
        if (/^\{+\s*$|^\}+\s*$/.test(prev)) { j--; continue; }
        if (prev.startsWith('///') || prev.startsWith('//!')) { foundDoc = true; break; }
        break;
      }
      if (foundDoc) pubFnDocced++;
    }
  }

  // Density denominator: total - blank - pure-braces
  const denom = Math.max(1, total - blank - braces);
  const commentLines = doc + slash;
  const density = commentLines / denom;
  const pubFnDocRate = pubFn === 0 ? 1.0 : pubFnDocced / pubFn;

  return {
    path: filePath,
    total,
    doc,
    slash,
    commentLines,
    density,
    code,
    blank,
    braces,
    denom,
    pubFn,
    pubFnDocced,
    pubFnDocRate,
    hasAisHeader,
    todoCount,
    stubCount,
    clippyMarkers: todoCount + stubCount,
  };
}

describe('T3 代码生成质量 & AIS 审计', function () {
  const reports = [];

  before(function () {
    for (const rel of TARGET_FILES) {
      const full = resolveFile(rel);
      assert.ok(full, `Target file must exist on disk: ${rel} (checked root=${INFOTOPOGRAPH_ROOT})`);
      reports.push({ rel, full, analysis: analyzeRust(full) });
    }
  });

  describe('(a) AIS 头标签 10/10 必须存在', function () {
    for (const rel of TARGET_FILES) {
      it(`${rel} 文件头 10 行内有 /// AIS- 或 //! AIS- 标签`, function () {
        const r = reports.find(x => x.rel === rel);
        assert.ok(r && r.analysis.hasAisHeader, `${rel} 缺少 AIS 头标签`);
      });
    }

    it('AIS 标签总覆盖率 = 10/10', function () {
      const hit = reports.filter(r => r.analysis.hasAisHeader).length;
      assert.strictEqual(hit, TARGET_FILES.length, `AIS 标签覆盖率: ${hit}/${TARGET_FILES.length}，必须 10/10`);
    });
  });

  describe('(b) 注释密度（平均值 ≥ 15%）', function () {
    it('每个文件单独密度 ≥ 8%（地板保证）', function () {
      let failing = reports.filter(r => r.analysis.density < 0.08);
      for (const f of failing) console.log(`    [density-low] ${f.rel}: ${(f.analysis.density * 100).toFixed(2)}% (doc=${f.analysis.doc} slash=${f.analysis.slash} total=${f.analysis.total} denom=${f.analysis.denom})`);
      assert.strictEqual(failing.length, 0, `${failing.length} 个文件注释密度 < 8%`);
    });

    it('10 文件平均注释密度 ≥ 15%', function () {
      const avg = reports.reduce((s, r) => s + r.analysis.density, 0) / reports.length;
      console.log(`    [comment-density-avg] = ${(avg * 100).toFixed(2)}%`);
      assert.ok(avg >= 0.15, `10 文件平均注释密度 ${(avg * 100).toFixed(2)}% < 15%`);
    });

    it('文档注释(doc+inner)占比 ≥ 3% (即说明性注释占比足够)', function () {
      let sumDoc = 0, sumDenom = 0;
      for (const r of reports) { sumDoc += r.analysis.doc; sumDenom += r.analysis.denom; }
      const docRate = sumDoc / Math.max(1, sumDenom);
      console.log(`    [doc-comment-rate] = ${(docRate * 100).toFixed(2)}%`);
      assert.ok(docRate >= 0.03, `文档注释占比 ${(docRate * 100).toFixed(2)}% < 3%`);
    });
  });

  describe('(c) 代码中 0 clippy stub/todo 残留', function () {
    it('全部 10 个文件: TODO/FIXME/todo!/unimplemented!/stub 标记 = 0', function () {
      for (const r of reports) {
        if (r.analysis.clippyMarkers !== 0) {
          console.log(`    [stub-found] ${r.rel}: TODO=${r.analysis.todoCount} stubs=${r.analysis.stubCount}`);
        }
      }
      const offenders = reports.filter(r => r.analysis.clippyMarkers !== 0).length;
      assert.strictEqual(offenders, 0, `${offenders} 个文件含 stub/todo 残留标记`);
    });
  });

  describe('(d) Rubric: 公共函数文档注释覆盖率 / 综合密度', function () {
    it('pub fn 文档注释覆盖率 ≥ 80%（全文件加权）', function () {
      let totalPub = 0, totalDocced = 0;
      for (const r of reports) {
        totalPub += r.analysis.pubFn;
        totalDocced += r.analysis.pubFnDocced;
      }
      const rate = totalPub === 0 ? 1 : totalDocced / totalPub;
      console.log(`    [pub-fn-doc-rate] = ${totalDocced}/${totalPub} = ${(rate * 100).toFixed(2)}%`);
      assert.ok(rate >= 0.8, `公共函数文档覆盖率 ${(rate * 100).toFixed(2)}% < 80%`);
    });

    it('(Rubric 5) 密度 ≥ 15% & AIS 10/10 & pub-fn-doc ≥ 80% 三项同时达标', function () {
      const avg = reports.reduce((s, r) => s + r.analysis.density, 0) / reports.length;
      const aisHit = reports.filter(r => r.analysis.hasAisHeader).length;
      let totalPub = 0, totalDocced = 0;
      for (const r of reports) {
        totalPub += r.analysis.pubFn;
        totalDocced += r.analysis.pubFnDocced;
      }
      const rate = totalPub === 0 ? 1 : totalDocced / totalPub;
      assert.ok(avg >= 0.15, `Rubric fail: avg density ${(avg * 100).toFixed(2)}% < 15%`);
      assert.strictEqual(aisHit, 10, `Rubric fail: AIS ${aisHit}/10 < 10`);
      assert.ok(rate >= 0.8, `Rubric fail: pub fn doc ${(rate * 100).toFixed(2)}% < 80%`);
    });
  });
});

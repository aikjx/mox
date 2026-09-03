/* T10 RED→GREEN 文档三方对账脚本
 * 读 docs/enterprise/02-architecture.md 与 docs/standards/project-atlas.md
 * Assertion 1 (AC-15)：02-architecture §3.2 表 - crate 相关行数 ≥ 15
 * Assertion 2 (AC-16)：部署/runtime 段含 6 关键字：runtime, L1, L2, cordis, rbac_middleware, subservers (6/6)
 * Assertion 3 (AC-17)：project-atlas 含 3 关键字：Rust, CRATE_ID, CRATE_META (3/3)
 */
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const ARCH = path.join(ROOT, 'docs', 'enterprise', '02-architecture.md');
const ATLAS = path.join(ROOT, 'docs', 'standards', 'project-atlas.md');

function read(p) {
  if (!fs.existsSync(p)) {
    console.error(`[FATAL] 文件不存在: ${p}`);
    process.exit(2);
  }
  return fs.readFileSync(p, 'utf8');
}

const arch = read(ARCH);
const atlas = read(ATLAS);

const errors = [];
const summary = {};

// ========== AC-15 §3.2 Rust Workspace 表行数 ==========
(function ac15() {
  const id = 'AC-15';
  // 找到 §3.2 锚点（文本或标题）
  const anchorRe = /§3\.2|3\.2 Rust Workspace 16 Crate|Rust Workspace 16 Crate/i;
  const archNorm = arch.replace(/^\uFEFF/, '');
  const anchorIdx = archNorm.search(anchorRe);

  // 找 §3.2 后紧跟的第一个表格：取 "| 序号 | Crate ID" 表头 或 标记 "|------" 表格起始
  let tableRows = [];
  let reason = '';
  if (anchorIdx < 0) {
    reason = '未找到 §3.2 / Rust Workspace 16 Crate 锚点文本';
  } else {
    // 从锚点开始往后续，抓取首个 table：以 "| 序号" 表头开始，到第一个空行/非 | 行（非表头分隔行）结束
    const tail = archNorm.slice(anchorIdx);
    const lines = tail.split(/\r?\n/);
    let inTable = false;
    let headerFound = false;
    for (const raw of lines) {
      const line = raw.trimEnd();
      if (!inTable) {
        // 表头特征：含 "序号" 和 "Crate" / "Crate ID"
        if (/^\|.*序号.*Crate/i.test(line) || /^\|.*序号.*crate/i.test(line)) {
          inTable = true;
          headerFound = true;
          continue; // 表头行不计入数据行
        }
        // 也可能是分隔行 ------------ 则跳过（需前一行是表头）
        continue;
      }
      // inside table
      if (!line.trim()) break; // 空行结束
      if (!/^\|/.test(line)) break; // 非表格行结束
      if (/^\|\s*-{2,}/.test(line)) continue; // 表头下方的 |----|----| 分隔行跳过
      tableRows.push(line);
    }
    if (!headerFound) {
      reason = '§3.2 区域未找到含"序号+Crate ID"的表头行';
    }
  }

  // 统计含有 "crate"（不区分大小写）的行数
  const crateRows = tableRows.filter(r => /crate/i.test(r));
  const rowCount = crateRows.length;
  summary.ac15 = { anchorFound: anchorIdx >= 0, rowsRead: tableRows.length, rowsContainingCrate: rowCount, required: 15, pass: rowCount >= 15 };

  if (anchorIdx < 0) {
    errors.push(`${id} FAIL: ${reason}`);
  } else if (!summary.ac15.pass) {
    errors.push(`${id} FAIL: 表格含"crate"行数=${rowCount} 未达到 ≥15 (实际读入数据行=${tableRows.length})。` + (reason ? ` 原因: ${reason}` : ''));
  } else {
    console.log(`[PASS] ${id} 行数含 crate=${rowCount} (≥15)，数据行总数=${tableRows.length}`);
  }
})();

// ========== AC-16 部署/runtime 段 6 关键字 ==========
(function ac16() {
  const id = 'AC-16';
  // 定位部署或 runtime crate 段
  const anchor = arch.search(/§7\.1|7\.1\s*部署|部署视图|runtime crate|7\.1\.1 runtime/i);
  let slice = arch;
  if (anchor >= 0) {
    slice = arch.slice(Math.max(0, anchor - 50));
  }
  const required = ['runtime', 'L1', 'L2', 'cordis', 'rbac_middleware', 'subservers'];
  const lower = slice.toLowerCase();
  // 注意 L1/L2 要大写匹配（但不区分大小写会与 l1 混淆，按字面直接匹配 L1/L2 大小写不敏感也行）
  const presence = {};
  for (const k of required) {
    const re = new RegExp(k.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'i');
    presence[k] = re.test(slice);
  }
  const missing = required.filter(k => !presence[k]);
  summary.ac16 = { anchorFound: anchor >= 0, presence, missingCount: missing.length, requiredCount: required.length, pass: missing.length === 0 };
  if (missing.length > 0) {
    errors.push(`${id} FAIL: 关键字缺失 ${missing.length}/${required.length}，缺失=[${missing.join(', ')}]`);
  } else {
    console.log(`[PASS] ${id} 6/6 关键字齐全 (${required.join(', ')})`);
  }
})();

// ========== AC-17 project-atlas 3 关键字 ==========
(function ac17() {
  const id = 'AC-17';
  const required = ['Rust', 'CRATE_ID', 'CRATE_META'];
  const presence = {};
  for (const k of required) {
    // Rust 大小写不敏感（要求存在该词即可），CRATE_ID / CRATE_META 常量通常大写，也允许宽松匹配
    presence[k] = arch.includes(k) || atlas.includes(k);
  }
  // 任务说明：在 project-atlas.md 中存在 3/3
  for (const k of required) {
    presence[k] = atlas.includes(k);
  }
  const missing = required.filter(k => !presence[k]);
  summary.ac17 = { presence, missingCount: missing.length, requiredCount: required.length, pass: missing.length === 0 };
  if (missing.length > 0) {
    errors.push(`${id} FAIL: project-atlas.md 关键字缺失 ${missing.length}/${required.length}，缺失=[${missing.join(', ')}]`);
  } else {
    console.log(`[PASS] ${id} 3/3 关键字齐全 (${required.join(', ')})`);
  }
})();

// 输出结果并退出
console.log('\n=== SUMMARY ===');
console.log(JSON.stringify(summary, null, 2));

if (errors.length > 0) {
  console.log('\n=== FAILURES ===');
  for (const e of errors) console.log('  - ' + e);
  process.exit(1);
}

console.log('\nALL ASSERTIONS PASSED.');
process.exit(0);

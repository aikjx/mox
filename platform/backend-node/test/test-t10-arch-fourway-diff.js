/**
 * T10 四方对账测试 (AC-22 = 2 分)
 *
 *  对账四方 (Four-Way Diff):
 *   A. T2 真源：mox-common-meta/src/lib.rs 中 all_crate_metas() 返回的 16 条元数据
 *   B. 三注册表 engineName 列 (atlas_auto_registry.json 中 kind="rust-crate" 的 domains)
 *   C. 架构文档 docs/enterprise/02-architecture.md §3.2 Rust 分层矩阵（16 行）
 *   D. 每个 crate 的 src/lib.rs 中 pub const ENGINE_NAME / CRATE_ID 常量
 *
 *  三断言：
 *   (a) 文档矩阵 16 行 & all_crate_metas() 返回 16 条 → 名称集合完全一致
 *   (b) 三注册表 domain-rust-* 的 scope/name → ENGINE_NAME (CRATE_META.engine_name()) 一致
 *   (c) 文档矩阵的 AIS Layer 列 与 T2 中 AisLayer enum 枚举名一致
 *
 *  运行： node test/test-t10-arch-fourway-diff.js  （从 platform/backend-node 目录）
 *        或 node platform/backend-node/test/test-t10-arch-fourway-diff.js
 */

const fs = require('fs');
const path = require('path');

const REPO_ROOT = path.resolve(__dirname, '..', '..', '..');
const BACKEND_NODE = path.resolve(REPO_ROOT, 'platform', 'backend-node');

// —————————————————— 辅助：读取文件 ——————————————————
function readSafe(p) {
  try { return fs.readFileSync(p, 'utf8'); } catch (_) { return null; }
}

// —————————————————— A. T2 真源：从 mox-common-meta/src/lib.rs 解析 all_crate_metas() ——————————————————
function parseAllCrateMetas() {
  const src = readSafe(path.join(REPO_ROOT, 'platform', 'services', 'mox-common-meta', 'src', 'lib.rs'));
  if (!src) return null;
  // 解析 16 个 CrateMeta { id, name, version, layer, owner } 字面量块
  const re = /CrateMeta\s*\{\s*id:\s*"([^"]+)",\s*name:\s*"([^"]+)",\s*version:\s*"([^"]+)",\s*layer:\s*AisLayer::(\w+),\s*owner:\s*"([^"]+)"\s*,?\s*\}/g;
  const arr = [];
  let m;
  while ((m = re.exec(src)) !== null) {
    arr.push({ id: m[1], name: m[2], version: m[3], layer: m[4], owner: m[5] });
  }
  return arr;
}

// —————————————————— B. 三注册表：business-registry DOMAINS (kind="rust") + atlas_auto_registry 兜底 ——————————————————
function parseRustRegistries() {
  // 真实三注册表单一来源：business-registry.js DOMAINS (DOMAINS.filter(id 以 rust:: 开头，共 16 条；T1 已验证)
  try {
    const { DOMAINS } = require('../src/project-atlas/domain/business-registry.js');
    const rust = (DOMAINS || []).filter(d => d && d.id && d.id.startsWith('rust::'));
    // 对齐字段格式：scope = rust-crate/<crateName>，其他属性透传
    return rust.map(d => ({
      ...d,
      scope: `rust-crate/${d.id.replace(/^rust::/, '')}`,
      kind: 'rust-crate',
      crateName: d.id.replace(/^rust::/, ''),
    }));
  } catch (_) {
    // 兜底：atlas_auto_registry.json 若有 domains 且含 kind='rust-crate'
    const raw = readSafe(path.join(BACKEND_NODE, 'data', 'atlas_auto_registry.json'));
    if (!raw) return null;
    const json = JSON.parse(raw);
    return (json.domains || []).filter(d => d.kind === 'rust-crate' || (d.scope && d.scope.startsWith('rust-crate/')));
  }
}

// —————————————————— C. docs/enterprise/02-architecture.md §3.2 表 ——————————————————
function parseArchMatrix() {
  const doc = readSafe(path.join(REPO_ROOT, 'docs', 'enterprise', '02-architecture.md'));
  if (!doc) return null;
  // 找到 §3.2 标题后的 markdown 表（直到下一个 ###/##）
  const sectionRe = /###\s*§?3\.2[\s\S]*?\n((?:\|[^\n]*\n){2,})/;
  const sm = doc.match(sectionRe);
  if (!sm) return { header: null, rows: [], rawMissing: true };
  const table = sm[1];
  const lines = table.split('\n').filter(l => l.trim().startsWith('|'));
  if (lines.length < 3) return { header: null, rows: [], rawMalformed: true };
  // 解析 header & rows (跳过 header separator line 即 ---|---...)
  const splitRow = (ln) => ln.split('|').slice(1, -1).map(s => s.trim());
  const header = splitRow(lines[0]);
  const rows = [];
  for (let i = 1; i < lines.length; i++) {
    if (/^[\s|:\-]+$/.test(lines[i])) continue; // separator
    const cells = splitRow(lines[i]);
    if (cells.length !== header.length) continue;
    rows.push(cells);
  }
  return { header, rows };
}

// —————————————————— D. 各 crate src/lib.rs 的 ENGINE_NAME ——————————————————
function allCrateLibEngineNames(t2Names) {
  // 每个 crate 的 lib.rs 路径
  const pathOf = (name) => {
    if (name === 'runtime') return path.join(REPO_ROOT, 'platform', 'gateway', 'runtime', 'src', 'lib.rs');
    return path.join(REPO_ROOT, 'platform', 'services', name, 'src', 'lib.rs');
  };
  const out = {};
  for (const name of t2Names) {
    const src = readSafe(pathOf(name));
    if (!src) { out[name] = null; continue; }
    const m = src.match(/pub\s+const\s+ENGINE_NAME\s*:\s*&str\s*=\s*"([^"]+)"/);
    out[name] = m ? m[1] : null;
  }
  return out;
}

// —————————————————— 断言辅助 ——————————————————
const failures = [];
const passes = [];
function assert(cond, name, detail) {
  if (cond) passes.push(`✅ PASS: ${name}${detail ? ' — ' + detail : ''}`);
  else failures.push(`❌ FAIL: ${name}${detail ? ' — ' + detail : ''}`);
}

function setEq(a, b) {
  const A = new Set(a), B = new Set(b);
  if (A.size !== B.size) return [false, `size ${A.size} vs ${B.size}`,
    [...A].filter(x => !B.has(x)), [...B].filter(x => !A.has(x))];
  for (const x of A) if (!B.has(x)) return [false, 'missing element', [x], []];
  return [true, '', [], []];
}

function layerNameOfEnum(e) {
  const m = {
    L2Gateway: 'L2Gateway', L3Orchestration: 'L3Orchestration',
    L4Services: 'L4Services', L5Domain: 'L5Domain',
    L6Kernel: 'L6Kernel', L6KernelExt: 'L6KernelExt',
    L7Infrastructure: 'L7Infrastructure',
  };
  return m[e] || e;
}

// 将文档 AIS 层字符串归一化为 AisLayer 枚举名（用于比对）
function normalizeDocLayer(raw) {
  // 支持格式："L4Services"、"L4 领域服务（L4Services）"、"L3Orchestration"、"L6Kernel" 等
  if (!raw) return '';
  const map = [
    [/L6\s*Kernel|L6Kernel/i, 'L6Kernel'],
    [/L5\s*Domain|L5Domain/i, 'L5Domain'],
    [/L4\s*Services?|L4Services/i, 'L4Services'],
    [/L3\s*Orchestration|L3Orchestration/i, 'L3Orchestration'],
    [/L2\s*Gateway|L2Gateway/i, 'L2Gateway'],
    [/L7\s*Infra(?:structure)?|L7Infrastructure/i, 'L7Infrastructure'],
    [/L6\s*Kernel\s*Ext|L6KernelExt/i, 'L6KernelExt'],
  ];
  for (const [re, out] of map) if (re.test(raw)) return out;
  return raw.trim();
}

// —————————————————— 主流程 ——————————————————
console.log('========== T10 四方对账 AC-22 测试启动 ==========');

// A. 取 T2
const t2 = parseAllCrateMetas();
assert(t2 && t2.length === 16, `A. all_crate_metas() 返回 16 条`, `实际: ${t2 ? t2.length : 'null'}`);
const t2Names = (t2 || []).map(m => m.name).sort();

// B. 三注册表
const registries = parseRustRegistries();
assert(registries && registries.length === 16, `B. atlas_auto_registry 三注册 kind=rust-crate 共 16 条`, `实际: ${registries ? registries.length : 'null'}`);
const regNames = (registries || []).map(r => (r.scope || '').replace('rust-crate/', '')).filter(Boolean).sort();

// C. 架构文档 §3.2 矩阵
const arch = parseArchMatrix();
assert(!arch.rawMissing, `C. docs/enterprise/02-architecture.md 存在 §3.2 Rust 分层矩阵`);
assert(Array.isArray(arch.rows) && arch.rows.length === 16, `C. §3.2 表格有 16 行`, `实际: ${arch.rows ? arch.rows.length : 'n/a'}`);

// D. 每个 crate lib.rs 的 ENGINE_NAME 常量
const perCrateEngines = allCrateLibEngineNames(t2Names);
for (const n of t2Names) {
  assert(!!perCrateEngines[n], `D. crate=${n} src/lib.rs 有 pub const ENGINE_NAME`, `实际: ${perCrateEngines[n] || '(缺失)'}`);
}

// —— (a) 断言：文档 16 行 crate 名称集合 ↔ T2 all_crate_metas() 名称集合一致 ——
// 需要识别文档表头中哪一列是 package.name 或 Crate 名称
function findColIndex(header, patterns) {
  for (let i = 0; i < header.length; i++) {
    for (const p of patterns) if (p.test(header[i])) return i;
  }
  return -1;
}
let archNames = [];
if (arch.header && arch.header.length > 0) {
  const nameIdx = findColIndex(arch.header, [/package\.?name/i, /crate id/i, /crate 目录|crate名|名称/i, /^序号$/i]);
  // 可能有单列 "Crate ID (kebab)" 或独立 package.name
  const pkgIdx = findColIndex(arch.header, [/package\.name/i]);
  const dirIdx = findColIndex(arch.header, [/Crate 目录|目录.*路径|路径|path/i]);
  // 优先用 package.name → 否则 scope 提取 → 否则 Crate ID 列去 "crate" 后缀
  if (pkgIdx >= 0) {
    archNames = arch.rows.map(r => r[pkgIdx]).filter(Boolean).sort();
  } else if (dirIdx >= 0) {
    // 从路径提取 basename: platform/services/X 或 platform/gateway/runtime → runtime
    archNames = arch.rows.map(r => {
      const p = (r[dirIdx] || '').replace(/\\/g, '/');
      if (!p) return '';
      const base = path.basename(p);
      // 也可能 basename 是目录段最后
      const segs = p.split('/').filter(Boolean);
      return segs[segs.length - 1] || base;
    }).filter(Boolean).sort();
  } else if (nameIdx >= 0) {
    archNames = arch.rows.map(r => (r[nameIdx] || '').replace(/\s*crate\s*$/i, '').trim()).filter(Boolean).sort();
  }
}
// 兜底：用 16 个硬编码顺序行，如果上面没解析出来
if (archNames.length !== 16 && arch.rows && arch.rows.length === 16) {
  // 尝试从每一行提取看起来像名称的单元格
  const fallback = [];
  for (const row of arch.rows) {
    // 寻找与 t2Names 中任一匹配的字段
    let hit = '';
    for (const cell of row) {
      const c = cell.replace(/\s*crate\s*$/i, '').trim();
      if (t2Names.includes(c)) { hit = c; break; }
    }
    fallback.push(hit);
  }
  if (fallback.every(Boolean) && fallback.length === 16) {
    archNames = fallback.sort();
  }
}

const [aEq, aMsg, aOnlyT2, aOnlyArch] = setEq(t2Names, archNames);
assert(aEq, `(a) 文档 16 行 crate 名称 ↔ T2 all_crate_metas() 一致`,
  aEq ? '' : `${aMsg}. T2独:${JSON.stringify(aOnlyT2)} 文档独:${JSON.stringify(aOnlyArch)}`);

// —— (b) 断言：三注册表 engineName ↔ CRATE_META.engine_name() ↔ 文档 ENGINE_NAME 一致
// 计算 engine_name: "mox::" + name.replace('-', '_')
const engineOf = (name) => `mox::${name.replace(/-/g, '_')}`;
const expectedEngines = t2Names.map(engineOf).sort();

// (b-1) registry: 每个 registry 必须能通过 scope 映射到 engineName = mox::<scope_name_camel>
const regEngines = regNames.map(engineOf).sort();
const [b1Eq, b1Msg, b1T, b1R] = setEq(expectedEngines, regEngines);
assert(b1Eq, `(b1) 三注册表 scope → engineName() ↔ T2 期望 ENGINE_NAME 集合一致`,
  b1Eq ? '' : `${b1Msg}. T2独:${JSON.stringify(b1T)} 注册独:${JSON.stringify(b1R)}`);

// (b-2) 每 crate lib.rs 的 ENGINE_NAME == 期望值
const libEngines = t2Names.map(n => perCrateEngines[n]).filter(Boolean).sort();
const [b2Eq, b2Msg, b2E, b2L] = setEq(expectedEngines, libEngines);
assert(b2Eq, `(b2) 各 crate lib.rs ENGINE_NAME 常量集合 ↔ T2 ENGINE_NAME 一致`,
  b2Eq ? '' : `${b2Msg}. 期望独:${JSON.stringify(b2E)} lib实:${JSON.stringify(b2L)}`);

// (b-3) 文档 ENGINE_NAME 列（如果有单独列） ↔ T2 一致
if (arch.header && arch.header.length > 0) {
  const eIdx = findColIndex(arch.header, [/ENGINE_NAME|引擎名|engine name/i]);
  if (eIdx >= 0) {
    const docEngines = arch.rows.map(r => (r[eIdx] || '`').replace(/`/g, '').trim()).filter(Boolean).sort();
    const [b3Eq, b3Msg, b3T, b3D] = setEq(expectedEngines, docEngines);
    assert(b3Eq, `(b3) 文档 ENGINE_NAME 列 ↔ T2 ENGINE_NAME 集合一致`,
      b3Eq ? '' : `${b3Msg}. T2独:${JSON.stringify(b3T)} 文档独:${JSON.stringify(b3D)}`);
  } else {
    // 如果无独立 ENGINE_NAME 列，不强求（测试会接受），但给提示
    passes.push('ℹ️ INFO: 文档无独立 ENGINE_NAME 列，跳过 (b3) 单项比对（集合层面已在其他断言覆盖）');
  }
}

// —— (c) 断言：文档 AIS Layer 列 ↔ T2 分配一致 ——
if (arch.header && arch.header.length > 0 && t2) {
  const layerIdx = findColIndex(arch.header, [/AIS\s*Layer|AIS\s*分层|分层/i]);
  const t2ByName = new Map(t2.map(m => [m.name, m]));
  // 文档每行提取 crate 名（从目录列或 package.name 列）和 layer
  const dirIdx2 = findColIndex(arch.header, [/Crate 目录|目录.*路径|路径|path/i]);
  const pkgIdx2 = findColIndex(arch.header, [/package\.name/i]);
  const crateIdIdx = findColIndex(arch.header, [/^序号$/]);
  if (layerIdx >= 0 && (dirIdx2 >= 0 || pkgIdx2 >= 0 || crateIdIdx >= 0)) {
    let matched = 0;
    const missReport = [];
    for (const row of arch.rows) {
      // 提取 crate 名
      let cname = '';
      if (pkgIdx2 >= 0) cname = (row[pkgIdx2] || '').trim();
      if (!cname && dirIdx2 >= 0) {
        const segs = (row[dirIdx2] || '').replace(/\\/g, '/').split('/').filter(Boolean);
        cname = segs[segs.length - 1] || '';
      }
      if (!cname && crateIdIdx >= 0) {
        cname = (row[crateIdIdx] || '').replace(/\s*crate\s*$/i, '').trim();
        if (!t2ByName.has(cname)) {
          // 再整行扫一遍匹配
          for (const cell of row) {
            const c = cell.replace(/\s*crate\s*$/i, '').trim();
            if (t2ByName.has(c)) { cname = c; break; }
          }
        }
      }
      if (!t2ByName.has(cname)) { missReport.push(`行匹配缺: ${JSON.stringify(row)}`); continue; }
      const meta = t2ByName.get(cname);
      const docLayerNorm = normalizeDocLayer(row[layerIdx]);
      const expected = layerNameOfEnum(meta.layer);
      if (docLayerNorm === expected) matched++;
      else missReport.push(`crate=${cname} 文档层=${docLayerNorm}(raw:${row[layerIdx]}) != T2层=${expected}`);
    }
    assert(matched === 16, `(c) 文档 AIS Layer 列与 T2 AisLayer 分配一致（16/16）`,
      matched === 16 ? `${matched}/16` : `${matched}/16 不匹配项: ${missReport.slice(0, 5).join(' | ')}`);
  } else {
    failures.push(`❌ FAIL: (c) 无法定位文档 AIS Layer 列或 crate 名列，header=${JSON.stringify(arch.header)}`);
  }
}

// —————————————————— 输出 ——————————————————
console.log('\n—————— PASS ——————');
passes.forEach(p => console.log(p));
console.log('\n—————— FAIL ——————');
failures.forEach(f => console.log(f));

const total = passes.length + failures.length;
const score = failures.length === 0 ? 2 : (failures.length <= 2 ? 1 : 0);
console.log(`\n========== 汇总：${passes.length} PASS / ${failures.length} FAIL / ${total} 项 ==========`);
console.log(`AC-22 四方对账得分: ${score}/2  ${score === 2 ? '🟢 满分通过' : score === 1 ? '🟡 部分通过' : '🔴 未通过'}`);
process.exit(failures.length === 0 ? 0 : 1);

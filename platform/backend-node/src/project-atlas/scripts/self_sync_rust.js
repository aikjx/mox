'use strict';
/**
 * self_sync_rust.js — 璇玑 Rust 16 crate 自发现 / 自登记脚本
 * ------------------------------------------------------------------
 * 扫描：
 *   - platform/services/**​/*.rs
 *   - platform/gateway/runtime/src/**​/*.rs
 * 对每个 .rs 文件提取：
 *   - pub (async )?fn <name> （简单正则）
 *   - pub struct <name>
 *   - pub const <name>
 *   - 所在 crate（目录名 / scope）
 *
 * 输出：data/atlas_auto_registry_rust.json
 *   shape = { entries: [{ crateName, crateId, fns, structs, consts, files:[{filePath, fns, structs, consts}] }] }
 *
 * CRATE_ID 表来自 mox-common-meta all_crate_metas（与 T2 一致，静态内置以避免 Node 端调用 Rust）。
 */

const fs = require('fs');
const path = require('path');

const BACKEND = path.join(__dirname, '..', '..', '..');           // backend-node
const REPO = path.join(BACKEND, '..', '..');                       // repo root
const OUT_JSON = path.join(BACKEND, 'data', 'atlas_auto_registry_rust.json');

/** 16 crate 静态 CRATE_ID 表（与 mox-common-meta/src/lib.rs all_crate_metas 完全一致） */
const CRATE_ID_TABLE = {
  'ai-agent':           '00374bdd-cc60-55bf-8970-a879afbfe443',
  'business-catalog':   '62b2cca1-d98f-5e41-b26e-8d2a43966117',
  'flow-ai':            '2fcd3eac-e894-5876-b007-fb33c56c0d65',
  'graph-algorithms':   'fbd31c6a-41cd-5274-be2f-2a28066eaf0a',
  'hermes-flow-bridge': '9bfaf43b-385a-5a44-9fb2-65b4003ee80d',
  'kg-hub':             'cb909f06-c0df-55ec-b397-543623a8c349',
  'operator-core':      'acf14283-3931-5528-adce-2c0cd3815363',
  'operator-wasm':      '5a1df407-b217-5340-a5ae-5f4535d1e6de',
  'optimizer':          'e56676c7-ec1f-5415-9587-ba8249d0178a',
  'primiflow-core':     '8c8d2382-6f9f-5218-894e-a07a43aa9554',
  'primiflow-fusion':   '75238345-b48b-534b-818b-8d9abe083a41',
  'template-market':    '4d2e50c1-9d64-525d-86cf-2d7d610a27b9',
  'mox-expert':      '50bb6200-04c5-5e4c-8354-4c6e1b230024',
  'mox-system':      'b81eec75-22ff-5155-ac49-19edf6f6b5ab',
  'mox-common-meta': '34a20231-1a80-5426-b392-40d7a2ddd9f7',
  'runtime':            'a6f7ad5c-dbc8-5c27-837f-d8332fd6f27b'
};

/** 扫描根目录（相对 REPO） -> crateName 映射 */
const ROOT_SCOPES = [
  { dir: path.join(REPO, 'platform', 'services'), crateOfFile: crateOfFileUnderServices },
  { dir: path.join(REPO, 'platform', 'gateway', 'runtime'), crateOfFile: () => 'runtime' }
];

function crateOfFileUnderServices(absFilePath, servicesDir) {
  const rel = path.relative(servicesDir, absFilePath).split(path.sep);
  return rel[0] || null;
}

function walkDir(dir, out) {
  if (!fs.existsSync(dir)) return;
  for (const name of fs.readdirSync(dir)) {
    if (name.startsWith('.')) continue;
    if (name === 'target') continue;
    const fp = path.join(dir, name);
    const st = fs.statSync(fp);
    if (st.isDirectory()) walkDir(fp, out);
    else if (name.endsWith('.rs')) out.push(fp);
  }
}

/** 对单个 .rs 文件做正则抽取 */
function parseRsFile(filePath) {
  const src = fs.readFileSync(filePath, 'utf8');
  const fns = [];
  const structs = [];
  const consts = [];
  const reFn = /^\s*pub\s+(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)/gm;
  const reStruct = /^\s*pub\s+struct\s+([A-Za-z_][A-Za-z0-9_]*)/gm;
  const reConst = /^\s*pub\s+const\s+([A-Za-z_][A-Za-z0-9_]*)/gm;
  let m;
  while ((m = reFn.exec(src))) fns.push(m[1]);
  while ((m = reStruct.exec(src))) structs.push(m[1]);
  while ((m = reConst.exec(src))) consts.push(m[1]);
  return {
    fns: Array.from(new Set(fns)).sort(),
    structs: Array.from(new Set(structs)).sort(),
    consts: Array.from(new Set(consts)).sort()
  };
}

function main() {
  const allRsFiles = [];
  const crateRoot = new Map(); // crateName -> crate scope dir
  for (const scope of ROOT_SCOPES) {
    if (!fs.existsSync(scope.dir)) continue;
    const dirFiles = [];
    walkDir(scope.dir, dirFiles);
    for (const f of dirFiles) {
      let crateName;
      if (scope.crateOfFile.length >= 2) crateName = scope.crateOfFile(f, scope.dir);
      else crateName = scope.crateOfFile(f);
      if (!crateName) continue;
      allRsFiles.push({ crateName, absPath: f });
    }
  }

  // crate 聚合
  const byCrate = new Map();
  for (const { crateName, absPath } of allRsFiles) {
    if (!byCrate.has(crateName)) {
      byCrate.set(crateName, { crateName, crateId: CRATE_ID_TABLE[crateName] || '', fns: [], structs: [], consts: [], files: [] });
    }
    const entry = byCrate.get(crateName);
    const relFromRepo = path.relative(REPO, absPath).split(path.sep).join('/');
    const parsed = parseRsFile(absPath);
    entry.files.push({
      filePath: relFromRepo,
      fns: parsed.fns,
      structs: parsed.structs,
      consts: parsed.consts
    });
    for (const f of parsed.fns) if (!entry.fns.includes(f)) entry.fns.push(f);
    for (const s of parsed.structs) if (!entry.structs.includes(s)) entry.structs.push(s);
    for (const c of parsed.consts) if (!entry.consts.includes(c)) entry.consts.push(c);
  }

  // 排序
  for (const entry of byCrate.values()) {
    entry.fns.sort();
    entry.structs.sort();
    entry.consts.sort();
    entry.files.sort((a, b) => (a.filePath < b.filePath ? -1 : 1));
  }
  const entries = Array.from(byCrate.values()).sort((a, b) => (a.crateName < b.crateName ? -1 : 1));

  const outDir = path.dirname(OUT_JSON);
  if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true });
  fs.writeFileSync(OUT_JSON, JSON.stringify({ entries, generatedAt: new Date().toISOString() }, null, 2), 'utf8');

  const totalFiles = entries.reduce((s, e) => s + e.files.length, 0);
  const totalFns = entries.reduce((s, e) => s + e.fns.length, 0);
  const totalStructs = entries.reduce((s, e) => s + e.structs.length, 0);
  const totalConsts = entries.reduce((s, e) => s + e.consts.length, 0);
  process.stdout.write(
    `[self_sync_rust] crates=${entries.length} files=${totalFiles} fns=${totalFns} ` +
    `structs=${totalStructs} consts=${totalConsts} out=${path.relative(process.cwd(), OUT_JSON)}\n`
  );
  return { entries, totalFiles, totalFns, totalStructs, totalConsts };
}

if (require.main === module) {
  main();
}

module.exports = { main, CRATE_ID_TABLE };

'use strict';
/**
 * T4 依赖治理合规脚本 (TR 4.x, AC-16)
 * ====================================
 * 铁律：所有 member crate 必须使用 workspace = true 继承版本，
 *      禁止在各自 Cargo.toml 里写与 workspace.dependencies 不一致的版本号。
 *
 *  TR 4.1：每个 member Cargo.toml 中常见依赖（见 CHECK_DEPS 列表）
 *          如果使用 version = "x.y" 而非 workspace = true → FAIL
 *  TR 4.2：显式声明 version 的字符串若与 workspace.dependencies 不一致 → FAIL
 *          （即 "侥幸一致但未继承" 仍 FAIL，以强化治理）
 *  TR 4.3：CHECK_DEPS 全集在 workspace.dependencies 都有定义
 *
 *  运行:
 *      cd platform/backend-node && node test/test-tr-4-compliance.js
 *      cd platform/backend-node && npx mocha test/test-tr-4-compliance.js --exit --timeout 30000
 */

const assert = require('assert');
const fs = require('fs');
const path = require('path');

const REPO_ROOT = path.resolve(__dirname, '..', '..', '..');
const WORKSPACE_TOML = path.join(REPO_ROOT, 'Cargo.toml');

const CHECK_DEPS = [
  'serde', 'serde_json', 'serde_yaml', 'tokio', 'anyhow', 'thiserror',
  'tracing', 'tracing-subscriber', 'uuid', 'chrono', 'rayon', 'http',
  'hostname', 'nalgebra', 'ndarray', 'num-traits', 'approx',
  'criterion', 'wasmer', 'wasmer-compiler-cranelift',
  'petgraph', 'axum', 'tower-http', 'tower',
  'reqwest', 'base64', 'rusqlite',
  'sha2', 'hmac', 'hex',
  'parking_lot', 'async-trait',
  'tokio-tungstenite', 'futures', 'futures-util',
  'sea-query', 'sqlx', 'xuanji-common-meta'
];

const MEMBERS = [
  'platform/services/operator-core',
  'platform/services/operator-wasm',
  'platform/services/graph-algorithms',
  'platform/services/optimizer',
  'platform/services/flow-ai',
  'platform/services/xuanji-expert',
  'platform/services/hermes-flow-bridge',
  'platform/services/business-catalog',
  'platform/services/ai-agent',
  'platform/services/template-market',
  'platform/gateway/runtime',
  'platform/services/xuanji-system',
  'platform/services/primiflow-core',
  'platform/services/primiflow-fusion',
  'platform/services/kg-hub',
  'platform/services/xuanji-common-meta',
];

// ===== 最小 TOML 解析器（仅限本脚本所需字段）=====
function stripComment(line) {
  let inStr = false, inSingle = false;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (c === '"' && !inSingle) inStr = !inStr;
    else if (c === "'" && !inStr) inSingle = !inSingle;
    else if (c === '#' && !inStr && !inSingle) return line.slice(0, i);
  }
  return line;
}

function parseTomlTables(text) {
  const lines = text.split(/\r?\n/).map(stripComment);
  const tables = {};
  let cur = '';
  let header = '';
  tables[''] = [];
  for (const raw of lines) {
    const l = raw.trim();
    if (!l) continue;
    const h = l.match(/^\[([^\]]+)\]$/);
    if (h) {
      header = h[1];
      cur = header;
      if (!(cur in tables)) tables[cur] = [];
      continue;
    }
    if (!(cur in tables)) tables[cur] = [];
    tables[cur].push(l);
  }
  return tables;
}

function parseDepsBlock(lines) {
  // 多行合并：{ ... } 或 key = "..." 或 key = { workspace = true, ... }
  const merged = [];
  let buf = '';
  let depth = 0;
  for (const l of lines) {
    if (!buf) buf = l; else buf += ' ' + l;
    for (const c of l) {
      if (c === '{') depth++;
      else if (c === '}') depth--;
    }
    if (depth <= 0) {
      merged.push(buf);
      buf = '';
      depth = 0;
    }
  }
  if (buf) merged.push(buf);

  const out = {};
  for (const stmt of merged) {
    const m = stmt.match(/^([A-Za-z0-9_\-]+)\s*=\s*(.+)$/);
    if (!m) continue;
    const key = m[1];
    const val = m[2].trim();
    let rec = { _raw: val };
    if (/^"/.test(val)) {
      rec.version = val.replace(/^"([^"]*)".*$/, '$1');
      rec.stringForm = true;
    } else if (/^\{/.test(val)) {
      const inner = val.replace(/^\{/, '').replace(/\}$/, '');
      const parts = inner.split(',').map(s => s.trim()).filter(Boolean);
      for (const p of parts) {
        const kv = p.match(/^(\w+)\s*=\s*(.+)$/);
        if (!kv) continue;
        const k = kv[1];
        let v = kv[2].trim();
        if (/^"/.test(v)) v = v.replace(/^"([^"]*)"$/, '$1');
        else if (v === 'true') v = true;
        else if (v === 'false') v = false;
        rec[k] = v;
      }
    }
    out[key] = rec;
  }
  return out;
}

function readSafe(p) { try { return fs.readFileSync(p, 'utf8'); } catch (_) { return null; } }

// —— 允许直接用 node 执行（非 mocha 环境）：先检测全局 describe 是否存在，若不存在则立即跑简易分支并退出 ——
const IS_MOCHA = (typeof describe === 'function') && (typeof it === 'function');
if (!IS_MOCHA && require.main === module) {
  const report = { pass: 0, fail: 0 };
  function caseIt(name, fn) {
    try { fn(); report.pass++; console.log('  PASS', name); }
    catch (e) { report.fail++; console.error('  FAIL', name, '\n    ', e.message); }
  }
  const wsText = readSafe(WORKSPACE_TOML);
  let wsDeps = {};
  console.log('\n=== T4 依赖治理合规脚本 (node direct mode) ===');
  caseIt('TR 4.3 workspace.dependencies 定义齐全', function () {
    if (!wsText) throw new Error('workspace Cargo.toml 不存在');
    const tables = parseTomlTables(wsText);
    wsDeps = parseDepsBlock(tables['workspace.dependencies'] || []);
    const missing = CHECK_DEPS.filter(d => !(d in wsDeps));
    if (missing.length) throw new Error('缺失：' + missing.join(', '));
  });
  caseIt('TR 4.1 workspace=true 继承', function () {
    const violations = [];
    for (const rel of MEMBERS) {
      const ctoml = path.join(REPO_ROOT, rel, 'Cargo.toml');
      const text = readSafe(ctoml);
      if (!text) { violations.push(rel + ': 缺文件'); continue; }
      const tables = parseTomlTables(text);
      for (const blockKey of ['dependencies', 'dev-dependencies', 'build-dependencies']) {
        const deps = parseDepsBlock(tables[blockKey] || []);
        for (const name of CHECK_DEPS) {
          if (!(name in deps)) continue;
          const rec = deps[name];
          if (rec.stringForm) { violations.push(`${rel}/${blockKey} ${name} 直接写字符串版本`); continue; }
          if (rec.version && !rec.workspace) { violations.push(`${rel}/${blockKey} ${name} version 字段未 workspace`); continue; }
          if (rec.workspace !== true && !('workspace' in rec)) {
            if (rec.path && /xuanji-common-meta/.test(rec.path)) continue;
            violations.push(`${rel}/${blockKey} ${name} 无 workspace=true`);
          }
        }
      }
    }
    if (violations.length) throw new Error(violations.length + ' 处：\n  - ' + violations.join('\n  - '));
  });
  caseIt('TR 4.2 版本号一致性（RED 护栏）', function () {
    const mismatches = [];
    for (const rel of MEMBERS) {
      const ctoml = path.join(REPO_ROOT, rel, 'Cargo.toml');
      const text = readSafe(ctoml);
      if (!text) continue;
      const tables = parseTomlTables(text);
      for (const blockKey of ['dependencies', 'dev-dependencies', 'build-dependencies']) {
        const deps = parseDepsBlock(tables[blockKey] || []);
        for (const name of CHECK_DEPS) {
          if (!(name in deps)) continue;
          const rec = deps[name];
          if (!rec.version) continue;
          const wsv = wsDeps[name] && wsDeps[name].version;
          if (wsv && rec.version !== wsv) mismatches.push(`${rel}::${name}`);
        }
      }
    }
    if (mismatches.length) throw new Error(mismatches.join(', '));
  });
  console.log(`\n结果：PASS=${report.pass}  FAIL=${report.fail}`);
  process.exit(report.fail > 0 ? 1 : 0);
}
// mocha 全局可用时，再执行 describe 定义
describe('T4 · 依赖治理归一化合规 (workspace = true)', function () {
  this.timeout(30000);

  const wsText = readSafe(WORKSPACE_TOML);
  let wsDeps = {};
  before(function () {
    assert.ok(wsText, `workspace Cargo.toml 不存在：${WORKSPACE_TOML}`);
    const tables = parseTomlTables(wsText);
    const depLines = tables['workspace.dependencies'] || [];
    wsDeps = parseDepsBlock(depLines);
  });

  it('TR 4.3：CHECK_DEPS 全部在 workspace.dependencies 定义', function () {
    const missing = CHECK_DEPS.filter(d => !(d in wsDeps));
    assert.deepStrictEqual(missing, [],
      `workspace.dependencies 缺少定义：${missing.join(', ')}`);
  });

  it('TR 4.1：所有 member crate 对 CHECK_DEPS 一律 workspace=true 继承（禁止直接写版本字面量）', function () {
    const violations = [];
    for (const rel of MEMBERS) {
      const ctoml = path.join(REPO_ROOT, rel, 'Cargo.toml');
      const text = readSafe(ctoml);
      if (!text) { violations.push(`${rel}: Cargo.toml 不存在`); continue; }
      const tables = parseTomlTables(text);
      for (const blockKey of ['dependencies', 'dev-dependencies', 'build-dependencies']) {
        const lines = tables[blockKey] || [];
        if (!lines.length) continue;
        const deps = parseDepsBlock(lines);
        for (const name of CHECK_DEPS) {
          if (!(name in deps)) continue;
          const rec = deps[name];
          if (rec.stringForm) {
            violations.push(`${rel} [${blockKey}] ${name} = "${rec.version}" (应使用 workspace=true)`);
            continue;
          }
          if (rec.version && !rec.workspace) {
            violations.push(`${rel} [${blockKey}] ${name} 写了 version=${rec.version} 但未 workspace=true`);
            continue;
          }
          if (rec.workspace !== true && !('workspace' in rec)) {
            // 可能是 path + features，但未声明 workspace=true
            // 如果是内部 crate path 指向 workspace 成员也接受：xuanji-common-meta 例外
            if (rec.path && /xuanji-common-meta/.test(rec.path)) continue;
            violations.push(`${rel} [${blockKey}] ${name} 缺少 workspace=true 声明`);
          }
        }
      }
    }
    assert.deepStrictEqual(violations, [],
      `发现依赖治理违规 ${violations.length} 处：\n  - ${violations.join('\n  - ')}`);
  });

  it('TR 4.2：若侥幸写了字面量版本必须与 workspace 一致（此断言作为 RED 护栏）', function () {
    const mismatches = [];
    for (const rel of MEMBERS) {
      const ctoml = path.join(REPO_ROOT, rel, 'Cargo.toml');
      const text = readSafe(ctoml);
      if (!text) continue;
      const tables = parseTomlTables(text);
      for (const blockKey of ['dependencies', 'dev-dependencies', 'build-dependencies']) {
        const lines = tables[blockKey] || [];
        const deps = parseDepsBlock(lines);
        for (const name of CHECK_DEPS) {
          if (!(name in deps)) continue;
          const rec = deps[name];
          if (!rec.version) continue;
          if (!(name in wsDeps)) continue;
          const wsv = wsDeps[name].version || (typeof wsDeps[name] === 'string' ? wsDeps[name] : null);
          if (wsv && rec.version !== wsv) {
            mismatches.push(`${rel}::${name} local=${rec.version} vs workspace=${wsv}`);
          }
        }
      }
    }
    assert.deepStrictEqual(mismatches, [],
      `版本不一致：\n  - ${mismatches.join('\n  - ')}`);
  });
});

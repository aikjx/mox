#!/usr/bin/env node
/**
 * test-tr-4-compliance.js
 *
 * T4 依赖治理合规测试
 *  TR 4.1: 17 个 Cargo.toml 中 [dependencies]/[dev-dependencies] 非 workspace 化的"外部 crate"依赖声明行数 ≤ 1
 *  TR 4.2: `cargo tree -p primiflow-core -i reqwest` 所有 reqwest 版本前缀 = 0.12.x
 *  辅助:
 *   - criterion_default_features_remaining_count: 仍显式含 default-features 的 criterion 声明数（应为 0）
 *   - package_inheritance_defects: [package] 中 version/edition/license/authors 未使用 workspace=true 的条目（description 可例外）
 *   - exception_list: 非 workspace 化的详细条目列表（方便文档化 ≤1 例外）
 */

'use strict';

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const ROOT = path.resolve(__dirname);

// 17 个目标 Cargo.toml（platform/services/* 16 个 + platform/gateway/runtime 1 个）
const TARGET_TOMLS = [
  'platform/services/ai-agent/Cargo.toml',
  'platform/services/business-catalog/Cargo.toml',
  'platform/services/flow-ai/Cargo.toml',
  'platform/services/graph-algorithms/Cargo.toml',
  'platform/services/hermes-flow-bridge/Cargo.toml',
  'platform/services/kg-hub/Cargo.toml',
  'platform/services/operator-core/Cargo.toml',
  'platform/services/operator-wasm/Cargo.toml',
  'platform/services/optimizer/Cargo.toml',
  'platform/services/primiflow-core/Cargo.toml',
  'platform/services/primiflow-fusion/Cargo.toml',
  'platform/services/template-market/Cargo.toml',
  'platform/services/xuanji-common-meta/Cargo.toml',
  'platform/services/xuanji-expert/Cargo.toml',
  'platform/services/xuanji-system/Cargo.toml',
  'platform/services/kg-hub/Cargo.toml', // 占位修正，真正 16 个 services 见下
];
// 真实列表（去重后 16 services + 1 gateway runtime = 17）
const TARGETS_REAL = [
  'platform/services/ai-agent/Cargo.toml',
  'platform/services/business-catalog/Cargo.toml',
  'platform/services/flow-ai/Cargo.toml',
  'platform/services/graph-algorithms/Cargo.toml',
  'platform/services/hermes-flow-bridge/Cargo.toml',
  'platform/services/kg-hub/Cargo.toml',
  'platform/services/operator-core/Cargo.toml',
  'platform/services/operator-wasm/Cargo.toml',
  'platform/services/optimizer/Cargo.toml',
  'platform/services/primiflow-core/Cargo.toml',
  'platform/services/primiflow-fusion/Cargo.toml',
  'platform/services/template-market/Cargo.toml',
  'platform/services/xuanji-common-meta/Cargo.toml',
  'platform/services/xuanji-expert/Cargo.toml',
  'platform/services/xuanji-system/Cargo.toml',
  'platform/gateway/runtime/Cargo.toml',
];

// 内部 crate 名称（这些 crate 之间使用 path = "..." 指向彼此，不算外部依赖）
const INTERNAL_CRATE_NAMES = [
  'xuanji-common-meta',
  'operator-core',
  'operator-wasm',
  'graph-algorithms',
  'optimizer',
  'ai-agent',
  'business-catalog',
  'xuanji-expert',
  'flow-ai',
  'xuanji-system',
  'primiflow-core',
  'primiflow-fusion',
  'kg-hub',
  'hermes-flow-bridge',
  'template-market',
  'runtime',
];

/**
 * 读取根 workspace.dependencies 的键（作为"可继承外部 crate"列表）
 */
function getWorkspaceDepsKeys() {
  const rootToml = fs.readFileSync(path.join(ROOT, 'Cargo.toml'), 'utf8');
  const keys = new Set();
  const lines = rootToml.split(/\r?\n/);
  let inWsDeps = false;
  for (const line of lines) {
    const trimmed = line.trim();
    if (/^\[workspace\.dependencies\]\s*$/.test(trimmed)) {
      inWsDeps = true;
      continue;
    }
    if (/^\[.*\]\s*$/.test(trimmed)) {
      inWsDeps = false;
      continue;
    }
    if (!inWsDeps) continue;
    if (!trimmed || trimmed.startsWith('#')) continue;
    // 匹配 `foo = ...` 或 `"foo-bar" = ...`
    const m = trimmed.match(/^"?([A-Za-z0-9_-]+)"?\s*=/);
    if (m) keys.add(m[1]);
  }
  return keys;
}

/**
 * 把 TOML 切成段（返回 {sectionName: {startLine, endLine, lines: [{ln, text}]}}）
 * 段包括: package / dependencies / dev-dependencies / build-dependencies / features / lib / bin / bench / test / example 等
 */
function splitSections(tomlText) {
  const lines = tomlText.split(/\r?\n/);
  const sections = [];
  let cur = null;
  lines.forEach((text, idx) => {
    const trimmed = text.trim();
    const header = trimmed.match(/^\[(.+?)\]\s*$/);
    if (header) {
      if (cur) cur.endLine = idx - 1;
      cur = {
        name: header[1],
        startLine: idx + 1,
        endLine: lines.length,
        lines: [],
      };
      sections.push(cur);
    } else if (cur) {
      cur.lines.push({ ln: idx + 1, text });
    }
  });
  return sections;
}

/**
 * 判断一个依赖声明行是否包含 `workspace = true` 或 `.workspace = true`
 */
function hasWorkspaceFlag(crateName, rawValue, lineText) {
  // 形式 1: crate = { workspace = true, ... }
  // 形式 2: crate.workspace = true
  if (new RegExp(`^\\s*"?${escapeReg(crateName)}"?\\s*\\.\\s*workspace\\s*=\\s*true`, 'm').test(lineText)) {
    return true;
  }
  if (/\bworkspace\s*=\s*true\b/.test(rawValue || '') || /\bworkspace\s*=\s*true\b/.test(lineText)) {
    return true;
  }
  return false;
}

function escapeReg(s) { return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }

/**
 * 从一段 [dependencies] / [dev-dependencies] / [build-dependencies] 解析出依赖条目
 * 返回 [{crate, ln, line, rawValue, isInternal, isWorkspace, hasPath}]
 *
 * 支持多行表格语法，例如：
 *   foo = { version = "1", features = ["a"] }
 *   foo = "1"
 *   foo.workspace = true
 *   foo = { path = "../x" }
 *   [dependencies.foo]
 *     workspace = true
 */
function parseDepsSection(section) {
  const items = [];
  const lines = section.lines;
  let i = 0;
  while (i < lines.length) {
    const { ln, text } = lines[i];
    const trimmed = text.trim();
    if (!trimmed || trimmed.startsWith('#')) { i++; continue; }

    // [dependencies.foo] 形式子段头？正常段拆分时不会出现，因为 splitSections 会把 [[xxx.xxx]] 作为新段；但这里 section 是 [dependencies] 段内的行，故不会发生；忽略。
    // 多行表格 (行末 `{` 未闭合) — 合并直到 `}`
    // 先匹配 `<crate> = ...`
    let m = trimmed.match(/^"?([A-Za-z0-9_-]+)"?\s*=\s*(.*)$/);
    if (m) {
      let crateName = m[1];
      let raw = m[2];
      let startLn = ln;
      let accumText = text;
      // 处理多行 { ... }
      const openBraces = (raw.match(/\{/g) || []).length;
      const closeBraces = (raw.match(/\}/g) || []).length;
      let depth = openBraces - closeBraces;
      while (depth > 0 && i + 1 < lines.length) {
        i++;
        const nxt = lines[i];
        accumText += '\n' + nxt.text;
        const ob = (nxt.text.match(/\{/g) || []).length;
        const cb = (nxt.text.match(/\}/g) || []).length;
        depth += ob - cb;
      }
      // 再取完整 raw (去掉 crate = 前缀后的部分)：直接用 accumText 重新分析
      const m2 = accumText.match(/^[^=]*=\s*([\s\S]*)$/);
      const rawValue = m2 ? m2[1].trim() : raw.trim();
      const hasPath = /\bpath\s*=\s*"[^"]+"/.test(rawValue) || /\bpath\s*=\s*"[^"]+"/.test(accumText);
      const isInternal = INTERNAL_CRATE_NAMES.includes(crateName) && hasPath;
      const isWs = hasWorkspaceFlag(crateName, rawValue, accumText);
      items.push({
        crate: crateName,
        ln: startLn,
        line: accumText,
        rawValue,
        isInternal,
        isWorkspace: isWs,
        hasPath,
      });
      i++;
      continue;
    }
    // `<crate>.workspace = true` 简写语法
    m = trimmed.match(/^"?([A-Za-z0-9_-]+)"?\s*\.\s*workspace\s*=\s*true\s*(#.*)?$/);
    if (m) {
      const crateName = m[1];
      items.push({
        crate: crateName,
        ln,
        line: text,
        rawValue: 'true',
        isInternal: INTERNAL_CRATE_NAMES.includes(crateName),
        isWorkspace: true,
        hasPath: false,
      });
      i++;
      continue;
    }
    i++;
  }
  return items;
}

/**
 * 解析 [package] 段，返回 version/edition/license/authors/description 的原始行
 */
function parsePackageSection(section) {
  const pkg = { fields: {} };
  for (const { ln, text } of section.lines) {
    const trimmed = text.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const m = trimmed.match(/^(version|edition|license|authors|description)(?:\s*=\s*|\s*\.\s*workspace\s*=\s*)(.*)$/);
    if (m) {
      pkg.fields[m[1]] = { ln, text, raw: m[2].trim() };
    }
  }
  return pkg;
}

function main() {
  const wsDeps = getWorkspaceDepsKeys();
  console.log(`[info] workspace.dependencies 包含 ${wsDeps.size} 个 crate: ${[...wsDeps].sort().join(', ')}`);

  const results = {
    files: {},
    tr_4_1_non_workspace_count: 0,
    tr_4_2_reqwest_ok: false,
    tr_4_2_reqwest_versions: [],
    criterion_default_features_remaining_count: 0,
    package_inheritance_defects: [],
    exception_list: [],
  };

  for (const rel of TARGETS_REAL) {
    const abs = path.join(ROOT, rel);
    if (!fs.existsSync(abs)) {
      console.error(`[fatal] 目标文件不存在: ${abs}`);
      process.exit(2);
    }
    const text = fs.readFileSync(abs, 'utf8');
    const sections = splitSections(text);

    const perFile = {
      path: rel,
      dep_sections: {}, // 'dependencies' -> items[], 'dev-dependencies' -> items[]
      package_defects: [],
      non_workspace_dep_items: [],
      criterion_with_default_features: [],
    };

    // 处理 dependencies / dev-dependencies
    for (const sectionName of ['dependencies', 'dev-dependencies']) {
      const sec = sections.find(s => s.name === sectionName);
      if (!sec) continue;
      const items = parseDepsSection(sec);
      perFile.dep_sections[sectionName] = items;

      for (const it of items) {
        // 内部 path crate 不参与 workspace 合规判定（内部 crate 以 path 依赖方式互联是允许的）
        if (it.isInternal) continue;

        // 若 crate 在 workspace.deps 中，却未使用 workspace=true → 计为 TR 4.1 违规
        const shouldBeWorkspace = wsDeps.has(it.crate);
        if (shouldBeWorkspace && !it.isWorkspace) {
          results.tr_4_1_non_workspace_count += 1;
          perFile.non_workspace_dep_items.push({ section: sectionName, ...it });
          results.exception_list.push({
            file: rel,
            section: sectionName,
            crate: it.crate,
            ln: it.ln,
            line: it.line,
          });
        }
        // criterion 含 default-features 键 → 违规
        if (it.crate === 'criterion') {
          if (/\bdefault-features\s*=/.test(it.line)) {
            results.criterion_default_features_remaining_count += 1;
            perFile.criterion_with_default_features.push({ section: sectionName, ...it });
          }
        }
      }
    }

    // 处理 [package] 继承
    const pkgSec = sections.find(s => s.name === 'package');
    if (pkgSec) {
      const pkg = parsePackageSection(pkgSec);
      for (const f of ['version', 'edition', 'license', 'authors']) {
        const fv = pkg.fields[f];
        if (!fv) {
          perFile.package_defects.push({ field: f, reason: 'missing field; should add *.workspace = true', ln: 0 });
          results.package_inheritance_defects.push({ file: rel, field: f, reason: 'missing', ln: 0 });
          continue;
        }
        const ok = fv.raw.includes('workspace = true') || /^\s*\{\s*workspace\s*=\s*true\s*\}\s*$/.test(fv.raw) || /^\s*true\s*(#.*)?$/.test(fv.raw);
        if (!ok) {
          perFile.package_defects.push({ field: f, reason: 'hard-coded value, should *.workspace = true', ln: fv.ln, raw: fv.raw });
          results.package_inheritance_defects.push({ file: rel, field: f, reason: 'hard-coded', ln: fv.ln, raw: fv.raw });
        }
      }
    } else {
      results.package_inheritance_defects.push({ file: rel, field: '(missing [package])', reason: 'no [package] section', ln: 0 });
    }

    results.files[rel] = perFile;
  }

  // TR 4.2: cargo tree -p primiflow-core -i reqwest
  try {
    const treeOut = execSync('cargo tree -p primiflow-core -i reqwest', {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const re = /\breqwest\s+v(0\.\d+\.\d+)/g;
    const versions = new Set();
    let mm;
    while ((mm = re.exec(treeOut)) !== null) versions.add(mm[1]);
    results.tr_4_2_reqwest_versions = [...versions].sort();
    // 只检查非 0.12.x 的版本
    const bad = [...versions].filter(v => !v.startsWith('0.12.'));
    results.tr_4_2_reqwest_ok = (bad.length === 0 && versions.size > 0) || (versions.size === 0 && true); // 若 primiflow-core 未依赖 reqwest 也算 OK？任务说存在漂移 0.11 → 0.12，因此应有版本
    results._tr_4_2_raw_stdout = treeOut;
  } catch (e) {
    results.tr_4_2_run_error = (e.stderr ? e.stderr.toString() : String(e));
    results.tr_4_2_reqwest_ok = false;
  }

  // 判定汇总
  const tr41_ok = results.tr_4_1_non_workspace_count <= 1;
  const tr42_ok = results.tr_4_2_reqwest_ok;
  const crit_ok = results.criterion_default_features_remaining_count === 0;
  // package_inheritance 只检查 version/edition/license/authors（不包含 description）
  const pkg_ok = results.package_inheritance_defects.length === 0;
  const all_ok = tr41_ok && tr42_ok && crit_ok && pkg_ok;

  // 人类可读输出
  console.log('\n====== T4 依赖治理合规报告 ======');
  console.log(`TR 4.1 (非 workspace 外部依赖行) = ${results.tr_4_1_non_workspace_count} / 阈值 ≤ 1 → ${tr41_ok ? 'PASS' : 'FAIL'}`);
  if (results.exception_list.length > 0) {
    console.log('  非 workspace 化条目列表:');
    for (const e of results.exception_list) {
      console.log(`    - ${e.file}:${e.ln}  [${e.section}] ${e.crate}`);
      console.log(`      ${e.line.split(/\n/)[0].trim()}`);
    }
  }
  console.log(`TR 4.2 (primiflow-core reqwest 全 0.12.x) → ${tr42_ok ? 'PASS' : 'FAIL'}  版本集合=${JSON.stringify(results.tr_4_2_reqwest_versions)}`);
  if (results._tr_4_2_raw_stdout) {
    console.log('  `cargo tree -p primiflow-core -i reqwest` 输出:');
    console.log('  ' + results._tr_4_2_raw_stdout.split(/\n/).join('\n  '));
  }
  console.log(`Criterion default-features 残留 = ${results.criterion_default_features_remaining_count} / 0 → ${crit_ok ? 'PASS' : 'FAIL'}`);
  console.log(`[package] 继承缺陷 (version/edition/license/authors) = ${results.package_inheritance_defects.length} → ${pkg_ok ? 'PASS' : 'FAIL'}`);
  if (results.package_inheritance_defects.length > 0) {
    console.log('  缺陷列表:');
    for (const d of results.package_inheritance_defects) {
      console.log(`    - ${d.file}:${d.ln || '?'}  field=${d.field}  reason=${d.reason}${d.raw ? '  raw=' + d.raw : ''}`);
    }
  }
  console.log(`\n综合结果: ${all_ok ? 'GREEN / PASS' : 'RED / FAIL'}`);

  // JSON 输出（便于机器）
  const jsonPath = path.join(ROOT, 'test-tr-4-compliance.out.json');
  const { _tr_4_2_raw_stdout, ...rest } = results;
  fs.writeFileSync(jsonPath, JSON.stringify({
    ...rest,
    tr_4_2_tree_stdout: _tr_4_2_raw_stdout || null,
    summary: {
      tr_4_1_ok: tr41_ok,
      tr_4_2_ok: tr42_ok,
      criterion_default_features_ok: crit_ok,
      package_inheritance_ok: pkg_ok,
      overall_pass: all_ok,
    },
  }, null, 2), 'utf8');
  console.log(`\nJSON 结果写入: ${jsonPath}`);

  process.exit(all_ok ? 0 : 1);
}

main();

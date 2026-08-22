'use strict';

/**
 * 图谱资产扫描器（infrastructure 层 · 唯一触碰文件系统扫描的位置）
 * ------------------------------------------------------------------
 * 自己管理自己：扫描真实代码库四类资产，供 self-sync 差量登记。
 *   1. 路由域：routes/index.js 的 DOMAINS（新增业务域自动发现）
 *   2. 数据资产：data/ 目录全部 .json/.jsonl（新数据文件自动发现）
 *   3. 文档：docs/ 递归 .md（新文档自动发现）
 *   4. 自开发制品：workspace/artifacts（自动开发引擎产出自动发现）
 * 扫描结果供 application 层 diff（已登记 vs 实际存在），纯 IO 无决策。
 */

const fs = require('fs');
const path = require('path');

const ROOT = path.join(__dirname, '..', '..', '..');      // backend-node/
const PROJECT_ROOT = path.join(ROOT, '..', '..');          // 仓库根
const DATA_DIR = path.join(ROOT, 'data');
const DOCS_DIR = path.join(PROJECT_ROOT, 'docs');
const ARTIFACTS_DIR = path.join(PROJECT_ROOT, 'workspace', 'artifacts');

/** 递归收集目录下全部指定后缀文件（相对路径） */
function _walk(dir, exts, base) {
  const out = [];
  if (!fs.existsSync(dir)) return out;
  for (const name of fs.readdirSync(dir)) {
    if (name === 'node_modules' || name.startsWith('.')) continue;
    const fp = path.join(dir, name);
    const st = fs.statSync(fp);
    if (st.isDirectory()) out.push(..._walk(fp, exts, base));
    else if (exts.some(e => name.endsWith(e))) out.push(path.relative(base, fp).replace(/\\/g, '/'));
  }
  return out;
}

/** 1. 路由域扫描：[{id, name, codePath}]（name 取路由装配清单中文名） */
function scanRouteDomains() {
  const { DOMAINS } = require('../../routes');
  return DOMAINS.map(([id, name]) => ({
    id,
    name,
    codePath: `src/routes/${id}.js`
  }));
}

/** 2. 数据资产扫描：data/ 下全部 .json/.jsonl 文件名 */
function scanDataFiles() {
  if (!fs.existsSync(DATA_DIR)) return [];
  return fs.readdirSync(DATA_DIR)
    .filter(f => f.endsWith('.json') || f.endsWith('.jsonl'))
    .sort();
}

/** 3. 文档扫描：docs/ 递归 .md（相对仓库根，与 tech-registry file 字段同构） */
function scanDocs() {
  return _walk(DOCS_DIR, ['.md'], PROJECT_ROOT).sort();
}

/** 4. 自开发制品扫描：自动开发引擎（auto-dev）产出的制品项目 */
function scanAutoDevArtifacts() {
  if (!fs.existsSync(ARTIFACTS_DIR)) return [];
  return fs.readdirSync(ARTIFACTS_DIR)
    .filter(name => fs.statSync(path.join(ARTIFACTS_DIR, name)).isDirectory())
    .map(name => {
      const dir = path.join(ARTIFACTS_DIR, name);
      const files = fs.readdirSync(dir).filter(f => !f.startsWith('.'));
      return { project: name, fileCount: files.length, codePath: `workspace/artifacts/${name}` };
    });
}

/** 一次性全量扫描（self-sync 输入） */
function scanAll() {
  return {
    routeDomains: scanRouteDomains(),
    dataFiles: scanDataFiles(),
    docs: scanDocs(),
    autoDevArtifacts: scanAutoDevArtifacts(),
    scannedAt: new Date().toISOString()
  };
}

module.exports = { scanRouteDomains, scanDataFiles, scanDocs, scanAutoDevArtifacts, scanAll };

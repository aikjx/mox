'use strict';

/**
 * 项目全息图谱 · 代码实体抽取器（infrastructure 层 · 零依赖自研）
 * ------------------------------------------------------------------
 * "本地代码工程维度" 数据源：本地代码 → 结构化实体（函数/类/导出/路由/依赖）。
 * 全自研轻量 AST 替代（正则状态机，零外部依赖，行号定位）：
 *   .js/.mjs/.cjs  函数声明/箭头函数/类/类方法/导出/require 依赖/HTTP 路由注册
 *   .py            def/class/async def/import 依赖
 *   .rs            fn/struct/enum/trait/axum 路由/use 依赖（Rust 平台层）
 * 目录扫描：递归遍历，跳过 node_modules/.git/dist/build/data/.runtime/target。
 */

const fs = require('fs');
const path = require('path');

const SKIP_DIRS = new Set(['node_modules', '.git', 'dist', 'build', 'data', '.runtime', 'coverage', '__pycache__', 'target']);
const JS_EXT = new Set(['.js', '.mjs', '.cjs']);
const PY_EXT = new Set(['.py']);
const RS_EXT = new Set(['.rs']);
const MAX_FILE_BYTES = 2 * 1024 * 1024; // 护栏：跳过超大文件

// ============ JS 实体抽取（正则状态机，带行号） ============

/** 计算匹配位置的行号（1-based） */
function lineOf(source, index) {
  let line = 1;
  for (let i = 0; i < index && i < source.length; i++) {
    if (source.charCodeAt(i) === 10) line++;
  }
  return line;
}

// 控制流关键字误报过滤（类方法形态近似匹配 if/for/while/switch/catch）
const CONTROL_KEYWORDS = new Set(['if', 'for', 'while', 'switch', 'catch', 'return', 'function', 'typeof', 'else', 'do', 'try']);

/**
 * JS 单文件实体抽取：
 * { functions:[{name,line,kind}], classes:[{name,line}], exports:[{name,line}],
 *   routes:[{method,path,line}], requires:[{module,line}], comment }
 */
function extractJsEntities(source) {
  const src = String(source || '');
  const out = { functions: [], classes: [], exports: [], routes: [], requires: [] };
  const seenFn = new Set(), seenCls = new Set();

  // 函数声明：function name( / async function name( / function* name(
  let m;
  const fnDecl = /(?:^|\n)[ \t]*(?:export\s+)?(?:default\s+)?(?:async\s+)?function\s*\*?\s*([A-Za-z_$][\w$]*)\s*\(/g;
  while ((m = fnDecl.exec(src)) !== null) {
    if (m[1] === 'use') continue; // 'use strict' 误报护栏
    if (!seenFn.has(m[1])) { seenFn.add(m[1]); out.functions.push({ name: m[1], line: lineOf(src, m.index), kind: 'declaration' }); }
  }

  // 箭头函数/函数表达式：const name = (...) => / const name = function(
  const fnExpr = /(?:^|\n)[ \t]*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:async\s*)?(?:function\b|\([^)]*\)\s*=>|[A-Za-z_$][\w$]*\s*=>)/g;
  while ((m = fnExpr.exec(src)) !== null) {
    if (!seenFn.has(m[1])) { seenFn.add(m[1]); out.functions.push({ name: m[1], line: lineOf(src, m.index), kind: 'expression' }); }
  }

  // 类声明
  const clsDecl = /(?:^|\n)[ \t]*(?:export\s+)?(?:default\s+)?class\s+([A-Za-z_$][\w$]*)/g;
  while ((m = clsDecl.exec(src)) !== null) {
    if (!seenCls.has(m[1])) { seenCls.add(m[1]); out.classes.push({ name: m[1], line: lineOf(src, m.index) }); }
  }

  // 类方法（近似形态：缩进两格+ 标识符(  且非控制流关键字）
  const methodDecl = /(?:^|\n)([ \t]{2,})(?:async\s+)?([A-Za-z_$][\w$]*)\s*\([^()]*\)\s*\{/g;
  while ((m = methodDecl.exec(src)) !== null) {
    const name = m[2];
    if (CONTROL_KEYWORDS.has(name)) continue;
    if (seenFn.has(name)) continue;
    seenFn.add(name);
    out.functions.push({ name, line: lineOf(src, m.index + m[1].length), kind: 'method' });
  }

  // 路由注册：reg('get', '/path'
  const routeDecl = /\breg\(\s*'(get|post|put|delete|patch)'\s*,\s*'([^']+)'/g;
  while ((m = routeDecl.exec(src)) !== null) {
    out.routes.push({ method: m[1].toUpperCase(), path: m[2], line: lineOf(src, m.index) });
  }

  // require 依赖
  const reqDecl = /require\(\s*['"]([^'"]+)['"]\s*\)/g;
  while ((m = reqDecl.exec(src)) !== null) {
    out.requires.push({ module: m[1], line: lineOf(src, m.index) });
  }

  // 导出：module.exports = { a, b } / exports.name = / module.exports = function name
  const me = /module\.exports\s*=\s*\{([^}]*)\}/.exec(src);
  if (me) {
    const names = me[1].split(',').map(s => s.trim().split(/[:\s]+/)[0]).filter(n => /^[A-Za-z_$][\w$]*$/.test(n));
    const line = lineOf(src, me.index);
    names.forEach(n => out.exports.push({ name: n, line }));
  }
  const exDecl = /(?:^|\n)[ \t]*exports\.([A-Za-z_$][\w$]*)\s*=/g;
  while ((m = exDecl.exec(src)) !== null) out.exports.push({ name: m[1], line: lineOf(src, m.index) });
  const meFn = /module\.exports\s*=\s*(?:async\s+)?function\s+([A-Za-z_$][\w$]*)/.exec(src);
  if (meFn) out.exports.push({ name: meFn[1], line: lineOf(src, meFn.index) });

  return out;
}

// ============ Python 实体抽取 ============

function extractPyEntities(source) {
  const src = String(source || '');
  const out = { functions: [], classes: [], exports: [], routes: [], requires: [] };
  let m;

  const fnDecl = /(?:^|\n)([ \t]*)(?:async\s+)?def\s+([A-Za-z_][\w]*)\s*\(/g;
  while ((m = fnDecl.exec(src)) !== null) {
    out.functions.push({ name: m[2], line: lineOf(src, m.index + m[1].length), kind: m[1].length === 0 ? 'declaration' : 'method' });
  }
  const clsDecl = /(?:^|\n)([ \t]*)class\s+([A-Za-z_][\w]*)/g;
  while ((m = clsDecl.exec(src)) !== null) {
    out.classes.push({ name: m[2], line: lineOf(src, m.index + m[1].length) });
  }
  const impDecl = /(?:^|\n)[ \t]*(?:from\s+([\w.]+)\s+import|import\s+([\w.]+))/g;
  while ((m = impDecl.exec(src)) !== null) {
    out.requires.push({ module: m[1] || m[2], line: lineOf(src, m.index) });
  }
  return out;
}

// ============ Rust 实体抽取 ============

/**
 * Rust 单文件实体抽取（Rust 平台层：网关运行时 / ai-agent 服务）：
 *   函数（fn / pub fn / pub async fn，impl 内缩进方法区分 kind）
 *   类型（struct / enum / trait → classes 桶）
 *   路由（axum 风格 .route("/path", get/post/...)）
 *   依赖（use 声明 → requires 桶）
 */
function extractRsEntities(source) {
  const src = String(source || '');
  const out = { functions: [], classes: [], exports: [], routes: [], requires: [] };
  const seenFn = new Set();
  let m;

  const fnDecl = /(?:^|\n)([ \t]*)(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]*"\s+)?fn\s+([A-Za-z_][\w]*)\s*[<(]/g;
  while ((m = fnDecl.exec(src)) !== null) {
    if (seenFn.has(m[2])) continue;
    seenFn.add(m[2]);
    out.functions.push({ name: m[2], line: lineOf(src, m.index + m[1].length), kind: m[1].length === 0 ? 'declaration' : 'method' });
  }

  const typeDecl = /(?:^|\n)[ \t]*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|trait)\s+([A-Za-z_][\w]*)/g;
  while ((m = typeDecl.exec(src)) !== null) {
    out.classes.push({ name: m[1], line: lineOf(src, m.index) });
  }

  const routeDecl = /\.route\(\s*"([^"]+)"\s*,\s*(get|post|put|delete|patch)/g;
  while ((m = routeDecl.exec(src)) !== null) {
    out.routes.push({ method: m[2].toUpperCase(), path: m[1], line: lineOf(src, m.index) });
  }

  const useDecl = /(?:^|\n)[ \t]*(?:pub\s+)?use\s+([\w:]+)/g;
  while ((m = useDecl.exec(src)) !== null) {
    out.requires.push({ module: m[1], line: lineOf(src, m.index) });
  }
  return out;
}

// ============ 语言分发与文件/目录扫描 ============

/** 单文件实体抽取（按扩展名分发） */
function scanFile(absPath) {
  const ext = path.extname(absPath).toLowerCase();
  let language = null, entities = null;
  if (JS_EXT.has(ext)) { language = 'javascript'; entities = extractJsEntities(readSafe(absPath)); }
  else if (PY_EXT.has(ext)) { language = 'python'; entities = extractPyEntities(readSafe(absPath)); }
  else if (RS_EXT.has(ext)) { language = 'rust'; entities = extractRsEntities(readSafe(absPath)); }
  else return null;
  const stat = fs.existsSync(absPath) ? fs.statSync(absPath) : null;
  return {
    file: absPath, language,
    lineCount: stat ? Math.max(1, Math.round(stat.size / 40)) : 0, // 近似行数（避免大文件二次全读）
    sizeBytes: stat ? stat.size : 0,
    ...entities,
    total: countEntities(entities)
  };
}

function readSafe(absPath) {
  try {
    const stat = fs.statSync(absPath);
    if (stat.size > MAX_FILE_BYTES) return '';
    return fs.readFileSync(absPath, 'utf8');
  } catch (e) {
    return '';
  }
}

/** 目录递归扫描（文件级实体聚合；深度护栏 8 层，文件数护栏 500） */
function scanDirectory(absDir, acc = { files: [], depth: 0 }) {
  if (acc.depth > 8 || acc.files.length > 500) return acc;
  let entries = [];
  try { entries = fs.readdirSync(absDir, { withFileTypes: true }); } catch (e) { return acc; }
  for (const entry of entries) {
    if (entry.name.startsWith('.') || SKIP_DIRS.has(entry.name)) continue;
    const full = path.join(absDir, entry.name);
    if (entry.isDirectory()) {
      scanDirectory(full, { ...acc, depth: acc.depth + 1 });
    } else if (JS_EXT.has(path.extname(entry.name).toLowerCase()) || PY_EXT.has(path.extname(entry.name).toLowerCase()) || RS_EXT.has(path.extname(entry.name).toLowerCase())) {
      const scanned = scanFile(full);
      if (scanned) acc.files.push(scanned);
    }
  }
  return acc;
}

/** 实体总数 */
function countEntities(e) {
  if (!e) return 0;
  return e.functions.length + e.classes.length + e.exports.length + e.routes.length;
}

/**
 * 扫描路径（文件或目录）→ 聚合实体报告
 * 返回 { files:[scanFile 结果], totals:{...}, scannedAt }
 */
function scanPath(absPath) {
  if (!fs.existsSync(absPath)) return { exists: false, files: [], totals: zeroTotals(), scannedAt: new Date().toISOString() };
  const stat = fs.statSync(absPath);
  let files = [];
  if (stat.isFile()) {
    const scanned = scanFile(absPath);
    if (scanned) files.push(scanned);
  } else if (stat.isDirectory()) {
    files = scanDirectory(absPath).files;
  }
  const totals = files.reduce((t, f) => ({
    files: t.files + 1,
    functions: t.functions + f.functions.length,
    classes: t.classes + f.classes.length,
    exports: t.exports + f.exports.length,
    routes: t.routes + f.routes.length,
    requires: t.requires + f.requires.length,
    entities: t.entities + f.total
  }), zeroTotals());
  return { exists: true, files, totals, scannedAt: new Date().toISOString() };
}

function zeroTotals() {
  return { files: 0, functions: 0, classes: 0, exports: 0, routes: 0, requires: 0, entities: 0 };
}

module.exports = { extractJsEntities, extractPyEntities, scanFile, scanDirectory, scanPath };

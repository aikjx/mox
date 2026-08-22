#!/usr/bin/env node
'use strict';

/**
 * AINA 架构门禁（AINA-STD-001 §7 · A5 机器可验证公理）
 * ------------------------------------------------------------------
 * 5 组断言（详见 docs/standards/ai-native-architecture-standard.md）：
 *   G1 依赖方向：domain 零引擎/IO 依赖；infrastructure 不引 application；
 *               routes 只引域门面/共享库，不直连域包 infrastructure
 *   G2 尺寸上限：domain/infrastructure/application 单文件 ≤400 行；
 *               routes 单文件 ≤500 行；组合根 api-server.js ≤300 行
 *   G3 真相源唯一：pagerank 仅 lib/graph-algos.js 定义；
 *               INTENT_PATTERNS 仅 expert-alliance/domain/intent-patterns.js 定义
 *   G4 无环依赖：顶层 require 静态图无环（函数体内惰性 require 记为延迟边，不违规）
 *   G5 注册完备：routes/index.js DOMAINS 表与 routes/*.js 文件集合一致；
 *               每个域包必有 index.js 门面
 *
 * 用法：node scripts/architecture-guard.js   （退出码 0=全绿，1=有违规）
 */

const fs = require('fs');
const path = require('path');

const SRC = path.join(__dirname, '..', 'src');
const ENGINES = [
  'llm-gateway', 'ai-engine', 'ai-integration-engine', 'ultimate-ai-engine',
  'orchestration-engine', 'expert-dispatcher', 'session-store', 'web-search-service',
  'infinite-dimension-optimizer', 'local-artifact-service', 'ai-engine-core',
  'auto-dev-engine', 'security', 'service-manager', 'nebulagraph-adapter',
  'ai-flow-graph', 'file-store', 'expert-graph'
];

const violations = [];
function fail(gate, msg) { violations.push(`[${gate}] ${msg}`); }
function passLog(gate, msg) { console.log(`  [PASS] ${gate} ${msg}`); }

function listJsFiles(dir, filter = () => true) {
  const out = [];
  (function walk(d) {
    fs.readdirSync(d, { withFileTypes: true }).forEach((ent) => {
      const fp = path.join(d, ent.name);
      if (ent.isDirectory()) walk(fp);
      else if (ent.name.endsWith('.js') && filter(fp)) out.push(fp);
    });
  })(dir);
  return out;
}

/** 解析一个文件的所有相对 require：返回 { target, lazy }，lazy=函数体内惰性 require */
function parseRequires(file) {
  const src = fs.readFileSync(file, 'utf8');
  const lines = src.split('\n');
  const reqs = [];
  const re = /require\(\s*['"](\.[^'"]+)['"]\s*\)/g;
  lines.forEach((line, i) => {
    if (/^\s*\/\//.test(line)) return; // 注释行
    let m;
    re.lastIndex = 0;
    while ((m = re.exec(line)) !== null) {
      const lazy = /^[ \t]+/.test(line); // 有缩进 → 函数体内惰性 require
      let target = m[1];
      // 解析为绝对路径（补 index.js）
      const abs = path.resolve(path.dirname(file), target);
      let resolved = null;
      if (fs.existsSync(abs) && fs.statSync(abs).isFile()) resolved = abs;
      else if (fs.existsSync(abs + '.js')) resolved = abs + '.js';
      else if (fs.existsSync(path.join(abs, 'index.js'))) resolved = path.join(abs, 'index.js');
      reqs.push({ raw: target, resolved, lazy, line: i + 1 });
    }
  });
  return reqs;
}

const rel = (fp) => path.relative(SRC, fp).replace(/\\/g, '/');

// ============ G1 依赖方向 ============
function gateG1() {
  const domainFiles = listJsFiles(path.join(SRC, 'expert-alliance', 'domain'));
  for (const f of domainFiles) {
    for (const r of parseRequires(f)) {
      const t = rel(r.resolved || '');
      if (r.raw.includes('infrastructure') || r.raw.includes('application')) {
        fail('G1', `domain 文件 ${rel(f)} 引用了 ${r.raw}（domain 禁止依赖 infrastructure/application）`);
      }
      if (ENGINES.some((e) => t.startsWith(e)) || r.raw.includes('../')) {
        fail('G1', `domain 文件 ${rel(f)} 跨出域包引用 ${r.raw}（domain 只允许域内依赖）`);
      }
    }
  }
  passLog('G1', `domain 层 ${domainFiles.length} 个文件依赖方向合法`);

  const infraFiles = listJsFiles(path.join(SRC, 'expert-alliance', 'infrastructure'));
  for (const f of infraFiles) {
    for (const r of parseRequires(f)) {
      if (r.raw.includes('application')) {
        fail('G1', `infrastructure 文件 ${rel(f)} 引用了 ${r.raw}（infrastructure 禁止依赖 application）`);
      }
    }
  }
  passLog('G1', `infrastructure 层 ${infraFiles.length} 个文件依赖方向合法`);

  const routeFiles = listJsFiles(path.join(SRC, 'routes'), (fp) => path.basename(fp) !== 'index.js');
  for (const f of routeFiles) {
    for (const r of parseRequires(f)) {
      if (r.resolved && rel(r.resolved).includes('infrastructure')) {
        fail('G1', `路由 ${rel(f)} 直连域包 infrastructure（${r.raw}），应只引域门面`);
      }
    }
  }
  passLog('G1', `routes 层 ${routeFiles.length} 个文件无越层直连`);
}

// ============ G2 尺寸上限 ============
function gateG2() {
  const limits = [
    [path.join(SRC, 'expert-alliance', 'domain'), 400, 'domain'],
    [path.join(SRC, 'expert-alliance', 'infrastructure'), 400, 'infrastructure'],
    [path.join(SRC, 'expert-alliance', 'application'), 400, 'application'],
    [path.join(SRC, 'routes'), 500, 'routes']
  ];
  let count = 0;
  for (const [dir, max, label] of limits) {
    if (!fs.existsSync(dir)) continue;
    for (const f of listJsFiles(dir)) {
      count++;
      const lines = fs.readFileSync(f, 'utf8').split('\n').length;
      if (lines > max) fail('G2', `${label} 文件 ${rel(f)} ${lines} 行超上限 ${max}`);
    }
  }
  const apiServer = path.join(SRC, 'api-server.js');
  const asLines = fs.readFileSync(apiServer, 'utf8').split('\n').length;
  if (asLines > 300) fail('G2', `组合根 api-server.js ${asLines} 行超上限 300`);
  passLog('G2', `尺寸上限检查完成（${count} 个分层文件 + 组合根 ${asLines} 行）`);
}

// ============ G3 真相源唯一 ============
function gateG3() {
  const all = listJsFiles(SRC);
  const defRe = {
    pagerank: /function\s+pagerank\s*\(/,
    INTENT_PATTERNS: /(?:const|let|var)\s+INTENT_PATTERNS\s*=/
  };
  const allowedHome = {
    pagerank: ['lib/graph-algos.js'],
    INTENT_PATTERNS: ['expert-alliance/domain/intent-patterns.js']
  };
  for (const f of all) {
    const src = fs.readFileSync(f, 'utf8');
    for (const [name, re] of Object.entries(defRe)) {
      if (re.test(src)) {
        const r = rel(f);
        if (!allowedHome[name].includes(r)) {
          fail('G3', `${name} 在 ${r} 存在重复定义（唯一真相源：${allowedHome[name][0]}）`);
        }
      }
    }
  }
  passLog('G3', `真相源唯一性检查完成（pagerank / INTENT_PATTERNS 各仅 1 处定义）`);
}

// ============ G4 无环依赖（顶层 require 图） ============
function gateG4() {
  const all = listJsFiles(SRC);
  const graph = new Map();
  const lazyEdges = [];
  for (const f of all) {
    const edges = [];
    for (const r of parseRequires(f)) {
      if (!r.resolved) continue;
      if (r.lazy) lazyEdges.push(`${rel(f)} → ${rel(r.resolved)}（延迟边）`);
      else edges.push(r.resolved);
    }
    graph.set(f, edges);
  }
  // DFS 三色标记检测环
  const WHITE = 0, GRAY = 1, BLACK = 2;
  const color = new Map();
  let cyclic = false;
  function dfs(node, stack) {
    color.set(node, GRAY);
    for (const next of graph.get(node) || []) {
      if (!graph.has(next)) continue;
      const c = color.get(next) || WHITE;
      if (c === GRAY) {
        cyclic = true;
        fail('G4', `顶层 require 环：${[...stack, next].map(rel).join(' → ')}`);
      } else if (c === WHITE) {
        dfs(next, [...stack, next]);
      }
    }
    color.set(node, BLACK);
  }
  for (const f of graph.keys()) {
    if ((color.get(f) || WHITE) === WHITE) dfs(f, [f]);
  }
  if (!cyclic) passLog('G4', `顶层依赖图无环（${all.length} 个模块，${lazyEdges.length} 条已登记延迟边）`);
  return lazyEdges;
}

// ============ G5 注册完备 ============
function gateG5() {
  const routesDir = path.join(SRC, 'routes');
  const files = fs.readdirSync(routesDir)
    .filter((n) => n.endsWith('.js') && n !== 'index.js')
    .map((n) => n.replace(/\.js$/, ''))
    .sort();
  const manifest = fs.readFileSync(path.join(routesDir, 'index.js'), 'utf8');
  const registered = [...manifest.matchAll(/require\('\.\/([^']+)'\)/g)]
    .map((m) => m[1])
    .filter((n) => n !== 'index')
    .sort();
  const missing = files.filter((f) => !registered.includes(f));
  const orphan = registered.filter((r) => !files.includes(r));
  if (missing.length) fail('G5', `路由文件未在 DOMAINS 登记：${missing.join(', ')}`);
  if (orphan.length) fail('G5', `DOMAINS 登记了不存在的文件：${orphan.join(', ')}`);

  // 域包必有 index.js 门面（识别标准：含 domain/ 子目录）
  const srcDirs = fs.readdirSync(SRC, { withFileTypes: true })
    .filter((e) => e.isDirectory() && fs.existsSync(path.join(SRC, e.name, 'domain')))
    .map((e) => e.name);
  for (const pkg of srcDirs) {
    if (!fs.existsSync(path.join(SRC, pkg, 'index.js'))) {
      fail('G5', `域包 ${pkg}/ 缺少 index.js 门面`);
    }
  }
  passLog('G5', `注册完备：routes ${registered.length} 个域${srcDirs.length ? `，域包 ${srcDirs.join('/')}` : ''}`);
}

// ============ 主流程 ============
console.log('====== AINA 架构门禁（AINA-STD-001 §7）======');
gateG1();
gateG2();
gateG3();
const lazyEdges = gateG4();
gateG5();
console.log('');

if (violations.length) {
  console.log(`====== 门禁未通过：${violations.length} 项违规 ======`);
  violations.forEach((v) => console.log('  ' + v));
  process.exit(1);
} else {
  console.log('====== 门禁全绿（G1-G5 通过）======');
  if (lazyEdges.length) {
    console.log(`\n已登记延迟边（惰性 require，运行时解耦，非违规）：`);
    lazyEdges.forEach((e) => console.log('  ' + e));
  }
  process.exit(0);
}

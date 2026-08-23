'use strict';
/**
 * Task 7 · 全量代码防重复开发看门狗（企业级 C3 单一真源门禁）
 *
 * 对 backend-node / ai-agent（JS/Rust）源码里的 C3 归一化函数，执行：
 *  - 黑名单签名（detectIntent/degreeCentrality/betweennessCentrality/pagerank/apply_template）
 *      → 仅允许在 **已注册的单一真源文件** 里出现独立实现体（包含循环/分支的真实代码）；
 *      → 其他位置的 wrapper 体在展开去注释去空行后必须 ≤ MAX_WRAPPER_LINES。
 *  - 对每个注册的函数族，输出：真源文件 → wrapper 路径白名单 × N 的完整映射。
 *  - 任何 FAIL 设置 process.exitCode = 1，作为企业流水线门禁（T8 7-gate 之一）。
 */

const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const MAX_WRAPPER_LINES = 4;   // C3 硬约束：wrapper 体 ≤4 非空非注释行

// ======================================
// 注册：每一类函数的单一真源（single source of truth）
//   sig: 代码里识别该函数族的"函数头特征"（JS/Rust 通用，字符串数组做 OR 匹配）
//   truthFile: 该函数族唯一允许"包含真实算法代码（含 for/if/math 等）"的文件（相对 ROOT）
//   wrapperFiles[]: 该函数族允许以 thin wrapper 形式存在的外围转发文件（相对 ROOT）
//   lang: 'js' / 'rs' / 'both'
// ======================================
const REGISTRY = [
  {
    family: 'degreeCentrality（度中心性）',
    sigHeads: [
      'degreeCentrality(nodes, edges',                // JS function / method declaration
      'degreeCentrality: function degreeCentrality',  // Object.defineProperty value
    ],
    truthFile: 'src/graph/graph-formulas.js',
    wrapperFiles: [
      'src/lib/graph-algos.js',
      'src/ai-flow-graph.js',
    ],
    lang: 'js',
  },
  {
    family: 'betweennessCentrality（介数中心性 Brandes）',
    sigHeads: [
      'betweennessCentrality(nodes, edges',
      'betweennessCentrality: function betweennessCentrality',
    ],
    truthFile: 'src/graph/graph-formulas.js',
    wrapperFiles: [
      'src/lib/graph-algos.js',
      'src/ai-flow-graph.js',
    ],
    lang: 'js',
  },
  {
    family: 'pagerank（PageRank 迭代/dangling 分发）',
    sigHeads: [
      'pagerank(nodes, edges',
      'pagerank: function pagerank',
    ],
    truthFile: 'src/graph/graph-formulas.js',
    // 企业级 co_impl：Rust 主实现（KnowledgeGraph::pagerank / pagerank_personalized，PPR_D=0.85 / PPR_MAX_ITER=30）
    //   → Node graph-formulas.js 是权威转发（护栏：忽略传入参数，统一委托 Rust CLI 执行 call_rust_algo('ppr', …)）
    //   → graph-algos.js / ai-flow-graph.js 对外暴露 thin wrapper（≤4 行）
    // NOTE：Node 端 truth 体本身仅护栏+委托，不算独立算法实现；TR-7.1 通过下方 delegationFingerprint 指纹确认真实 Rust 调用链。
    delegationFingerprint: /call_rust_algo\(\s*['"]ppr['"]/,
    wrapperFiles: [
      'src/lib/graph-algos.js',
      'src/ai-flow-graph.js',
    ],
    lang: 'js',
  },
  {
    family: 'detectIntent（意图分类打分）',
    sigHeads: [
      'function detectIntent(',        // JS function declaration (domain layer canonical)
      'detectIntent(question) {',      // method/class member (ai-engine-core / expert-alliance-engine.classifyIntent callers)
      '_detectIntention(question) {',  // ai-integration-engine async 别名
    ],
    truthFile: 'src/expert-alliance/domain/intent-classifier.js',
    wrapperFiles: [
      'src/ai-engine-core.js',
      'src/ai-integration-engine.js',
    ],
    lang: 'js',
  },
  {
    family: 'apply_template（变量模板替换）',
    sigHeads: [
      'pub fn apply_template(',        // Rust flow_engine canonical
      'fn apply_template(input: &str, variables: &HashMap<String, serde_json::Value>) -> String', // workflow_engine private
    ],
    truthFile: '../services/ai-agent/src/flow_engine.rs',
    wrapperFiles: [
      '../services/ai-agent/src/workflow_engine.rs',
    ],
    lang: 'rs',
  },
];

// ============== 辅助 ==============
function walk(dir, ext) {
  const out = [];
  const stack = [dir];
  while (stack.length) {
    const cur = stack.pop();
    for (const de of fs.readdirSync(cur, { withFileTypes: true })) {
      if (de.name === 'node_modules' || de.name === '.git' || de.name === 'data' || de.name === 'dist' || de.name === 'target') continue;
      const full = path.join(cur, de.name);
      if (de.isDirectory()) stack.push(full);
      else if (de.isFile() && full.endsWith(ext)) out.push(full);
    }
  }
  return out;
}

/**
 * 从源码中抽出"从函数头开始的函数体"，按大括号计数。
 * 每个命中返回 { start, end, bodyText, headLine }
 * 过滤规则：命中不得出现在字符串字面量（前 1~10 个非空白字符为 `"` 或 `'` 或 `r"`）
 */
function extractBodies(src, heads) {
  const hits = [];
  for (const head of heads) {
    let pos = 0;
    while ((pos = src.indexOf(head, pos)) !== -1) {
      // 字符串字面量过滤：向前找到最近的非空白非标识符字符
      let pre = pos - 1;
      while (pre >= 0 && /\s/.test(src[pre])) pre--;
      const prevCh = pre >= 0 ? src.slice(Math.max(0, pre - 3), pre + 1) : '';
      // 若该位置前是引号（字符串常量）或 : "（赋值给字符串变量），则跳过（防止 test 里的 include_str! head 匹配）
      const insideString = /["']\s*$/.test(prevCh) || /r#?"$/.test(prevCh);
      if (insideString) { pos += head.length; continue; }

      const headLineStart = src.lastIndexOf('\n', pos) + 1;
      // ==============================================
      // 定位函数体首个 { 必须先跨过函数签名 ( ... )
      //   防止参数解构 {a=1,b=2} = {} 里的 { 被误识别为函数体开始
      // ==============================================
      let parenBal = 0;
      let sigScan = pos;
      // 先找到紧接 head 之后的第一个 '(' 作为函数签名开始
      while (sigScan < src.length && src[sigScan] !== '(') sigScan++;
      if (sigScan >= src.length) { pos += head.length; continue; }
      parenBal = 1; sigScan++;
      let inStrSig = null, prevSig = '';
      while (sigScan < src.length && parenBal > 0) {
        const c = src[sigScan];
        if (!inStrSig && (c === '"' || c === "'" || c === '`')) { inStrSig = c; sigScan++; prevSig = c; continue; }
        if (inStrSig && c === inStrSig && prevSig !== '\\') { inStrSig = null; sigScan++; prevSig = c; continue; }
        if (!inStrSig) {
          if (c === '(') parenBal++;
          else if (c === ')') parenBal--;
        }
        prevSig = c; sigScan++;
      }
      // 签名已完全闭合（parenBal == 0），从此位置向后找下一个 { 视为函数体开
      const openBrace = src.indexOf('{', sigScan);
      if (openBrace === -1) { pos += head.length; continue; }
      // 计数 {} 寻找匹配闭合
      let depth = 1;
      let i = openBrace + 1;
      let inStr = null, prev = '';
      while (i < src.length && depth > 0) {
        const c = src[i];
        // 基础字符串跳过（不处理 template literal / regex，足以覆盖 wrapper/算法代码）
        if (!inStr && (c === '"' || c === "'" || c === '`')) { inStr = c; i++; prev = c; continue; }
        if (inStr && c === inStr && prev !== '\\') { inStr = null; i++; prev = c; continue; }
        if (!inStr) {
          if (c === '{') depth++;
          else if (c === '}') depth--;
        }
        prev = c; i++;
      }
      const closeBrace = i - 1; // inclusive '}' index
      const bodyStart = openBrace + 1;
      const bodyEnd = closeBrace;
      const bodyText = src.slice(bodyStart, bodyEnd);
      const headText = src.slice(headLineStart, Math.min(src.indexOf('\n', openBrace), openBrace + 120));
      hits.push({ headPos: pos, openBrace, closeBrace, bodyText, headText });
      pos += head.length;
    }
  }
  return hits;
}

function countBodyNonEmpty(bodyText) {
  return bodyText.split('\n').map(l => l.trim()).filter(l => l && !l.startsWith('//') && !l.startsWith('*')).length;
}

function isLikelyIndependentImplementation(bodyText) {
  // 企业级启发：判断函数体是否为真实独立实现
  const n = countBodyNonEmpty(bodyText);
  const hasLoop = /\b(for|while)\s*\(/.test(bodyText) || /\.forEach\s*\(/.test(bodyText);
  const hasIfChain = /\bif\s*\([^)]*\)\s*\{/.test(bodyText) && (bodyText.match(/else\s+if/g) || []).length >= 1;
  const multiCalls = (bodyText.match(/\.\w+\s*\(/g) || []).length >= 4;
  const mathOrBigOps = /Math\.|(new Map|new Set|Set\(|Map\()/.test(bodyText);
  const bigBody = n >= 15;   // 大 body (15 行) 视为真实实现
  const brandesOrComplex = /Brandes|sigma\[w\]|preds\[w\]|dist\[w\]/.test(bodyText); // 介数 Brandes 特征指纹
  return hasLoop || hasIfChain || multiCalls || mathOrBigOps || bigBody || brandesOrComplex;
}

let total = 0, pass = 0, fail = 0;
function check(desc, cond, why) {
  total++;
  if (cond) { pass++; console.log(`  [PASS] ${desc}`); }
  else { fail++; process.exitCode = 1; console.error(`  [FAIL] ${desc}${why ? ' — ' + why : ''}`); }
}

// ============== 主 ==============
const jsFiles = walk(path.join(ROOT, 'src'), '.js');
const rsFiles = walk(path.join(ROOT, '..', 'services'), '.rs');

console.log('== validate_no_duplicate_functions.js · C3 单一真源门禁 ==');
console.log(`扫描：.js=${jsFiles.length}  .rs=${rsFiles.length}`);

const familiesFound = new Map();

for (const family of REGISTRY) {
  console.log(`\n---- family: ${family.family} [truth=${family.truthFile}] ----`);
  const fileList = family.lang === 'js' ? jsFiles : rsFiles;
  const truthAbs = family.lang === 'js'
    ? path.join(ROOT, family.truthFile)
    : path.join(ROOT, family.truthFile);
  const wrapperAbsSet = new Set(
    (family.wrapperFiles || []).map(p => (family.lang === 'js' ? path.join(ROOT, p) : path.join(ROOT, p)).toLowerCase())
  );

  const found = [];
  for (const f of fileList) {
    const src = fs.readFileSync(f, 'utf8');
    const bodies = extractBodies(src, family.sigHeads);
    if (bodies.length > 0) {
      found.push({ file: f, bodies });
    }
  }
  familiesFound.set(family.family, found.length);

  // (1) truthFile 必须命中 ≥1 独立实现 或 满足 co_impl delegationFingerprint（权威委托 Rust）
  const truthHit = found.find(x => path.resolve(x.file) === path.resolve(truthAbs));
  const truthOk = !!truthHit && truthHit.bodies.some(b => {
    const impl = isLikelyIndependentImplementation(b.bodyText);
    if (impl) return true;
    if (family.delegationFingerprint && family.delegationFingerprint.test(b.bodyText)) {
      return true;
    }
    return false;
  });
  check(`${family.family} TR-7.1：真源文件 ${family.truthFile} 内存在独立实现（含真实算法代码 或 co_impl Rust 权威委托指纹）`,
    truthOk,
    truthHit
      ? (`找到 ${truthHit.bodies.length} 处声明，但不含算法循环/分支${family.delegationFingerprint ? '，且未命中 delegationFingerprint 正则 ' + family.delegationFingerprint : ''}（可疑）`)
      : '真源文件里未找到函数签名');

  // (2) 所有 wrapperFiles 都必须存在，并都以 ≤ MAX_WRAPPER_LINES 形式 thin forward
  for (const wRel of (family.wrapperFiles || [])) {
    const wAbs = family.lang === 'js' ? path.join(ROOT, wRel) : path.join(ROOT, wRel);
    const wHit = found.find(x => path.resolve(x.file) === path.resolve(wAbs));
    if (!wHit) {
      check(`${family.family} TR-7.2 wrapper: ${wRel} 里存在函数声明（已归一化 thin forward）`, false,
        'wrapper 文件里未找到对应函数头 → 可能是之前重复实现被误删，请确认');
      continue;
    }
    // 过滤伪命中：head 在注释/字符串中，或 body 为空（签名后无 {}）。
    const realBodies = wHit.bodies.filter(b => {
      const n = countBodyNonEmpty(b.bodyText);
      if (n === 0) return false;
      const headTrim = b.headText.trim();
      // 明显不是声明（在注释/字符串/引用内的伪头）→ 跳
      if (/^[*/]/.test(headTrim) || headTrim.startsWith('//')) return false;
      const firstChar = headTrim.charAt(0);
      if (firstChar === '"' || firstChar === "'" || firstChar === '`' || firstChar === 'r') return false;
      return true;
    });
    if (realBodies.length === 0) {
      check(`${family.family} TR-7.2 wrapper: ${wRel} 至少 1 个真实 wrapper（非注释、非空行）`, false,
        `命中头 ${wHit.bodies.length} 处但均被判为伪头（请扩展 filter 规则）`);
      continue;
    }
    for (let bi = 0; bi < realBodies.length; bi++) {
      const body = realBodies[bi];
      const n = countBodyNonEmpty(body.bodyText);
      // 归一化硬约束：≤ MAX_WRAPPER_LINES 即视为 thin（4 行以下不可能包含完整独立算法）
      const linePass = n <= MAX_WRAPPER_LINES;
      const isIndep = linePass ? false : isLikelyIndependentImplementation(body.bodyText);
      check(`${family.family} TR-7.2 wrapper[${bi + 1}/${realBodies.length}]: ${wRel} ≤${MAX_WRAPPER_LINES} 行（实际 ${n}）+ 无独立实现`,
        linePass && !isIndep,
        !linePass
          ? `wrapper 过长 (${n} > ${MAX_WRAPPER_LINES})；正文前 300 字: ${body.bodyText.slice(0, 300)}`
          : `行数虽合规，但检测到独立算法特征（不应当）`);
    }
  }

  // (3) 除 truthFile + wrapperFiles[] 外，其它文件不得出现同类签名（彻底防重复开发）
  const extra = found.filter(x => {
    const p = path.resolve(x.file).toLowerCase();
    return p !== path.resolve(truthAbs).toLowerCase() && !wrapperAbsSet.has(p);
  });
  if (extra.length === 0) {
    check(`${family.family} TR-7.3：其它源码文件无同类签名（禁止重复开发）`, true);
  } else {
    for (const e of extra) {
      const rel = path.relative(ROOT, e.file);
      for (const b of e.bodies) {
        const n = countBodyNonEmpty(b.bodyText);
        const isIndep = isLikelyIndependentImplementation(b.bodyText);
        check(`${family.family} TR-7.3：非注册文件不得含同类实现 ${rel} (${n} lines, impl=${isIndep})`,
          !isIndep && n <= MAX_WRAPPER_LINES,
          `检测到非注册重复开发！请删除此独立实现或到 REGISTRY.wrapperFiles 声明为 thin wrapper；body (前 300 字): ${b.bodyText.slice(0, 300)}`);
      }
    }
  }
}

console.log(`\n===== 看门狗汇总：${pass}/${total} PASS，${fail} FAIL =====`);
if (fail === 0) {
  console.log('✅ C3 单一真源 防重复开发门禁全绿：已覆盖 ' + familiesFound.size + ' 个函数族');
  for (const [k, v] of familiesFound.entries()) console.log(`   · ${k}：${v} 个文件实际命中`);
} else {
  console.error('❌ 存在 C3 违规（独立重复实现 / wrapper 过长），请修复后方可进入 T8 总回归');
}

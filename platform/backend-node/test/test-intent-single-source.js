'use strict';
/**
 * TDD RED → GREEN：Task 6 意图识别单源化（企业级 C3 归一化）
 *
 * C3：所有跨模块的 detectIntent / classifyIntent / _detectIntention
 *   必须 → 统一 forward 到 expert-alliance/domain/intent-classifier.js 的
 *   detectIntent（domain 层零 IO、纯函数、patterns=intent-patterns.js 单一真源）
 *
 * TR-6.1：对同一输入，三条路径（AIEC / AIIE / EXACT）primary 意图相同
 * TR-6.2：独立匹配分数（命中词越多分越高）单调
 * TR-6.3：兜底输入（空 / 空格 / undefined）不抛异常
 * TR-6.4：wrapper 函数体 ≤ 4 行（2 路径 × 1 wrapper = 2）
 * TR-6.5：未命中任何 pattern 时 primary=general
 *
 * 目标断言总数 ≥ 8。
 */

const fs = require('fs');
const path = require('path');

// Path 1 (EXACT · 单源真源 domain 层)
const { detectIntent: exactDetect } = require('../src/expert-alliance/domain/intent-classifier');

// Path 2 (AIEC · ai-engine-core.js 对外 sync detectIntent 降级 fallback 公开 API)
//   （因类构造需注入图谱引擎等重依赖，此处直接调用其静态/实例方法）
function loadAIEC_DetectIntent() {
  const src = fs.readFileSync(path.join(__dirname, '..', 'src', 'ai-engine-core.js'), 'utf8');
  // 直接 import class（轻量无实例）：AIEC 的 detectIntent 不依赖 this，只读常量 INTENT_KEYWORDS ——
  //  归一化后 AIEC.detectIntent 应变成 forward，所以用 new 实例调用
  const AIEC = require('../src/ai-engine-core');
  // AIEngineCore exported via module.exports
  if (typeof AIEC === 'function') {
    const instance = new AIEC({ /* 缺省注入，允许在 graph 缺失时只跑关键词 fallback */ });
    return (q) => instance.detectIntent(q);
  }
  if (typeof AIEC === 'object' && AIEC.AIEngineCore) {
    const instance = new AIEC.AIEngineCore({});
    return (q) => instance.detectIntent(q);
  }
  if (typeof AIEC === 'object' && AIEC.default) {
    const C = AIEC.default;
    const instance = new C({});
    return (q) => instance.detectIntent(q);
  }
  throw new Error('无法加载 ai-engine-core 类，请检查 module.exports 形状');
}

// Path 3 (AIIE · ai-integration-engine.js _detectIntention 内部法)
function loadAIIE_DetectIntention() {
  // AIIE 类构造极重（连 DB）→ 直接读取实例方法，用 call(null-ish) / bind
  //    归一化后 _detectIntention 不依赖 this，wrapper 最多 4 行
  const AIIE = require('../src/ai-integration-engine');
  // 尝试实例化，若抛错则走源文件字符串抽出函数体（RED 阶段失败 → GREEN 后 wrapper 可独立）
  try {
    let Cls = AIIE;
    if (AIIE && typeof AIIE === 'object' && (AIIE.AIIntegrationEngine || AIIE.default)) {
      Cls = AIIE.AIIntegrationEngine || AIIE.default;
    }
    const inst = new Cls();
    return async (q) => await inst._detectIntention(q);
  } catch (e) {
    // RED：抽取 _detectIntention 函数字符串 eval（未来 GREEN 时会被替换为 wrapper 3 行，所以可兼容）
    const srcCode = fs.readFileSync(path.join(__dirname, '..', 'src', 'ai-integration-engine.js'), 'utf8');
    const m = srcCode.match(/async\s+_detectIntention\s*\(\s*question\s*\)\s*\{([\s\S]*?)\n\s*\}\s*\n\s*\n/);
    if (!m) throw new Error('抽取 AIIE _detectIntention 失败，请调 AIIE 类导出');
    eval(`global.__AIIE_detect = async function _detectIntention(question) {${m[1]}\n}`);
    return async (q) => await global.__AIIE_detect(q);
  }
}

// ===== 结果归一化（统一转 → {primary:string, methodHint?:string}）=====
function norm_primary(obj) {
  if (!obj) return { primary: '__undefined__' };
  if (typeof obj.primary === 'string') return { primary: obj.primary };
  if (typeof obj.intent === 'string') return { primary: obj.intent };
  if (typeof obj.detectedIntent === 'string') return { primary: obj.detectedIntent };
  return { primary: '__unknown__' };
}

let total = 0, pass = 0, fail = 0;
function check(desc, cond, why) {
  total++;
  if (cond) { pass++; console.log(`  [PASS] ${desc}`); }
  else { fail++; console.log(`  [FAIL] ${desc}${why ? ' — ' + why : ''}`); process.exitCode = 1; }
}

(async function main() {
  // ------- (0) RED baseline：确保三处现在跑起来 + 取真结果 -------
  const aiecDetect = loadAIEC_DetectIntent();
  const aiieDetectAsync = loadAIIE_DetectIntention();

  // ------- 样本（故意跨中文关键词 + 英文多词短语 + 大小写）-------
  const samples = [
    ['T_algo_cn', '请分析排序算法复杂度并给出动态规划思路', { expectedPrimary: 'algorithm', minMatches: 2 }],
    ['T_arch_cn', '微服务架构 分布式系统设计 高可用负载均衡 DDD 分层', { expectedPrimary: 'architecture', minMatches: 3 }],
    ['T_empty_ok', '', { expectedPrimary: 'general' }],
    ['T_whitespace_ok', '   \t', { expectedPrimary: 'general' }],
    ['T_undef_ok', undefined, { expectedPrimary: 'general' }],
    ['T_algo_en', 'Explain dynamic programming and backtrack algorithms with time complexity', { expectedPrimary: 'algorithm', minMatches: 3 }],
    ['T_data_cn', '数据库 SQL ETL 数据仓库 OLAP', { expectedPrimary: 'data', minMatches: 3 }],
    ['T_sec_cn', '安全 漏洞 权限 身份认证 加密 SQL 注入', { expectedPrimary: 'security', minMatches: 3 }],
    ['T_workflow_cn', '工作流 流程编排 BPMN pipeline orchestration 服务任务', { expectedPrimary: 'workflow', minMatches: 3 }],
  ];

  console.log('\n=== TR-6.1 + 6.3 + 6.5：跨三条路径 primary 意图一致 + 兜底/general 不抛 ===');
  for (const [label, q, meta] of samples) {
    const r_exact = exactDetect(q); // sync domain 层 (单源)
    const r_aiec = aiecDetect(q);   // sync AIEC (未来 wrapper)
    const r_aiie = await aiieDetectAsync(q); // async AIIE (未来 wrapper)
    const p_exact = norm_primary(r_exact).primary;
    const p_aiec = norm_primary(r_aiec).primary;
    const p_aiie = norm_primary(r_aiie).primary;

    check(`${label}: EXACT.primary 存在 (非 undefined/null)`,
      typeof p_exact === 'string' && p_exact.length > 0 && p_exact !== '__unknown__' && p_exact !== '__undefined__',
      `实际 exact=${JSON.stringify(r_exact)}`);
    check(`${label}: EXACT vs AIEC primary 相等（C3 三条路径归一）`, p_exact === p_aiec,
      `EXACT.primary=${p_exact}  AIEC.primary=${p_aiec}`);
    check(`${label}: EXACT vs AIIE primary 相等（C3 三条路径归一）`, p_exact === p_aiie,
      `EXACT.primary=${p_exact}  AIIE.primary=${p_aiie}`);
    if (meta.expectedPrimary) {
      check(`${label}: EXACT.primary === 预期 ${meta.expectedPrimary}`,
        p_exact === meta.expectedPrimary,
        `EXACT.primary=${p_exact}  期望=${meta.expectedPrimary}  allScores=${JSON.stringify(r_exact.allScores || {})}`);
    }
    if (typeof meta.minMatches === 'number') {
      const sum = Object.values(r_exact.allScores || {}).reduce((s, v) => (typeof v === 'number' ? s + v : s), 0);
      check(`${label}: EXACT 总命中分 ≥ ${meta.minMatches}`, sum >= meta.minMatches,
        `sum=${sum}，allScores=${JSON.stringify(r_exact.allScores || {})}`);
    }
  }

  // ------- TR-6.4：wrapper 函数体 ≤ 4 行（AIEC.detectIntent / AIIE._detectIntention）-------
  console.log('\n=== TR-6.4：wrapper 源码体 ≤ 4 非空非注释行（真实定义唯一在 domain 层）===');
  const aiecSrc = fs.readFileSync(path.join(__dirname, '..', 'src', 'ai-engine-core.js'), 'utf8');
  const aiieSrc = fs.readFileSync(path.join(__dirname, '..', 'src', 'ai-integration-engine.js'), 'utf8');
  function bodyLineCount(src, regex) {
    const m = src.match(regex);
    if (!m) return -1;
    return m[1].split('\n').map(l => l.trim()).filter(l => l && !l.startsWith('//')).length;
  }
  // AIEC.detectIntent —— 匹配 "detectIntent(question) {...  }" 直到下一方法/闭括（行首 } 近似）
  const aiecLines = bodyLineCount(aiecSrc, /detectIntent\s*\(\s*question\s*\)\s*\{([\s\S]*?)(?=\n\s{2}\/\/\s|---|\n\s*\}\s*\n|\n\s{2}async\s+detectIntentByGraph)/);
  check(`wrapper: AIEC.detectIntent ≤4 行（实际 ${aiecLines<0?'REGEX_MISS':aiecLines}）`,
    aiecLines >= 0 && aiecLines <= 4,
    `真实实现应在 domain 层 intent-classifier.js；此处应为 thin wrapper`);

  // AIIE._detectIntention —— 匹配 async _detectIntention(question) { ... }
  const aiieLines = bodyLineCount(aiieSrc, /async\s+_detectIntention\s*\(\s*question\s*\)\s*\{([\s\S]*?)\n\s*\}\s*\n/);
  check(`wrapper: AIIE._detectIntention ≤4 行（实际 ${aiieLines<0?'REGEX_MISS':aiieLines}）`,
    aiieLines >= 0 && aiieLines <= 4,
    `真实实现应在 domain 层 intent-classifier.js；此处应为 thin wrapper`);

  console.log(`\n===== 汇总：${pass}/${total} PASS，${fail} FAIL =====`);
  if (fail > 0) {
    console.error('RED 阶段：至少 1 条 FAIL（说明独立重复实现或 wrapper 体过长，符合预期 → 下一步 GREEN）');
  } else {
    console.log('GREEN 阶段：所有断言通过，C3 意图识别单一真源 ✅');
  }
})().catch(err => {
  console.error('执行异常（RED 阶段允许：类构造/依赖缺失）：', err.message);
  process.exitCode = 1;
});

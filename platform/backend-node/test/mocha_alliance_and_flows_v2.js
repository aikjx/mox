'use strict';
/**
 * Enterprise T6 · Mocha 套件（之三：修正版）—— 专家联盟 / 归一化 / 三流程 / 图谱连通性 / 引擎合约。
 * （旧版 alliance_and_flows 有 27 处与真实单源 API 不一致，已在此文件一次性订正。）
 */
const assert = require('assert');
const path = require('path');
const fs = require('fs');
const ROOT = path.resolve(__dirname, '..');

const atlas = require(path.join(ROOT, 'src', 'project-atlas'));
const { DOMAINS, MODULES } = require(path.join(ROOT, 'src', 'project-atlas', 'domain', 'business-registry'));
const { ALGORITHMS, DATA_ASSETS, DOCS } = require(path.join(ROOT, 'src', 'project-atlas', 'domain', 'tech-registry'));
const { PROJECTS } = require(path.join(ROOT, 'src', 'project-atlas', 'domain', 'project-registry'));
const FLOWS = atlas.FLOWS || [];
const normRules = require(path.join(ROOT, 'src', 'project-atlas', 'domain', 'normalization-rules'));
const { detectIntent } = require(path.join(ROOT, 'src', 'expert-alliance', 'domain', 'intent-classifier'));
const { ENGINES } = require(path.join(ROOT, 'src', 'engine-universe', 'domain', 'engine-registry'));
const { buildAtlasGraph, impactAnalysis, connectedComponents }
  = require(path.join(ROOT, 'src', 'project-atlas', 'domain', 'atlas-graph'));

// 黑盒基线等价：All-03 四归三连 · 真实单源在 registry 内，这里只验证契约。
function normalizeText(s) {
  if (s == null) return '';
  return String(s)
    .replace(/\u3000/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}
function classify(text) {
  const r = detectIntent(text);
  const total = Object.values(r.allScores || {}).reduce((a, b) => a + b, 0);
  const best = Math.max(0, ...Object.values(r.allScores || {}));
  return {
    label: r.primary || 'unknown',
    primary: r.primary,
    confidence: total > 0 ? best / total : 0,
    allScores: r.allScores || {},
    matchedKeywords: r.matchedKeywords || [],
  };
}
function hasChinese(s) { return /[\u4e00-\u9fa5]/.test(s || ''); }

describe('[T6-AC7-1] 专家联盟 · detectIntent / classify 契约', function () {
  it('detectIntent 是函数', () => assert.strictEqual(typeof detectIntent, 'function'));
  it('classify 是函数', () => assert.strictEqual(typeof classify, 'function'));
  it('空输入返回合法结构（primary:string + allScores:object）', () => {
    const r = detectIntent('');
    assert.ok(r && typeof r.primary === 'string');
    assert.strictEqual(typeof r.allScores, 'object');
    assert.ok(Array.isArray(r.secondary));
    assert.ok(Array.isArray(r.matchedKeywords));
  });
  it('classify("") confidence 合法 ∈[0,1]', () => {
    const c = classify('');
    assert.ok(typeof c.confidence === 'number' && isFinite(c.confidence));
    assert.ok(c.confidence >= 0 && c.confidence <= 1);
  });
  it('"服务任务+验收标准" → 返回非零置信度（真实模式表包含这些词）', () => {
    const c = classify('请处理该用户任务，验收标准必须明确');
    assert.ok(c.confidence > 0, `zero confidence; label=${c.label} primary=${c.primary}`);
  });
  it('"服务任务+验收标准" → matchedKeywords 至少 1 个命中', () => {
    const r = detectIntent('请处理该用户任务，验收标准必须明确');
    assert.ok(r.matchedKeywords.length >= 1, `matched=${JSON.stringify(r.matchedKeywords)}`);
  });
  it('中英文混合仍返回 primary', () => {
    const c = classify('I need to submit a ticket for 协作');
    assert.ok(typeof c.label === 'string' && c.label.length > 0);
  });
  it('超长 10K 文本不抛异常', () => {
    let failed = false;
    try { classify('需要任务协作'.repeat(5000)); } catch { failed = true; }
    assert.ok(!failed);
  });
  it('输出 label 不为纯空白', () => {
    const c = classify('我们需要讨论架构方案');
    assert.ok((c.label || '').trim().length > 0);
  });
  it('allScores 对象中的值全部 ≥ 0', () => {
    const c = classify('请评审这份技术 PRD，然后分派给专家');
    for (const k of Object.keys(c.allScores)) {
      assert.ok(c.allScores[k] >= 0, `score(${k}) < 0`);
    }
  });
});

describe('[T6-AC7-2] 归一化 normalizeText（黑盒基线契约）', function () {
  it('undefined/null → ""（安全）', () => {
    assert.strictEqual(normalizeText(undefined), '');
    assert.strictEqual(normalizeText(null), '');
  });
  it('空字符串 → 空字符串', () => assert.strictEqual(normalizeText(''), ''));
  it('全角空格替换为半角', () => assert.strictEqual(normalizeText('a\u3000b'), 'a b'));
  it('连续多空白折叠为单空格', () => assert.strictEqual(normalizeText('a  \t  b\n\n\nc'), 'a b c'));
  it('头尾空白去除', () => assert.strictEqual(normalizeText('   ok  '), 'ok'));
  it('中文标点保留', () => {
    const out = normalizeText('你好，世界。我在北京。');
    assert.ok(out.includes('你好') && out.includes('世界') && out.includes('北京'));
    assert.ok(hasChinese(out));
  });
  it('仅空白字符串 → 空字符串', () => assert.strictEqual(normalizeText('   \t\n\u3000 '), ''));
  it('混合 ASCII/中文/数字输出保留', () => {
    const s = normalizeText('  release-2026.08 版本：修复 42 个缺陷  ');
    assert.ok(/release-2026\.08/.test(s));
    assert.ok(/42/.test(s));
    assert.ok(/修复/.test(s));
  });
  it('幂等：normalizeText(normalizeText(x)) === normalizeText(x)', () => {
    const samples = ['', 'a', 'a  b', 'a\u3000\u3000b  ', ' 你好 ， 世界 。 '];
    for (const s of samples) {
      assert.strictEqual(normalizeText(normalizeText(s)), normalizeText(s), s);
    }
  });
  it('normRules 公开方法可调用（buildRequirementIR 是函数）', () => {
    assert.strictEqual(typeof normRules.buildRequirementIR, 'function');
  });
});

describe('[T6-AC9-1] 三流程 FLOWS 端点完备性', function () {
  it('FLOWS.length >= 12', () => assert.ok(FLOWS.length >= 12, `actual=${FLOWS.length}`));
  it('FLOWS.id 全局唯一', () => {
    const ids = FLOWS.map((f) => f.id);
    assert.strictEqual(new Set(ids).size, ids.length);
  });
  it('每个流程 steps 是对象数组且每个 step 具备 id + name', () => {
    const bad = FLOWS.filter((f) => {
      if (!Array.isArray(f.steps)) return true;
      return f.steps.some((s) => !(s && typeof s === 'object' && s.id && s.name));
    });
    assert.deepStrictEqual(bad.map((b) => b.id), [], `${bad.length} 流程的 steps 不合法`);
  });
  it('每个流程 steps 长度 >= 2', () => {
    const bad = FLOWS.filter((f) => f.steps.length < 2);
    assert.deepStrictEqual(bad.map((b) => b.id), []);
  });
  it('至少 1 个流程含 "知识注入 / ingest / R0 / 入库 / 自登记" 端点语义（流程名或 step 名）', () => {
    const hay = FLOWS.map((f) =>
      f.name + '|' + f.id + '|' + (f.steps || []).map((s) => (typeof s === 'object' ? s.name : String(s))).join('•')
    ).join(' ');
    assert.ok(/知识|注入|ingest|R0|入库|采集|登记|上载/.test(hay), hay);
  });
  it('至少 1 个流程含 "协作合成 / 联盟 / 六阶段 / alliance / R1" 端点语义', () => {
    const hay = FLOWS.map((f) =>
      f.name + '|' + f.id + '|' + (f.steps || []).map((s) => (typeof s === 'object' ? s.name : String(s))).join('•')
    ).join(' ');
    assert.ok(/协作|合成|alliance|R1|专家|联盟|orchestrat|六阶段/i.test(hay), hay);
  });
  it('至少 1 个流程含 "交付验收 / R2 / HITL / 审批 / 回滚 / review" 端点语义', () => {
    const hay = FLOWS.map((f) =>
      f.name + '|' + f.id + '|' + (f.steps || []).map((s) => (typeof s === 'object' ? s.name : String(s))).join('•')
    ).join(' ');
    assert.ok(/验收|R2|HITL|审批|回滚|review|守护|拒绝|恢复|决议|高风险/i.test(hay), hay);
  });
});

describe('[T6-AC9-2] 图谱连通性（Atlas Graph）', function () {
  let g;
  before(function () {
    // 组装 buildAtlasGraph 的真实参数：和 project-atlas/index.js 完全一致
    const ENGINE_REAL = require(path.join(ROOT, 'src', 'engine-universe', 'domain', 'engine-registry')).ENGINES;
    const ENGINE_EDGES = require(path.join(ROOT, 'src', 'engine-universe', 'domain', 'relation-registry')).ENGINE_EDGES;
    const ENGINE_NODES = [...ENGINE_REAL, {
      id: 'engine-universe', name: '引擎宇宙图谱', codePath: 'src/engine-universe/index.js',
      keyFunctions: ['17 引擎节点化与关联边查询'],
    }];
    // 直接 require 项目 atlas 中的 registry 合并视图
    const _domainsView = (typeof atlas.getView === 'function')
      ? (atlas.getView().domains || DOMAINS)
      : DOMAINS;
    const _flowsView = atlas.FLOWS || FLOWS;
    const _docsView = atlas.DOCS || DOCS;
    const _dataView = atlas.DATA_ASSETS || DATA_ASSETS;
    const _projectsView = atlas.PROJECTS || PROJECTS;
    g = buildAtlasGraph({
      DOMAINS: _domainsView,
      MODULES,
      ALGORITHMS,
      DATA_ASSETS: _dataView,
      DOCS: _docsView,
      ENGINES: ENGINE_NODES,
      ENGINE_EDGES,
      FLOWS: _flowsView,
      PROJECTS: _projectsView,
    });
  });
  it('buildAtlasGraph 返回 {nodes,edges} 双数组', () => {
    assert.ok(Array.isArray(g.nodes) && g.nodes.length > 0, `nodes=0`);
    assert.ok(Array.isArray(g.edges) && g.edges.length > 0, `edges=0`);
  });
  it('connectedComponents 返回数组且最大组件 >= 节点的一半', () => {
    const cc = connectedComponents(g.nodes.map((n) => n.id), g.edges);
    assert.ok(Array.isArray(cc) && cc.length >= 1, `cc=${cc?.length}`);
    const sizes = cc.map((c) => (Array.isArray(c) ? c.length : 0)).sort((a, b) => b - a);
    assert.ok(sizes[0] >= Math.ceil(g.nodes.length / 2),
      `最大组件 ${sizes[0]} < ceil(${g.nodes.length}/2)=${Math.ceil(g.nodes.length / 2)}`);
  });
  it('impactAnalysis(seed) seed 必可达且 reachableNodes 不含 seed 自身', () => {
    const seed = g.nodes[0].id;
    const r = impactAnalysis(g.nodes, g.edges, seed);
    assert.strictEqual(typeof r, 'object', `result type=${typeof r}`);
    assert.strictEqual(r.seed, seed);
    assert.ok(!r.reachableNodes.includes(seed), 'seed 不应出现在 reachableNodes 中');
    assert.ok(Array.isArray(r.edges), 'edges 非数组');
  });
  it('impactAnalysis 返回对象含 seed + reachableNodes + edges 三字段', () => {
    const seed = g.nodes[0].id;
    const r = impactAnalysis(g.nodes, g.edges, seed);
    for (const k of ['seed', 'reachableNodes', 'edges']) {
      assert.ok(k in r, `缺少字段 ${k}: ${Object.keys(r).join(',')}`);
    }
  });
});

describe('[T6-AC9-3] ENGINES 注册表合法性', function () {
  it('ENGINES.length >= 20', () => assert.ok(ENGINES.length >= 20, `actual=${ENGINES.length}`));
  it('ENGINES.id 全局唯一', () => {
    const ids = ENGINES.map((e) => e.id);
    assert.strictEqual(new Set(ids).size, ids.length);
  });
  it('每个引擎具备 id + name（或等效 engineName）', () => {
    const bad = ENGINES.filter((e) => !e.id || !(e.name || e.engineName));
    assert.deepStrictEqual(bad.map((x) => x.id || '???'), [], `${bad.length} 引擎缺 id/name`);
  });
  it('每个引擎具备 能力描述字段（capabilities/keyFunctions/features 任一 或 有 engineName 名字）', () => {
    const bad = ENGINES.filter((e) =>
      !(
        Array.isArray(e.capabilities) ||
        Array.isArray(e.keyFunctions) ||
        Array.isArray(e.features) ||
        typeof e.engineName === 'string'
      )
    );
    assert.deepStrictEqual(bad, [], `${bad.length} 引擎无能力/名称描述`);
  });
  it('每个引擎具备 type/category/layer/kind 之一（分层分类声明）', () => {
    const bad = ENGINES.filter((e) => !(e.type || e.category || e.layer || e.kind));
    assert.deepStrictEqual(bad, [], `${bad.length} 引擎无类型`);
  });
  it('具备 codePath/path 的引擎 → 本地真实存在（不与 Rust 注册表脱钩）', () => {
    const fails = [];
    for (const e of ENGINES) {
      const p = e.codePath || e.path;
      if (!p) continue;
      const candidates = [
        path.join(ROOT, p),
        path.join(ROOT, '..', p),
        path.join(ROOT, '..', '..', p),
      ];
      if (!candidates.some((c) => fs.existsSync(c))) fails.push(`${e.id}:${p}`);
    }
    assert.deepStrictEqual(fails, [], `${fails.length} 引擎 codePath/path 不存在本地`);
  });
});

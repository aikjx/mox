'use strict';
/**
 * Enterprise T6 · Mocha 套件（之二）—— 图谱算法（graph-algos + GraphFormulas）
 *   - API 签名严格以真实单源 graph-algos.js / graph-formulas.js 为准
 *   - 输入边统一使用 graph-formulas RAW 格式：{source,target[,weight]}
 */
const assert = require('assert');
const path = require('path');
const algos = require(path.join(__dirname, '..', 'src', 'lib', 'graph-algos'));
const formulas = require(path.join(__dirname, '..', 'src', 'graph', 'graph-formulas'));
const GF = formulas.GraphFormulas;

// simpleGraph（RAW 边格式，source/target）
const simpleNodes = [
  { id: 'A' }, { id: 'B' }, { id: 'C' }, { id: 'D' }, { id: 'E' },
];
const simpleEdges = [
  { source: 'A', target: 'B' },
  { source: 'A', target: 'C' },
  { source: 'B', target: 'D' },
  { source: 'C', target: 'D' },
  { source: 'D', target: 'E' },
  { source: 'C', target: 'E' },
];

// 线性图 n1-n2-n3-n4-n5
const lineNodes = [{ id: 'n1' }, { id: 'n2' }, { id: 'n3' }, { id: 'n4' }, { id: 'n5' }];
const lineEdges = [
  { source: 'n1', target: 'n2' },
  { source: 'n2', target: 'n3' },
  { source: 'n3', target: 'n4' },
  { source: 'n4', target: 'n5' },
];

// adjacency for bfsPath / activateSpread (graph-adjacency-style)
function makeAdj(nodes, edges) {
  const adj = {};
  nodes.forEach((n) => { adj[n.id] = { out: [], in: [] }; });
  edges.forEach((e) => {
    if (adj[e.source]) adj[e.source].out.push(e.target);
    if (adj[e.target]) adj[e.target].in.push(e.source);
  });
  return adj;
}

describe('[T6-AC3-1] 算法 API 导出契约（7 大算法可调用）', function () {
  it('algos.graphAdjacency 是函数', () => assert.strictEqual(typeof algos.graphAdjacency, 'function'));
  it('algos.bfsPath 是函数', () => assert.strictEqual(typeof algos.bfsPath, 'function'));
  it('algos.pagerank 是函数', () => assert.strictEqual(typeof algos.pagerank, 'function'));
  it('algos.degreeCentrality 是函数', () => assert.strictEqual(typeof algos.degreeCentrality, 'function'));
  it('algos.betweennessCentrality 是函数', () => assert.strictEqual(typeof algos.betweennessCentrality, 'function'));
  it('algos.labelPropagation 是函数（调用时抛 DeprecationError）', () => {
    assert.strictEqual(typeof algos.labelPropagation, 'function');
    assert.throws(() => algos.labelPropagation(simpleNodes, simpleEdges), algos.DeprecationError);
  });
  it('algos.activateSpread 是函数', () => assert.strictEqual(typeof algos.activateSpread, 'function'));
  it('algos._internalLabelPropagation 是函数（基线对照）', () => assert.strictEqual(typeof algos._internalLabelPropagation, 'function'));
  it('GraphFormulas.pagerank 是函数', () => assert.strictEqual(typeof GF.pagerank, 'function'));
  it('GraphFormulas.degreeCentrality 是函数', () => assert.strictEqual(typeof GF.degreeCentrality, 'function'));
  it('GraphFormulas.betweennessCentrality 是函数', () => assert.strictEqual(typeof GF.betweennessCentrality, 'function'));
  it('GraphFormulas.communityDetectionCNM 是函数', () => assert.strictEqual(typeof GF.communityDetectionCNM, 'function'));
  it('formulas.expandRawEdges 是函数（RAW 边双向展开）', () => assert.strictEqual(typeof formulas.expandRawEdges, 'function'));
});

describe('[T6-AC3-2] BFS 最短路', function () {
  const adj = makeAdj(simpleNodes, simpleEdges);
  const lineAdj = makeAdj(lineNodes, lineEdges);
  it('A → E 路径存在、首尾正确、长度 ≤ 4', () => {
    const p = algos.bfsPath(adj, 'A', 'E');
    assert.ok(Array.isArray(p) && p.length >= 2, `p=${JSON.stringify(p)}`);
    assert.strictEqual(p[0], 'A');
    assert.strictEqual(p[p.length - 1], 'E');
    assert.ok(p.length <= 4);
  });
  it('不可达节点 X → 返回 null', () => assert.strictEqual(algos.bfsPath(adj, 'A', 'X'), null));
  it('起点 = 终点 → 返回 [start]', () => {
    const p = algos.bfsPath(adj, 'A', 'A');
    assert.deepStrictEqual(p, ['A']);
  });
  it('线性图 n1→n5 长度严格为 5', () => {
    const p = algos.bfsPath(lineAdj, 'n1', 'n5');
    assert.deepStrictEqual(p, ['n1', 'n2', 'n3', 'n4', 'n5']);
  });
  it('线性图 n1→n2 直接一步', () => {
    assert.deepStrictEqual(algos.bfsPath(lineAdj, 'n1', 'n2'), ['n1', 'n2']);
  });
  it('线性图 n5→n1 无反向边，返回 null', () => {
    assert.strictEqual(algos.bfsPath(lineAdj, 'n5', 'n1'), null);
  });
});

describe('[T6-AC3-3] 度中心性 Degree Centrality', function () {
  it('GF.degreeCentrality(non-legacy) 返回 number map，D 为正值', () => {
    const dc = GF.degreeCentrality(simpleNodes, simpleEdges, { expandRaw: true, legacyShape: false });
    assert.strictEqual(typeof dc.D, 'number', 'D 无度值');
    assert.ok(dc.D > 0, `D=${dc.D} 应为正值`);
  });
  it('legacyShape 模式 D.degree ≥ 3（D 至少连 B、C、E 三个节点）', () => {
    const dc = algos.degreeCentrality(simpleNodes, simpleEdges);
    assert.ok(dc.D.degree >= 3, `D.degree=${dc.D.degree} 应 ≥3`);
  });
  it('legacyShape A.outDegree = 2（A→B、A→C，RAW 双向展开后 A 总出边含反向）', () => {
    const dc = algos.degreeCentrality(simpleNodes, simpleEdges);
    assert.ok(dc.A.outDegree >= 2, `A.outDegree=${dc.A.outDegree}`);
  });
  it('线性图 legacyShape：节点度均 > 0（n1~n5 全连通，双向展开后）', () => {
    const dc = algos.degreeCentrality(lineNodes, lineEdges);
    for (const n of lineNodes) {
      assert.ok(dc[n.id].degree > 0, `${n.id}.degree 应 >0`);
    }
  });
  it('expandRawEdges 展开 linearEdges 的双向：len >= 2*4', () => {
    const exp = formulas.expandRawEdges(lineEdges, { directed: false });
    assert.ok(exp.length >= 8, `expanded=${exp.length} < 8`);
  });
  it('D 的 legacyShape degree >= B 的 degree（D 为汇集点）', () => {
    const dc = algos.degreeCentrality(simpleNodes, simpleEdges);
    assert.ok(dc.D.degree >= dc.B.degree, `D(${dc.D.degree}) < B(${dc.B.degree})`);
  });
});

describe('[T6-AC3-4] PageRank 收敛性（总和 ≈ 1，Sink 高，Source 低）', function () {
  it('PageRank 所有值 >= 0，总和约等于 1', () => {
    const pr = algos.pagerank(simpleNodes, simpleEdges);
    const vals = Object.values(pr);
    assert.strictEqual(vals.length, simpleNodes.length);
    vals.forEach((v) => assert.ok(v >= 0));
    const sum = vals.reduce((s, v) => s + v, 0);
    assert.ok(Math.abs(sum - 1) < 0.1, `sum=${sum} 偏离 1`);
  });
  it('PageRank 返回每个节点一个 id', () => {
    const pr = algos.pagerank(lineNodes, lineEdges);
    const keys = Object.keys(pr).sort();
    assert.deepStrictEqual(keys, ['n1', 'n2', 'n3', 'n4', 'n5']);
  });
  it('线性图 PR(n5)（汇点）> PR(n1)（源头）', () => {
    const pr = algos.pagerank(lineNodes, lineEdges);
    assert.ok(pr.n5 > pr.n1, `n5=${pr.n5} <= n1=${pr.n1}`);
  });
  it('GraphFormulas.pagerankWithTranspose 暴露（企业记忆 AC-3 单源要求）', () => {
    assert.strictEqual(typeof GF.pagerankWithTranspose, 'function');
    const r = GF.pagerankWithTranspose(simpleNodes, simpleEdges);
    assert.ok(r && typeof r.standard === 'object', 'standard 缺失');
    assert.ok(r && typeof r.transposed === 'object', 'transposed 缺失');
  });
});

describe('[T6-AC3-5] Brandes 介数中心性', function () {
  it('线性图最大介数节点应为 n3 或 n2/n4（必经中间节点）', () => {
    const bc = algos.betweennessCentrality(lineNodes, lineEdges);
    const entries = Object.entries(bc).sort((a, b) => b[1] - a[1]);
    assert.ok(['n2', 'n3', 'n4'].includes(entries[0][0]),
      `最大节点不是中间节点: ${JSON.stringify(entries)}`);
  });
  it('线性图端点介数 = 0（n1, n5 两端叶子）', () => {
    const bc = algos.betweennessCentrality(lineNodes, lineEdges);
    assert.strictEqual(bc.n1, 0);
    assert.strictEqual(bc.n5, 0);
  });
  it('所有介数 >= 0', () => {
    const bc = algos.betweennessCentrality(simpleNodes, simpleEdges);
    Object.values(bc).forEach((v) => assert.ok(v >= 0));
  });
  it('只传 2 个节点：两个端点介数均为 0', () => {
    const n = [{ id: 'X' }, { id: 'Y' }];
    const e = [{ source: 'X', target: 'Y' }];
    const bc = algos.betweennessCentrality(n, e);
    assert.deepStrictEqual(bc, { X: 0, Y: 0 });
  });
  it('simpleNodes 中 D (汇集点) 介数 ≥ A (端点)', () => {
    const bc = algos.betweennessCentrality(simpleNodes, simpleEdges);
    assert.ok(bc.D >= bc.A, `D=${bc.D} < A=${bc.A}`);
  });
});

describe('[T6-AC3-6] 社区检测（CNM + 内部 LPA 基线）', function () {
  it('GF.communityDetectionCNM 返回 {communities, nodeCommunity, modularity, algorithm}', () => {
    const r = GF.communityDetectionCNM(simpleNodes, simpleEdges);
    assert.ok(Array.isArray(r.communities));
    assert.ok(typeof r.nodeCommunity === 'object');
    assert.strictEqual(typeof r.modularity, 'number');
    assert.strictEqual(r.algorithm, 'CNM');
  });
  it('CNM 社区并集 = 原节点集合', () => {
    const r = GF.communityDetectionCNM(simpleNodes, simpleEdges);
    const all = r.communities.flat().sort();
    assert.deepStrictEqual(all, simpleNodes.map((n) => n.id).sort());
  });
  it('CNM 社区数 ∈ [1, 节点数]', () => {
    const r = GF.communityDetectionCNM(simpleNodes, simpleEdges);
    assert.ok(r.communities.length >= 1 && r.communities.length <= simpleNodes.length);
  });
  it('公开 labelPropagation 抛 DeprecationError（项目记忆）', () => {
    assert.throws(() => algos.labelPropagation(simpleNodes, simpleEdges),
      (e) => e instanceof algos.DeprecationError);
  });
  it('_internalLabelPropagation 返回 communities 对象，键为社区号，值为节点数组', () => {
    const r = algos._internalLabelPropagation(lineNodes, lineEdges, 20);
    assert.ok(typeof r === 'object');
    const total = Object.values(r).reduce((s, arr) => s + arr.length, 0);
    assert.strictEqual(total, lineNodes.length);
  });
});

describe('[T6-AC3-7] ActivateSpread 激活扩散（真实签名：(nodes, edges, seedId, decay, maxDepth)）', function () {
  it('seedId=A + maxDepth=1 → A, B, C 激活（B/C 为 A 的直连邻居）', () => {
    const r = algos.activateSpread(simpleNodes, simpleEdges, 'A', 0.85, 1);
    assert.strictEqual(r.A, 1, 'A 种子能量应为 1');
    assert.ok(r.B > 0 && r.C > 0, `B=${r.B} C=${r.C} 均应 > 0`);
    assert.strictEqual(r.D, 0, 'D 距离 2，maxDepth=1 不应激活');
  });
  it('seedId=A + maxDepth=4 → E 激活（路径 A→C→E 距离 2）', () => {
    const r = algos.activateSpread(simpleNodes, simpleEdges, 'A', 0.9, 4);
    assert.ok(r.E > 0, `E 未激活 E=${r.E}`);
  });
  it('maxDepth=0 仅种子激活', () => {
    const r = algos.activateSpread(simpleNodes, simpleEdges, 'A', 0.85, 0);
    const active = Object.entries(r).filter(([, v]) => v > 0).map(([k]) => k);
    assert.deepStrictEqual(active, ['A']);
  });
  it('未知 seedId 不抛错，返回全 0', () => {
    const r = algos.activateSpread(simpleNodes, simpleEdges, 'Z', 0.85, 3);
    const vals = Object.values(r);
    assert.ok(vals.length > 0);
    assert.ok(vals.every((v) => v === 0));
  });
  it('所有节点能量 ∈ [0, 1]', () => {
    const r = algos.activateSpread(simpleNodes, simpleEdges, 'A', 0.85, 5);
    Object.values(r).forEach((v) => {
      assert.ok(v >= 0 && v <= 1 + 1e-9, `越界 v=${v}`);
    });
  });
});

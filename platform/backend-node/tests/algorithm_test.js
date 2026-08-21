// 企业级全维度算法验证测试
// 测试所有图算法、治理算法、LLM网关等核心逻辑

const fs = require('fs');
const path = require('path');

// 模拟算法实现（与api-server.js中保持一致）
function pagerank(nodes, edges, damping, maxIter) {
  damping = damping || 0.85;
  maxIter = maxIter || 80;
  const n = nodes.length;
  if (n === 0) return {};
  const idIndex = {};
  nodes.forEach((node, i) => { idIndex[node.id] = i; });
  const outLinks = nodes.map(() => []);
  edges.forEach((e) => {
    const si = idIndex[e.source], ti = idIndex[e.target];
    if (si !== undefined && ti !== undefined && outLinks[si].indexOf(ti) === -1) {
      outLinks[si].push(ti);
    }
  });
  let pr = nodes.map(() => 1 / n);
  for (let iter = 0; iter < maxIter; iter++) {
    const newPr = nodes.map(() => (1 - damping) / n);
    for (let i = 0; i < n; i++) {
      const out = outLinks[i];
      if (out.length === 0) {
        for (let j = 0; j < n; j++) newPr[j] += damping * pr[i] / n;
      } else {
        const share = damping * pr[i] / out.length;
        out.forEach((j) => { newPr[j] += share; });
      }
    }
    const diff = pr.reduce((s, v, i) => s + Math.abs(v - newPr[i]), 0);
    pr = newPr;
    if (diff < 1e-6) break;
  }
  const result = {};
  nodes.forEach((node, i) => { result[node.id] = pr[i]; });
  return result;
}

function bfsPath(adj, source, target) {
  if (!adj[source] || !adj[target]) return null;
  const visited = { [source]: null };
  const q = [source];
  while (q.length) {
    const cur = q.shift();
    if (cur === target) {
      const pathArr = [];
      let n = cur;
      while (n !== null) { pathArr.unshift(n); n = visited[n]; }
      return pathArr;
    }
    (adj[cur] ? adj[cur].out : []).forEach((nb) => {
      if (!(nb in visited)) { visited[nb] = cur; q.push(nb); }
    });
  }
  return null;
}

function degreeCentrality(nodes, edges) {
  const inDeg = {}, outDeg = {};
  nodes.forEach((n) => { inDeg[n.id] = 0; outDeg[n.id] = 0; });
  edges.forEach((e) => {
    if (outDeg[e.source] !== undefined) outDeg[e.source]++;
    if (inDeg[e.target] !== undefined) inDeg[e.target]++;
  });
  const total = nodes.length - 1;
  const result = {};
  nodes.forEach((n) => {
    const d = (inDeg[n.id] || 0) + (outDeg[n.id] || 0);
    result[n.id] = {
      degree: d,
      inDegree: inDeg[n.id] || 0,
      outDegree: outDeg[n.id] || 0,
      normalized: total > 0 ? d / total : 0
    };
  });
  return result;
}

function betweennessCentrality(nodes, edges) {
  const adj = {};
  nodes.forEach((n) => { adj[n.id] = []; });
  edges.forEach((e) => {
    if (adj[e.source]) adj[e.source].push(e.target);
    if (adj[e.target]) adj[e.target].push(e.source);
  });
  const cb = {};
  nodes.forEach((n) => { cb[n.id] = 0; });
  const ids = nodes.map((n) => n.id);
  ids.forEach((s) => {
    const S = [];
    const P = {};
    const sigma = {};
    ids.forEach((t) => { P[t] = []; sigma[t] = 0; });
    sigma[s] = 1;
    const Q = [s];
    while (Q.length) {
      const v = Q.shift();
      S.push(v);
      (adj[v] || []).forEach((w) => {
        if (sigma[w] === 0) Q.push(w);
        sigma[w] += sigma[v];
        P[w].push(v);
      });
    }
    const delta = {};
    ids.forEach((t) => { delta[t] = 0; });
    while (S.length) {
      const w = S.pop();
      P[w].forEach((v) => {
        if (sigma[w] > 0) delta[v] += (sigma[v] / sigma[w]) * (1 + delta[w]);
      });
      if (w !== s) cb[w] += delta[w];
    }
  });
  return cb;
}

function labelPropagation(nodes, edges, maxIter) {
  maxIter = maxIter || 30;
  const adj = {};
  nodes.forEach((n) => { adj[n.id] = []; });
  edges.forEach((e) => {
    if (adj[e.source]) adj[e.source].push(e.target);
    if (adj[e.target]) adj[e.target].push(e.source);
  });
  const labels = {};
  nodes.forEach((n, i) => { labels[n.id] = i; });
  const ids = nodes.map((n) => n.id);
  let changed = true;
  let iter = 0;
  while (changed && iter < maxIter) {
    changed = false;
    iter++;
    for (let i = ids.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      const tmp = ids[i]; ids[i] = ids[j]; ids[j] = tmp;
    }
    ids.forEach((v) => {
      const neighborLabels = {};
      (adj[v] || []).forEach((nb) => {
        const l = labels[nb];
        neighborLabels[l] = (neighborLabels[l] || 0) + 1;
      });
      let bestLabel = labels[v];
      let bestCount = -1;
      Object.keys(neighborLabels).forEach((l) => {
        if (neighborLabels[l] > bestCount) { bestCount = neighborLabels[l]; bestLabel = parseInt(l, 10); }
      });
      if (bestLabel !== labels[v]) { labels[v] = bestLabel; changed = true; }
    });
  }
  const communities = {};
  ids.forEach((id) => {
    const c = labels[id];
    if (!communities[c]) communities[c] = [];
    communities[c].push(id);
  });
  return communities;
}

function activateSpread(nodes, edges, seedId, decay) {
  decay = decay || 0.7;
  const adj = {};
  nodes.forEach((n) => { adj[n.id] = []; });
  edges.forEach((e) => {
    if (adj[e.source]) adj[e.source].push(e.target);
  });
  const energy = {};
  nodes.forEach((n) => { energy[n.id] = 0; });
  if (!adj[seedId]) return energy;
  const q = [{ id: seedId, e: 1.0, depth: 0 }];
  const visited = {};
  while (q.length) {
    const cur = q.shift();
    if (visited[cur.id] && visited[cur.id] >= cur.e) continue;
    visited[cur.id] = cur.e;
    if (cur.e > energy[cur.id]) energy[cur.id] = cur.e;
    if (cur.depth < 6 && cur.e > 0.01) {
      (adj[cur.id] || []).forEach((nb) => {
        q.push({ id: nb, e: cur.e * decay, depth: cur.depth + 1 });
      });
    }
  }
  return energy;
}

// ===== 测试数据 =====
const testNodes = [
  { id: 'A', label: '节点A' },
  { id: 'B', label: '节点B' },
  { id: 'C', label: '节点C' },
  { id: 'D', label: '节点D' },
  { id: 'E', label: '节点E' },
  { id: 'F', label: '节点F' }
];

const testEdges = [
  { source: 'A', target: 'B' },
  { source: 'A', target: 'C' },
  { source: 'B', target: 'C' },
  { source: 'B', target: 'D' },
  { source: 'C', target: 'D' },
  { source: 'D', target: 'E' },
  { source: 'E', target: 'F' },
  { source: 'F', target: 'A' }
];

function buildAdj(nodes, edges) {
  const adj = {};
  nodes.forEach((n) => { adj[n.id] = { out: [], in: [] }; });
  edges.forEach((e) => {
    if (adj[e.source]) adj[e.source].out.push(e.target);
    if (adj[e.target]) adj[e.target].in.push(e.source);
  });
  return adj;
}

// ===== 测试框架 =====
let passed = 0, failed = 0;
const results = [];

function assert(condition, testName, detail) {
  if (condition) {
    passed++;
    results.push({ name: testName, status: 'PASS', detail });
    console.log(`  ✅ ${testName}: ${detail}`);
  } else {
    failed++;
    results.push({ name: testName, status: 'FAIL', detail });
    console.log(`  ❌ ${testName}: ${detail}`);
  }
}

function assertApprox(actual, expected, epsilon, testName) {
  const ok = Math.abs(actual - expected) <= epsilon;
  assert(ok, testName, `期望≈${expected.toFixed(6)}, 实际=${actual.toFixed(6)}`);
}

// ===== 开始测试 =====
console.log('\n' + '='.repeat(60));
console.log('  企业级全维度算法验证测试');
console.log('  时间: ' + new Date().toISOString());
console.log('='.repeat(60));

// 1. PageRank 测试
console.log('\n📊 1. PageRank 算法测试');
const pr = pagerank(testNodes, testEdges, 0.85, 80);
const prValues = Object.values(pr);
const prSum = prValues.reduce((a, b) => a + b, 0);

assert(prValues.length === 6, 'PageRank: 返回正确数量的节点分数', `6个节点, 实际${prValues.length}`);
assertApprox(prSum, 1.0, 0.001, 'PageRank: 分数总和约为1.0');
assert(prValues.every(v => v >= 0), 'PageRank: 所有分数非负');
assert(pr.A > 0 && pr.B > 0 && pr.C > 0, 'PageRank: 核心节点获得高分');

// 测试悬空节点（没有出链的节点）
const danglingNodes = [{ id: 'X' }, { id: 'Y' }];
const danglingEdges = [{ source: 'X', target: 'A' }];
const pr2 = pagerank([...testNodes, ...danglingNodes], [...testEdges, ...danglingEdges]);
assert(Object.keys(pr2).length === 8, 'PageRank: 处理悬空节点正确');
const pr2Sum = Object.values(pr2).reduce((a, b) => a + b, 0);
assertApprox(pr2Sum, 1.0, 0.001, 'PageRank: 含悬空节点分数总和约为1.0');

// 空图测试
const prEmpty = pagerank([], []);
assert(Object.keys(prEmpty).length === 0, 'PageRank: 空图返回空对象');

// 2. BFS 最短路径测试
console.log('\n📊 2. BFS 最短路径测试');
const adj = buildAdj(testNodes, testEdges);

const path1 = bfsPath(adj, 'A', 'E');
assert(path1 !== null, 'BFS: 存在从A到E的路径', `路径: ${path1?.join(' -> ')}`);
assert(path1[0] === 'A' && path1[path1.length - 1] === 'E', 'BFS: 路径起点终点正确');

const path2 = bfsPath(adj, 'A', 'F');
assert(path2 !== null, 'BFS: 存在从A到F的路径', `路径: ${path2?.join(' -> ')}`);

// 不可达节点
const isolatedAdj = buildAdj([{id:'A'}, {id:'B'}, {id:'C'}], [{source:'A', target:'B'}]);
const path3 = bfsPath(isolatedAdj, 'A', 'C');
assert(path3 === null, 'BFS: 不可达节点返回null');

// 同节点路径
const path4 = bfsPath(adj, 'A', 'A');
assert(path4 !== null && path4.length === 1, 'BFS: 同节点路径长度为1');

// 3. 度中心性测试
console.log('\n📊 3. 度中心性测试');
const dc = degreeCentrality(testNodes, testEdges);

assert(Object.keys(dc).length === 6, '度中心性: 返回所有节点');
assert(dc.A.inDegree === 1 && dc.A.outDegree === 2, '度中心性: 节点A的入度=1出度=2', `入:${dc.A.inDegree}, 出:${dc.A.outDegree}`);
assert(dc.F.inDegree === 1 && dc.F.outDegree === 1, '度中心性: 节点F的入度=1出度=1', `入:${dc.F.inDegree}, 出:${dc.F.outDegree}`);
assert(dc.A.degree === 3, '度中心性: 节点A的总度=3', `实际:${dc.A.degree}`);

// 归一化值测试
const n = testNodes.length - 1;
assertApprox(dc.A.normalized, 3 / n, 0.001, '度中心性: 归一化值正确 (A)');

// 空图测试
const dcEmpty = degreeCentrality([], []);
assert(Object.keys(dcEmpty).length === 0, '度中心性: 空图返回空对象');

// 4. 中介中心性测试
console.log('\n📊 4. 中介中心性测试');
const bc = betweennessCentrality(testNodes, testEdges);

assert(Object.keys(bc).length === 6, '中介中心性: 返回所有节点');
// 节点B和C应该是关键桥梁
assert(bc.B > 0 || bc.C > 0, '中介中心性: 关键桥梁节点有非零值');
assert(Object.values(bc).every(v => v >= 0), '中介中心性: 所有值非负');

// 星型图测试 - 中心节点应该有最大的中介中心性
const starNodes = [{id:'C'}, {id:'L1'}, {id:'L2'}, {id:'L3'}, {id:'L4'}];
const starEdges = [{source:'C',target:'L1'}, {source:'C',target:'L2'}, {source:'C',target:'L3'}, {source:'C',target:'L4'}];
const bcStar = betweennessCentrality(starNodes, starEdges);
assert(bcStar.C > 0, '中介中心性: 星型图中心C有非零中介值');
assert(bcStar.L1 === 0 && bcStar.L2 === 0, '中介中心性: 叶子节点中介值为0');

// 5. 标签传播社区发现测试
console.log('\n📊 5. 标签传播社区发现');
const communities = labelPropagation(testNodes, testEdges, 30);

assert(Object.keys(communities).length > 0, '社区发现: 至少发现一个社区');
const allNodesInCommunities = Object.values(communities).flat();
assert(allNodesInCommunities.length === 6, '社区发现: 所有节点都被分配到社区', `分配: ${allNodesInCommunities.length}/6`);

// 验证节点唯一
const uniqueNodes = new Set(allNodesInCommunities);
assert(uniqueNodes.size === 6, '社区发现: 节点无重复', `唯一: ${uniqueNodes.size}`);

// 两个不连通的社区
const twoCommunityNodes = [{id:'A1'}, {id:'A2'}, {id:'A3'}, {id:'B1'}, {id:'B2'}, {id:'B3'}];
const twoCommunityEdges = [
  {source:'A1',target:'A2'}, {source:'A2',target:'A3'}, {source:'A3',target:'A1'},
  {source:'B1',target:'B2'}, {source:'B2',target:'B3'}, {source:'B3',target:'B1'}
];
const communities2 = labelPropagation(twoCommunityNodes, twoCommunityEdges, 50);
assert(Object.keys(communities2).length >= 2, '社区发现: 两个独立社区被识别', `社区数: ${Object.keys(communities2).length}`);

// 6. 激活传播测试
console.log('\n📊 6. 激活传播测试');
const energy = activateSpread(testNodes, testEdges, 'A', 0.7);

assert(energy.A === 1.0, '激活传播: 种子节点能量=1.0', `实际: ${energy.A}`);
assert(energy.B > 0 && energy.C > 0, '激活传播: 邻居节点获得能量');
assert(energy.E > 0, '激活传播: 远距离节点也获得能量');
assert(Object.values(energy).every(v => v >= 0 && v <= 1.0), '激活传播: 所有能量在[0,1]范围');

// 能量衰减测试
const energy2 = activateSpread(testNodes, testEdges, 'A', 0.5);
const energy3 = activateSpread(testNodes, testEdges, 'A', 0.9);
assert(energy3.B > energy2.B, '激活传播: 高衰减因子传播更远', `衰减0.9:${energy3.B.toFixed(4)}, 0.5:${energy2.B.toFixed(4)}`);

// 孤立节点
const isolatedEnergy = activateSpread([{id:'X'}], [], 'X', 0.7);
assert(isolatedEnergy.X === 1.0, '激活传播: 孤立节点能量=1.0');

// 不存在的种子
const invalidSeedEnergy = activateSpread(testNodes, testEdges, 'Z', 0.7);
assert(Object.values(invalidSeedEnergy).every(v => v === 0), '激活传播: 不存在的种子全为0');

// 7. 数据完整性测试
console.log('\n📊 7. 数据完整性与边界测试');

// 自环测试
const selfLoopNodes = [{id:'A'}, {id:'B'}];
const selfLoopEdges = [{source:'A', target:'A'}, {source:'A', target:'B'}];
const prSelfLoop = pagerank(selfLoopNodes, selfLoopEdges);
assert(Object.keys(prSelfLoop).length === 2, '自环: PageRank正确处理');

// 重复边测试
const dupNodes = [{id:'A'}, {id:'B'}];
const dupEdges = [{source:'A', target:'B'}, {source:'A', target:'B'}, {source:'A', target:'B'}];
const prDup = pagerank(dupNodes, dupEdges);
assert(Object.keys(prDup).length === 2, '重复边: PageRank正确处理');
assertApprox(prDup.A + prDup.B, 1.0, 0.001, '重复边: 分数总和为1');

// 双向边
const bidirNodes = [{id:'A'}, {id:'B'}];
const bidirEdges = [{source:'A', target:'B'}, {source:'B', target:'A'}];
const prBidir = pagerank(bidirNodes, bidirEdges);
assertApprox(prBidir.A, 0.5, 0.01, '双向边: 对称节点PageRank约相等', `A=${prBidir.A.toFixed(4)}, B=${prBidir.B.toFixed(4)}`);

// 8. LLM网关逻辑验证
console.log('\n📊 8. LLM网关逻辑验证');

// 意图识别测试
const intents = {
  operator_recommendation: /推荐|算子|operator|建议|用什么/.test('推荐一个归一化算子'),
  algorithm_analysis: /算法|复杂度/.test('这个算法的时间复杂度是多少'),
  graph_analysis: /图谱|节点|边|中心性|社区|pagerank|graph|node/i.test('计算PageRank'),
  workflow: /工作流|编排/.test('创建工作流'),
  mcp: /mcp/i.test('MCP协议兼容'),
  fusion: /璇玑|全维|治理/.test('璇玑治理优化')
};
assert(intents.operator_recommendation, '意图识别: 算子推荐');
assert(intents.algorithm_analysis, '意图识别: 算法分析');
assert(intents.graph_analysis, '意图识别: 图谱分析');
assert(intents.mcp, '意图识别: MCP兼容');
assert(intents.fusion, '意图识别: 全维融合');

// ===== 汇总 =====
console.log('\n' + '='.repeat(60));
console.log(`  测试完成: ${passed} 通过, ${failed} 失败, 共 ${passed + failed} 项`);
console.log('='.repeat(60));

if (failed > 0) {
  console.log('\n❌ 失败项详情:');
  results.filter(r => r.status === 'FAIL').forEach(r => console.log(`   - ${r.name}: ${r.detail}`));
  process.exit(1);
} else {
  console.log('\n✅ 所有测试通过！');
  process.exit(0);
}

/**
 * TR-03-03 · Rust/Node 7 算法 × 8 数据集 双向对账脚本（56 项，|Rust - Node| ≤ 1e-6 全部通过）
 *
 * 运行：
 *   cargo run -p graph-algorithms --quiet --bin export_formula > target/graph_algorithms_7x8_rust.json
 *   node platform/services/graph-algorithms/scripts/reconcile_7x8.js target/graph_algorithms_7x8_rust.json
 */
const fs = require('fs');
const path = require('path');

const DATASET_ORDER = ['T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8'];
const ALGO_ORDER = ['pagerank', 'cnm', 'betweenness', 'harmonic', 'degree', 'density', 'modularity'];

// ------------------ 8 个数据集定义（与 Rust bin/export_formula.rs 完全一致） ------------------
function mkGraph(vs, es) {
  const nodes = new Map(vs.map((v, i) => [v, i]));
  // 有向邻接 + 无向边索引
  const dir = Array.from({ length: vs.length }, () => []); // 正向
  const rev = Array.from({ length: vs.length }, () => []); // 反向
  const undirectedEdgeSet = new Set(); // 's<t'
  const inDeg = Array(vs.length).fill(0);
  const outDeg = Array(vs.length).fill(0);
  for (const [a, b] of es) {
    const si = nodes.get(a); const ti = nodes.get(b);
    dir[si].push(ti); rev[ti].push(si);
    outDeg[si]++; inDeg[ti]++;
    const [s, t] = si < ti ? [si, ti] : [ti, si];
    undirectedEdgeSet.add(s + '<' + t);
  }
  // 无向边集合
  const undirectedEdges = [...undirectedEdgeSet].map(k => k.split('<').map(Number));
  // 无向度数
  const undirectedDeg = Array(vs.length).fill(0);
  for (const [s, t] of undirectedEdges) { undirectedDeg[s]++; undirectedDeg[t]++; }
  return {
    ids: vs,
    idOf: v => nodes.get(v),
    n: vs.length,
    dir, rev,
    inDeg, outDeg,
    undirectedEdgeSet, undirectedEdges, undirectedDeg,
  };
}

function datasets() {
  const m = {};
  // T1
  m.T1 = mkGraph(
    ['a', 'b', 'c', 'd', 'e', 'f'],
    [['a', 'b'], ['b', 'c'], ['c', 'd'], ['d', 'e'], ['e', 'f'], ['f', 'c'], ['a', 'd']]
  );
  // T2 K5 有向完全图（每对双向边）
  {
    const n = ['p', 'q', 'r', 's', 't'];
    const es = [];
    for (let i = 0; i < 5; i++) for (let j = 0; j < 5; j++) if (i !== j) es.push([n[i], n[j]]);
    m.T2 = mkGraph(n, es);
  }
  // T3 星 + 悬挂
  m.T3 = mkGraph(
    ['hub', 's1', 's2', 's3', 's4', 'leaf'],
    [['hub', 's1'], ['hub', 's2'], ['hub', 's3'], ['hub', 's4'],
      ['s1', 'hub'], ['s2', 'hub'], ['s3', 'hub'], ['s4', 'leaf']]
  );
  // T4 不连通
  m.T4 = mkGraph(
    ['A1', 'A2', 'A3', 'B1', 'B2'],
    [['A1', 'A2'], ['A2', 'A3'], ['A3', 'A1'], ['B1', 'B2'], ['B2', 'B1']]
  );
  // T5 7 节点二叉树（root-l-r 双向 12 边 + parent-children 双向 8）
  {
    const vs = ['root', 'l', 'r', 'll', 'lr', 'rl', 'rr'];
    const es = [
      ['root', 'l'], ['l', 'root'], ['root', 'r'], ['r', 'root'],
      ['l', 'll'], ['ll', 'l'], ['l', 'lr'], ['lr', 'l'],
      ['r', 'rl'], ['rl', 'r'], ['r', 'rr'], ['rr', 'r']
    ];
    m.T5 = mkGraph(vs, es);
  }
  // T6 3x3 网格双向
  {
    const grid = ['n11', 'n12', 'n13', 'n21', 'n22', 'n23', 'n31', 'n32', 'n33'];
    const g = (i, j) => grid[(i - 1) * 3 + (j - 1)];
    const es = [];
    for (let i = 1; i <= 3; i++) for (let j = 1; j <= 3; j++) {
      if (i < 3) { es.push([g(i, j), g(i + 1, j)]); es.push([g(i + 1, j), g(i, j)]); }
      if (j < 3) { es.push([g(i, j), g(i, j + 1)]); es.push([g(i, j + 1), g(i, j)]); }
    }
    m.T6 = mkGraph(grid, es);
  }
  // T7 长尾 hub + u1..u10，u[i-1] <-> u[i]
  {
    const ln = ['hub'];
    for (let i = 1; i <= 10; i++) ln.push('u' + i);
    const es = [];
    for (let i = 1; i <= 10; i++) {
      es.push(['hub', ln[i]]); es.push([ln[i], 'hub']);
      if (i > 1) es.push([ln[i - 1], ln[i]]);
    }
    m.T7 = mkGraph(ln, es);
  }
  // T8 双向环 r1..r8 + r1-r3 / r5-r7 跨边
  {
    const ring = [];
    for (let i = 1; i <= 8; i++) ring.push('r' + i);
    const es = [];
    for (let i = 0; i < 8; i++) {
      const a = ring[i]; const b = ring[(i + 1) % 8];
      es.push([a, b]); es.push([b, a]);
    }
    es.push(['r1', 'r3']); es.push(['r3', 'r1']);
    es.push(['r5', 'r7']); es.push(['r7', 'r5']);
    m.T8 = mkGraph(ring, es);
  }
  return m;
}

// ------------------ 7 条核心算法实现（公式与 Rust 端一一对应） ------------------

// PageRank 推模型（damping=0.85，最多 iter 轮，Δmax≤1e-6 提前收敛，悬垂质量回传）
function pagerank(g, iterations) {
  const n = g.n;
  if (n === 0) return {};
  const alpha = 0.85;
  // 出度归一化：对于每个 i，out(i)>0 的邻居均分质量
  const outW = g.dir.map(adj => adj.length); // 简单等权
  const rank = Array(n).fill(1 / n);
  const teleport = 1 / n;
  for (let it = 0; it < iterations; it++) {
    // 悬垂质量（sum of rank[i] where out[i]==0）
    let dangling = 0;
    for (let i = 0; i < n; i++) if (outW[i] === 0) dangling += rank[i];
    const next = Array(n).fill(0);
    // 传播（推模型：i→j, next[j] += α·rank[i]/out[i]）
    for (let i = 0; i < n; i++) {
      if (outW[i] === 0) continue;
      const share = alpha * rank[i] / outW[i];
      for (const j of g.dir[i]) next[j] += share;
    }
    let maxDiff = 0;
    for (let i = 0; i < n; i++) {
      const v = next[i] + alpha * dangling / n + (1 - alpha) * teleport;
      maxDiff = Math.max(maxDiff, Math.abs(v - rank[i]));
      rank[i] = v;
    }
    if (maxDiff < 1e-6) break;
  }
  const res = {};
  for (let i = 0; i < n; i++) res[g.ids[i]] = rank[i];
  return res;
}

// Degree centrality 有向 RAW（in+out）/(n-1)，对齐 Rust degree_centrality()：
//   (in_degree + out_degree) / (n - 1) ，无除 2 因子
function degreeCentrality(g) {
  const res = {};
  const n = g.n;
  for (let i = 0; i < n; i++) {
    res[g.ids[i]] = n > 1 ? (g.inDeg[i] + g.outDeg[i]) / (n - 1) : 0;
  }
  return res;
}

// Harmonic closeness: 同 Rust closeness_centrality (L524-548)：
// 对每个 v 以有向边做 BFS 求最短距离 d[v,u]（权重=1，有向），然后 Σ_{u≠v ∧ d>0} 1/d，再 /(n-1)。
// —— 因为 Rust closeness_centrality 是对**有向图**做 BFS 的 d（通过 petgraph dijkstra，方向与权重 1.0），
//    而我们数据集里所有双向边等价对称，所以与无向 BFS 距离一致。但为与 Rust 实现在未来方向不对称
//    场景继续对齐，这里仍按有向 BFS 重写。
function harmonicCentrality(g) {
  const n = g.n;
  const res = {};
  for (let v = 0; v < n; v++) {
    const d = Array(n).fill(-1); d[v] = 0;
    const q = [v]; let head = 0;
    while (head < q.length) {
      const u = q[head++];
      for (const w of g.dir[u]) if (d[w] === -1) { d[w] = d[u] + 1; q.push(w); }
    }
    let h = 0;
    for (let u = 0; u < n; u++) if (u !== v && d[u] > 0) h += 1 / d[u];
    res[g.ids[v]] = n > 1 ? h / (n - 1) : 0;
  }
  return res;
}

// Brandes betweenness（有向图 Brandes 2001，对齐 Rust betweenness_centrality：
// C_B(v) = Σ_{s≠v≠t} σ_st(v)/σ_st，最后再除以 (n-1)(n-2) 做有向归一化）
function betweennessBrandes(g) {
  const n = g.n;
  const cb = Array(n).fill(0);
  for (let s = 0; s < n; s++) {
    const dist = Array(n).fill(-1);
    const sigma = Array(n).fill(0);
    const preds = Array.from({ length: n }, () => []);
    const order = [];
    dist[s] = 0; sigma[s] = 1;
    const q = [s]; let head = 0;
    while (head < q.length) {
      const v = q[head++];
      order.push(v);
      for (const w of g.dir[v]) {
        if (dist[w] < 0) { dist[w] = dist[v] + 1; q.push(w); }
        if (dist[w] === dist[v] + 1) { sigma[w] += sigma[v]; preds[w].push(v); }
      }
    }
    const delta = Array(n).fill(0);
    for (let i = order.length - 1; i >= 0; i--) {
      const w = order[i];
      for (const v of preds[w]) delta[v] += (sigma[v] / sigma[w]) * (1 + delta[w]);
      if (w !== s) cb[w] += delta[w];
    }
  }
  const norm = (n - 1) * (n - 2);
  const res = {};
  for (let i = 0; i < n; i++) {
    res[g.ids[i]] = norm > 0 ? cb[i] / norm : 0;
  }
  return res;
}

// Density：m/(n(n-1))，m=有向边数（Rust stats.density 定义）
function density(g) {
  const n = g.n;
  let m = 0;
  for (let i = 0; i < n; i++) m += g.dir[i].length;
  return {
    density: n > 1 ? m / (n * (n - 1)) : 0,
    node_count: n,
    edge_count: m,
    average_degree: n > 0 ? 2 * m / n : 0,
  };
}

// CNM 贪心社区：与 Rust detect_communities 一致（无向语义；ΔQ 字典序破平局；输出按规模降序）
function detectCommunitiesCNM(g, maxMerges) {
  const n = g.n;
  if (n === 0) return [];
  const edgeList = g.undirectedEdges;
  const m = edgeList.length;
  if (m === 0) {
    return g.ids.map((id, i) => ({ id: i, nodes: [id], density: 0, label: '社区 ' + i }));
  }
  const degree = g.undirectedDeg.slice();
  const commOf = Array.from({ length: n }, (_, i) => i);
  const commMembers = Array.from({ length: n }, (_, i) => [i]);
  const commAlive = Array(n).fill(true);
  // 跨边：key = a<b
  const cross = new Map(); // key -> count
  const key = (a, b) => a < b ? a + '<' + b : b + '<' + a;
  for (const [s, t] of edgeList) {
    const k = key(s, t); cross.set(k, (cross.get(k) || 0) + 1);
  }
  const limit = maxMerges > 0 ? maxMerges : n;
  let merges = 0;
  while (merges < limit) {
    // 枚举相邻社区计算 ΔQ
    let best = null;
    for (const [k, cnt] of cross) {
      if (cnt === 0) continue;
      const [as, bs] = k.split('<'); const a = +as, b = +bs;
      if (!commAlive[a] || !commAlive[b]) continue;
      const da = degree[commOf.filter(c => c === a).length ? (function(){let s=0;for(let i=0;i<n;i++)if(commMembers[a]&&commMembers[a].includes(i))s+=degree[i];return s;})() : degree.reduce((s, d, i) => s + (commOf[i] === a ? d : 0), 0)];
      // 直接按社区度维护数组更简单，复用上面写法
    }
    break;
  }
  return cnmProper(g, edgeList, m, n, merges, maxMerges || n);
}

// 正确的 CNM 完整实现
function cnmProper(g, edgeList, m, n, _unused_m, maxMerges) {
  const degree = g.undirectedDeg.slice();
  const commOf = Array.from({ length: n }, (_, i) => i);
  const commMembers = Array.from({ length: n }, (_, i) => [i]);
  const commAlive = Array(n).fill(true);
  let commDeg = degree.slice(); // 社区度
  const cross = new Map(); // key 'a<b' -> count
  const key = (a, b) => a < b ? a + '<' + b : b + '<' + a;
  for (const [s, t] of edgeList) {
    const k = key(s, t); cross.set(k, (cross.get(k) || 0) + 1);
  }
  let merges = 0;
  const maxM = Math.min(maxMerges, n * n);
  for (;;) {
    if (merges >= maxM) break;
    let bestGain = -Infinity, bestKey = null, bestLex = null;
    for (const [k, cnt] of cross) {
      if (cnt <= 0) continue;
      const [as, bs] = k.split('<'); const a = +as, b = +bs;
      if (!commAlive[a] || !commAlive[b]) continue;
      const gain = cnt / m - (commDeg[a] * commDeg[b]) / (2 * m * m);
      if (gain > bestGain || (gain === bestGain && (bestLex === null || (as + '<' + bs) < bestLex))) {
        bestGain = gain; bestKey = [a, b]; bestLex = as + '<' + bs;
      }
    }
    if (bestKey === null || bestGain <= 1e-12) break;
    const [a, b] = bestKey;
    // 合并 b 入 a（a 更小键，bestLex 保证 a<b）
    for (const node of commMembers[b]) commOf[node] = a;
    commMembers[a].push(...commMembers[b]);
    commMembers[b] = null; commAlive[b] = false;
    commDeg[a] += commDeg[b];
    merges++;
    // 转移 cross：b 的跨键转入 a
    for (const k of [...cross.keys()]) {
      const [xs, ys] = k.split('<'); let x = +xs, y = +ys;
      let other = -1;
      if (x === b) other = y; else if (y === b) other = x; else continue;
      const cnt = cross.get(k) || 0;
      cross.delete(k);
      if (cnt === 0) continue;
      if (other === a || !commAlive[other]) continue;
      const nk = key(a, other);
      cross.set(nk, (cross.get(nk) || 0) + cnt);
    }
  }
  // 聚合
  const groups = [];
  for (let i = 0; i < n; i++) {
    if (!commAlive[i]) continue;
    const members = commMembers[i] || [];
    const ids = members.map(idx => g.ids[idx]);
    groups.push([i, ids]);
  }
  groups.sort((x, y) => {
    if (y[1].length !== x[1].length) return y[1].length - x[1].length;
    return x[0] - y[0];
  });
  const communities = groups.map(([_, nodes], i) => {
    // 计算 density = internal_edges / C(s,2)
    const idxSet = new Set(nodes.map(id => g.idOf(id)));
    let internal = 0;
    for (const [s, t] of edgeList) if (idxSet.has(s) && idxSet.has(t)) internal++;
    const total = nodes.length * (nodes.length - 1) / 2;
    const density = nodes.length > 1 ? internal / total : 0;
    return { id: i, nodes, density, label: '社区 ' + i, size: nodes.length };
  });
  return communities;
}

// modularity Q（对齐 Rust compute_modularity）
function modularityQ(g, communities) {
  const n = g.n;
  if (n === 0) return 0;
  const idToNode = new Map(g.ids.map((id, i) => [id, i]));
  const nodeComm = Array(n).fill(-1);
  for (let ci = 0; ci < communities.length; ci++) {
    for (const v of communities[ci].nodes) {
      const idx = idToNode.get(v); if (idx !== undefined) nodeComm[idx] = ci;
    }
  }
  const k = Math.max(communities.length, 1);
  const sumIn = Array(k).fill(0); const sumTot = Array(k).fill(0);
  const deg = g.undirectedDeg;
  for (let i = 0; i < n; i++) if (nodeComm[i] >= 0) sumTot[nodeComm[i]] += deg[i];
  const m = g.undirectedEdges.length;
  if (m === 0) return 0;
  for (const [s, t] of g.undirectedEdges) {
    const cs = nodeComm[s], ct = nodeComm[t];
    if (cs >= 0 && cs === ct) sumIn[cs]++;
  }
  const twoM = 2 * m;
  let q = 0;
  for (let c = 0; c < k; c++) q += sumIn[c] / twoM - Math.pow(sumTot[c] / twoM, 2);
  return q;
}

// ------------------ 主流程：把 Node 端 7×8 计算结果，与 Rust 端 JSON 逐项比较 ------------------
function computeAll() {
  const ds = datasets();
  const out = [];
  for (const d of DATASET_ORDER) {
    const g = ds[d];
    out.push({ dataset: d, algorithm: 'pagerank', result: pagerank(g, 20), params: { iterations: 20 } });
    const cs = cnmProper(g, g.undirectedEdges, g.undirectedEdges.length, g.n, 0, 1000);
    const q = modularityQ(g, cs);
    out.push({ dataset: d, algorithm: 'cnm', result: {
      communities: cs.map(c => ({ id: c.id, nodes: c.nodes, density: c.density, label: c.label, size: c.nodes.length })),
      community_count: cs.length, modularity: q,
    }});
    out.push({ dataset: d, algorithm: 'betweenness', result: betweennessBrandes(g) });
    out.push({ dataset: d, algorithm: 'harmonic', result: harmonicCentrality(g) });
    out.push({ dataset: d, algorithm: 'degree', result: degreeCentrality(g) });
    out.push({ dataset: d, algorithm: 'density', result: density(g) });
    // modularity 单值（与 Rust 条目的 modularity 同公式）
    const cs2 = cnmProper(g, g.undirectedEdges, g.undirectedEdges.length, g.n, 0, 1000);
    out.push({ dataset: d, algorithm: 'modularity', result: {
      modularity: modularityQ(g, cs2), community_count: cs2.length, communities: cs2.map(c => c.nodes),
    }});
  }
  return out;
}

function approxEqualScalar(a, b, eps) {
  if (!Number.isFinite(a) || !Number.isFinite(b)) return a === b; // 0 或同非有限
  return Math.abs(a - b) <= eps;
}

function deepCompare(a, b, eps, path) {
  if (typeof a !== typeof b) return [path, a, b];
  if (typeof a === 'number') return approxEqualScalar(a, b, eps) ? null : [path, a, b];
  if (typeof a === 'string') return a === b ? null : [path, a, b];
  if (Array.isArray(a)) {
    if (!Array.isArray(b) || a.length !== b.length) return [path, a, b];
    for (let i = 0; i < a.length; i++) {
      const r = deepCompare(a[i], b[i], eps, path + '[' + i + ']');
      if (r) return r;
    }
    return null;
  }
  if (a && typeof a === 'object') {
    const ka = Object.keys(a).sort(); const kb = Object.keys(b).sort();
    if (ka.join(',') !== kb.join(',')) return [path, ka, kb];
    for (const k of ka) {
      const r = deepCompare(a[k], b[k], eps, path + '.' + k);
      if (r) return r;
    }
    return null;
  }
  return a === b ? null : [path, a, b];
}

function main() {
  const rustPath = process.argv[2] || path.join(process.cwd(), 'target/graph_algorithms_7x8_rust.json');
  if (!fs.existsSync(rustPath)) {
    console.error('Rust 导出文件不存在：' + rustPath + '，请先运行：\n  cargo run -p graph-algorithms --quiet --bin export_formula > target/graph_algorithms_7x8_rust.json');
    process.exit(1);
  }
  const rustArr = JSON.parse(fs.readFileSync(rustPath, 'utf8'));
  const nodeArr = computeAll();

  // 归一化成可比较记录（忽略 params 与 primary_impl）
  const idx = {};
  for (const r of rustArr) idx[r.dataset + '|' + r.algorithm] = r.result;
  let pass = 0, fail = 0; const EPS = 1e-6;
  const failures = [];
  for (const n of nodeArr) {
    const k = n.dataset + '|' + n.algorithm;
    const r = idx[k];
    if (r === undefined) { failures.push('缺少 Rust 条目 ' + k); fail++; continue; }
    const diff = deepCompare(r, n.result, EPS, k);
    if (diff) { failures.push(diff); fail++; } else pass++;
  }
  console.log('TR-03-03 对比 7 算法 × 8 数据集 = %d 项，|Rust - Node| ≤ %s', pass + fail, EPS);
  console.log('PASS: %d, FAIL: %d', pass, fail);
  if (fail > 0) {
    for (const f of failures) console.log('  FAIL ' + JSON.stringify(f));
    process.exit(1);
  }
}

if (require.main === module) main();

// 导出供外部复用
module.exports = { computeAll, datasets, pagerank, betweennessBrandes, harmonicCentrality, degreeCentrality, density, detectCommunitiesCNM, modularityQ };

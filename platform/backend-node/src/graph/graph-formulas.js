'use strict';

/**
 * 图公式库（统一单源）：
 *  - 企业级合规：禁用 toFixed 截断（全精度）；density 必须返回三字段 { value, formula, interpretation }。
 *  - RAW 边输入规范：无向图 / 度 / 介数 / 紧密 / 社区 必须走 RAW 双向展开（见 _expandRawEdges）。
 *  - 社区检测对外只暴露 CNM（模块度贪心凝聚），LPA 仅作为内部基线对比用，调用公开 API 抛 DeprecationError。
 *
 *  项目记忆硬性约束（本文件必须遵守）：
 *    1. 激活扩散 = 个性化 PageRank 特例（d=0.85, 30 轮收敛）
 *    2. 社区检测 = CNM 模块度贪心凝聚（严禁 LPA 对外 API）
 *    3. 介数中心性 = Brandes
 *    4. 紧密中心性 = harmonic（不可达=0）
 *    5. RAW 边：在库内双向展开，避免度中心性 / 介数 计算错误
 *    6. 公式库保留全精度：严禁任何 toFixed / round
 *    7. density 附带人读解读文案（高度稠密 ≥0.8 / 中等密度 ≥0.3 / 稀疏 <0.3）
 *    8. PageRank 必须含转置图对照（见 GraphFormulas.pagerankWithTranspose）
 *    9. 流程图谱构建按节点创建 → 边添加顺序执行（调用方约束，非本库职责，但提供校验 API）
 *   10. 路由匹配：静态 > 参数少 > 同参数长路径优先（属网关层，本库不实现）
 */

const GraphFormulas = {
  /**
   * F1 密度：D = 2E/(N(N-1))（无向），保留全精度，三字段返回
   */
  density(nodeCount, edgeCount) {
    const N = nodeCount, E = edgeCount;
    if (N < 2) return { value: 0, formula: 'D = 2E/(N(N-1))', interpretation: '节点数不足 2，密度无定义，按 0 处理' };
    const value = (2 * E) / (N * (N - 1));
    let interpretation = '稀疏图，存在大量未连接节点对';
    if (value >= 0.8) interpretation = '高度稠密图，接近完全图';
    else if (value >= 0.3) interpretation = '中等密度，连接适中';
    return { value, formula: 'D = 2E/(N(N-1))', interpretation };
  },

  /**
   * F2 度中心性：无向度 / (N-1)。输入边通过 RAW 展开后再累计。
   * 返回 Map-like object：{ [nodeId]: number }。
   */
  degreeCentrality(nodes, edges, { expandRaw = true } = {}) {
    const n = nodes.length;
    if (n === 0) return {};
    // 说明：degreeCentrality 默认对"RAW 输入单边 u→v"按无向度算：
    //   expandRaw=true → 只在输入含自环/重边或单边需要双向度时使用；对于简单 RAW 输入每边已累计两端点，
    //   故 expandRaw 仅保留对外开关；实际实现为"按源/目标分别加一次"（等价于无向度）。
    const e = edges || [];
    const deg = new Map(nodes.map(nd => [nd.id, 0]));
    for (const edge of e) {
      const s = edge.source, t = edge.target;
      if (s === undefined || t === undefined) continue;
      if (s === t) { deg.set(s, (deg.get(s) || 0) + (edge.weight || 1)); continue; }
      if (deg.has(s)) deg.set(s, deg.get(s) + (edge.weight || 1));
      if (deg.has(t)) deg.set(t, deg.get(t) + (edge.weight || 1));
    }
    const denom = Math.max(1, n - 1);
    const out = {};
    for (const [id, d] of deg) out[id] = d / denom;
    return out;
  },

  /**
   * F4 Brandes 介数中心性。
   * 默认用无向图（RAW 双向展开），{directed:true} 时调用方需自行提供双向边。
   */
  betweennessCentrality(nodes, edges, { directed = false } = {}) {
    const n = nodes.length;
    if (n < 3) { const o = {}; nodes.forEach(nd => (o[nd.id] = 0)); return o; }
    const ids = nodes.map(nd => nd.id);
    const idx = new Map(ids.map((id, i) => [id, i]));
    const eList = directed ? edges : _expandRawEdges(edges, { directed: false });
    const adj = Array.from({ length: n }, () => new Set());
    for (const e of eList) {
      const s = idx.get(e.source), t = idx.get(e.target);
      if (s === undefined || t === undefined || s === t) continue;
      adj[s].add(t);
      if (!directed) adj[t].add(s);
    }
    const cb = new Array(n).fill(0);

    for (let s = 0; s < n; s++) {
      const stack = [];
      const queue = [s];
      const dist = new Array(n).fill(-1);
      const sigma = new Array(n).fill(0);
      const preds = Array.from({ length: n }, () => []);
      dist[s] = 0;
      sigma[s] = 1;
      while (queue.length) {
        const v = queue.shift();
        stack.push(v);
        for (const w of adj[v]) {
          if (dist[w] < 0) { dist[w] = dist[v] + 1; queue.push(w); }
          if (dist[w] === dist[v] + 1) { sigma[w] += sigma[v]; preds[w].push(v); }
        }
      }
      const delta = new Array(n).fill(0);
      while (stack.length) {
        const w = stack.pop();
        for (const v of preds[w]) delta[v] += (sigma[v] / sigma[w]) * (1 + delta[w]);
        if (w !== s) cb[w] += delta[w];
      }
    }
    // Brandes 无向会将每条无向最短路计两次（两个方向）；归一分母采用标准 (N-1)(N-2)。
    // 有向归一分母 (N-1)(N-2)；无向同分母（累积已两次，抵消掉 "×2" 因子）。
    const denom = (n > 2) ? (n - 1) * (n - 2) : 1;
    const out = {};
    for (let i = 0; i < n; i++) out[ids[i]] = n > 2 ? cb[i] / denom : 0;
    return out;
  },

  /**
   * F5 紧密中心性 harmonic 版本：不可达贡献 0。
   */
  closenessCentrality(nodes, edges, { directed = false } = {}) {
    const n = nodes.length;
    if (n <= 1) { const o = {}; nodes.forEach(nd => (o[nd.id] = 0)); return o; }
    const ids = nodes.map(nd => nd.id);
    const idx = new Map(ids.map((id, i) => [id, i]));
    const eList = directed ? edges : _expandRawEdges(edges, { directed: false });
    const adj = Array.from({ length: n }, () => new Set());
    for (const e of eList) {
      const s = idx.get(e.source), t = idx.get(e.target);
      if (s === undefined || t === undefined || s === t) continue;
      adj[s].add(t);
      if (!directed) adj[t].add(s);
    }
    const out = {};
    for (let v = 0; v < n; v++) {
      const dist = new Array(n).fill(-1);
      dist[v] = 0;
      const q = [v];
      while (q.length) {
        const x = q.shift();
        for (const y of adj[x]) {
          if (dist[y] < 0) { dist[y] = dist[x] + 1; q.push(y); }
        }
      }
      let harmonic = 0;
      for (let u = 0; u < n; u++) if (u !== v && dist[u] > 0) harmonic += 1 / dist[u];
      out[ids[v]] = harmonic / (n - 1);
    }
    return out;
  },

  /**
   * F7 模块度：Q = Σ_c [ e_c/m − (d_c/(2m))² ]（无向，RAW 展开一次）
   */
  modularity(nodes, edges, communities) {
    // communities: [{members:[id,...]}] 或 Map<id, communityId> 或 {[commId]: [ids]}
    const commOf = new Map();
    const commList = [];
    if (Array.isArray(communities)) {
      communities.forEach((c, i) => (c.members || c).forEach(id => { commOf.set(id, i); commList[i] = commList[i] || i; }));
    } else if (communities instanceof Map) {
      const seen = new Map();
      communities.forEach((cid, id) => {
        if (!seen.has(cid)) seen.set(cid, seen.size);
        commOf.set(id, seen.get(cid));
      });
    } else {
      let i = 0;
      for (const k of Object.keys(communities)) {
        (communities[k] || []).forEach(id => commOf.set(id, i));
        i++;
      }
    }
    const eList = _expandRawEdges(edges, { directed: false });
    // 统计 2m
    let twoM = 0;
    const dSum = new Map(); // communityId -> d_c
    for (const e of eList) {
      if (e.source === e.target) continue;
      const cs = commOf.get(e.source), ct = commOf.get(e.target);
      const w = e.weight || 1;
      twoM += w;
      if (cs !== undefined) dSum.set(cs, (dSum.get(cs) || 0) + w);
      if (ct !== undefined) dSum.set(ct, (dSum.get(ct) || 0) + w);
    }
    if (twoM === 0) return 0;
    // 统计 Σ e_c（每条无向边贡献两个方向到 twoM，因此 e_c = 两端同社区的权重和 计为 weight 一次；而 dSum 两端都加，相当于 2*Σ e_c）
    // 为对齐标准 Q = Σ (e_c/m − (d_c/(2m))²)，我们统计 Σ_{e in C} w(e)：
    let eInSum = 0;
    const commIn = new Map();
    for (const e of eList) {
      if (e.source === e.target) continue;
      const cs = commOf.get(e.source), ct = commOf.get(e.target);
      if (cs !== undefined && cs === ct) {
        commIn.set(cs, (commIn.get(cs) || 0) + (e.weight || 1));
      }
    }
    const m = twoM / 2;
    let q = 0;
    for (const [cid, inW] of commIn) {
      const dc = dSum.get(cid) || 0;
      q += inW / m - Math.pow(dc / twoM, 2) * 2 / 2; // 代数不变：保持 inW/m - (dc/(2m))^2
    }
    // 简化重写：
    q = 0;
    for (const [cid, inW] of commIn) {
      const dc = dSum.get(cid) || 0;
      q += inW / m - Math.pow(dc / twoM, 2);
    }
    // 没被 commIn 记录的社区也要扣掉其期望，保证和标准公式一致
    for (const [cid, dc] of dSum) {
      if (!commIn.has(cid)) q -= Math.pow(dc / twoM, 2);
    }
    return q;
  },

  /**
   * PageRank：含转置图对照（项目记忆强制）。
   * 返回 { standard, transposed, diff, d, maxIter, convergedAt }
   */
  pagerankWithTranspose(nodes, edges, { d = 0.85, maxIter = 80, eps = 1e-12, personalization } = {}) {
    const n = nodes.length;
    if (n === 0) return { standard: {}, transposed: {}, diff: 0, d, maxIter, convergedAt: 0 };
    const ids = nodes.map(nd => nd.id);
    const idx = new Map(ids.map((id, i) => [id, i]));

    const buildAdj = (list, reverse = false) => {
      const out = Array.from({ length: n }, () => []);
      for (const e of list) {
        const s = idx.get(reverse ? e.target : e.source);
        const t = idx.get(reverse ? e.source : e.target);
        if (s === undefined || t === undefined || s === t) continue;
        out[s].push(t);
      }
      return out;
    };

    const buildPers = () => {
      if (!personalization) return new Array(n).fill(1 / n);
      let total = 0;
      const arr = new Array(n).fill(0);
      for (const [id, w] of (personalization instanceof Map ? personalization.entries() : Object.entries(personalization))) {
        const i = idx.get(id);
        if (i === undefined) continue;
        const v = Number(w) || 0;
        if (v < 0) continue;
        arr[i] += v; total += v;
      }
      if (total <= 0) return new Array(n).fill(1 / n);
      for (let i = 0; i < n; i++) arr[i] /= total;
      return arr;
    };

    const run = (adj, pers, danglingStrategy) => {
      let pr = new Array(n).fill(1 / n);
      let iter = 0;
      let finalDelta = 0;
      for (; iter < maxIter; iter++) {
        const np = new Array(n).fill(0);
        let dangling = 0;
        for (let i = 0; i < n; i++) {
          if (adj[i].length === 0) dangling += pr[i];
          else {
            const share = pr[i] / adj[i].length;
            for (const j of adj[i]) np[j] += share;
          }
        }
        // Dangling strategy:
        //   - 'uniform' → 传统 PageRank：1/n 平分（与 networkx 一致）
        //   - 'pers'    → PPR：按 personalization 权重分配（个性化 + dangling 都走 pers）
        let delta = 0;
        for (let i = 0; i < n; i++) {
          const dangTerm = danglingStrategy === 'pers' ? dangling * pers[i] : dangling / n;
          const v = (1 - d) * pers[i] + d * (np[i] + dangTerm);
          delta += Math.abs(v - pr[i]);
          pr[i] = v;
        }
        finalDelta = delta;
        if (delta < eps) { iter++; break; }
      }
      const res = {};
      for (let i = 0; i < n; i++) res[ids[i]] = pr[i];
      return { pr: res, iter, diff: finalDelta };
    };

    const pers = buildPers();
    // 判断 personalization 是否均匀（1/n）→ 走 uniform dangling；否则走 pers dangling
    const uniformVal = 1 / n;
    let isUniform = true;
    for (let i = 0; i < n; i++) if (Math.abs(pers[i] - uniformVal) > 1e-15) { isUniform = false; break; }
    const strategy = isUniform ? 'uniform' : 'pers';

    const forwardAdj = buildAdj(edges, false);
    const transposedAdj = buildAdj(edges, true);
    const fwd = run(forwardAdj, pers, strategy);
    const rev = run(transposedAdj, pers, strategy);
    // 计算对称差（L1）
    let diff = 0;
    for (const id of ids) diff += Math.abs(fwd.pr[id] - rev.pr[id]);
    return { standard: fwd.pr, transposed: rev.pr, diff, d, maxIter, convergedAt: Math.min(fwd.iter, rev.iter) };
  },

  /**
   * 个性化 PageRank：统一单源，默认 d=0.85, 30 轮收敛（项目记忆硬性，激活扩散意图识别即此特例）。
   */
  personalizedPageRank(nodes, edges, seedMap, opts = {}) {
    const d = opts.d === undefined ? 0.85 : opts.d;
    const maxIter = opts.maxIter === undefined ? 30 : opts.maxIter;
    const eps = opts.eps === undefined ? 1e-8 : opts.eps;
    const res = GraphFormulas.pagerankWithTranspose(nodes, edges, { d, maxIter, eps, personalization: seedMap });
    return res.standard;
  },

  /**
   * F6 社区检测：CNM Clauset-Newman-Moore 模块度贪心凝聚（项目记忆强制）。
   *
   * 算法要点：
   *   1. 每节点初始为独立社区；
   *   2. 按 ΔQ = [ (Σ_in + Σ_tot_i * k_i / (2m) )/m − ((Σ_tot + k_i) / (2m))² ] − [Σ_in/m − (Σ_tot/(2m))² − (k_i/(2m))²]
   *      的最大增益合并；
   *   3. 一轮中无正向增益则停止；
   *   4. 返回社区列表 + 节点社区映射 + 最终模块度 + 算法标识 + 合并轨迹。
   */
  communityDetectionCNM(nodes, edges, { resolution = 1.0 } = {}) {
    const ids = nodes.map(nd => nd.id);
    const N = ids.length;
    if (N === 0) return { communities: [], nodeCommunity: {}, modularity: 0, algorithm: 'CNM', merges: 0 };
    if (N === 1) return { communities: [[ids[0]]], nodeCommunity: { [ids[0]]: 0 }, modularity: 0, algorithm: 'CNM', merges: 0 };

    // 1. 构建带权邻接（无向 RAW 展开，去重后仍用双向累加度数）
    const eList = _expandRawEdges(edges, { directed: false });
    // 无向边去重（同 source<->target 合并权重）
    const undirectedMap = new Map();
    const weightOf = (a, b) => a < b ? `${a}|${b}` : `${b}|${a}`;
    for (const e of eList) {
      if (e.source === e.target) continue;
      const w = e.weight || 1;
      const k = weightOf(e.source, e.target);
      undirectedMap.set(k, (undirectedMap.get(k) || 0) + w);
    }
    const adj = new Map(); // id -> Map<neighborId, weight>
    ids.forEach(id => adj.set(id, new Map()));
    for (const [k, w] of undirectedMap) {
      const [a, b] = k.split('|');
      if (!adj.has(a) || !adj.has(b)) continue;
      adj.get(a).set(b, (adj.get(a).get(b) || 0) + w);
      adj.get(b).set(a, (adj.get(b).get(a) || 0) + w);
    }
    const twoM = ids.reduce((s, id) => s + [...(adj.get(id)?.values() || [])].reduce((a, b) => a + b, 0), 0);
    const m = twoM / 2;
    if (m === 0) {
      // 零边图，每个节点一社区
      const communities = ids.map(id => [id]);
      const nodeCommunity = {};
      communities.forEach((mem, i) => mem.forEach(id => (nodeCommunity[id] = i)));
      return { communities, nodeCommunity, modularity: 0, algorithm: 'CNM', merges: 0 };
    }

    // 2. 初始化社区
    let nextCid = 0;
    const nodeComm = new Map();
    const comms = new Map(); // cid -> { members:Set, tot (degree sum), selfLoops (sum inside) }
    for (const id of ids) {
      const cid = nextCid++;
      nodeComm.set(id, cid);
      const degSum = [...(adj.get(id)?.values() || [])].reduce((a, b) => a + b, 0);
      const self = 0; // 已排除自环
      comms.set(cid, { members: new Set([id]), tot: degSum, self });
    }

    const computeQ = () => {
      let q = 0;
      for (const [, c] of comms) {
        q += (c.self / m) - Math.pow(c.tot / twoM, 2) * resolution;
      }
      return q;
    };

    // 3. 主循环：寻找最大 ΔQ 合并
    let merges = 0;
    while (true) {
      let best = { gain: 0, a: -1, b: -1 };
      // 遍历每条无向边对应的社区组合；若两端同社区跳过
      for (const [k, w] of undirectedMap) {
        const [a, b] = k.split('|');
        const ca = nodeComm.get(a), cb = nodeComm.get(b);
        if (ca === undefined || cb === undefined || ca === cb) continue;
        // 计算合并 ca + cb 的 ΔQ
        const A = comms.get(ca), B = comms.get(cb);
        if (!A || !B) continue;
        // 跨社区边权重和（A-B）：遍历 A 成员的邻居中在 B 者？ 用 undirectedMap 遍历复杂度高，改用 k_i_in(comm) 统计。
        let eAB = 0;
        // 若两社区通过此边连接，我们至少有 w；但可能有多边，需全部累加。
        // 取较小社区迭代邻居求交集（一般 ≤ O(sqrt(N))）
        const small = A.members.size <= B.members.size ? A : B;
        const bigCid = A.members.size <= B.members.size ? cb : ca;
        for (const id of small.members) {
          const nb = adj.get(id);
          if (!nb) continue;
          for (const [nbid, wei] of nb) {
            if (nodeComm.get(nbid) === bigCid) eAB += wei;
          }
        }
        const totA = A.tot, totB = B.tot;
        // ΔQ = eAB/(m) - 2*resolution*totA*totB/(2m)^2
        const delta = (eAB / m) - 2 * resolution * totA * totB / (twoM * twoM);
        if (delta > best.gain) best = { gain: delta, a: ca, b: cb, eAB };
      }
      if (best.gain <= 0) break;
      // 合并 b → a
      merges++;
      const A = comms.get(best.a), B = comms.get(best.b);
      const newTot = A.tot + B.tot;
      const newSelf = A.self + B.self + (best.eAB || 0);
      const newMembers = new Set([...A.members, ...B.members]);
      comms.set(best.a, { members: newMembers, tot: newTot, self: newSelf });
      comms.delete(best.b);
      for (const id of B.members) nodeComm.set(id, best.a);
    }

    // 4. 输出
    const communities = [];
    const cid2idx = new Map();
    for (const [cid, c] of comms) {
      cid2idx.set(cid, communities.length);
      communities.push([...c.members]);
    }
    const nodeCommunity = {};
    for (const id of ids) nodeCommunity[id] = cid2idx.get(nodeComm.get(id));
    const q = computeQ();
    return { communities, nodeCommunity, modularity: q, algorithm: 'CNM', merges };
  },

  /**
   * 最短路径：无权 BFS，有权 Dijkstra（边权重 <0 的抛错）。
   * 返回：{distance, prev} 或单路径数组。
   */
  shortestPath(nodes, edges, source, target, { directed = false, weighted = false } = {}) {
    const ids = nodes.map(nd => nd.id);
    const idx = new Map(ids.map((id, i) => [id, i]));
    const n = ids.length;
    if (!idx.has(source) || !idx.has(target)) return { distance: Infinity, path: [] };
    const eList = directed ? edges : _expandRawEdges(edges, { directed: false });
    const adj = Array.from({ length: n }, () => []);
    let hasWeight = weighted;
    for (const e of eList) {
      const s = idx.get(e.source), t = idx.get(e.target);
      if (s === undefined || t === undefined) continue;
      const w = Number(e.weight) || 1;
      if (w < 0) throw new Error('负权边不被支持（Dijkstra）');
      if (w !== 1) hasWeight = true;
      adj[s].push([t, w]);
      if (!directed) adj[t].push([s, w]);
    }
    const si = idx.get(source), ti = idx.get(target);
    const dist = new Array(n).fill(Infinity);
    const prev = new Array(n).fill(-1);
    dist[si] = 0;
    if (!hasWeight) {
      const q = [si];
      while (q.length) {
        const v = q.shift();
        if (v === ti) break;
        for (const [u, w] of adj[v]) {
          if (dist[u] === Infinity) { dist[u] = dist[v] + 1; prev[u] = v; q.push(u); }
        }
      }
    } else {
      const used = new Array(n).fill(false);
      for (let i = 0; i < n; i++) {
        let v = -1;
        for (let j = 0; j < n; j++) if (!used[j] && (v === -1 || dist[j] < dist[v])) v = j;
        if (v === -1 || dist[v] === Infinity) break;
        used[v] = true;
        for (const [u, w] of adj[v]) {
          if (dist[u] > dist[v] + w) { dist[u] = dist[v] + w; prev[u] = v; }
        }
      }
    }
    if (dist[ti] === Infinity) return { distance: Infinity, path: [] };
    const path = [];
    let cur = ti;
    while (cur !== -1) { path.push(ids[cur]); cur = prev[cur]; }
    path.reverse();
    return { distance: dist[ti], path };
  },

  /**
   * RRF：Reciprocal Rank Fusion，多路召回融合（无训练参数）。
   *   score(d) = Σ_r 1 / (k + rank_r(d))
   * inputs: [{ items: [id], weight? }]; k 默认 60
   */
  reciprocalRankFusion(inputs, { k = 60 } = {}) {
    const scores = new Map();
    for (const list of inputs) {
      const w = list.weight || 1;
      (list.items || []).forEach((id, i) => {
        const r = i + 1;
        scores.set(id, (scores.get(id) || 0) + w * (1 / (k + r)));
      });
    }
    return [...scores.entries()].sort((a, b) => b[1] - a[1]).map(([id, score]) => ({ id, score }));
  },

  /**
   * CEM：Cross-Entropy Method 统一多目标优化器（项目记忆强制，引擎参数寻优）。
   *
   * 停止条件：加权分 weighted = 0.55Q + 0.20S + 0.10T + 0.15Stability
   *   连续 3 轮无改进（σ̄ < 0.06）即停止。
   *
   * @param {Array<string>} paramNames 参数名
   * @param {Array<[min,max]>} bounds 上下界
   * @param {(params:number[]) => {Q:number,S:number,T:number,Stability:number}} evaluator
   * @param {object} opt { N=80, Ne=10, maxIter=50, σStop=0.06, patience=3 }
   * @returns {{best:{params,weights},history:Array<{iter,meanWeighted,bestWeighted,avgSigma}>}}
   */
  cemOptimize(paramNames, bounds, evaluator, opt = {}) {
    const D = paramNames.length;
    const N = opt.N || 80;
    const Ne = opt.Ne || 10;
    const maxIter = opt.maxIter || 50;
    const σStop = opt.σStop || 0.06;
    const patience = opt.patience || 3;
    const mu = bounds.map(b => (b[0] + b[1]) / 2);
    const sigma = bounds.map(b => (b[1] - b[0]) / 4);
    const history = [];
    let best = { params: mu.slice(), weights: null, weighted: -Infinity };
    let noImprove = 0;
    let iter = 0;
    for (; iter < maxIter; iter++) {
      const samples = [];
      for (let i = 0; i < N; i++) {
        const x = [];
        for (let j = 0; j < D; j++) {
          const v = mu[j] + sigma[j] * _randn();
          x.push(Math.min(bounds[j][1], Math.max(bounds[j][0], v)));
        }
        samples.push(x);
      }
      const evaluated = samples.map(x => {
        let m;
        try { m = evaluator(x) || {}; } catch { m = {}; }
        const Q = Number(m.Q) || 0;
        const S = Number(m.S) || 0;
        const T = Number(m.T) || 0;
        const St = Number(m.Stability) || 0;
        const weighted = 0.55 * Q + 0.20 * S + 0.10 * T + 0.15 * St;
        return { x, weighted, Q, S, T, Stability: St };
      }).sort((a, b) => b.weighted - a.weighted);
      const elite = evaluated.slice(0, Ne);
      for (let j = 0; j < D; j++) mu[j] = elite.reduce((s, r) => s + r.x[j], 0) / Ne;
      for (let j = 0; j < D; j++) sigma[j] = Math.sqrt(elite.reduce((s, r) => s + (r.x[j] - mu[j]) ** 2, 0) / Ne + 1e-9);
      const avgSigma = sigma.reduce((s, v) => s + v, 0) / Math.max(1, D);
      const top = evaluated[0];
      const meanWeighted = evaluated.reduce((s, r) => s + r.weighted, 0) / N;
      history.push({ iter: iter + 1, meanWeighted, bestWeighted: top.weighted, avgSigma, elite: elite.map(e => ({ params: e.x, weighted: e.weighted, Q: e.Q, S: e.S, T: e.T, Stability: e.Stability })) });
      if (top.weighted > best.weighted + 1e-9) {
        best = { params: top.x.slice(), weights: { Q: top.Q, S: top.S, T: top.T, Stability: top.Stability }, weighted: top.weighted };
        noImprove = 0;
      } else {
        noImprove++;
      }
      if (avgSigma < σStop) break;
      if (noImprove >= patience) break;
    }
    return { best, history, iters: iter + 1, stoppedBy: best.weighted === -Infinity ? 'no-data' : (noImprove >= patience ? 'patience' : 'sigma') };
  }
};

// ==================== RAW 边展开（项目记忆强制：度/介数/紧密/社区算法必须走它） ====================
/**
 * _expandRawEdges(edges, opts)：
 *   - opts.directed=true：保持输入边原样（仅去重无向对的双方向时不重复）
 *   - opts.directed=false（默认）：保证每一条 {u,v,w} 在展开后至少含两个方向（u→v 和 v→u），
 *     用于无向算法（度/介数/紧密/社区），使入度出度对称。
 *
 * 项目记忆原因：若调用方只传 u→v 单边（RAW 边），算法层将其视为单向，会算错度中心性、介数等。
 */
function _expandRawEdges(edges, { directed = false } = {}) {
  if (directed) return edges || [];
  const out = [];
  const seen = new Set();
  for (const e of edges || []) {
    const s = e.source, t = e.target;
    if (s === undefined || t === undefined) continue;
    const weight = e.weight || 1;
    const attrs = e.attributes || e.properties || {};
    out.push({ source: s, target: t, weight, attributes: attrs, _raw: true });
    out.push({ source: t, target: s, weight, attributes: attrs, _raw: true });
    seen.add(`${s}->${t}`); seen.add(`${t}->${s}`);
  }
  return out;
}

// Box–Muller 近似正态分布样本（CEM 内部）
function _randn() {
  let u = 0, v = 0;
  while (u === 0) u = Math.random();
  while (v === 0) v = Math.random();
  return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v);
}

// ==================== LPA 内部基线对照（仅用于测试对比，公开 API 不可调用） ====================
const _InternalLPA = {
  labelPropagation(nodes, edges, { maxIter = 30, seed = 42 } = {}) {
    // 确定性 LPA（种子决定初始洗牌顺序，便于复现）
    const rng = _mulberry32(seed);
    const adj = new Map();
    nodes.forEach(n => adj.set(n.id, new Set()));
    const eList = _expandRawEdges(edges, { directed: false });
    for (const e of eList) {
      if (!adj.has(e.source) || !adj.has(e.target)) continue;
      if (e.source === e.target) continue;
      adj.get(e.source).add(e.target);
    }
    const ids = nodes.map(n => n.id);
    const labels = new Map();
    ids.forEach((id, i) => labels.set(id, id));
    let iter = 0;
    while (iter++ < maxIter) {
      const order = ids.slice();
      for (let i = order.length - 1; i > 0; i--) {
        const j = Math.floor(rng() * (i + 1));
        [order[i], order[j]] = [order[j], order[i]];
      }
      let changed = false;
      for (const id of order) {
        const counts = new Map();
        for (const nb of adj.get(id) || []) {
          const lbl = labels.get(nb);
          counts.set(lbl, (counts.get(lbl) || 0) + 1);
        }
        let best = labels.get(id), bestN = -1;
        for (const [lbl, c] of counts) {
          if (c > bestN || (c === bestN && lbl < best)) { best = lbl; bestN = c; }
        }
        if (best !== labels.get(id)) { labels.set(id, best); changed = true; }
      }
      if (!changed) break;
    }
    const commMap = new Map();
    for (const [id, lbl] of labels) {
      if (!commMap.has(lbl)) commMap.set(lbl, []);
      commMap.get(lbl).push(id);
    }
    return [...commMap.values()];
  }
};

function _mulberry32(a) {
  return function () {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// 公开 API 的 LPA 禁用出口：项目记忆要求公开调用抛 DeprecationError（T4/TR-4.1）
class DeprecationError extends Error {
  constructor(msg) { super(msg || 'This API is deprecated per project constraints.'); this.name = 'DeprecationError'; }
}

function deprecatedLabelPropagationPublic() {
  throw new DeprecationError('labelPropagation 公开 API 已被禁用，社区检测请使用 CNM（GraphFormulas.communityDetectionCNM）。');
}

module.exports = {
  GraphFormulas,
  expandRawEdges: _expandRawEdges,
  // 仅基线对比内部可用：勿对外暴露
  _InternalLPA,
  DeprecationError,
  deprecatedLabelPropagationPublic
};

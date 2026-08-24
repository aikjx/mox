'use strict';

/**
 * 路由域：知识图谱
 * /graph/* 图查询、PageRank、社区发现、中心性、导入导出与 AI 生成
 */
module.exports = function registerGraphRoutes(ctx) {
  const { path, url, gateway, uid, p, readJSON, writeJSON, ok, fail, readBody, appendLog, reg, graphAdjacency, bfsPath, pagerank, degreeCentrality, betweennessCentrality, labelPropagation, activateSpread, aiEngine, aiIntegration } = ctx;

  // 数值归一化工具（最小-最大，零常数输入退化为 0，避免 NaN）
  function _normalize(vals) {
    if (!vals || vals.length === 0) return [];
    let min = +Infinity, max = -Infinity;
    for (const v of vals) { if (v < min) min = v; if (v > max) max = v; }
    if (!isFinite(min) || !isFinite(max) || max === min) return vals.map(() => 0);
    return vals.map(v => (v - min) / (max - min));
  }
  function _normalizeObj(obj) {
    const keys = Object.keys(obj || {});
    const vals = keys.map(k => obj[k]);
    const n = _normalize(vals);
    const out = {};
    keys.forEach((k, i) => { out[k] = n[i]; });
    return out;
  }

  // 单源图公式（含人读公式 + 解读文案）—— GraphFormulas 与 aiEngine 保持一致
  let _GraphFormulas = null;
  function graphFormulas() {
    if (!_GraphFormulas) {
      // 延迟 require：避免循环依赖；与 ai-engine._computeCentrality 委托同一真相源
      _GraphFormulas = require('../ai-flow-graph').GraphFormulas;
    }
    return _GraphFormulas;
  }


  // ===== 域局部状态：图谱自动同步定时器 =====
  const autoSync = { active: false, interval: null };
  function toggleAutoSync(req, res) {
    autoSync.active = !autoSync.active;
    if (autoSync.active) {
      autoSync.interval = setInterval(() => {
        const nodes = readJSON('graph_nodes.json', []);
        const edges = readJSON('graph_edges.json', []);
        appendLog({ type: 'auto-sync', msg: 'auto sync tick', nodes: nodes.length, edges: edges.length });
      }, 3000);
    } else if (autoSync.interval) {
      clearInterval(autoSync.interval);
      autoSync.interval = null;
    }
    ok(res, { active: autoSync.active });
  }

  reg('get', '/graph', (req, res) => {
    ok(res, {
      nodes: readJSON('graph_nodes.json', []),
      edges: readJSON('graph_edges.json', [])
    });
  });

  reg('get', '/graph/stats', (req, res) => {
    const { nodes, edges } = graphAdjacency();
    const n = nodes.length;
    const m = edges.length;
    // G3 修复：无向图密度 D = 2E/(N(N-1))（RAW 边：每边贡献度数 2 给分子，故乘 2）
    // 委托 GraphFormulas 单源实现，附人读公式与解读文案
    const densityInfo = graphFormulas().density(n, m);
    const typeCounts = {};
    nodes.forEach((nd) => { typeCounts[nd.type] = (typeCounts[nd.type] || 0) + 1; });
    const degreeDist = {};
    const degMap = {};
    nodes.forEach((nd) => { degMap[nd.id] = 0; });
    edges.forEach((e) => {
      degMap[e.source] = (degMap[e.source] || 0) + 1;
      degMap[e.target] = (degMap[e.target] || 0) + 1;
    });
    Object.keys(degMap).forEach((id) => {
      const d = degMap[id];
      degreeDist[d] = (degreeDist[d] || 0) + 1;
    });
    const avgDegree = n > 0 ? Object.keys(degMap).reduce((s, k) => s + degMap[k], 0) / n : 0;
    ok(res, {
      nodes: n,
      edges: m,
      density: densityInfo.value,
      density_formula: densityInfo.formula,
      density_interpretation: densityInfo.interpretation,
      avgDegree: avgDegree,
      types: typeCounts,
      degreeDistribution: degreeDist
    });
  });

  reg('get', '/graph/centrality', (req, res) => {
    const { nodes, edges } = graphAdjacency();
    // G2 修复：新增 harmonic 紧密中心性（harmonic 算法，不可达稳健）
    // 委托 GraphFormulas 单源实现，附人读公式
    const GF = graphFormulas();
    const degree = GF.degreeCentrality(nodes, edges);
    const betweenness = GF.betweennessCentrality(nodes, edges, { directed: false });
    const closeness = GF.closenessCentrality(nodes, edges, { directed: false });
    ok(res, {
      degree,
      betweenness,
      closeness,
      formulas: {
        degree: 'C_D(v) = deg(v) / (N-1)（无向，RAW 边双向展开计度）',
        betweenness: 'C_B(v) = Σ_{s≠v≠t} σ_st(v)/σ_st  ÷ ((N-1)(N-2)/2)（Brandes 2001 算法）',
        closeness: 'C_C(v) = (Σ_{u≠v} 1/d(v,u)) / (N-1)（harmonic 版本，不可达贡献 0）'
      }
    });
  });

  // G1 修复：社区检测从 LPA 切换为 CNM（模块度贪心凝聚算法）
  //   硬约束：禁止使用 LPA（标签传播存在平局歧义与标签吞并问题）
  reg('get', '/graph/communities', (req, res) => {
    const { nodes, edges } = graphAdjacency();
    const communities = aiEngine._detectCommunities(nodes, edges, nodes.length || 1);
    const arr = communities.map((c, i) => ({
      id: 'c' + i,
      members: c.members,
      size: c.members.length,
      assignment: c.assignment
    }));
    const modularity = graphFormulas().modularity(
      nodes, edges, arr.map(a => ({ members: a.members }))
    );
    ok(res, {
      communities: arr,
      count: arr.length,
      algorithm: 'CNM (Clauset-Newman-Moore 模块度贪心凝聚)',
      algorithm_formula: 'Q = Σ_c [ e_c/m − (d_c/(2m))² ]，每轮合并 ΔQ 最大社区对',
      modularity
    });
  });

  // G7 修复：PageRank 从 graph-algos.js 本地实现迁移到 aiIntegration 统一单源实现
  //   硬约束：PageRank 实现必须包含转置图处理，确保质量沿出边方向正确传播
  reg('get', '/graph/pagerank', async (req, res) => {
    const { nodes, edges } = graphAdjacency();
    const result = await aiIntegration.graphEngine.computePersonalizedPageRank(
      { nodes, edges },
      { damping: 0.85, maxIterations: 30, topK: nodes.length }
    );
    const pr = {};
    (result.scores || []).forEach(r => { pr[r.id] = r.score; });
    const sorted = (result.scores || []).map(r => ({ id: r.id, score: r.score }));
    ok(res, {
      pagerank: pr,
      sorted,
      top10: sorted.slice(0, 10),
      algorithm: '个性化 PageRank 推模型（转置图处理，悬挂节点质量回传）',
      algorithm_formula: 'PR(i) = (1-d)·p_i + d·(Σ_{j→i} PR(j)/outDeg(j) + danglingMass/N)',
      convergence: result.convergence,
      iterations: result.iterations
    });
  });

  reg('get', '/graph/neighbors/:id', (req, res, params) => {
    const { nodes, edges, adj } = graphAdjacency();
    const id = params.id;
    if (!adj[id]) return fail(res, 404, 'node not found');
    const outNodes = adj[id].out.map((t) => nodes.find((n) => n.id === t)).filter(Boolean);
    const inNodes = adj[id].in.map((t) => nodes.find((n) => n.id === t)).filter(Boolean);
    ok(res, { id: id, outgoing: outNodes, incoming: inNodes, degree: outNodes.length + inNodes.length });
  });

  reg('get', '/graph/path', (req, res) => {
    const q = url.parse(req.url, true).query;
    const source = q.source, target = q.target;
    if (!source || !target) return fail(res, 400, 'source and target required');
    const { adj } = graphAdjacency();
    const p = bfsPath(adj, source, target);
    if (!p) return fail(res, 404, 'no path found');
    ok(res, { source: source, target: target, path: p, length: p.length - 1 });
  });

  // G6 修复：推荐从 BFS 跳距倒数升级为个性化 PageRank（多种子向量，d=0.85）
  //   硬约束：激活扩散 = 个性化 PageRank 特例（method=spread, d=0.85, 30 轮收敛）
  reg('post', '/graph/recommend', async (req, res) => {
    const body = await readBody(req);
    const seedIds = body.seeds || [];
    const { nodes, edges } = graphAdjacency();

    if (!seedIds.length) {
      ok(res, { seeds: [], recommendations: [], note: '未提供 seeds，无法推荐' });
      return;
    }
    const existingSeeds = seedIds.filter(sid => nodes.find(n => n.id === sid));
    if (!existingSeeds.length) {
      ok(res, { seeds: seedIds, recommendations: [], note: 'seeds 均不在图谱中' });
      return;
    }
    // 构造多种子个性化向量（等权 1/n）
    const personalization = {};
    existingSeeds.forEach(sid => { personalization[sid] = 1; });
    const result = await aiIntegration.graphEngine.computePersonalizedPageRank(
      { nodes, edges },
      { damping: 0.85, maxIterations: 30, personalization, topK: nodes.length }
    );
    const seedSet = new Set(existingSeeds);
    const recs = (result.scores || [])
      .filter(r => !seedSet.has(r.id))
      .map(r => ({ id: r.id, score: r.score, node: nodes.find(n => n.id === r.id) }))
      .filter(r => r.node)
      .slice(0, body.topK || 10);
    ok(res, {
      seeds: seedIds,
      valid_seeds: existingSeeds,
      algorithm: '个性化 PageRank（多种子向量，d=0.85 传模型，30 轮收敛）',
      iterations: result.iterations,
      convergence: result.convergence,
      recommendations: recs
    });
  });

  reg('post', '/graph/node', async (req, res) => {
    const body = await readBody(req);
    if (!body.id || !body.label) return fail(res, 400, 'id and label required');
    const nodes = readJSON('graph_nodes.json', []);
    if (nodes.find((n) => n.id === body.id)) return fail(res, 409, 'id exists');
    const node = Object.assign({
      type: 'operator', node_type: body.type || 'operator',
      color: '#5B8FF9', size: 8,
      created_at: new Date().toISOString()
    }, body);
    nodes.push(node);
    writeJSON('graph_nodes.json', nodes);
    appendLog({ type: 'graph', msg: 'add node ' + node.id });
    ok(res, node);
  });

  reg('post', '/graph/edge', async (req, res) => {
    const body = await readBody(req);
    if (!body.source || !body.target) return fail(res, 400, 'source and target required');
    const edges = readJSON('graph_edges.json', []);
    const edge = Object.assign({
      id: uid('graph_edges'),
      weight: 1,
      created_at: new Date().toISOString()
    }, body);
    edges.push(edge);
    writeJSON('graph_edges.json', edges);
    appendLog({ type: 'graph', msg: 'add edge ' + body.source + '->' + body.target });
    ok(res, edge);
  });

  // G6 修复：图谱激活扩散从 BFS 能量衰减升级为个性化 PageRank
  //   硬约束：method=spread 必须作为个性化 PageRank 特例，d=0.85，30 轮收敛
  reg('post', '/graph/activate', async (req, res) => {
    const body = await readBody(req);
    // 兼容：前端可能传 string（单种子）或 string[]（多种子，如多选激活）；多种子时均等分配初始激活能量
    let seedRaw = body.seed || body.seedId;
    if (!seedRaw) return fail(res, 400, 'seed required');
    const seeds = Array.isArray(seedRaw) ? seedRaw.slice() : [seedRaw];
    const { nodes, edges } = graphAdjacency();
    const validSeeds = seeds.filter(s => nodes.find(n => n.id === s));
    if (!validSeeds.length) {
      return fail(res, 404, 'seed node not found in graph: ' + seeds.join(','));
    }
    // decay 参数兼容（旧 API 曾允许传入 body.decay）：若在 (0,1) 范围则覆盖默认 0.85
    const userDecay = parseFloat(body.decay);
    const damping = (userDecay && userDecay > 0 && userDecay < 1) ? userDecay : 0.85;
    // 多种子：平均分配 personalization 初值 1.0（与老接口单种子总能量保持一致，便于结果横向比较）
    const unitWeight = 1.0 / validSeeds.length;
    const personalization = {};
    validSeeds.forEach(s => { personalization[s] = unitWeight; });
    const result = await aiIntegration.graphEngine.computePersonalizedPageRank(
      { nodes, edges },
      { damping, maxIterations: 30, personalization, topK: Math.min(nodes.length, 50) }
    );
    const energy = {};
    (result.scores || []).forEach(r => { energy[r.id] = r.score; });
    const rank = (result.topK || result.scores || []).slice(0, 20).map(r => ({
      id: r.id, energy: r.score
    }));
    // body.iterations 仅作为诊断字段保存，但实际执行硬编码 30 轮（§18-4 激活扩散约束）
    const rawIter = parseInt(body.iterations, 10);
    ok(res, {
      seed: validSeeds.length === 1 ? validSeeds[0] : validSeeds,
      seeds: validSeeds,
      energy,
      rank,
      activation: {
        method: 'spread',
        damping,
        max_iterations: 30,
        iterations: result.iterations,
        converged: result.converged,
        requested_iterations: Number.isInteger(rawIter) ? rawIter : null,
        note: '个性化 PageRank 特例（method=spread, d=0.85, 30 轮收敛）'
      }
    });
  });

  reg('get', '/graph/search', (req, res) => {
    const q = url.parse(req.url, true).query;
    const query = (q.q || '').toLowerCase();
    const limit = parseInt(q.limit, 10) || 20;
    const rerankWeight = q.spread_weight === undefined ? 0.7 : Math.max(0, Math.min(1, parseFloat(q.spread_weight) || 0));
    const nodes = readJSON('graph_nodes.json', []);
    const edges = readJSON('graph_edges.json', []);
    if (!query) return ok(res, { nodes: [], edges: [] });
    const matchedNodes = nodes.filter((n) =>
      (n.label || '').toLowerCase().indexOf(query) !== -1 ||
      (n.id || '').toLowerCase().indexOf(query) !== -1 ||
      (n.name || '').toLowerCase().indexOf(query) !== -1 ||
      (n.type || '').toLowerCase().indexOf(query) !== -1 ||
      (n.kind || '').toLowerCase().indexOf(query) !== -1 ||
      JSON.stringify(n.properties || {}).toLowerCase().indexOf(query) !== -1
    );
    const matchedEdges = edges.filter((e) =>
      (e.source || '').toLowerCase().indexOf(query) !== -1 ||
      (e.target || '').toLowerCase().indexOf(query) !== -1
    ).slice(0, limit);

    // AC-9 第 TR-7.2：激活扩散重排（默认权重 0.7）
    // 做法：
    //   a. 基础分 bm25-like：命中字段数 + 关键词位置接近前缀加分；
    //   b. 激活扩散种子 = matchedNodes 集合（每个 id → 1），做个性化 PageRank（d=0.85, 30 轮）；
    //   c. 融合：final = (1-w)*norm(baseScore) + w*norm(prScore)
    //   d. 对未命中但激活扩散得分靠前的 nodes，扩展召回（最多 25% limit），作为"新命中"。
    const { GraphFormulas } = require('../graph/graph-formulas');
    const nodeList = nodes.map(n => ({ id: n.id }));
    const edgeList = edges.map(e => ({ source: e.from || e.source, target: e.to || e.target, weight: e.weight || 1 }));
    const seedMap = {};
    matchedNodes.forEach(n => (seedMap[n.id] = 1));
    const pr = Object.keys(seedMap).length > 0 && nodeList.length > 0
      ? GraphFormulas.personalizedPageRank(nodeList, edgeList, seedMap, { d: 0.85, maxIter: 30 })
      : {};

    // baseScore
    const scores = {};
    const haystacks = (n) => [n.id, n.label, n.name, n.type, n.kind, JSON.stringify(n.properties || {})].map(s => String(s || '').toLowerCase());
    nodes.forEach(n => {
      let b = 0;
      for (const h of haystacks(n)) {
        const idx = h.indexOf(query);
        if (idx !== -1) {
          b += 1 + Math.max(0, 1 - idx / Math.max(1, h.length));
        }
      }
      scores[n.id] = b;
    });
    const baseVals = Object.values(scores);
    const baseNorm = _normalize(baseVals);
    const prVals = Object.values(pr);
    const prNormObj = _normalizeObj(pr);
    const baseNormObj = _normalizeObj(scores);
    const finalScores = {};
    nodes.forEach(n => {
      const bs = baseNormObj[n.id] || 0;
      const ps = prNormObj[n.id] || 0;
      finalScores[n.id] = (1 - rerankWeight) * bs + rerankWeight * ps;
    });

    // 召回集合：matchedNodes ∪ Top-k (未 matchedNodes 且 finalScore > 0)
    const matchedIds = new Set(matchedNodes.map(n => n.id));
    const extraCandidates = nodes
      .filter(n => !matchedIds.has(n.id) && (finalScores[n.id] || 0) > 0)
      .sort((a, b) => (finalScores[b.id] || 0) - (finalScores[a.id] || 0))
      .slice(0, Math.max(1, Math.floor(limit * 0.25)));
    const reranked = [...matchedNodes, ...extraCandidates]
      .map(n => ({ node: n, score: finalScores[n.id] || 0 }))
      .sort((a, b) => b.score - a.score)
      .slice(0, limit)
      .map(x => x.node);

    return ok(res, {
      nodes: reranked,
      edges: matchedEdges,
      query,
      spread_weight: rerankWeight,
      stats: {
        matched_count: matchedNodes.length,
        extra_from_spread: extraCandidates.length,
        base_max: baseVals.length ? Math.max(...baseVals) : 0,
        spread_max: prVals.length ? Math.max(...prVals) : 0
      },
      // 兼容老字段 shape：保持 nodes/edges/query 三数组（老客户端零改）
    });
  });

  reg('post', '/graph/auto-sync/toggle', toggleAutoSync);

  reg('get', '/graph/auto-sync/status', (req, res) => {
    ok(res, { active: autoSync.active });
  });

  reg('get', '/dialogue/sessions', (req, res) => {
    ok(res, readJSON('dialogue_sessions.json', []));
  });

  reg('get', '/ai/sessions', (req, res) => {
    ok(res, readJSON('dialogue_sessions.json', []));
  });

  reg('get', '/graph/export', (req, res) => {
    const nodes = readJSON('graph_nodes.json', []);
    const edges = readJSON('graph_edges.json', []);
    const pr = pagerank(nodes, edges, 0.85, 80);
    const comms = labelPropagation(nodes, edges);
    ok(res, {
      version: '1.0',
      exportedAt: new Date().toISOString(),
      graph: { nodes: nodes, edges: edges },
      analytics: { pagerank: pr, communities: comms }
    });
  });

  reg('post', '/graph/import', async (req, res) => {
    const body = await readBody(req);
    if (!body || !body.graph) return fail(res, 400, 'graph required');
    if (body.graph.nodes) writeJSON('graph_nodes.json', body.graph.nodes);
    if (body.graph.edges) writeJSON('graph_edges.json', body.graph.edges);
    appendLog({ type: 'graph', msg: 'import graph', nodes: body.graph.nodes ? body.graph.nodes.length : 0 });
    ok(res, { imported: true });
  });

  // AI 知识图谱自动生成
  reg('post', '/graph/ai-generate', async (req, res) => {
    const body = await readBody(req);
    const topic = body.topic || body.requirement || '';
    const description = body.description || '';
    const seedNodes = body.seed_nodes || [];
    const existingNodes = readJSON('graph_nodes.json', []);
    const existingEdges = readJSON('graph_edges.json', []);

    if (!topic) return fail(res, 400, 'topic 为必填项');

    appendLog({ type: 'graph', msg: 'ai-generate start', topic: topic });

    const systemPrompt = `你是一个知识图谱专家。请根据用户提供的主题/需求，生成一个完整的知识图谱。

返回严格的JSON格式（不要任何其他文字）：
{
  "nodes": [
    {
      "id": "node_id",
      "label": "节点名称",
      "type": "节点类型(如:概念|组件|流程|角色|数据|约束|目标|技术)",
      "description": "节点描述",
      "attributes": {"key": "value"}
    }
  ],
  "edges": [
    {
      "source": "源节点id",
      "target": "目标节点id",
      "label": "关系标签(如:包含|依赖|使用|属于|影响|流程|数据流向|约束)",
      "weight": 1.0
    }
  ],
  "summary": "图谱总结"
}

要求：
1. 生成 8-20 个节点，覆盖核心概念、组件、流程、角色、数据、约束等维度
2. 生成 10-30 条边，形成完整的关系网络
3. 节点ID使用有意义的英文标识（如 concept_user, component_frontend, process_deploy）
4. 节点类型使用：概念|组件|流程|角色|数据|约束|目标|技术|架构|业务
5. 边关系使用：包含|依赖|使用|属于|影响|流程|数据流向|约束|实现|交互`;

    const userPrompt = `请为以下主题/需求生成知识图谱：
主题：${topic}
${description ? '详细描述：' + description : ''}
${seedNodes.length ? '已有种子节点：' + seedNodes.map(n => n.id + '(' + n.label + ')').join(', ') : ''}

请生成完整的知识图谱JSON。`;

    try {
      const result = await gateway.chat({
        messages: [
          { role: 'system', content: systemPrompt },
          { role: 'user', content: userPrompt }
        ],
        expertType: 'graph',
        systemPrompt: systemPrompt,
        temperature: 0.7,
        maxTokens: 4000
      });

      let parsed = {};
      try {
        const text = (result.content || '').replace(/```json|```/g, '').trim();
        const match = text.match(/\{[\s\S]*\}/);
        if (match) parsed = JSON.parse(match[0]);
      } catch (e) {
        return fail(res, 500, 'AI 返回格式解析失败', { raw: result.content });
      }

      const newNodes = parsed.nodes || [];
      const newEdges = parsed.edges || [];

      if (newNodes.length === 0) {
        return fail(res, 500, 'AI 未生成有效节点');
      }

      const mergedNodes = [...existingNodes];
      const mergedEdges = [...existingEdges];
      const addedNodes = [];
      const addedEdges = [];

      const existingIds = new Set(existingNodes.map(n => n.id));
      for (const node of newNodes) {
        if (!existingIds.has(node.id)) {
          const enriched = {
            id: node.id,
            label: node.label || node.id,
            type: node.type || 'concept',
            description: node.description || '',
            attributes: node.attributes || {},
            community: 0,
            degree: 0,
            created_at: new Date().toISOString(),
            ai_generated: true,
            topic: topic
          };
          mergedNodes.push(enriched);
          addedNodes.push(enriched);
          existingIds.add(node.id);
        }
      }

      const existingEdgeKeys = new Set(existingEdges.map(e => `${e.source}_${e.target}`));
      for (const edge of newEdges) {
        const key = `${edge.source}_${edge.target}`;
        if (!existingEdgeKeys.has(key)) {
          const enriched = {
            id: uid('graph_edge'),
            source: edge.source,
            target: edge.target,
            label: edge.label || 'related',
            weight: edge.weight || 1.0,
            created_at: new Date().toISOString(),
            ai_generated: true
          };
          mergedEdges.push(enriched);
          addedEdges.push(enriched);
          existingEdgeKeys.add(key);
        }
      }

      writeJSON('graph_nodes.json', mergedNodes);
      writeJSON('graph_edges.json', mergedEdges);
      appendLog({ type: 'graph', msg: 'ai-generate complete', topic: topic, nodes: addedNodes.length, edges: addedEdges.length });

      const pr = pagerank(mergedNodes, mergedEdges, 0.85, 80);
      const comms = labelPropagation(mergedNodes, mergedEdges);

      ok(res, {
        success: true,
        topic: topic,
        generated: {
          nodes: addedNodes.length,
          edges: addedEdges.length
        },
        total: {
          nodes: mergedNodes.length,
          edges: mergedEdges.length
        },
        new_nodes: addedNodes,
        new_edges: addedEdges,
        summary: parsed.summary || '',
        analytics: {
          pagerank: pr,
          communities: comms
        }
      });
    } catch (e) {
      appendLog({ type: 'graph', msg: 'ai-generate failed', topic: topic, error: e.message });
      fail(res, 500, 'AI 图谱生成失败: ' + e.message);
    }
  });

};

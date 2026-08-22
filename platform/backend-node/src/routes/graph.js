'use strict';

/**
 * 路由域：知识图谱
 * /graph/* 图查询、PageRank、社区发现、中心性、导入导出与 AI 生成
 */
module.exports = function registerGraphRoutes(ctx) {
  const { path, url, gateway, uid, p, readJSON, writeJSON, ok, fail, readBody, appendLog, reg, graphAdjacency, bfsPath, pagerank, degreeCentrality, betweennessCentrality, labelPropagation, activateSpread } = ctx;


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
    const density = n > 1 ? m / (n * (n - 1)) : 0;
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
      density: density,
      avgDegree: avgDegree,
      types: typeCounts,
      degreeDistribution: degreeDist
    });
  });

  reg('get', '/graph/centrality', (req, res) => {
    const { nodes, edges } = graphAdjacency();
    ok(res, {
      degree: degreeCentrality(nodes, edges),
      betweenness: betweennessCentrality(nodes, edges)
    });
  });

  reg('get', '/graph/communities', (req, res) => {
    const { nodes, edges } = graphAdjacency();
    const communities = labelPropagation(nodes, edges);
    const arr = Object.keys(communities).map((k, i) => ({
      id: 'c' + i,
      members: communities[k],
      size: communities[k].length
    }));
    ok(res, { communities: arr, count: arr.length });
  });

  reg('get', '/graph/pagerank', (req, res) => {
    const { nodes, edges } = graphAdjacency();
    const pr = pagerank(nodes, edges, 0.85, 80);
    const sorted = Object.keys(pr).map((id) => ({ id: id, score: pr[id] })).sort((a, b) => b.score - a.score);
    ok(res, { pagerank: pr, sorted: sorted, top10: sorted.slice(0, 10) });
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

  reg('post', '/graph/recommend', async (req, res) => {
    const body = await readBody(req);
    const seedIds = body.seeds || [];
    const { nodes, edges, adj } = graphAdjacency();
    const scores = {};
    seedIds.forEach((sid) => {
      if (!adj[sid]) return;
      const visited = {};
      const q = [{ id: sid, d: 0 }];
      visited[sid] = 0;
      while (q.length) {
        const cur = q.shift();
        if (cur.d > 3) continue;
        (adj[cur.id] ? adj[cur.id].out : []).forEach((nb) => {
          if (visited[nb] === undefined) {
            visited[nb] = cur.d + 1;
            q.push({ id: nb, d: cur.d + 1 });
          }
        });
      }
      Object.keys(visited).forEach((id) => {
        if (seedIds.indexOf(id) === -1) {
          const score = 1 / (visited[id] + 1);
          scores[id] = (scores[id] || 0) + score;
        }
      });
    });
    const recs = Object.keys(scores)
      .map((id) => ({ id: id, score: scores[id], node: nodes.find((n) => n.id === id) }))
      .filter((r) => r.node)
      .sort((a, b) => b.score - a.score)
      .slice(0, body.topK || 10);
    ok(res, { seeds: seedIds, recommendations: recs });
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

  reg('post', '/graph/activate', async (req, res) => {
    const body = await readBody(req);
    const seed = body.seed || body.seedId;
    if (!seed) return fail(res, 400, 'seed required');
    const { nodes, edges } = graphAdjacency();
    const energy = activateSpread(nodes, edges, seed, body.decay || 0.7);
    const rank = Object.keys(energy).map((id) => ({ id: id, energy: energy[id] }))
      .sort((a, b) => b.energy - a.energy).slice(0, 20);
    ok(res, { seed: seed, energy: energy, rank: rank });
  });

  reg('get', '/graph/search', (req, res) => {
    const q = url.parse(req.url, true).query;
    const query = (q.q || '').toLowerCase();
    const limit = parseInt(q.limit, 10) || 20;
    const nodes = readJSON('graph_nodes.json', []);
    const edges = readJSON('graph_edges.json', []);
    if (!query) return ok(res, { nodes: [], edges: [] });
    const matchedNodes = nodes.filter((n) =>
      (n.label || '').toLowerCase().indexOf(query) !== -1 ||
      (n.id || '').toLowerCase().indexOf(query) !== -1 ||
      (n.type || '').toLowerCase().indexOf(query) !== -1
    ).slice(0, limit);
    const matchedEdges = edges.filter((e) =>
      (e.source || '').toLowerCase().indexOf(query) !== -1 ||
      (e.target || '').toLowerCase().indexOf(query) !== -1
    ).slice(0, limit);
    ok(res, { nodes: matchedNodes, edges: matchedEdges, query: query });
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

'use strict';

/**
 * Node 侧内部端点（sidecar 调用）：
 *   POST /internal/intent     → 意图识别（关键词 + 激活扩散个性化 PR，d=0.85/30 轮）
 *   POST /internal/graph-algo → 图算法分发（list_nodes / pagerank / cnm / betweenness / closeness / list_files / spread_activate ...）
 *
 * 路由注册：挂在 ctx 的 internal 域上（通过 routes/internal.js 暴露给路由索引）。
 */

const { GraphFormulas } = require('../graph/graph-formulas');
const { intentClassify } = require('../expert-alliance/domain/intent-classifier');

module.exports = function registerInternalRoutes(ctx) {
  const { ok, fail, readBody, reg } = ctx;

  // ---- Intent: query + context -> intent + capability + confidence + explain
  reg('post', '/internal/intent', async (req, res) => {
    const body = await readBody(req) || {};
    const query = typeof body.query === 'string' ? body.query.trim() : '';
    const context = body.context || {};
    try {
      const first = _keywordClassify(query);
      // 二级：激活扩散 intent→capability 图谱
      const second = _activateSpreadCapabilities(query, first);
      const intent = second.intent || first.intent || 'chat';
      const capability = second.capability || _intentToCapability(intent);
      const confidence = Math.max(first.confidence, second.confidence, 0.1);
      return ok(res, {
        ok: true,
        intent,
        confidence,
        capability,
        explain: [
          `关键词分类 intent=${first.intent} confidence=${first.confidence.toFixed?.(first.confidence)}`,
          `激活扩散能力匹配 intent=${second.intent} capability=${second.capability}`,
          `上下文: ${JSON.stringify(context).slice(0, 200)}`
        ]
      });
    } catch (e) {
      return fail(res, 500, String(e && e.message || e));
    }
  });

  // ---- Graph-algo: { algorithm, payload } → 统一响应
  reg('post', '/internal/graph-algo', async (req, res) => {
    const body = await readBody(req) || {};
    const algorithm = (body.algorithm || '').toString();
    const payload = body.payload == null ? {} : body.payload;
    try {
      const start = Date.now();
      const store = require('../storage').getStorage();
      const nodes = store.getList('graph_nodes', []);
      const edges = store.getList('graph_edges', []);
      const result = (() => {
        switch (algorithm) {
          case 'list_nodes':
            return nodes.map(n => {
              const out = {
                id: n.id,
                kind: n.kind || n.type || 'Node',
                name: n.name || n.label || n.id,
                layer: n.layer,
                kind_name: n.kind || n.type,
                tags: n.tags || [],
                labels: n.labels || [],
                description: n.description,
                properties: n.properties || {},
                created_at: n.createdAt || n.created_at,
                updated_at: n.updatedAt || n.updated_at,
                createdAt: n.createdAt || n.created_at,   // 本地字段别名（兼容老客户端）
                updatedAt: n.updatedAt || n.updated_at,   // 本地字段别名（兼容老客户端）
                degree: n.degree,
                in_degree: n.inDegree,
                out_degree: n.outDegree,
                inDegree: n.inDegree,                      // 本地字段别名
                outDegree: n.outDegree,                    // 本地字段别名
                community: n.community,
                label: n.label || n.name || n.id,
                type: n.type || n.kind,
              };
              // 超集：将原节点的其它自定义字段（若存在）原样透传
              for (const [k, v] of Object.entries(n || {})) {
                if (!(k in out)) out[k] = v;
              }
              return out;
            });
          case 'pagerank':
            return GraphFormulas.pagerankWithTranspose(
              nodes.map(idNode), edges.map(mapEdge)
            ).standard;
          case 'betweenness':
            return GraphFormulas.betweennessCentrality(nodes.map(idNode), edges.map(mapEdge));
          case 'closeness':
            return GraphFormulas.closenessCentrality(nodes.map(idNode), edges.map(mapEdge));
          case 'cnm':
            return GraphFormulas.communityDetectionCNM(nodes.map(idNode), edges.map(mapEdge));
          case 'spread_activate': {
            const seed = payload && payload.seed;
            if (!seed) throw new Error('payload.seed 缺失');
            const seedMap = {};
            if (Array.isArray(seed)) seed.forEach(s => (seedMap[s] = 1));
            else seedMap[seed] = 1;
            return GraphFormulas.personalizedPageRank(nodes.map(idNode), edges.map(mapEdge), seedMap);
          }
          case 'list_files': {
            try {
              const fMod = require('../file-store');
              const fs = typeof fMod.getFileStore === 'function' ? fMod.getFileStore() : null;
              return fs && fs.listFiles ? fs.listFiles(payload || {}) : [];
            } catch { return []; }
          }
          default:
            throw new Error(`未知算法: ${algorithm}`);
        }
      })();
      return ok(res, { ok: true, algorithm, result, timing_ms: Date.now() - start });
    } catch (e) {
      return fail(res, 500, String(e && e.message || e));
    }
  });
};
const idNode = (n) => ({ id: n.id });
const mapEdge = (e) => ({
  source: e.source !== undefined ? e.source : e.from,
  target: e.target !== undefined ? e.target : e.to,
  weight: e.weight || 1
});

function _intentToCapability(intent) {
  switch (intent) {
    case 'graph_query': return 'graph_query';
    case 'graph_list': return 'graph_list';
    case 'file_search': return 'file_graph_search';
    case 'kb_search': return 'kb_search';
    case 'chat': return 'llm_chat';
    default: return 'llm_chat';
  }
}

function _keywordClassify(query) {
  // 轻量封装：存在 intent-classifier.js → 走它；否则关键词粗分类
  if (typeof intentClassify === 'function') {
    try {
      const r = intentClassify(query) || {};
      return { intent: r.intent || 'chat', confidence: r.confidence || 0.2 };
    } catch {}
  }
  const q = (query || '').toLowerCase();
  if (/节点|图谱|graph|neighbor/.test(q)) return { intent: 'graph_query', confidence: 0.6 };
  if (/文档|文件|doc|上传/.test(q)) return { intent: 'file_search', confidence: 0.6 };
  if (/知识|检索|需求|知识库/.test(q)) return { intent: 'kb_search', confidence: 0.55 };
  return { intent: 'chat', confidence: 0.2 };
}

function _activateSpreadCapabilities(query, first) {
  // 构建 intent-keyword-capability 微图谱，做个性化 PR（d=0.85, 30 轮）
  const keywords = _extractKeywords(query);
  const intents = ['chat', 'graph_query', 'file_search', 'kb_search', 'graph_list', 'file_list', 'atlas_trace'];
  const caps = intents.map(_intentToCapability);
  const nodeSet = new Map();
  let idx = 0;
  const nodes = [];
  const makeNode = (t, id) => {
    const key = `${t}:${id}`;
    if (!nodeSet.has(key)) { nodeSet.set(key, idx++); nodes.push({ id: key }); }
  };
  keywords.forEach(k => makeNode('kw', k));
  intents.forEach(i => makeNode('intent', i));
  caps.forEach(c => makeNode('cap', c));
  const kwMap = (k) => `kw:${k}`;
  const intentId = (i) => `intent:${i}`;
  const capId = (c) => `cap:${c}`;
  const edges = [];
  // 关键词 → 意图（关键词加权）
  for (const k of keywords) {
    for (const i of intents) {
      if (_kwIntentMatch(k, i)) edges.push({ source: kwMap(k), target: intentId(i), weight: 1 });
    }
  }
  // 意图 → 能力
  intents.forEach((i, j) => edges.push({ source: intentId(i), target: capId(caps[j]), weight: 2 }));
  // 构造 seed：关键词集合 + 初意图（如果有）
  const seedMap = {};
  keywords.forEach(k => (seedMap[kwMap(k)] = 1));
  if (first && first.intent) seedMap[intentId(first.intent)] = 3;
  if (Object.keys(seedMap).length === 0) return { intent: 'chat', capability: 'llm_chat', confidence: 0.1 };
  const scores = GraphFormulas.personalizedPageRank(nodes, edges, seedMap, { d: 0.85, maxIter: 30 });
  // 取最高能力 & 最高意图
  let bestCap = { id: 'cap:llm_chat', score: -1 };
  let bestIntent = { id: 'intent:chat', score: -1 };
  for (const [key, score] of Object.entries(scores)) {
    if (key.startsWith('cap:') && score > bestCap.score) bestCap = { id: key, score };
    if (key.startsWith('intent:') && score > bestIntent.score) bestIntent = { id: key, score };
  }
  return {
    intent: bestIntent.id.replace('intent:', ''),
    capability: bestCap.id.replace('cap:', ''),
    confidence: Math.max(0.0, Math.min(1.0, bestCap.score * 3 + 0.1)),
    _debug: { bestCap, bestIntent }
  };
}

function _extractKeywords(q) {
  if (!q) return [];
  return Array.from(new Set(q.toLowerCase().split(/\s+|[^\w\u4e00-\u9fa5]/).filter(Boolean)));
}

function _kwIntentMatch(kw, intent) {
  const map = {
    graph_query: ['图谱', 'graph', '节点', '邻', '社区', '中心', 'pagerank', '路径', 'node', 'edge'],
    graph_list: ['列出', '列表', '全部', 'list', 'nodes'],
    file_search: ['文件', '文档', 'doc', '上传', '下载', '附件'],
    kb_search: ['知识', '检索', '需求', '分类', 'kb', '知识库'],
    atlas_trace: ['trace', '追踪', '全息', 'atlas', '项目', 'project'],
    file_list: ['文件列表', '列出文件', 'files'],
    chat: []
  };
  const list = map[intent] || [];
  if (list.length === 0) return kw.length < 2 ? false : false;
  return list.some(x => x.includes(kw) || kw.includes(x));
}

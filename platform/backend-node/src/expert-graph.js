'use strict';

const fs = require('fs');
const path = require('path');

const DATA_DIR = path.join(__dirname, '..', 'data');
const GRAPH_FILE = 'expert_capability_graph.json';

function readJSON(file, fallback) {
  try {
    const fp = path.join(DATA_DIR, file);
    if (!fs.existsSync(fp)) return fallback;
    const raw = fs.readFileSync(fp, 'utf8');
    return raw ? JSON.parse(raw) : fallback;
  } catch (e) {
    return fallback;
  }
}

function writeJSON(file, data) {
  try {
    fs.writeFileSync(path.join(DATA_DIR, file), JSON.stringify(data, null, 2), 'utf8');
    return true;
  } catch (e) {
    console.error('[expert-graph] writeJSON', file, e.message);
    return false;
  }
}

class ExpertGraph {
  constructor(alliance) {
    this.alliance = alliance;
    this.nodes = [];
    this.edges = [];
    this._init();
  }

  _init() {
    const saved = readJSON(GRAPH_FILE, null);
    if (saved) {
      this.nodes = saved.nodes || [];
      this.edges = saved.edges || [];
    } else {
      this._buildFromAlliance();
    }
  }

  /**
   * 能力 2-gram 提取：纯中文标签切 2 字滑窗，英文标签整体小写化。
   * 用于跨专家的"语义邻接"判定（如 '性能调优' ↔ '性能分析' 共享 gram「性能」）。
   */
  _capabilityGrams(capabilities) {
    const grams = new Set();
    for (const cap of capabilities || []) {
      const c = String(cap);
      if (/[a-zA-Z]/.test(c)) {
        const en = c.toLowerCase().match(/[a-z][a-z0-9-]{2,}/g) || [];
        en.forEach(w => grams.add(w));
      } else {
        for (let i = 0; i + 2 <= c.length; i++) grams.add(c.slice(i, i + 2));
      }
    }
    return grams;
  }

  _buildFromAlliance() {
    const experts = this.alliance ? this.alliance.listExperts() : [];
    const nodes = experts.map(e => ({
      id: e.id,
      label: e.name,
      type: e.type,
      capabilities: e.capabilities,
      status: e.status,
      metrics: e.metrics || {}
    }));

    const edges = [];
    const typeGroups = {};
    const gramCache = new Map();

    for (const expert of experts) {
      if (!typeGroups[expert.type]) typeGroups[expert.type] = [];
      typeGroups[expert.type].push(expert.id);
      gramCache.set(expert.id, this._capabilityGrams(expert.capabilities));
    }

    for (const expert of experts) {
      for (const other of experts) {
        if (expert.id >= other.id) continue;

        let weight = 0;
        let relation = null;
        let sharedCapabilities = [];

        // A19 配套修复：能力匹配由"精确相等"放宽为"包含式重叠"
        // （如 '算法设计' ↔ '图算法'：'图算法' 含子串 '算法'）。
        // 原精确匹配下 15 位专家两两零共享 → 能力图 0 边，图谱与社区检测全部失效。
        sharedCapabilities = expert.capabilities.filter(c =>
          other.capabilities.some(oc => oc === c || oc.includes(c) || c.includes(oc))
        );
        weight += sharedCapabilities.length * 2;

        // A24 增强：2-gram 语义邻接建边。包含式匹配仅能覆盖字面重叠
        // （实测 15 专家仅 2 边，CNM 凝聚出 13 个近孤立社区，协同增益失效）；
        // 语义邻接捕捉"性能调优↔性能分析"这类概念级关联，权重按共享 gram
        // 数分层（封顶 3），保持强弱可辨。实测 15 专家 20 边，密度 0.19。
        const gramsA = gramCache.get(expert.id);
        const gramsB = gramCache.get(other.id);
        const sharedGrams = gramsA
          ? Array.from(gramsA).filter(g => gramsB.has(g))
          : [];
        weight += Math.min(3, sharedGrams.length);

        const groupA = typeGroups[expert.type] || [];
        const groupB = typeGroups[other.type] || [];
        if (groupA.length > 0 && groupB.length > 0) {
          const sharedTypes = groupA.some(id => groupB.includes(id));
          if (sharedTypes) weight += 1;
        }

        if (sharedCapabilities.length > 0) {
          relation = 'capability_overlap';
        } else if (sharedGrams.length > 0) {
          relation = 'semantic_adjacent';
        } else if (weight > 0) {
          relation = 'type_related';
        }

        if (weight > 0) {
          edges.push({
            source: expert.id,
            target: other.id,
            weight,
            shared_capabilities: sharedCapabilities,
            shared_grams: sharedGrams,
            relation
          });
        }
      }
    }

    this.nodes = nodes;
    this.edges = edges;
    this._persist();
  }

  _persist() {
    writeJSON(GRAPH_FILE, { nodes: this.nodes, edges: this.edges });
  }

  getNode(id) {
    return this.nodes.find(n => n.id === id);
  }

  getNeighbors(id) {
    const neighbors = new Set();
    const neighborEdges = this.edges.filter(e => e.source === id || e.target === id);

    for (const edge of neighborEdges) {
      const otherId = edge.source === id ? edge.target : edge.source;
      neighbors.add(otherId);
    }

    return Array.from(neighbors).map(nid => this.getNode(nid)).filter(Boolean);
  }

  getCollaborationPath(sourceId, targetId) {
    const visited = new Set();
    const queue = [[sourceId, [sourceId], 0]];

    while (queue.length > 0) {
      const [currentId, path, depth] = queue.shift();
      if (currentId === targetId) {
        return { path, depth, found: true };
      }

      if (depth >= 4) continue;

      const neighbors = this.edges
        .filter(e => e.source === currentId || e.target === currentId)
        .map(e => e.source === currentId ? e.target : e.source);

      for (const neighbor of neighbors) {
        if (!visited.has(neighbor)) {
          visited.add(neighbor);
          queue.push([neighbor, [...path, neighbor], depth + 1]);
        }
      }
    }

    return { path: [], depth: -1, found: false };
  }

  findTopCollaborators(expertId, limit = 5) {
    const connected = this.edges
      .filter(e => e.source === expertId || e.target === expertId)
      .map(e => ({
        expert_id: e.source === expertId ? e.target : e.source,
        weight: e.weight,
        shared: e.shared_capabilities || []
      }))
      .sort((a, b) => b.weight - a.weight)
      .slice(0, limit);

    return connected.map(c => ({
      ...c,
      expert: this.getNode(c.expert_id)
    }));
  }

  detectCommunities() {
    // A19 修复：委托 ai-engine 的 CNM 模块度贪心凝聚单源实现。
    // 原实现为 BFS 连通分量——专家能力图近乎全连通，恒返回 1 个社区，无分析价值；
    // 且违反项目硬约束「社区检测必须使用模块度贪心凝聚（CNM）而非连通分量/LPA」。
    // 延迟 require 与 ai-engine._computeCentrality 的惯例一致（防御循环依赖）。
    const { getAIEngine } = require('./ai-engine');
    const { getGateway } = require('./llm-gateway');
    const engine = getAIEngine(getGateway());

    const cnm = engine._detectCommunities(this.nodes, this.edges, this.nodes.length || 1);
    if (!cnm.length) return [];

    return cnm.map((c, idx) => ({
      id: `community_${idx}`,
      size: c.size,
      members: c.members.map(m => this.getNode(m)).filter(Boolean),
      dominant_type: this._getDominantType(c.members)
    }));
  }

  _getDominantType(memberIds) {
    const typeCounts = {};
    for (const id of memberIds) {
      const node = this.getNode(id);
      if (node) {
        typeCounts[node.type] = (typeCounts[node.type] || 0) + 1;
      }
    }
    let dominant = null;
    let maxCount = 0;
    for (const [type, count] of Object.entries(typeCounts)) {
      if (count > maxCount) {
        maxCount = count;
        dominant = type;
      }
    }
    return dominant;
  }

  getGraphStats() {
    return {
      total_nodes: this.nodes.length,
      total_edges: this.edges.length,
      density: this.nodes.length > 0
        ? (2 * this.edges.length) / (this.nodes.length * (this.nodes.length - 1))
        : 0,
      communities: this.detectCommunities(),
      type_distribution: this.nodes.reduce((acc, n) => {
        acc[n.type] = (acc[n.type] || 0) + 1;
        return acc;
      }, {}),
      avg_collaboration_weight: this.edges.length > 0
        ? this.edges.reduce((s, e) => s + e.weight, 0) / this.edges.length
        : 0
    };
  }

  findOptimalTeam(question, size = 3) {
    const candidates = this.alliance
      ? this.alliance.listExperts().filter(e => e.status === 'active')
      : [];

    if (candidates.length === 0) return [];

    const scored = candidates.map(expert => {
      let score = 0;
      const capLower = expert.capabilities.map(c => c.toLowerCase());
      const questionLower = question.toLowerCase();

      capLower.forEach(cap => {
        if (questionLower.includes(cap)) score += 5;
      });

      if (questionLower.includes(expert.type)) score += 3;

      if (expert.metrics) {
        score += (expert.metrics.success_rate || 0.5) * 2;
        score += (expert.metrics.avg_confidence || 0.5) * 1;
      }

      return { expert, score };
    });

    scored.sort((a, b) => b.score - a.score);
    const team = scored.slice(0, size).map(s => s.expert);

    const collaborationBoost = [];
    for (let i = 0; i < team.length; i++) {
      for (let j = i + 1; j < team.length; j++) {
        const edge = this.edges.find(e =>
          (e.source === team[i].id && e.target === team[j].id) ||
          (e.source === team[j].id && e.target === team[i].id)
        );
        if (edge) {
          collaborationBoost.push({
            pair: [team[i].id, team[j].id],
            synergy: edge.weight,
            shared_capabilities: edge.shared_capabilities
          });
        }
      }
    }

    return {
      team,
      scores: scored.slice(0, size),
      collaboration_boost: collaborationBoost,
      total_synergy: collaborationBoost.reduce((s, b) => s + b.synergy, 0)
    };
  }

  rebuild() {
    this._buildFromAlliance();
    return this.getGraphStats();
  }

  export() {
    return {
      version: '2.0',
      nodes: this.nodes,
      edges: this.edges,
      stats: this.getGraphStats(),
      exported_at: new Date().toISOString()
    };
  }
}

let graphInstance = null;

function getExpertGraph(alliance) {
  if (!graphInstance) {
    graphInstance = new ExpertGraph(alliance);
  }
  return graphInstance;
}

module.exports = { ExpertGraph, getExpertGraph };

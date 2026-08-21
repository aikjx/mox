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

    const typeKeywords = {
      algorithm: ['复杂度', '算法', '数据结构'],
      architecture: ['架构', '设计模式', '系统设计'],
      data: ['数据', '数据库', '存储', '缓存'],
      ai: ['AI', '模型', '训练', '推理', 'LLM'],
      workflow: ['工作流', '流程', '编排'],
      operator: ['算子', '节点', '计算'],
      graph: ['图谱', '图算法', '关系'],
      security: ['安全', '加密', '认证', '权限'],
      performance: ['性能', '优化', '速度', '瓶颈'],
      monitor: ['监控', '日志', '指标', '告警'],
      market: ['市场', '商业', '产品'],
      mcp: ['MCP', '协议', '工具'],
      automation: ['自动化', '自动', '脚本'],
      requirement: ['需求', '分析', '业务'],
      fusion: ['融合', '集成', '综合']
    };

    for (const expert of experts) {
      if (!typeGroups[expert.type]) typeGroups[expert.type] = [];
      typeGroups[expert.type].push(expert.id);
    }

    for (const expert of experts) {
      for (const other of experts) {
        if (expert.id >= other.id) continue;

        let weight = 0;
        const sharedCapabilities = expert.capabilities.filter(c =>
          other.capabilities.some(oc => oc === c)
        );
        weight += sharedCapabilities.length * 2;

        const groupA = typeGroups[expert.type] || [];
        const groupB = typeGroups[other.type] || [];
        if (groupA.length > 0 && groupB.length > 0) {
          const sharedTypes = groupA.some(id => groupB.includes(id));
          if (sharedTypes) weight += 1;
        }

        if (weight > 0) {
          edges.push({
            source: expert.id,
            target: other.id,
            weight,
            shared_capabilities: sharedCapabilities,
            relation: sharedCapabilities.length > 0 ? 'capability_overlap' : 'type_related'
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
    const nodeIds = this.nodes.map(n => n.id);
    const adjacency = {};

    for (const id of nodeIds) {
      adjacency[id] = new Set();
    }
    for (const edge of this.edges) {
      adjacency[edge.source].add(edge.target);
      adjacency[edge.target].add(edge.source);
    }

    const communities = [];
    const visited = new Set();

    for (const id of nodeIds) {
      if (visited.has(id)) continue;
      const community = [];
      const queue = [id];

      while (queue.length > 0) {
        const current = queue.shift();
        if (visited.has(current)) continue;
        visited.add(current);
        community.push(current);

        const neighbors = adjacency[current] || new Set();
        for (const neighbor of neighbors) {
          if (!visited.has(neighbor)) {
            queue.push(neighbor);
          }
        }
      }

      if (community.length > 0) {
        communities.push(community);
      }
    }

    return communities.map((members, idx) => ({
      id: `community_${idx}`,
      size: members.length,
      members: members.map(m => this.getNode(m)).filter(Boolean),
      dominant_type: this._getDominantType(members)
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

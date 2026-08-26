'use strict';

/**
 * MOX Enterprise · 依赖图与拓扑排序
 * =================================
 * 模块依赖关系的图论分析引擎
 *
 * 核心能力：
 *  - 有向无环图（DAG）构建
 *  - Kahn 算法拓扑排序（确定启动/关闭顺序）
 *  - 循环依赖检测
 *  - 依赖层级计算（BFS 分层）
 *  - 并行启动组识别（同一层无依赖可并行）
 *  - 影响分析（修改一个模块影响哪些模块）
 *  - 依赖图可视化导出（DOT/Mermaid/JSON）
 */

const { EventEmitter } = require('events');

// ─── 边类型 ───
const EDGE_TYPE = {
  REQUIRED: 'required',     // 硬依赖
  OPTIONAL: 'optional',     // 可选依赖
  WEAK: 'weak',             // 弱依赖（运行时发现）
  CONFLICT: 'conflict',     // 冲突（不能同时加载）
};

class DependencyGraph extends EventEmitter {
  constructor() {
    super();
    this.nodes = new Map();    // nodeId -> { id, metadata, edges: Map(targetId, edgeType) }
    this.reverseEdges = new Map(); // nodeId -> Set(sourceId)
    this._cachedTopoOrder = null;
    this._dirty = true;
  }

  /**
   * 添加节点
   */
  addNode(nodeId, metadata = {}) {
    if (!this.nodes.has(nodeId)) {
      this.nodes.set(nodeId, {
        id: nodeId,
        metadata,
        edges: new Map(),
      });
      this.reverseEdges.set(nodeId, new Set());
      this._dirty = true;
    }
    return this;
  }

  /**
   * 添加有向边（source 依赖 target）
   */
  addEdge(sourceId, targetId, edgeType = EDGE_TYPE.REQUIRED) {
    this.addNode(sourceId);
    this.addNode(targetId);

    const source = this.nodes.get(sourceId);
    source.edges.set(targetId, edgeType);
    this.reverseEdges.get(targetId).add(sourceId);
    this._dirty = true;

    this.emit('graph:edge_added', { source: sourceId, target: targetId, type: edgeType });
    return this;
  }

  /**
   * 移除边
   */
  removeEdge(sourceId, targetId) {
    const source = this.nodes.get(sourceId);
    if (source) source.edges.delete(targetId);
    const reverse = this.reverseEdges.get(targetId);
    if (reverse) reverse.delete(sourceId);
    this._dirty = true;
    return this;
  }

  /**
   * 移除节点
   */
  removeNode(nodeId) {
    // 移除所有出边
    const node = this.nodes.get(nodeId);
    if (node) {
      for (const targetId of node.edges.keys()) {
        this.reverseEdges.get(targetId)?.delete(nodeId);
      }
    }
    // 移除所有入边
    const sources = this.reverseEdges.get(nodeId);
    if (sources) {
      for (const sourceId of sources) {
        this.nodes.get(sourceId)?.edges.delete(nodeId);
      }
    }
    this.nodes.delete(nodeId);
    this.reverseEdges.delete(nodeId);
    this._dirty = true;
    return this;
  }

  /**
   * 检测循环依赖（DFS 三色标记）
   * @returns {string[][]} 循环路径列表
   */
  detectCycles() {
    const cycles = [];
    const color = new Map(); // 0=white, 1=gray, 2=black
    const path = [];

    const dfs = (nodeId) => {
      color.set(nodeId, 1);
      path.push(nodeId);

      const node = this.nodes.get(nodeId);
      if (node) {
        for (const [targetId, edgeType] of node.edges) {
          if (edgeType === EDGE_TYPE.CONFLICT) continue;
          const c = color.get(targetId) || 0;
          if (c === 0) {
            dfs(targetId);
          } else if (c === 1) {
            // 发现循环
            const cycleStart = path.indexOf(targetId);
            if (cycleStart !== -1) {
              cycles.push([...path.slice(cycleStart), targetId]);
            }
          }
        }
      }

      path.pop();
      color.set(nodeId, 2);
    };

    for (const nodeId of this.nodes.keys()) {
      if ((color.get(nodeId) || 0) === 0) dfs(nodeId);
    }

    return cycles;
  }

  /**
   * 拓扑排序（Kahn 算法）
   * 返回启动顺序：被依赖的先启动
   * @returns {string[]} 排序后的节点 ID 列表
   */
  topologicalSort() {
    if (!this._dirty && this._cachedTopoOrder) return this._cachedTopoOrder;

    // 检测循环
    const cycles = this.detectCycles();
    if (cycles.length > 0) {
      throw new Error(`检测到 ${cycles.length} 个循环依赖: ${cycles.map(c => c.join(' → ')).join('; ')}`);
    }

    // 计算入度（只计硬依赖）
    const inDegree = new Map();
    for (const nodeId of this.nodes.keys()) {
      inDegree.set(nodeId, 0);
    }
    for (const [sourceId, node] of this.nodes) {
      for (const [targetId, edgeType] of node.edges) {
        if (edgeType === EDGE_TYPE.REQUIRED) {
          inDegree.set(targetId, (inDegree.get(targetId) || 0) + 1);
        }
      }
    }

    // Kahn 算法
    const queue = [];
    for (const [nodeId, degree] of inDegree) {
      if (degree === 0) queue.push(nodeId);
    }

    const result = [];
    while (queue.length > 0) {
      const nodeId = queue.shift();
      result.push(nodeId);

      const node = this.nodes.get(nodeId);
      if (node) {
        for (const [targetId, edgeType] of node.edges) {
          if (edgeType === EDGE_TYPE.REQUIRED) {
            const newDegree = (inDegree.get(targetId) || 0) - 1;
            inDegree.set(targetId, newDegree);
            if (newDegree === 0) queue.push(targetId);
          }
        }
      }
    }

    if (result.length !== this.nodes.size) {
      throw new Error('拓扑排序失败，存在无法解析的依赖');
    }

    this._cachedTopoOrder = result;
    this._dirty = false;
    return result;
  }

  /**
   * 获取反向拓扑排序（关闭顺序：先关依赖者）
   */
  reverseTopologicalSort() {
    return [...this.topologicalSort()].reverse();
  }

  /**
   * 计算依赖层级（BFS 分层）
   * 同一层的模块无相互依赖，可并行启动
   * @returns {string[][]} 层级数组，每层是可并行的模块 ID 列表
   */
  computeLayers() {
    const topo = this.topologicalSort();
    const layerMap = new Map(); // nodeId -> layerIndex

    for (const nodeId of topo) {
      const node = this.nodes.get(nodeId);
      let maxDepLayer = -1;
      if (node) {
        for (const [targetId, edgeType] of node.edges) {
          if (edgeType === EDGE_TYPE.REQUIRED) {
            const depLayer = layerMap.get(targetId);
            if (depLayer !== undefined && depLayer > maxDepLayer) {
              maxDepLayer = depLayer;
            }
          }
        }
      }
      layerMap.set(nodeId, maxDepLayer + 1);
    }

    // 按层分组
    const layers = [];
    for (const [nodeId, layer] of layerMap) {
      if (!layers[layer]) layers[layer] = [];
      layers[layer].push(nodeId);
    }

    return layers;
  }

  /**
   * 影响分析：修改 nodeId 会影响哪些模块（递归向上）
   */
  impactAnalysis(nodeId) {
    const affected = new Set();
    const stack = [nodeId];

    while (stack.length > 0) {
      const current = stack.pop();
      const sources = this.reverseEdges.get(current);
      if (sources) {
        for (const sourceId of sources) {
          if (!affected.has(sourceId)) {
            affected.add(sourceId);
            stack.push(sourceId);
          }
        }
      }
    }

    return Array.from(affected);
  }

  /**
   * 获取节点的所有依赖（递归向下）
   */
  getAllDependencies(nodeId) {
    const deps = new Set();
    const stack = [nodeId];

    while (stack.length > 0) {
      const current = stack.pop();
      const node = this.nodes.get(current);
      if (node) {
        for (const targetId of node.edges.keys()) {
          if (!deps.has(targetId)) {
            deps.add(targetId);
            stack.push(targetId);
          }
        }
      }
    }

    return Array.from(deps);
  }

  /**
   * 检测冲突（两个模块不能同时加载）
   */
  detectConflicts() {
    const conflicts = [];
    for (const [sourceId, node] of this.nodes) {
      for (const [targetId, edgeType] of node.edges) {
        if (edgeType === EDGE_TYPE.CONFLICT) {
          conflicts.push({ moduleA: sourceId, moduleB: targetId });
        }
      }
    }
    return conflicts;
  }

  /**
   * 导出为 Mermaid 流程图
   */
  toMermaid() {
    let mermaid = 'graph TD\n';
    for (const [nodeId, node] of this.nodes) {
      const label = node.metadata.label || nodeId;
      mermaid += `    ${nodeId.replace(/[^a-zA-Z0-9]/g, '_')}["${label}"]\n`;
    }
    for (const [sourceId, node] of this.nodes) {
      for (const [targetId, edgeType] of node.edges) {
        const s = sourceId.replace(/[^a-zA-Z0-9]/g, '_');
        const t = targetId.replace(/[^a-zA-Z0-9]/g, '_');
        const arrow = edgeType === EDGE_TYPE.OPTIONAL ? '-.->' :
                      edgeType === EDGE_TYPE.CONFLICT ? '--x' : '-->';
        mermaid += `    ${s} ${arrow} ${t}\n`;
      }
    }
    return mermaid;
  }

  /**
   * 导出为 DOT 格式（Graphviz）
   */
  toDOT() {
    let dot = 'digraph dependencies {\n';
    dot += '    rankdir=LR;\n';
    dot += '    node [shape=box, style=rounded];\n';
    for (const [nodeId, node] of this.nodes) {
      const label = node.metadata.label || nodeId;
      dot += `    "${nodeId}" [label="${label}"];\n`;
    }
    for (const [sourceId, node] of this.nodes) {
      for (const [targetId, edgeType] of node.edges) {
        const style = edgeType === EDGE_TYPE.OPTIONAL ? 'dashed' :
                      edgeType === EDGE_TYPE.CONFLICT ? 'dotted,color=red' : 'solid';
        dot += `    "${sourceId}" -> "${targetId}" [style=${style}];\n`;
      }
    }
    dot += '}\n';
    return dot;
  }

  /**
   * 导出为 JSON
   */
  toJSON() {
    return {
      nodes: Array.from(this.nodes.entries()).map(([id, node]) => ({
        id,
        metadata: node.metadata,
        edges: Array.from(node.edges.entries()).map(([target, type]) => ({ target, type })),
      })),
      stats: this.getStats(),
    };
  }

  /**
   * 从模块注册中心构建依赖图
   */
  static fromRegistry(registry) {
    const graph = new DependencyGraph();
    for (const [name, mod] of registry.modules) {
      graph.addNode(name, { version: mod.version, category: mod.category });
      for (const dep of mod.dependencies) {
        graph.addEdge(name, dep, EDGE_TYPE.REQUIRED);
      }
      for (const dep of mod.optionalDependencies || []) {
        graph.addEdge(name, dep, EDGE_TYPE.OPTIONAL);
      }
    }
    return graph;
  }

  /**
   * 获取统计
   */
  getStats() {
    let edgeCount = 0;
    let requiredCount = 0;
    let optionalCount = 0;
    for (const node of this.nodes.values()) {
      for (const [, edgeType] of node.edges) {
        edgeCount++;
        if (edgeType === EDGE_TYPE.REQUIRED) requiredCount++;
        if (edgeType === EDGE_TYPE.OPTIONAL) optionalCount++;
      }
    }

    let layers = [];
    let cycles = [];
    try {
      layers = this.computeLayers();
      cycles = this.detectCycles();
    } catch {}

    return {
      nodeCount: this.nodes.size,
      edgeCount,
      requiredEdges: requiredCount,
      optionalEdges: optionalCount,
      cycleCount: cycles.length,
      maxLayerDepth: layers.length,
      parallelGroups: layers.map(l => l.length),
    };
  }
}

module.exports = {
  DependencyGraph,
  EDGE_TYPE,
};

# 05 · KG 算法核心接口规范

> **版本**: v1.0 · **日期**: 2026-08-27
> **实现文件**: `platform/domains/kg/core/mox-kg-algo-core/src/lib.rs`
> **测试状态**: 18/18 PASSED · CSR PR/PPR Pearson ≥ 0.9999

## 一、核心数据结构

### KnowledgeGraph

```rust
pub struct KnowledgeGraph {
    nodes: HashMap<String, Node>,
    edges: Vec<Edge>,
    adjacency: HashMap<String, Vec<(String, f64)>>,  // 邻接表
    csr: Option<CSRGraph>,  // CSR 压缩存储（延迟构建）
}
```

### CSRGraph（Compressed Sparse Row）

```rust
pub struct CSRGraph {
    pub offsets: Vec<usize>,     // 行偏移数组（n+1）
    pub indices: Vec<usize>,     // 列索引数组（m）
    pub weights: Vec<f64>,       // 边权重数组（m）
    pub node_ids: Vec<String>,   // 节点 ID 映射
    pub id_to_idx: HashMap<String, usize>,
}
```

**设计理由**: CSR 存储将图压缩为 3 个数组，内存复杂度 O(n+m)，PageRank 幂迭代缓存友好，比邻接表快 2-3 倍。

---

## 二、算法清单

| # | 算法 | 方法 | 复杂度 | 红线合规 |
|---|---|---|---|---|
| 1 | PageRank | `pagerank(damping, iterations)` | O(n+m) × iter | ✅ CSR + Pearson ≥ 0.9999 |
| 2 | 个性化 PageRank (PPR) | `personalized_page_rank(source, damping, iterations)` | O(n+m) × iter | ✅ CSR + Pearson ≥ 0.9999 |
| 3 | Brandes 介数中心性 | `betweenness_centrality(normalized)` | O(n×m) | ✅ 强制算法 |
| 4 | Harmonic 紧密中心性 | `harmonic_closeness(normalized)` | O(n×(n+m)) | ✅ 强制算法 |
| 5 | 度中心性 | `degree_centrality(normalized)` | O(n) | ✅ |
| 6 | CNM 社区发现 | `detect_communities()` | O(m×log n) | ✅ 红线：禁用 LPA |
| 7 | Dijkstra 最短路径 | `shortest_path(src, dst)` | O((n+m)×log n) | ✅ |
| 8 | Yen's K 条路径 | `find_paths(src, dst, k)` | O(k×(n+m)×log n) | ✅ |
| 9 | 邻域子图 | `neighborhood_subgraph(center, depth, limit)` | O(b^depth) | ✅ |
| 10 | 激活扩散 | `activation_spread(start_nodes, steps, decay)` | O(steps × m) | ✅ |
| 11 | 余弦相似度 | `cosine_similarity(node1, node2)` | O(deg) | ✅ |
| 12 | RAW 双向展开 | `raw_bidirectional_expand(node)` | O(deg) | ✅ 项目强制 |

---

## 三、核心算法详解

### 3.1 PageRank（CSR 优化版）

**公式**:
$$PR(v) = \frac{1-d}{N} + d \sum_{u \in In(v)} \frac{PR(u)}{L(u)}$$

其中 $d=0.85$ 为阻尼系数，$L(u)$ 为节点 $u$ 的出度。

**方法签名**:
```rust
pub fn pagerank(&self, damping: f64, iterations: usize) -> HashMap<String, f64>
pub fn pagerank_csr(&self, damping: f64, iterations: usize) -> Vec<f64>
```

**验证**: CSR 版与 Dense 版皮尔逊相关系数 ≥ 0.9999（test_pagerack_csr_vs_dense）

---

### 3.2 个性化 PageRank (PPR)

**公式**:
$$PPR(v) = (1-d) \cdot \mathbf{1}_{v=s} + d \sum_{u \in In(v)} \frac{PPR(u)}{L(u)}$$

**方法签名**:
```rust
pub fn personalized_page_rank(&self, source: &str, damping: f64, iterations: usize) -> HashMap<String, f64>
```

**应用场景**: 知识推荐、相关实体发现、个性化搜索排序

---

### 3.3 Brandes 介数中心性

**公式**:
$$BC(v) = \sum_{s \neq v \neq t} \frac{\sigma(s \to t | v)}{\sigma(s \to t)}$$

其中 $\sigma(s \to t)$ 为 $s$ 到 $t$ 的最短路径总数，$\sigma(s \to t | v)$ 为经过 $v$ 的最短路径数。

**归一化**:
- 无向图: $BC(v) / \frac{(n-1)(n-2)}{2}$
- 有向图: $BC(v) / (n-1)(n-2)$

**方法签名**:
```rust
pub fn betweenness_centrality(&self, normalized: bool) -> HashMap<String, f64>
```

**直觉解释**: 衡量节点作为"桥梁"的重要性——介数高的节点控制着图中信息流动的瓶颈。

---

### 3.4 Harmonic 紧密中心性

**公式**:
$$HC(v) = \frac{1}{n-1} \sum_{u \neq v, u \text{ 可达}} \frac{1}{d(v, u)}$$

其中 $d(v, u)$ 为 $v$ 到 $u$ 的最短距离。

**方法签名**:
```rust
pub fn harmonic_closeness(&self, normalized: bool) -> HashMap<String, f64>
```

**与传统 Closeness 的区别**:
- 传统 Closeness: $C(v) = \frac{n-1}{\sum d(v,u)}$，不可达节点时无定义
- Harmonic: 使用距离倒数求和，天然处理不可达节点（不可达贡献 0），更适合非连通图

**直觉解释**: 衡量节点"到达其他节点的便捷程度"——harmonic 高的节点平均距离其他节点更近。

---

### 3.5 CNM 社区发现（Clauset-Newman-Moore）

**算法**: 模块度贪心凝聚（Agglomerative Greedy Modularity）

**模块度公式**:
$$Q = \frac{1}{2m} \sum_{ij} \left[ A_{ij} - \frac{k_i k_j}{2m} \right] \delta(c_i, c_j)$$

**算法步骤**:
1. 初始化：每个节点为独立社区
2. 迭代：选择使模块度增量 $\Delta Q$ 最大的两个社区合并
3. 终止：无法再增加模块度时停止

**方法签名**:
```rust
pub fn detect_communities(&self) -> CommunityResult
```

**红线合规**: ⚠️ 项目强制使用 CNM，**禁用 LPA（标签传播算法）**。LPA 结果不稳定且不可复现，CNM 以模块度为优化目标，结果确定且可解释。

---

### 3.6 最短路径（BFS / Dijkstra）

**无权图（BFS）**:
```rust
pub fn shortest_path(&self, src: &str, dst: &str) -> Option<Vec<String>>
```

**加权图（Dijkstra）**:
```rust
pub fn dijkstra(&self, src: &str) -> HashMap<String, f64>
```

---

### 3.7 Yen's K 条最短路径

**算法**: Yen's 算法——先求最短路，然后依次在最短路的每个"偏离点"上求约束最短路，取前 k 条。

**方法签名**:
```rust
pub fn find_paths(&self, src: &str, dst: &str, k: usize) -> PathResult
```

**返回结构**:
```rust
pub struct PathResult {
    pub paths: Vec<Path>,           // 路径列表（按权重升序）
    pub total_weight: f64,          // 所有路径权重之和
    pub avg_hops: f64,              // 平均跳数
    pub k_requested: usize,         // 请求的路径数
    pub k_returned: usize,          // 实际返回的路径数
}
```

---

### 3.8 邻域子图

**方法签名**:
```rust
pub fn neighborhood_subgraph(&self, center: &str, depth: usize, limit: usize) -> NeighborhoodResult
```

**返回结构**:
```rust
pub struct NeighborhoodResult {
    pub nodes: Vec<SubgraphNode>,
    pub edges: Vec<SubgraphEdge>,
    pub meta: NeighborhoodMeta,  // hops, excluded, center
}
```

**算法**: BFS 双向扩展，从中心节点出发，逐层探索邻居，直到达到 depth 或 limit。

---

## 四、图统计与解读

### GraphStats 结构

```rust
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub density: f64,
    pub density_interpretation: String,  // 密度解读文案（强制）
    pub avg_degree: f64,
    pub clustering_coefficient: f64,
    pub connected_components: usize,
    pub centrality_formulas: CentralityFormulaDoc,  // 公式文档（强制）
}
```

### 密度三档解读

| 密度范围 | 分类 | 解读文案 |
|---|---|---|
| $D \geq 0.5$ | 高度稠密 | "高度稠密图（density ≥ 0.5）：节点间连接紧密，社区结构明显，适合社区检测和子图挖掘" |
| $0.2 \leq D < 0.5$ | 中等密度 | "中等密度图（0.2 ≤ density < 0.5）：节点连接适中，兼具局部聚类和全局连通性，适合路径分析和中心性计算" |
| $D < 0.2$ | 稀疏图 | "稀疏图（density < 0.2）：节点间连接松散，适合邻域扩展查询和个性化推荐，路径查询可能存在多条不连通路径" |

### 中心性公式文档（CentralityFormulaDoc）

```rust
pub struct CentralityFormulaDoc {
    pub pagerank: FormulaEntry,
    pub betweenness: FormulaEntry,
    pub harmonic: FormulaEntry,
    pub degree: FormulaEntry,
    pub personalized_pagerank: FormulaEntry,
}

pub struct FormulaEntry {
    pub tex: String,        // LaTeX 公式
    pub intuition: String,  // 人读直觉解释
}
```

---

## 五、测试验证

### 测试套件（18/18 PASSED）

| 测试名 | 验证内容 | 阈值 |
|---|---|---|
| test_pagerank_csr_vs_dense | CSR PageRank 与 Dense 等价 | Pearson ≥ 0.9999 |
| test_ppr_csr_vs_dense | CSR PPR 与 Dense 等价 | Pearson ≥ 0.9999 |
| test_normalized_laplacian_csr_vs_dense | 规范化拉普拉斯 CSR=Dense | 元素级相等 |
| test_degree_matrix_csr_row_sum | 度矩阵 CSR 行和验证 | 精确相等 |
| test_communities | CNM 社区发现（2社区拆分） | 模块度 > 0 |
| test_betweenness_centrality | Brandes 介数计算 | 非负 + 归一化 ≤ 1 |
| test_harmonic_closeness | Harmonic 紧密计算 | 非负 + 归一化 ≤ 1 |
| test_shortest_path | BFS 最短路径 | 路径正确 |
| test_find_paths | Yen's K 条路径 | k_returned ≤ k |
| test_neighborhood_subgraph | 邻域子图 | hops ≤ depth |
| test_stats_density_interpretation | 密度解读文案 | 三档分类正确 |
| test_activation_spread | 激活扩散 | 收敛 |
| test_cosine_similarity | 余弦相似度 | [-1, 1] |
| test_raw_bidirectional_expand | RAW 双向展开 | 双向边齐全 |
| test_pagerank_convergence | PageRank 收敛 | 残差 < 1e-6 |
| test_graph_add_node_edge | 图构建 | 节点/边数正确 |
| test_csr_build | CSR 构建 | offsets/indices 正确 |
| test_empty_graph | 空图边界 | 无 panic |

### 运行测试

```bash
cargo test -p mox-kg-algo-core
```

---

## 六、性能基准

| 算法 | 节点数 | 边数 | 耗时 | 内存 |
|---|---|---|---|---|
| PageRank (CSR, 100 iter) | 10,000 | 50,000 | ~15ms | ~2MB |
| Brandes 介数 | 1,000 | 5,000 | ~120ms | ~1MB |
| CNM 社区 | 1,000 | 5,000 | ~80ms | ~500KB |
| Dijkstra | 10,000 | 50,000 | ~5ms | ~1MB |

> 基准数据为预估，实际性能以 `cargo bench` 为准。

---

## 七、使用示例

```rust
use mox_kg_algo_core::KnowledgeGraph;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 构建图
    let mut g = KnowledgeGraph::new();
    g.add_node("P0", "需求输入", "phase");
    g.add_node("P1", "立项", "phase");
    g.add_node("P2", "需求分析", "phase");
    g.add_edge("P0", "P1", 1.0, "flows_to");
    g.add_edge("P1", "P2", 1.0, "flows_to");

    // 2. PageRank
    let pr = g.pagerank(0.85, 100);
    println!("PageRank: {:?}", pr);

    // 3. 中心性（附带公式）
    let stats = g.stats();
    println!("密度解读: {}", stats.density_interpretation);
    println!("介数公式: {}", stats.centrality_formulas.betweenness.tex);

    // 4. 社区检测（CNM）
    let communities = g.detect_communities();
    println!("模块度: {}", communities.modularity);

    // 5. K 条路径
    let paths = g.find_paths("P0", "P2", 3);
    println!("找到 {} 条路径", paths.k_returned);

    Ok(())
}
```

---

*详见 [04-api-gateway-routes.md](./04-api-gateway-routes.md) 获取 KG 6接口的 HTTP API 规范。*

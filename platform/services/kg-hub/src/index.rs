//! 智能层：混合检索索引。
//!
//! 企业知识中枢的检索不能只靠关键词，也不能只靠向量：
//! - 关键词（BM25 简化版）解决**精确命名**召回（"operator-core"、"REQ-D01"）
//! - 向量余弦解决**语义近似**召回（"怎么保证需求不跑偏" → 偏离检测）
//! - 图扩散解决**关联召回**（命中一个节点，其强关联邻居也应被带出）
//!
//! 三路召回加权融合，权重可调且显式暴露分数构成，便于解释与调参——
//! 检索结果不可解释，治理就无从追责。

use std::collections::{HashMap, HashSet};

use primiflow_fusion::{EntityKind, Layer, RelKind, UnifiedGraph};
use serde::{Deserialize, Serialize};

/// 检索权重配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HybridWeights {
    pub keyword: f64,
    pub vector: f64,
    pub graph: f64,
    /// 图扩散每跳衰减
    pub hop_decay: f64,
}

impl Default for HybridWeights {
    fn default() -> Self {
        // 关键词权重最高：企业场景下精确命名召回的可信度高于语义猜测
        Self { keyword: 1.0, vector: 0.8, graph: 0.5, hop_decay: 0.5 }
    }
}

/// 查询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridQuery {
    #[serde(default)]
    pub text: String,
    /// 语义向量（可选，由外部 embedding 服务提供）
    #[serde(default)]
    pub vector: Option<Vec<f64>>,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// 限定实体类型（空 = 不限）
    #[serde(default)]
    pub kinds: Vec<EntityKind>,
    /// 限定层（空 = 不限）
    #[serde(default)]
    pub layers: Vec<Layer>,
    /// 图扩散跳数（0 = 关闭关联召回）
    #[serde(default)]
    pub expand_hops: usize,
    #[serde(default)]
    pub weights: Option<HybridWeights>,
}

fn default_top_k() -> usize {
    10
}

impl Default for HybridQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            vector: None,
            top_k: default_top_k(),
            kinds: Vec::new(),
            layers: Vec::new(),
            expand_hops: 0,
            weights: None,
        }
    }
}

/// 单条命中，分数构成全部暴露
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub layer: String,
    pub path: String,
    pub summary: String,
    pub evidence: String,
    pub score: f64,
    pub keyword_score: f64,
    pub vector_score: f64,
    pub graph_score: f64,
    /// 命中来源说明，便于解释
    pub matched_by: Vec<String>,
}

/// 混合索引。与图分离构建，图变更后需 `rebuild`。
#[derive(Debug, Default)]
pub struct HybridIndex {
    /// term → (node_id → term_freq)
    inverted: HashMap<String, HashMap<String, usize>>,
    /// node_id → 该节点总 term 数（长度归一化用）
    doc_len: HashMap<String, usize>,
    /// node_id → 语义向量
    vectors: HashMap<String, Vec<f64>>,
    doc_count: usize,
}

impl HybridIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// 从统一图构建索引：对 name/path/summary 分词建立倒排。
    pub fn build(graph: &UnifiedGraph) -> Self {
        let mut idx = Self::new();
        idx.rebuild(graph);
        idx
    }

    pub fn rebuild(&mut self, graph: &UnifiedGraph) {
        self.inverted.clear();
        self.doc_len.clear();
        self.doc_count = graph.nodes.len();
        for (id, n) in &graph.nodes {
            let text = format!("{} {} {}", n.name, n.path, n.summary);
            let terms = tokenize(&text);
            self.doc_len.insert(id.clone(), terms.len().max(1));
            for t in terms {
                *self
                    .inverted
                    .entry(t)
                    .or_default()
                    .entry(id.clone())
                    .or_insert(0) += 1;
            }
        }
    }

    /// 注入节点语义向量（来自外部 embedding 服务或 graph-algorithms 的 embedding 字段）
    pub fn put_vector(&mut self, id: &str, v: Vec<f64>) {
        self.vectors.insert(id.to_string(), v);
    }

    pub fn vector_count(&self) -> usize {
        self.vectors.len()
    }

    pub fn term_count(&self) -> usize {
        self.inverted.len()
    }

    /// 混合检索
    pub fn search(&self, graph: &UnifiedGraph, q: &HybridQuery) -> Vec<SearchHit> {
        let w = q.weights.clone().unwrap_or_default();
        let mut kw: HashMap<String, f64> = HashMap::new();
        let mut vec_s: HashMap<String, f64> = HashMap::new();

        // ── 1) 关键词召回：tf-idf 简化式，长度归一 ──
        let terms = tokenize(&q.text);
        if !terms.is_empty() && self.doc_count > 0 {
            for t in &terms {
                if let Some(postings) = self.inverted.get(t) {
                    // idf：命中文档越少，权重越高
                    let idf =
                        ((self.doc_count as f64 + 1.0) / (postings.len() as f64 + 1.0)).ln() + 1.0;
                    for (id, tf) in postings {
                        let len = *self.doc_len.get(id).unwrap_or(&1) as f64;
                        let tf_norm = *tf as f64 / len.sqrt();
                        *kw.entry(id.clone()).or_insert(0.0) += tf_norm * idf;
                    }
                }
            }
            // 归一到 [0,1]，使三路分数可加
            normalize(&mut kw);
        }

        // ── 2) 向量召回：余弦相似度 ──
        if let Some(qv) = &q.vector {
            for (id, v) in &self.vectors {
                let c = cosine(qv, v);
                if c > 0.0 {
                    vec_s.insert(id.clone(), c);
                }
            }
        }

        // ── 3) 融合 ──
        let mut total: HashMap<String, (f64, f64, f64)> = HashMap::new();
        for (id, s) in &kw {
            total.entry(id.clone()).or_insert((0.0, 0.0, 0.0)).0 = *s;
        }
        for (id, s) in &vec_s {
            total.entry(id.clone()).or_insert((0.0, 0.0, 0.0)).1 = *s;
        }

        // ── 4) 图扩散：以已召回节点为种子，按跳衰减带出邻居 ──
        if q.expand_hops > 0 && !total.is_empty() {
            let seeds: Vec<String> = total.keys().cloned().collect();
            let spread = diffuse(graph, &seeds, q.expand_hops, w.hop_decay);
            for (id, s) in spread {
                total.entry(id).or_insert((0.0, 0.0, 0.0)).2 = s;
            }
        }

        // ── 5) 过滤 + 打分 + 排序 ──
        let mut hits: Vec<SearchHit> = total
            .into_iter()
            .filter_map(|(id, (k, v, g))| {
                let n = graph.node(&id)?;
                if !q.kinds.is_empty() && !q.kinds.contains(&n.kind) {
                    return None;
                }
                if !q.layers.is_empty() && !q.layers.contains(&n.layer) {
                    return None;
                }
                let score = k * w.keyword + v * w.vector + g * w.graph;
                if score <= 0.0 {
                    return None;
                }
                let mut matched_by = Vec::new();
                if k > 0.0 {
                    matched_by.push("keyword".to_string());
                }
                if v > 0.0 {
                    matched_by.push("vector".to_string());
                }
                if g > 0.0 {
                    matched_by.push("graph".to_string());
                }
                Some(SearchHit {
                    id: id.clone(),
                    name: n.name.clone(),
                    kind: n.kind.zh().to_string(),
                    layer: n.layer.code().to_string(),
                    path: n.path.clone(),
                    summary: n.summary.clone(),
                    evidence: n.evidence.clone(),
                    score,
                    keyword_score: k,
                    vector_score: v,
                    graph_score: g,
                    matched_by,
                })
            })
            .collect();

        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                // 同分时按 id 稳定排序，保证结果可复现
                .then_with(|| a.id.cmp(&b.id))
        });
        hits.truncate(q.top_k.max(1));
        hits
    }
}

/// 分词：中英混合。英文/数字按非字母数字切分并小写；
/// 中文按**二元组（bigram）**切分，因为无分词器时 bigram 的召回质量显著优于单字。
pub fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut cjk: Vec<char> = Vec::new();

    let flush_ascii = |buf: &mut String, out: &mut Vec<String>| {
        if !buf.is_empty() {
            out.push(buf.to_ascii_lowercase());
            buf.clear();
        }
    };
    let flush_cjk = |cjk: &mut Vec<char>, out: &mut Vec<String>| {
        if cjk.is_empty() {
            return;
        }
        if cjk.len() == 1 {
            out.push(cjk[0].to_string());
        } else {
            for w in cjk.windows(2) {
                out.push(w.iter().collect());
            }
        }
        cjk.clear();
    };

    for ch in s.chars() {
        if is_cjk(ch) {
            flush_ascii(&mut buf, &mut out);
            cjk.push(ch);
        } else if ch.is_alphanumeric() {
            flush_cjk(&mut cjk, &mut out);
            buf.push(ch);
        } else {
            flush_ascii(&mut buf, &mut out);
            flush_cjk(&mut cjk, &mut out);
        }
    }
    flush_ascii(&mut buf, &mut out);
    flush_cjk(&mut cjk, &mut out);
    out
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF)
}

/// 余弦相似度。维度不等时按较短维度截断比较，避免直接返回 0 丢失召回。
pub fn cosine(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut na = 0.0;
    let mut nb = 0.0;
    for i in 0..n {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na <= 0.0 || nb <= 0.0 {
        return 0.0;
    }
    (dot / (na.sqrt() * nb.sqrt())).clamp(-1.0, 1.0)
}

fn normalize(m: &mut HashMap<String, f64>) {
    let max = m.values().cloned().fold(0.0f64, f64::max);
    if max > 0.0 {
        for v in m.values_mut() {
            *v /= max;
        }
    }
}

/// 图扩散：从种子出发做带衰减的无向 BFS，返回 node_id → 扩散分数。
/// `Bind` 边权重加倍——六维绑定是最强关联，理应优先带出。
pub fn diffuse(
    graph: &UnifiedGraph,
    seeds: &[String],
    hops: usize,
    decay: f64,
) -> HashMap<String, f64> {
    let mut adj: HashMap<&str, Vec<(&str, f64)>> = HashMap::new();
    for e in &graph.edges {
        let w = if e.kind == RelKind::Bind { 2.0 } else { 1.0 };
        adj.entry(&e.from).or_default().push((&e.to, w));
        adj.entry(&e.to).or_default().push((&e.from, w));
    }

    let mut out: HashMap<String, f64> = HashMap::new();
    let mut visited: HashSet<String> = seeds.iter().cloned().collect();
    let mut frontier: Vec<String> = seeds.to_vec();

    for h in 1..=hops {
        let mut next: Vec<String> = Vec::new();
        let level = decay.powi(h as i32);
        for cur in &frontier {
            if let Some(neis) = adj.get(cur.as_str()) {
                for (nb, w) in neis {
                    if visited.contains(*nb) {
                        continue;
                    }
                    let s = level * w;
                    let e = out.entry((*nb).to_string()).or_insert(0.0);
                    if s > *e {
                        *e = s;
                    }
                    next.push((*nb).to_string());
                }
            }
        }
        for n in &next {
            visited.insert(n.clone());
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use primiflow_fusion::{PrimitiveCoords, UnifiedEdge, UnifiedNode};

    fn node(id: &str, name: &str, summary: &str, kind: EntityKind) -> UnifiedNode {
        UnifiedNode {
            id: id.into(),
            kind,
            layer: crate::ontology::default_layer(kind),
            name: name.into(),
            path: id.into(),
            summary: summary.into(),
            evidence: id.into(),
            primitive: PrimitiveCoords::zero(),
            bind_id: None,
            external: false,
        }
    }

    fn sample() -> UnifiedGraph {
        let mut g = UnifiedGraph::new();
        g.add_node(node("A", "operator-core", "算子内核执行引擎", EntityKind::Code));
        g.add_node(node("B", "偏离检测", "需求对齐覆盖率与偏离治理", EntityKind::Function));
        g.add_node(node("C", "README", "项目说明文档", EntityKind::Doc));
        g.add_edge(UnifiedEdge {
            id: "e1".into(),
            from: "A".into(),
            to: "B".into(),
            kind: RelKind::Bind,
            label: "bind".into(),
            evidence: "t".into(),
        });
        g.add_edge(UnifiedEdge {
            id: "e2".into(),
            from: "B".into(),
            to: "C".into(),
            kind: RelKind::Reference,
            label: "ref".into(),
            evidence: "t".into(),
        });
        g
    }

    #[test]
    fn tokenize_handles_mixed_cjk_and_ascii() {
        let t = tokenize("operator-core 算子内核");
        assert!(t.contains(&"operator".to_string()));
        assert!(t.contains(&"core".to_string()));
        // 中文 bigram
        assert!(t.contains(&"算子".to_string()));
        assert!(t.contains(&"子内".to_string()));
        assert!(t.contains(&"内核".to_string()));
    }

    #[test]
    fn single_cjk_char_still_indexed() {
        let t = tokenize("图");
        assert_eq!(t, vec!["图".to_string()]);
    }

    #[test]
    fn keyword_search_finds_exact_name() {
        let g = sample();
        let idx = HybridIndex::build(&g);
        let q = HybridQuery { text: "operator-core".into(), ..Default::default() };
        let hits = idx.search(&g, &q);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "A");
        assert!(hits[0].matched_by.contains(&"keyword".to_string()));
    }

    #[test]
    fn cjk_semantic_query_hits_by_bigram() {
        let g = sample();
        let idx = HybridIndex::build(&g);
        let q = HybridQuery { text: "偏离治理".into(), ..Default::default() };
        let hits = idx.search(&g, &q);
        assert_eq!(hits[0].id, "B", "中文查询必须命中偏离检测节点");
    }

    #[test]
    fn graph_expansion_brings_related_neighbors() {
        let g = sample();
        let idx = HybridIndex::build(&g);
        // 只查 A，但开启 1 跳扩散后 B 应被带出
        let q = HybridQuery {
            text: "operator-core".into(),
            expand_hops: 1,
            ..Default::default()
        };
        let hits = idx.search(&g, &q);
        let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
        assert!(ids.contains(&"B"), "1 跳扩散应带出 Bind 邻居 B, got {ids:?}");
        let b = hits.iter().find(|h| h.id == "B").unwrap();
        assert!(b.graph_score > 0.0);
        assert!(b.matched_by.contains(&"graph".to_string()));
    }

    #[test]
    fn bind_edge_weighs_more_than_reference() {
        let g = sample();
        // 从 B 出发 1 跳：A 经 Bind(权2)、C 经 Reference(权1)
        let d = diffuse(&g, &["B".to_string()], 1, 0.5);
        assert!(d["A"] > d["C"], "Bind 关联应强于普通引用");
    }

    #[test]
    fn kind_filter_excludes_other_types() {
        let g = sample();
        let idx = HybridIndex::build(&g);
        let q = HybridQuery {
            text: "项目说明文档".into(),
            kinds: vec![EntityKind::Code],
            ..Default::default()
        };
        assert!(idx.search(&g, &q).is_empty(), "类型过滤必须生效");
    }

    #[test]
    fn vector_recall_works_and_dims_mismatch_is_tolerated() {
        let g = sample();
        let mut idx = HybridIndex::build(&g);
        idx.put_vector("C", vec![1.0, 0.0, 0.0]);
        let q = HybridQuery {
            text: String::new(),
            vector: Some(vec![1.0, 0.0]), // 维度不等，按短维截断
            ..Default::default()
        };
        let hits = idx.search(&g, &q);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "C");
        assert!(hits[0].vector_score > 0.99);
    }

    #[test]
    fn results_are_deterministic() {
        let g = sample();
        let idx = HybridIndex::build(&g);
        let q = HybridQuery { text: "算子".into(), expand_hops: 2, ..Default::default() };
        let a: Vec<String> = idx.search(&g, &q).into_iter().map(|h| h.id).collect();
        for _ in 0..5 {
            let b: Vec<String> = idx.search(&g, &q).into_iter().map(|h| h.id).collect();
            assert_eq!(a, b, "同一查询必须结果稳定，否则无法回归");
        }
    }

    #[test]
    fn empty_query_returns_nothing() {
        let g = sample();
        let idx = HybridIndex::build(&g);
        assert!(idx.search(&g, &HybridQuery::default()).is_empty());
    }
}

//! 代码骨架 · 由关联图谱自动生成（primiflow_core::assoc::primiflow_seed）
//! 溯源链路: R5 → F3 → B4 → A3 → T4 → C3
//! 数据设计: S4(Asset)
//! 说明: κ 复用检索 / 冻结（本地确定性 embedding 替代 pgvector，契约一致）。
//! 规格: primiflow/SPEC.md（§4 资产 / §5 κ 检索 / §7 模块）

use flow_ai::model::FlowGraph;
use crate::gen::schema::Asset;
use crate::gen::c4::Domain;

/// embedding 维度（生产用 pgvector 1536，本地用 512 维哈希向量做离线可跑）
const EMBED_DIM: usize = 512;

/// 一次检索命中
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub asset: Asset,
    /// 余弦相似度 0..1
    pub score: f32,
}

/// 资产服务：冻结拓扑为资产 Q，并按语义相似度 + 域硬过滤做 κ 复用检索。
/// 内部 embedding 为确定性哈希向量，替换 pgvector 时只需替换 `embed` 与存储后端。
#[derive(Debug, Default)]
pub struct AssetService {
    /// 资产 + 预计算 embedding
    store: Vec<(Asset, Vec<f32>)>,
}

impl AssetService {
    pub fn new() -> Self {
        Self { store: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// 冻结资产 Q：将拓扑落库并写入 embedding，供后续 κ 复用检索。
    /// 返回冻结后的资产记录与拓扑荷预估（节点越多、命中越深，荷越大）。
    pub fn freeze_asset(
        &mut self,
        topology_id: uuid::Uuid,
        name: impl Into<String>,
        domain: Domain,
        graph: &FlowGraph,
    ) -> (Asset, f64) {
        let graph_json = serde_json::to_string(graph).unwrap_or_default();
        let asset = Asset::new(topology_id, name, Some(domain.as_str().to_string()), graph_json);
        // 以资产名 + 所有节点名为语料生成 embedding
        let mut corpus = asset.name.clone();
        for n in &graph.nodes {
            corpus.push(' ');
            corpus.push_str(&n.name);
        }
        let emb = embed(&corpus, EMBED_DIM);
        let charge = (0.5 + graph.nodes.len() as f64 * 0.1).clamp(0.0, 5.0);
        self.store.push((asset.clone(), emb));
        (asset, charge)
    }

    /// κ 复用检索：语义相似度 Top‑K + 域硬过滤双保险（SPEC §9 风险缓解）。
    pub fn search(&self, query: &str, domain: Domain, top_k: usize) -> Vec<SearchHit> {
        let q = embed(query, EMBED_DIM);
        let mut hits: Vec<SearchHit> = self
            .store
            .iter()
            .filter(|(a, _)| match &a.domain {
                Some(d) => d == domain.as_str(), // 域硬过滤
                None => true,
            })
            .map(|(a, emb)| SearchHit { asset: a.clone(), score: cosine(&q, emb) })
            .filter(|h| h.score > SIMILARITY_THRESHOLD)
            .collect();
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        hits.into_iter().take(top_k).collect()
    }

    /// 取全部资产（AssetLibrary 浏览用）
    pub fn list(&self) -> Vec<Asset> {
        self.store.iter().map(|(a, _)| a.clone()).collect()
    }

    /// 将已冻结资产沉淀为 κ‑τ 引擎可用的知识库，供下一次涌现时检索复用。
    /// 资产名 + 节点名作为关键词实体，命中后 `generate` 会把对应子任务实例化为 SubFlow。
    pub fn to_knowledge_base(&self) -> flow_ai::primitive::KnowledgeBase {
        use flow_ai::primitive::KnowledgeBase;
        use flow_ai::topology::{Entity, EntityKind, Relation, RelationKind};
        let mut kb = KnowledgeBase::new();
        for (asset, _) in &self.store {
            let skill_id = format!("skill:asset:{}", asset.id);
            if kb.graph.entity(&skill_id).is_none() {
                kb.graph.add_entity(
                    Entity::new(&skill_id, EntityKind::Skill, asset.name.clone())
                        .with_keywords([asset.name.clone()]),
                );
            }
            if let Ok(g) = serde_json::from_str::<flow_ai::model::FlowGraph>(&asset.graph_json) {
                for n in g.nodes {
                    if n.kind.is_executable() {
                        let fid = format!("flow:asset:{}:{}", asset.id, n.id);
                        if kb.graph.entity(&fid).is_none() {
                            kb.graph.add_entity(
                                Entity::new(&fid, EntityKind::FlowNode, n.name.clone())
                                    .with_keywords([n.name.clone()]),
                            );
                            kb.graph.add_relation(Relation::new(&skill_id, &fid, RelationKind::Implements, 1.0));
                        }
                    }
                }
            }
        }
        kb
    }
}

/// 相似度阈值（低于此值不视为可复用资产）
const SIMILARITY_THRESHOLD: f32 = 0.05;

/// 确定性文本 embedding：字符 bigram → 多探针哈希到维度 → L2 归一化。
///
/// 采用 bigram（相比 unigram 大幅降低不同中文文本因哈希碰撞产生的误命中），
/// 并对每个 bigram 做 5 路独立探针，兼顾区分度与召回。
fn embed(text: &str, dim: usize) -> Vec<f32> {
    let mut v = vec![0.0_f32; dim];
    for tok in bigrams(text) {
        for seed in 0..5u64 {
            let h = fxhash_seeded(&tok, seed) as usize % dim;
            v[h] += 1.0;
        }
    }
    let norm = (v.iter().map(|x| x * x).sum::<f32>()).sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

/// 取字符级 bigram（忽略空白），CJK 与 ASCII 统一处理
fn bigrams(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut out = Vec::new();
    for w in chars.windows(2) {
        out.push(w.iter().collect::<String>());
    }
    out
}

/// 余弦相似度
fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na = (a.iter().map(|x| x * x).sum::<f32>()).sqrt();
    let nb = (b.iter().map(|x| x * x).sum::<f32>()).sqrt();
    if na > 0.0 && nb > 0.0 {
        dot / (na * nb)
    } else {
        0.0
    }
}

/// 轻量 FNV‑1a 风格哈希（带种子，确定性，无外部依赖）
fn fxhash_seeded(s: &str, seed: u64) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325 ^ seed.wrapping_mul(0x100000001b3);
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_ai::model::{FlowEdge, FlowNode, ToolKind};

    fn sample_graph() -> FlowGraph {
        let mut g = FlowGraph::new("topo:r1", "电商月度经营分析报告");
        g.add_node(FlowNode::task("a", "抓取销售数据", ToolKind::Http, 300));
        g.add_node(FlowNode::task("b", "清洗对账", ToolKind::Compute, 200));
        g.add_node(FlowNode::task("c", "生成图表报告", ToolKind::Llm, 400));
        g.add_edge(FlowEdge::seq("a", "b"));
        g.add_edge(FlowEdge::seq("b", "c"));
        g
    }

    #[test]
    fn freeze_then_search_hits_same_domain() {
        let mut svc = AssetService::new();
        let g = sample_graph();
        let (asset, charge) = svc.freeze_asset(uuid::Uuid::new_v4(), "电商月度经营分析报告", Domain::BusinessSoftware, &g);
        assert_eq!(svc.len(), 1);
        assert!(charge > 0.0);
        assert_eq!(asset.domain.as_deref(), Some("business_software"));

        let hits = svc.search("我想做一份电商经营分析报告", Domain::BusinessSoftware, 3);
        assert!(!hits.is_empty(), "同域相似查询应命中历史资产");
        assert!(hits[0].score > 0.1, "相似度应显著大于阈值");
    }

    #[test]
    fn domain_hard_filter_blocks_other_domain() {
        let mut svc = AssetService::new();
        let g = sample_graph();
        svc.freeze_asset(uuid::Uuid::new_v4(), "电商报告", Domain::BusinessSoftware, &g);
        // 用 Unknown 域查询：硬过滤应排除 business_software 资产
        let hits = svc.search("电商", Domain::Unknown, 3);
        assert!(hits.is_empty(), "域硬过滤应阻断跨域命中");
    }

    #[test]
    fn embedding_is_deterministic() {
        let a = embed("清洗对账", EMBED_DIM);
        let b = embed("清洗对账", EMBED_DIM);
        assert_eq!(a, b);
    }

    #[test]
    fn related_query_outranks_unrelated() {
        let mut svc = AssetService::new();
        let g = sample_graph();
        svc.freeze_asset(uuid::Uuid::new_v4(), "电商报告", Domain::BusinessSoftware, &g);
        let related = svc.search("我想做一份电商经营分析报告", Domain::BusinessSoftware, 3);
        let unrelated = svc.search("宠物狗喂养指南", Domain::BusinessSoftware, 3);
        let related_score = related.first().map(|h| h.score).unwrap_or(0.0);
        let unrelated_score = unrelated.first().map(|h| h.score).unwrap_or(0.0);
        assert!(related_score > unrelated_score, "κ 复用应保证相关查询相似度高于无关查询");
        assert!(related_score > 0.1, "相关查询应显著命中");
    }
}

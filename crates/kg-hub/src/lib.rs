//! # KG-Hub：智能自动化信息知识图谱关联关系平台（关图中枢）
//!
//! 企业**全部知识库的唯一中心**。它解决本工程此前最根本的架构断点：
//! 三套图引擎彼此零引用，却各自声称"唯一事实源"——
//!
//! | 来源 | 能力 | 接入前状态 |
//! | --- | --- | --- |
//! | `tools/info-graph` | 静态代码关图（13 类实体 / 8 类关系） | CLI 孤岛，**无任何 crate 引用** |
//! | `crates/operator-graph` | 运行时 AI 知识图（PageRank/社区/激活/推荐） | 有 HTTP，但与静态关图身份不通 |
//! | `crates/primiflow-fusion` | 六维统一图（L1-L7 / 20 类 / κτCQ 守恒） | 本体正确，但**仅自引用** |
//!
//! KG-Hub 以 [`urn`] 身份规范 + [`ontology`] 本体归一，把三者合并为一张
//! 可检索（[`index`]）、可推理（[`reason`]）、可治理（[`govern`]）、
//! 可自动驱动（[`loop_engine`]）的统一事实源，并通过 [`api`] 对外服务。
//!
//! ## 八段智能闭环
//!
//! 感知 → 归一 → 关联 → 推理 → 决策 → 执行 → 校验 → 沉淀，
//! 每段留痕可审计，校验不通过即拦停，不合规变更不进事实源。
//!
//! ## 最小用法
//!
//! ```
//! use kg_hub::{KgHub, ingest::InfoGraphConnector};
//!
//! let json = r#"{"nodes":[
//!   {"id":"Requirement:D01","kind":"Requirement","name":"D01","path":"REQ/D01","summary":"","external":false},
//!   {"id":"CodeFile:a.rs","kind":"CodeFile","name":"a.rs","path":"a.rs","summary":"","external":false}],
//!   "edges":[{"id":"e1","from":"Requirement:D01","to":"CodeFile:a.rs","kind":"Bind","label":"","evidence":"x"}]}"#;
//!
//! let mut hub = KgHub::new("default");
//! hub.ingest(&InfoGraphConnector::from_str(json)).unwrap();
//!
//! // 全域治理结论
//! assert_eq!(hub.governance().deviation.coverage, 100.0);
//! // 需求溯源：代码回答"我为什么存在"
//! assert!(hub.trace("urn:kg:default:L5:cod:a.rs").grounded);
//! ```

pub mod api;
pub mod govern;
pub mod index;
pub mod ingest;
pub mod loop_engine;
pub mod ontology;
pub mod reason;
pub mod urn;

use primiflow_fusion::UnifiedGraph;
use serde::{Deserialize, Serialize};

pub use govern::{DeviationReport, GateReport, GovernanceSummary};
pub use index::{HybridIndex, HybridQuery, HybridWeights, SearchHit};
pub use ingest::{
    Connector, GraphSink, IngestStat, InfoGraphConnector, KnowledgeBaseConnector, KnowledgeItem,
    KnowledgeGraphConnector, SixDimConnector,
};
pub use loop_engine::{Decision, LoopConfig, LoopReport, Stage, StageTrace};
pub use reason::{Hotspot, ImpactReport, TraceReport};

/// 中枢门面：持有统一图与索引，对外提供一站式能力。
///
/// 索引与图强一致：任何写图操作后自动重建索引，
/// 避免"改了图但搜不到"这类难以察觉的陈旧读。
pub struct KgHub {
    sink: GraphSink,
    index: HybridIndex,
    stats: Vec<IngestStat>,
}

/// 中枢总览
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubOverview {
    pub tenant: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub term_count: usize,
    pub vector_count: usize,
    /// 各来源接入统计
    pub sources: Vec<IngestStat>,
    /// 按实体类型分布
    pub by_kind: std::collections::HashMap<String, usize>,
    /// 按层分布
    pub by_layer: std::collections::HashMap<String, usize>,
}

impl KgHub {
    pub fn new(tenant: &str) -> Self {
        Self {
            sink: GraphSink::new(tenant),
            index: HybridIndex::new(),
            stats: Vec::new(),
        }
    }

    /// 接入一个知识源，并同步重建索引。
    pub fn ingest(&mut self, c: &dyn Connector) -> anyhow::Result<IngestStat> {
        let st = c.ingest(&mut self.sink)?;
        self.stats.push(st.clone());
        self.reindex();
        Ok(st)
    }

    /// 批量接入（任一源失败不影响其余源，失败以 Err 汇总返回）
    pub fn ingest_all(&mut self, cs: &[&dyn Connector]) -> Vec<anyhow::Result<IngestStat>> {
        let mut out = Vec::with_capacity(cs.len());
        for c in cs {
            let r = c.ingest(&mut self.sink);
            if let Ok(st) = &r {
                self.stats.push(st.clone());
            }
            out.push(r);
        }
        self.reindex();
        out
    }

    fn reindex(&mut self) {
        self.index.rebuild(self.sink.graph());
    }

    pub fn graph(&self) -> &UnifiedGraph {
        self.sink.graph()
    }

    pub fn index(&self) -> &HybridIndex {
        &self.index
    }

    pub fn index_mut(&mut self) -> &mut HybridIndex {
        &mut self.index
    }

    pub fn stats(&self) -> &[IngestStat] {
        &self.stats
    }

    /// 混合检索
    pub fn search(&self, q: &HybridQuery) -> Vec<SearchHit> {
        self.index.search(self.sink.graph(), q)
    }

    /// 关键词快查（便捷入口）
    pub fn quick_search(&self, text: &str, top_k: usize) -> Vec<SearchHit> {
        self.search(&HybridQuery {
            text: text.to_string(),
            top_k,
            ..Default::default()
        })
    }

    /// 变更影响面
    pub fn impact(&self, id: &str, hops: usize) -> ImpactReport {
        reason::impact(self.sink.graph(), id, hops)
    }

    /// 需求溯源
    pub fn trace(&self, id: &str) -> TraceReport {
        reason::trace_to_requirement(self.sink.graph(), id)
    }

    /// 知识热点
    pub fn hotspots(&self, top: usize) -> Vec<Hotspot> {
        reason::hotspots(self.sink.graph(), top)
    }

    /// 孤立（沉睡）知识
    pub fn isolated(&self) -> Vec<String> {
        reason::isolated(self.sink.graph())
    }

    /// 治理总评
    pub fn governance(&self) -> GovernanceSummary {
        govern::summarize(self.sink.graph())
    }

    /// 总览
    pub fn overview(&self) -> HubOverview {
        let g = self.sink.graph();
        let mut by_kind = std::collections::HashMap::new();
        let mut by_layer = std::collections::HashMap::new();
        for n in g.nodes.values() {
            *by_kind.entry(n.kind.zh().to_string()).or_insert(0) += 1;
            *by_layer.entry(n.layer.code().to_string()).or_insert(0) += 1;
        }
        HubOverview {
            tenant: self.sink.tenant().to_string(),
            node_count: g.nodes.len(),
            edge_count: g.edges.len(),
            term_count: self.index.term_count(),
            vector_count: self.index.vector_count(),
            sources: self.stats.clone(),
            by_kind,
            by_layer,
        }
    }

    /// 导出 Mermaid（复用 fusion 已有渲染）
    pub fn to_mermaid(&self) -> String {
        self.sink.graph().to_mermaid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json() -> &'static str {
        r#"{
          "nodes": [
            {"id":"Requirement:D01","kind":"Requirement","name":"D01 算子内核","path":"REQ/D01","summary":"算子内核与执行","external":false},
            {"id":"CodeFile:crates/operator-core/src/lib.rs","kind":"CodeFile","name":"lib.rs","path":"crates/operator-core/src/lib.rs","summary":"算子内核实现","external":false},
            {"id":"Doc:README.md","kind":"Doc","name":"README.md","path":"README.md","summary":"项目说明","external":false}
          ],
          "edges": [
            {"id":"e1","from":"Requirement:D01","to":"CodeFile:crates/operator-core/src/lib.rs","kind":"Bind","label":"绑定","evidence":"guantu.req.json"},
            {"id":"e2","from":"CodeFile:crates/operator-core/src/lib.rs","to":"Doc:README.md","kind":"Reference","label":"文档","evidence":"lib.rs:1"}
          ]
        }"#
    }

    fn hub() -> KgHub {
        let mut h = KgHub::new("default");
        h.ingest(&InfoGraphConnector::from_str(json())).unwrap();
        h
    }

    #[test]
    fn ingest_builds_graph_and_index_together() {
        let h = hub();
        assert_eq!(h.graph().nodes.len(), 3);
        assert_eq!(h.graph().edges.len(), 2);
        // 索引必须随图同步建立，不能出现"图有数据但搜不到"
        assert!(h.index().term_count() > 0);
        assert!(!h.quick_search("operator-core", 5).is_empty());
    }

    #[test]
    fn overview_reports_distribution() {
        let h = hub();
        let o = h.overview();
        assert_eq!(o.tenant, "default");
        assert_eq!(o.node_count, 3);
        assert_eq!(o.sources.len(), 1);
        // 需求在 L1、代码在 L5、文档在 L6
        assert_eq!(o.by_layer["L1"], 1);
        assert_eq!(o.by_layer["L5"], 1);
        assert_eq!(o.by_layer["L6"], 1);
        assert_eq!(o.by_kind["需求"], 1);
        assert_eq!(o.by_kind["代码"], 1);
    }

    #[test]
    fn trace_and_impact_work_end_to_end() {
        let h = hub();
        let code = urn::build_default(
            primiflow_fusion::Layer::ExecutionRuntime,
            primiflow_fusion::EntityKind::Code,
            "crates/operator-core/src/lib.rs",
        );
        // 代码可溯源到需求
        let t = h.trace(&code);
        assert!(t.grounded, "代码必须能回答为何存在");
        // 改动该代码会波及 README
        let im = h.impact(&code, 2);
        assert_eq!(im.total, 1);
        assert!(im.affected[0].id.contains("README.md"));
    }

    #[test]
    fn governance_is_clean_for_aligned_graph() {
        let h = hub();
        let g = h.governance();
        assert_eq!(g.deviation.coverage, 100.0);
        assert!(g.deviation.passed);
        assert!(g.acyclic);
        assert!(h.isolated().is_empty());
    }

    #[test]
    fn reingest_is_idempotent_at_hub_level() {
        let mut h = hub();
        let n1 = h.graph().nodes.len();
        let e1 = h.graph().edges.len();
        h.ingest(&InfoGraphConnector::from_str(json())).unwrap();
        assert_eq!(h.graph().nodes.len(), n1, "重复接入不得膨胀图");
        assert_eq!(h.graph().edges.len(), e1);
        assert_eq!(h.stats().len(), 2, "但每次接入都应留统计");
    }

    #[test]
    fn ingest_all_isolates_failures() {
        let mut h = KgHub::new("default");
        let good = InfoGraphConnector::from_str(json());
        let bad = InfoGraphConnector::from_str("not json");
        let rs = h.ingest_all(&[&good, &bad]);
        assert!(rs[0].is_ok());
        assert!(rs[1].is_err(), "坏源必须报错");
        // 好源数据不能因坏源而丢失
        assert_eq!(h.graph().nodes.len(), 3);
    }

    #[test]
    fn hotspots_and_mermaid_are_available() {
        let h = hub();
        assert!(!h.hotspots(3).is_empty());
        let m = h.to_mermaid();
        assert!(!m.is_empty());
    }

    #[test]
    fn vector_search_via_index_mut() {
        let mut h = hub();
        let doc = urn::build_default(
            primiflow_fusion::Layer::AssetPrecipitation,
            primiflow_fusion::EntityKind::Doc,
            "README.md",
        );
        h.index_mut().put_vector(&doc, vec![0.0, 1.0]);
        let hits = h.search(&HybridQuery {
            vector: Some(vec![0.0, 1.0]),
            ..Default::default()
        });
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, doc);
    }
}

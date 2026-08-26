//! 接入层：企业知识库连接器 + 三图归一。
//!
//! 本模块解决工程的核心断点——三套图彼此零引用、各自都自称"唯一事实源"：
//! - `tools/info-graph` 静态关图：CLI 独占，`graph.json` 落盘，**无任何 crate 引用**
//! - `crates/graph-algorithms` 运行时 AI 图：有 HTTP，但与静态关图无身份互通
//! - `crates/primiflow-fusion` 六维统一图：本体正确，但仅自引用
//!
//! 归一策略：一切来源经 `Connector` 归一为 `UnifiedNode/UnifiedEdge`，
//! 以 URN 为主键做**幂等合并**（同 URN 多次接入不产生重复，且信息更全者胜出）。

use std::collections::{HashMap, HashSet};

use mox_flow_fusion_svc::{
    EntityKind, Layer, PrimitiveCoords, RelKind, SixDimRegistry, UnifiedEdge, UnifiedGraph,
    UnifiedNode,
};
use serde::{Deserialize, Serialize};

use crate::ontology;
use crate::urn;

/// 接入统计：每个连接器的产出与去重情况，用于可观测与回归断言。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct IngestStat {
    pub source: String,
    pub nodes_in: usize,
    pub edges_in: usize,
    /// 新建节点数
    pub nodes_new: usize,
    /// 命中同一 URN 被合并的节点数
    pub nodes_merged: usize,
    /// 新建边数
    pub edges_new: usize,
    /// 重复边（同 from|kind|to）被抑制数
    pub edges_dedup: usize,
    /// 因端点缺失被丢弃的边（悬挂边，必须显式暴露而非静默吞掉）
    pub edges_dangling: usize,
}

/// 归一化写入槽：所有连接器只能通过它写图，保证幂等与去重规则唯一。
pub struct GraphSink {
    graph: UnifiedGraph,
    tenant: String,
    edge_keys: HashSet<String>,
    /// 来源 id → URN 的映射（供同一来源内的边解析端点）
    alias: HashMap<String, String>,
}

impl GraphSink {
    pub fn new(tenant: &str) -> Self {
        Self {
            graph: UnifiedGraph::new(),
            tenant: if tenant.trim().is_empty() {
                urn::DEFAULT_TENANT.to_string()
            } else {
                tenant.trim().to_string()
            },
            edge_keys: HashSet::new(),
            alias: HashMap::new(),
        }
    }

    pub fn tenant(&self) -> &str {
        &self.tenant
    }

    /// 登记来源 id → URN 别名，使跨来源的边能找到同一实体。
    pub fn register_alias(&mut self, source_id: &str, urn_id: &str) {
        self.alias.insert(source_id.to_string(), urn_id.to_string());
    }

    /// 解析任意来源 id 为 URN：已是 URN 则直通，否则查别名表。
    pub fn resolve(&self, source_id: &str) -> Option<String> {
        if urn::is_urn(source_id) {
            return Some(source_id.to_string());
        }
        self.alias.get(source_id).cloned()
    }

    /// 幂等写入节点。返回 `(urn, is_new)`。
    ///
    /// 合并规则：同 URN 时保留"信息更全"的字段——非空覆盖空，
    /// 已有六维绑定或非零坐标不被空值回退覆盖。
    pub fn upsert(&mut self, mut node: UnifiedNode, source_id: &str) -> (String, bool) {
        let id = urn::build(&self.tenant, node.layer, node.kind, &node.path_or_name());
        node.id = id.clone();
        self.register_alias(source_id, &id);
        self.register_alias(&id, &id);

        match self.graph.nodes.get_mut(&id) {
            Some(existing) => {
                if existing.name.is_empty() && !node.name.is_empty() {
                    existing.name = node.name;
                }
                if existing.summary.is_empty() && !node.summary.is_empty() {
                    existing.summary = node.summary;
                }
                if existing.evidence.is_empty() && !node.evidence.is_empty() {
                    existing.evidence = node.evidence;
                }
                if existing.path.is_empty() && !node.path.is_empty() {
                    existing.path = node.path;
                }
                if existing.bind_id.is_none() && node.bind_id.is_some() {
                    existing.bind_id = node.bind_id;
                }
                // 坐标：仅当已有为零而新值非零时才提升，避免高信息被低信息覆盖
                if existing.primitive.c == 0.0 && node.primitive.c != 0.0 {
                    existing.primitive = node.primitive;
                }
                // external 收紧：任一来源认为它是内部实体，就不再是外部
                if existing.external && !node.external {
                    existing.external = false;
                }
                (id, false)
            }
            None => {
                self.graph.add_node(node);
                (id, true)
            }
        }
    }

    /// 幂等写入边。端点必须已存在，否则计为悬挂边并拒绝。
    ///
    /// 返回 `Ok(true)` 新建、`Ok(false)` 重复抑制、`Err(())` 悬挂。
    #[allow(clippy::result_unit_err)]
    pub fn link(
        &mut self,
        from_src: &str,
        to_src: &str,
        kind: RelKind,
        label: &str,
        evidence: &str,
    ) -> Result<bool, ()> {
        let from = match self.resolve(from_src) {
            Some(f) => f,
            None => return Err(()),
        };
        let to = match self.resolve(to_src) {
            Some(t) => t,
            None => return Err(()),
        };
        if !self.graph.nodes.contains_key(&from) || !self.graph.nodes.contains_key(&to) {
            return Err(());
        }
        let key = format!("{from}|{:?}|{to}", kind);
        if !self.edge_keys.insert(key.clone()) {
            return Ok(false);
        }
        self.graph.add_edge(UnifiedEdge {
            id: format!("E:{}", key),
            from,
            to,
            kind,
            label: label.to_string(),
            evidence: evidence.to_string(),
        });
        Ok(true)
    }

    pub fn graph(&self) -> &UnifiedGraph {
        &self.graph
    }

    pub fn into_graph(self) -> UnifiedGraph {
        self.graph
    }
}

/// 供 upsert 计算 URN key：优先 path（稳定），否则退化为 name。
trait PathOrName {
    fn path_or_name(&self) -> String;
}
impl PathOrName for UnifiedNode {
    fn path_or_name(&self) -> String {
        if !self.path.trim().is_empty() {
            self.path.clone()
        } else {
            self.name.clone()
        }
    }
}

/// 知识库连接器统一契约。企业任何知识源接入中枢，只需实现本 trait。
pub trait Connector {
    /// 连接器名称（进入 `IngestStat.source`）
    fn name(&self) -> String;
    /// 把来源数据归一并写入 sink
    fn ingest(&self, sink: &mut GraphSink) -> anyhow::Result<IngestStat>;
}

// ───────────────────── 连接器 1：静态关图 graph.json ─────────────────────

/// 静态关图 JSON 的节点结构（对齐 `tools/info-graph` 落盘格式：节点无 evidence 字段）
#[derive(Debug, Clone, Deserialize)]
struct RawNode {
    id: String,
    kind: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    external: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct RawEdge {
    from: String,
    to: String,
    kind: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    evidence: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RawGraph {
    #[serde(default)]
    nodes: Vec<RawNode>,
    #[serde(default)]
    edges: Vec<RawEdge>,
}

/// 接入 `tools/info-graph` 产出的 `graph.json` / `graph.enterprise.json`。
///
/// 这是把静态关图从"CLI 孤岛"接进中枢的关键连接器。
pub struct InfoGraphConnector {
    pub json: String,
    pub label: String,
}

impl InfoGraphConnector {
    // 命名与 std::str::FromStr 易混淆；此处为 JSON 构造器（impl Into<String>），保留显式 allow
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(json: impl Into<String>) -> Self {
        Self {
            json: json.into(),
            label: "info-graph".to_string(),
        }
    }

    pub fn from_path(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let p = path.as_ref();
        let json = std::fs::read_to_string(p)
            .map_err(|e| anyhow::anyhow!("读取静态关图失败 {}: {e}", p.display()))?;
        Ok(Self {
            json,
            label: format!("info-graph:{}", p.display()),
        })
    }
}

impl Connector for InfoGraphConnector {
    fn name(&self) -> String {
        self.label.clone()
    }

    fn ingest(&self, sink: &mut GraphSink) -> anyhow::Result<IngestStat> {
        let raw: RawGraph = serde_json::from_str(&self.json)
            .map_err(|e| anyhow::anyhow!("静态关图 JSON 解析失败: {e}"))?;
        let mut st = IngestStat {
            source: self.name(),
            nodes_in: raw.nodes.len(),
            edges_in: raw.edges.len(),
            ..Default::default()
        };

        for n in &raw.nodes {
            let kind = ontology::map_info_kind(&n.kind);
            let node = UnifiedNode {
                id: String::new(),
                kind,
                layer: ontology::default_layer(kind),
                name: if n.name.is_empty() {
                    n.id.clone()
                } else {
                    n.name.clone()
                },
                path: n.path.clone(),
                summary: n.summary.clone(),
                // 静态关图节点自身即证据：来自代码库扫描的真实路径
                evidence: if n.path.is_empty() {
                    format!("info-graph:{}", n.id)
                } else {
                    n.path.clone()
                },
                primitive: PrimitiveCoords::zero(),
                bind_id: None,
                external: n.external,
            };
            let (_, is_new) = sink.upsert(node, &n.id);
            if is_new {
                st.nodes_new += 1;
            } else {
                st.nodes_merged += 1;
            }
        }

        for e in &raw.edges {
            let kind = ontology::map_relation(&e.kind);
            match sink.link(&e.from, &e.to, kind, &e.label, &e.evidence) {
                Ok(true) => st.edges_new += 1,
                Ok(false) => st.edges_dedup += 1,
                Err(()) => st.edges_dangling += 1,
            }
        }
        Ok(st)
    }
}

// ───────────────────── 连接器 2：运行时 AI 知识图 ─────────────────────

/// 接入 `crates/graph-algorithms` 的运行时 AI 知识图（含 embedding / activation）。
///
/// AI 图的 `node_type` 是自由字符串，经 `ontology::map_node_type` 模糊归一；
/// 其 `embedding` 与 `activation` 由中枢索引层单独承载（见 `index.rs`）。
pub struct KnowledgeGraphConnector<'a> {
    pub graph: &'a mox_kg_algo_core::KnowledgeGraph,
}

impl<'a> Connector for KnowledgeGraphConnector<'a> {
    fn name(&self) -> String {
        "graph-algorithms".to_string()
    }

    fn ingest(&self, sink: &mut GraphSink) -> anyhow::Result<IngestStat> {
        let nodes = self.graph.nodes();
        let edges = self.graph.edges();
        let mut st = IngestStat {
            source: self.name(),
            nodes_in: nodes.len(),
            edges_in: edges.len(),
            ..Default::default()
        };

        for kn in &nodes {
            let kind = ontology::map_node_type(&kn.node_type);
            let path = kn
                .metadata
                .get("path")
                .cloned()
                .unwrap_or_else(|| kn.id.clone());
            let node = UnifiedNode {
                id: String::new(),
                kind,
                layer: ontology::default_layer(kind),
                name: kn.label.clone(),
                path,
                summary: kn
                    .properties
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                evidence: format!("graph-algorithms:{}", kn.id),
                primitive: PrimitiveCoords::zero(),
                bind_id: None,
                external: false,
            };
            let (_, is_new) = sink.upsert(node, &kn.id);
            if is_new {
                st.nodes_new += 1;
            } else {
                st.nodes_merged += 1;
            }
        }

        for ke in &edges {
            let kind = ontology::map_relation(&ke.relation_type);
            match sink.link(
                &ke.source,
                &ke.target,
                kind,
                &ke.relation_type,
                &format!("graph-algorithms:w={:.3}", ke.weight),
            ) {
                Ok(true) => st.edges_new += 1,
                Ok(false) => st.edges_dedup += 1,
                Err(()) => st.edges_dangling += 1,
            }
        }
        Ok(st)
    }
}

// ───────────────────── 连接器 3：六维绑定注册表 ─────────────────────

/// 接入 `primiflow-fusion::SixDimRegistry`，把 REQ→FUN→BIZ→ALG→TSK→COD
/// 六维链路注入中枢，并生成 `Bind` 边。
///
/// 这条链是需求可溯源性的唯一来源：没有它，代码实体无法回答"我为什么存在"。
pub struct SixDimConnector<'a> {
    pub registry: &'a SixDimRegistry,
}

impl<'a> Connector for SixDimConnector<'a> {
    fn name(&self) -> String {
        "sixdim-registry".to_string()
    }

    fn ingest(&self, sink: &mut GraphSink) -> anyhow::Result<IngestStat> {
        let bindings = &self.registry.bindings;
        let mut st = IngestStat {
            source: self.name(),
            nodes_in: bindings.len() * 6,
            edges_in: bindings.len() * 5,
            ..Default::default()
        };

        for b in bindings {
            let chain: [(EntityKind, &str); 6] = [
                (EntityKind::Requirement, &b.requirement),
                (EntityKind::Feature, &b.feature),
                (EntityKind::Business, &b.business),
                (EntityKind::Algorithm, &b.algorithm),
                (EntityKind::Task, &b.task),
                (EntityKind::Code, &b.code),
            ];

            let mut urns: Vec<String> = Vec::with_capacity(6);
            for (kind, raw_id) in chain.iter() {
                if raw_id.trim().is_empty() {
                    continue;
                }
                let summary = if *kind == EntityKind::Requirement {
                    b.req_text.clone()
                } else {
                    String::new()
                };
                let node = UnifiedNode {
                    id: String::new(),
                    kind: *kind,
                    layer: ontology::default_layer(*kind),
                    name: (*raw_id).to_string(),
                    path: (*raw_id).to_string(),
                    summary,
                    evidence: format!("sixdim:{}#{}", b.project_id, b.req_id),
                    // 六维坐标只在链路 Completed 时带守恒荷
                    primitive: if b.is_completed() {
                        b.coords
                    } else {
                        PrimitiveCoords::zero()
                    },
                    bind_id: Some(b.req_id.clone()),
                    external: false,
                };
                let (u, is_new) = sink.upsert(node, raw_id);
                if is_new {
                    st.nodes_new += 1;
                } else {
                    st.nodes_merged += 1;
                }
                urns.push(u);
            }

            // 相邻维度间建 Bind 边，形成完整溯源链
            for w in urns.windows(2) {
                match sink.link(
                    &w[0],
                    &w[1],
                    RelKind::Bind,
                    "六维绑定",
                    &format!("sixdim:{}", b.req_id),
                ) {
                    Ok(true) => st.edges_new += 1,
                    Ok(false) => st.edges_dedup += 1,
                    Err(()) => st.edges_dangling += 1,
                }
            }
        }
        Ok(st)
    }
}

// ───────────────────── 连接器 4：文档知识库（文件系统） ─────────────────────

/// 一条待接入的知识条目——企业文档库/Wiki/工单等外部系统的通用投影。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub key: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    /// 未指定时按 `Doc` 处理
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub evidence: String,
    /// 指向其它条目 key 的引用，生成 `Reference` 边
    #[serde(default)]
    pub refs: Vec<String>,
}

/// 通用知识库连接器：任何外部系统（Confluence/腾讯文档/工单/邮件）
/// 只要能投影成 `KnowledgeItem` 列表，即可接入中枢。
pub struct KnowledgeBaseConnector {
    pub source: String,
    pub items: Vec<KnowledgeItem>,
}

impl Connector for KnowledgeBaseConnector {
    fn name(&self) -> String {
        self.source.clone()
    }

    fn ingest(&self, sink: &mut GraphSink) -> anyhow::Result<IngestStat> {
        let mut st = IngestStat {
            source: self.name(),
            nodes_in: self.items.len(),
            edges_in: self.items.iter().map(|i| i.refs.len()).sum(),
            ..Default::default()
        };

        for it in &self.items {
            let kind = it
                .kind
                .as_deref()
                .map(ontology::map_node_type)
                .unwrap_or(EntityKind::Doc);
            let node = UnifiedNode {
                id: String::new(),
                kind,
                layer: ontology::default_layer(kind),
                name: it.title.clone(),
                path: it.key.clone(),
                summary: summarize(&it.body),
                evidence: if it.evidence.is_empty() {
                    format!("{}:{}", self.source, it.key)
                } else {
                    it.evidence.clone()
                },
                primitive: PrimitiveCoords::zero(),
                bind_id: None,
                external: false,
            };
            let (_, is_new) = sink.upsert(node, &it.key);
            if is_new {
                st.nodes_new += 1;
            } else {
                st.nodes_merged += 1;
            }
        }

        // 引用边必须在所有节点落库后再建，否则前向引用全成悬挂边
        for it in &self.items {
            for r in &it.refs {
                match sink.link(
                    &it.key,
                    r,
                    RelKind::Reference,
                    "文档引用",
                    &format!("{}:{}", self.source, it.key),
                ) {
                    Ok(true) => st.edges_new += 1,
                    Ok(false) => st.edges_dedup += 1,
                    Err(()) => st.edges_dangling += 1,
                }
            }
        }
        Ok(st)
    }
}

/// 摘要：取正文首个非空行，限长 200 字符（按字符而非字节，避免截断多字节汉字）
fn summarize(body: &str) -> String {
    let line = body
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    if line.chars().count() <= 200 {
        return line.to_string();
    }
    line.chars().take(200).collect::<String>() + "…"
}

/// 六维绑定链最小 URN：对外暴露便于测试与治理层构造需求根
pub fn req_urn(tenant: &str, req_id: &str) -> String {
    urn::build(
        tenant,
        Layer::RequirementSemantic,
        EntityKind::Requirement,
        req_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_info_graph_json() -> &'static str {
        r#"{
          "nodes": [
            {"id":"CodeFile:crates/a/src/lib.rs","kind":"CodeFile","name":"lib.rs","path":"crates/a/src/lib.rs","summary":"","external":false},
            {"id":"Doc:README.md","kind":"Doc","name":"README.md","path":"README.md","summary":"说明","external":false},
            {"id":"Dependency:serde","kind":"Dependency","name":"serde","path":"","summary":"","external":true}
          ],
          "edges": [
            {"id":"e1","from":"CodeFile:crates/a/src/lib.rs","to":"Dependency:serde","kind":"Dependency","label":"use","evidence":"crates/a/src/lib.rs:1"},
            {"id":"e2","from":"CodeFile:crates/a/src/lib.rs","to":"Doc:MISSING.md","kind":"Reference","label":"ref","evidence":"x"}
          ]
        }"#
    }

    #[test]
    fn info_graph_ingest_maps_kinds_and_reports_dangling() {
        let mut sink = GraphSink::new("default");
        let st = InfoGraphConnector::from_str(sample_info_graph_json())
            .ingest(&mut sink)
            .expect("ingest ok");

        assert_eq!(st.nodes_in, 3);
        assert_eq!(st.nodes_new, 3);
        assert_eq!(st.edges_new, 1);
        // 指向不存在节点的边必须被识别为悬挂，而不是静默建出幽灵节点
        assert_eq!(st.edges_dangling, 1);

        // CodeFile 归一为 Code 并落在 L5
        let u = urn::build_default(
            Layer::ExecutionRuntime,
            EntityKind::Code,
            "crates/a/src/lib.rs",
        );
        let n = sink.graph().node(&u).expect("code node exists");
        assert_eq!(n.kind, EntityKind::Code);
        assert_eq!(n.evidence, "crates/a/src/lib.rs");
        // 依赖 serde 被标记为外部
        let d = urn::build_default(Layer::AssetPrecipitation, EntityKind::Dependency, "serde");
        assert!(sink.graph().node(&d).expect("dep").external);
    }

    #[test]
    fn repeated_ingest_is_idempotent() {
        let mut sink = GraphSink::new("default");
        let c = InfoGraphConnector::from_str(sample_info_graph_json());
        let first = c.ingest(&mut sink).unwrap();
        let n1 = sink.graph().nodes.len();
        let e1 = sink.graph().edges.len();

        let second = c.ingest(&mut sink).unwrap();
        // 二次接入：节点全部命中合并、边全部被去重，图规模不变
        assert_eq!(first.nodes_new, 3);
        assert_eq!(second.nodes_new, 0);
        assert_eq!(second.nodes_merged, 3);
        assert_eq!(second.edges_new, 0);
        assert_eq!(second.edges_dedup, 1);
        assert_eq!(sink.graph().nodes.len(), n1);
        assert_eq!(sink.graph().edges.len(), e1);
    }

    #[test]
    fn cross_source_same_entity_merges_into_one_urn() {
        // 静态关图与 AI 图描述同一份代码文件，必须合并为一个节点
        let mut sink = GraphSink::new("default");
        InfoGraphConnector::from_str(sample_info_graph_json())
            .ingest(&mut sink)
            .unwrap();
        let before = sink.graph().nodes.len();

        let kg = mox_kg_algo_core::KnowledgeGraphBuilder::new()
            .add_node("n-1", "lib.rs", "code")
            .build();
        // AI 图节点无 metadata.path 时以 id 作 key，故此处显式对齐 path
        let mut kg2 = mox_kg_algo_core::KnowledgeGraph::new();
        let mut node = kg.nodes()[0].clone();
        node.metadata
            .insert("path".into(), "crates/a/src/lib.rs".into());
        kg2.add_node(node);

        let st = KnowledgeGraphConnector { graph: &kg2 }
            .ingest(&mut sink)
            .unwrap();
        assert_eq!(st.nodes_merged, 1, "同一实体跨来源必须合并");
        assert_eq!(st.nodes_new, 0);
        assert_eq!(sink.graph().nodes.len(), before, "图规模不得增长");
    }

    #[test]
    fn knowledge_base_forward_refs_resolve() {
        let items = vec![
            KnowledgeItem {
                key: "kb/a.md".into(),
                title: "A".into(),
                body: "\n\n第一段正文".into(),
                kind: None,
                evidence: String::new(),
                // 前向引用 b.md（尚未登记），两段式写入必须能解析
                refs: vec!["kb/b.md".into()],
            },
            KnowledgeItem {
                key: "kb/b.md".into(),
                title: "B".into(),
                body: String::new(),
                kind: None,
                evidence: String::new(),
                refs: vec![],
            },
        ];
        let mut sink = GraphSink::new("default");
        let st = KnowledgeBaseConnector {
            source: "wiki".into(),
            items,
        }
        .ingest(&mut sink)
        .unwrap();
        assert_eq!(st.nodes_new, 2);
        assert_eq!(st.edges_new, 1);
        assert_eq!(st.edges_dangling, 0, "前向引用不应成为悬挂边");

        let a = urn::build_default(Layer::AssetPrecipitation, EntityKind::Doc, "kb/a.md");
        assert_eq!(sink.graph().node(&a).unwrap().summary, "第一段正文");
    }

    #[test]
    fn summarize_truncates_by_chars_not_bytes() {
        let long = "汉".repeat(300);
        let s = summarize(&long);
        // 200 字符 + 省略号，若按字节截断会 panic 或产生乱码
        assert_eq!(s.chars().count(), 201);
    }
}

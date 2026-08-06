//! 归一化 IR 扩展：在 `flow_ai` 的 FlowGraph 之上叠加「维度」着色。
//!
//! 设计铁律：四种流程图（业务/算法/权限/资源）在内存里是**同一个 FlowGraph**，
//! 维度只是节点/边上的标签。物理节点唯一，因此「改一处，全维同步」天然成立。

use flow_ai::model::{FlowGraph, NodeKind, ToolKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 七个优化维度（专家联盟的七个镜头）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    Business,
    Algorithm,
    Permission,
    Resource,
    Security,
    Data,
    Observability,
}

impl Dimension {
    /// 全维裁决时的优先级权重：值越大越优先被采纳（权限/安全不可被性能绕过）
    pub fn priority(&self) -> u8 {
        match self {
            Dimension::Permission => 7,
            Dimension::Security => 7,
            Dimension::Resource => 6,
            Dimension::Data => 5,
            Dimension::Business => 4,
            Dimension::Observability => 3,
            Dimension::Algorithm => 2,
        }
    }
}

/// 节点上的维度着色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionTag {
    pub dimension: Dimension,
    pub owner_expert: ExpertId,
    pub policy_refs: Vec<PolicyId>,
    /// 该维度在此节点的相对重要性 0..1
    pub weight: f64,
}

/// 扩展后的流程图：持有原始 flow_ai 图 + 维度标注
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionedFlow {
    pub base: FlowGraph,
    /// node_id -> 维度标签
    pub node_dimensions: HashMap<String, Vec<DimensionTag>>,
}

impl DimensionedFlow {
    pub fn from_base(base: FlowGraph) -> Self {
        Self { base, node_dimensions: HashMap::new() }
    }

    pub fn tag(&mut self, node_id: impl Into<String>, tag: DimensionTag) {
        self.node_dimensions.entry(node_id.into()).or_default().push(tag);
    }

    pub fn dimensions_of(&self, node_id: &str) -> &[DimensionTag] {
        self.node_dimensions.get(node_id).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

/// 专家标识
pub type ExpertId = String;
/// 策略标识
pub type PolicyId = String;

/// 把业务/算法/权限/资源四类图的语义标签挂到归一化图上。
///
/// 这是一个示例性的「维度着色器」：把 `tags` 中含 `dim:<x>` 的节点标到对应维度，
/// 让后续专家能按维度分镜评估。真实系统可由设计器或 AI 自动标注。
pub fn auto_dimension(base: &FlowGraph) -> DimensionedFlow {
    let mut df = DimensionedFlow::from_base(base.clone());
    for n in &base.nodes {
        // 默认所有节点至少属于业务维度
        df.tag(n.id.clone(), DimensionTag {
            dimension: Dimension::Business,
            owner_expert: "business".into(),
            policy_refs: Vec::new(),
            weight: 1.0,
        });
        for t in &n.tags {
            if let Some(dim) = t.strip_prefix("dim:") {
                let (dim, expert) = match dim {
                    "algo" => (Dimension::Algorithm, "algorithm"),
                    "perm" => (Dimension::Permission, "permission"),
                    "res" => (Dimension::Resource, "resource"),
                    "sec" => (Dimension::Security, "security"),
                    "data" => (Dimension::Data, "data"),
                    "obs" => (Dimension::Observability, "observability"),
                    _ => continue,
                };
                df.tag(n.id.clone(), DimensionTag {
                    dimension: dim,
                    owner_expert: expert.into(),
                    policy_refs: Vec::new(),
                    weight: 1.0,
                });
            }
        }
        // 工具类自动带资源维度；LLM 带算法维度
        match n.tool {
            Some(ToolKind::Llm) => {
                df.tag(n.id.clone(), DimensionTag {
                    dimension: Dimension::Algorithm,
                    owner_expert: "algorithm".into(),
                    policy_refs: Vec::new(),
                    weight: 1.0,
                });
            }
            Some(_) => {
                df.tag(n.id.clone(), DimensionTag {
                    dimension: Dimension::Resource,
                    owner_expert: "resource".into(),
                    policy_refs: Vec::new(),
                    weight: 1.0,
                });
            }
            None => {}
        }
        if n.kind == NodeKind::Guard {
            df.tag(n.id.clone(), DimensionTag {
                dimension: Dimension::Permission,
                owner_expert: "permission".into(),
                policy_refs: Vec::new(),
                weight: 1.0,
            });
        }
    }
    df
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_ai::model::{FlowEdge, FlowNode, NodeKind, ToolKind};

    #[test]
    fn priority_ordering() {
        assert!(Dimension::Permission.priority() > Dimension::Algorithm.priority());
        assert!(Dimension::Security.priority() >= Dimension::Resource.priority());
    }

    #[test]
    fn auto_dimension_tags_llm_as_algo() {
        let mut g = FlowGraph::new("x", "t");
        g.add_node(FlowNode::task("n1", "推理", ToolKind::Llm, 100));
        let df = auto_dimension(&g);
        let tags = df.dimensions_of("n1");
        assert!(tags.iter().any(|t| t.dimension == Dimension::Algorithm));
        assert!(tags.iter().any(|t| t.dimension == Dimension::Business));
    }

    #[test]
    fn auto_dimension_explicit_tag() {
        let mut g = FlowGraph::new("x", "t");
        let mut n = FlowNode::task("n1", "取数", ToolKind::Browser, 200);
        n.tags.push("dim:sec".into());
        g.add_node(n);
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_edge(FlowEdge::seq("s", "n1"));
        let df = auto_dimension(&g);
        assert!(df.dimensions_of("n1").iter().any(|t| t.dimension == Dimension::Security));
    }
}

//! Designer blueprint adapter shared by governance and publication.
//! Keeps legacy permissive mapping; domain validation remains a separate operation.
pub fn normalize_blueprint(
    v: &serde_json::Value,
    id: &str,
    name: &str,
) -> mox_ai_flow_core::model::FlowGraph {
    let mut g = mox_ai_flow_core::model::FlowGraph::new(id, name);
    if let Some(nodes) = v.get("nodes").and_then(|n| n.as_array()) {
        for n in nodes {
            let id = n.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let name = n.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let t = n.get("type").and_then(|x| x.as_str()).unwrap_or("operator");
            let kind = match t {
                "start" => mox_ai_flow_core::model::NodeKind::Start,
                "end" => mox_ai_flow_core::model::NodeKind::End,
                "condition" | "decision" => mox_ai_flow_core::model::NodeKind::Decision,
                "parallel" => mox_ai_flow_core::model::NodeKind::ParallelFork,
                "guard" => mox_ai_flow_core::model::NodeKind::Guard,
                "subflow" => mox_ai_flow_core::model::NodeKind::SubFlow,
                _ => mox_ai_flow_core::model::NodeKind::Task,
            };
            let mut node = mox_ai_flow_core::model::FlowNode::new(id, name, kind);
            if let Some(tool) = n.get("tool").and_then(|x| x.as_str()) {
                node.tool = serde_json::from_value::<mox_ai_flow_core::model::ToolKind>(
                    serde_json::Value::String(tool.into()),
                )
                .ok();
            }
            g.add_node(node);
        }
    }
    if let Some(edges) = v.get("edges").and_then(|e| e.as_array()) {
        for e in edges {
            let from = e.get("from").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let to = e.get("to").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let kind = if e.get("condition").is_some() || e.get("label").is_some() {
                mox_ai_flow_core::model::EdgeKind::Conditional
            } else {
                mox_ai_flow_core::model::EdgeKind::Sequence
            };
            let condition = e.get("condition").and_then(|x| x.as_str()).map(|s| s.to_string());
            let edge = mox_ai_flow_core::model::FlowEdge { from, to, kind, condition };
            g.add_edge(edge);
        }
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_flow_core::model::{EdgeKind, NodeKind, ToolKind};
    use serde_json::json;

    #[test]
    fn governance_and_publication_preserve_the_same_semantics() {
        let value = json!({"nodes": [
            {"id":"start","type":"start"},
            {"id":"task","name":"query","tool":"database"},
            {"id":"gate","type":"condition"}
        ], "edges":[{"from":"start","to":"task"},
                     {"from":"task","to":"gate","condition":"ok"}]});
        let governance = normalize_blueprint(&value, "unified", "unified-flow");
        let publication = normalize_blueprint(&value, "unified", "codegen-unified-flow");
        assert_eq!(governance.nodes[0].kind, NodeKind::Start);
        assert_eq!(publication.nodes[1].tool, Some(ToolKind::Database));
        assert_eq!(publication.nodes[2].kind, NodeKind::Decision);
        assert_eq!(publication.edges[1].kind, EdgeKind::Conditional);
        assert_eq!(publication.edges[1].condition.as_deref(), Some("ok"));
        assert_eq!(
            serde_json::to_value(&governance.nodes).unwrap(),
            serde_json::to_value(&publication.nodes).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&governance.edges).unwrap(),
            serde_json::to_value(&publication.edges).unwrap()
        );
        assert_ne!(governance.name, publication.name);
    }

    #[test]
    fn legacy_defaults_and_unknown_tools_remain_compatible() {
        let graph =
            normalize_blueprint(&json!({"nodes":[{"id":"a","tool":"unknown"}]}), "id", "name");
        assert_eq!(graph.nodes[0].kind, NodeKind::Task);
        assert_eq!(graph.nodes[0].tool, None);
        assert!(normalize_blueprint(&json!({}), "id", "name").nodes.is_empty());
    }
}

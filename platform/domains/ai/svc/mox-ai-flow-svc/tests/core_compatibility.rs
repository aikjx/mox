use mox_ai_flow_svc::model::{FlowGraph, FlowNode, ToolKind};

#[test]
fn service_facade_preserves_core_type_identity_and_json() {
    let mut graph = FlowGraph::new("compat", "compatibility");
    graph.add_node(FlowNode::task("a", "compute", ToolKind::Compute, 10));
    // No conversion or duplicate DTO: both public paths name the same type.
    let core_graph: mox_ai_flow_core::model::FlowGraph = graph;
    let json = mox_ai_flow_svc::dump_flow(&core_graph).unwrap();
    let restored = mox_ai_flow_core::load_flow(&json).unwrap();
    assert_eq!(json, mox_ai_flow_core::dump_flow(&restored).unwrap());
    assert_eq!(mox_ai_flow_svc::to_mermaid(&restored), mox_ai_flow_core::to_mermaid(&restored));
}

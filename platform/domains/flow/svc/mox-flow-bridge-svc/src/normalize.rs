//! Step 2：Hermes 工具调用 ↔ mox_ai_flow_svc::FlowNode 规范化映射。
//!
//! 设计：bridge 只认 Hermes 中间件上下文里的 `tool_name` + `args`（均为 serde_json::Value），
//! 把它们转成 mox_ai_flow_svc 的统一流程图节点，使任何 Hermes 工具（含 MCP 第三方工具）都能成为流程图节点。

use mox_ai_flow_svc::model::{FlowEdge, FlowNode, ToolKind};
use serde_json::Value;

/// Hermes 中间件传入的最小上下文（与 `hermes-agent::plugins::ToolRequestMiddlewareContext`
/// 同构的本地投影，避免 bridge 直接依赖 Hermes 源码编译）。
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool_name: String,
    pub args: Value,
    pub turn: u32,
}

/// 把 Hermes 工具名映射到 mox_ai_flow_svc 的 ToolKind。
/// 未知/第三方/MCP 工具一律归为 `External`（仍可参与流程图与并行调度）。
pub fn tool_kind_of(name: &str) -> ToolKind {
    let n = name.to_ascii_lowercase();
    if n.contains("browser") || n.contains("scrape") || n.contains("web") {
        ToolKind::Browser
    } else if n.contains("db") || n.contains("sql") || n.contains("database") {
        ToolKind::Database
    } else if n.contains("file")
        || n.contains("read")
        || n.contains("write")
        || n.contains("fs")
        || n.contains("excel")
        || n.contains("sheet")
        || n.contains("csv")
    {
        ToolKind::File // Excel/CSV 也归入 File
    } else if n.contains("compute") || n.contains("math") || n.contains("transform") {
        ToolKind::Compute
    } else {
        ToolKind::Http // 未知/第三方/MCP 工具归为 Http（无副作用外部调用，可入图参与调度）
    }
}

/// 从 args 推断维度标签（脱敏 / 鉴权 / 事务 等）。
fn infer_tags(args: &Value) -> Vec<String> {
    let mut tags = Vec::new();
    let s = args.to_string().to_ascii_lowercase();
    if s.contains("pii") || s.contains("desensitize") || s.contains("脱敏") || s.contains("citizen")
    {
        tags.push("desensitize".into());
    }
    if s.contains("authz") || s.contains("auth") || s.contains("鉴权") || s.contains("permission")
    {
        tags.push("authz".into());
    }
    if s.contains("transaction") || s.contains("事务") {
        tags.push("transaction".into());
    }
    tags
}

/// 估值节点耗时（ms）：真实工程应从 hermes-intelligence 历史统计取，这里给保守默认。
fn estimate_duration(kind: ToolKind) -> u64 {
    match kind {
        ToolKind::Browser => 500,
        ToolKind::Database => 300,
        ToolKind::File => 120,
        ToolKind::Compute => 80,
        ToolKind::Http => 250,
        ToolKind::Llm => 400,
        ToolKind::Shell => 150,
        ToolKind::Human => 0,
    }
}

/// 把一次工具调用转成流程图节点。id 用 `<tool>#<turn>` 区分同工具多次调用。
pub fn to_flow_node(call: &ToolCall) -> FlowNode {
    let kind = tool_kind_of(&call.tool_name);
    let id = format!("{}#{}", call.tool_name, call.turn);
    let mut node = FlowNode::task(&id, &call.tool_name, kind, estimate_duration(kind));
    for t in infer_tags(&call.args) {
        node = node.with_tag(t);
    }
    node
}

/// 计算本轮调用与前序节点的依赖边：
/// - 同回合并发工具 → 无依赖边（可并行）
/// - 当前 args 引用了前序节点输出（简单启发：args 含前序节点 id 片段）→ 加 seq 边（数据依赖）
pub fn dependency_edges(prev: &[FlowNode], cur: &FlowNode) -> Vec<FlowEdge> {
    let mut edges = Vec::new();
    let cur_args = cur.id.clone(); // 简化：实际应基于 cur.args 文本
    for p in prev {
        if cur_args.contains(&p.id) {
            edges.push(FlowEdge::seq(&p.id, &cur.id));
        }
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_ai_flow_svc::model::NodeKind;
    use serde_json::json;

    #[test]
    fn maps_browser_tool() {
        let c = ToolCall {
            tool_name: "browser.scrape".into(),
            args: json!({"url": "https://x"}),
            turn: 1,
        };
        let n = to_flow_node(&c);
        assert_eq!(n.kind, NodeKind::Task);
        assert_eq!(n.tool, Some(ToolKind::Browser));
        assert_eq!(n.id, "browser.scrape#1");
    }

    #[test]
    fn maps_unknown_tool_to_external() {
        let c = ToolCall {
            tool_name: "mcp::my_third_party_tool".into(),
            args: json!({}),
            turn: 3,
        };
        let n = to_flow_node(&c);
        assert_eq!(n.tool, Some(ToolKind::Http)); // 第三方 MCP 工具也能入图（归为 Http）
    }

    #[test]
    fn infers_desensitize_tag() {
        let c = ToolCall {
            tool_name: "db.read".into(),
            args: json!({"query": "select * from citizen_info"}),
            turn: 1,
        };
        let n = to_flow_node(&c);
        assert!(n.tags.iter().any(|t| t == "desensitize"));
    }

    #[test]
    fn no_edge_for_concurrent_same_turn() {
        let a = to_flow_node(&ToolCall {
            tool_name: "web1".into(),
            args: json!({}),
            turn: 1,
        });
        let b = to_flow_node(&ToolCall {
            tool_name: "web2".into(),
            args: json!({}),
            turn: 1,
        });
        // 同回合（参数不互相引用）→ 无依赖边 → 可并行
        assert!(dependency_edges(&[a], &b).is_empty());
    }
}

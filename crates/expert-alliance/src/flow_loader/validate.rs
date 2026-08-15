//! 流程 YAML 校验 — 在构建 FlowGraph 前捕获结构性错误

use super::yaml::FlowDef;
use std::collections::HashSet;

/// 校验结果（空 = 通过）
pub type ValidationResult = Result<(), ValidationError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    NoStartNode,
    MultipleStartNodes(Vec<String>),
    NoEndNode,
    MultipleEndNodes(Vec<String>),
    MissingNode { edge: String, ref_id: String },
    DuplicateNodeId(String),
    OrphanNode(String),
    GuardWithoutTags(String),
    BlockingRuleWithoutGuardTags(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoStartNode => write!(f, "流程缺少 start 节点"),
            Self::MultipleStartNodes(ids) => write!(f, "流程有 {} 个 start 节点（应只有1个）：{:?}", ids.len(), ids),
            Self::NoEndNode => write!(f, "流程缺少 end 节点"),
            Self::MultipleEndNodes(ids) => write!(f, "流程有 {} 个 end 节点（应只有1个）：{:?}", ids.len(), ids),
            Self::MissingNode { edge, ref_id } => write!(f, "边 '{edge}' 引用了不存在的节点 '{ref_id}'"),
            Self::DuplicateNodeId(id) => write!(f, "节点 ID '{id}' 重复定义"),
            Self::OrphanNode(id) => write!(f, "节点 '{id}' 未连接到 start 或 end（悬空孤立）"),
            Self::GuardWithoutTags(id) => write!(f, "Guard 节点 '{id}' 缺少 tags（无法触发权限/安全专家）"),
            Self::BlockingRuleWithoutGuardTags(rule_id) => write!(f, "Blocking 规则 '{rule_id}' 必须指定 required_guard_tags"),
        }
    }
}

impl std::error::Error for ValidationError {}

pub fn validate(def: &FlowDef) -> ValidationResult {
    let mut errs = Vec::new();

    // ── 1. 唯一 Start / End ──────────────────────────────────────────────────
    let starts: Vec<_> = def.nodes.iter().filter(|n| node_kind_is(&n.kind, "start")).collect();
    let ends: Vec<_> = def.nodes.iter().filter(|n| node_kind_is(&n.kind, "end")).collect();

    if starts.is_empty() { errs.push(ValidationError::NoStartNode); }
    else if starts.len() > 1 { errs.push(ValidationError::MultipleStartNodes(starts.iter().map(|n| n.id.clone()).collect())); }

    if ends.is_empty() { errs.push(ValidationError::NoEndNode); }
    else if ends.len() > 1 { errs.push(ValidationError::MultipleEndNodes(ends.iter().map(|n| n.id.clone()).collect())); }

    // ── 2. 节点 ID 不重复 ────────────────────────────────────────────────────
    let mut seen_ids: HashSet<&str> = HashSet::new();
    for n in &def.nodes {
        if !seen_ids.insert(n.id.as_str()) {
            errs.push(ValidationError::DuplicateNodeId(n.id.clone()));
        }
    }

    // ── 3. 边引用的节点都存在 ───────────────────────────────────────────────
    let all_ids: HashSet<&str> = def.nodes.iter().map(|n| n.id.as_str()).collect();
    for e in &def.edges {
        if !all_ids.contains(e.from.as_str()) {
            errs.push(ValidationError::MissingNode { edge: format!("{}→{}", e.from, e.to), ref_id: e.from.clone() });
        }
        if !all_ids.contains(e.to.as_str()) {
            errs.push(ValidationError::MissingNode { edge: format!("{}→{}", e.from, e.to), ref_id: e.to.clone() });
        }
    }

    // ── 4. 无孤立节点（每个节点至少有一条入边或出边，除了 start/end） ──────
    let mut node_in_degree: std::collections::HashMap<&str, usize> = def.nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
    let mut node_out_degree: std::collections::HashMap<&str, usize> = def.nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
    for e in &def.edges {
        *node_in_degree.entry(e.to.as_str()).or_insert(0) += 1;
        *node_out_degree.entry(e.from.as_str()).or_insert(0) += 1;
    }
    for n in &def.nodes {
        let is_start = node_kind_is(&n.kind, "start");
        let is_end = node_kind_is(&n.kind, "end");
        if !is_start && !is_end {
            let indeg = *node_in_degree.get(n.id.as_str()).unwrap_or(&0);
            let outdeg = *node_out_degree.get(n.id.as_str()).unwrap_or(&0);
            if indeg == 0 && outdeg == 0 {
                errs.push(ValidationError::OrphanNode(n.id.clone()));
            }
        }
    }

    // ── 5. Guard 节点必须有 tags ─────────────────────────────────────────────
    for n in &def.nodes {
        if node_kind_is(&n.kind, "guard") {
            if n.tags.as_ref().map(|t| t.is_empty()).unwrap_or(true) {
                errs.push(ValidationError::GuardWithoutTags(n.id.clone()));
            }
        }
    }

    // ── 6. Blocking 规则必须有 required_guard_tags ──────────────────────────
    for r in &def.rules {
        if severity_is(&r.severity, "blocking") || severity_is(&r.severity, "info") {
            if r.required_guard_tags.as_ref().map(|t| t.is_empty()).unwrap_or(true) {
                errs.push(ValidationError::BlockingRuleWithoutGuardTags(r.id.clone()));
            }
        }
    }

    if errs.is_empty() { Ok(()) }
    else { Err(errs.remove(0)) } // 返回第一个错误（实用主义）
}

fn node_kind_is(kind: &serde_yaml::Value, target: &str) -> bool {
    kind.as_str().map(|s| s.eq_ignore_ascii_case(target)).unwrap_or(false)
}

fn severity_is(sev: &serde_yaml::Value, target: &str) -> bool {
    sev.as_str().map(|s| s.eq_ignore_ascii_case(target)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_flow() -> FlowDef {
        FlowDef {
            name: "test".into(),
            description: None,
            tags: vec![],
            regulated: false,
            rules: vec![],
            nodes: vec![
                node("s", "start", "start", 0, None),
                node("a", "A", "task", 100, Some(vec!["dim:algo".into()])),
                node("e", "end", "end", 0, None),
            ],
            edges: vec![edge("s","a"), edge("a","e")],
        }
    }

    fn node(id: &str, name: &str, kind: &str, dur: u64, tags: Option<Vec<String>>) -> super::super::yaml::NodeDef {
        super::super::yaml::NodeDef {
            id: id.into(), name: name.into(),
            kind: serde_yaml::Value::String(kind.into()),
            duration_ms: dur, tags, tool: None, access: None, transactional: None,
        }
    }

    fn edge(from: &str, to: &str) -> super::super::yaml::EdgeDef {
        super::super::yaml::EdgeDef {
            from: from.into(), to: to.into(),
            kind: serde_yaml::Value::String("sequence".into()),
        }
    }

    #[test]
    fn good_flow_passes() {
        let f = good_flow();
        match validate(&f) {
            Ok(_) => {}
            Err(e) => panic!("validate failed: {:?}", e),
        }
    }

    #[test]
    fn no_start_fails() {
        let mut f = good_flow();
        f.nodes.retain(|n| !node_kind_is(&n.kind, "start"));
        assert!(matches!(validate(&f), Err(ValidationError::NoStartNode)));
    }

    #[test]
    fn no_end_fails() {
        let mut f = good_flow();
        f.nodes.retain(|n| !node_kind_is(&n.kind, "end"));
        assert!(matches!(validate(&f), Err(ValidationError::NoEndNode)));
    }

    #[test]
    fn orphan_node_fails() {
        let mut f = good_flow();
        f.nodes.push(node("orphan", "Orphan", "task", 50, None));
        assert!(matches!(validate(&f), Err(ValidationError::OrphanNode(_))));
    }

    #[test]
    fn guard_without_tags_fails() {
        let mut f = good_flow();
        // insert guard node + connect it in the flow
        f.nodes.insert(1, node("g", "Guard", "guard", 10, None));
        f.edges.clear();
        f.edges.push(edge("s", "a"));
        f.edges.push(edge("a", "g")); // guard receives traffic
        f.edges.push(edge("g", "e"));
        assert!(matches!(validate(&f), Err(ValidationError::GuardWithoutTags(_))));
    }

    #[test]
    fn missing_edge_node_fails() {
        let mut f = good_flow();
        f.edges.push(edge("a", "nonexistent"));
        assert!(matches!(validate(&f), Err(ValidationError::MissingNode { .. })));
    }
}

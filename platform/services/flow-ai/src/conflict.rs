//! 冲突检测与前置拦截
//!
//! 覆盖需求中列出的四类冲突 + 结构缺陷 + 合规规则：
//! 1. 数据库读写事务冲突
//! 2. 浏览器多实例抢占
//! 3. 文件读写锁冲突
//! 4. 政务数据脱敏 / 合规规则冲突（缺失 Guard）
//! 5. 结构缺陷：环、不可达节点、悬垂节点、分支缺失默认分支、非幂等节点无异常边
//!
//! 检测在**代码生成之前**执行，命中 Blocking 即拒绝出码 —— 这就是「异常分支前置拦截」。

use crate::model::{
    EdgeKind, FlowEdge, FlowGraph, FlowNode, NodeKind, Severity, ToolKind,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// 冲突类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    /// 并发写同一数据库表 / 事务交叉
    DbTransaction,
    /// 浏览器单实例被并发抢占
    BrowserContention,
    /// 文件读写锁冲突
    FileLock,
    /// 合规规则违反（如敏感数据未脱敏）
    Compliance,
    /// 流程存在环
    Cycle,
    /// 节点不可达
    Unreachable,
    /// 悬垂：无出边且非 End
    DanglingEnd,
    /// 判断节点分支不完整
    IncompleteBranch,
    /// 高风险节点缺少异常处理
    MissingExceptionPath,
    /// 非幂等节点被放入可重试并行段
    UnsafeRetry,
}

/// 一条冲突诊断
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub kind: ConflictKind,
    pub severity: Severity,
    /// 涉及节点
    pub nodes: Vec<String>,
    /// 涉及资源
    pub resource: Option<String>,
    pub message: String,
    /// 自动修正建议（可直接应用）
    pub remedy: Option<Remedy>,
}

impl Conflict {
    /// 构造一个冲突（供测试 / 外部注入使用）。
    pub fn new(
        kind: ConflictKind,
        severity: Severity,
        nodes: Vec<String>,
        resource: Option<String>,
        message: impl Into<String>,
        remedy: Option<Remedy>,
    ) -> Self {
        Self { kind, severity, nodes, resource, message: message.into(), remedy }
    }
}

/// 可自动应用的修正动作
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Remedy {
    /// 在两节点之间加互斥序（串行化）
    Serialize { first: String, second: String },
    /// 插入 Guard 校验节点
    InsertGuard { before: String, tag: String, name: String },
    /// 为节点补一条异常边到指定处理节点
    AddExceptionEdge { from: String, to: String },
    /// 提示人工处理
    Manual { hint: String },
}

/// 冲突报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    pub conflicts: Vec<Conflict>,
}

impl ConflictReport {
    pub fn blocking(&self) -> Vec<&Conflict> {
        self.conflicts.iter().filter(|c| c.severity == Severity::Blocking).collect()
    }
    pub fn has_blocking(&self) -> bool {
        self.conflicts.iter().any(|c| c.severity == Severity::Blocking)
    }
    pub fn count_of(&self, kind: ConflictKind) -> usize {
        self.conflicts.iter().filter(|c| c.kind == kind).count()
    }
}

/// 主检测入口
///
/// `concurrent_groups`: 由数据流分析给出的可并行层，用于判定「并发访问」冲突。
pub fn detect(graph: &FlowGraph, concurrent_groups: &[Vec<String>]) -> ConflictReport {
    let mut conflicts = Vec::new();
    structural_checks(graph, &mut conflicts);
    concurrency_checks(graph, concurrent_groups, &mut conflicts);
    compliance_checks(graph, &mut conflicts);
    conflicts.sort_by(|a, b| b.severity.cmp(&a.severity).then(a.message.cmp(&b.message)));
    ConflictReport { conflicts }
}

// ---------------- 结构检查 ----------------

fn structural_checks(graph: &FlowGraph, out: &mut Vec<Conflict>) {
    // 环
    if let Err(cyc) = graph.topo_order() {
        // 循环节点对 (LoopStart/LoopEnd) 构成的回边是合法的，只要显式声明
        let declared_loop = cyc.iter().any(|id| {
            graph
                .node(id)
                .map(|n| matches!(n.kind, NodeKind::LoopStart | NodeKind::LoopEnd))
                .unwrap_or(false)
        });
        out.push(Conflict {
            kind: ConflictKind::Cycle,
            severity: if declared_loop { Severity::Warning } else { Severity::Blocking },
            nodes: cyc.clone(),
            resource: None,
            message: if declared_loop {
                format!("检测到显式循环结构，涉及 {} 个节点，请确认存在终止条件", cyc.len())
            } else {
                format!("流程存在未声明的环（死循环风险），涉及节点: {}", cyc.join(", "))
            },
            remedy: Some(Remedy::Manual {
                hint: "将回边改为 LoopStart/LoopEnd 显式循环，并补充最大迭代次数".into(),
            }),
        });
        // 有环时后续可达性分析不可靠，直接返回结构部分
        return;
    }

    let n = graph.nodes.len();
    let succ = graph.successors();
    let pred = graph.predecessors();

    // 不可达（从任一 Start 出发）
    let starts: Vec<usize> = (0..n)
        .filter(|&i| graph.nodes[i].kind == NodeKind::Start || pred[i].is_empty())
        .collect();
    let mut seen = vec![false; n];
    let mut stack = starts.clone();
    for &s in &starts {
        seen[s] = true;
    }
    while let Some(u) = stack.pop() {
        for &v in &succ[u] {
            if !seen[v] {
                seen[v] = true;
                stack.push(v);
            }
        }
    }
    for (i, node) in graph.nodes.iter().enumerate() {
        if !seen[i] {
            out.push(Conflict {
                kind: ConflictKind::Unreachable,
                severity: Severity::Warning,
                nodes: vec![node.id.clone()],
                resource: None,
                message: format!("节点 `{}` 从起点不可达，属于死代码", node.name),
                remedy: Some(Remedy::Manual { hint: "删除该节点或补充入边".into() }),
            });
        }
    }

    for (i, node) in graph.nodes.iter().enumerate() {
        // 悬垂终点
        if succ[i].is_empty() && node.kind != NodeKind::End {
            out.push(Conflict {
                kind: ConflictKind::DanglingEnd,
                severity: Severity::Warning,
                nodes: vec![node.id.clone()],
                resource: None,
                message: format!("节点 `{}` 没有后继且不是终点，流程会静默中断", node.name),
                remedy: Some(Remedy::Manual { hint: "连接到 End 节点或后续任务".into() }),
            });
        }

        // 判断节点分支完整性
        if node.kind == NodeKind::Decision {
            let outs: Vec<&FlowEdge> = graph.edges.iter().filter(|e| e.from == node.id).collect();
            let has_default = outs.iter().any(|e| e.condition.is_none() || e.condition.as_deref() == Some("else"));
            if outs.len() < 2 {
                out.push(Conflict {
                    kind: ConflictKind::IncompleteBranch,
                    severity: Severity::Blocking,
                    nodes: vec![node.id.clone()],
                    resource: None,
                    message: format!("判断节点 `{}` 只有 {} 条出边，分支不完整", node.name, outs.len()),
                    remedy: Some(Remedy::Manual { hint: "补齐 true/false 或 else 分支".into() }),
                });
            } else if !has_default {
                out.push(Conflict {
                    kind: ConflictKind::IncompleteBranch,
                    severity: Severity::Warning,
                    nodes: vec![node.id.clone()],
                    resource: None,
                    message: format!("判断节点 `{}` 缺少默认(else)分支，条件全不命中时会卡死", node.name),
                    remedy: Some(Remedy::Manual { hint: "增加 else 兜底分支".into() }),
                });
            }
        }

        // 外部高风险节点缺异常边
        let risky = matches!(
            node.tool,
            Some(ToolKind::Browser) | Some(ToolKind::Database) | Some(ToolKind::Http) | Some(ToolKind::Shell)
        );
        if risky {
            let has_exc = graph
                .edges
                .iter()
                .any(|e| e.from == node.id && e.kind == EdgeKind::Exception);
            if !has_exc {
                out.push(Conflict {
                    kind: ConflictKind::MissingExceptionPath,
                    severity: Severity::Warning,
                    nodes: vec![node.id.clone()],
                    resource: None,
                    message: format!(
                        "外部调用节点 `{}`（{:?}）没有异常分支，失败时无兜底",
                        node.name,
                        node.tool.unwrap()
                    ),
                    remedy: Some(Remedy::AddExceptionEdge {
                        from: node.id.clone(),
                        to: "__error_handler".into(),
                    }),
                });
            }
        }
    }
}

// ---------------- 并发冲突检查 ----------------

fn concurrency_checks(graph: &FlowGraph, groups: &[Vec<String>], out: &mut Vec<Conflict>) {
    for group in groups {
        let nodes: Vec<&FlowNode> = group.iter().filter_map(|id| graph.node(id)).collect();
        if nodes.len() < 2 {
            continue;
        }

        // 浏览器实例抢占
        let browsers: Vec<&&FlowNode> = nodes
            .iter()
            .filter(|n| n.tool == Some(ToolKind::Browser))
            .collect();
        let cap = graph.capacity_of(ToolKind::Browser.resource_pool());
        if browsers.len() as u32 > cap {
            out.push(Conflict {
                kind: ConflictKind::BrowserContention,
                severity: Severity::Blocking,
                nodes: browsers.iter().map(|n| n.id.clone()).collect(),
                resource: Some("browser".into()),
                message: format!(
                    "{} 个浏览器任务被安排并发执行，但实例容量仅 {}，会互相抢占页面",
                    browsers.len(),
                    cap
                ),
                remedy: Some(Remedy::Serialize {
                    first: browsers[0].id.clone(),
                    second: browsers[1].id.clone(),
                }),
            });
        }

        // 两两资源冲突
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                let (a, b) = (nodes[i], nodes[j]);
                if let Some((res, both_write)) = resource_clash(a, b) {
                    let (kind, sev) = classify_clash(a, b, both_write);
                    out.push(Conflict {
                        kind,
                        severity: sev,
                        nodes: vec![a.id.clone(), b.id.clone()],
                        resource: Some(res.clone()),
                        message: format!(
                            "并发节点 `{}` 与 `{}` 同时访问 `{}`（{}），存在{}冲突",
                            a.name,
                            b.name,
                            res,
                            if both_write { "均写入" } else { "读写混合" },
                            match kind {
                                ConflictKind::DbTransaction => "事务",
                                ConflictKind::FileLock => "文件锁",
                                _ => "资源",
                            }
                        ),
                        remedy: Some(Remedy::Serialize {
                            first: a.id.clone(),
                            second: b.id.clone(),
                        }),
                    });
                }

                // 事务交叉
                if a.transactional && b.transactional && shares_any(a, b) {
                    out.push(Conflict {
                        kind: ConflictKind::DbTransaction,
                        severity: Severity::Blocking,
                        nodes: vec![a.id.clone(), b.id.clone()],
                        resource: None,
                        message: format!(
                            "事务节点 `{}` 与 `{}` 并发且共享资源，可能产生死锁",
                            a.name, b.name
                        ),
                        remedy: Some(Remedy::Serialize {
                            first: a.id.clone(),
                            second: b.id.clone(),
                        }),
                    });
                }
            }
        }

        // 并行段中的非幂等节点
        for nd in &nodes {
            if nd.kind.is_executable() && !nd.idempotent && group.len() > 1
                && matches!(nd.tool, Some(ToolKind::Http) | Some(ToolKind::Shell)) {
                    out.push(Conflict {
                        kind: ConflictKind::UnsafeRetry,
                        severity: Severity::Info,
                        nodes: vec![nd.id.clone()],
                        resource: None,
                        message: format!(
                            "节点 `{}` 非幂等却处于并行段，重试可能造成重复副作用",
                            nd.name
                        ),
                        remedy: Some(Remedy::Manual {
                            hint: "标记 idempotent=true 或增加幂等键".into(),
                        }),
                    });
            }
        }
    }
}

/// 返回 (冲突资源, 是否双写)
fn resource_clash(a: &FlowNode, b: &FlowNode) -> Option<(String, bool)> {
    let aw = a.write_set();
    let bw = b.write_set();
    let ar = a.read_set();
    let br = b.read_set();
    if let Some(r) = aw.intersection(&bw).next() {
        return Some((r.to_string(), true));
    }
    let rw: BTreeSet<&&str> = aw.intersection(&br).collect();
    if let Some(r) = rw.into_iter().next() {
        return Some((r.to_string(), false));
    }
    let wr: BTreeSet<&&str> = ar.intersection(&bw).collect();
    if let Some(r) = wr.into_iter().next() {
        return Some((r.to_string(), false));
    }
    None
}

fn classify_clash(a: &FlowNode, b: &FlowNode, both_write: bool) -> (ConflictKind, Severity) {
    let is_db = a.tool == Some(ToolKind::Database) || b.tool == Some(ToolKind::Database);
    let is_file = a.tool == Some(ToolKind::File) || b.tool == Some(ToolKind::File);
    let sev = if both_write { Severity::Blocking } else { Severity::Warning };
    if is_db {
        (ConflictKind::DbTransaction, sev)
    } else if is_file {
        (ConflictKind::FileLock, sev)
    } else {
        (ConflictKind::FileLock, Severity::Info)
    }
}

fn shares_any(a: &FlowNode, b: &FlowNode) -> bool {
    let mut sa: BTreeSet<&str> = a.read_set();
    sa.extend(a.write_set());
    let mut sb: BTreeSet<&str> = b.read_set();
    sb.extend(b.write_set());
    sa.intersection(&sb).next().is_some()
}

// ---------------- 合规规则检查 ----------------

fn compliance_checks(graph: &FlowGraph, out: &mut Vec<Conflict>) {
    if graph.rules.is_empty() {
        return;
    }
    let pred = graph.predecessors();
    // 每个节点的所有祖先 Guard 标签
    let ancestors_tags = ancestor_guard_tags(graph, &pred);

    for (i, node) in graph.nodes.iter().enumerate() {
        for rule in &graph.rules {
            if !rule_matches(rule, node) {
                continue;
            }
            let missing: Vec<&String> = rule
                .required_guard_tags
                .iter()
                .filter(|t| !ancestors_tags[i].contains(t.as_str()) && !node.tags.contains(t))
                .collect();
            if missing.is_empty() {
                continue;
            }
            let tag = missing[0].clone();
            out.push(Conflict {
                kind: ConflictKind::Compliance,
                severity: rule.severity,
                nodes: vec![node.id.clone()],
                resource: node.accesses.first().map(|a| a.resource.clone()),
                message: format!(
                    "[{}] 节点 `{}` 违反规则：{}（缺少前置校验: {}）",
                    rule.id,
                    node.name,
                    rule.description,
                    missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                ),
                remedy: Some(Remedy::InsertGuard {
                    before: node.id.clone(),
                    tag: tag.clone(),
                    name: format!("{} 校验", tag),
                }),
            });
        }
    }
}

fn rule_matches(rule: &crate::model::ExpertRule, node: &FlowNode) -> bool {
    let tool_hit = rule
        .tool_kinds
        .iter()
        .any(|t| Some(*t) == node.tool);
    let res_hit = node.accesses.iter().any(|a| {
        rule.resource_prefixes
            .iter()
            .any(|p| a.resource.starts_with(p.as_str()))
    });
    if rule.tool_kinds.is_empty() {
        res_hit
    } else if rule.resource_prefixes.is_empty() {
        tool_hit
    } else {
        tool_hit && res_hit
    }
}

fn ancestor_guard_tags<'a>(graph: &'a FlowGraph, pred: &[Vec<usize>]) -> Vec<BTreeSet<&'a str>> {
    let n = graph.nodes.len();
    let mut tags: Vec<BTreeSet<&str>> = vec![BTreeSet::new(); n];
    let order = graph.topo_order().unwrap_or_else(|_| (0..n).collect());
    for &u in &order {
        let mut acc: BTreeSet<&str> = BTreeSet::new();
        for &p in &pred[u] {
            for t in &tags[p] {
                acc.insert(t);
            }
            if graph.nodes[p].kind == NodeKind::Guard {
                for t in &graph.nodes[p].tags {
                    acc.insert(t.as_str());
                }
            }
        }
        tags[u] = acc;
    }
    tags
}

/// 自动应用可修复的修正建议，返回修正后的新图与已应用的修正数
pub fn auto_repair(graph: &FlowGraph, report: &ConflictReport) -> (FlowGraph, usize) {
    let mut g = graph.clone();
    let mut applied = 0usize;
    let mut error_handler_added = false;

    for c in &report.conflicts {
        match &c.remedy {
            Some(Remedy::Serialize { first, second }) => {
                if g.node(first).is_none() || g.node(second).is_none() {
                    continue;
                }
                // 已存在硬互斥 → 无需重复处理
                if g.edges.iter().any(|e| {
                    e.kind == EdgeKind::Mutex
                        && ((e.from == *first && e.to == *second)
                            || (e.from == *second && e.to == *first))
                }) {
                    continue;
                }
                // 已存在软边（顺序/推断）→ 升级为硬互斥。
                // 否则后续数据流分析会把它当伪依赖剪掉，冲突死灶复燃。
                if let Some(e) = g.edges.iter_mut().find(|e| {
                    !e.kind.is_hard()
                        && ((e.from == *first && e.to == *second)
                            || (e.from == *second && e.to == *first))
                }) {
                    e.kind = EdgeKind::Mutex;
                    applied += 1;
                    continue;
                }
                {
                    // 使用 Mutex 硬约束，确保后续数据流分析不会把它当作伪依赖剪掉
                    g.edges.push(FlowEdge::mutex(first.clone(), second.clone()));
                    // 保持 DAG：若引入了环则改为反向互斥，再不行则撤销
                    if g.topo_order().is_err() {
                        g.edges.pop();
                        g.edges.push(FlowEdge::mutex(second.clone(), first.clone()));
                        if g.topo_order().is_err() {
                            g.edges.pop();
                        } else {
                            applied += 1;
                        }
                    } else {
                        applied += 1;
                    }
                }
            }
            Some(Remedy::InsertGuard { before, tag, name }) => {
                let gid = format!("__guard_{}_{}", tag, before);
                if g.node(&gid).is_some() {
                    continue;
                }
                let mut guard = FlowNode::new(gid.clone(), name.clone(), NodeKind::Guard);
                guard.tags.push(tag.clone());
                guard.duration_ms = 5;
                g.nodes.push(guard);
                // 重接: 所有 before 的入边改指向 guard, guard→before
                for e in g.edges.iter_mut() {
                    if e.to == *before {
                        e.to = gid.clone();
                    }
                }
                g.edges.push(FlowEdge::seq(gid.clone(), before.clone()));
                applied += 1;
            }
            Some(Remedy::AddExceptionEdge { from, to }) => {
                if !error_handler_added && g.node(to).is_none() {
                    let mut h = FlowNode::new(to.clone(), "统一异常处理", NodeKind::Guard);
                    h.tags.push("error_handler".into());
                    g.nodes.push(h);
                    // 把处理器接到终点，否则它自己就是一个“悬垂节点”缺陷
                    if let Some(end) = g
                        .nodes
                        .iter()
                        .find(|n| n.kind == NodeKind::End)
                        .map(|n| n.id.clone())
                    {
                        g.edges.push(FlowEdge::seq(to.clone(), end));
                    }
                    error_handler_added = true;
                }
                let dup = g
                    .edges
                    .iter()
                    .any(|e| e.from == *from && e.to == *to && e.kind == EdgeKind::Exception);
                if !dup {
                    g.edges.push(FlowEdge::exception(from.clone(), to.clone()));
                    applied += 1;
                }
            }
            _ => {}
        }
    }
    (g, applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Access, ExpertRule, FlowEdge, FlowNode, ToolKind};

    #[test]
    fn detects_browser_contention() {
        let mut g = FlowGraph::new("b", "browser");
        g.add_node(FlowNode::task("b1", "抓取A", ToolKind::Browser, 100));
        g.add_node(FlowNode::task("b2", "抓取B", ToolKind::Browser, 100));
        let rep = detect(&g, &[vec!["b1".into(), "b2".into()]]);
        assert!(rep.count_of(ConflictKind::BrowserContention) >= 1);
        assert!(rep.has_blocking());
    }

    #[test]
    fn detects_file_lock() {
        let mut g = FlowGraph::new("f", "file");
        g.add_node(
            FlowNode::task("f1", "写报表", ToolKind::File, 10).with_access(Access::write("file:r.xlsx")),
        );
        g.add_node(
            FlowNode::task("f2", "改报表", ToolKind::File, 10).with_access(Access::write("file:r.xlsx")),
        );
        let rep = detect(&g, &[vec!["f1".into(), "f2".into()]]);
        assert!(rep.count_of(ConflictKind::FileLock) >= 1);
    }

    #[test]
    fn detects_missing_desensitize_guard() {
        let mut g = FlowGraph::new("c", "compliance");
        g.add_node(
            FlowNode::task("q", "查询公民信息", ToolKind::Database, 50)
                .with_access(Access::read("db:citizen_info")),
        );
        g.rules.push(ExpertRule {
            id: "GOV-001".into(),
            description: "公民敏感数据必须先脱敏".into(),
            severity: Severity::Blocking,
            resource_prefixes: vec!["db:citizen_".into()],
            tool_kinds: vec![],
            required_guard_tags: vec!["desensitize".into()],
        });
        let rep = detect(&g, &[]);
        assert!(rep.count_of(ConflictKind::Compliance) == 1);
        assert!(rep.has_blocking());
    }

    #[test]
    fn auto_repair_inserts_guard_and_clears_conflict() {
        let mut g = FlowGraph::new("c", "compliance");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(
            FlowNode::task("q", "查询公民信息", ToolKind::Database, 50)
                .with_access(Access::read("db:citizen_info")),
        );
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "q"));
        g.add_edge(FlowEdge::seq("q", "e"));
        g.rules.push(ExpertRule {
            id: "GOV-001".into(),
            description: "公民敏感数据必须先脱敏".into(),
            severity: Severity::Blocking,
            resource_prefixes: vec!["db:citizen_".into()],
            tool_kinds: vec![],
            required_guard_tags: vec!["desensitize".into()],
        });
        let rep = detect(&g, &[]);
        let (fixed, applied) = auto_repair(&g, &rep);
        assert!(applied >= 1);
        let rep2 = detect(&fixed, &[]);
        assert_eq!(rep2.count_of(ConflictKind::Compliance), 0, "修复后不应再有合规冲突");
        assert!(fixed.topo_order().is_ok());
    }

    #[test]
    fn detects_cycle_as_blocking() {
        let mut g = FlowGraph::new("cy", "cycle");
        g.add_node(FlowNode::task("a", "A", ToolKind::Compute, 1));
        g.add_node(FlowNode::task("b", "B", ToolKind::Compute, 1));
        g.add_edge(FlowEdge::seq("a", "b"));
        g.add_edge(FlowEdge::seq("b", "a"));
        let rep = detect(&g, &[]);
        assert!(rep.count_of(ConflictKind::Cycle) == 1);
        assert!(rep.has_blocking());
    }

    #[test]
    fn decision_without_else_warns() {
        let mut g = FlowGraph::new("d", "decision");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::new("d1", "是否合规", NodeKind::Decision));
        g.add_node(FlowNode::task("t1", "通过", ToolKind::Compute, 1));
        g.add_node(FlowNode::task("t2", "驳回", ToolKind::Compute, 1));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "d1"));
        g.add_edge(FlowEdge::cond("d1", "t1", "ok == true"));
        g.add_edge(FlowEdge::cond("d1", "t2", "ok == false"));
        g.add_edge(FlowEdge::seq("t1", "e"));
        g.add_edge(FlowEdge::seq("t2", "e"));
        let rep = detect(&g, &[]);
        assert!(rep.count_of(ConflictKind::IncompleteBranch) == 1);
        assert!(!rep.has_blocking());
    }
}

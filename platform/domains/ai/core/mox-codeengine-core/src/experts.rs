// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Team / Diagnose 阶段：开发专家联盟的四位内置角色专家。
//!
//! 每位专家是一个**自研确定性规则推理器**（角色 = 关注面 + 规则集 + 打分函数），
//! 相互独立、可并行执行——对齐联盟处理模式「多专家并行诊断 → 结果融合」。
//!
//! | 专家 | 角色 | 关注面 |
//! |------|------|--------|
//! | analyst    | 需求分析师 | 结构完整性：start/end、可达性、步骤规模 |
//! | builder    | 构建专家   | 可优化性：加速比、并行层、伪依赖剪除 |
//! | auditor    | 合规审计师 | 安全合规：敏感数据护栏、阻断冲突、高危工具（一票否决） |
//! | coordinator| 协调官     | 协作质量：修复覆盖、算力路由、模型降级 |

use crate::ir::Requirement;
use mox_ai_flow_core::model::{FlowGraph, NodeKind, Severity, ToolKind};
use mox_ai_flow_core::pipeline::OptimizationReport;
use serde::Serialize;

/// 诊断上下文：三位一体的证据（原始需求 IR / 原始图 / 求解报告）
pub struct DiagContext<'a> {
    pub req: &'a Requirement,
    pub raw: &'a FlowGraph,
    pub report: &'a OptimizationReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// 稳定错误码，如 "CE-A-001"
    pub code: &'static str,
    pub expert: &'static str,
    pub severity: Severity,
    pub message: String,
    pub suggestion: String,
    pub confidence: f64,
    /// 结构化定位（自修复循环的突变目标节点 id / id 列表）
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpertOpinion {
    pub expert: &'static str,
    pub role: &'static str,
    /// 0.0~1.0，专家对「本需求可直接出码」的信心分
    pub score: f64,
    pub findings: Vec<Finding>,
}

impl ExpertOpinion {
    pub fn blocking_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Blocking)
            .count()
    }
    fn base(expert: &'static str, role: &'static str) -> Self {
        Self {
            expert,
            role,
            score: 1.0,
            findings: Vec::new(),
        }
    }
    fn penalize(&mut self, per_blocking: f64, per_warning: f64, per_info: f64) {
        for f in &self.findings {
            self.score -= match f.severity {
                Severity::Blocking => per_blocking,
                Severity::Warning => per_warning,
                Severity::Info => per_info,
            };
        }
        self.score = self.score.clamp(0.0, 1.0);
    }
}

/// 专家契约：每个联盟成员实现自己的独立诊断
pub trait Expert {
    fn id() -> &'static str;
    fn role() -> &'static str;
    fn consult(ctx: &DiagContext) -> ExpertOpinion;
}

fn detect_finding(
    expert: &'static str,
    code: &'static str,
    severity: Severity,
    message: impl Into<String>,
    suggestion: impl Into<String>,
    confidence: f64,
) -> Finding {
    Finding {
        code,
        expert,
        severity,
        message: message.into(),
        suggestion: suggestion.into(),
        confidence,
        target: None,
    }
}

/// 从 start BFS / 到 end 反向 BFS 的可达性（自研，不依赖 petgraph）
fn reachable(graph: &FlowGraph, from: &str, forward: bool) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut queue = std::collections::VecDeque::new();
    seen.insert(from.to_string());
    queue.push_back(from.to_string());
    while let Some(cur) = queue.pop_front() {
        let nexts: Vec<&str> = graph
            .edges
            .iter()
            .filter(|e| {
                if forward {
                    e.from == cur
                } else {
                    e.to == cur
                }
            })
            .map(|e| if forward { e.to.as_str() } else { e.from.as_str() })
            .collect();
        for n in nexts {
            if seen.insert(n.to_string()) {
                queue.push_back(n.to_string());
            }
        }
    }
    seen.into_iter().collect()
}

// ─── analyst 需求分析师 ───────────────────────────────────────────────

pub struct Analyst;

impl Expert for Analyst {
    fn id() -> &'static str {
        "analyst"
    }
    fn role() -> &'static str {
        "需求分析师"
    }
    fn consult(ctx: &DiagContext) -> ExpertOpinion {
        const ID: &str = "analyst";
        let mut op = ExpertOpinion::base(Self::id(), Self::role());
        if ctx.req.steps.is_empty() {
            op.findings.push(detect_finding(
                ID,
                "CE-A-001",
                Severity::Blocking,
                "需求未能解析出任何可执行步骤",
                "请补充明确的动作句（每句一个动作）",
                0.95,
            ));
        }
        if ctx.raw.node("start").is_none() || ctx.raw.node("end").is_none() {
            op.findings.push(detect_finding(
                ID,
                "CE-A-002",
                Severity::Blocking,
                "流程图缺少 start/end 端点",
                "检查建图逻辑",
                1.0,
            ));
        }
        let from_start = reachable(ctx.raw, "start", true);
        let orphans: Vec<&str> = ctx
            .raw
            .nodes
            .iter()
            .filter(|n| n.kind != NodeKind::Start && !from_start.contains(&n.id))
            .map(|n| n.id.as_str())
            .collect();
        if !orphans.is_empty() {
            let mut f = detect_finding(
                ID,
                "CE-A-003",
                Severity::Warning,
                format!("不可达节点: {}", orphans.join(", ")),
                "为孤立节点补充依赖边或移除",
                0.9,
            );
            f.target = Some(orphans.join(", "));
            op.findings.push(f);
        }
        if ctx.req.steps.len() > 24 {
            op.findings.push(detect_finding(
                ID,
                "CE-A-004",
                Severity::Warning,
                format!("步骤数 {} 超过 24，建议拆分子流程", ctx.req.steps.len()),
                "拆分为 SubFlow 组合",
                0.7,
            ));
        }
        if op.findings.is_empty() {
            op.findings.push(detect_finding(
                ID,
                "CE-A-010",
                Severity::Info,
                format!("结构完整：{} 步、{} 边", ctx.raw.nodes.len(), ctx.raw.edges.len()),
                "无",
                0.9,
            ));
        }
        op.penalize(0.6, 0.15, 0.0);
        op
    }
}

// ─── builder 构建专家 ─────────────────────────────────────────────────

pub struct Builder;

impl Expert for Builder {
    fn id() -> &'static str {
        "builder"
    }
    fn role() -> &'static str {
        "构建专家"
    }
    fn consult(ctx: &DiagContext) -> ExpertOpinion {
        const ID: &str = "builder";
        let mut op = ExpertOpinion::base(Self::id(), Self::role());
        let g = &ctx.report.gains;
        if g.speedup < 1.05 && ctx.req.steps.len() > 1 {
            op.findings.push(detect_finding(
                ID,
                "CE-B-001",
                Severity::Warning,
                format!("加速比仅 {:.2}x，未挖出有效并行", g.speedup),
                "检查步骤间数据读写声明以暴露并行度",
                0.8,
            ));
        }
        if g.removed_false_deps > 0 {
            op.findings.push(detect_finding(
                ID,
                "CE-B-002",
                Severity::Info,
                format!("自动剪除伪依赖 {} 条，并行 {} 层 / 峰值并发 {}", g.removed_false_deps, g.parallel_layers, g.max_concurrency),
                "无",
                0.85,
            ));
        }
        if g.scheduled_ms == 0 && !ctx.req.steps.is_empty() {
            op.findings.push(detect_finding(
                ID,
                "CE-B-003",
                Severity::Warning,
                "排程结果为 0ms，疑似步骤耗时全缺省",
                "为 Task 节点补充 duration 估计",
                0.75,
            ));
        }
        op.penalize(0.6, 0.1, 0.0);
        op
    }
}

// ─── auditor 合规审计师（Blocking 一票否决） ──────────────────────────

pub struct Auditor;

impl Expert for Auditor {
    fn id() -> &'static str {
        "auditor"
    }
    fn role() -> &'static str {
        "合规审计师"
    }
    fn consult(ctx: &DiagContext) -> ExpertOpinion {
        const ID: &str = "auditor";
        let mut op = ExpertOpinion::base(Self::id(), Self::role());

        // ① 敏感资源必须有前置 desensitize 护栏（在**优化后**的图上核查）
        let opt = &ctx.report.optimized_graph;
        for node in &opt.nodes {
            if !node.kind.is_executable() {
                continue;
            }
            let has_sensitive = node
                .props
                .keys()
                .any(|k| k.starts_with("sensitive:"))
                || node
                    .accesses
                    .iter()
                    .any(|a| crate::ir::is_sensitive(&a.resource));
            if !has_sensitive {
                continue;
            }
            let guarded = opt.edges.iter().any(|e| {
                e.to == node.id
                    && opt
                        .node(&e.from)
                        .map(|p| p.kind == NodeKind::Guard && p.tags.iter().any(|t| t == "desensitize"))
                        .unwrap_or(false)
            });
            if !guarded {
                let mut f = detect_finding(
                    ID,
                    "CE-D-001",
                    Severity::Blocking,
                    format!("节点 {} 触碰敏感资源但缺少前置脱敏护栏", node.id),
                    "开启 auto_guard 或在需求中显式声明脱敏步骤",
                    0.98,
                );
                f.target = Some(node.id.clone());
                op.findings.push(f);
            }
        }

        // ② 阻断级冲突必须为零才可出码
        let blocking = ctx.report.conflicts.blocking();
        if !blocking.is_empty() {
            op.findings.push(detect_finding(
                ID,
                "CE-D-002",
                Severity::Blocking,
                format!("存在 {} 项阻断级资源冲突", blocking.len()),
                "查看冲突详情并为互斥资源注入串行边",
                1.0,
            ));
        }

        // ③ 高危工具（Shell/Browser）要求 path_check 护栏或人工节点兜底
        for node in &opt.nodes {
            if node.tool == Some(ToolKind::Shell) {
                let guarded = opt.edges.iter().any(|e| {
                    e.to == node.id
                        && opt.node(&e.from)
                            .map(|p| p.kind == NodeKind::Guard && p.tags.iter().any(|t| t == "path_check"))
                            .unwrap_or(false)
                });
                if !guarded {
                    let mut f = detect_finding(
                        ID,
                        "CE-D-003",
                        Severity::Warning,
                        format!("Shell 节点 {} 未挂 path_check 护栏", node.id),
                        "开启 auto_guard 自动注入命令白名单护栏",
                        0.85,
                    );
                    f.target = Some(node.id.clone());
                    op.findings.push(f);
                }
            }
        }

        if op.findings.is_empty() {
            op.findings.push(detect_finding(
                ID,
                "CE-D-010",
                Severity::Info,
                "合规审查通过：护栏齐备、零阻断冲突",
                "无",
                0.95,
            ));
        }
        op.penalize(1.0, 0.15, 0.0);
        op
    }
}

// ─── coordinator 协调官 ───────────────────────────────────────────────

pub struct Coordinator;

impl Expert for Coordinator {
    fn id() -> &'static str {
        "coordinator"
    }
    fn role() -> &'static str {
        "协调官"
    }
    fn consult(ctx: &DiagContext) -> ExpertOpinion {
        const ID: &str = "coordinator";
        let mut op = ExpertOpinion::base(Self::id(), Self::role());
        let g = &ctx.report.gains;
        if g.conflicts_found > g.conflicts_auto_fixed {
            op.findings.push(detect_finding(
                ID,
                "CE-C-001",
                Severity::Warning,
                format!(
                    "冲突 {} 项仅自动修复 {} 项，剩余需人工介入",
                    g.conflicts_found, g.conflicts_auto_fixed
                ),
                "在需求中声明互斥资源边界",
                0.8,
            ));
        }
        if g.compute_saved_pct <= 0.0 {
            op.findings.push(detect_finding(
                ID,
                "CE-C-002",
                Severity::Info,
                "算力路由未产生压缩（无 LLM 重任务，属正常）",
                "无",
                0.6,
            ));
        } else {
            op.findings.push(detect_finding(
                ID,
                "CE-C-003",
                Severity::Info,
                format!("模型分级路由压缩算力成本 {:.1}%", g.compute_saved_pct),
                "无",
                0.85,
            ));
        }
        op.penalize(0.6, 0.1, 0.0);
        op
    }
}

/// 联盟默认阵容：四角色全员并行
pub fn default_panel(ctx: &DiagContext) -> Vec<ExpertOpinion> {
    vec![
        Analyst::consult(ctx),
        Builder::consult(ctx),
        Auditor::consult(ctx),
        Coordinator::consult(ctx),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::build_graph;
    use crate::ir::parse_requirement;
    use mox_ai_flow_core::{optimize, OptimizeConfig};

    fn ctx_of(text: &str) -> (Requirement, FlowGraph, OptimizationReport) {
        let req = parse_requirement("t", "t", text);
        let g = build_graph(&req, true);
        let cfg = OptimizeConfig {
            emit_code: false,
            ..Default::default()
        };
        let report = optimize(&g, &cfg);
        (req, g, report)
    }

    #[test]
    fn clean_requirement_gets_high_scores() {
        let (req, raw, report) = ctx_of("读取 input.csv 文件；把结果写入 db.orders 表");
        let c = DiagContext {
            req: &req,
            raw: &raw,
            report: &report,
        };
        let panel = default_panel(&c);
        assert_eq!(panel.len(), 4);
        assert!(panel.iter().all(|o| o.score > 0.7));
    }

    #[test]
    fn auditor_blocks_unguarded_sensitive_write() {
        let req = parse_requirement("t", "t", "查询 db.citizen_idcard 表");
        let raw = build_graph(&req, false); // 关护栏 → 敏感节点裸奔
        let cfg = OptimizeConfig {
            emit_code: false,
            ..Default::default()
        };
        let report = optimize(&raw, &cfg);
        let c = DiagContext {
            req: &req,
            raw: &raw,
            report: &report,
        };
        let op = Auditor::consult(&c);
        assert_eq!(op.blocking_count(), 1, "应恰好命中 CE-D-001");
        assert_eq!(op.findings[0].code, "CE-D-001");
        assert!(op.score < 0.5);
    }
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! AI 大模型辅助编程 · 全维处理算法（Phase 1 落地）
//!
//! 把设计文档《ai_programming_algorithm_design》的 10 步流程图落成可执行流水线：
//!   ① 需求归一化(Normalize) → ② 流程图建模 → ③ 七专家审查 → ④ 裁决 →
//!   ⑤ 流程图优化 → ⑥ verify() 验证网关 → ⑦ 代码生成(Emit) →
//!   ⑧ 代码?图双向校验 → ⑨ 治理闸门 → ⑩ 交付可视化
//!
//! 核心护栏（对应"不能问了 AI 就处理"）：
//!   G-A: 任何 AI 产出默认「草稿·未确认」，不得直接进入执行/出码
//!   G-B: 每个执行动作必须映射到已确认流程图节点 + 处理流程 + 规范条款
//!   G-C: verify 通过 + ⑧一致 + ⑨ Approved，三证不全禁止出码
//!   G-D: AI 产出必须署名（哪个模型/专家视角），禁止"AI 说行"式无来源结论
//!   G-E: 任一闸门失败必须回退最近安全点，禁止将错就错

use crate::context::GovernContext;
use crate::govern::AuditChain;
use crate::pipeline::{mox_optimize, GovernanceReport};
use mox_ai_flow_svc::codegen::{generate, CodeBundle};
use mox_ai_flow_svc::model::{FlowGraph, FlowNode, NodeKind};
use serde::{Deserialize, Serialize};

/// 回退点：流程在任一闸门失败时回退到的最近安全态（护栏 G-E）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Checkpoint {
    /// 仅收到原始意图，尚未建模
    Intent,
    /// 已归一化需求书（① 通过）
    Normalized,
    /// 已建模 FlowGraph（② 通过）
    Modeled,
    /// 七专家+裁决+优化完成（③-⑤）
    Optimized,
    /// 验证网关通过（⑥）
    Verified,
    /// 代码生成+双向校验通过（⑦-⑧）
    Emitted,
    /// 治理闸门通过（⑨）→ 可交付
    Governed,
}

/// 草稿状态：所有 AI 产出默认未确认（护栏 G-A）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftStatus {
    /// AI 草稿，未确认，不可作为执行依据
    AiDraft,
    /// 已人工/治理确认
    Confirmed,
}

/// 归一化需求书（① 产出）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedRequirement {
    pub goal: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    /// 约束清单（每条必须可判定，规范 N1）
    pub constraints: Vec<String>,
    /// 禁忌操作清单（非空或显式"无"，规范 N3）
    pub forbidden: Vec<String>,
    /// 来源署名（护栏 G-D）：谁抽取的这条需求
    pub authored_by: String,
    /// 草稿状态（护栏 G-A）
    pub status: DraftStatus,
}

impl NormalizedRequirement {
    /// 护栏 G-A + 规范 N2/N3：草稿未确认 / 约束为空抽象 / forbidden 未声明 → 不可进入建模
    pub fn is_confirmed(&self) -> bool {
        if self.status != DraftStatus::Confirmed {
            return false; // G-A: AI 草稿不可直接处理
        }
        let concrete = self
            .constraints
            .iter()
            .all(|c| !c.is_empty() && !is_vague(c));
        let forbidden_declared = self
            .forbidden
            .iter()
            .any(|f| !f.is_empty() && (f == "无" || f.eq_ignore_ascii_case("none")));
        concrete && forbidden_declared
    }
}

fn is_vague(s: &str) -> bool {
    let v = s.trim();
    v.is_empty()
        || v == "尽量"
        || v == "差不多"
        || v.contains("尽量")
        || v.contains("差不多")
        || v.contains("尽可能")
}

/// 从大模型抽取结果构建需求书（默认草稿·未确认，规范 N2）
pub fn from_llm_extract(goal: &str, extracted: Vec<String>) -> NormalizedRequirement {
    NormalizedRequirement {
        goal: goal.into(),
        inputs: Vec::new(),
        outputs: Vec::new(),
        constraints: extracted,
        forbidden: Vec::new(),
        authored_by: "AI-extract(unconfirmed)".into(),
        status: DraftStatus::AiDraft,
    }
}

/// 需求归一化（步骤①）：把原始意图转成可判定需求书。
/// `confirmed` 表示是否已由人工/治理确认（护栏 G-A 开关）。
pub fn normalize_requirement(
    raw_intent: &str,
    extracted_constraints: Vec<String>,
    confirmed: bool,
) -> NormalizedRequirement {
    let status = if confirmed {
        DraftStatus::Confirmed
    } else {
        DraftStatus::AiDraft
    };
    NormalizedRequirement {
        goal: raw_intent.into(),
        inputs: Vec::new(),
        outputs: Vec::new(),
        constraints: extracted_constraints,
        forbidden: vec!["无".into()], // 规范 N3: 显式声明
        authored_by: if confirmed {
            "human+ai".into()
        } else {
            "AI-extract(unconfirmed)".into()
        },
        status,
    }
}

/// 编程全维处理报告（①-⑩ 总报告）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgrammingReport {
    pub flow_id: String,
    /// 回退点（护栏 G-E）
    pub checkpoint: Checkpoint,
    /// 是否可安全交付（三证齐全：verify + 双向一致 + 治理 Approved）
    pub safe_to_emit: bool,
    /// ① 需求书
    pub requirement: NormalizedRequirement,
    /// ②-⑥⑨ 治理报告（专家/裁决/优化/验证/治理）
    pub governance: Option<GovernanceReport>,
    /// ⑦ 出码产物（仅三证齐全时存在）
    pub code: Option<CodeBundle>,
    /// ⑧ 双向映射校验结论
    pub roundtrip_ok: Option<bool>,
    /// ⑩ 审计链
    pub audit: AuditChain,
}

/// 顶层编排：AI 大模型辅助编程全维处理流水线（①-⑩）。
///
/// 护栏执行顺序（对应设计文档第 5 章）：
///   G-A: 需求书未确认 → 直接止于 Normalized 回退点，不出码
///   G-C: ⑥ veto / ⑧ 不一致 / ⑨ 未 Approved → 禁止出码，回退最近安全点
///   G-E: 任一失败回退 checkpoint，不将错就错
pub fn programming_pipeline(
    raw_intent: &str,
    ai_extracted: Vec<String>,
    requirement_confirmed: bool,
    graph: &FlowGraph,
    ctx: &GovernContext,
) -> ProgrammingReport {
    let mut audit = AuditChain::new();

    // ① 需求归一化 + 护栏 G-A
    let req = normalize_requirement(raw_intent, ai_extracted, requirement_confirmed);
    if !req.is_confirmed() {
        audit.append(
            &ctx.principal.subject,
            &graph.id,
            "normalize",
            "blocked:draft-or-vague",
        );
        // G-A: AI 草稿/模糊约束，禁止进入建模与出码
        return ProgrammingReport {
            flow_id: graph.id.clone(),
            checkpoint: Checkpoint::Normalized,
            safe_to_emit: false,
            requirement: req,
            governance: None,
            code: None,
            roundtrip_ok: None,
            audit,
        };
    }
    audit.append(&ctx.principal.subject, &graph.id, "normalize", "confirmed");

    // ①.5 循环护栏：消费 ctx.registry.loops，拦截无界/未登记循环（兼容性缺口修复）
    if let Some(reason) = check_loops(graph, ctx) {
        audit.append(
            &ctx.principal.subject,
            &graph.id,
            "loop_guard",
            &format!("vetoed:{}", reason),
        );
        // G-E: 回退到建模后最近安全点（尚未进入专家/优化，无副作用）
        return ProgrammingReport {
            flow_id: graph.id.clone(),
            checkpoint: Checkpoint::Modeled,
            safe_to_emit: false,
            requirement: req,
            governance: None,
            code: None,
            roundtrip_ok: None,
            audit,
        };
    }

    // ①.6 生产环境写保护已从编排补丁迁移至 permission 专家 push_veto（正交机制），
    //      此处不再重复检查。
    // ②-⑥⑨ 复用已验证的 mox_optimize（含七专家/裁决/优化/verify/治理）
    let gov = mox_optimize(graph, ctx);

    // ⑥ 验证网关优先级铁律：algo.vetoed 必须 BLOCK（即使治理批准）
    if gov.algo.vetoed {
        audit.append(&ctx.principal.subject, &graph.id, "verify", "vetoed");
        return ProgrammingReport {
            flow_id: graph.id.clone(),
            checkpoint: Checkpoint::Optimized,
            safe_to_emit: false,
            requirement: req,
            governance: Some(gov),
            code: None,
            roundtrip_ok: None,
            audit,
        };
    }

    // ⑨ 治理闸门未 Approved → 禁止出码（G-C 第三证缺失）
    if !gov.gate.approved {
        audit.append(
            &ctx.principal.subject,
            &graph.id,
            "govern",
            &format!("blocked:{}", gov.gate.reason),
        );
        // checkpoint 已是治理后态：回退到治理闸门前的最近安全点（Emitted 之上即 Optimized/Governed）
        return ProgrammingReport {
            flow_id: graph.id.clone(),
            checkpoint: Checkpoint::Governed,
            safe_to_emit: false,
            requirement: req,
            governance: Some(gov),
            code: None,
            roundtrip_ok: None,
            audit,
        };
    }

    // ⑦ Emit：独立调用 codegen（不在 optimize 内部出码，堵住 C1 红线：未过验证网关不得出码）
    //     出码前把 LoopGuard(Bounded) 的 max_iter 桥接到对应 LoopStart 节点 props，
    //     供 codegen 生成迭代上限护栏（有界循环不得无限重放）。
    let mut emit_graph = gov.optimization.optimized_graph.clone();
    for lg in &ctx.registry.loops {
        if let crate::context::LoopPolicy::Bounded { max_iter } = lg.policy {
            if let Some(n) = emit_graph.node_mut(&lg.node) {
                n.props.insert("max_iter".into(), max_iter.to_string());
            }
        }
    }
    let opt = &gov.optimization;
    let code = generate(&emit_graph, &opt.plan, &opt.schedule, &opt.conflicts);

    // ⑧ 代码?图双向映射校验（护栏 G-C 第二证）
    let roundtrip_ok = verify_code_roundtrip(&opt.optimized_graph, &code);
    if !roundtrip_ok {
        audit.append(
            &ctx.principal.subject,
            &graph.id,
            "roundtrip",
            "mismatch:rejected",
        );
        return ProgrammingReport {
            flow_id: graph.id.clone(),
            checkpoint: Checkpoint::Emitted,
            safe_to_emit: false,
            requirement: req,
            governance: Some(gov),
            code: Some(code),
            roundtrip_ok: Some(false),
            audit,
        };
    }

    // ⑩ 审计闭环
    audit.append(&ctx.principal.subject, &graph.id, "emit", "approved");
    ProgrammingReport {
        flow_id: graph.id.clone(),
        checkpoint: Checkpoint::Governed,
        safe_to_emit: true,
        requirement: req,
        governance: Some(gov),
        code: Some(code),
        roundtrip_ok: Some(true),
        audit,
    }
}

/// ⑧ 双向映射：代码?图一致性校验（与设计 M1/M2 一致）。
///
/// 判据与 flow-ai `verify::code_roundtrip_invariant` 对齐（已验证行为）：
///   - code.rejected（阻断冲突未解）→ 不一致（M2 硬红线，必须阻断）
///   - 反向解析未识别到工具节点 → 仅告警不阻断（反向解析器对调度层结构保守，
///     不误杀正常出码；与设计文档「代码往返失败最多告警」一致）
///   - 因此⑧在此作为**强校验的兜底**：code.rejected 仍判 false（M2 硬红线）。
///     此外本函数做**节点级双向映射校验**（迭代 5-① 增强）：正向生成的代码里，
///     每个「可执行节点」（Task/SubFlow/Guard，含子流程）必须有对应的函数定义
///     `def py_ident(id)(ctx:`，否则属「图节点未映射到代码」的双向不一致，判 false。
///     控制节点（Decision/Loop/Parallel 网关）由 scheduler 体现，不在此逐节点校验。
fn verify_code_roundtrip(graph: &FlowGraph, code: &CodeBundle) -> bool {
    if code.rejected {
        return false; // M2 硬红线：阻断冲突未解即不一致
    }
    let joined = code
        .files
        .iter()
        .map(|f| f.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for n in graph.nodes.iter().filter(|n| n.kind.is_executable()) {
        let fname = mox_ai_flow_svc::codegen::py_ident(&n.id);
        let def = format!("def {}(ctx:", fname);
        if !joined.contains(&def) {
            // 节点未映射到任何生成函数 → 双向映射断裂
            return false;
        }
    }
    true
}

/// 循环护栏（兼容性：消费 `ctx.registry.loops`，弥补既有 `mox_optimize`
/// 未读取 LoopGuard 的缺口——无界循环此前无人拦截）。
///
/// 规则（护栏 G-B/G-E 在循环维度上的体现）：
///   - 图中存在 LoopStart 节点但 `ctx.registry.loops` 未登记对应 LoopGuard
///     → 默认视为无界，必须否决（保守优先）。
///   - 已登记 LoopGuard 为 `LoopPolicy::Unbounded` 且无人/安全专家审批
///     → 否决（无界循环需安全专家严格审批）。
///   - 已登记 `Bounded { max_iter }` → 放行。
///
/// 返回 `None` 表示循环安全；`Some(reason)` 表示必须否决。
fn check_loops(graph: &FlowGraph, ctx: &GovernContext) -> Option<String> {
    // 收集图中所有循环入口节点
    let loop_starts: Vec<&FlowNode> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::LoopStart)
        .collect();
    if loop_starts.is_empty() {
        return None;
    }
    for ls in loop_starts {
        // 在 registry.loops 中查找对应护栏
        let guard = ctx.registry.loops.iter().find(|g| g.node == ls.id);
        match guard {
            None => {
                return Some(format!(
                    "循环节点 {} 未登记 LoopGuard（默认视为无界，禁止出码）",
                    ls.id
                ));
            }
            Some(g) => match &g.policy {
                crate::context::LoopPolicy::Unbounded => {
                    // 无界循环：要求专门的安全专家审批角色（不能仅凭普通 approver）
                    let approved = ctx.principal.roles.iter().any(|r| r == "safety_approver")
                        && ctx.tenant.regulated;
                    if !approved {
                        return Some(format!(
                            "循环节点 {} 为无界循环，缺少安全专家(safety_approver)审批",
                            ls.id
                        ));
                    }
                }
                crate::context::LoopPolicy::Bounded { .. } => {
                    // 有界循环放行
                }
                crate::context::LoopPolicy::HumanInLoop => {
                    // 人在环：要求 RunFlow 能力（人工介入）
                    let can_run = ctx
                        .principal
                        .roles
                        .iter()
                        .any(|r| r == "operator" || r == "approver");
                    if !can_run {
                        return Some(format!("循环节点 {} 为人在环，缺少操作员授权", ls.id));
                    }
                }
            },
        }
    }
    None
}

/// 生产环境写保护说明：
/// 越权写生产/敏感库现由 permission 专家 `push_veto` 正交触发 algo.vetoed，
/// 不再需要编排层补丁。原 `_deprecated_check_protected_writes` 空函数已删除。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{Principal, ResourceQuota, Tenant};
    use mox_ai_flow_svc::codegen::{CodeBundle, GeneratedFile};
    use mox_ai_flow_svc::model::{Access, FlowEdge, ToolKind};

    fn sample_graph() -> FlowGraph {
        let mut g = FlowGraph::new("p", "示例编程流程");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(
            FlowNode::task("read", "读数据", ToolKind::Database, 300)
                .with_access(Access::read("db:src")),
        );
        g.add_node(
            FlowNode::task("calc", "计算", ToolKind::Compute, 200)
                .with_access(Access::read("var:x"))
                .with_access(Access::write("var:y")),
        );
        g.add_node(
            FlowNode::task("out", "写结果", ToolKind::File, 100)
                .with_access(Access::write("file:res")),
        );
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "read"));
        g.add_edge(FlowEdge::seq("read", "calc"));
        g.add_edge(FlowEdge::seq("calc", "out"));
        g.add_edge(FlowEdge::seq("out", "e"));
        g
    }

    fn sample_ctx() -> GovernContext {
        let tenant = Tenant::new("t1", "ns1").with_pool("browser", 2);
        let principal = Principal::new("dev").with_roles(vec!["editor".into(), "approver".into()]);
        let mut ctx = GovernContext::new(tenant, principal);
        // 放宽配额，避免 SLA/成本预算误杀正常示例（真实系统按租户配置）
        ctx.quota = ResourceQuota {
            max_parallel: 8,
            max_cost_budget: 100.0,
            sla_ms: 5_000,
        };
        ctx
    }

    #[test]
    fn g_a_guard_blocks_ai_draft() {
        // 护栏 G-A: AI 草稿未确认 → 止于 Normalized，不出码
        let g = sample_graph();
        let rep = programming_pipeline(
            "帮我写个脚本",
            vec!["尽量快".into()], // 模糊约束，且未确认
            false,
            &g,
            &sample_ctx(),
        );
        assert!(!rep.safe_to_emit);
        assert_eq!(rep.checkpoint, Checkpoint::Normalized);
        assert!(rep.governance.is_none());
        assert!(rep.code.is_none());
    }

    #[test]
    fn node_level_roundtrip_catches_missing_def() {
        // 构造含 SubFlow + Task 的图
        let mut g = FlowGraph::new("rt", "双向映射校验");
        g.add_node(FlowNode::task("a", "任务A", ToolKind::Compute, 10));
        g.add_node(FlowNode::new("sub", "子流程", NodeKind::SubFlow));

        // 情形1：完整生成代码（含 a 与 sub 的 def）→ 应一致
        let full = CodeBundle {
            files: vec![GeneratedFile {
                path: "generated/tasks.py".into(),
                content: "def a(ctx:)\n    return ctx\ndef sub(ctx:)\n    return ctx\n".into(),
            }],
            rejected: false,
            reject_reasons: vec![],
        };
        assert!(
            verify_code_roundtrip(&g, &full),
            "完整生成应判定双向映射一致"
        );

        // 情形2：sub 的 def 缺失（节点未映射到代码）→ 应不一致（M2 硬红线）
        let missing = CodeBundle {
            files: vec![GeneratedFile {
                path: "generated/tasks.py".into(),
                content: "def a(ctx:)\n    return ctx\n".into(),
            }],
            rejected: false,
            reject_reasons: vec![],
        };
        assert!(
            !verify_code_roundtrip(&g, &missing),
            "缺失子流程 def 应判定双向映射断裂"
        );

        // 情形3：code.rejected 兜底 → 应不一致
        let rejected = CodeBundle {
            files: vec![],
            rejected: true,
            reject_reasons: vec!["阻断冲突".into()],
        };
        assert!(!verify_code_roundtrip(&g, &rejected));
    }

    #[test]
    fn confirmed_requirement_flows_to_emit() {
        let g = sample_graph();
        let rep = programming_pipeline(
            "从 db:src 读数据，计算后写 file:res",
            vec![
                "读 db:src 必须有读权限".into(),
                "写 file:res 必须授权".into(),
            ],
            true, // 已确认
            &g,
            &sample_ctx(),
        );
        assert!(
            rep.safe_to_emit,
            "expected safe_to_emit, got {:?} gate={:?}",
            rep.checkpoint,
            rep.governance.as_ref().map(|x| &x.gate.reason)
        );
        assert_eq!(rep.checkpoint, Checkpoint::Governed);
        assert!(rep.code.is_some());
        assert_eq!(rep.roundtrip_ok, Some(true));
        let gov = rep.governance.expect("governance present");
        assert!(!gov.algo.vetoed, "正常优化不应被否决");
    }

    #[test]
    fn from_llm_extract_is_draft_by_default() {
        let r = from_llm_extract("做点什么", vec!["尽量好".into()]);
        assert_eq!(r.status, DraftStatus::AiDraft);
        assert!(!r.is_confirmed()); // G-A 生效
    }
}

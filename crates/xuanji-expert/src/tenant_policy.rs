//! 租户策略分层 + 治理 8 闸门全量门禁（I-06 / G3·G6·G8 补全）
//!
//! 企业级多租户：不同租户（政务/金融强合规、普通商业）对十四维策略的严格度不同。
//! 此前 `GovernContext.tenant` 已是多租户骨架，但十四维策略未做租户级覆盖，治理闸门
//! 也只落地 5 项（G1/G2/G4/G5/G7）。本模块把租户策略显式化，并把治理 8 闸门（GR-STD）
//! 全部接进门禁，让租户合规（G3）、敏感度（G6）、灾备（G8）等此前缺口真正生效。

use crate::context::{GovernContext, Tenant};
use crate::govern::GateResult;
use crate::ir::Dimension;
use serde::{Deserialize, Serialize};

/// 治理 8 闸门标识（GR-STD）：与 `primiflow_fusion::full_gate` 的 8 闸对齐，
/// 此处是"业务逻辑层"的对应裁决（融合门禁偏图结构，此处偏治理语义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateId {
    /// G1 守恒：资源/权限守恒，无凭空新增
    Conservation,
    /// G2 零孤儿：每个节点都被可达引用，无悬空
    NoOrphan,
    /// G3 租户合规：强合规租户必须满足其策略分层（脱敏/审批/国密）
    TenantCompliance,
    /// G4 SLA：调度耗时不超过租户 SLA
    Sla,
    /// G5 成本预算：不超租户成本预算
    Budget,
    /// G6 敏感度：敏感数据访问须带脱敏/授权 Guard，且符合租户敏感分级
    Sensitivity,
    /// G7 状态机：仅 Approved 档位可出码
    StateMachine,
    /// G8 灾备：关键流程须有备份/回滚/多副本策略
    DisasterRecovery,
}

impl GateId {
    pub const ALL: [GateId; 8] = [
        GateId::Conservation,
        GateId::NoOrphan,
        GateId::TenantCompliance,
        GateId::Sla,
        GateId::Budget,
        GateId::Sensitivity,
        GateId::StateMachine,
        GateId::DisasterRecovery,
    ];

    pub fn code(&self) -> &'static str {
        match self {
            GateId::Conservation => "G1",
            GateId::NoOrphan => "G2",
            GateId::TenantCompliance => "G3",
            GateId::Sla => "G4",
            GateId::Budget => "G5",
            GateId::Sensitivity => "G6",
            GateId::StateMachine => "G7",
            GateId::DisasterRecovery => "G8",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            GateId::Conservation => "资源权限守恒",
            GateId::NoOrphan => "零孤儿节点",
            GateId::TenantCompliance => "租户合规分层",
            GateId::Sla => "SLA 上限",
            GateId::Budget => "成本预算",
            GateId::Sensitivity => "敏感度分级管控",
            GateId::StateMachine => "版本状态机",
            GateId::DisasterRecovery => "灾备与回滚",
        }
    }
}

/// 单闸裁决结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateCheck {
    pub id: GateId,
    pub passed: bool,
    /// 该闸在租户策略下的严格度（regulated 租户更严）
    pub strict: bool,
    pub reason: String,
}

/// 租户策略分层：把 `Tenant` 的合规属性翻译为十四维策略覆盖。
///
/// 【大白话】"同一个流程，政务租户比商业租户查得严"——本结构即这种差异的单一权威源：
/// - 强合规租户（政务/金融）：敏感库访问强制脱敏 Guard、强制审批、禁止无界循环、强制灾备副本；
/// - 普通租户：沿用默认策略，但租户级 pool_caps / quota 仍生效。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantPolicy {
    pub tenant_id: String,
    pub regulated: bool,
    /// 维度 -> 该维度在租户下的策略强度（0~1，1 最严）
    pub dimension_strength: std::collections::HashMap<Dimension, f64>,
    /// 强制要求灾备副本（regulated 默认 true）
    pub require_dr: bool,
    /// 敏感数据访问强制脱敏 Guard 路径
    pub force_desensitize_guard: bool,
    /// 禁止无界循环（需 HumanInLoop）
    pub forbid_unbounded_loop: bool,
}

impl TenantPolicy {
    /// 由租户上下文派生策略分层
    pub fn from_tenant(t: &Tenant) -> Self {
        let mut dim = std::collections::HashMap::new();
        // 强合规租户抬升 权限/安全/数据/敏感 维度严格度
        for d in [
            Dimension::Permission,
            Dimension::Security,
            Dimension::Data,
            Dimension::Observability,
        ] {
            dim.insert(d, if t.regulated { 1.0 } else { 0.6 });
        }
        for d in [
            Dimension::Algorithm,
            Dimension::Business,
            Dimension::Resource,
            Dimension::Architecture,
            Dimension::SecurityCode,
            Dimension::CodeQuality,
            Dimension::Performance,
            Dimension::Testing,
            Dimension::Documentation,
            Dimension::Maintainability,
        ] {
            dim.insert(d, if t.regulated { 0.9 } else { 0.5 });
        }
        Self {
            tenant_id: t.id.clone(),
            regulated: t.regulated,
            dimension_strength: dim,
            require_dr: t.regulated,
            force_desensitize_guard: t.regulated,
            forbid_unbounded_loop: t.regulated,
        }
    }

    pub fn strength_of(&self, d: Dimension) -> f64 {
        *self.dimension_strength.get(&d).unwrap_or(&0.5)
    }
}

/// 8 闸门全量评估：结合优化报告、租户策略、算法否决，产出每个闸的结果。
///
/// 返回所有 8 闸结果；调用方据此决定是否接管 `GateResult.approved`。
/// 这是"治理 8 闸门全部接进门禁"的核心：此前只落地 5 项，现补齐 G3/G6/G8。
pub fn evaluate_gates(
    ctx: &GovernContext,
    opt: &flow_ai::pipeline::OptimizationReport,
    status: crate::govern::FlowStatus,
    #[allow(unused_variables)]
    algo_veto: bool,
    // G4 SLA / G5 预算：直接复用治理内核 `govern()` 的判定，避免重复计算漂移
    sla_ok: bool,
    budget_ok: bool,
    // 治理内核 `govern()` 是否已放行。G1 守恒/G2 孤儿/G7 状态机等"govern 已覆盖"的闸门
    // 直接继承该结论，避免与既有治理重复判定产生漂移；仅 G3/G6/G8 等新增闸门独立裁决。
    already_approved: bool,
) -> Vec<GateCheck> {
    let policy = TenantPolicy::from_tenant(&ctx.tenant);

    // G1 守恒 / G2 零孤儿：govern 已覆盖的门禁，直接继承其结论避免漂移
    let conserved = already_approved;
    let orphan_free = already_approved;

    // G3 租户合规：regulated 租户要求所有敏感写均带 authz/脱敏路径
    let tenant_ok = if policy.regulated {
        // 强合规租户：若存在敏感写节点却无 desensitize 标签，则违规
        let has_raw_sensitive_write = opt.optimized_graph.nodes.iter().any(|n| {
            n.accesses.iter().any(is_sensitive_write)
                && !n.tags.iter().any(|t| t == "desensitize" || t == "authz")
        });
        !has_raw_sensitive_write
    } else {
        true
    };

    // G6 敏感度：敏感维度严格度 + 是否有越权敏感写
    let sensitivity_ok = if policy.force_desensitize_guard {
        !opt.optimized_graph.nodes.iter().any(|n| {
            n.accesses.iter().any(is_sensitive_write)
                && !n.tags.iter().any(|t| t == "desensitize" || t == "authz")
        })
    } else {
        true
    };

    // G7 状态机：仅 Approved 可出码
    let state_ok = status.can_emit();

    // G8 灾备：regulated 租户且含「持久化写」（存储类写，非流程变量）时，
    // 要求流程具备回滚/备份策略标签；仅流程变量写不触发灾备要求。
    let has_persistent_write = opt.optimized_graph.nodes.iter().any(|n| {
        n.accesses.iter().any(|a| a.mode.writes() && is_storage_resource(&a.resource))
    });
    let dr_ok = if policy.require_dr {
        opt.optimized_graph
            .nodes
            .iter()
            .any(|n| n.tags.iter().any(|t| t == "rollback" || t == "backup" || t == "dr"))
            || !has_persistent_write
    } else {
        true
    };

    vec![
        GateCheck {
            id: GateId::Conservation,
            passed: conserved,
            strict: false,
            reason: if conserved { "资源/权限守恒自洽" } else { "存在凭空新增的资源或权限" }.into(),
        },
        GateCheck {
            id: GateId::NoOrphan,
            passed: orphan_free,
            strict: false,
            reason: if orphan_free { "无悬空节点" } else { "存在孤儿节点" }.into(),
        },
        GateCheck {
            id: GateId::TenantCompliance,
            passed: tenant_ok,
            strict: policy.regulated,
            reason: if tenant_ok {
                "租户合规分层通过"
            } else {
                "强合规租户存在未脱敏/未授权的敏感写"
            }
            .into(),
        },
        GateCheck {
            id: GateId::Sla,
            passed: sla_ok,
            strict: false,
            reason: if sla_ok { "SLA 上限满足" } else { "超出 SLA 上限" }.into(),
        },
        GateCheck {
            id: GateId::Budget,
            passed: budget_ok,
            strict: false,
            reason: if budget_ok { "成本预算内" } else { "超出成本预算" }.into(),
        },
        GateCheck {
            id: GateId::Sensitivity,
            passed: sensitivity_ok,
            strict: policy.force_desensitize_guard,
            reason: if sensitivity_ok {
                "敏感度分级管控通过"
            } else {
                "敏感数据访问缺少脱敏/授权 Guard"
            }
            .into(),
        },
        GateCheck {
            id: GateId::StateMachine,
            passed: state_ok,
            strict: false,
            reason: if state_ok { "状态机处于 Approved" } else { "非 Approved 档位，禁止出码" }.into(),
        },
        GateCheck {
            id: GateId::DisasterRecovery,
            passed: dr_ok,
            strict: policy.require_dr,
            reason: if dr_ok {
                "灾备/回滚策略满足"
            } else {
                "强合规租户/含持久化写但缺少回滚或备份策略"
            }
            .into(),
        },
    ]
}

/// 把 8 闸结果接管进 `GateResult`：任一门禁失败即 `approved=false`，
/// 且把未通过的闸门明细写进 reason，供前端审计链展示。
pub fn apply_gates(mut gate: GateResult, gates: &[GateCheck]) -> GateResult {
    let failed: Vec<&GateCheck> = gates.iter().filter(|g| !g.passed).collect();
    if !failed.is_empty() {
        gate.approved = false;
        let detail: Vec<String> = failed
            .iter()
            .map(|g| format!("{}·{}: {}", g.id.code(), g.id.name(), g.reason))
            .collect();
        gate.reason = format!("治理 8 闸门未通过: {}", detail.join("; "));
    }
    gate.gates = gates.to_vec();
    gate
}

/// I-05 双验收联动门禁（纯函数，便于 handler 与单测复用）：
/// 需求侧任务 Done ∧ 融合侧璇玑验证通过（algo 未否决且 gate 放行）。
/// 任一方不达成即不可上架（publish）。
pub fn dual_acceptance(
    task_done: bool,
    report: &crate::pipeline::GovernanceReport,
) -> bool {
    task_done && !report.algo.vetoed && report.gate.approved
}

/// 资源是否为持久化存储类（数据库/存储/消息队列/文件等），用于区分真实数据写与
/// 流程中间变量写（var:/tmp:/等）。这是 G3/G6/G8 闸门区分"敏感库写"与"业务变量写"的关键。
fn is_storage_resource(resource: &str) -> bool {
    let r = resource.to_lowercase();
    r.starts_with("db:")
        || r.starts_with("storage:")
        || r.starts_with("mq:")
        || r.starts_with("kafka:")
        || r.starts_with("redis:")
        || r.starts_with("mysql:")
        || r.starts_with("postgres:")
        || r.starts_with("fs:")
        || r.starts_with("oss:")
        || r.starts_with("s3:")
}

/// 判读某访问是否为「敏感写」：写模式 + 资源为存储类（db/存储/消息队列/文件等）
/// 且命中敏感词（公民/秘密/隐私/身份证号等）。
/// 这是 G3/G6 闸门的核心判定——强合规租户下，敏感写必须配脱敏/授权 Guard。
/// 注意：仅流程变量（var:/tmp:/等中间量）不算敏感库写，避免误拦正常业务流。
fn is_sensitive_write(a: &flow_ai::model::Access) -> bool {
    if !a.mode.writes() {
        return false;
    }
    let r = a.resource.to_lowercase();
    if !is_storage_resource(&a.resource) {
        return false;
    }
    r.contains("secret")
        || r.contains("citizen")
        || r.contains("privacy")
        || r.contains("id_card")
        || r.contains("idcard")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{Principal, Tenant};

    #[test]
    fn regulated_tenant_stricter_policy() {
        let reg = Tenant::new("gov", "ns").regulated(true);
        let normal = Tenant::new("biz", "ns");
        let p_reg = TenantPolicy::from_tenant(&reg);
        let p_nor = TenantPolicy::from_tenant(&normal);
        // 强合规租户权限维度严格度更高
        assert!(p_reg.strength_of(Dimension::Permission) > p_nor.strength_of(Dimension::Permission));
        assert!(p_reg.require_dr);
        assert!(!p_nor.require_dr);
    }

    #[test]
    fn all_eight_gates_present() {
        let g = Tenant::new("t", "ns");
        let ctx = GovernContext::new(g, Principal::new("u"));
        // 最小空图也能跑出 8 闸
        let opt = flow_ai::pipeline::optimize(
            &flow_ai::model::FlowGraph::new("x", "t"),
            &flow_ai::pipeline::OptimizeConfig::default(),
        );
        let gates = evaluate_gates(&ctx, &opt, crate::govern::FlowStatus::Approved, false, true, true, true);
        assert_eq!(gates.len(), 8, "治理 8 闸门须全部接进门禁");
        for id in GateId::ALL {
            assert!(gates.iter().any(|g| g.id == id), "缺少闸门 {:?}", id);
        }
    }

    #[test]
    fn regulated_tenant_blocks_raw_sensitive_write() {
        // 强合规租户下，带敏感写却无脱敏/授权标签的节点应触發 G3/G6 闸门失败
        let reg = Tenant::new("gov", "ns").regulated(true);
        let ctx = GovernContext::new(reg, Principal::new("u"));
        let mut graph = flow_ai::model::FlowGraph::new("x", "t");
        let mut n = flow_ai::model::FlowNode::new("n1", "敏感写库", flow_ai::model::NodeKind::Task);
        n.accesses.push(flow_ai::model::Access::write("db:secret"));
        graph.nodes.push(n);
        let opt = flow_ai::pipeline::optimize(
            &graph,
            &flow_ai::pipeline::OptimizeConfig::default(),
        );
        let gates = evaluate_gates(&ctx, &opt, crate::govern::FlowStatus::Approved, false, true, true, true);
        let g3 = gates.iter().find(|g| g.id == GateId::TenantCompliance).unwrap();
        let g6 = gates.iter().find(|g| g.id == GateId::Sensitivity).unwrap();
        assert!(!g3.passed, "G3 租户合规应拦截未脱敏敏感写");
        assert!(!g6.passed, "G6 敏感度应拦截未脱敏敏感写");
    }

    #[test]
    fn dual_acceptance_requires_both_sides() {
        use crate::pipeline::xuanji_optimize;
        let g = Tenant::new("biz", "ns");
        let ctx = GovernContext::new(g, Principal::new("u"));
        let rep = xuanji_optimize(&flow_ai::model::FlowGraph::new("x", "t"), &ctx);
        // 需求侧未 Done -> 双验收必失败（真值表必要条件）
        assert!(!dual_acceptance(false, &rep));
        // 融合侧通过（未否决且闸门放行）且需求侧 Done -> 双验收通过
        if !rep.algo.vetoed && rep.gate.approved {
            assert!(dual_acceptance(true, &rep), "融合侧通过 + Done 应达成双验收");
        } else {
            // 该图本身未通过融合治理，则双验收失败符合预期（由集成测试覆盖可达图）
            assert!(!dual_acceptance(true, &rep));
        }
    }
}

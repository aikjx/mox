// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 租户策略分层 + 治理 8 闸门（骨架 · TODO：后续迭代补全完整实现）
//!
//! P2 架构解耦 · 阶段 4：
//! 当前提供 GateId / GateCheck / TenantPolicy 的基础结构，
//! 完整的 8 闸门评估逻辑待后续迭代迁移。

use crate::context::Tenant;
use mox_ai_expert_proto::Dimension;
use serde::{Deserialize, Serialize};

/// 治理 8 闸门标识（GR-STD）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateId {
    /// G1 守恒：资源/权限守恒，无凭空新增
    Conservation,
    /// G2 零孤儿：每个节点都被可达引用，无悬空
    NoOrphan,
    /// G3 租户合规：强合规租户必须满足其策略分层
    TenantCompliance,
    /// G4 SLA：调度耗时不超过租户 SLA
    Sla,
    /// G5 成本预算：不超租户成本预算
    Budget,
    /// G6 敏感度：敏感数据访问须带脱敏/授权 Guard
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
    /// 该闸在租户策略下的严格度
    pub strict: bool,
    pub reason: String,
}

/// 租户策略分层
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

/// 8 闸门评估（骨架实现：仅返回结构，完整逻辑待后续迁移）
///
/// TODO(P2 阶段 4 后续迭代)：迁移完整的 8 闸门评估逻辑
pub fn evaluate_gates(
    ctx: &crate::context::GovernContext,
    opt: &mox_ai_flow_core::pipeline::OptimizationReport,
    status: crate::govern::FlowStatus,
    _algo_veto: bool,
    sla_ok: bool,
    budget_ok: bool,
    already_approved: bool,
) -> Vec<GateCheck> {
    let policy = TenantPolicy::from_tenant(&ctx.tenant);

    vec![
        GateCheck {
            id: GateId::Conservation,
            passed: already_approved,
            strict: false,
            reason: if already_approved {
                "资源/权限守恒自洽"
            } else {
                "存在凭空新增的资源或权限"
            }
            .into(),
        },
        GateCheck {
            id: GateId::NoOrphan,
            passed: already_approved,
            strict: false,
            reason: if already_approved {
                "无悬空节点"
            } else {
                "存在孤儿节点"
            }
            .into(),
        },
        GateCheck {
            id: GateId::TenantCompliance,
            passed: true, // 骨架：暂默认通过
            strict: policy.regulated,
            reason: "骨架实现：租户合规待迁移".into(),
        },
        GateCheck {
            id: GateId::Sla,
            passed: sla_ok,
            strict: false,
            reason: if sla_ok {
                "SLA 上限满足"
            } else {
                "超出 SLA 上限"
            }
            .into(),
        },
        GateCheck {
            id: GateId::Budget,
            passed: budget_ok,
            strict: false,
            reason: if budget_ok {
                "成本预算内"
            } else {
                "超出成本预算"
            }
            .into(),
        },
        GateCheck {
            id: GateId::Sensitivity,
            passed: true, // 骨架：暂默认通过
            strict: policy.force_desensitize_guard,
            reason: "骨架实现：敏感度分级管控待迁移".into(),
        },
        GateCheck {
            id: GateId::StateMachine,
            passed: status.can_emit(),
            strict: false,
            reason: if status.can_emit() {
                "状态机处于 Approved"
            } else {
                "非 Approved 档位，禁止出码"
            }
            .into(),
        },
        GateCheck {
            id: GateId::DisasterRecovery,
            passed: true, // 骨架：暂默认通过
            strict: policy.require_dr,
            reason: "骨架实现：灾备与回滚待迁移".into(),
        },
    ]
}

/// 把 8 闸结果接管进 GateResult
pub fn apply_gates(
    mut gate: crate::govern::GateResult,
    gates: &[GateCheck],
) -> crate::govern::GateResult {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{GovernContext, Principal, Tenant};

    #[test]
    fn regulated_tenant_stricter_policy() {
        let reg = Tenant::new("gov", "ns").regulated(true);
        let normal = Tenant::new("biz", "ns");
        let p_reg = TenantPolicy::from_tenant(&reg);
        let p_nor = TenantPolicy::from_tenant(&normal);
        assert!(
            p_reg.strength_of(Dimension::Permission)
                > p_nor.strength_of(Dimension::Permission)
        );
        assert!(p_reg.require_dr);
        assert!(!p_nor.require_dr);
    }

    #[test]
    fn all_eight_gates_present() {
        let g = Tenant::new("t", "ns");
        let ctx = GovernContext::new(g, Principal::new("u"));
        let opt = mox_ai_flow_core::pipeline::optimize(
            &mox_ai_flow_core::model::FlowGraph::new("x", "t"),
            &mox_ai_flow_core::pipeline::OptimizeConfig::default(),
        );
        let gates = evaluate_gates(
            &ctx,
            &opt,
            crate::govern::FlowStatus::Approved,
            false,
            true,
            true,
            true,
        );
        assert_eq!(gates.len(), 8, "治理 8 闸门须全部接进门禁");
        for id in GateId::ALL {
            assert!(gates.iter().any(|g| g.id == id), "缺少闸门 {:?}", id);
        }
    }
}

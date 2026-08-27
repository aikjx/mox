// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 权限专家：RBAC、越权、合规脱敏（政务等保）

use crate::context::ExpertContext;
use crate::expert::{Constraint, Expert, ExpertOpinion};
use crate::ir::Dimension;
use crate::sensitivity::{is_production_or_sensitive_write, is_sensitive_leak};
use mox_ai_flow_svc::model::{AccessMode, NodeKind, ToolKind};

pub struct PermissionExpert;

/// 节点 tag 形如 `desensitize:<resource>` 时，判断该资源是否已脱敏
fn is_desensitized_by_tag(tag: &str) -> bool {
    if let Some(res) = tag.strip_prefix("desensitize:") {
        crate::sensitivity::is_desensitized(res)
    } else {
        false
    }
}

impl Expert for PermissionExpert {
    fn id(&self) -> String {
        "permission".into()
    }
    fn dimension(&self) -> Dimension {
        Dimension::Permission
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        if !ctx.can(crate::context::Capability::EditFlow) {
            return ExpertOpinion::skipped(
                "permission",
                Dimension::Permission,
                "无 edit-flow 权限",
            );
        }
        let mut o = ExpertOpinion::empty("permission", Dimension::Permission);
        let g = ctx.flow;

        // 强合规策略：公民敏感字段（未脱敏）出库前必须脱敏
        for n in &g.nodes {
            let touches_sensitive = n.accesses.iter().any(|a| is_sensitive_leak(&a.resource));
            if !touches_sensitive {
                continue;
            }
            // 该节点或其后继是否有脱敏 Guard？
            let has_guard = n.tags.iter().any(|t| t == "desensitize" || t == "authz")
                || n.tags
                    .iter()
                    .any(|t| t.starts_with("desensitize") && is_desensitized_by_tag(t));
            let has_guard = has_guard
                || g.edges.iter().any(|e| {
                    e.from == n.id
                        && g.node(&e.to)
                            .map(|s| s.kind == NodeKind::Guard)
                            .unwrap_or(false)
                });
            if !has_guard {
                o.constraints.push(Constraint::MustGuard(
                    n.id.clone(),
                    vec!["desensitize".into()],
                ));
                o.push_risk(
                    mox_ai_flow_svc::model::Severity::Blocking,
                    vec![n.id.clone()],
                    format!(
                        "节点「{}」触碰敏感数据但缺少脱敏/鉴权前置拦截（政务等保）",
                        n.name
                    ),
                    Some("在其前驱插入 desensitize Guard".into()),
                );
            }
        }

        // 外部写操作（入库/外发）需鉴权 Guard
        for n in &g.nodes {
            let writes_external = matches!(n.tool, Some(ToolKind::Database) | Some(ToolKind::Http))
                && n.accesses.iter().any(|a| a.mode == AccessMode::Write);
            if !writes_external || n.tags.iter().any(|t| t == "authz") {
                continue;
            }
            // 区分：生产库/敏感数据写 → 否决级（数据不可回退，须安全审批）；普通外部写 → 建议级
            let writes_prod = n.accesses.iter().any(|a| {
                a.mode == AccessMode::Write && is_production_or_sensitive_write(&a.resource)
            });
            if writes_prod {
                o.push_veto(
                    vec![n.id.clone()],
                    format!(
                        "节点「{}」写生产/敏感数据但缺少 authz 鉴权且无安全审批（数据不可回退）",
                        n.name
                    ),
                    Some("补充 authz Guard 并由 safety_approver 审批".into()),
                );
            } else {
                o.constraints
                    .push(Constraint::MustGuard(n.id.clone(), vec!["authz".into()]));
                o.push_risk(
                    mox_ai_flow_svc::model::Severity::Warning,
                    vec![n.id.clone()],
                    format!("节点「{}」执行外部写操作但未见鉴权环节", n.name),
                    Some("补充鉴权 Guard".into()),
                );
            }
        }

        // 绑定合规策略
        for p in ctx.policies_of(Dimension::Permission) {
            o.constraints.push(Constraint::Compliance(p.id.clone()));
        }
        o
    }
}

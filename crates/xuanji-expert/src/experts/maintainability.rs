//! 可维护性专家（开发维度）：审查可维护性指标、技术债务
//!
//! 分析基于 `CodeUnit` 的**预分析真字段**（耦合度 / 复杂度 / 重复率），
//! 不再用字符串特征猜测可维护性。

use crate::expert::{Expert, ExpertOpinion, Risk, Suggestion};
use crate::ir::{Dimension, ExpertId};
use crate::context::ExpertContext;
use flow_ai::model::Severity;

/// 可维护性专家：审查代码可维护性
pub struct MaintainabilityExpert;

impl Expert for MaintainabilityExpert {
    fn id(&self) -> ExpertId {
        "maintainability".to_string()
    }

    fn dimension(&self) -> Dimension {
        Dimension::Maintainability
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let mut risks = Vec::new();
        let mut suggestions = Vec::new();
        let mut score = 1.0;

        if let Some(code_ir) = &ctx.code_ir {
            for unit in &code_ir.units {
                // 1. 耦合度过高（预分析字段）
                if unit.coupling > 0.7 {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Maintainability,
                        message: format!("模块 {} 耦合度过高（{:.1}）", unit.name, unit.coupling),
                        remediation: Some("解耦模块".to_string()),
                        veto: false,
                    });
                    score *= 0.85;
                }

                // 2. 复杂度过高（预分析字段）
                if unit.cyclomatic_complexity > 15 {
                    suggestions.push(Suggestion::Split);
                    score *= 0.85;
                }

                // 3. 重复率过高（预分析字段）
                if unit.duplication_score > 0.1 {
                    risks.push(Risk {
                        severity: Severity::Info,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Maintainability,
                        message: format!(
                            "模块 {} 重复率过高（{:.1}%）",
                            unit.name, unit.duplication_score * 100.0
                        ),
                        remediation: Some("消除重复代码".to_string()),
                        veto: false,
                    });
                    score *= 0.85;
                }
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::Maintainability,
            constraints: Vec::new(),
            risks,
            score,
            metrics: Default::default(),
            suggestions,
            skipped: false,
            skip_reason: None,
        }
    }
}

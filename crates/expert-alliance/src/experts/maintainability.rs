//! 可维护性专家（开发维度）：审查代码可维护性、技术债务、重构建议

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
        let constraints = Vec::new();
        let mut risks = Vec::new();
        let mut suggestions = Vec::new();
        let mut score = 1.0;

        // 分析代码IR
        if let Some(code_ir) = &ctx.code_ir {
            for unit in &code_ir.units {
                // 1. 检查魔法数字
                if contains_magic_numbers(&unit.source_code) {
                    suggestions.push(Suggestion::Merge);
                    score *= 0.9;
                }

                // 2. 检查TODO/FIXME
                if contains_tech_debt(&unit.source_code) {
                    risks.push(Risk {
                        severity: Severity::Info,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Maintainability,
                        message: format!(
                            "模块 {} 存在技术债务标记（TODO/FIXME）",
                            unit.name
                        ),
                        remediation: Some("清理技术债务".to_string()),
                        veto: false,
                    });
                    score *= 0.85;
                }

                // 3. 检查代码重复
                if unit.duplication_score > 0.1 {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Maintainability,
                        message: format!(
                            "模块 {} 存在代码重复（{:.1}%）",
                            unit.name, unit.duplication_score * 100.0
                        ),
                        remediation: Some("提取公共逻辑".to_string()),
                        veto: false,
                    });
                    score *= 0.75;
                }

                // 4. 检查过时依赖
                if unit.has_outdated_deps {
                    suggestions.push(Suggestion::Offload(flow_ai::schedule::ModelTier::Heavy));
                    score *= 0.85;
                }
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::Maintainability,
            constraints,
            risks,
            score,
            metrics: Default::default(),
            suggestions,
            skipped: false,
            skip_reason: None,
        }
    }
}

/// 检查魔法数字
fn contains_magic_numbers(code: &str) -> bool {
    code.matches(char::is_numeric).count() > 10 && !code.contains("const")
}

/// 检查技术债务标记
fn contains_tech_debt(code: &str) -> bool {
    code.contains("TODO") || code.contains("FIXME")
        || code.contains("HACK") || code.contains("XXX")
}

//! 代码质量专家（开发维度）：审查代码复杂度、可读性、最佳实践

use crate::expert::{Expert, ExpertOpinion, Risk, Suggestion};
use crate::ir::{Dimension, ExpertId};
use crate::context::ExpertContext;
use flow_ai::model::Severity;

/// 代码质量专家：审查代码质量指标
pub struct CodeQualityExpert;

impl Expert for CodeQualityExpert {
    fn id(&self) -> ExpertId {
        "code_quality".to_string()
    }

    fn dimension(&self) -> Dimension {
        Dimension::CodeQuality
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let constraints = Vec::new();
        let mut risks = Vec::new();
        let mut suggestions = Vec::new();
        let mut score = 1.0;

        // 分析代码IR
        if let Some(code_ir) = &ctx.code_ir {
            for unit in &code_ir.units {
                // 1. 检查圈复杂度
                if unit.cyclomatic_complexity > 15 {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::CodeQuality,
                        message: format!(
                            "函数 {} 圈复杂度过高（{}）",
                            unit.name, unit.cyclomatic_complexity
                        ),
                        remediation: Some("拆分复杂函数".to_string()),
                        veto: false,
                    });
                    score *= 0.8;
                }

                // 2. 检查函数长度
                if unit.lines_of_code > 100 {
                    suggestions.push(Suggestion::Split);
                    score *= 0.9;
                }

                // 3. 检查命名规范
                if !follows_naming_convention(&unit.name) {
                    suggestions.push(Suggestion::Merge); // 用Merge表示需要重构命名
                    score *= 0.95;
                }

                // 4. 检查注释覆盖率
                if unit.comment_lines == 0 && unit.lines_of_code > 20 {
                    risks.push(Risk {
                        severity: Severity::Info,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::CodeQuality,
                        message: format!("模块 {} 缺少注释", unit.name),
                        remediation: Some("添加文档注释".to_string()),
                        veto: false,
                    });
                    score *= 0.95;
                }
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::CodeQuality,
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

/// 检查命名规范
fn follows_naming_convention(name: &str) -> bool {
    if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        !name.contains('_')
    } else {
        name.chars().all(|c| c.is_lowercase() || c.is_ascii_digit() || c == '_')
    }
}

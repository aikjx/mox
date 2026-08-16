//! 文档专家（开发维度）：审查文档完整性、API文档、示例代码

use crate::expert::{Expert, ExpertOpinion, Risk, Suggestion};
use crate::ir::{Dimension, ExpertId};
use crate::context::ExpertContext;
use flow_ai::model::Severity;

/// 文档专家：审查文档质量
pub struct DocumentationExpert;

impl Expert for DocumentationExpert {
    fn id(&self) -> ExpertId {
        "documentation".to_string()
    }

    fn dimension(&self) -> Dimension {
        Dimension::Documentation
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let constraints = Vec::new();
        let mut risks = Vec::new();
        let mut suggestions = Vec::new();
        let mut score = 1.0;

        // 分析代码IR
        if let Some(code_ir) = &ctx.code_ir {
            for unit in &code_ir.units {
                // 1. 检查文档字符串
                if !has_doc_string(&unit.source_code) {
                    if unit.is_public {
                        risks.push(Risk {
                            severity: Severity::Warning,
                            nodes: vec![unit.id.clone()],
                            dimension: Dimension::Documentation,
                            message: format!("公共模块 {} 缺少文档字符串", unit.name),
                            remediation: Some("添加文档注释".to_string()),
                            veto: false,
                        });
                        score *= 0.7;
                    } else {
                        suggestions.push(Suggestion::Merge);
                        score *= 0.9;
                    }
                }

                // 2. 检查示例代码
                if !has_examples(&unit.source_code) && unit.is_public {
                    suggestions.push(Suggestion::Split);
                    score *= 0.85;
                }

                // 3. 检查README
                if !unit.has_readme && unit.is_entry_point {
                    risks.push(Risk {
                        severity: Severity::Info,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Documentation,
                        message: format!("入口模块 {} 缺少README", unit.name),
                        remediation: Some("添加README文档".to_string()),
                        veto: false,
                    });
                    score *= 0.8;
                }
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::Documentation,
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

/// 检查文档字符串
fn has_doc_string(code: &str) -> bool {
    code.contains("///") || code.contains("//!")
}

/// 检查示例代码
fn has_examples(code: &str) -> bool {
    code.contains("# Example") || code.contains("# Examples")
        || code.contains("```")
}

//! 文档专家（开发维度）：审查 API/代码文档完整性
//!
//! 分析基于 `CodeUnit` 的**预分析真字段**（`has_doc` / `comment_lines`），
//! 不再用字符串猜测是否有文档。

use crate::expert::{Expert, ExpertOpinion, Risk, Suggestion};
use crate::ir::{Dimension, ExpertId};
use crate::context::ExpertContext;
use flow_ai::model::Severity;

/// 文档专家：审查文档完整性
pub struct DocumentationExpert;

impl Expert for DocumentationExpert {
    fn id(&self) -> ExpertId {
        "documentation".to_string()
    }

    fn dimension(&self) -> Dimension {
        Dimension::Documentation
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let mut risks = Vec::new();
        let mut suggestions = Vec::new();
        let mut score = 1.0;

        if let Some(code_ir) = &ctx.code_ir {
            for unit in &code_ir.units {
                // 1. 导出/公共模块缺文档（预分析字段）
                if !unit.has_doc && unit.is_public {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Documentation,
                        message: format!("公共模块 {} 缺少文档注释", unit.name),
                        remediation: Some("添加 API/模块文档".to_string()),
                        veto: false,
                    });
                    score *= 0.85;
                }

                // 2. 整体注释不足
                if unit.comment_lines == 0 && unit.lines_of_code > 50 {
                    suggestions.push(Suggestion::Split);
                    score *= 0.9;
                }
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::Documentation,
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

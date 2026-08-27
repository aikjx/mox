// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 代码质量专家（开发维度）：审查代码复杂度、可读性、最佳实践
//!
//! 分析基于 `CodeUnit` 的**预分析真字段**（圈复杂度 / 行数 / 注释行 / 重复率），
//! 不再做脆弱的字符串命名猜测。

use crate::context::ExpertContext;
use crate::expert::{Expert, ExpertOpinion, Risk, Suggestion};
use crate::ir::{Dimension, ExpertId};
use mox_ai_flow_svc::model::Severity;

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
        let mut risks = Vec::new();
        let mut suggestions = Vec::new();
        let mut score = 1.0;

        if let Some(code_ir) = &ctx.code_ir {
            for unit in &code_ir.units {
                // 1. 圈复杂度过高（>15）
                if unit.cyclomatic_complexity > 15 {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::CodeQuality,
                        message: format!(
                            "函数 {} 圈复杂度过高（{}）",
                            unit.name, unit.cyclomatic_complexity
                        ),
                        remediation: Some("拆分复杂函数/提前返回".to_string()),
                        veto: false,
                    });
                    score *= 0.8;
                }

                // 2. 函数/模块过长（>100 行）
                if unit.lines_of_code > 100 {
                    suggestions.push(Suggestion::Split);
                    score *= 0.9;
                }

                // 3. 注释覆盖率（>20 行却零注释）
                let comment_ratio = if unit.lines_of_code > 0 {
                    unit.comment_lines as f64 / unit.lines_of_code as f64
                } else {
                    1.0
                };
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
                } else if comment_ratio < 0.05 && unit.lines_of_code > 100 {
                    score *= 0.97;
                }

                // 4. 代码重复率（预分析字段）
                if unit.duplication_score > 0.1 {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::CodeQuality,
                        message: format!(
                            "模块 {} 存在代码重复（{:.1}%）",
                            unit.name,
                            unit.duplication_score * 100.0
                        ),
                        remediation: Some("提取公共逻辑".to_string()),
                        veto: false,
                    });
                    score *= 0.75;
                }
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::CodeQuality,
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

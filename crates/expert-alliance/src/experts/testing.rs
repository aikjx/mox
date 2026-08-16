//! 测试专家（开发维度）：审查测试覆盖率、测试质量、边界条件

use crate::expert::{Expert, ExpertOpinion, Risk, Suggestion};
use crate::ir::{Dimension, ExpertId};
use crate::context::ExpertContext;
use flow_ai::model::Severity;

/// 测试专家：审查测试完整性
pub struct TestingExpert;

impl Expert for TestingExpert {
    fn id(&self) -> ExpertId {
        "testing".to_string()
    }

    fn dimension(&self) -> Dimension {
        Dimension::Testing
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let constraints = Vec::new();
        let mut risks = Vec::new();
        let mut suggestions = Vec::new();
        let mut score: f64 = 1.0;

        // 分析代码IR
        if let Some(code_ir) = &ctx.code_ir {
            for unit in &code_ir.units {
                // 1. 检查测试覆盖率
                if unit.test_coverage < 0.6 {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Testing,
                        message: format!(
                            "模块 {} 测试覆盖率不足（{:.1}%）",
                            unit.name, unit.test_coverage * 100.0
                        ),
                        remediation: Some("增加单元测试".to_string()),
                        veto: false,
                    });
                    score *= 0.7;
                }

                // 2. 检查边界测试
                if !contains_boundary_tests(&unit.test_cases) {
                    suggestions.push(Suggestion::Retry);
                    score *= 0.85;
                }

                // 3. 检查异常测试
                if !contains_error_tests(&unit.test_cases) {
                    suggestions.push(Suggestion::Debounce);
                    score *= 0.85;
                }

                // 4. 检查集成测试
                if unit.has_integration_tests {
                    score = (score + 0.05).min(1.0);
                }
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::Testing,
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

/// 检查边界测试
fn contains_boundary_tests(test_cases: &[String]) -> bool {
    test_cases.iter().any(|t| {
        t.contains("boundary") || t.contains("edge") || t.contains("limit")
    })
}

/// 检查异常测试
fn contains_error_tests(test_cases: &[String]) -> bool {
    test_cases.iter().any(|t| {
        t.contains("error") || t.contains("fail") || t.contains("exception")
    })
}

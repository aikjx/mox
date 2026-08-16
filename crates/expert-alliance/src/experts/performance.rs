//! 性能专家（开发维度）：审查性能瓶颈、资源使用、算法复杂度

use crate::expert::{Expert, ExpertOpinion, Constraint, Risk, Suggestion};
use crate::ir::{Dimension, ExpertId};
use crate::context::ExpertContext;
use flow_ai::model::Severity;

/// 性能专家：审查代码性能问题
pub struct PerformanceExpert;

impl Expert for PerformanceExpert {
    fn id(&self) -> ExpertId {
        "performance".to_string()
    }

    fn dimension(&self) -> Dimension {
        Dimension::Performance
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let mut constraints = Vec::new();
        let mut risks = Vec::new();
        let mut suggestions = Vec::new();
        let mut score = 1.0;

        // 分析代码IR
        if let Some(code_ir) = &ctx.code_ir {
            for unit in &code_ir.units {
                // 1. 检查嵌套循环（O(n²)风险）
                if contains_nested_loops(&unit.source_code) {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Performance,
                        message: format!(
                            "模块 {} 包含嵌套循环，可能有性能问题",
                            unit.name
                        ),
                        remediation: Some("优化算法或使用缓存".to_string()),
                        veto: false,
                    });
                    score *= 0.7;
                }

                // 2. 检查大对象复制
                if contains_large_clone(&unit.source_code) {
                    suggestions.push(Suggestion::Cache);
                    score *= 0.85;
                }

                // 3. 检查阻塞IO
                if contains_blocking_io(&unit.source_code) {
                    constraints.push(Constraint::RouteModel(
                        unit.id.clone(),
                        flow_ai::schedule::ModelTier::Light,
                    ));
                    score *= 0.8;
                }

                // 4. 检查内存泄漏风险
                if contains_memory_leak_pattern(&unit.source_code) {
                    risks.push(Risk {
                        severity: Severity::Blocking,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Performance,
                        message: format!("模块 {} 可能存在内存泄漏", unit.name),
                        remediation: Some("检查资源释放逻辑".to_string()),
                        veto: false,
                    });
                    score *= 0.5;
                }
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::Performance,
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

/// 检查嵌套循环
fn contains_nested_loops(code: &str) -> bool {
    let loop_count = code.matches("for ").count() + code.matches("while ").count();
    loop_count > 1
}

/// 检查大对象克隆
fn contains_large_clone(code: &str) -> bool {
    code.contains(".clone()") && (code.contains("Vec") || code.contains("HashMap"))
}

/// 检查阻塞IO
fn contains_blocking_io(code: &str) -> bool {
    (code.contains("std::io::Read") || code.contains("std::io::Write"))
        && !code.contains("async")
}

/// 检查内存泄漏模式
fn contains_memory_leak_pattern(code: &str) -> bool {
    code.contains("Box::leak") || code.contains("mem::forget")
}

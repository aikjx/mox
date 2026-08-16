//! 架构专家（开发维度）：审查代码架构、模块边界、依赖关系

use crate::expert::{Expert, ExpertOpinion, Risk, Suggestion};
use crate::ir::{Dimension, CodeUnit, ExpertId};
use crate::context::ExpertContext;
use flow_ai::model::Severity;

/// 架构专家：审查系统架构设计
pub struct ArchitectureExpert;

impl Expert for ArchitectureExpert {
    fn id(&self) -> ExpertId {
        "architecture".to_string()
    }

    fn dimension(&self) -> Dimension {
        Dimension::Architecture
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let constraints = Vec::new();
        let mut risks = Vec::new();
        let mut suggestions = Vec::new();
        let mut score = 1.0;

        // 分析代码IR
        if let Some(code_ir) = &ctx.code_ir {
            // 1. 检查模块循环依赖
            if has_circular_dependency(&code_ir.units) {
                risks.push(Risk {
                    severity: Severity::Blocking,
                    nodes: code_ir.units.iter().map(|u| u.id.clone()).collect(),
                    dimension: Dimension::Architecture,
                    message: "检测到模块循环依赖".to_string(),
                    remediation: Some("重构模块依赖关系，打破循环".to_string()),
                    veto: false,
                });
                score *= 0.6;
            }

            // 2. 检查模块大小
            for unit in &code_ir.units {
                if unit.lines_of_code > 500 {
                    suggestions.push(Suggestion::Split);
                    score *= 0.9;
                }
            }

            // 3. 检查依赖深度
            let max_depth = calculate_max_dependency_depth(&code_ir.units);
            if max_depth > 5 {
                risks.push(Risk {
                    severity: Severity::Warning,
                    nodes: vec![],
                    dimension: Dimension::Architecture,
                    message: format!("依赖深度过深（{}层），增加维护成本", max_depth),
                    remediation: Some("扁平化依赖结构".to_string()),
                    veto: false,
                });
                score *= 0.85;
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::Architecture,
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

/// 检查循环依赖
fn has_circular_dependency(_units: &[CodeUnit]) -> bool {
    // 简化实现：实际应使用拓扑排序检测环
    false
}

/// 计算最大依赖深度
fn calculate_max_dependency_depth(units: &[CodeUnit]) -> usize {
    // 简化实现：实际应使用DFS计算
    units.iter().map(|u| u.dependencies.len()).max().unwrap_or(0)
}

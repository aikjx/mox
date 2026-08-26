//! 测试专家（开发维度）：审查测试覆盖、可测试性
//!
//! 分析基于 `CodeUnit` 的**预分析真字段**（`test_coverage` / `uncovered`），
//! 不再用字符串猜测是否有测试。

use crate::context::ExpertContext;
use crate::expert::{Expert, ExpertOpinion, Risk};
use crate::ir::{Dimension, ExpertId};
use mox_ai_flow_svc::model::Severity;

/// 测试专家：审查代码测试
pub struct TestingExpert;

impl Expert for TestingExpert {
    fn id(&self) -> ExpertId {
        "testing".to_string()
    }

    fn dimension(&self) -> Dimension {
        Dimension::Testing
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let mut risks = Vec::new();
        let suggestions = Vec::new();
        let mut score = 1.0;

        if let Some(code_ir) = &ctx.code_ir {
            for unit in &code_ir.units {
                // 1. 单测覆盖率过低（<60%）
                if unit.test_coverage < 0.6 {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Testing,
                        message: format!(
                            "模块 {} 测试覆盖率过低（{:.1}%）",
                            unit.name,
                            unit.test_coverage * 100.0
                        ),
                        remediation: Some("补充单元测试".to_string()),
                        veto: false,
                    });
                    score *= 0.7;
                }

                // 2. 未覆盖路径（预分析字段）
                if unit.uncovered {
                    risks.push(Risk {
                        severity: Severity::Info,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Testing,
                        message: format!("模块 {} 存在未覆盖的执行路径", unit.name),
                        remediation: Some("针对边界条件补充测试".to_string()),
                        veto: false,
                    });
                    score *= 0.9;
                }

                // 3. 复杂度高但覆盖低（组合风险）
                if unit.cyclomatic_complexity > 10 && unit.test_coverage < 0.5 {
                    score *= 0.8;
                }
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::Testing,
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

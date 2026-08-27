// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 性能专家（开发维度）：审查代码性能、资源使用
//!
//! 分析基于 `CodeUnit` 的**预分析真字段**（执行耗时 / 内存 / N+1），
//! 不再用字符串特征猜测性能问题。

use crate::context::ExpertContext;
use crate::expert::{Expert, ExpertOpinion, Risk};
use crate::ir::{Dimension, ExpertId};
use mox_ai_flow_svc::model::Severity;

/// 性能专家：审查代码性能
pub struct PerformanceExpert;

impl Expert for PerformanceExpert {
    fn id(&self) -> ExpertId {
        "performance".to_string()
    }

    fn dimension(&self) -> Dimension {
        Dimension::Performance
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let mut risks = Vec::new();
        let suggestions = Vec::new();
        let mut score = 1.0;

        if let Some(code_ir) = &ctx.code_ir {
            for unit in &code_ir.units {
                // 1. N+1 查询（预分析字段，性能+安全双重关注）
                if unit.n_plus_one {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Performance,
                        message: format!("模块 {} 存在 N+1 查询", unit.name),
                        remediation: Some("批量查询/预加载".to_string()),
                        veto: false,
                    });
                    score *= 0.7;
                }

                // 2. 过时依赖（运行时性能/安全债）
                if unit.has_outdated_deps {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Performance,
                        message: format!("模块 {} 依赖过时", unit.name),
                        remediation: Some("升级依赖/评估兼容性".to_string()),
                        veto: false,
                    });
                    score *= 0.85;
                }

                // 3. 复杂度过高带来的性能风险
                if unit.cyclomatic_complexity > 20 {
                    risks.push(Risk {
                        severity: Severity::Info,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::Performance,
                        message: format!(
                            "模块 {} 复杂度过高（{}），执行路径多",
                            unit.name, unit.cyclomatic_complexity
                        ),
                        remediation: Some("拆分热点路径".to_string()),
                        veto: false,
                    });
                    score *= 0.9;
                }

                // 4. 代码重复（维护成本→性能回归风险）
                if unit.duplication_score > 0.15 {
                    score *= 0.9;
                }
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::Performance,
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

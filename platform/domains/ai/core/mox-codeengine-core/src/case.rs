// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Learn 阶段：引擎运行案例库（经验沉淀，联盟「Learn」语义）。
//!
//! 每次 `CodeEngine::run` 产出一条 [`CodeCase`]，可导出 JSONL 供后续
//! 匹配复用（如调度器按历史成功率调整权重）。

use crate::engine::EngineResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeCase {
    pub req_id: String,
    pub req_name: String,
    pub steps: usize,
    pub verdict_score: f64,
    pub approved: bool,
    pub vetoed: bool,
    pub gates_passed: bool,
    pub delivered: bool,
    pub speedup: f64,
    pub files: usize,
    pub lines: usize,
    /// 评审轮数（自修复循环收敛速度）
    pub rounds: usize,
    /// 应用的自修复动作数
    pub repairs_applied: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CaseBook {
    pub cases: Vec<CodeCase>,
}

impl CaseBook {
    pub fn record(result: &EngineResult) -> CodeCase {
        CodeCase {
            req_id: result.requirement.id.clone(),
            req_name: result.requirement.name.clone(),
            steps: result.requirement.steps.len(),
            verdict_score: result.verdict.score,
            approved: result.verdict.approved,
            vetoed: result.verdict.vetoed,
            gates_passed: result.gates.passed,
            delivered: result.delivered,
            speedup: result.report.gains.speedup,
            files: result.bundle.as_ref().map(|b| b.files.len()).unwrap_or(0),
            lines: result.bundle.as_ref().map(|b| b.total_lines()).unwrap_or(0),
            rounds: result.rounds,
            repairs_applied: result.repairs.len(),
        }
    }

    pub fn push(&mut self, case: CodeCase) -> &mut Self {
        self.cases.push(case);
        self
    }

    /// 历史交付成功率（学习信号）
    pub fn delivery_rate(&self) -> f64 {
        if self.cases.is_empty() {
            return 0.0;
        }
        self.cases.iter().filter(|c| c.delivered).count() as f64 / self.cases.len() as f64
    }

    pub fn to_jsonl(&self) -> String {
        self.cases
            .iter()
            .map(|c| serde_json::to_string(c).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_delivery_rate() {
        let mut book = CaseBook::default();
        book.push(CodeCase {
            req_id: "a".into(),
            req_name: String::new(),
            steps: 1,
            verdict_score: 0.9,
            approved: true,
            vetoed: false,
            gates_passed: true,
            delivered: true,
            speedup: 2.0,
            files: 5,
            lines: 100,
            rounds: 1,
            repairs_applied: 0,
        });
        book.push(CodeCase {
            req_id: "b".into(),
            req_name: String::new(),
            steps: 1,
            verdict_score: 0.1,
            approved: false,
            vetoed: true,
            gates_passed: false,
            delivered: false,
            speedup: 1.0,
            files: 0,
            lines: 0,
            rounds: 3,
            repairs_applied: 2,
        });
        assert!((book.delivery_rate() - 0.5).abs() < 1e-9);
        assert_eq!(book.to_jsonl().lines().count(), 2);
    }
}

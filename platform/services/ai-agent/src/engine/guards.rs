//! 引擎守卫 - 安全边界与质量关卡检查
//!
//! Guards 在关键阶段（ACT、REFLECT）执行检查，防止引擎失控：
//! - BudgetGuard: 步数/预算限制，防止无限循环
//! - ProgressGuard: 进展停滞检测，识别无效迭代
//! - RiskGuard: 高风险动作检测，保护系统安全

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 守卫检查结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GuardResult {
    /// 检查通过
    Passed,
    /// 检查触发（被守卫拦截），附带原因描述
    Triggered { reason: String },
}

impl GuardResult {
    pub fn is_passed(&self) -> bool {
        matches!(self, GuardResult::Passed)
    }

    pub fn is_triggered(&self) -> bool {
        matches!(self, GuardResult::Triggered { .. })
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            GuardResult::Passed => None,
            GuardResult::Triggered { reason } => Some(reason),
        }
    }
}

/// 守卫 trait：所有检查器的统一接口
pub trait Guard: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self, context: &GuardContext) -> GuardResult;
}

/// 守卫执行所需的上下文信息
#[derive(Debug, Clone, Default)]
pub struct GuardContext {
    /// 当前已执行步数
    pub step_count: u64,
    /// 最大允许步数
    pub max_steps: u64,
    /// 当前预算（Token/金额等）
    pub budget_used: f64,
    /// 最大预算
    pub budget_limit: f64,
    /// 最近 N 步的执行摘要（用于进展检测）
    pub recent_outcomes: Vec<String>,
    /// 允许的最大连续相同结果数
    pub max_stagnant: usize,
    /// 当前动作描述
    pub current_action: Option<String>,
    /// 风险等级阈值
    pub risk_threshold: RiskLevel,
    /// 元数据扩展
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 风险等级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Default)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

// ── BudgetGuard ──────────────────────────────────────────────

/// 预算守卫：限制执行步数与消耗预算
pub struct BudgetGuard {
    pub max_steps: u64,
    pub budget_limit: f64,
}

impl BudgetGuard {
    pub fn new(max_steps: u64, budget_limit: f64) -> Self {
        Self {
            max_steps,
            budget_limit,
        }
    }
}

impl Default for BudgetGuard {
    fn default() -> Self {
        Self {
            max_steps: 100,
            budget_limit: 1000.0,
        }
    }
}

impl Guard for BudgetGuard {
    fn name(&self) -> &str {
        "budget"
    }

    fn check(&self, ctx: &GuardContext) -> GuardResult {
        if ctx.step_count > self.max_steps {
            return GuardResult::Triggered {
                reason: format!("步数 {} 超过上限 {}", ctx.step_count, self.max_steps),
            };
        }
        if ctx.budget_used > self.budget_limit {
            return GuardResult::Triggered {
                reason: format!(
                    "预算 {:.2} 超过上限 {:.2}",
                    ctx.budget_used, self.budget_limit
                ),
            };
        }
        GuardResult::Passed
    }
}

// ── ProgressGuard ────────────────────────────────────────────

/// 进展守卫：检测执行结果是否长期停滞
pub struct ProgressGuard {
    pub max_stagnant: usize,
}

impl ProgressGuard {
    pub fn new(max_stagnant: usize) -> Self {
        Self { max_stagnant }
    }

    fn detect_stagnation(outcomes: &[String], max_stagnant: usize) -> Option<String> {
        if outcomes.len() < max_stagnant {
            return None;
        }
        let start = outcomes.len() - max_stagnant;
        let window: Vec<&String> = outcomes[start..].iter().collect();
        let first = window[0];
        let all_same = window.iter().all(|o| *o == first);
        if all_same {
            Some(format!(
                "最近 {} 次执行结果完全相同，进展停滞",
                max_stagnant
            ))
        } else {
            None
        }
    }
}

impl Default for ProgressGuard {
    fn default() -> Self {
        Self { max_stagnant: 5 }
    }
}

impl Guard for ProgressGuard {
    fn name(&self) -> &str {
        "progress"
    }

    fn check(&self, ctx: &GuardContext) -> GuardResult {
        if let Some(reason) = Self::detect_stagnation(&ctx.recent_outcomes, self.max_stagnant) {
            return GuardResult::Triggered { reason };
        }
        GuardResult::Passed
    }
}

// ── RiskGuard ────────────────────────────────────────────────

/// 风险守卫：检测高风险动作
pub struct RiskGuard {
    pub threshold: RiskLevel,
    pub dangerous_keywords: Vec<String>,
}

impl RiskGuard {
    pub fn new(threshold: RiskLevel) -> Self {
        Self {
            threshold,
            dangerous_keywords: vec![
                "DELETE".to_string(),
                "DROP".to_string(),
                "rm -rf".to_string(),
                "truncate".to_string(),
                "shutdown".to_string(),
                "drop table".to_string(),
                "drop database".to_string(),
                "format".to_string(),
                "dangerous".to_string(),
            ],
        }
    }

    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.dangerous_keywords.extend(keywords);
        self
    }

    fn assess_risk(&self, action: &str) -> Option<(RiskLevel, String)> {
        let lower = action.to_lowercase();
        for kw in &self.dangerous_keywords {
            if lower.contains(&kw.to_lowercase()) {
                let level = match kw.as_str() {
                    "rm -rf" | "shutdown" | "drop database" | "format" => RiskLevel::Critical,
                    "DELETE" | "DROP" | "drop table" | "truncate" => RiskLevel::High,
                    _ => RiskLevel::Medium,
                };
                return Some((level, format!("检测到危险关键词 '{}' 在动作中", kw)));
            }
        }
        None
    }
}

impl Default for RiskGuard {
    fn default() -> Self {
        Self::new(RiskLevel::High)
    }
}

impl Guard for RiskGuard {
    fn name(&self) -> &str {
        "risk"
    }

    fn check(&self, ctx: &GuardContext) -> GuardResult {
        let action = match &ctx.current_action {
            Some(a) => a,
            None => return GuardResult::Passed,
        };

        if let Some((level, reason)) = self.assess_risk(action) {
            if level >= self.threshold {
                return GuardResult::Triggered {
                    reason: format!(
                        "风险等级 {:?} 超过阈值 {:?}，{}",
                        level, self.threshold, reason
                    ),
                };
            }
        }
        GuardResult::Passed
    }
}

/// 复合守卫：依次执行多个守卫
pub struct CompositeGuard {
    pub guards: Vec<Box<dyn Guard>>,
}

impl CompositeGuard {
    pub fn new() -> Self {
        Self { guards: Vec::new() }
    }

    pub fn add(&mut self, guard: Box<dyn Guard>) {
        self.guards.push(guard);
    }

    /// 依次执行所有守卫，返回第一个被触发的结果；全部通过则返回 Passed
    pub fn check_all(&self, ctx: &GuardContext) -> GuardResult {
        for g in &self.guards {
            let result = g.check(ctx);
            if result.is_triggered() {
                tracing::warn!(
                    target: "engine_guards",
                    guard = g.name(),
                    reason = ?result.reason(),
                    "守卫触发"
                );
                return result;
            }
        }
        GuardResult::Passed
    }
}

impl Default for CompositeGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_guard_passes_within_limit() {
        let g = BudgetGuard::new(100, 1000.0);
        let ctx = GuardContext {
            step_count: 50,
            max_steps: 100,
            budget_used: 500.0,
            budget_limit: 1000.0,
            ..Default::default()
        };
        assert!(g.check(&ctx).is_passed());
    }

    #[test]
    fn test_budget_guard_triggers_on_step_limit() {
        let g = BudgetGuard::new(10, 1000.0);
        let ctx = GuardContext {
            step_count: 11,
            max_steps: 100,
            budget_used: 100.0,
            budget_limit: 1000.0,
            ..Default::default()
        };
        let r = g.check(&ctx);
        assert!(r.is_triggered());
        assert!(r.reason().unwrap().contains("步数"));
    }

    #[test]
    fn test_budget_guard_triggers_on_budget_limit() {
        let g = BudgetGuard::new(100, 100.0);
        let ctx = GuardContext {
            step_count: 5,
            max_steps: 100,
            budget_used: 101.0,
            budget_limit: 1000.0,
            ..Default::default()
        };
        let r = g.check(&ctx);
        assert!(r.is_triggered());
        assert!(r.reason().unwrap().contains("预算"));
    }

    #[test]
    fn test_progress_guard_detects_stagnation() {
        let g = ProgressGuard::new(3);
        let ctx = GuardContext {
            recent_outcomes: vec!["same".to_string(), "same".to_string(), "same".to_string()],
            ..Default::default()
        };
        let r = g.check(&ctx);
        assert!(r.is_triggered());
    }

    #[test]
    fn test_progress_guard_passes_with_varied_outcomes() {
        let g = ProgressGuard::new(3);
        let ctx = GuardContext {
            recent_outcomes: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            ..Default::default()
        };
        assert!(g.check(&ctx).is_passed());
    }

    #[test]
    fn test_risk_guard_detects_dangerous_action() {
        let g = RiskGuard::new(RiskLevel::High);
        let ctx = GuardContext {
            current_action: Some("DROP TABLE users".to_string()),
            ..Default::default()
        };
        let r = g.check(&ctx);
        assert!(r.is_triggered());
    }

    #[test]
    fn test_risk_guard_passes_safe_action() {
        let g = RiskGuard::new(RiskLevel::High);
        let ctx = GuardContext {
            current_action: Some("SELECT * FROM users".to_string()),
            ..Default::default()
        };
        assert!(g.check(&ctx).is_passed());
    }

    #[test]
    fn test_risk_guard_no_action_passes() {
        let g = RiskGuard::new(RiskLevel::High);
        let ctx = GuardContext::default();
        assert!(g.check(&ctx).is_passed());
    }

    #[test]
    fn test_composite_combines_all_guards() {
        let mut composite = CompositeGuard::new();
        composite.add(Box::new(BudgetGuard::new(100, 1000.0)));
        composite.add(Box::new(RiskGuard::new(RiskLevel::High)));

        let ctx = GuardContext {
            step_count: 5,
            max_steps: 100,
            budget_used: 10.0,
            budget_limit: 1000.0,
            current_action: Some("normal operation".to_string()),
            ..Default::default()
        };
        assert!(composite.check_all(&ctx).is_passed());

        let ctx_danger = GuardContext {
            step_count: 5,
            max_steps: 100,
            budget_used: 10.0,
            budget_limit: 1000.0,
            current_action: Some("DELETE FROM users".to_string()),
            ..Default::default()
        };
        assert!(composite.check_all(&ctx_danger).is_triggered());
    }
}

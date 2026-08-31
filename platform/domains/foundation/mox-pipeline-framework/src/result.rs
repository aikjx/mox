// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 阶段结果统一抽象
//!
//! 每个阶段的输出都实现 `PhaseResult` trait，
//! 使得管线核心可以统一处理、存储和序列化所有阶段的结果。
//!
//! 同时提供 `PhaseResultExt` 扩展 trait，用于类型安全的 downcast，
//! 以及 `GenericPhaseResult` 通用结果类型（用于简单场景）。

use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::phase::{Phase, PhaseExecution};

// ================== PhaseResult trait ==================

/// 阶段结果：所有阶段输出的统一抽象
///
/// 每个具体阶段的结果类型都应实现此 trait，
/// 以便能被 `PipelineContext` 统一存储和检索。
///
/// # 设计考量
///
/// - 使用 `as_any()` 支持类型安全的 downcast
/// - `payload()` 提供 JSON 序列化，用于 SSE / 审计 / 调试
/// - `success()` 统一表达阶段是否成功
/// - `phase()` 标识结果属于哪个阶段
pub trait PhaseResult: Send + Sync + std::fmt::Debug {
    /// 结果所属的阶段
    fn phase(&self) -> Phase;

    /// 阶段是否成功执行
    fn success(&self) -> bool;

    /// 序列化为 JSON Value（用于 SSE 事件、审计日志、调试输出）
    fn payload(&self) -> serde_json::Value;

    /// 用于类型安全的 downcast
    fn as_any(&self) -> &dyn Any;

    /// 阶段执行元信息（耗时、状态等）
    fn execution(&self) -> &PhaseExecution;
}

// ================== PhaseResultExt ==================

/// `PhaseResult` 的扩展 trait，提供类型安全的 downcast 方法
pub trait PhaseResultExt: PhaseResult {
    /// 尝试 downcast 为具体类型
    fn downcast_ref<T: 'static>(&self) -> Option<&T>
    where
        T: PhaseResult,
    {
        self.as_any().downcast_ref::<T>()
    }
}

impl<T: PhaseResult + ?Sized> PhaseResultExt for T {}

// ================== GenericPhaseResult ==================

/// 通用阶段结果：用于简单场景，避免为每个阶段都定义新类型
///
/// 当阶段只需要返回一个 JSON payload 和成功状态时，可直接使用此类型。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericPhaseResult {
    phase: Phase,
    success: bool,
    payload: serde_json::Value,
    execution: PhaseExecution,
}

impl GenericPhaseResult {
    pub fn new(
        phase: Phase,
        success: bool,
        payload: serde_json::Value,
        execution: PhaseExecution,
    ) -> Self {
        Self {
            phase,
            success,
            payload,
            execution,
        }
    }

    pub fn success(phase: Phase, payload: serde_json::Value, latency_ms: u64) -> Self {
        Self::new(
            phase,
            true,
            payload,
            PhaseExecution::success(phase, latency_ms),
        )
    }

    pub fn failed(phase: Phase, error: impl Into<String>, latency_ms: u64) -> Self {
        let err = error.into();
        Self::new(
            phase,
            false,
            serde_json::json!({ "error": err }),
            PhaseExecution::failed(phase, latency_ms, err),
        )
    }

    pub fn blocked(phase: Phase, reason: impl Into<String>, latency_ms: u64) -> Self {
        let reason = reason.into();
        Self::new(
            phase,
            false,
            serde_json::json!({ "blocked_reason": reason }),
            PhaseExecution::blocked(phase, latency_ms, reason),
        )
    }

    pub fn skipped(phase: Phase) -> Self {
        Self::new(
            phase,
            true, // 跳过也算"成功执行"（因为没有错误）
            serde_json::json!({ "skipped": true }),
            PhaseExecution::skipped(phase),
        )
    }
}

impl PhaseResult for GenericPhaseResult {
    fn phase(&self) -> Phase {
        self.phase
    }

    fn success(&self) -> bool {
        self.success
    }

    fn payload(&self) -> serde_json::Value {
        self.payload.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn execution(&self) -> &PhaseExecution {
        &self.execution
    }
}

// ================== GateResult 统一抽象 ==================

/// 统一闸门结果抽象
///
/// 适用于所有需要质量门禁的场景，支持多种评分模型：
/// - 布尔型（通过/不通过）
/// - 四级制（A/B/C/D）
/// - 连续分数（0..1）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedGateResult {
    pub phase: Phase,
    /// 是否通过（布尔型）
    pub approved: bool,
    /// 综合分数 0..1
    pub score: f64,
    /// 等级标签（如 "A"/"B"/"C"/"D" 或 "pass"/"fail"）
    pub grade: String,
    /// 各维度分数明细
    pub dimension_scores: std::collections::BTreeMap<String, f64>,
    /// 是否被算法否决（最高权限）
    pub algorithm_veto: bool,
    /// 阻断级风险数量
    pub blocking_risks: usize,
    /// 是否重试过
    pub retried: bool,
    /// 改进建议
    pub suggestions: Vec<String>,
    /// 原因/说明
    pub reason: String,
    /// 执行元信息
    pub execution: PhaseExecution,
}

impl UnifiedGateResult {
    pub fn passed(&self) -> bool {
        self.approved && !self.algorithm_veto
    }

    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }

    pub fn with_algorithm_veto(mut self, veto: bool) -> Self {
        self.algorithm_veto = veto;
        self
    }

    /// 创建一个简单的通过结果
    pub fn pass(phase: Phase, score: f64, latency_ms: u64) -> Self {
        Self {
            phase,
            approved: true,
            score,
            grade: "pass".into(),
            dimension_scores: std::collections::BTreeMap::new(),
            algorithm_veto: false,
            blocking_risks: 0,
            retried: false,
            suggestions: Vec::new(),
            reason: String::new(),
            execution: PhaseExecution::success(phase, latency_ms),
        }
    }

    /// 创建一个简单的拒绝结果
    pub fn reject(phase: Phase, score: f64, reason: &str, latency_ms: u64) -> Self {
        Self {
            phase,
            approved: false,
            score,
            grade: "fail".into(),
            dimension_scores: std::collections::BTreeMap::new(),
            algorithm_veto: false,
            blocking_risks: 0,
            retried: false,
            suggestions: Vec::new(),
            reason: reason.into(),
            execution: PhaseExecution::failed(phase, latency_ms, reason),
        }
    }
}

impl PhaseResult for UnifiedGateResult {
    fn phase(&self) -> Phase {
        self.phase
    }

    fn success(&self) -> bool {
        // 闸门"成功执行"不等于"通过"
        // 只要闸门逻辑正常执行了就算 success，结果在 approved 中
        true
    }

    fn payload(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::Value::Null)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn execution(&self) -> &PhaseExecution {
        &self.execution
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::PhaseStatus;

    #[test]
    fn generic_result_success() {
        let r = GenericPhaseResult::success(
            Phase::Analyze,
            serde_json::json!({"key": "value"}),
            100,
        );
        assert_eq!(r.phase(), Phase::Analyze);
        assert!(r.success());
        assert_eq!(r.execution().status, PhaseStatus::Success);
        assert_eq!(r.execution().latency_ms, 100);
        assert_eq!(r.payload()["key"], "value");
    }

    #[test]
    fn generic_result_failed() {
        let r = GenericPhaseResult::failed(Phase::Gate, "timeout", 50);
        assert!(!r.success());
        assert_eq!(r.execution().status, PhaseStatus::Failed);
        assert_eq!(r.payload()["error"], "timeout");
    }

    #[test]
    fn generic_result_blocked() {
        let r = GenericPhaseResult::blocked(Phase::Gate, "quality too low", 50);
        assert!(!r.success());
        assert_eq!(r.execution().status, PhaseStatus::Blocked);
    }

    #[test]
    fn generic_result_skipped() {
        let r = GenericPhaseResult::skipped(Phase::Learn);
        // 跳过算 success（没有错误）
        assert!(r.success());
        assert_eq!(r.execution().status, PhaseStatus::Skipped);
        assert_eq!(r.payload()["skipped"], true);
    }

    #[test]
    fn gate_result_passed() {
        let r = UnifiedGateResult::pass(Phase::Gate, 0.9, 100);
        assert!(r.passed());
        assert!(r.approved);
        assert_eq!(r.score, 0.9);
        // 闸门的 success() 是 true（执行成功，只是结果可能不通过）
        assert!(r.success());
    }

    #[test]
    fn gate_result_veto_blocks_pass() {
        let mut r = UnifiedGateResult::pass(Phase::Gate, 0.9, 100);
        r.algorithm_veto = true;
        assert!(!r.passed());
        assert!(r.approved); // approved 仍为 true，但 veto 覆盖了
    }

    #[test]
    fn gate_result_rejected() {
        let r = UnifiedGateResult::reject(Phase::Gate, 0.3, "score too low", 50);
        assert!(!r.passed());
        assert!(!r.approved);
        assert_eq!(r.reason, "score too low");
    }

    #[test]
    fn gate_result_with_suggestions() {
        let r = UnifiedGateResult::pass(Phase::Gate, 0.8, 100)
            .with_suggestions(vec!["improve clarity".into(), "add examples".into()]);
        assert_eq!(r.suggestions.len(), 2);
    }

    #[test]
    fn phase_result_downcast() {
        let r: Box<dyn PhaseResult> = Box::new(GenericPhaseResult::success(
            Phase::Analyze,
            serde_json::json!({}),
            0,
        ));

        // 测试 downcast
        let concrete = r.downcast_ref::<GenericPhaseResult>();
        assert!(concrete.is_some());
        assert_eq!(concrete.unwrap().phase(), Phase::Analyze);
    }

    #[test]
    fn phase_result_downcast_wrong_type() {
        let r: Box<dyn PhaseResult> = Box::new(GenericPhaseResult::success(
            Phase::Analyze,
            serde_json::json!({}),
            0,
        ));

        // 尝试 downcast 为错误类型
        let concrete = r.downcast_ref::<UnifiedGateResult>();
        assert!(concrete.is_none());
    }

    #[test]
    fn generic_result_serde_roundtrip() {
        let r = GenericPhaseResult::success(
            Phase::Analyze,
            serde_json::json!({"data": 42}),
            123,
        );
        let json = serde_json::to_value(&r).unwrap();
        let parsed: GenericPhaseResult = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.phase(), Phase::Analyze);
        assert!(parsed.success());
        assert_eq!(parsed.payload()["data"], 42);
    }
}

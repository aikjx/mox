// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 阶段结果统一抽象
//!
//! 每个阶段的输出都实现 `PhaseResult` trait，
//! 使得管线核心可以统一处理、存储和序列化所有阶段的结果。
//!
//! 同时提供 `PhaseResultExt` 扩展 trait，用于类型安全的 downcast。

use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::pipeline_core::phase::{Phase, PhaseExecution};

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
    pub fn new(phase: Phase, success: bool, payload: serde_json::Value, execution: PhaseExecution) -> Self {
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
/// 目标：将 `govern::GateResult`（布尔型）和 `alliance::gate::GateResult`（四级制）
/// 统一到同一抽象下，支持多种评分模型。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedGateResult {
    pub phase: Phase,
    /// 是否通过（布尔型，兼容 govern::GateResult.approved）
    pub approved: bool,
    /// 综合分数 0..1（兼容 alliance::GateScore.total）
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

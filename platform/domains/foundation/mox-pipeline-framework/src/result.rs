// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 阶段结果统一抽象
//!
//! 每个阶段的输出都实现 `PhaseResult<P>` trait，
//! 使得管线核心可以统一处理、存储和序列化所有阶段的结果。
//!
//! 同时提供 `PhaseResultExt` 扩展 trait，用于类型安全的 downcast，
//! 以及 `GenericPhaseResult<P>` 通用结果类型（用于简单场景）。
//!
//! # 设计考量
//!
//! - 使用 `as_any()` 支持类型安全的 downcast
//! - `payload()` 提供 JSON 序列化，用于 SSE / 审计 / 调试
//! - `success()` 统一表达阶段是否成功
//! - `phase()` 标识结果属于哪个阶段

use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::phase::{PhaseExecution, PhaseId};

// ================== PhaseResult trait ==================

/// 阶段结果：所有阶段输出的统一抽象
///
/// 每个具体阶段的结果类型都应实现此 trait，
/// 以便能被 `PipelineContext<P>` 统一存储和检索。
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
pub trait PhaseResult<P: PhaseId>: Send + Sync + std::fmt::Debug {
    /// 结果所属的阶段
    fn phase(&self) -> P;

    /// 阶段是否成功执行
    fn success(&self) -> bool;

    /// 序列化为 JSON Value（用于 SSE 事件、审计日志、调试输出）
    fn payload(&self) -> serde_json::Value;

    /// 用于类型安全的 downcast
    fn as_any(&self) -> &dyn Any;

    /// 阶段执行元信息（耗时、状态等）
    fn execution(&self) -> &PhaseExecution<P>;
}

// ================== PhaseResultExt ==================

/// `PhaseResult` 的扩展 trait，提供类型安全的 downcast 方法
pub trait PhaseResultExt<P: PhaseId>: PhaseResult<P> {
    /// 尝试 downcast 为具体类型
    fn downcast_ref<T: 'static>(&self) -> Option<&T>
    where
        T: PhaseResult<P>,
    {
        self.as_any().downcast_ref::<T>()
    }
}

impl<P: PhaseId, T: PhaseResult<P> + ?Sized> PhaseResultExt<P> for T {}

// ================== GenericPhaseResult ==================

/// 通用阶段结果：用于简单场景，避免为每个阶段都定义新类型
///
/// 当阶段只需要返回一个 JSON payload 和成功状态时，可直接使用此类型。
/// 对于复杂的阶段结果（如闸门结果、多维度评分等），建议定义专用类型
/// 并实现 `PhaseResult<P>` trait。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericPhaseResult<P: PhaseId> {
    phase: P,
    success: bool,
    payload: serde_json::Value,
    execution: PhaseExecution<P>,
}

impl<P: PhaseId> GenericPhaseResult<P> {
    pub fn new(
        phase: P,
        success: bool,
        payload: serde_json::Value,
        execution: PhaseExecution<P>,
    ) -> Self {
        Self {
            phase,
            success,
            payload,
            execution,
        }
    }

    pub fn success(phase: P, payload: serde_json::Value, latency_ms: u64) -> Self {
        Self::new(
            phase.clone(),
            true,
            payload,
            PhaseExecution::success(phase, latency_ms),
        )
    }

    pub fn failed(phase: P, error: impl Into<String>, latency_ms: u64) -> Self {
        let err = error.into();
        Self::new(
            phase.clone(),
            false,
            serde_json::json!({ "error": err }),
            PhaseExecution::failed(phase, latency_ms, err),
        )
    }

    pub fn blocked(phase: P, reason: impl Into<String>, latency_ms: u64) -> Self {
        let reason = reason.into();
        Self::new(
            phase.clone(),
            false,
            serde_json::json!({ "blocked_reason": reason }),
            PhaseExecution::blocked(phase, latency_ms, reason),
        )
    }

    pub fn skipped(phase: P) -> Self {
        Self::new(
            phase.clone(),
            true, // 跳过也算"成功执行"（因为没有错误）
            serde_json::json!({ "skipped": true }),
            PhaseExecution::skipped(phase),
        )
    }
}

impl<P: PhaseId> PhaseResult<P> for GenericPhaseResult<P> {
    fn phase(&self) -> P {
        self.phase.clone()
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

    fn execution(&self) -> &PhaseExecution<P> {
        &self.execution
    }
}

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::{NamedPhase, PhaseStatus};

    #[test]
    fn generic_result_success() {
        let phase = NamedPhase::new("analyze");
        let r = GenericPhaseResult::success(phase.clone(), serde_json::json!({"key": "value"}), 100);
        assert_eq!(r.phase().name(), "analyze");
        assert!(r.success());
        assert_eq!(r.execution().status, PhaseStatus::Success);
        assert_eq!(r.execution().latency_ms, 100);
        assert_eq!(r.payload()["key"], "value");
    }

    #[test]
    fn generic_result_failed() {
        let phase = NamedPhase::blocking("gate");
        let r = GenericPhaseResult::failed(phase, "timeout", 50);
        assert!(!r.success());
        assert_eq!(r.execution().status, PhaseStatus::Failed);
        assert_eq!(r.payload()["error"], "timeout");
    }

    #[test]
    fn generic_result_blocked() {
        let phase = NamedPhase::blocking("gate");
        let r = GenericPhaseResult::blocked(phase, "quality too low", 50);
        assert!(!r.success());
        assert_eq!(r.execution().status, PhaseStatus::Blocked);
    }

    #[test]
    fn generic_result_skipped() {
        let phase = NamedPhase::new("learn");
        let r = GenericPhaseResult::skipped(phase);
        // 跳过算 success（没有错误）
        assert!(r.success());
        assert_eq!(r.execution().status, PhaseStatus::Skipped);
        assert_eq!(r.payload()["skipped"], true);
    }

    #[test]
    fn phase_result_downcast() {
        let phase = NamedPhase::new("test");
        let r: Box<dyn PhaseResult<NamedPhase>> = Box::new(GenericPhaseResult::success(
            phase,
            serde_json::json!({}),
            0,
        ));

        // 测试 downcast
        let concrete = r.downcast_ref::<GenericPhaseResult<NamedPhase>>();
        assert!(concrete.is_some());
    }

    #[test]
    fn phase_result_downcast_wrong_type() {
        let phase = NamedPhase::new("test");
        let r: Box<dyn PhaseResult<NamedPhase>> = Box::new(GenericPhaseResult::success(
            phase,
            serde_json::json!({}),
            0,
        ));

        // 尝试 downcast 为错误类型
        // 用一个不同的泛型参数来测试
        #[derive(Debug)]
        struct DummyResult<P: PhaseId> {
            phase: P,
        }
        impl<P: PhaseId> PhaseResult<P> for DummyResult<P> {
            fn phase(&self) -> P {
                self.phase.clone()
            }
            fn success(&self) -> bool {
                true
            }
            fn payload(&self) -> serde_json::Value {
                serde_json::Value::Null
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn execution(&self) -> &PhaseExecution<P> {
                panic!("not implemented")
            }
        }

        let concrete = r.downcast_ref::<DummyResult<NamedPhase>>();
        assert!(concrete.is_none());
    }

    #[test]
    fn generic_result_serde_roundtrip() {
        let phase = NamedPhase::new("analyze");
        let r = GenericPhaseResult::success(
            phase,
            serde_json::json!({"data": 42}),
            123,
        );
        let json = serde_json::to_value(&r).unwrap();
        let parsed: GenericPhaseResult<NamedPhase> = serde_json::from_value(json).unwrap();
        assert!(parsed.success());
        assert_eq!(parsed.payload()["data"], 42);
    }
}

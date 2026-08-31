// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 管线事件（Pipeline Events）
//!
//! 管线执行过程中产生的事件，用于 SSE 推送、进度通知、事件总线等场景。
//!
//! # 事件类型
//!
//! - `PhaseEvent<P>`: 单个阶段的执行事件（成功/失败/阻断/跳过）
//! - `PipelineEvent<P>`: 管线级别的事件（启动/结束/阶段事件）

use crate::phase::PhaseId;

// ================== PhaseEvent ==================

/// 异步管线阶段事件（对应 SSE 每帧输出）
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
#[derive(Debug, Clone)]
pub enum PhaseEvent<P: PhaseId> {
    /// 阶段成功完成
    Success {
        phase: P,
        latency_ms: u64,
        payload: serde_json::Value,
    },
    /// 阶段失败（非阻断）
    Failed {
        phase: P,
        latency_ms: u64,
        payload: serde_json::Value,
    },
    /// 阶段阻断（管线终止）
    Blocked {
        phase: P,
        latency_ms: u64,
        payload: serde_json::Value,
    },
    /// 阶段被跳过
    Skipped { phase: P },
}

impl<P: PhaseId> PhaseEvent<P> {
    /// 获取事件所属的阶段
    pub fn phase(&self) -> &P {
        match self {
            Self::Success { phase, .. }
            | Self::Failed { phase, .. }
            | Self::Blocked { phase, .. }
            | Self::Skipped { phase } => phase,
        }
    }

    /// 事件是否表示成功（成功或跳过都算成功执行）
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. } | Self::Skipped { .. })
    }

    /// 事件是否表示阻断
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked { .. })
    }

    /// 事件是否表示失败
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// 获取阶段耗时（毫秒），跳过阶段返回 0
    pub fn latency_ms(&self) -> u64 {
        match self {
            Self::Success { latency_ms, .. }
            | Self::Failed { latency_ms, .. }
            | Self::Blocked { latency_ms, .. } => *latency_ms,
            Self::Skipped { .. } => 0,
        }
    }

    /// 获取事件 payload（跳过阶段返回 `{"skipped": true}`）
    pub fn payload(&self) -> serde_json::Value {
        match self {
            Self::Success { payload, .. }
            | Self::Failed { payload, .. }
            | Self::Blocked { payload, .. } => payload.clone(),
            Self::Skipped { .. } => serde_json::json!({ "skipped": true }),
        }
    }

    /// 事件名称（用于 SSE event 字段）
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::Success { .. } => "phase_success",
            Self::Failed { .. } => "phase_failed",
            Self::Blocked { .. } => "phase_blocked",
            Self::Skipped { .. } => "phase_skipped",
        }
    }
}

// ================== PipelineEvent ==================

/// 管线级别的事件
///
/// 用于 SSE 流式输出或事件总线，包含管线生命周期的所有事件。
///
/// # 类型参数
///
/// - `P`: 阶段标识类型（实现 `PhaseId`）
#[derive(Debug, Clone)]
pub enum PipelineEvent<P: PhaseId> {
    /// 管线启动
    PipelineStarted {
        /// 管线名称
        pipeline_name: String,
        /// trace id
        trace_id: uuid::Uuid,
    },
    /// 管线结束
    PipelineFinished {
        /// 管线名称
        pipeline_name: String,
        /// trace id
        trace_id: uuid::Uuid,
        /// 总耗时（毫秒）
        total_ms: u64,
        /// 是否成功
        success: bool,
    },
    /// 阶段事件
    Phase(PhaseEvent<P>),
    /// 管线被阻断
    PipelineBlocked {
        /// 管线名称
        pipeline_name: String,
        /// trace id
        trace_id: uuid::Uuid,
        /// 阻断发生的阶段
        phase: P,
        /// 总耗时（毫秒）
        total_ms: u64,
    },
}

impl<P: PhaseId> PipelineEvent<P> {
    /// 事件类型名称（用于 SSE event 字段或日志分类）
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::PipelineStarted { .. } => "pipeline_started",
            Self::PipelineFinished { .. } => "pipeline_finished",
            Self::Phase(pe) => pe.event_name(),
            Self::PipelineBlocked { .. } => "pipeline_blocked",
        }
    }
}

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::NamedPhase;

    #[test]
    fn phase_event_success() {
        let phase = NamedPhase::new("analyze");
        let evt = PhaseEvent::Success {
            phase: phase.clone(),
            latency_ms: 100,
            payload: serde_json::json!({"ok": true}),
        };
        assert_eq!(evt.phase().name(), "analyze");
        assert!(evt.is_success());
        assert!(!evt.is_blocked());
        assert!(!evt.is_failed());
        assert_eq!(evt.latency_ms(), 100);
        assert_eq!(evt.payload()["ok"], true);
        assert_eq!(evt.event_name(), "phase_success");
    }

    #[test]
    fn phase_event_failed() {
        let phase = NamedPhase::new("analyze");
        let evt = PhaseEvent::Failed {
            phase: phase.clone(),
            latency_ms: 50,
            payload: serde_json::json!({"error": "timeout"}),
        };
        assert!(!evt.is_success());
        assert!(evt.is_failed());
        assert!(!evt.is_blocked());
        assert_eq!(evt.event_name(), "phase_failed");
    }

    #[test]
    fn phase_event_blocked() {
        let phase = NamedPhase::blocking("gate");
        let evt = PhaseEvent::Blocked {
            phase: phase.clone(),
            latency_ms: 50,
            payload: serde_json::json!({}),
        };
        assert!(evt.is_blocked());
        assert!(!evt.is_success());
        assert_eq!(evt.event_name(), "phase_blocked");
    }

    #[test]
    fn phase_event_skipped() {
        let phase = NamedPhase::new("learn");
        let evt = PhaseEvent::Skipped {
            phase: phase.clone(),
        };
        assert!(evt.is_success()); // 跳过也算成功
        assert!(!evt.is_blocked());
        assert_eq!(evt.latency_ms(), 0);
        assert_eq!(evt.payload()["skipped"], true);
        assert_eq!(evt.event_name(), "phase_skipped");
    }

    #[test]
    fn pipeline_event_types() {
        use uuid::Uuid;

        let trace_id = Uuid::new_v4();
        let started = PipelineEvent::<NamedPhase>::PipelineStarted {
            pipeline_name: "test_pipeline".into(),
            trace_id,
        };
        assert_eq!(started.event_type(), "pipeline_started");

        let finished = PipelineEvent::<NamedPhase>::PipelineFinished {
            pipeline_name: "test_pipeline".into(),
            trace_id,
            total_ms: 1000,
            success: true,
        };
        assert_eq!(finished.event_type(), "pipeline_finished");

        let phase = NamedPhase::new("analyze");
        let phase_evt = PipelineEvent::Phase(PhaseEvent::Success {
            phase: phase.clone(),
            latency_ms: 100,
            payload: serde_json::json!({}),
        });
        assert_eq!(phase_evt.event_type(), "phase_success");

        let blocked = PipelineEvent::<NamedPhase>::PipelineBlocked {
            pipeline_name: "test_pipeline".into(),
            trace_id,
            phase,
            total_ms: 500,
        };
        assert_eq!(blocked.event_type(), "pipeline_blocked");
    }
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 统一阶段定义与阶段处理器 trait
//!
//! 通用管线阶段枚举，涵盖常见的处理流水线模式。
//! 业务方可以直接使用内置阶段，也可以通过 `Custom` 变体扩展自定义阶段。
//!
//! # 内置阶段分类
//!
//! ## 通用基础阶段
//! - `Normalize` : 归一化/预处理
//! - `Analyze`   : 分析阶段
//! - `Reconcile` : 裁决/合成（多观点归一）
//! - `Gate`      : 质量门禁（评分 + 分级 + 决策）
//! - `Learn`     : 指标学习（可选阶段）
//! - `Done`      : 完成阶段（收尾 + 审计）
//!
//! ## 扩展阶段（按需选用）
//! - `Optimize`  : 优化求解阶段
//! - `Verify`    : 验证阶段（最高权限否决）
//! - `Team`      : 组队/资源分配阶段
//! - `Synthesize`: 合成输出阶段
//!
//! ## 自定义扩展
//! - `Custom(&'static str)` : 运行时动态扩展阶段

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::context::PipelineContext;
use crate::result::PhaseResult;

// ================== Phase 枚举 ==================

/// 管线阶段枚举
///
/// 设计为 `non_exhaustive`，允许未来新增阶段而不破坏匹配。
/// 使用 `Custom(&'static str)` 支持运行时动态扩展阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Phase {
    // ---- 通用基础阶段 ----
    /// 归一化/预处理阶段
    Normalize,

    /// 分析阶段
    Analyze,

    /// 裁决/合成阶段（多观点归一、结果汇总）
    Reconcile,

    /// 质量门禁阶段（评分 + 分级 + 决策）
    Gate,

    /// 指标学习阶段（可选）
    Learn,

    /// 完成阶段（收尾 + 审计汇总）
    Done,

    // ---- 扩展阶段 ----
    /// 优化求解阶段
    Optimize,

    /// 验证阶段（最高权限，不可被治理覆盖）
    Verify,

    /// 组队/资源分配阶段
    Team,

    /// 合成输出阶段（生成最终输出）
    Synthesize,

    // ---- 自定义扩展 ----
    /// 自定义阶段，用于动态扩展
    #[serde(skip)]
    Custom(&'static str),
}

impl Phase {
    /// 阶段的稳定名称（用于日志、审计、SSE 事件名）
    pub fn name(&self) -> &'static str {
        match self {
            Self::Normalize => "normalize",
            Self::Analyze => "analyze",
            Self::Reconcile => "reconcile",
            Self::Gate => "gate",
            Self::Learn => "learn",
            Self::Done => "done",
            Self::Optimize => "optimize",
            Self::Verify => "verify",
            Self::Team => "team",
            Self::Synthesize => "synthesize",
            Self::Custom(name) => name,
        }
    }

    /// 是否为终端阶段（之后不应再有阶段）
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done)
    }

    /// 是否为阻断性阶段（失败则终止管线）
    pub fn is_blocking(&self) -> bool {
        // Gate 阶段默认阻断；可通过选项配置
        matches!(self, Self::Gate)
    }

    /// 阶段序号（用于排序，越小越先执行）
    pub fn order(&self) -> u8 {
        match self {
            Self::Normalize => 10,
            Self::Team => 15,
            Self::Analyze => 20,
            Self::Reconcile => 30,
            Self::Optimize => 40,
            Self::Verify => 50,
            Self::Synthesize => 55,
            Self::Gate => 60,
            Self::Learn => 70,
            Self::Done => 99,
            Self::Custom(_) => 50, // 自定义阶段默认中间位置
        }
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl PartialOrd for Phase {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.order().cmp(&other.order()))
    }
}

// ================== PhaseStatus ==================

/// 阶段执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 执行成功
    Success,
    /// 执行失败（可继续，非阻断）
    Failed,
    /// 被阻断（管线终止）
    Blocked,
    /// 被跳过（条件不满足）
    Skipped,
}

impl PhaseStatus {
    pub fn is_done(&self) -> bool {
        matches!(
            self,
            Self::Success | Self::Failed | Self::Blocked | Self::Skipped
        )
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked)
    }

    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }
}

impl fmt::Display for PhaseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Skipped => "skipped",
        };
        f.write_str(s)
    }
}

// ================== PhaseExecution ==================

/// 阶段执行元信息（耗时、状态等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseExecution {
    pub phase: Phase,
    pub status: PhaseStatus,
    /// 阶段耗时（毫秒）
    pub latency_ms: u64,
    /// 是否降级模式
    #[serde(default)]
    pub degraded: bool,
    /// 降级原因
    #[serde(default)]
    pub degrade_reason: Option<String>,
    /// 错误信息（失败时）
    #[serde(default)]
    pub error: Option<String>,
}

impl PhaseExecution {
    pub fn new(phase: Phase) -> Self {
        Self {
            phase,
            status: PhaseStatus::Pending,
            latency_ms: 0,
            degraded: false,
            degrade_reason: None,
            error: None,
        }
    }

    pub fn success(phase: Phase, latency_ms: u64) -> Self {
        Self {
            phase,
            status: PhaseStatus::Success,
            latency_ms,
            degraded: false,
            degrade_reason: None,
            error: None,
        }
    }

    pub fn failed(phase: Phase, latency_ms: u64, error: impl Into<String>) -> Self {
        Self {
            phase,
            status: PhaseStatus::Failed,
            latency_ms,
            degraded: false,
            degrade_reason: None,
            error: Some(error.into()),
        }
    }

    pub fn blocked(phase: Phase, latency_ms: u64, reason: impl Into<String>) -> Self {
        Self {
            phase,
            status: PhaseStatus::Blocked,
            latency_ms,
            degraded: false,
            degrade_reason: None,
            error: Some(reason.into()),
        }
    }

    pub fn skipped(phase: Phase) -> Self {
        Self {
            phase,
            status: PhaseStatus::Skipped,
            latency_ms: 0,
            degraded: false,
            degrade_reason: None,
            error: None,
        }
    }
}

// ================== PhaseHandler trait ==================

/// 阶段处理器：管线中的一个执行单元
///
/// 每个阶段实现此 trait，负责：
/// 1. 从 PipelineContext 读取输入
/// 2. 执行阶段逻辑
/// 3. 将结果写回 PipelineContext
///
/// # 同步 vs 异步
///
/// 同步管线使用 `execute` 方法；异步管线使用 `execute_async` 方法。
/// 默认实现提供了两者之间的桥接（异步调用同步，或同步阻塞异步）。
///
/// # 错误处理
///
/// 返回 `Result<Box<dyn PhaseResult>, String>`：
/// - `Ok(result)` 表示阶段执行完成，结果写入上下文
/// - `Err(msg)` 表示阶段执行失败，由管线决定是否继续
///
/// 对于阻断性阶段（如 Gate），失败应返回 `PhaseStatus::Blocked` 的结果，
/// 而不是返回 `Err`，因为阻断是预期的业务结果，而非异常。
pub trait PhaseHandler: Send + Sync {
    /// 返回此处理器对应的阶段
    fn phase(&self) -> Phase;

    /// 同步执行阶段逻辑
    ///
    /// 默认实现返回未实现错误，以便纯异步处理器不必实现此方法。
    fn execute(&self, _ctx: &mut PipelineContext) -> Result<Box<dyn PhaseResult>, String> {
        Err(format!(
            "PhaseHandler for '{}' does not support synchronous execution",
            self.phase().name()
        ))
    }

    /// 异步执行阶段逻辑
    ///
    /// 默认实现调用同步版本并包装为 ready future，
    /// 以便纯同步处理器不必实现此方法。
    fn execute_async<'a>(
        &'a self,
        ctx: &'a mut PipelineContext,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Box<dyn PhaseResult>, String>> + Send + 'a>,
    > {
        Box::pin(async move { self.execute(ctx) })
    }

    /// 阶段是否应被跳过
    ///
    /// 默认不跳过。子类可根据上下文条件判断是否跳过（如 Learn 阶段在低级别跳过）。
    fn should_skip(&self, _ctx: &PipelineContext) -> bool {
        false
    }
}

// ── 测试辅助：简单的阶段处理器实现 ──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_names_are_stable() {
        assert_eq!(Phase::Normalize.name(), "normalize");
        assert_eq!(Phase::Analyze.name(), "analyze");
        assert_eq!(Phase::Reconcile.name(), "reconcile");
        assert_eq!(Phase::Gate.name(), "gate");
        assert_eq!(Phase::Learn.name(), "learn");
        assert_eq!(Phase::Done.name(), "done");
        assert_eq!(Phase::Optimize.name(), "optimize");
        assert_eq!(Phase::Verify.name(), "verify");
        assert_eq!(Phase::Team.name(), "team");
        assert_eq!(Phase::Synthesize.name(), "synthesize");
    }

    #[test]
    fn custom_phase_has_custom_name() {
        let p = Phase::Custom("my_custom_phase");
        assert_eq!(p.name(), "my_custom_phase");
    }

    #[test]
    fn done_is_terminal() {
        assert!(Phase::Done.is_terminal());
        assert!(!Phase::Analyze.is_terminal());
        assert!(!Phase::Gate.is_terminal());
    }

    #[test]
    fn gate_is_blocking() {
        assert!(Phase::Gate.is_blocking());
        assert!(!Phase::Analyze.is_blocking());
        assert!(!Phase::Done.is_blocking());
    }

    #[test]
    fn phase_ordering() {
        assert!(Phase::Normalize.order() < Phase::Analyze.order());
        assert!(Phase::Analyze.order() < Phase::Reconcile.order());
        assert!(Phase::Reconcile.order() < Phase::Gate.order());
        assert!(Phase::Gate.order() < Phase::Done.order());
    }

    #[test]
    fn phase_display() {
        assert_eq!(format!("{}", Phase::Analyze), "analyze");
        assert_eq!(format!("{}", Phase::Gate), "gate");
    }

    #[test]
    fn phase_status_display() {
        assert_eq!(format!("{}", PhaseStatus::Success), "success");
        assert_eq!(format!("{}", PhaseStatus::Blocked), "blocked");
        assert_eq!(format!("{}", PhaseStatus::Skipped), "skipped");
    }

    #[test]
    fn phase_status_is_done() {
        assert!(PhaseStatus::Success.is_done());
        assert!(PhaseStatus::Failed.is_done());
        assert!(PhaseStatus::Blocked.is_done());
        assert!(PhaseStatus::Skipped.is_done());
        assert!(!PhaseStatus::Pending.is_done());
        assert!(!PhaseStatus::Running.is_done());
    }

    #[test]
    fn phase_execution_constructors() {
        let exec = PhaseExecution::success(Phase::Analyze, 100);
        assert_eq!(exec.status, PhaseStatus::Success);
        assert_eq!(exec.latency_ms, 100);

        let exec = PhaseExecution::failed(Phase::Gate, 50, "timeout");
        assert_eq!(exec.status, PhaseStatus::Failed);
        assert_eq!(exec.error.as_deref(), Some("timeout"));

        let exec = PhaseExecution::blocked(Phase::Gate, 50, "quality too low");
        assert_eq!(exec.status, PhaseStatus::Blocked);
        assert!(exec.error.is_some());

        let exec = PhaseExecution::skipped(Phase::Learn);
        assert_eq!(exec.status, PhaseStatus::Skipped);
        assert_eq!(exec.latency_ms, 0);
    }

    #[test]
    fn phase_serde_roundtrip() {
        let phase = Phase::Analyze;
        let json = serde_json::to_string(&phase).unwrap();
        assert_eq!(json, "\"analyze\"");
        let parsed: Phase = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, phase);
    }

    // 测试 PhaseHandler trait 的默认实现
    struct TestHandler;

    impl PhaseHandler for TestHandler {
        fn phase(&self) -> Phase {
            Phase::Analyze
        }
    }

    #[test]
    fn phase_handler_default_execute_returns_error() {
        let handler = TestHandler;
        let mut ctx = PipelineContext::new(
            crate::context::PipelineInput::Query {
                query: "test".into(),
                session_id: None,
                context: std::collections::HashMap::new(),
            },
            crate::context::PipelineOptions::default(),
        );
        let result = handler.execute(&mut ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not support synchronous"));
    }
}

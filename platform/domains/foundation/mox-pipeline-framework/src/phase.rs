// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 阶段标识与阶段处理器 trait
//!
//! 管线框架的核心抽象：阶段（Phase）是管线中的一个执行单元。
//! 通过 `PhaseId` trait，业务方可以定义自己的阶段枚举或类型，
//! 框架不绑定任何特定领域的阶段名称。
//!
//! # 设计原则
//!
//! - **泛型阶段标识**：`PhaseId` trait，任何满足约束的类型都可作为阶段标识
//! - **默认实现**：提供 `NamedPhase` 开箱即用的字符串阶段标识
//! - **阶段处理器**：`PhaseHandler<P>` trait，定义阶段的执行逻辑
//! - **同步/异步统一**：核心支持两种执行模式，默认桥接
//!
//! # 快速开始
//!
//! ```ignore
//! // 方式一：使用内置 NamedPhase
//! let phase = NamedPhase::new("analyze");
//!
//! // 方式二：自定义阶段枚举（推荐用于领域特定管线）
//! #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
//! enum MyPhase { Normalize, Analyze, Done }
//!
//! impl PhaseId for MyPhase {
//!     fn name(&self) -> &str { ... }
//!     fn is_terminal(&self) -> bool { ... }
//!     fn is_blocking(&self) -> bool { ... }
//! }
//! ```

use std::fmt;

use crate::context::PipelineContext;
use crate::result::PhaseResult;

// ================== PhaseId trait ==================

/// 阶段标识 trait：管线中阶段的唯一标识
///
/// 业务方可以用枚举、字符串或任何满足约束的类型实现此 trait。
/// 框架通过此 trait 与具体阶段类型解耦。
///
/// # 必需方法
///
/// - `name()`：阶段的稳定名称，用于日志、审计、事件名
///
/// # 可选方法（有默认实现）
///
/// - `is_terminal()`：是否为终端阶段（默认 false）
/// - `is_blocking()`：是否为阻断性阶段（默认 false）
/// - `order()`：阶段序号，用于排序（默认 0）
pub trait PhaseId:
    fmt::Debug + Clone + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static
{
    /// 阶段的稳定名称（用于日志、审计、SSE 事件名）
    fn name(&self) -> &str;

    /// 是否为终端阶段（之后不应再有阶段）
    ///
    /// 默认返回 `false`。
    fn is_terminal(&self) -> bool {
        false
    }

    /// 是否为阻断性阶段（失败则终止管线）
    ///
    /// 默认返回 `false`。
    fn is_blocking(&self) -> bool {
        false
    }

    /// 阶段序号（用于排序，越小越先执行）
    ///
    /// 默认返回 0。
    fn order(&self) -> u32 {
        0
    }
}

// ================== NamedPhase（默认实现） ==================

/// 命名阶段：基于字符串的通用阶段标识
///
/// 适用于快速原型、动态阶段或不需要强类型阶段枚举的场景。
/// 对于领域特定管线，推荐定义自己的枚举并实现 `PhaseId` trait。
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NamedPhase {
    name: String,
    terminal: bool,
    blocking: bool,
    order: u32,
}

impl NamedPhase {
    /// 创建一个普通阶段
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            terminal: false,
            blocking: false,
            order: 0,
        }
    }

    /// 创建一个阻断性阶段（如质量门禁）
    pub fn blocking(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            terminal: false,
            blocking: true,
            order: 0,
        }
    }

    /// 创建一个终端阶段（如完成阶段）
    pub fn terminal(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            terminal: true,
            blocking: false,
            order: u32::MAX,
        }
    }

    /// 设置阶段序号（用于排序）
    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }

    /// 设置是否为阻断性阶段
    pub fn with_blocking(mut self, blocking: bool) -> Self {
        self.blocking = blocking;
        self
    }

    /// 设置是否为终端阶段
    pub fn with_terminal(mut self, terminal: bool) -> Self {
        self.terminal = terminal;
        self
    }
}

impl PhaseId for NamedPhase {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_terminal(&self) -> bool {
        self.terminal
    }

    fn is_blocking(&self) -> bool {
        self.blocking
    }

    fn order(&self) -> u32 {
        self.order
    }
}

impl fmt::Display for NamedPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

// ================== PhaseStatus ==================

/// 阶段执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhaseExecution<P: PhaseId> {
    pub phase: P,
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

impl<P: PhaseId> PhaseExecution<P> {
    pub fn new(phase: P) -> Self {
        Self {
            phase,
            status: PhaseStatus::Pending,
            latency_ms: 0,
            degraded: false,
            degrade_reason: None,
            error: None,
        }
    }

    pub fn success(phase: P, latency_ms: u64) -> Self {
        Self {
            phase,
            status: PhaseStatus::Success,
            latency_ms,
            degraded: false,
            degrade_reason: None,
            error: None,
        }
    }

    pub fn failed(phase: P, latency_ms: u64, error: impl Into<String>) -> Self {
        Self {
            phase,
            status: PhaseStatus::Failed,
            latency_ms,
            degraded: false,
            degrade_reason: None,
            error: Some(error.into()),
        }
    }

    pub fn blocked(phase: P, latency_ms: u64, reason: impl Into<String>) -> Self {
        Self {
            phase,
            status: PhaseStatus::Blocked,
            latency_ms,
            degraded: false,
            degrade_reason: None,
            error: Some(reason.into()),
        }
    }

    pub fn skipped(phase: P) -> Self {
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
/// 1. 从 `PipelineContext<P>` 读取输入
/// 2. 执行阶段逻辑
/// 3. 将结果写回 `PipelineContext<P>`
///
/// # 同步 vs 异步
///
/// 同步管线使用 `execute` 方法；异步管线使用 `execute_async` 方法。
/// 默认实现提供了两者之间的桥接（异步调用同步，或同步阻塞异步）。
///
/// # 错误处理
///
/// 返回 `Result<Box<dyn PhaseResult<P>>, String>`：
/// - `Ok(result)` 表示阶段执行完成，结果写入上下文
/// - `Err(msg)` 表示阶段执行失败，由管线决定是否继续
///
/// 对于阻断性阶段（如质量门禁），失败应返回 `PhaseStatus::Blocked` 的结果，
/// 而不是返回 `Err`，因为阻断是预期的业务结果，而非异常。
pub trait PhaseHandler<P: PhaseId>: Send + Sync {
    /// 返回此处理器对应的阶段
    fn phase(&self) -> P;

    /// 同步执行阶段逻辑
    ///
    /// 默认实现返回未实现错误，以便纯异步处理器不必实现此方法。
    fn execute(&self, _ctx: &mut PipelineContext<P>) -> Result<Box<dyn PhaseResult<P>>, String> {
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
        ctx: &'a mut PipelineContext<P>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Box<dyn PhaseResult<P>>, String>> + Send + 'a>,
    > {
        Box::pin(async move { self.execute(ctx) })
    }

    /// 阶段是否应被跳过
    ///
    /// 默认不跳过。实现者可根据上下文条件判断是否跳过。
    fn should_skip(&self, _ctx: &PipelineContext<P>) -> bool {
        false
    }
}

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // -- NamedPhase 测试 --

    #[test]
    fn named_phase_basic() {
        let p = NamedPhase::new("analyze");
        assert_eq!(p.name(), "analyze");
        assert!(!p.is_terminal());
        assert!(!p.is_blocking());
        assert_eq!(p.order(), 0);
    }

    #[test]
    fn named_phase_blocking() {
        let p = NamedPhase::blocking("gate");
        assert_eq!(p.name(), "gate");
        assert!(p.is_blocking());
        assert!(!p.is_terminal());
    }

    #[test]
    fn named_phase_terminal() {
        let p = NamedPhase::terminal("done");
        assert_eq!(p.name(), "done");
        assert!(p.is_terminal());
        assert_eq!(p.order(), u32::MAX);
    }

    #[test]
    fn named_phase_with_order() {
        let p = NamedPhase::new("step1").with_order(10);
        assert_eq!(p.order(), 10);
    }

    #[test]
    fn named_phase_display() {
        let p = NamedPhase::new("my_phase");
        assert_eq!(format!("{p}"), "my_phase");
    }

    #[test]
    fn named_phase_equality() {
        let a = NamedPhase::new("phase_a");
        let b = NamedPhase::new("phase_a");
        let c = NamedPhase::new("phase_c");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // -- 自定义 PhaseId 测试 --

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum TestPhase {
        Start,
        Process,
        End,
    }

    impl PhaseId for TestPhase {
        fn name(&self) -> &str {
            match self {
                Self::Start => "start",
                Self::Process => "process",
                Self::End => "end",
            }
        }

        fn is_terminal(&self) -> bool {
            matches!(self, Self::End)
        }

        fn is_blocking(&self) -> bool {
            false
        }

        fn order(&self) -> u32 {
            match self {
                Self::Start => 10,
                Self::Process => 20,
                Self::End => 99,
            }
        }
    }

    #[test]
    fn custom_phase_id_works() {
        assert_eq!(TestPhase::Start.name(), "start");
        assert!(!TestPhase::Start.is_terminal());
        assert!(TestPhase::End.is_terminal());
        assert!(!TestPhase::Process.is_blocking());
        assert!(TestPhase::Start.order() < TestPhase::Process.order());
    }

    // -- PhaseStatus 测试 --

    #[test]
    fn phase_status_display() {
        assert_eq!(format!("{}", PhaseStatus::Success), "success");
        assert_eq!(format!("{}", PhaseStatus::Blocked), "blocked");
        assert_eq!(format!("{}", PhaseStatus::Skipped), "skipped");
        assert_eq!(format!("{}", PhaseStatus::Running), "running");
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
    fn phase_status_is_success() {
        assert!(PhaseStatus::Success.is_success());
        assert!(!PhaseStatus::Failed.is_success());
    }

    #[test]
    fn phase_status_serde_roundtrip() {
        let status = PhaseStatus::Success;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"success\"");
        let parsed: PhaseStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }

    // -- PhaseExecution 测试 --

    #[test]
    fn phase_execution_constructors() {
        let phase = NamedPhase::new("test");

        let exec = PhaseExecution::success(phase.clone(), 100);
        assert_eq!(exec.status, PhaseStatus::Success);
        assert_eq!(exec.latency_ms, 100);

        let exec = PhaseExecution::failed(phase.clone(), 50, "timeout");
        assert_eq!(exec.status, PhaseStatus::Failed);
        assert_eq!(exec.error.as_deref(), Some("timeout"));

        let exec = PhaseExecution::blocked(phase.clone(), 50, "quality too low");
        assert_eq!(exec.status, PhaseStatus::Blocked);
        assert!(exec.error.is_some());

        let exec = PhaseExecution::skipped(phase);
        assert_eq!(exec.status, PhaseStatus::Skipped);
        assert_eq!(exec.latency_ms, 0);
    }

    // -- PhaseHandler 默认实现测试 --

    struct TestHandler {
        phase: NamedPhase,
    }

    impl PhaseHandler<NamedPhase> for TestHandler {
        fn phase(&self) -> NamedPhase {
            self.phase.clone()
        }
    }

    #[test]
    fn phase_handler_default_execute_returns_error() {
        use crate::context::{PipelineInput, PipelineOptions};

        let handler = TestHandler {
            phase: NamedPhase::new("test"),
        };
        let mut ctx = PipelineContext::new(
            PipelineInput::Query {
                query: "test".into(),
                session_id: None,
                context: std::collections::HashMap::new(),
            },
            PipelineOptions::default(),
        );
        let result = handler.execute(&mut ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not support synchronous"));
    }
}

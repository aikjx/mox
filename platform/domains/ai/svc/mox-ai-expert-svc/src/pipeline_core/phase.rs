// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 统一阶段定义与阶段处理器 trait
//!
//! 两套管线共享同一组基础阶段，各自扩展特有阶段：
//!
//! 通用阶段（两套管线都可能用到）：
//! - Normalize : 归一化/预处理（维度着色 / 意图分类）
//! - Analyze   : 分析阶段（专家并行 / 辩论咨询）
//! - Reconcile : 裁决/合成（多观点归一）
//! - Gate      : 质量门禁（评分 + 分级 + 决策）
//! - Learn     : 指标学习（可选阶段）
//! - Done      : 完成阶段（收尾 + 审计）
//!
//! mox 模块化系统架构管线特有：
//! - Optimize  : flow-ai 优化求解
//! - Verify    : 璇玑算法验证（最高权限否决）
//!
//! 联盟管线特有：
//! - Team      : 专家组队
//! - Synthesize: 合成输出（Markdown 等）

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::pipeline_core::context::PipelineContext;
use crate::pipeline_core::result::PhaseResult;

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
    /// 归一化/预处理
    /// - mox 模块化系统架构管线：维度着色 (auto_dimension)
    /// - 联盟管线：意图分类 (classify_intent)
    Normalize,

    /// 分析阶段
    /// - mox 模块化系统架构管线：14 位专家并行分析 (run_experts)
    /// - 联盟管线：并行咨询 + 辩论 (consult_and_debate)
    Analyze,

    /// 裁决/合成阶段
    /// - mox 模块化系统架构管线：多专家观点裁决 (reconcile)
    /// - 联盟管线：辩论结果合成
    Reconcile,

    /// 质量门禁阶段
    /// - mox 模块化系统架构管线：govern() 布尔型闸门 + 算法否决
    /// - 联盟管线：HC-8 四级评分闸门 (evaluate_gate)
    Gate,

    /// 指标学习阶段（可选）
    /// - 联盟管线：learn_metrics() 维度增益学习
    /// - mox 模块化系统架构管线：预留，未来可接入 CEM
    Learn,

    /// 完成阶段（收尾 + 审计汇总）
    Done,

    // ---- mox 模块化系统架构管线特有阶段 ----
    /// flow-ai 优化求解
    Optimize,

    /// 璇玑算法验证（最高权限，不可被治理覆盖）
    Verify,

    // ---- 联盟管线特有阶段 ----
    /// 专家组队（基于意图选择专家）
    Team,

    /// 合成输出（Markdown / 报告生成）
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
        // Gate 阶段失败默认阻断；可通过选项配置
        matches!(self, Self::Gate)
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
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
        matches!(self, Self::Success | Self::Failed | Self::Blocked | Self::Skipped)
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked)
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
    /// 默认不跳过。子类可根据上下文条件判断是否跳过（如 Learn 阶段在 C 级以下跳过）。
    fn should_skip(&self, _ctx: &PipelineContext) -> bool {
        false
    }
}

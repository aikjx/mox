// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 联盟统一错误类型
//!
//! 基于 mox-error，定义专家联盟域的统一错误码和错误类型。

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 联盟错误码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum AllianceErrorCode {
    // === 通用错误 (1000-1999) ===
    /// 未知错误
    Unknown = 1000,
    /// 无效参数
    InvalidArgument = 1001,
    /// 未找到
    NotFound = 1002,
    /// 已存在
    AlreadyExists = 1003,
    /// 权限不足
    PermissionDenied = 1004,
    /// 租户不匹配
    TenantMismatch = 1005,

    // === 任务相关 (2000-2999) ===
    /// 任务不存在
    TaskNotFound = 2000,
    /// 任务状态非法
    InvalidTaskStatus = 2001,
    /// 任务已终止，不可操作
    TaskAlreadyTerminal = 2002,
    /// 任务创建失败
    TaskCreationFailed = 2003,

    // === 计划相关 (3000-3999) ===
    /// 计划生成失败
    PlanGenerationFailed = 3000,
    /// 计划无效（有环/依赖缺失）
    InvalidPlan = 3001,
    /// 计划版本冲突
    PlanVersionConflict = 3002,

    // === 执行相关 (4000-4999) ===
    /// 节点执行失败
    NodeExecutionFailed = 4000,
    /// 节点不存在
    NodeNotFound = 4001,
    /// 执行引擎不可用
    ExecutorUnavailable = 4002,
    /// 节点依赖不满足
    DependencyNotMet = 4003,

    // === 专家相关 (5000-5999) ===
    /// 专家不存在
    ExpertNotFound = 5000,
    /// 专家不可用
    ExpertUnavailable = 5001,
    /// 专家匹配失败
    ExpertMatchFailed = 5002,
    /// 专家注册失败
    ExpertRegistrationFailed = 5003,

    // === 融合相关 (6000-6999) ===
    /// 融合失败
    FusionFailed = 6000,
    /// 融合策略不支持
    UnsupportedFusionStrategy = 6001,

    // === 调度相关 (7000-7999) ===
    /// 调度器已满
    SchedulerFull = 7000,
    /// 任务排队超时
    QueueTimeout = 7001,
}

/// 联盟统一错误
#[derive(Debug, Error)]
pub enum AllianceError {
    #[error("{code:?}: {message}")]
    Business {
        code: AllianceErrorCode,
        message: String,
    },

    #[error("Internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AllianceError {
    pub fn new(code: AllianceErrorCode, message: impl Into<String>) -> Self {
        Self::Business {
            code,
            message: message.into(),
        }
    }

    pub fn not_found(resource: &str, id: &str) -> Self {
        Self::Business {
            code: AllianceErrorCode::NotFound,
            message: format!("{} not found: {}", resource, id),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::Business {
            code: AllianceErrorCode::InvalidArgument,
            message: message.into(),
        }
    }

    pub fn code(&self) -> Option<AllianceErrorCode> {
        match self {
            Self::Business { code, .. } => Some(*code),
            _ => None,
        }
    }
}

/// 联盟统一结果类型
pub type AllianceResult<T> = Result<T, AllianceError>;

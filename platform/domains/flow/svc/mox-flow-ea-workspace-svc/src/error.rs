// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! 错误类型定义

use thiserror::Error;

pub type WorkspaceResult<T> = Result<T, WorkspaceError>;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("KG 域服务错误: {0}")]
    KgServiceError(String),

    #[error("Cloud 域服务错误: {0}")]
    CloudServiceError(String),

    #[error("Expert 域服务错误: {0}")]
    ExpertServiceError(String),

    #[error("算法执行错误: {0}")]
    AlgorithmError(String),

    #[error("资源未找到: {0}")]
    NotFound(String),

    #[error("参数错误: {0}")]
    InvalidArgument(String),

    #[error("聚合超时: {0}")]
    AggregationTimeout(String),

    #[error("内部错误: {0}")]
    InternalError(String),
}

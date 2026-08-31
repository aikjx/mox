// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 融合模块错误类型
//!
//! 使用 thiserror 定义融合相关的所有错误类型，
//! 遵循纯计算、零 IO 的 Core 层原则。

use thiserror::Error;

/// 融合结果类型
pub type FusionResult<T> = Result<T, FusionError>;

/// 融合错误枚举
///
/// 覆盖融合过程中可能出现的所有错误场景：
/// - 输入验证错误
/// - 配置错误
/// - 计算错误（如不收敛、除零等）
/// - 策略不支持
#[derive(Debug, Error)]
pub enum FusionError {
    /// 输入为空
    #[error("empty input: no candidates provided for fusion")]
    EmptyInput,

    /// 权重和为零（无法归一化）
    #[error("total weight is zero, cannot normalize weighted sum")]
    ZeroTotalWeight,

    /// 权重包含非有限值（NaN 或 Infinity）
    #[error("invalid weight value at index {index}: {value}")]
    InvalidWeight {
        /// 出错的权重索引
        index: usize,
        /// 具体的无效值描述
        value: String,
    },

    /// 置信度包含非有限值或超出 [0, 1] 范围
    #[error("invalid confidence value at index {index}: {value} (must be in [0, 1])")]
    InvalidConfidence {
        /// 出错的置信度索引
        index: usize,
        /// 具体的无效值
        value: f64,
    },

    /// 迭代次数不足或不收敛
    #[error("iterative refinement failed to converge after {max_iterations} iterations (last_delta: {last_delta})")]
    NotConverged {
        /// 最大迭代次数
        max_iterations: usize,
        /// 最后一次迭代的变化量
        last_delta: f64,
    },

    /// 策略参数无效
    #[error("invalid parameter '{param}': {reason}")]
    InvalidParameter {
        /// 参数名
        param: &'static str,
        /// 错误原因
        reason: String,
    },

    /// 融合策略不支持
    #[error("unsupported fusion strategy: {0}")]
    UnsupportedStrategy(String),

    /// 辩论轮次不足
    #[error("debate requires at least {required} participants, got {actual}")]
    InsufficientParticipants {
        /// 最少需要的参与者数量
        required: usize,
        /// 实际参与者数量
        actual: usize,
    },

    /// 维度不匹配
    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// 期望的维度
        expected: usize,
        /// 实际的维度
        actual: usize,
    },
}

impl FusionError {
    /// 便捷构造函数：无效参数
    pub fn invalid_param(param: &'static str, reason: impl Into<String>) -> Self {
        Self::InvalidParameter {
            param,
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_messages() {
        let err = FusionError::EmptyInput;
        assert!(err.to_string().contains("empty input"));

        let err = FusionError::ZeroTotalWeight;
        assert!(err.to_string().contains("zero"));

        let err = FusionError::InvalidWeight {
            index: 3,
            value: "NaN".to_string(),
        };
        assert!(err.to_string().contains("index 3"));
        assert!(err.to_string().contains("NaN"));

        let err = FusionError::InvalidConfidence {
            index: 0,
            value: 1.5,
        };
        assert!(err.to_string().contains("1.5"));
        assert!(err.to_string().contains("[0, 1]"));

        let err = FusionError::NotConverged {
            max_iterations: 100,
            last_delta: 0.05,
        };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("0.05"));

        let err = FusionError::invalid_param("k", "must be positive");
        assert!(err.to_string().contains("k"));
        assert!(err.to_string().contains("positive"));

        let err = FusionError::UnsupportedStrategy("unknown".to_string());
        assert!(err.to_string().contains("unknown"));

        let err = FusionError::InsufficientParticipants {
            required: 2,
            actual: 1,
        };
        assert!(err.to_string().contains("2"));
        assert!(err.to_string().contains("1"));

        let err = FusionError::DimensionMismatch {
            expected: 5,
            actual: 3,
        };
        assert!(err.to_string().contains("5"));
        assert!(err.to_string().contains("3"));
    }
}

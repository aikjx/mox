// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 统一错误类型（基于 mox-error 的专家域错误码）
//!
//! ## 错误码规范
//! 格式：`AI{模块码:02d}{序号:03d}`
//!
//! - AI10xxx: 专家注册/管理模块
//! - AI11xxx: 咨询服务模块
//! - AI12xxx: 联盟编排模块
//! - AI13xxx: 治理闸门模块
//! - AI14xxx: 意图识别模块
//! - AI15xxx: 算法分析模块
//!
//! 所有错误都实现为 `mox_error::MoxError`，支持：
//! - 统一错误码
//! - 错误等级（Info/Warning/Error/Critical）
//! - HTTP 状态码映射
//! - trace_id 追踪
//! - 错误链（source）

use mox_error::{define_domain_errors, ErrorDomain, MoxError, MoxResult};

// ============================================================================
// 专家域错误码常量
// ============================================================================

/// 专家注册/管理模块错误（AI10xxx）
pub mod expert {
    use super::*;

    define_domain_errors!(ExpertErrors, Ai,
        // 专家注册模块 (10)
        NOT_FOUND:          (10, 001, "专家不存在", 404, Warning),
        ALREADY_EXISTS:     (10, 002, "专家已存在", 409, Warning),
        INVALID_CONFIG:     (10, 003, "专家配置无效", 400, Warning),
        DIMENSION_MISMATCH: (10, 004, "专家维度不匹配", 422, Warning),
        REGISTRY_LOCKED:    (10, 005, "注册表锁失败", 500, Error),
    );
}

/// 咨询服务模块错误（AI11xxx）
pub mod consult {
    use super::*;

    define_domain_errors!(ConsultErrors, Ai,
        TIMEOUT:            (11, 001, "专家咨询超时", 504, Error),
        FAILED:             (11, 002, "专家咨询失败", 500, Error),
        EMPTY_QUERY:        (11, 003, "咨询查询不能为空", 400, Warning),
        FLOW_INVALID:       (11, 004, "流程图无效", 400, Warning),
        EXPERT_UNAVAILABLE: (11, 005, "指定专家不可用", 404, Warning),
        CONTEXT_INVALID:    (11, 006, "上下文参数无效", 400, Warning),
    );
}

/// 联盟编排模块错误（AI12xxx）
pub mod alliance {
    use super::*;

    define_domain_errors!(AllianceErrors, Ai,
        TEAM_BUILD_FAILED:  (12, 001, "专家组队失败", 500, Error),
        INTENT_CLASSIFY_FAILED: (12, 002, "意图分类失败", 500, Error),
        DEBATE_TIMEOUT:     (12, 003, "专家辩论超时", 504, Error),
        GATE_BLOCKED:       (12, 004, "质量门禁不通过", 422, Error),
        UNAUTHORIZED:       (12, 005, "RBAC 未授权", 403, Warning),
        EMPTY_TASK:         (12, 006, "任务规格不能为空", 400, Warning),
        ORCHESTRATION_FAILED: (12, 007, "任务编排失败", 500, Error),
    );
}

/// 治理闸门模块错误（AI13xxx）
pub mod governance {
    use super::*;

    define_domain_errors!(GovernanceErrors, Ai,
        VETOED:             (13, 001, "治理闸门否决", 422, Error),
        SLA_EXCEEDED:       (13, 002, "SLA 超出限制", 422, Warning),
        BUDGET_EXCEEDED:    (13, 003, "预算超出限制", 422, Warning),
        POLICY_LOAD_FAILED: (13, 004, "策略加载失败", 500, Error),
        SENSITIVITY_MISMATCH: (13, 005, "敏感等级不匹配", 422, Warning),
    );
}

// ============================================================================
// 便捷类型别名
// ============================================================================

/// 专家域统一结果类型
pub type ExpertResult<T> = MoxResult<T>;

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expert_not_found_error_code() {
        let err = expert::ExpertErrors::NOT_FOUND();
        assert_eq!(err.code, "AI10001");
        assert_eq!(err.http_status, 404);
    }

    #[test]
    fn consult_timeout_error_code() {
        let err = consult::ConsultErrors::TIMEOUT();
        assert_eq!(err.code, "AI11001");
        assert_eq!(err.http_status, 504);
    }

    #[test]
    fn alliance_gate_blocked_error_code() {
        let err = alliance::AllianceErrors::GATE_BLOCKED();
        assert_eq!(err.code, "AI12004");
        assert_eq!(err.http_status, 422);
    }

    #[test]
    fn governance_vetoed_error_code() {
        let err = governance::GovernanceErrors::VETOED();
        assert_eq!(err.code, "AI13001");
        assert_eq!(err.http_status, 422);
    }

    #[test]
    fn expert_result_type_alias() {
        let r: ExpertResult<i32> = Ok(42);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn error_has_trace_id() {
        let err = expert::ExpertErrors::NOT_FOUND();
        assert!(!err.trace_id.is_empty());
    }

    #[test]
    fn error_has_timestamp() {
        let err = consult::ConsultErrors::FAILED();
        assert!(!err.timestamp.is_empty());
    }

    #[test]
    fn all_error_codes_have_unique_prefix() {
        let codes = vec![
            expert::ExpertErrors::NOT_FOUND().code,
            consult::ConsultErrors::TIMEOUT().code,
            alliance::AllianceErrors::GATE_BLOCKED().code,
            governance::GovernanceErrors::VETOED().code,
        ];
        // 所有错误码都以 AI 开头（AI 域）
        for code in &codes {
            assert!(code.starts_with("AI"), "错误码应该以 AI 开头: {}", code);
        }
    }
}

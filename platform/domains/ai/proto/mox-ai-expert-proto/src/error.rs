// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 统一错误类型（基于 mox-error 的专家域错误码）
//!
//! ## 错误码规范
//! 格式：`AI{模块码:02d}{序号:03d}`
//!
//! | 模块码 | 模块名称       | 错误码范围   | 说明                     |
//! |--------|---------------|-------------|--------------------------|
//! | 10     | expert        | AI10001~    | 专家注册 / 管理模块       |
//! | 11     | consult       | AI11001~    | 咨询服务模块              |
//! | 12     | alliance      | AI12001~    | 联盟编排模块              |
//! | 13     | governance    | AI13001~    | 治理闸门模块              |
//! | 14     | intent        | AI14001~    | 意图识别模块              |
//! | 15     | algo_analysis | AI15001~    | 算法分析模块              |
//!
//! 所有错误都实现为 `mox_error::MoxError`，支持：
//! - 统一错误码（AI + 模块 + 序号）
//! - 错误等级（Info / Warning / Error / Critical）
//! - HTTP 状态码映射
//! - trace_id 分布式追踪
//! - 错误链（source / detail）
//! - 序列化 / 反序列化（serde）
//!
//! ## 迁移说明
//! 当前实现直接基于 `mox-error` crate 的 `define_domain_errors!` 宏。
//! 如果未来需要脱离 mox-error 独立使用，可通过 feature flag 切换到
//! `thiserror` 实现，并保留相同的错误码前缀和 API 形状。

use mox_error::{define_domain_errors, MoxError, MoxResult};

// ============================================================================
// 类型别名
// ============================================================================

/// 专家域统一错误类型（MoxError 的别名，便于语义化使用）
///
/// 下游通过 `ExpertError` 引用专家域错误，与 `ExpertResult<T>` 配对使用。
/// 实际类型为 `mox_error::MoxError`，保留完整的错误码、trace_id、
/// 错误等级、HTTP 状态码等能力。
pub type ExpertError = MoxError;

/// 专家域统一结果类型
pub type ExpertResult<T> = MoxResult<T>;

// ============================================================================
// 专家注册/管理模块（AI10xxx）
// ============================================================================

/// 专家注册/管理模块错误码（AI10xxx）
///
/// 覆盖专家注册、查询、更新、注销等生命周期操作。
pub mod expert {
    use super::*;

    define_domain_errors!(ExpertErrors, Ai,
        // -- 基础 CRUD (001-010) --
        // 专家不存在
        NOT_FOUND:            (10, 001, "专家不存在", 404, Warning),
        // 专家已存在
        ALREADY_EXISTS:       (10, 002, "专家已存在", 409, Warning),
        // 专家配置无效
        INVALID_CONFIG:       (10, 003, "专家配置无效", 400, Warning),
        // 专家维度不匹配
        DIMENSION_MISMATCH:   (10, 004, "专家维度不匹配", 422, Warning),
        // 注册表锁失败
        REGISTRY_LOCKED:      (10, 005, "注册表锁失败", 500, Error),

        // -- 注册/注销 (011-020) --
        // 专家注册失败
        REGISTER_FAILED:      (10, 011, "专家注册失败", 500, Error),
        // 专家注销失败
        UNREGISTER_FAILED:    (10, 012, "专家注销失败", 500, Error),
        // 专家已禁用
        DISABLED:             (10, 013, "专家已禁用", 403, Warning),

        // -- 能力/元数据 (021-030) --
        // 专家能力不支持
        CAPABILITY_UNSUPPORTED: (10, 021, "专家不支持该能力", 400, Warning),
        // 专家元数据缺失
        METADATA_MISSING:     (10, 022, "专家元数据缺失", 422, Warning),
    );
}

// ============================================================================
// 咨询服务模块（AI11xxx）
// ============================================================================

/// 咨询服务模块错误码（AI11xxx）
///
/// 覆盖单次专家咨询的输入校验、执行、输出全流程。
pub mod consult {
    use super::*;

    define_domain_errors!(ConsultErrors, Ai,
        // -- 输入校验 (001-010) --
        // 咨询查询不能为空
        EMPTY_QUERY:          (11, 001, "咨询查询不能为空", 400, Warning),
        // 上下文参数无效
        CONTEXT_INVALID:      (11, 002, "上下文参数无效", 400, Warning),
        // 流程图无效
        FLOW_INVALID:         (11, 003, "流程图无效", 400, Warning),
        // 查询格式不支持
        QUERY_FORMAT_UNSUPPORTED: (11, 004, "查询格式不支持", 400, Warning),

        // -- 执行错误 (011-020) --
        // 专家咨询超时
        TIMEOUT:              (11, 011, "专家咨询超时", 504, Error),
        // 专家咨询失败
        FAILED:               (11, 012, "专家咨询失败", 500, Error),
        // 指定专家不可用
        EXPERT_UNAVAILABLE:   (11, 013, "指定专家不可用", 404, Warning),
        // 专家执行异常
        EXECUTION_ERROR:      (11, 014, "专家执行异常", 500, Error),

        // -- 输出/报告 (021-030) --
        // 报告生成失败
        REPORT_GENERATION_FAILED: (11, 021, "咨询报告生成失败", 500, Error),
        // 报告格式无效
        REPORT_FORMAT_INVALID: (11, 022, "咨询报告格式无效", 500, Error),
    );
}

// ============================================================================
// 联盟编排模块（AI12xxx）
// ============================================================================

/// 联盟编排模块错误码（AI12xxx）
///
/// 覆盖多专家协作、路由、辩论、质量门禁等编排逻辑。
pub mod alliance {
    use super::*;

    define_domain_errors!(AllianceErrors, Ai,
        // -- 组队/路由 (001-010) --
        // 专家组队失败
        TEAM_BUILD_FAILED:    (12, 001, "专家组队失败", 500, Error),
        // 意图分类失败
        INTENT_CLASSIFY_FAILED: (12, 002, "意图分类失败", 500, Error),
        // 任务规格不能为空
        EMPTY_TASK:           (12, 003, "任务规格不能为空", 400, Warning),
        // 无匹配专家
        NO_MATCHING_EXPERT:   (12, 004, "无匹配的专家", 404, Warning),

        // -- 编排执行 (011-020) --
        // 任务编排失败
        ORCHESTRATION_FAILED: (12, 011, "任务编排失败", 500, Error),
        // 专家辩论超时
        DEBATE_TIMEOUT:       (12, 012, "专家辩论超时", 504, Error),
        // 质量门禁不通过
        GATE_BLOCKED:         (12, 013, "质量门禁不通过", 422, Error),
        // RBAC 未授权
        UNAUTHORIZED:         (12, 014, "RBAC 未授权", 403, Warning),

        // -- 结果聚合 (021-030) --
        // 结果聚合失败
        AGGREGATION_FAILED:   (12, 021, "专家结果聚合失败", 500, Error),
        // 共识计算失败
        CONSENSUS_FAILED:     (12, 022, "共识计算失败", 500, Error),
    );
}

// ============================================================================
// 治理闸门模块（AI13xxx）
// ============================================================================

/// 治理闸门模块错误码（AI13xxx）
///
/// 覆盖治理策略加载、SLA/预算校验、敏感等级、否决裁决等治理操作。
pub mod governance {
    use super::*;

    define_domain_errors!(GovernanceErrors, Ai,
        // -- 策略/规则 (001-010) --
        // 治理闸门否决
        VETOED:               (13, 001, "治理闸门否决", 422, Error),
        // 策略加载失败
        POLICY_LOAD_FAILED:   (13, 002, "策略加载失败", 500, Error),
        // 策略规则无效
        POLICY_INVALID:       (13, 003, "治理策略规则无效", 400, Warning),

        // -- 资源限制 (011-020) --
        // SLA 超出限制
        SLA_EXCEEDED:         (13, 011, "SLA 超出限制", 422, Warning),
        // 预算超出限制
        BUDGET_EXCEEDED:      (13, 012, "预算超出限制", 422, Warning),
        // 敏感等级不匹配
        SENSITIVITY_MISMATCH: (13, 013, "敏感等级不匹配", 422, Warning),
        // 并行度超限
        PARALLELISM_EXCEEDED: (13, 014, "并行度超出限制", 422, Warning),

        // -- 合规/审计 (021-030) --
        // 合规校验失败
        COMPLIANCE_FAILED:    (13, 021, "合规校验失败", 422, Error),
        // 审计记录失败
        AUDIT_LOG_FAILED:     (13, 022, "审计记录失败", 500, Error),
    );
}

// ============================================================================
// 意图识别模块（AI14xxx）
// ============================================================================

/// 意图识别模块错误码（AI14xxx）
///
/// 覆盖用户意图解析、分类、槽位填充、歧义消解等 NLU 相关操作。
pub mod intent {
    use super::*;

    define_domain_errors!(IntentErrors, Ai,
        // -- 输入/解析 (001-010) --
        // 意图文本为空
        EMPTY_INPUT:          (14, 001, "意图识别输入为空", 400, Warning),
        // 意图文本过长
        INPUT_TOO_LONG:       (14, 002, "意图识别输入过长", 400, Warning),
        // 语言不支持
        LANGUAGE_UNSUPPORTED: (14, 003, "不支持的语言", 400, Warning),

        // -- 分类/识别 (011-020) --
        // 意图识别失败
        RECOGNITION_FAILED:   (14, 011, "意图识别失败", 500, Error),
        // 意图置信度过低
        LOW_CONFIDENCE:       (14, 012, "意图置信度过低", 422, Warning),
        // 意图歧义无法消解
        AMBIGUOUS:            (14, 013, "意图歧义无法消解", 422, Warning),
        // 未知意图
        UNKNOWN_INTENT:       (14, 014, "无法识别的意图", 422, Warning),

        // -- 槽位/实体 (021-030) --
        // 槽位填充失败
        SLOT_FILL_FAILED:     (14, 021, "槽位填充失败", 500, Error),
        // 必需槽位缺失
        REQUIRED_SLOT_MISSING: (14, 022, "必需槽位缺失", 422, Warning),
    );
}

// ============================================================================
// 算法分析模块（AI15xxx）
// ============================================================================

/// 算法分析模块错误码（AI15xxx）
///
/// 覆盖图算法、优化算法、统计算法、归一化算法等分析类操作。
pub mod algo_analysis {
    use super::*;

    define_domain_errors!(AlgoAnalysisErrors, Ai,
        // -- 输入校验 (001-010) --
        // 输入数据为空
        EMPTY_INPUT:          (15, 001, "算法输入数据为空", 400, Warning),
        // 数据格式无效
        INVALID_FORMAT:       (15, 002, "数据格式无效", 400, Warning),
        // 数据规模超限
        SIZE_EXCEEDED:        (15, 003, "数据规模超出算法处理上限", 400, Warning),

        // -- 执行错误 (011-020) --
        // 算法执行失败
        EXECUTION_FAILED:     (15, 011, "算法执行失败", 500, Error),
        // 算法执行超时
        TIMEOUT:              (15, 012, "算法执行超时", 504, Error),
        // 算法不收敛
        NOT_CONVERGED:        (15, 013, "算法不收敛", 422, Warning),
        // 数值溢出/不稳定
        NUMERICAL_INSTABILITY: (15, 014, "数值计算不稳定", 500, Error),

        // -- 结果/归一化 (021-030) --
        // 归一化失败
        NORMALIZATION_FAILED: (15, 021, "归一化计算失败", 500, Error),
        // 结果为空
        EMPTY_RESULT:         (15, 022, "算法结果为空", 422, Warning),
    );
}

// ============================================================================
// 便捷入口（模块路径别名）
// ============================================================================

/// 专家域错误模块便捷引用（便于 use 单一路径访问所有子模块）
///
/// # 用法
///
/// ```rust,ignore
/// use mox_ai_expert_proto::error::{expert, consult, alliance};
///
/// let err = expert::ExpertErrors::NOT_FOUND();
/// let err2 = consult::ConsultErrors::TIMEOUT();
/// ```
///
/// 所有错误构造函数都是关联函数（associated function），
/// 通过 `ModuleErrors::ERROR_NAME()` 形式调用。
pub mod errors {
    pub use super::algo_analysis as algo;
    pub use super::alliance;
    pub use super::consult;
    pub use super::expert;
    pub use super::governance;
    pub use super::intent;
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- 各模块错误码格式验证 --

    #[test]
    fn expert_module_error_codes() {
        assert_eq!(expert::ExpertErrors::NOT_FOUND().code, "AI10001");
        assert_eq!(expert::ExpertErrors::NOT_FOUND().http_status, 404);
        assert_eq!(expert::ExpertErrors::ALREADY_EXISTS().code, "AI10002");
        assert_eq!(expert::ExpertErrors::REGISTER_FAILED().code, "AI10011");
        assert_eq!(expert::ExpertErrors::CAPABILITY_UNSUPPORTED().code, "AI10021");
        assert_eq!(expert::ExpertErrors::DISABLED().http_status, 403);
    }

    #[test]
    fn consult_module_error_codes() {
        assert_eq!(consult::ConsultErrors::EMPTY_QUERY().code, "AI11001");
        assert_eq!(consult::ConsultErrors::TIMEOUT().code, "AI11011");
        assert_eq!(consult::ConsultErrors::TIMEOUT().http_status, 504);
        assert_eq!(consult::ConsultErrors::FAILED().code, "AI11012");
        assert_eq!(consult::ConsultErrors::REPORT_GENERATION_FAILED().code, "AI11021");
        assert_eq!(consult::ConsultErrors::EXPERT_UNAVAILABLE().http_status, 404);
    }

    #[test]
    fn alliance_module_error_codes() {
        assert_eq!(alliance::AllianceErrors::TEAM_BUILD_FAILED().code, "AI12001");
        assert_eq!(alliance::AllianceErrors::GATE_BLOCKED().code, "AI12013");
        assert_eq!(alliance::AllianceErrors::GATE_BLOCKED().http_status, 422);
        assert_eq!(alliance::AllianceErrors::ORCHESTRATION_FAILED().code, "AI12011");
        assert_eq!(alliance::AllianceErrors::AGGREGATION_FAILED().code, "AI12021");
        assert_eq!(alliance::AllianceErrors::NO_MATCHING_EXPERT().http_status, 404);
    }

    #[test]
    fn governance_module_error_codes() {
        assert_eq!(governance::GovernanceErrors::VETOED().code, "AI13001");
        assert_eq!(governance::GovernanceErrors::VETOED().http_status, 422);
        assert_eq!(governance::GovernanceErrors::SLA_EXCEEDED().code, "AI13011");
        assert_eq!(governance::GovernanceErrors::BUDGET_EXCEEDED().code, "AI13012");
        assert_eq!(governance::GovernanceErrors::COMPLIANCE_FAILED().code, "AI13021");
        assert_eq!(governance::GovernanceErrors::POLICY_INVALID().http_status, 400);
    }

    #[test]
    fn intent_module_error_codes() {
        assert_eq!(intent::IntentErrors::EMPTY_INPUT().code, "AI14001");
        assert_eq!(intent::IntentErrors::EMPTY_INPUT().http_status, 400);
        assert_eq!(intent::IntentErrors::RECOGNITION_FAILED().code, "AI14011");
        assert_eq!(intent::IntentErrors::LOW_CONFIDENCE().code, "AI14012");
        assert_eq!(intent::IntentErrors::AMBIGUOUS().code, "AI14013");
        assert_eq!(intent::IntentErrors::SLOT_FILL_FAILED().code, "AI14021");
        assert_eq!(intent::IntentErrors::REQUIRED_SLOT_MISSING().http_status, 422);
    }

    #[test]
    fn algo_analysis_module_error_codes() {
        assert_eq!(algo_analysis::AlgoAnalysisErrors::EMPTY_INPUT().code, "AI15001");
        assert_eq!(algo_analysis::AlgoAnalysisErrors::EXECUTION_FAILED().code, "AI15011");
        assert_eq!(algo_analysis::AlgoAnalysisErrors::TIMEOUT().code, "AI15012");
        assert_eq!(algo_analysis::AlgoAnalysisErrors::TIMEOUT().http_status, 504);
        assert_eq!(algo_analysis::AlgoAnalysisErrors::NOT_CONVERGED().code, "AI15013");
        assert_eq!(algo_analysis::AlgoAnalysisErrors::NORMALIZATION_FAILED().code, "AI15021");
        assert_eq!(algo_analysis::AlgoAnalysisErrors::EMPTY_RESULT().http_status, 422);
    }

    // -- 类型别名验证 --

    #[test]
    fn expert_result_type_alias() {
        let r: ExpertResult<i32> = Ok(42);
        assert_eq!(r.unwrap(), 42);

        let err: ExpertResult<i32> = Err(expert::ExpertErrors::NOT_FOUND());
        assert!(err.is_err());
    }

    #[test]
    fn expert_error_type_alias_is_mox_error() {
        // ExpertError 应该就是 MoxError 的别名
        let err: ExpertError = expert::ExpertErrors::NOT_FOUND();
        assert_eq!(err.code, "AI10001");
        assert_eq!(err.domain.code(), "AI");
    }

    // -- 通用字段验证 --

    #[test]
    fn error_has_trace_id() {
        let err = expert::ExpertErrors::NOT_FOUND();
        assert!(!err.trace_id.is_empty());
        // trace_id 应该是 UUID 格式（36 字符含连字符）
        assert_eq!(err.trace_id.len(), 36);
    }

    #[test]
    fn error_has_timestamp() {
        let err = consult::ConsultErrors::FAILED();
        assert!(!err.timestamp.is_empty());
        // 时间戳应该是 RFC3339 格式
        assert!(err.timestamp.contains('T'));
    }

    #[test]
    fn error_supports_detail_and_source_chain() {
        let err = alliance::AllianceErrors::ORCHESTRATION_FAILED()
            .with_detail("task_id=abc, retry=3")
            .with_trace_id("trace-12345");

        assert_eq!(err.detail.as_deref(), Some("task_id=abc, retry=3"));
        assert_eq!(err.trace_id, "trace-12345");
    }

    // -- 模块前缀唯一性验证 --

    #[test]
    fn all_error_codes_have_ai_prefix() {
        let codes = vec![
            expert::ExpertErrors::NOT_FOUND().code,
            consult::ConsultErrors::TIMEOUT().code,
            alliance::AllianceErrors::GATE_BLOCKED().code,
            governance::GovernanceErrors::VETOED().code,
            intent::IntentErrors::RECOGNITION_FAILED().code,
            algo_analysis::AlgoAnalysisErrors::EXECUTION_FAILED().code,
        ];
        for code in &codes {
            assert!(code.starts_with("AI"), "错误码应该以 AI 开头: {}", code);
            // AI + 2位模块码 + 3位序号 = 7字符
            assert_eq!(code.len(), 7, "错误码长度应为 7: {}", code);
        }
    }

    #[test]
    fn each_module_has_unique_module_code() {
        // 验证六个模块的模块码各不相同
        let e1 = expert::ExpertErrors::NOT_FOUND();
        let e2 = consult::ConsultErrors::EMPTY_QUERY();
        let e3 = alliance::AllianceErrors::TEAM_BUILD_FAILED();
        let e4 = governance::GovernanceErrors::VETOED();
        let e5 = intent::IntentErrors::EMPTY_INPUT();
        let e6 = algo_analysis::AlgoAnalysisErrors::EMPTY_INPUT();

        let prefixes = vec![
            &e1.code[..4],   // AI10
            &e2.code[..4],   // AI11
            &e3.code[..4],   // AI12
            &e4.code[..4],   // AI13
            &e5.code[..4],   // AI14
            &e6.code[..4],   // AI15
        ];

        use std::collections::HashSet;
        let unique: HashSet<_> = prefixes.iter().collect();
        assert_eq!(unique.len(), 6, "六个模块应有不同的模块码前缀");
    }

    // -- 便捷入口模块验证 --

    #[test]
    fn errors_module_reexports_work() {
        // 通过 errors 便捷模块访问各子模块应该等价于直接访问
        use errors::{algo, alliance, consult, expert, governance, intent};

        assert_eq!(expert::ExpertErrors::NOT_FOUND().code, "AI10001");
        assert_eq!(consult::ConsultErrors::TIMEOUT().code, "AI11011");
        assert_eq!(alliance::AllianceErrors::GATE_BLOCKED().code, "AI12013");
        assert_eq!(governance::GovernanceErrors::VETOED().code, "AI13001");
        assert_eq!(intent::IntentErrors::LOW_CONFIDENCE().code, "AI14012");
        assert_eq!(algo::AlgoAnalysisErrors::EXECUTION_FAILED().code, "AI15011");
    }

    // -- Display / Debug --

    #[test]
    fn error_display_format() {
        let err = governance::GovernanceErrors::VETOED();
        let display = format!("{}", err);
        assert!(display.contains("AI13001"));
        assert!(display.contains("ERROR"));
    }

    // -- 错误等级分布 --

    #[test]
    fn error_levels_are_appropriate() {
        use mox_error::ErrorLevel;

        // 4xx 类通常是 Warning
        assert_eq!(expert::ExpertErrors::NOT_FOUND().level, ErrorLevel::Warning);
        assert_eq!(intent::IntentErrors::LOW_CONFIDENCE().level, ErrorLevel::Warning);

        // 5xx 类通常是 Error
        assert_eq!(consult::ConsultErrors::FAILED().level, ErrorLevel::Error);
        assert_eq!(algo_analysis::AlgoAnalysisErrors::EXECUTION_FAILED().level, ErrorLevel::Error);

        // 422 否决类是 Error
        assert_eq!(governance::GovernanceErrors::VETOED().level, ErrorLevel::Error);
        assert_eq!(alliance::AllianceErrors::GATE_BLOCKED().level, ErrorLevel::Error);
    }

    // -- 序列化 --

    #[test]
    fn error_serialization_roundtrip() {
        let err = intent::IntentErrors::AMBIGUOUS()
            .with_detail("multiple intents matched: a, b, c");

        let json = serde_json::to_string(&err).expect("序列化失败");
        let parsed: MoxError = serde_json::from_str(&json).expect("反序列化失败");

        assert_eq!(parsed.code, err.code);
        assert_eq!(parsed.message, err.message);
        assert_eq!(parsed.level, err.level);
        assert_eq!(parsed.trace_id, err.trace_id);
        assert_eq!(parsed.detail, err.detail);
    }
}

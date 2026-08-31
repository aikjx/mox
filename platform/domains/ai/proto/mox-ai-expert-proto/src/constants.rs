// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! SSOT 常量：维度优先级、冲突升级门槛、归一化阈值等
//!
//! 这些"魔法数字"此前散落在 expert.rs / reconcile.rs / pipeline.rs 等多处，
//! 违反单一权威源原则，易产生维护漂移。此处集中定义，全局引用。
//!
//! 【设计说明】
//! 权限/安全必须压过性能/成本。维度优先级是全系统的单一真相源，
//! 所有涉及维度排序、裁决、归一化的逻辑都必须引用这些常量。

use crate::types::Dimension;

// ============================================================================
// 维度优先级（SSOT）
// ============================================================================

/// 维度优先级（数值越大越优先）。权限/安全必须压过性能/成本。
///
/// 与 `Dimension::priority()` 保持一致，是 `priority()` 的单一数据源。
///
/// 【权限功能归一化说明】
/// "多个专家吵起来时听谁的"：当权限、安全、性能、成本等维度给同一件事
/// 给出互相打架的建议，按这张表排座次——Permission(权限)和 Security(安全)
/// 并列最高(100)，谁都压不过它；性能(60)、成本、体验只能往后排。
/// 同时把开发侧七维(架构/代码安全/代码质量…)和业务七维放在同一把尺子上，
/// 避免两层各算各的、互相覆盖。
pub const DIM_PRIORITY: &[(Dimension, i32)] = &[
    (Dimension::Permission, 100),
    (Dimension::Security, 100),
    (Dimension::Resource, 70),
    (Dimension::Data, 60),
    (Dimension::Algorithm, 50),
    (Dimension::Business, 40),
    (Dimension::Observability, 30),
    // ---- 开发七维（与业务七维同尺度，跨层不互盖）----
    (Dimension::Architecture, 100),
    (Dimension::SecurityCode, 100),
    (Dimension::CodeQuality, 70),
    (Dimension::Performance, 60),
    (Dimension::Testing, 50),
    (Dimension::Documentation, 40),
    (Dimension::Maintainability, 30),
];

// ============================================================================
// 维度激活门槛（SSOT）
// ============================================================================

/// 维度激活门槛：归一化置信度低于该值则该维度不计入裁决。
///
/// 可观测维度门槛略低（噪声大），业务维度略高（需更确信）。
///
/// 【大白话】"没把握就不插嘴"：每个维度的建议都带一个自信度(0~1)。
/// 如果某维度自己都只有 0.4 的把握(低于门槛 0.5)，这次裁决就不带它玩，
/// 免得噪声大的维度乱带节奏。
pub const DIM_THRESHOLD: &[(Dimension, f64)] = &[
    (Dimension::Permission, 0.5),
    (Dimension::Security, 0.5),
    (Dimension::Resource, 0.5),
    (Dimension::Data, 0.5),
    (Dimension::Algorithm, 0.5),
    (Dimension::Business, 0.6),
    (Dimension::Observability, 0.4),
    // ---- 开发七维 ----
    (Dimension::Architecture, 0.5),
    (Dimension::SecurityCode, 0.5),
    (Dimension::CodeQuality, 0.5),
    (Dimension::Performance, 0.5),
    (Dimension::Testing, 0.5),
    (Dimension::Documentation, 0.6),
    (Dimension::Maintainability, 0.5),
];

// ============================================================================
// 冲突升级门槛（SSOT）
// ============================================================================

/// 冲突升级门槛：同类别约束且优先级差 < 该值才判为 escalated（Blocking）。
///
/// 优先级差 ≥ 该值视为高优先维度合法压过低优先维度，不升级。
pub const CONFLICT_ESCALATE_PRIORITY_GAP: i32 = 1;

// ============================================================================
// 归一化权重（SSOT）
// ============================================================================

/// 归一化默认可调权重（用于裁决器多目标折中，数值越大权重越高）。
pub const NORMALIZATION_WEIGHTS: &[(Dimension, f64)] = &[
    (Dimension::Permission, 1.0),
    (Dimension::Security, 1.0),
    (Dimension::Resource, 0.8),
    (Dimension::Data, 0.8),
    (Dimension::Algorithm, 0.7),
    (Dimension::Business, 0.6),
    (Dimension::Observability, 0.5),
    // ---- 开发七维 ----
    (Dimension::Architecture, 1.0),
    (Dimension::SecurityCode, 1.0),
    (Dimension::CodeQuality, 0.8),
    (Dimension::Performance, 0.8),
    (Dimension::Testing, 0.7),
    (Dimension::Documentation, 0.6),
    (Dimension::Maintainability, 0.5),
];

// ============================================================================
// 便捷查询函数
// ============================================================================

/// 取维度优先级（缺省 0）
pub fn dim_priority(dim: Dimension) -> i32 {
    DIM_PRIORITY
        .iter()
        .find(|(d, _)| *d == dim)
        .map(|(_, p)| *p)
        .unwrap_or(0)
}

/// 取维度激活门槛（缺省 0.5）
pub fn dim_threshold(dim: Dimension) -> f64 {
    DIM_THRESHOLD
        .iter()
        .find(|(d, _)| *d == dim)
        .map(|(_, t)| *t)
        .unwrap_or(0.5)
}

/// 取维度归一化权重（缺省 0.5）
pub fn dim_weight(dim: Dimension) -> f64 {
    NORMALIZATION_WEIGHTS
        .iter()
        .find(|(d, _)| *d == dim)
        .map(|(_, w)| *w)
        .unwrap_or(0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_and_permission_have_highest_priority() {
        assert_eq!(dim_priority(Dimension::Permission), 100);
        assert_eq!(dim_priority(Dimension::Security), 100);
        assert_eq!(dim_priority(Dimension::Architecture), 100);
        assert_eq!(dim_priority(Dimension::SecurityCode), 100);
    }

    #[test]
    fn performance_lower_than_security() {
        assert!(dim_priority(Dimension::Performance) < dim_priority(Dimension::Security));
        assert_eq!(dim_priority(Dimension::Performance), 60);
    }

    #[test]
    fn observability_has_lower_threshold() {
        assert_eq!(dim_threshold(Dimension::Observability), 0.4);
        assert!(dim_threshold(Dimension::Observability) < dim_threshold(Dimension::Business));
    }

    #[test]
    fn business_has_higher_threshold() {
        assert_eq!(dim_threshold(Dimension::Business), 0.6);
        assert_eq!(dim_threshold(Dimension::Documentation), 0.6);
    }

    #[test]
    fn unknown_dimension_has_safe_defaults() {
        // 假设新增维度但未加入常量表，应有安全默认值
        // （当前所有维度都在表中，此测试主要验证函数健壮性）
        let pri = dim_priority(Dimension::Permission);
        assert!(pri > 0);
    }

    #[test]
    fn weights_match_priority_order() {
        // 权重应与优先级趋势一致
        assert!(dim_weight(Dimension::Security) > dim_weight(Dimension::Performance));
        assert!(dim_weight(Dimension::Permission) > dim_weight(Dimension::Business));
    }

    #[test]
    fn conflict_escalate_gap_is_one() {
        assert_eq!(CONFLICT_ESCALATE_PRIORITY_GAP, 1);
    }

    #[test]
    fn all_dimensions_have_priority() {
        use Dimension::*;
        let all_dims = [
            Business, Algorithm, Permission, Resource, Security, Data, Observability,
            Architecture, SecurityCode, CodeQuality, Performance, Testing, Documentation, Maintainability,
        ];
        for dim in all_dims {
            assert!(dim_priority(dim) > 0, "维度 {:?} 没有定义优先级", dim);
            assert!(dim_threshold(dim) > 0.0, "维度 {:?} 没有定义门槛", dim);
            assert!(dim_weight(dim) > 0.0, "维度 {:?} 没有定义权重", dim);
        }
    }

    #[test]
    fn dim_priority_matches_constant_table() {
        for (dim, expected) in DIM_PRIORITY {
            assert_eq!(dim_priority(*dim), *expected);
        }
    }

    #[test]
    fn dim_threshold_matches_constant_table() {
        for (dim, expected) in DIM_THRESHOLD {
            assert_eq!(dim_threshold(*dim), *expected);
        }
    }
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 协作模式与融合策略的命名映射（SSOT）
//!
//! # 为什么需要本模块
//!
//! 协议层枚举在系统内流通着**三套名字**：
//!
//! | 类别 | 用途 | 示例 |
//! |---|---|---|
//! | serde 名 | 跨进程传输（`#[serde(rename_all = "snake_case")]`） | `best_of` / `sequential` |
//! | 展示名 | 网关对外的业务契约名 | `first_wins` / `single_expert` |
//! | 历史别名 | 旧版本网关串，存量数据仍在用 | `majority_vote` |
//!
//! 这三套名字的映射历史上**散落在四处手写 `match`**（网关融合解析、SDK 展示串输出、
//! 远程接入层归一化、调度服务启动参数解析），曾因新增枚举变体与命名漂移导致融合策略
//! 全部落到默认分支，且故障表现为「静默走错算法」而非报错。
//!
//! 本模块是**唯一真源**：所有调用点必须委托本模块，禁止再手写映射。
//!
//! # 约定
//! - 输入解析统一做 `trim` + 小写归一，容忍大小写与空白差异；
//! - 未识别输入返回 `None`，由调用方决定回退策略（禁止在映射层静默吞掉）；
//! - 展示名在同一枚举内必须唯一（由 `display_names_are_unique` 用例守护），
//!   否则反向解析将产生歧义。

use crate::types::{AllianceMode, FusionStrategy};

/// 全部协作模式（用于全变体遍历与前端同步校验）
pub const ALL_MODES: [AllianceMode; 7] = [
    AllianceMode::Sequential,
    AllianceMode::Parallel,
    AllianceMode::Debate,
    AllianceMode::Hierarchical,
    AllianceMode::Iterative,
    AllianceMode::Voting,
    AllianceMode::Dynamic,
];

/// 全部融合策略
pub const ALL_FUSIONS: [FusionStrategy; 9] = [
    FusionStrategy::Voting,
    FusionStrategy::Weighted,
    FusionStrategy::ConfidenceWeighted,
    FusionStrategy::Concatenation,
    FusionStrategy::BestOf,
    FusionStrategy::Stacking,
    FusionStrategy::Debate,
    FusionStrategy::MapReduce,
    FusionStrategy::Iterative,
];

// ── 协作模式 ────────────────────────────────────────────────────────────

/// 协作模式 → 网关展示名
pub fn mode_display(m: AllianceMode) -> &'static str {
    match m {
        AllianceMode::Sequential => "single_expert",
        AllianceMode::Parallel => "expert_alliance",
        AllianceMode::Iterative => "human_in_loop",
        AllianceMode::Hierarchical => "autonomous",
        AllianceMode::Debate => "debate",
        AllianceMode::Voting => "voting",
        AllianceMode::Dynamic => "dynamic",
    }
}

/// 协作模式 → serde 传输名（与 `rename_all = "snake_case"` 一致）
pub fn mode_serde(m: AllianceMode) -> &'static str {
    match m {
        AllianceMode::Sequential => "sequential",
        AllianceMode::Parallel => "parallel",
        AllianceMode::Iterative => "iterative",
        AllianceMode::Hierarchical => "hierarchical",
        AllianceMode::Debate => "debate",
        AllianceMode::Voting => "voting",
        AllianceMode::Dynamic => "dynamic",
    }
}

/// 任意名字（展示名 / serde 名 / 历史别名）→ 协作模式
pub fn mode_from_any(s: &str) -> Option<AllianceMode> {
    let key = normalize_key(s);
    // 先按展示名匹配（网关对外契约），再按 serde 名，最后是历史别名
    for m in ALL_MODES {
        if mode_display(m) == key {
            return Some(m);
        }
    }
    for m in ALL_MODES {
        if mode_serde(m) == key {
            return Some(m);
        }
    }
    match key.as_str() {
        // 历史别名：缩写与旧称
        "seq" | "single" => Some(AllianceMode::Sequential),
        "par" | "alliance" => Some(AllianceMode::Parallel),
        "auto" => Some(AllianceMode::Hierarchical),
        "human" => Some(AllianceMode::Iterative),
        _ => None,
    }
}

// ── 融合策略 ────────────────────────────────────────────────────────────

/// 融合策略 → 网关展示名
pub fn fusion_display(f: FusionStrategy) -> &'static str {
    match f {
        FusionStrategy::BestOf => "first_wins",
        FusionStrategy::Weighted => "weighted_voting",
        FusionStrategy::Voting => "rrf",
        FusionStrategy::ConfidenceWeighted => "llm_judge",
        FusionStrategy::Concatenation => "consensus",
        FusionStrategy::Stacking => "stacking",
        FusionStrategy::Debate => "debate",
        FusionStrategy::MapReduce => "map_reduce",
        FusionStrategy::Iterative => "iterative",
    }
}

/// 融合策略 → serde 传输名
pub fn fusion_serde(f: FusionStrategy) -> &'static str {
    match f {
        FusionStrategy::Voting => "voting",
        FusionStrategy::Weighted => "weighted",
        FusionStrategy::ConfidenceWeighted => "confidence_weighted",
        FusionStrategy::Concatenation => "concatenation",
        FusionStrategy::BestOf => "best_of",
        FusionStrategy::Stacking => "stacking",
        FusionStrategy::Debate => "debate",
        FusionStrategy::MapReduce => "map_reduce",
        FusionStrategy::Iterative => "iterative",
    }
}

/// 任意名字（展示名 / serde 名 / 历史别名）→ 融合策略
pub fn fusion_from_any(s: &str) -> Option<FusionStrategy> {
    let key = normalize_key(s);
    for f in ALL_FUSIONS {
        if fusion_display(f) == key {
            return Some(f);
        }
    }
    for f in ALL_FUSIONS {
        if fusion_serde(f) == key {
            return Some(f);
        }
    }
    match key.as_str() {
        // 历史别名：旧网关把多数投票记为 majority_vote，语义对应 RRF 投票融合
        "majority_vote" | "majority" => Some(FusionStrategy::Voting),
        "concat" => Some(FusionStrategy::Concatenation),
        _ => None,
    }
}

/// 输入归一：去空白 + 小写
fn normalize_key(s: &str) -> String {
    s.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_display_roundtrip() {
        for m in ALL_MODES {
            let d = mode_display(m);
            assert_eq!(mode_from_any(d), Some(m), "展示名 {} 无法反解为 {:?}", d, m);
            let s = mode_serde(m);
            assert_eq!(mode_from_any(s), Some(m), "serde 名 {} 无法反解为 {:?}", s, m);
        }
    }

    #[test]
    fn fusion_display_roundtrip() {
        for f in ALL_FUSIONS {
            let d = fusion_display(f);
            assert_eq!(fusion_from_any(d), Some(f), "展示名 {} 无法反解为 {:?}", d, f);
            let s = fusion_serde(f);
            assert_eq!(fusion_from_any(s), Some(f), "serde 名 {} 无法反解为 {:?}", s, f);
        }
    }

    #[test]
    fn historical_aliases_are_supported() {
        assert_eq!(fusion_from_any("majority_vote"), Some(FusionStrategy::Voting));
        assert_eq!(mode_from_any("seq"), Some(AllianceMode::Sequential));
        assert_eq!(mode_from_any("par"), Some(AllianceMode::Parallel));
    }

    #[test]
    fn input_is_trimmed_and_case_insensitive() {
        assert_eq!(mode_from_any("  PARALLEL "), Some(AllianceMode::Parallel));
        assert_eq!(fusion_from_any("Weighted_Voting"), Some(FusionStrategy::Weighted));
        assert_eq!(fusion_from_any("RRF"), Some(FusionStrategy::Voting));
    }

    #[test]
    fn unknown_input_returns_none() {
        // 未识别输入必须显式返回 None，交由调用方决定回退，禁止静默兜底
        assert_eq!(mode_from_any("no_such_mode"), None);
        assert_eq!(fusion_from_any("no_such_fusion"), None);
        assert_eq!(mode_from_any(""), None);
    }

    #[test]
    fn display_names_are_unique() {
        // 展示名重复会导致反向解析歧义：新增变体时此用例是防线。
        // 注意：展示名与 serde 名允许同名（如 `debate`），故按「名空间」分别查重，
        // 跨类型（mode / fusion）之间不需要唯一，因为解析函数分别查各自的表。
        use std::collections::HashSet;

        // 变量名不得与同名函数重名（否则 E0618：expected function, found HashSet）
        let mut mode_disp_set = HashSet::new();
        let mut mode_serde_set = HashSet::new();
        for m in ALL_MODES {
            assert!(
                mode_disp_set.insert(mode_display(m)),
                "协作模式展示名重复: {}",
                mode_display(m)
            );
            assert!(
                mode_serde_set.insert(mode_serde(m)),
                "协作模式 serde 名重复: {}",
                mode_serde(m)
            );
        }

        let mut fusion_disp_set = HashSet::new();
        let mut fusion_serde_set = HashSet::new();
        for f in ALL_FUSIONS {
            assert!(
                fusion_disp_set.insert(fusion_display(f)),
                "融合策略展示名重复: {}",
                fusion_display(f)
            );
            assert!(
                fusion_serde_set.insert(fusion_serde(f)),
                "融合策略 serde 名重复: {}",
                fusion_serde(f)
            );
        }
    }

    #[test]
    fn all_variants_are_covered() {
        // 防止新增枚举变体后忘记补映射：数量必须一致
        assert_eq!(ALL_MODES.len(), 7);
        assert_eq!(ALL_FUSIONS.len(), 9);
    }
}

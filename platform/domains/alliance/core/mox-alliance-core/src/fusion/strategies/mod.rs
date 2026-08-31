// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 六大融合策略实现
//!
//! - [`weighted_voting`] — 加权投票融合
//! - [`confidence_weighting`] — 置信度加权融合
//! - [`stacking`] — 堆叠融合（元学习器）
//! - [`debate`] — 辩论融合（多智能体辩论）
//! - [`map_reduce`] — Map-Reduce 融合
//! - [`iterative_refinement`] — 迭代精炼融合

pub mod weighted_voting;
pub mod confidence_weighting;
pub mod stacking;
pub mod debate;
pub mod map_reduce;
pub mod iterative_refinement;

pub use weighted_voting::WeightedVotingFusion;
pub use confidence_weighting::ConfidenceWeightingFusion;
pub use stacking::StackingFusion;
pub use debate::DebateFusion;
pub use map_reduce::MapReduceFusion;
pub use iterative_refinement::IterativeRefinementFusion;

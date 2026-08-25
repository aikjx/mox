//! # MOX Intent Core · 意图分类 + 联盟打分权威单源
//!
//! ## 核心算法
//! 1. Aho-Corasick 自动机（`aho-corasick` crate，最高性能 Rust 实现，DFA 编译一次）
//! 2. 等级评分：type match 10 / secondary 5 / capability 3 / keyword 2 / name 2
//! 3. TOP-K 排序（稳定性 + 确定性平局）
//! 4. 联盟打分（AllianceScoreCore）：rayon 并行逐专家，SIMD-like 浮点累计

pub mod classifier;
pub mod alliance;

pub use classifier::{classify_intent, IntentPattern, IntentResult, IntentClassifier};
pub use alliance::{ExpertCandidate, ScoredExpert, ScoreBreakdown, score_alliance_candidates, AllianceScorer};

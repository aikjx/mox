// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! # MOX Norm Core · 归一化流水线权威单源
//!
//! - 去重：Ahash 指纹 (64-bit) + 二级内容校验。
//! - 规则求解器：规则表达式（attribute-based conditions + actions）。
//! - 冲突融合：按优先级/新鲜度/来源权威加权合并。
//! - 增量字段合并：仅当新值可信度 ≥ 旧值才更新。

pub mod dedup;
pub mod rules;
pub mod merge;

use ahash::RandomState;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

/// 归一化记录（JSON Value 载体，与 Node project-atlas normalize API schema 对齐）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormRecord {
    pub id: String,
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value, RandomState>,
    /// 来源标签（如 "registry" / "atlas" / "ingest"）
    #[serde(default)]
    pub source: String,
    /// 新鲜度：UNIX ms
    #[serde(default)]
    pub updated_at_ms: i64,
    /// 来源可信度 [0,1]
    #[serde(default = "default_confidence")]
    pub confidence: f32,
}

fn default_confidence() -> f32 { 0.5 }

pub use dedup::{dedup_records, DedupReport};
pub use merge::{merge_records, merge_conflicts, ConflictMergeFn, MergeStrategy, MergeResult};
pub use rules::{Rule, RuleEngine, RuleOutcome, resolve_rules};

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn record_roundtrip() {
        let r = NormRecord {
            id: "x".into(),
            attributes: HashMap::with_hasher(RandomState::new()),
            source: "src".into(),
            updated_at_ms: 1,
            confidence: 0.8,
        };
        assert_eq!(r.id, "x");
        assert_eq!(r.confidence, 0.8);
    }
}

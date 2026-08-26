//! MOX KG Fusion Service
//!
//! Multi-source result fusion using Reciprocal Rank Fusion (RRF) and entity alignment.
//! Combines results from multiple graph query engines / indexes into a unified ranked list.
//!
//! Algorithms:
//! - RRF (Reciprocal Rank Fusion): k=60 default, robust rank aggregation
//! - Entity alignment: deduplicate by canonical ID with confidence scoring
//! - Weighted fusion: per-source quality weights

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FusionError {
    #[error("empty result set")]
    EmptyResultSet,
    #[error("invalid rank: {0}")]
    InvalidRank(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionResult {
    pub id: String,
    pub score: f64,
    pub rank: usize,
    pub sources: Vec<String>,
    pub source_scores: HashMap<String, f64>,
    pub source_ranks: HashMap<String, usize>,
    pub entity_type: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceResult {
    pub source: String,
    pub id: String,
    pub score: f64,
    pub rank: usize,
    pub entity_type: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct SourceResultSet {
    pub source: String,
    pub results: Vec<SourceResult>,
    pub weight: f64,
}

/// Reciprocal Rank Fusion (RRF) engine.
///
/// RRF score = sum over sources of weight / (k + rank)
/// Default k=60 (Cormack et al. 2009).
#[derive(Clone)]
pub struct RrfFusion {
    k: f64,
    source_weights: HashMap<String, f64>,
    default_weight: f64,
}

impl RrfFusion {
    pub fn new() -> Self {
        Self { k: 60.0, source_weights: HashMap::new(), default_weight: 1.0 }
    }

    pub fn with_k(k: f64) -> Self {
        Self { k, ..Self::new() }
    }

    pub fn set_source_weight(&mut self, source: &str, weight: f64) {
        self.source_weights.insert(source.into(), weight);
    }

    fn source_weight(&self, source: &str) -> f64 {
        self.source_weights.get(source).copied().unwrap_or(self.default_weight)
    }

    /// Fuse multiple result sets using RRF.
    pub fn fuse(&self, sets: &[SourceResultSet]) -> Result<Vec<FusionResult>, FusionError> {
        if sets.is_empty() { return Err(FusionError::EmptyResultSet); }

        let mut accum: HashMap<String, FusionResult> = HashMap::new();

        for set in sets {
            let w = self.source_weight(&set.source) * set.weight;
            for result in &set.results {
                let rrf_score = w / (self.k + result.rank as f64);
                let entry = accum.entry(result.id.clone()).or_insert_with(|| FusionResult {
                    id: result.id.clone(),
                    score: 0.0,
                    rank: 0,
                    sources: vec![],
                    source_scores: HashMap::new(),
                    source_ranks: HashMap::new(),
                    entity_type: result.entity_type.clone(),
                    metadata: serde_json::Value::Null,
                });
                entry.score += rrf_score;
                entry.sources.push(set.source.clone());
                entry.source_scores.insert(set.source.clone(), result.score);
                entry.source_ranks.insert(set.source.clone(), result.rank);
                if entry.entity_type.is_none() && result.entity_type.is_some() {
                    entry.entity_type = result.entity_type.clone();
                }
                // Merge metadata
                if result.metadata != serde_json::Value::Null {
                    if entry.metadata == serde_json::Value::Null {
                        entry.metadata = result.metadata.clone();
                    } else if let (Some(existing), Some(new)) = (entry.metadata.as_object_mut(), result.metadata.as_object()) {
                        for (k, v) in new { existing.insert(k.clone(), v.clone()); }
                    }
                }
            }
        }

        let mut fused: Vec<FusionResult> = accum.into_values().collect();
        fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        for (i, r) in fused.iter_mut().enumerate() { r.rank = i + 1; }

        Ok(fused)
    }

    /// Fuse with top-k truncation.
    pub fn fuse_top_k(&self, sets: &[SourceResultSet], k: usize) -> Result<Vec<FusionResult>, FusionError> {
        let mut results = self.fuse(sets)?;
        results.truncate(k);
        Ok(results)
    }
}

impl Default for RrfFusion {
    fn default() -> Self { Self::new() }
}

/// Entity alignment: deduplicate entities across sources by canonical ID mapping.
#[derive(Clone, Default)]
pub struct EntityAligner {
    aliases: HashMap<String, String>, // alias -> canonical_id
    confidence: HashMap<String, f64>, // canonical_id -> alignment confidence
}

impl EntityAligner {
    pub fn new() -> Self { Self::default() }

    /// Register an alias mapping to a canonical ID.
    pub fn register_alias(&mut self, alias: &str, canonical: &str, confidence: f64) {
        self.aliases.insert(alias.into(), canonical.into());
        self.confidence.insert(canonical.into(), confidence.max(*self.confidence.get(canonical).unwrap_or(&0.0)));
    }

    /// Get canonical ID for an entity.
    pub fn canonical(&self, id: &str) -> &str {
        self.aliases.get(id).map(|s| s.as_str()).unwrap_or(id)
    }

    /// Align a result set: map all IDs to canonical, merge duplicates.
    pub fn align(&self, results: Vec<SourceResult>) -> Vec<SourceResult> {
        let mut merged: HashMap<String, SourceResult> = HashMap::new();
        for mut r in results {
            let canon = self.canonical(&r.id).to_string();
            r.id = canon.clone();
            if let Some(existing) = merged.get_mut(&canon) {
                existing.score = existing.score.max(r.score);
                existing.rank = existing.rank.min(r.rank);
            } else {
                merged.insert(canon, r);
            }
        }
        merged.into_values().collect()
    }

    pub fn alignment_confidence(&self, canonical: &str) -> f64 {
        self.confidence.get(canonical).copied().unwrap_or(1.0)
    }
}

/// Combined fusion pipeline: align → RRF → rank.
#[derive(Clone)]
pub struct FusionPipeline {
    pub rrf: RrfFusion,
    pub aligner: EntityAligner,
}

impl FusionPipeline {
    pub fn new() -> Self {
        Self { rrf: RrfFusion::new(), aligner: EntityAligner::new() }
    }

    pub fn process(&self, sets: &[SourceResultSet]) -> Result<Vec<FusionResult>, FusionError> {
        // Step 1: Align entities in each set
        let aligned_sets: Vec<SourceResultSet> = sets.iter().map(|s| {
            SourceResultSet {
                source: s.source.clone(),
                results: self.aligner.align(s.results.clone()),
                weight: s.weight,
            }
        }).collect();

        // Step 2: RRF fusion
        self.rrf.fuse(&aligned_sets)
    }

    pub fn process_top_k(&self, sets: &[SourceResultSet], k: usize) -> Result<Vec<FusionResult>, FusionError> {
        let mut results = self.process(sets)?;
        results.truncate(k);
        Ok(results)
    }
}

impl Default for FusionPipeline {
    fn default() -> Self { Self::new() }
}

/// Weighted linear fusion (alternative to RRF for normalized scores).
pub struct WeightedFusion {
    weights: HashMap<String, f64>,
}

impl WeightedFusion {
    pub fn new() -> Self { Self { weights: HashMap::new() } }
    pub fn set_weight(&mut self, source: &str, w: f64) { self.weights.insert(source.into(), w); }

    pub fn fuse(&self, sets: &[SourceResultSet]) -> Result<Vec<FusionResult>, FusionError> {
        if sets.is_empty() { return Err(FusionError::EmptyResultSet); }
        let mut accum: HashMap<String, FusionResult> = HashMap::new();
        for set in sets {
            let w = self.weights.get(&set.source).copied().unwrap_or(1.0) * set.weight;
            for r in &set.results {
                let entry = accum.entry(r.id.clone()).or_insert_with(|| FusionResult {
                    id: r.id.clone(), score: 0.0, rank: 0, sources: vec![],
                    source_scores: HashMap::new(), source_ranks: HashMap::new(),
                    entity_type: r.entity_type.clone(), metadata: serde_json::Value::Null,
                });
                entry.score += r.score * w;
                entry.sources.push(set.source.clone());
                entry.source_scores.insert(set.source.clone(), r.score);
                entry.source_ranks.insert(set.source.clone(), r.rank);
            }
        }
        let mut results: Vec<FusionResult> = accum.into_values().collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        for (i, r) in results.iter_mut().enumerate() { r.rank = i + 1; }
        Ok(results)
    }
}

impl Default for WeightedFusion {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(source: &str, id: &str, rank: usize, score: f64) -> SourceResult {
        SourceResult { source: source.into(), id: id.into(), score, rank, entity_type: None, metadata: serde_json::Value::Null }
    }

    #[test]
    fn rrf_basic_fusion() {
        let rrf = RrfFusion::new();
        let set1 = SourceResultSet {
            source: "index_a".into(), weight: 1.0,
            results: vec![make_result("index_a", "x", 1, 0.9), make_result("index_a", "y", 2, 0.8)],
        };
        let set2 = SourceResultSet {
            source: "index_b".into(), weight: 1.0,
            results: vec![make_result("index_b", "y", 1, 0.95), make_result("index_b", "z", 2, 0.7)],
        };
        let fused = rrf.fuse(&[set1, set2]).unwrap();
        assert_eq!(fused.len(), 3);
        // "y" appears in both at ranks 2 and 1 → highest RRF score
        assert_eq!(fused[0].id, "y");
        assert!(fused[0].sources.contains(&"index_a".to_string()));
        assert!(fused[0].sources.contains(&"index_b".to_string()));
    }

    #[test]
    fn rrf_top_k() {
        let rrf = RrfFusion::new();
        let set = SourceResultSet {
            source: "s".into(), weight: 1.0,
            results: (1..=5).map(|i| make_result("s", &format!("n{}", i), i, 1.0 - i as f64 * 0.1)).collect(),
        };
        let top = rrf.fuse_top_k(&[set], 3).unwrap();
        assert_eq!(top.len(), 3);
    }

    #[test]
    fn entity_alignment() {
        let mut aligner = EntityAligner::new();
        aligner.register_alias("alias_1", "canonical_x", 0.95);
        assert_eq!(aligner.canonical("alias_1"), "canonical_x");
        assert_eq!(aligner.canonical("unknown"), "unknown");
    }

    #[test]
    fn fusion_pipeline() {
        let pipeline = FusionPipeline::new();
        let set1 = SourceResultSet {
            source: "a".into(), weight: 1.0,
            results: vec![make_result("a", "x", 1, 0.9)],
        };
        let set2 = SourceResultSet {
            source: "b".into(), weight: 1.0,
            results: vec![make_result("b", "x", 1, 0.8)],
        };
        let results = pipeline.process(&[set1, set2]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "x");
        assert_eq!(results[0].sources.len(), 2);
    }

    #[test]
    fn weighted_fusion() {
        let mut wf = WeightedFusion::new();
        wf.set_weight("high_quality", 2.0);
        let set1 = SourceResultSet {
            source: "high_quality".into(), weight: 1.0,
            results: vec![make_result("high_quality", "x", 1, 0.5)],
        };
        let set2 = SourceResultSet {
            source: "low_quality".into(), weight: 1.0,
            results: vec![make_result("low_quality", "x", 1, 0.9)],
        };
        let results = wf.fuse(&[set1, set2]).unwrap();
        // high_quality weight 2.0 * 0.5 = 1.0, low_quality 1.0 * 0.9 = 0.9
        assert!(results[0].score >= 1.0);
    }

    #[test]
    fn empty_set_error() {
        let rrf = RrfFusion::new();
        assert!(rrf.fuse(&[]).is_err());
    }
}

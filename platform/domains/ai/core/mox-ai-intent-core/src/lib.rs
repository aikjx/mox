// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! MOX AI Intent Core
//!
//! Intent recognition engine using A5 Activation Diffusion (personalized PageRank).
//! Maps natural language intents to capability routes on the knowledge graph.
//!
//! Core algorithm:
//! - A5 Activation Diffusion: d=0.85 damping, 30 iterations convergence
//! - Personalized PageRank over capability graph
//! - Threshold-based routing with fallback chain

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// 专家联盟打分模块
pub mod alliance;
/// 意图分类模块
pub mod classifier;

pub use alliance::{score_alliance_candidates, AllianceScorer, ExpertCandidate, ScoreBreakdown, ScoredExpert};
pub use classifier::{classify_intent, intent_to_capability, IntentClassifier, IntentPattern, IntentResult};

#[derive(Debug, Error)]
pub enum IntentError {
    #[error("empty input")]
    EmptyInput,
    #[error("no capabilities registered")]
    NoCapabilities,
    #[error("convergence failed after {0} iterations")]
    ConvergenceFailed(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub name: String,
    pub description: String,
    pub route: String,
    pub keywords: Vec<String>,
    pub domain: String,
    pub sla_latency_ms: u64,
    pub required_permissions: Vec<String>,
    pub owners: CapabilityOwners,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityOwners {
    pub product_alliance: Option<String>,
    pub algorithm_alliance: Option<String>,
    pub development_alliance: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentRequest {
    pub intent: String,
    pub context: Option<serde_json::Value>,
    pub principal: Option<String>,
    pub top_k: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentMatch {
    pub capability_id: String,
    pub capability_name: String,
    pub score: f64,
    pub route: String,
    pub matched_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResponse {
    pub request_id: String,
    pub query: String,
    pub matches: Vec<IntentMatch>,
    pub route_trace: RouteTrace,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteTrace {
    pub algorithm: String,
    pub damping_factor: f64,
    pub iterations: usize,
    pub activation_path: Vec<String>,
    pub fallback_used: bool,
}

/// A5 Activation Diffusion intent router.
///
/// Implements personalized PageRank over a capability graph:
/// - Nodes = capabilities
/// - Edges = semantic similarity / co-occurrence
/// - Personalization = keyword activation from query
#[derive(Clone)]
pub struct ActivationDiffusionRouter {
    capabilities: HashMap<String, Capability>,
    adjacency: HashMap<String, Vec<(String, f64)>>,
    damping: f64,
    max_iterations: usize,
    convergence_threshold: f64,
}

impl ActivationDiffusionRouter {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
            adjacency: HashMap::new(),
            damping: 0.85,
            max_iterations: 30,
            convergence_threshold: 1e-6,
        }
    }

    pub fn with_params(damping: f64, max_iterations: usize, threshold: f64) -> Self {
        Self { damping, max_iterations, convergence_threshold: threshold, ..Self::new() }
    }

    pub fn register(&mut self, cap: Capability) {
        let id = cap.id.clone();
        self.capabilities.insert(id.clone(), cap);
        self.adjacency.entry(id).or_default();
        self.rebuild_adjacency();
    }

    pub fn register_batch(&mut self, caps: Vec<Capability>) {
        for cap in caps {
            let id = cap.id.clone();
            self.capabilities.insert(id, cap);
        }
        self.rebuild_adjacency();
    }

    fn rebuild_adjacency(&mut self) {
        let ids: Vec<String> = self.capabilities.keys().cloned().collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let sim = self.keyword_similarity(&ids[i], &ids[j]);
                if sim > 0.1 {
                    self.adjacency.entry(ids[i].clone()).or_default().push((ids[j].clone(), sim));
                    self.adjacency.entry(ids[j].clone()).or_default().push((ids[i].clone(), sim));
                }
            }
        }
    }

    fn keyword_similarity(&self, a: &str, b: &str) -> f64 {
        let ka = self.capabilities.get(a).map(|c| &c.keywords).cloned().unwrap_or_default();
        let kb = self.capabilities.get(b).map(|c| &c.keywords).cloned().unwrap_or_default();
        if ka.is_empty() || kb.is_empty() { return 0.0; }
        let set_a: std::collections::HashSet<&str> = ka.iter().map(|s| s.as_str()).collect();
        let set_b: std::collections::HashSet<&str> = kb.iter().map(|s| s.as_str()).collect();
        let intersection = set_a.intersection(&set_b).count() as f64;
        let union = set_a.union(&set_b).count() as f64;
        if union == 0.0 { 0.0 } else { intersection / union }
    }

    /// Compute activation scores using personalized PageRank.
    pub fn route(&self, request: &IntentRequest) -> Result<IntentResponse, IntentError> {
        let start = std::time::Instant::now();
        if request.intent.trim().is_empty() { return Err(IntentError::EmptyInput); }
        if self.capabilities.is_empty() { return Err(IntentError::NoCapabilities); }

        let query_lower = request.intent.to_lowercase();
        let query_tokens: Vec<&str> = query_lower.split_whitespace().collect();

        // Personalization vector: activate capabilities with matching keywords
        let mut personalization: HashMap<String, f64> = HashMap::new();
        let mut total_activation = 0.0f64;
        for cap in self.capabilities.values() {
            let mut score = 0.0f64;
            let mut matched = vec![];
            for kw in &cap.keywords {
                if query_tokens.contains(&kw.to_lowercase().as_str()) {
                    score += 1.0;
                    matched.push(kw.clone());
                }
            }
            // Description substring match
            for token in &query_tokens {
                if cap.description.to_lowercase().contains(token) {
                    score += 0.3;
                }
            }
            if score > 0.0 {
                personalization.insert(cap.id.clone(), score);
                total_activation += score;
            }
        }

        // If no keyword match, uniform personalization (fallback)
        let fallback_used = personalization.is_empty();
        if fallback_used {
            let n = self.capabilities.len() as f64;
            for id in self.capabilities.keys() {
                personalization.insert(id.clone(), 1.0 / n);
            }
            total_activation = 1.0;
        }

        // Normalize personalization
        for v in personalization.values_mut() {
            *v /= total_activation;
        }

        // Power iteration: PageRank with personalization
        let n = self.capabilities.len();
        let mut scores: HashMap<String, f64> = self.capabilities.keys().map(|k| (k.clone(), 1.0 / n as f64)).collect();
        let mut iterations = 0;

        for iter in 0..self.max_iterations {
            iterations = iter + 1;
            let mut new_scores: HashMap<String, f64> = HashMap::new();
            let mut dangling_sum = 0.0;

            // Compute dangling node contribution
            for (id, score) in &scores {
                if self.adjacency.get(id).map(|v| v.is_empty()).unwrap_or(true) {
                    dangling_sum += score;
                }
            }

            for id in self.capabilities.keys() {
                let teleport = (1.0 - self.damping) * personalization.get(id).copied().unwrap_or(0.0);
                let dangling = self.damping * dangling_sum / n as f64;
                let mut incoming = 0.0;
                if let Some(neighbors) = self.adjacency.get(id) {
                    for (src, weight) in neighbors {
                        if let Some(src_score) = scores.get(src) {
                            let out_sum: f64 = self.adjacency.get(src).map(|v| v.iter().map(|(_, w)| w).sum()).unwrap_or(1.0);
                            if out_sum > 0.0 {
                                incoming += src_score * weight / out_sum;
                            }
                        }
                    }
                }
                new_scores.insert(id.clone(), teleport + dangling + self.damping * incoming);
            }

            // Check convergence
            let diff: f64 = scores.iter().map(|(k, v)| (v - new_scores.get(k).unwrap_or(&0.0)).abs()).sum();
            scores = new_scores;
            if diff < self.convergence_threshold { break; }
        }

        // Sort by score
        let mut ranked: Vec<(String, f64)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top_k = request.top_k.unwrap_or(5).min(ranked.len());
        let matches: Vec<IntentMatch> = ranked.into_iter().take(top_k).map(|(id, score)| {
            let cap = self.capabilities.get(&id).unwrap();
            let matched_keywords: Vec<String> = cap.keywords.iter()
                .filter(|kw| query_tokens.contains(&kw.to_lowercase().as_str()))
                .cloned().collect();
            IntentMatch {
                capability_id: id,
                capability_name: cap.name.clone(),
                score,
                route: cap.route.clone(),
                matched_keywords,
            }
        }).collect();

        let activation_path: Vec<String> = matches.iter().map(|m| m.capability_id.clone()).collect();

        Ok(IntentResponse {
            request_id: uuid::Uuid::now_v7().to_string(),
            query: request.intent.clone(),
            matches,
            route_trace: RouteTrace {
                algorithm: "A5-ActivationDiffusion".into(),
                damping_factor: self.damping,
                iterations,
                activation_path,
                fallback_used,
            },
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    pub fn list_capabilities(&self) -> Vec<&Capability> {
        self.capabilities.values().collect()
    }

    pub fn capability_count(&self) -> usize {
        self.capabilities.len()
    }
}

impl Default for ActivationDiffusionRouter {
    fn default() -> Self { Self::new() }
}

/// Simple keyword-based intent classifier (fast path, no graph).
pub struct KeywordIntentClassifier {
    patterns: Vec<(String, String)>, // (pattern, capability_id)
}

impl KeywordIntentClassifier {
    pub fn new() -> Self { Self { patterns: vec![] } }
    pub fn add_pattern(&mut self, pattern: &str, capability_id: &str) {
        self.patterns.push((pattern.to_lowercase(), capability_id.into()));
    }
    pub fn classify(&self, text: &str) -> Option<&str> {
        let lower = text.to_lowercase();
        self.patterns.iter().find(|(p, _)| lower.contains(p)).map(|(_, id)| id.as_str())
    }
}

impl Default for KeywordIntentClassifier {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_capabilities() -> Vec<Capability> {
        vec![
            Capability {
                id: "graph.search".into(), name: "Graph Search".into(),
                description: "Search knowledge graph nodes and edges".into(),
                route: "/api/graph/search".into(),
                keywords: vec!["search".into(), "graph".into(), "find".into(), "query".into()],
                domain: "kg".into(), sla_latency_ms: 100,
                required_permissions: vec![],
                owners: CapabilityOwners { product_alliance: None, algorithm_alliance: Some("kg".into()), development_alliance: Some("kg".into()) },
            },
            Capability {
                id: "flow.execute".into(), name: "Flow Execute".into(),
                description: "Execute operator workflow DAG".into(),
                route: "/api/flow/execute".into(),
                keywords: vec!["execute".into(), "workflow".into(), "flow".into(), "run".into()],
                domain: "flow".into(), sla_latency_ms: 500,
                required_permissions: vec!["flow:execute".into()],
                owners: CapabilityOwners { product_alliance: None, algorithm_alliance: None, development_alliance: Some("flow".into()) },
            },
            Capability {
                id: "ai.chat".into(), name: "AI Chat".into(),
                description: "Conversational AI assistant".into(),
                route: "/ai/engine/process".into(),
                keywords: vec!["chat".into(), "ask".into(), "ai".into(), "help".into()],
                domain: "ai".into(), sla_latency_ms: 2000,
                required_permissions: vec![],
                owners: CapabilityOwners { product_alliance: Some("ai".into()), algorithm_alliance: Some("ai".into()), development_alliance: Some("ai".into()) },
            },
        ]
    }

    #[test]
    fn activation_diffusion_routes() {
        let mut router = ActivationDiffusionRouter::new();
        router.register_batch(test_capabilities());
        let resp = router.route(&IntentRequest {
            intent: "search the graph for nodes".into(),
            context: None, principal: None, top_k: Some(3),
        }).unwrap();
        assert!(!resp.matches.is_empty());
        assert_eq!(resp.matches[0].capability_id, "graph.search");
        assert!(resp.latency_ms < 1000);
    }

    #[test]
    fn fallback_when_no_match() {
        let mut router = ActivationDiffusionRouter::new();
        router.register_batch(test_capabilities());
        let resp = router.route(&IntentRequest {
            intent: "xyzzy nonsense".into(), context: None, principal: None, top_k: Some(1),
        }).unwrap();
        assert!(resp.route_trace.fallback_used);
    }

    #[test]
    fn keyword_classifier() {
        let mut cls = KeywordIntentClassifier::new();
        cls.add_pattern("search", "graph.search");
        assert_eq!(cls.classify("please search this"), Some("graph.search"));
        assert_eq!(cls.classify("nothing here"), None);
    }

    #[test]
    fn empty_input_rejected() {
        let router = ActivationDiffusionRouter::new();
        assert!(router.route(&IntentRequest { intent: "".into(), context: None, principal: None, top_k: None }).is_err());
    }
}

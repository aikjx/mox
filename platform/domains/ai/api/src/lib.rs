//! MOX AI Domain API — trait contracts for intent routing and capability registry.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiApiError {
    #[error("intent not found: {0}")]
    IntentNotFound(String),
    #[error("capability not found: {0}")]
    CapabilityNotFound(String),
    #[error("routing failed: {0}")]
    RoutingFailed(String),
    #[error("internal: {0}")]
    Internal(String),
}

pub type AiApiResult<T> = Result<T, AiApiError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    pub intent: String,
    pub confidence: f64,
    pub matched_capabilities: Vec<String>,
    pub scores: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub domain: String,
    pub keywords: Vec<String>,
    pub weight: f64,
    pub enabled: bool,
}

#[async_trait]
pub trait IntentRouter: Send + Sync {
    async fn route(&self, query: &str) -> AiApiResult<IntentResult>;
    async fn route_with_context(&self, query: &str, context: &serde_json::Value) -> AiApiResult<IntentResult>;
    fn list_intents(&self) -> Vec<String>;
}

#[async_trait]
pub trait CapabilityRegistry: Send + Sync {
    async fn register(&self, capability: CapabilityInfo) -> AiApiResult<()>;
    async fn unregister(&self, capability_id: &str) -> AiApiResult<bool>;
    async fn get(&self, capability_id: &str) -> AiApiResult<Option<CapabilityInfo>>;
    async fn search(&self, query: &str) -> AiApiResult<Vec<CapabilityInfo>>;
    async fn list(&self, domain: Option<&str>) -> AiApiResult<Vec<CapabilityInfo>>;
}

pub trait ActivationDiffusion: Send + Sync {
    fn spread(&self, start_nodes: &[String], damping: f64, max_iter: usize) -> HashMap<String, f64>;
    fn converge_threshold(&self) -> f64;
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX Data Domain API — trait contracts for cross-domain data operations.
//!
//! This crate defines the public trait boundaries that other domains depend on.
//! Implementations live in `mox-data-*-core` / `mox-data-*-svc` crates.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

// ─── Common Types ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRecord {
    pub id: String,
    pub entity_type: String,
    pub payload: serde_json::Value,
    pub metadata: RecordMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecordMetadata {
    pub created_at: String,
    pub updated_at: String,
    pub source: String,
    pub version: u64,
    pub tags: Vec<String>,
}

impl DataRecord {
    pub fn new(entity_type: impl Into<String>, payload: serde_json::Value) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            entity_type: entity_type.into(),
            payload,
            metadata: RecordMetadata {
                created_at: now.clone(),
                updated_at: now,
                source: "unknown".into(),
                version: 1,
                tags: vec![],
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum DataApiError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("pipeline error: {0}")]
    Pipeline(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("internal: {0}")]
    Internal(String),
}

pub type DataApiResult<T> = Result<T, DataApiError>;

// ─── Data Record Store ───

#[async_trait]
pub trait DataRecordStore: Send + Sync {
    async fn get(&self, id: &str) -> DataApiResult<Option<DataRecord>>;
    async fn put(&self, record: DataRecord) -> DataApiResult<()>;
    async fn delete(&self, id: &str) -> DataApiResult<bool>;
    async fn query(&self, entity_type: &str, filter: &serde_json::Value) -> DataApiResult<Vec<DataRecord>>;
    async fn list(&self, entity_type: &str, limit: usize, offset: usize) -> DataApiResult<Vec<DataRecord>>;
}

// ─── Data Pipeline Operator ───

pub trait PipelineOperator: Send + Sync {
    fn name(&self) -> &str;
    fn process(&self, record: DataRecord) -> DataApiResult<DataRecord>;
}

#[async_trait]
pub trait DataPipeline: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, records: Vec<DataRecord>) -> DataApiResult<Vec<DataRecord>>;
    async fn execute_stream(&self, records: Vec<DataRecord>) -> DataApiResult<Vec<DataRecord>> {
        self.execute(records).await
    }
}

// ─── PII Detection & Masking ───

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PiiType {
    Email, Phone, IdCard, BankCard, IpAddress,
    Name, Address, DateOfBirth, Ssn, Passport,
    LicensePlate, Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PiiSeverity { Low, Medium, High, Critical }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiMatch {
    pub pii_type: PiiType,
    pub severity: PiiSeverity,
    pub start: usize,
    pub end: usize,
    pub matched: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaskMode {
    Redact, Hash, Partial, Replace, Skip,
}

pub trait PiiDetector: Send + Sync {
    fn detect(&self, text: &str) -> Vec<PiiMatch>;
    fn mask(&self, text: &str, mode: MaskMode) -> String;
    fn scan_record(&self, record: &DataRecord) -> Vec<PiiMatch>;
}

// ─── ETL Job ───

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EtlJobStatus { Pending, Running, Completed, Failed, Cancelled }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtlJobInfo {
    pub id: String,
    pub name: String,
    pub status: EtlJobStatus,
    pub progress: f64,
    pub records_processed: u64,
    pub error: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[async_trait]
pub trait EtlJobRunner: Send + Sync {
    async fn submit(&self, name: &str, config: &serde_json::Value) -> DataApiResult<String>;
    async fn status(&self, job_id: &str) -> DataApiResult<EtlJobInfo>;
    async fn cancel(&self, job_id: &str) -> DataApiResult<bool>;
    async fn list(&self, limit: usize) -> DataApiResult<Vec<EtlJobInfo>>;
}

// ─── Formula Engine ───

pub trait FormulaEngine: Send + Sync {
    fn evaluate(&self, formula: &str, context: &serde_json::Value) -> DataApiResult<serde_json::Value>;
    fn validate(&self, formula: &str) -> DataApiResult<()>;
    fn list_functions(&self) -> Vec<String>;
}

// ─── Data Normalization ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormRecord {
    pub id: String,
    pub fields: HashMap<String, serde_json::Value>,
    pub source: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStrategy {
    LastWriteWins, UnionFields, Majority, HighestConfidenceFirst, SourceAuthority,
}

pub trait DataNormalizer: Send + Sync {
    fn dedup(&self, records: &[NormRecord]) -> (Vec<NormRecord>, usize);
    fn merge(&self, records: &[NormRecord], strategy: MergeStrategy) -> Vec<NormRecord>;
    fn resolve_conflicts(&self, records: &[NormRecord]) -> Vec<NormRecord>;
}

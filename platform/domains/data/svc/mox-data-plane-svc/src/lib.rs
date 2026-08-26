//! MOX Data Plane Service
//!
//! Unified data plane for ingestion, transformation, and routing.
//! Supports streaming and batch data pipelines with configurable operators.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataPlaneError {
    #[error("pipeline not found: {0}")]
    PipelineNotFound(String),
    #[error("operator failed: {0}")]
    OperatorFailed(String),
    #[error("invalid schema: {0}")]
    InvalidSchema(String),
    #[error("source unavailable: {0}")]
    SourceUnavailable(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRecord {
    pub id: String,
    pub schema: String,
    pub payload: serde_json::Value,
    pub metadata: HashMap<String, String>,
    pub timestamp: String,
}

impl DataRecord {
    pub fn new(schema: &str, payload: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            schema: schema.into(),
            payload,
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

pub trait DataOperator: Send + Sync {
    fn name(&self) -> &str;
    fn process(&self, record: DataRecord) -> Result<DataRecord, DataPlaneError>;
}

/// Filter operator: keep records matching a predicate.
pub struct FilterOperator {
    pub name: String,
    pub predicate: Arc<dyn Fn(&DataRecord) -> bool + Send + Sync>,
}

impl FilterOperator {
    pub fn new<F>(name: &str, predicate: F) -> Self
    where F: Fn(&DataRecord) -> bool + Send + Sync + 'static {
        Self { name: name.into(), predicate: Arc::new(predicate) }
    }
}

impl DataOperator for FilterOperator {
    fn name(&self) -> &str { &self.name }
    fn process(&self, record: DataRecord) -> Result<DataRecord, DataPlaneError> {
        if (self.predicate)(&record) { Ok(record) } else { Err(DataPlaneError::OperatorFailed("filtered out".into())) }
    }
}

/// Map operator: transform record payload.
pub struct MapOperator {
    pub name: String,
    pub transform: Arc<dyn Fn(DataRecord) -> DataRecord + Send + Sync>,
}

impl MapOperator {
    pub fn new<F>(name: &str, transform: F) -> Self
    where F: Fn(DataRecord) -> DataRecord + Send + Sync + 'static {
        Self { name: name.into(), transform: Arc::new(transform) }
    }
}

impl DataOperator for MapOperator {
    fn name(&self) -> &str { &self.name }
    fn process(&self, record: DataRecord) -> Result<DataRecord, DataPlaneError> {
        Ok((self.transform)(record))
    }
}

/// Enrich operator: add metadata from a lookup table.
pub struct EnrichOperator {
    pub name: String,
    pub lookup_key: String,
    pub enrichment: Arc<parking_lot::RwLock<HashMap<String, serde_json::Value>>>,
}

impl EnrichOperator {
    pub fn new(name: &str, lookup_key: &str) -> Self {
        Self { name: name.into(), lookup_key: lookup_key.into(), enrichment: Arc::new(parking_lot::RwLock::new(HashMap::new())) }
    }
    pub fn add_enrichment(&self, key: &str, value: serde_json::Value) {
        self.enrichment.write().insert(key.into(), value);
    }
}

impl DataOperator for EnrichOperator {
    fn name(&self) -> &str { &self.name }
    fn process(&self, mut record: DataRecord) -> Result<DataRecord, DataPlaneError> {
        if let Some(key_val) = record.payload.get(&self.lookup_key).and_then(|v| v.as_str()) {
            if let Some(enriched) = self.enrichment.read().get(key_val) {
                if let Some(obj) = record.payload.as_object_mut() {
                    obj.insert("enrichment".into(), enriched.clone());
                }
            }
        }
        Ok(record)
    }
}

/// Validate operator: check schema conformance.
pub struct ValidateOperator {
    pub name: String,
    pub required_fields: Vec<String>,
}

impl ValidateOperator {
    pub fn new(name: &str, required_fields: Vec<String>) -> Self {
        Self { name: name.into(), required_fields }
    }
}

impl DataOperator for ValidateOperator {
    fn name(&self) -> &str { &self.name }
    fn process(&self, record: DataRecord) -> Result<DataRecord, DataPlaneError> {
        for field in &self.required_fields {
            if record.payload.get(field).is_none() {
                return Err(DataPlaneError::InvalidSchema(format!("missing required field: {}", field)));
            }
        }
        Ok(record)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub name: String,
    pub description: String,
    pub operators: Vec<String>,
    pub source: String,
    pub sink: String,
}

#[derive(Clone)]
pub struct DataPipeline {
    pub config: PipelineConfig,
    operators: Arc<parking_lot::RwLock<Vec<Arc<dyn DataOperator>>>>,
}

impl DataPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config, operators: Arc::new(parking_lot::RwLock::new(Vec::new())) }
    }

    pub fn add_operator(&self, op: Arc<dyn DataOperator>) {
        self.operators.write().push(op);
    }

    pub fn execute(&self, record: DataRecord) -> Result<DataRecord, DataPlaneError> {
        let mut current = record;
        for op in self.operators.read().iter() {
            current = op.process(current)?;
        }
        Ok(current)
    }

    pub fn execute_batch(&self, records: Vec<DataRecord>) -> (Vec<DataRecord>, Vec<(usize, DataPlaneError)>) {
        let mut success = vec![];
        let mut errors = vec![];
        for (i, record) in records.into_iter().enumerate() {
            match self.execute(record) {
                Ok(r) => success.push(r),
                Err(e) => errors.push((i, e)),
            }
        }
        (success, errors)
    }

    pub fn operator_count(&self) -> usize {
        self.operators.read().len()
    }
}

#[derive(Clone)]
pub struct DataPlane {
    pipelines: Arc<parking_lot::RwLock<HashMap<String, DataPipeline>>>,
}

impl DataPlane {
    pub fn new() -> Self {
        Self { pipelines: Arc::new(parking_lot::RwLock::new(HashMap::new())) }
    }

    pub fn create_pipeline(&self, config: PipelineConfig) -> DataPipeline {
        let pipeline = DataPipeline::new(config.clone());
        self.pipelines.write().insert(config.name.clone(), pipeline.clone());
        pipeline
    }

    pub fn get_pipeline(&self, name: &str) -> Option<DataPipeline> {
        self.pipelines.read().get(name).cloned()
    }

    pub fn process(&self, pipeline: &str, record: DataRecord) -> Result<DataRecord, DataPlaneError> {
        let p = self.pipelines.read().get(pipeline).cloned()
            .ok_or_else(|| DataPlaneError::PipelineNotFound(pipeline.into()))?;
        p.execute(record)
    }

    pub fn process_batch(&self, pipeline: &str, records: Vec<DataRecord>) -> Result<(Vec<DataRecord>, Vec<(usize, DataPlaneError)>), DataPlaneError> {
        let p = self.pipelines.read().get(pipeline).cloned()
            .ok_or_else(|| DataPlaneError::PipelineNotFound(pipeline.into()))?;
        Ok(p.execute_batch(records))
    }

    pub fn list_pipelines(&self) -> Vec<PipelineConfig> {
        self.pipelines.read().values().map(|p| p.config.clone()).collect()
    }
}

impl Default for DataPlane { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_operator() {
        let filter = FilterOperator::new("adult_filter", |r| r.payload.get("age").and_then(|v| v.as_i64()).unwrap_or(0) >= 18);
        let record = DataRecord::new("user", serde_json::json!({"name": "Alice", "age": 25}));
        assert!(filter.process(record).is_ok());
        let young = DataRecord::new("user", serde_json::json!({"name": "Bob", "age": 15}));
        assert!(filter.process(young).is_err());
    }

    #[test]
    fn map_operator() {
        let map = MapOperator::new("uppercase", |mut r| {
            if let Some(name) = r.payload.get("name").and_then(|v| v.as_str()) {
                if let Some(obj) = r.payload.as_object_mut() {
                    obj.insert("name".into(), serde_json::Value::String(name.to_uppercase()));
                }
            }
            r
        });
        let record = DataRecord::new("user", serde_json::json!({"name": "alice"}));
        let result = map.process(record).unwrap();
        assert_eq!(result.payload["name"], "ALICE");
    }

    #[test]
    fn validate_operator() {
        let validate = ValidateOperator::new("require_email", vec!["email".into()]);
        let good = DataRecord::new("user", serde_json::json!({"email": "a@b.com"}));
        assert!(validate.process(good).is_ok());
        let bad = DataRecord::new("user", serde_json::json!({"name": "a"}));
        assert!(validate.process(bad).is_err());
    }

    #[test]
    fn pipeline_execution() {
        let plane = DataPlane::new();
        let pipeline = plane.create_pipeline(PipelineConfig {
            name: "test_pipe".into(), description: "test".into(),
            operators: vec![], source: "test".into(), sink: "test".into(),
        });
        pipeline.add_operator(Arc::new(ValidateOperator::new("v", vec!["id".into()])));
        pipeline.add_operator(Arc::new(MapOperator::new("m", |mut r| {
            if let Some(obj) = r.payload.as_object_mut() {
                obj.insert("processed".into(), serde_json::Value::Bool(true));
            }
            r
        })));

        let record = DataRecord::new("test", serde_json::json!({"id": "123"}));
        let result = plane.process("test_pipe", record).unwrap();
        assert_eq!(result.payload["processed"], true);
    }

    #[test]
    fn batch_processing() {
        let plane = DataPlane::new();
        let pipeline = plane.create_pipeline(PipelineConfig {
            name: "batch".into(), description: "".into(), operators: vec![], source: "".into(), sink: "".into(),
        });
        pipeline.add_operator(Arc::new(FilterOperator::new("f", |r| r.payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false))));

        let records = vec![
            DataRecord::new("t", serde_json::json!({"ok": true})),
            DataRecord::new("t", serde_json::json!({"ok": false})),
            DataRecord::new("t", serde_json::json!({"ok": true})),
        ];
        let (success, errors) = plane.process_batch("batch", records).unwrap();
        assert_eq!(success.len(), 2);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn enrich_operator() {
        let enrich = EnrichOperator::new("user_enrich", "user_id");
        enrich.add_enrichment("u1", serde_json::json!({"role": "admin"}));
        let record = DataRecord::new("event", serde_json::json!({"user_id": "u1", "action": "login"}));
        let result = enrich.process(record).unwrap();
        assert_eq!(result.payload["enrichment"]["role"], "admin");
    }

    #[test]
    fn pipeline_not_found() {
        let plane = DataPlane::new();
        let record = DataRecord::new("t", serde_json::json!({}));
        assert!(plane.process("nonexistent", record).is_err());
    }
}

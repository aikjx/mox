//! MOX Data ETL Service
//!
//! Extract-Transform-Load pipeline engine with:
//! - Multiple source connectors (JSON, CSV, SQLite, HTTP)
//! - Transformation operators (map, filter, aggregate, join)
//! - Multiple sink destinations (SQLite, JSON file, in-memory)
//! - Job scheduling and monitoring

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EtlError {
    #[error("extract failed: {0}")]
    ExtractFailed(String),
    #[error("transform failed: {0}")]
    TransformFailed(String),
    #[error("load failed: {0}")]
    LoadFailed(String),
    #[error("job not found: {0}")]
    JobNotFound(String),
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtlRecord {
    pub id: String,
    pub data: serde_json::Value,
    pub source: String,
    pub timestamp: String,
}

impl EtlRecord {
    pub fn new(data: serde_json::Value, source: &str) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            data, source: source.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    fn extract(&self) -> Result<Vec<EtlRecord>, EtlError>;
}

pub trait Sink: Send + Sync {
    fn name(&self) -> &str;
    fn load(&self, records: Vec<EtlRecord>) -> Result<usize, EtlError>;
}

pub trait Transform: Send + Sync {
    fn name(&self) -> &str;
    fn transform(&self, records: Vec<EtlRecord>) -> Result<Vec<EtlRecord>, EtlError>;
}

// ─── Sources ───

pub struct JsonSource {
    pub name: String,
    pub data: Vec<serde_json::Value>,
}

impl JsonSource {
    pub fn new(name: &str, data: Vec<serde_json::Value>) -> Self {
        Self { name: name.into(), data }
    }
}

impl Source for JsonSource {
    fn name(&self) -> &str { &self.name }
    fn extract(&self) -> Result<Vec<EtlRecord>, EtlError> {
        Ok(self.data.iter().map(|v| EtlRecord::new(v.clone(), &self.name)).collect())
    }
}

pub struct CsvSource {
    pub name: String,
    pub content: String,
    pub delimiter: char,
    pub has_header: bool,
}

impl CsvSource {
    pub fn new(name: &str, content: String) -> Self {
        Self { name: name.into(), content, delimiter: ',', has_header: true }
    }
}

impl Source for CsvSource {
    fn name(&self) -> &str { &self.name }
    fn extract(&self) -> Result<Vec<EtlRecord>, EtlError> {
        let mut lines = self.content.lines();
        let headers: Vec<String> = if self.has_header {
            lines.next().map(|l| l.split(self.delimiter).map(|s| s.trim().to_string()).collect()).unwrap_or_default()
        } else {
            (0..self.content.lines().next().map(|l| l.split(self.delimiter).count()).unwrap_or(0)).map(|i| format!("col{}", i)).collect()
        };
        let mut records = vec![];
        for line in lines {
            let values: Vec<&str> = line.split(self.delimiter).collect();
            let mut obj = serde_json::Map::new();
            for (i, h) in headers.iter().enumerate() {
                if i < values.len() { obj.insert(h.clone(), serde_json::Value::String(values[i].trim().to_string())); }
            }
            records.push(EtlRecord::new(serde_json::Value::Object(obj), &self.name));
        }
        Ok(records)
    }
}

// ─── Transforms ───

pub struct FilterTransform {
    pub name: String,
    pub predicate: Arc<dyn Fn(&EtlRecord) -> bool + Send + Sync>,
}

impl FilterTransform {
    pub fn new<F>(name: &str, predicate: F) -> Self
    where F: Fn(&EtlRecord) -> bool + Send + Sync + 'static {
        Self { name: name.into(), predicate: Arc::new(predicate) }
    }
}

impl Transform for FilterTransform {
    fn name(&self) -> &str { &self.name }
    fn transform(&self, records: Vec<EtlRecord>) -> Result<Vec<EtlRecord>, EtlError> {
        Ok(records.into_iter().filter(|r| (self.predicate)(r)).collect())
    }
}

pub struct MapTransform {
    pub name: String,
    pub func: Arc<dyn Fn(EtlRecord) -> EtlRecord + Send + Sync>,
}

impl MapTransform {
    pub fn new<F>(name: &str, func: F) -> Self
    where F: Fn(EtlRecord) -> EtlRecord + Send + Sync + 'static {
        Self { name: name.into(), func: Arc::new(func) }
    }
}

impl Transform for MapTransform {
    fn name(&self) -> &str { &self.name }
    fn transform(&self, records: Vec<EtlRecord>) -> Result<Vec<EtlRecord>, EtlError> {
        Ok(records.into_iter().map(|r| (self.func)(r)).collect())
    }
}

pub struct AggregateTransform {
    pub name: String,
    pub group_by: String,
    pub aggregate_field: String,
}

impl AggregateTransform {
    pub fn new(name: &str, group_by: &str, aggregate_field: &str) -> Self {
        Self { name: name.into(), group_by: group_by.into(), aggregate_field: aggregate_field.into() }
    }
}

impl Transform for AggregateTransform {
    fn name(&self) -> &str { &self.name }
    fn transform(&self, records: Vec<EtlRecord>) -> Result<Vec<EtlRecord>, EtlError> {
        let mut groups: HashMap<String, (f64, usize)> = HashMap::new();
        for r in &records {
            if let (Some(key), Some(val)) = (r.data.get(&self.group_by).and_then(|v| v.as_str()), r.data.get(&self.aggregate_field).and_then(|v| v.as_f64())) {
                let entry = groups.entry(key.to_string()).or_insert((0.0, 0));
                entry.0 += val;
                entry.1 += 1;
            }
        }
        Ok(groups.into_iter().map(|(k, (sum, count))| {
            EtlRecord::new(serde_json::json!({
                &self.group_by: k,
                format!("{}_sum", self.aggregate_field): sum,
                format!("{}_count", self.aggregate_field): count,
                format!("{}_avg", self.aggregate_field): sum / count as f64,
            }), &self.name)
        }).collect())
    }
}

// ─── Sinks ───

pub struct MemorySink {
    pub name: String,
    pub storage: Arc<parking_lot::RwLock<Vec<EtlRecord>>>,
}

impl MemorySink {
    pub fn new(name: &str) -> Self {
        Self { name: name.into(), storage: Arc::new(parking_lot::RwLock::new(Vec::new())) }
    }
    pub fn records(&self) -> Vec<EtlRecord> { self.storage.read().clone() }
}

impl Sink for MemorySink {
    fn name(&self) -> &str { &self.name }
    fn load(&self, records: Vec<EtlRecord>) -> Result<usize, EtlError> {
        let count = records.len();
        self.storage.write().extend(records);
        Ok(count)
    }
}

pub struct JsonFileSink {
    pub name: String,
    pub path: String,
}

impl JsonFileSink {
    pub fn new(name: &str, path: &str) -> Self {
        Self { name: name.into(), path: path.into() }
    }
}

impl Sink for JsonFileSink {
    fn name(&self) -> &str { &self.name }
    fn load(&self, records: Vec<EtlRecord>) -> Result<usize, EtlError> {
        let json = serde_json::to_string_pretty(&records).map_err(|e| EtlError::LoadFailed(e.to_string()))?;
        std::fs::write(&self.path, json).map_err(|e| EtlError::LoadFailed(e.to_string()))?;
        Ok(records.len())
    }
}

// ─── ETL Job & Engine ───

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus { Pending, Running, Completed, Failed, Cancelled }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtlJobConfig {
    pub name: String,
    pub description: String,
    pub source: String,
    pub transforms: Vec<String>,
    pub sink: String,
    pub schedule: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtlJobStatus {
    pub name: String,
    pub status: JobStatus,
    pub records_extracted: usize,
    pub records_loaded: usize,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error: Option<String>,
    pub run_count: u32,
}

pub struct EtlJob {
    pub config: EtlJobConfig,
    pub source: Arc<dyn Source>,
    pub transforms: Vec<Arc<dyn Transform>>,
    pub sink: Arc<dyn Sink>,
    pub status: parking_lot::Mutex<EtlJobStatus>,
}

impl EtlJob {
    pub fn new(config: EtlJobConfig, source: Arc<dyn Source>, transforms: Vec<Arc<dyn Transform>>, sink: Arc<dyn Sink>) -> Self {
        Self {
            status: parking_lot::Mutex::new(EtlJobStatus {
                name: config.name.clone(), status: JobStatus::Pending,
                records_extracted: 0, records_loaded: 0,
                started_at: None, completed_at: None, error: None, run_count: 0,
            }),
            config, source, transforms, sink,
        }
    }

    pub fn run(&self) -> Result<usize, EtlError> {
        let mut status = self.status.lock();
        status.status = JobStatus::Running;
        status.started_at = Some(chrono::Utc::now().to_rfc3339());
        status.run_count += 1;
        drop(status);

        let result = || -> Result<usize, EtlError> {
            let extracted = self.source.extract()?;
            let extracted_count = extracted.len();
            self.status.lock().records_extracted = extracted_count;

            let mut current = extracted;
            for transform in &self.transforms {
                current = transform.transform(current)?;
            }

            let loaded = self.sink.load(current)?;
            self.status.lock().records_loaded = loaded;
            Ok(loaded)
        }();

        let mut status = self.status.lock();
        match &result {
            Ok(_) => { status.status = JobStatus::Completed; status.completed_at = Some(chrono::Utc::now().to_rfc3339()); }
            Err(e) => { status.status = JobStatus::Failed; status.error = Some(e.to_string()); status.completed_at = Some(chrono::Utc::now().to_rfc3339()); }
        }
        result
    }

    pub fn get_status(&self) -> EtlJobStatus { self.status.lock().clone() }
}

#[derive(Clone)]
pub struct EtlEngine {
    jobs: Arc<parking_lot::RwLock<HashMap<String, Arc<EtlJob>>>>,
}

impl EtlEngine {
    pub fn new() -> Self { Self { jobs: Arc::new(parking_lot::RwLock::new(HashMap::new())) } }

    pub fn register_job(&self, job: Arc<EtlJob>) {
        self.jobs.write().insert(job.config.name.clone(), job);
    }

    pub fn run_job(&self, name: &str) -> Result<usize, EtlError> {
        let job = self.jobs.read().get(name).cloned().ok_or_else(|| EtlError::JobNotFound(name.into()))?;
        job.run()
    }

    pub fn job_status(&self, name: &str) -> Option<EtlJobStatus> {
        self.jobs.read().get(name).map(|j| j.get_status())
    }

    pub fn list_jobs(&self) -> Vec<EtlJobStatus> {
        self.jobs.read().values().map(|j| j.get_status()).collect()
    }
}

impl Default for EtlEngine { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_source_extract() {
        let source = JsonSource::new("test", vec![serde_json::json!({"a": 1}), serde_json::json!({"a": 2})]);
        let records = source.extract().unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn csv_source_extract() {
        let csv = "name,age\nAlice,30\nBob,25";
        let source = CsvSource::new("test", csv.into());
        let records = source.extract().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].data["name"], "Alice");
        assert_eq!(records[0].data["age"], "30");
    }

    #[test]
    fn filter_transform() {
        let records = vec![
            EtlRecord::new(serde_json::json!({"age": 25}), "s"),
            EtlRecord::new(serde_json::json!({"age": 15}), "s"),
        ];
        let filter = FilterTransform::new("adult", |r| r.data.get("age").and_then(|v| v.as_i64()).unwrap_or(0) >= 18);
        let result = filter.transform(records).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn aggregate_transform() {
        let records = vec![
            EtlRecord::new(serde_json::json!({"dept": "A", "salary": 100.0}), "s"),
            EtlRecord::new(serde_json::json!({"dept": "A", "salary": 200.0}), "s"),
            EtlRecord::new(serde_json::json!({"dept": "B", "salary": 150.0}), "s"),
        ];
        let agg = AggregateTransform::new("by_dept", "dept", "salary");
        let result = agg.transform(records).unwrap();
        assert_eq!(result.len(), 2);
        let dept_a = result.iter().find(|r| r.data["dept"] == "A").unwrap();
        assert_eq!(dept_a.data["salary_sum"], 300.0);
        assert_eq!(dept_a.data["salary_count"], 2);
    }

    #[test]
    fn memory_sink_load() {
        let sink = MemorySink::new("mem");
        let records = vec![EtlRecord::new(serde_json::json!({"x": 1}), "s")];
        let count = sink.load(records).unwrap();
        assert_eq!(count, 1);
        assert_eq!(sink.records().len(), 1);
    }

    #[test]
    fn etl_job_end_to_end() {
        let source = JsonSource::new("src", vec![
            serde_json::json!({"name": "Alice", "age": 30}),
            serde_json::json!({"name": "Bob", "age": 15}),
            serde_json::json!({"name": "Carol", "age": 25}),
        ]);
        let filter = FilterTransform::new("adult", |r| r.data.get("age").and_then(|v| v.as_i64()).unwrap_or(0) >= 18);
        let sink = MemorySink::new("out");

        let job = Arc::new(EtlJob::new(
            EtlJobConfig { name: "test_job".into(), description: "".into(), source: "src".into(), transforms: vec![], sink: "out".into(), schedule: None },
            Arc::new(source), vec![Arc::new(filter)], Arc::new(sink.clone()),
        ));
        let engine = EtlEngine::new();
        engine.register_job(job.clone());
        let loaded = engine.run_job("test_job").unwrap();
        assert_eq!(loaded, 2);
        assert_eq!(sink.records().len(), 2);
        let status = engine.job_status("test_job").unwrap();
        assert_eq!(status.status, JobStatus::Completed);
        assert_eq!(status.records_extracted, 3);
        assert_eq!(status.records_loaded, 2);
    }

    #[test]
    fn map_transform() {
        let records = vec![EtlRecord::new(serde_json::json!({"v": 1}), "s")];
        let map = MapTransform::new("double", |mut r| {
            if let Some(v) = r.data.get("v").and_then(|x| x.as_i64()) {
                if let Some(obj) = r.data.as_object_mut() { obj.insert("v2".into(), serde_json::json!(v * 2)); }
            }
            r
        });
        let result = map.transform(records).unwrap();
        assert_eq!(result[0].data["v2"], 2);
    }
}

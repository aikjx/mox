// =============================================================================
// 指标收集模块（Prometheus 格式）
// =============================================================================

use crate::{ObservabilityError, ObservabilityResult};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

// =============================================================================
// 指标类型
// =============================================================================

/// 指标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricType {
    /// 计数器（只增不减）
    Counter,
    /// 仪表盘（可增可减）
    Gauge,
    /// 直方图（分布统计）
    Histogram,
    /// 摘要（分位数统计）
    Summary,
}

impl MetricType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MetricType::Counter => "counter",
            MetricType::Gauge => "gauge",
            MetricType::Histogram => "histogram",
            MetricType::Summary => "summary",
        }
    }
}

/// 指标值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 当前值
    pub value: f64,
    /// 标签（维度）
    pub labels: BTreeMap<String, String>,
    /// 帮助文本
    pub help: Option<String>,
    /// 最后更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// =============================================================================
// 指标定义
// =============================================================================

/// 指标定义
#[derive(Debug, Clone)]
struct MetricDefinition {
    name: String,
    metric_type: MetricType,
    help: String,
    labels: Vec<String>,
}

// =============================================================================
// 指标注册表
// =============================================================================

/// 指标注册表
#[derive(Debug, Clone)]
pub struct MetricsRegistry {
    definitions: Arc<RwLock<BTreeMap<String, MetricDefinition>>>,
    values: Arc<RwLock<BTreeMap<String, MetricValue>>>,
    namespace: String,
}

impl MetricsRegistry {
    /// 创建新的指标注册表
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            definitions: Arc::new(RwLock::new(BTreeMap::new())),
            values: Arc::new(RwLock::new(BTreeMap::new())),
            namespace: namespace.into(),
        }
    }

    /// 注册计数器
    pub fn register_counter(
        &self,
        name: impl Into<String>,
        help: impl Into<String>,
        labels: Vec<String>,
    ) -> ObservabilityResult<()> {
        self.register(name, MetricType::Counter, help, labels)
    }

    /// 注册仪表盘
    pub fn register_gauge(
        &self,
        name: impl Into<String>,
        help: impl Into<String>,
        labels: Vec<String>,
    ) -> ObservabilityResult<()> {
        self.register(name, MetricType::Gauge, help, labels)
    }

    /// 注册直方图
    pub fn register_histogram(
        &self,
        name: impl Into<String>,
        help: impl Into<String>,
        labels: Vec<String>,
    ) -> ObservabilityResult<()> {
        self.register(name, MetricType::Histogram, help, labels)
    }

    /// 注册指标
    fn register(
        &self,
        name: impl Into<String>,
        metric_type: MetricType,
        help: impl Into<String>,
        labels: Vec<String>,
    ) -> ObservabilityResult<()> {
        let name = format!("{}_{}", self.namespace, name.into());
        let def = MetricDefinition {
            name: name.clone(),
            metric_type,
            help: help.into(),
            labels,
        };

        let mut definitions = self.definitions.write();
        if definitions.contains_key(&name) {
            return Err(ObservabilityError::MetricRegistrationFailed(format!(
                "指标 '{}' 已注册",
                name
            )));
        }
        definitions.insert(name, def);
        Ok(())
    }

    /// 增加计数器
    pub fn increment_counter(
        &self,
        name: impl Into<String>,
        labels: BTreeMap<String, String>,
        value: f64,
    ) -> ObservabilityResult<()> {
        let name = format!("{}_{}", self.namespace, name.into());
        let key = self.metric_key(&name, &labels);

        let mut values = self.values.write();
        let metric = values.entry(key).or_insert_with(|| MetricValue {
            name: name.clone(),
            metric_type: MetricType::Counter,
            value: 0.0,
            labels: labels.clone(),
            help: None,
            updated_at: chrono::Utc::now(),
        });

        metric.value += value;
        metric.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// 设置仪表盘值
    pub fn set_gauge(
        &self,
        name: impl Into<String>,
        labels: BTreeMap<String, String>,
        value: f64,
    ) -> ObservabilityResult<()> {
        let name = format!("{}_{}", self.namespace, name.into());
        let key = self.metric_key(&name, &labels);

        let mut values = self.values.write();
        let metric = values.entry(key).or_insert_with(|| MetricValue {
            name: name.clone(),
            metric_type: MetricType::Gauge,
            value: 0.0,
            labels: labels.clone(),
            help: None,
            updated_at: chrono::Utc::now(),
        });

        metric.value = value;
        metric.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// 记录直方图观测值
    pub fn observe_histogram(
        &self,
        name: impl Into<String>,
        labels: BTreeMap<String, String>,
        value: f64,
    ) -> ObservabilityResult<()> {
        // 简化实现：记录为 gauge，实际应使用直方图桶
        self.set_gauge(name, labels, value)
    }

    /// 获取指标值
    pub fn get_metric(&self, name: &str, labels: &BTreeMap<String, String>) -> Option<MetricValue> {
        let name = format!("{}_{}", self.namespace, name);
        let key = self.metric_key(&name, labels);
        self.values.read().get(&key).cloned()
    }

    /// 获取所有指标
    pub fn get_all_metrics(&self) -> Vec<MetricValue> {
        self.values.read().values().cloned().collect()
    }

    /// 导出为 Prometheus 文本格式
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();
        let values = self.values.read();
        let definitions = self.definitions.read();

        for (name, def) in definitions.iter() {
            output.push_str(&format!("# HELP {} {}\n", name, def.help));
            output.push_str(&format!("# TYPE {} {}\n", name, def.metric_type.as_str()));

            for metric in values.values() {
                if metric.name == *name {
                    let labels_str: Vec<String> = metric
                        .labels
                        .iter()
                        .map(|(k, v)| format!("{}=\"{}\"", k, v))
                        .collect();

                    if labels_str.is_empty() {
                        output.push_str(&format!("{} {}\n", name, metric.value));
                    } else {
                        output.push_str(&format!(
                            "{}{{{}}} {}\n",
                            name,
                            labels_str.join(","),
                            metric.value
                        ));
                    }
                }
            }
        }

        output
    }

    /// 生成指标键（名称 + 标签哈希）
    fn metric_key(&self, name: &str, labels: &BTreeMap<String, String>) -> String {
        let labels_str: Vec<String> = labels
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        format!("{}|{}", name, labels_str.join("|"))
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new("mox")
    }
}

// =============================================================================
// 指标收集器（便捷 API）
// =============================================================================

/// 指标收集器
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    registry: MetricsRegistry,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            registry: MetricsRegistry::new(namespace),
            start_time: Instant::now(),
        }
    }

    /// 记录请求
    pub fn record_request(&self, method: &str, path: &str, status: u16, duration_ms: u64) {
        let mut labels = BTreeMap::new();
        labels.insert("method".to_string(), method.to_string());
        labels.insert("path".to_string(), path.to_string());
        labels.insert("status".to_string(), status.to_string());

        let _ = self.registry.increment_counter("http_requests_total", labels.clone(), 1.0);
        let _ = self.registry.observe_histogram("http_request_duration_ms", labels, duration_ms as f64);
    }

    /// 记录任务完成
    pub fn record_task_completed(&self, task_type: &str, success: bool, duration_ms: u64) {
        let mut labels = BTreeMap::new();
        labels.insert("type".to_string(), task_type.to_string());
        labels.insert("result".to_string(), if success { "success".to_string() } else { "failure".to_string() });

        let _ = self.registry.increment_counter("tasks_total", labels.clone(), 1.0);
        let _ = self.registry.observe_histogram("task_duration_ms", labels, duration_ms as f64);
    }

    /// 设置活跃连接数
    pub fn set_active_connections(&self, count: u64) {
        let _ = self.registry.set_gauge("active_connections", BTreeMap::new(), count as f64);
    }

    /// 设置队列深度
    pub fn set_queue_depth(&self, queue: &str, depth: u64) {
        let mut labels = BTreeMap::new();
        labels.insert("queue".to_string(), queue.to_string());
        let _ = self.registry.set_gauge("queue_depth", labels, depth as f64);
    }

    /// 获取运行时间（秒）
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// 获取注册表
    pub fn registry(&self) -> &MetricsRegistry {
        &self.registry
    }

    /// 导出 Prometheus 格式
    pub fn export(&self) -> String {
        self.registry.export_prometheus()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new("mox")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_increment_counter() {
        let registry = MetricsRegistry::new("test");
        registry
            .register_counter("requests_total", "总请求数", vec!["method".to_string()])
            .unwrap();

        let mut labels = BTreeMap::new();
        labels.insert("method".to_string(), "GET".to_string());

        registry.increment_counter("requests_total", labels.clone(), 1.0).unwrap();
        registry.increment_counter("requests_total", labels.clone(), 2.0).unwrap();

        let metric = registry.get_metric("requests_total", &labels).unwrap();
        assert_eq!(metric.value, 3.0);
        assert_eq!(metric.metric_type, MetricType::Counter);
    }

    #[test]
    fn test_set_gauge() {
        let registry = MetricsRegistry::new("test");
        registry
            .register_gauge("active_connections", "活跃连接数", vec![])
            .unwrap();

        registry.set_gauge("active_connections", BTreeMap::new(), 42.0).unwrap();

        let metric = registry.get_metric("active_connections", &BTreeMap::new()).unwrap();
        assert_eq!(metric.value, 42.0);
    }

    #[test]
    fn test_duplicate_registration_fails() {
        let registry = MetricsRegistry::new("test");
        registry.register_counter("test_metric", "测试", vec![]).unwrap();
        assert!(registry.register_counter("test_metric", "测试", vec![]).is_err());
    }

    #[test]
    fn test_export_prometheus() {
        let registry = MetricsRegistry::new("test");
        registry
            .register_counter("requests_total", "总请求数", vec!["method".to_string()])
            .unwrap();

        let mut labels = BTreeMap::new();
        labels.insert("method".to_string(), "GET".to_string());
        registry.increment_counter("requests_total", labels, 5.0).unwrap();

        let output = registry.export_prometheus();
        assert!(output.contains("# HELP test_requests_total 总请求数"));
        assert!(output.contains("# TYPE test_requests_total counter"));
        assert!(output.contains("test_requests_total{method=\"GET\"} 5"));
    }

    #[test]
    fn test_metrics_collector_request() {
        let collector = MetricsCollector::new("app");
        collector.record_request("GET", "/api/v1/tasks", 200, 150);

        let metrics = collector.registry().get_all_metrics();
        assert!(!metrics.is_empty());
    }

    #[test]
    fn test_metrics_collector_task() {
        let collector = MetricsCollector::new("app");
        collector.record_task_completed("alliance", true, 5000);

        let metrics = collector.registry().get_all_metrics();
        assert!(!metrics.is_empty());
    }

    #[test]
    fn test_uptime() {
        let collector = MetricsCollector::new("app");
        assert!(collector.uptime_seconds() >= 0);
    }

    #[test]
    fn test_metric_type_as_str() {
        assert_eq!(MetricType::Counter.as_str(), "counter");
        assert_eq!(MetricType::Gauge.as_str(), "gauge");
        assert_eq!(MetricType::Histogram.as_str(), "histogram");
        assert_eq!(MetricType::Summary.as_str(), "summary");
    }
}

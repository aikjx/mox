// =============================================================================
// 结构化日志模块
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

// =============================================================================
// 日志级别
// =============================================================================

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
            LogLevel::Fatal => "fatal",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "trace" => Some(LogLevel::Trace),
            "debug" => Some(LogLevel::Debug),
            "info" => Some(LogLevel::Info),
            "warn" | "warning" => Some(LogLevel::Warn),
            "error" => Some(LogLevel::Error),
            "fatal" | "critical" => Some(LogLevel::Fatal),
            _ => None,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// 日志条目
// =============================================================================

/// 结构化日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// 时间戳（RFC3339）
    pub timestamp: String,
    /// 日志级别
    pub level: LogLevel,
    /// 服务名称
    pub service: String,
    /// 消息
    pub message: String,
    /// 追踪 ID
    pub trace_id: Option<String>,
    /// Span ID
    pub span_id: Option<String>,
    /// 结构化字段
    pub fields: BTreeMap<String, String>,
}

impl LogEntry {
    pub fn new(level: LogLevel, service: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level,
            service: service.into(),
            message: message.into(),
            trace_id: None,
            span_id: None,
            fields: BTreeMap::new(),
        }
    }

    pub fn with_trace(mut self, trace_id: impl Into<String>, span_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self.span_id = Some(span_id.into());
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// 序列化为 JSON 行
    pub fn to_json_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| format!("{{\"message\":\"{}\"}}", self.message))
    }

    /// 格式化为人类可读的文本
    pub fn to_text(&self) -> String {
        let trace = match (&self.trace_id, &self.span_id) {
            (Some(t), Some(s)) => format!(" [{}:{}]", t, s),
            _ => String::new(),
        };
        let fields: Vec<String> = self
            .fields
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let fields_str = if fields.is_empty() {
            String::new()
        } else {
            format!(" {}", fields.join(" "))
        };

        format!(
            "{} [{}] {}{}: {}{}",
            self.timestamp, self.level, self.service, trace, self.message, fields_str
        )
    }
}

// =============================================================================
// 日志收集器
// =============================================================================

/// 日志收集器（内存缓冲，用于测试和调试）
#[derive(Debug, Clone)]
pub struct LogCollector {
    entries: Arc<parking_lot::RwLock<Vec<LogEntry>>>,
    max_entries: usize,
    min_level: LogLevel,
}

impl LogCollector {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(parking_lot::RwLock::new(Vec::new())),
            max_entries,
            min_level: LogLevel::Debug,
        }
    }

    pub fn with_min_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    pub fn log(&self, entry: LogEntry) {
        if entry.level < self.min_level {
            return;
        }

        let mut entries = self.entries.write();
        entries.push(entry);
        if entries.len() > self.max_entries {
            let overflow = entries.len() - self.max_entries;
            entries.drain(0..overflow);
        }
    }

    pub fn get_entries(&self) -> Vec<LogEntry> {
        self.entries.read().clone()
    }

    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    pub fn clear(&self) {
        self.entries.write().clear();
    }

    /// 按级别过滤
    pub fn filter_by_level(&self, level: LogLevel) -> Vec<LogEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.level == level)
            .cloned()
            .collect()
    }

    /// 导出为 JSON 行
    pub fn export_json_lines(&self) -> String {
        self.entries
            .read()
            .iter()
            .map(|e| e.to_json_line())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for LogCollector {
    fn default() -> Self {
        Self::new(1000)
    }
}

// =============================================================================
// 结构化日志记录器
// =============================================================================

/// 结构化日志记录器
#[derive(Debug, Clone)]
pub struct StructuredLogger {
    service: String,
    collector: Option<LogCollector>,
    min_level: LogLevel,
    output_json: bool,
}

impl StructuredLogger {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            collector: None,
            min_level: LogLevel::Info,
            output_json: false,
        }
    }

    pub fn with_collector(mut self, collector: LogCollector) -> Self {
        self.collector = Some(collector);
        self
    }

    pub fn with_min_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    pub fn with_json_output(mut self) -> Self {
        self.output_json = true;
        self
    }

    pub fn log(&self, level: LogLevel, message: impl Into<String>) {
        if level < self.min_level {
            return;
        }

        let entry = LogEntry::new(level, &self.service, message);

        // 输出到 stdout
        if self.output_json {
            println!("{}", entry.to_json_line());
        } else {
            eprintln!("{}", entry.to_text());
        }

        // 收集到内存
        if let Some(collector) = &self.collector {
            collector.log(entry);
        }
    }

    pub fn trace(&self, message: impl Into<String>) {
        self.log(LogLevel::Trace, message);
    }

    pub fn debug(&self, message: impl Into<String>) {
        self.log(LogLevel::Debug, message);
    }

    pub fn info(&self, message: impl Into<String>) {
        self.log(LogLevel::Info, message);
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.log(LogLevel::Warn, message);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.log(LogLevel::Error, message);
    }

    pub fn fatal(&self, message: impl Into<String>) {
        self.log(LogLevel::Fatal, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Fatal);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("WARNING"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::from_str("error"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("invalid"), None);
    }

    #[test]
    fn test_log_entry_json() {
        let entry = LogEntry::new(LogLevel::Info, "test-service", "test message")
            .with_field("key", "value");

        let json = entry.to_json_line();
        assert!(json.contains("test message"));
        assert!(json.contains("test-service"));
        assert!(json.contains("info"));
    }

    #[test]
    fn test_log_entry_text() {
        let entry = LogEntry::new(LogLevel::Error, "test-service", "something failed")
            .with_field("code", "500");

        let text = entry.to_text();
        assert!(text.contains("something failed"));
        assert!(text.contains("error"));
        assert!(text.contains("code=500"));
    }

    #[test]
    fn test_log_collector() {
        let collector = LogCollector::new(100);
        let logger = StructuredLogger::new("test").with_collector(collector.clone());

        logger.info("message 1");
        logger.warn("message 2");
        logger.error("message 3");

        assert_eq!(collector.len(), 3);
        assert_eq!(collector.filter_by_level(LogLevel::Error).len(), 1);
        assert_eq!(collector.filter_by_level(LogLevel::Warn).len(), 1);
    }

    #[test]
    fn test_log_collector_max_entries() {
        let collector = LogCollector::new(5);
        let logger = StructuredLogger::new("test").with_collector(collector.clone());

        for i in 0..10 {
            logger.info(format!("message {}", i));
        }

        assert_eq!(collector.len(), 5);
    }

    #[test]
    fn test_log_collector_min_level() {
        let collector = LogCollector::new(100).with_min_level(LogLevel::Warn);
        let logger = StructuredLogger::new("test")
            .with_collector(collector.clone())
            .with_min_level(LogLevel::Warn);

        logger.info("should not be collected");
        logger.warn("should be collected");

        assert_eq!(collector.len(), 1);
    }

    #[test]
    fn test_log_entry_with_trace() {
        let entry = LogEntry::new(LogLevel::Info, "test", "msg")
            .with_trace("trace-123", "span-456");

        assert_eq!(entry.trace_id.as_deref(), Some("trace-123"));
        assert_eq!(entry.span_id.as_deref(), Some("span-456"));
    }

    #[test]
    fn test_structured_logger_levels() {
        let collector = LogCollector::new(100).with_min_level(LogLevel::Trace);
        let logger = StructuredLogger::new("test")
            .with_collector(collector.clone())
            .with_min_level(LogLevel::Trace);

        logger.trace("trace");
        logger.debug("debug");
        logger.info("info");
        logger.warn("warn");
        logger.error("error");
        logger.fatal("fatal");

        assert_eq!(collector.len(), 6);
    }
}

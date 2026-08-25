//! # Mox 8-Stage Trace 埋点模块
//!
//! 对应 Mox 写入与查询流水线的 8 个阶段：
//! 1. `Emit`           — 客户端发出事件 / 请求入口
//! 2. `CdcNext`        — CDC 变更捕获 / 游标推进
//! 3. `Dedup`          — 基于 Bloom Filter + 主键哈希去重
//! 4. `SparkWrite`     — Spark 桥批量写入 / shuffle 聚合
//! 5. `Projection`     — 投影算子下推 / 物化视图同步
//! 6. `Audit`          — 合规审计与字段签名校验
//! 7. `CircuitBreaker` — 熔断 / 限流 / 质量关卡
//! 8. `Sink`           — 最终下沉落地 (StorageEngine / MetaStore)
//!
//! 设计目标：
//! - 零第三方依赖，纯 `std` 实现，方便内嵌 `mox-graph-service`。
//! - 线程安全：`Tracer` 使用 `Mutex<Vec<Span>>` + `AtomicUsize`。
//! - 低开销：单次 `emit_span` 分配 ~ O(attrs.len())。
//! - 可观测对接：`export_json()` 输出 Grafana / OTLP JSON 兼容数组。

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::result::Result;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// 阶段枚举
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraceStage {
    Emit,
    CdcNext,
    Dedup,
    SparkWrite,
    Projection,
    Audit,
    CircuitBreaker,
    Sink,
}

impl TraceStage {
    pub const ALL: [TraceStage; 8] = [
        TraceStage::Emit,
        TraceStage::CdcNext,
        TraceStage::Dedup,
        TraceStage::SparkWrite,
        TraceStage::Projection,
        TraceStage::Audit,
        TraceStage::CircuitBreaker,
        TraceStage::Sink,
    ];

    pub const STAGE_INDEX_STRS: [&'static str; 8] = ["0", "1", "2", "3", "4", "5", "6", "7"];

    pub fn as_str(&self) -> &'static str {
        match self {
            TraceStage::Emit => "Emit",
            TraceStage::CdcNext => "CdcNext",
            TraceStage::Dedup => "Dedup",
            TraceStage::SparkWrite => "SparkWrite",
            TraceStage::Projection => "Projection",
            TraceStage::Audit => "Audit",
            TraceStage::CircuitBreaker => "CircuitBreaker",
            TraceStage::Sink => "Sink",
        }
    }

    pub fn index(&self) -> u8 {
        match self {
            TraceStage::Emit => 0,
            TraceStage::CdcNext => 1,
            TraceStage::Dedup => 2,
            TraceStage::SparkWrite => 3,
            TraceStage::Projection => 4,
            TraceStage::Audit => 5,
            TraceStage::CircuitBreaker => 6,
            TraceStage::Sink => 7,
        }
    }
}

impl fmt::Display for TraceStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// 错误类型
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceError {
    PoisonedMutex,
    InvalidTraceId,
    InvalidSpanId,
}

impl fmt::Display for TraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraceError::PoisonedMutex => write!(f, "tracer mutex poisoned"),
            TraceError::InvalidTraceId => write!(f, "trace_id must be non-empty"),
            TraceError::InvalidSpanId => write!(f, "span_id generation failed"),
        }
    }
}

impl std::error::Error for TraceError {}

// ---------------------------------------------------------------------------
// Span 结构
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub stage: TraceStage,
    pub start_ms: u64,
    pub end_ms: u64,
    pub attrs: HashMap<String, String>,
}

impl Span {
    pub fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }
}

// ---------------------------------------------------------------------------
// Tracer 核心
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Tracer {
    spans: Mutex<Vec<Span>>,
    span_count: AtomicUsize,
}

impl Tracer {
    pub fn new() -> Self {
        Self::default()
    }

    /// 发射 span。attrs 参数为 BTreeMap<&str, &str>。
    pub fn emit_span<E: From<TraceError>>(
        &self,
        stage: TraceStage,
        trace_id: &str,
        attrs: BTreeMap<&str, &str>,
    ) -> Result<(), E> {
        if trace_id.is_empty() {
            return Err(TraceError::InvalidTraceId.into());
        }

        let now_ms = unix_ms();
        let start_ms = now_ms;
        let end_ms = now_ms.saturating_add(1);

        let span_id = generate_span_id(self.span_count.load(Ordering::Relaxed), trace_id);

        let mut map = HashMap::with_capacity(attrs.len());
        for (k, v) in attrs {
            map.insert(k.to_string(), v.to_string());
        }

        let span = Span {
            trace_id: trace_id.to_string(),
            span_id,
            stage,
            start_ms,
            end_ms,
            attrs: map,
        };

        let mut guard = self
            .spans
            .lock()
            .map_err(|_| TraceError::PoisonedMutex)?;
        guard.push(span);
        self.span_count.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    pub fn span_count(&self) -> usize {
        self.span_count.load(Ordering::SeqCst)
    }

    pub fn len(&self) -> Result<usize, TraceError> {
        let guard = self.spans.lock().map_err(|_| TraceError::PoisonedMutex)?;
        Ok(guard.len())
    }

    pub fn is_empty(&self) -> Result<bool, TraceError> {
        Ok(self.len()? == 0)
    }

    pub fn clear(&self) -> Result<(), TraceError> {
        let mut guard = self.spans.lock().map_err(|_| TraceError::PoisonedMutex)?;
        guard.clear();
        self.span_count.store(0, Ordering::SeqCst);
        Ok(())
    }

    pub fn export_json(&self) -> String {
        let guard = match self.spans.lock() {
            Ok(g) => g,
            Err(_) => return String::from("[]"),
        };

        let mut out = String::with_capacity(64 + guard.len() * 256);
        out.push('[');
        for (i, span) in guard.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            push_escaped_kv(&mut out, "trace_id", &span.trace_id, true);
            push_escaped_kv(&mut out, "span_id", &span.span_id, true);
            push_escaped_kv(&mut out, "stage", span.stage.as_str(), true);
            out.push_str("\"start_ms\":");
            out.push_str(&span.start_ms.to_string());
            out.push(',');
            out.push_str("\"end_ms\":");
            out.push_str(&span.end_ms.to_string());
            out.push(',');
            out.push_str("\"duration_ms\":");
            out.push_str(&span.duration_ms().to_string());
            out.push(',');
            out.push_str("\"attrs\":{");
            let mut sorted: Vec<(&String, &String)> = span.attrs.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(b.0));
            for (j, (k, v)) in sorted.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                push_escaped_kv(&mut out, k, v, false);
            }
            out.push_str("}}");
        }
        out.push(']');
        out
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn generate_span_id(count: usize, trace_id: &str) -> String {
    let mut hash: u32 = 2166136261u32;
    for b in trace_id.as_bytes() {
        hash ^= *b as u32;
        hash = hash.wrapping_mul(16777619u32);
    }
    let c = (count as u64) & 0xFFFFFFFFFF;
    format!("{:010x}-{:04x}", c, hash & 0xFFFF)
}

fn push_escaped_kv(out: &mut String, key: &str, value: &str, comma: bool) {
    if comma {
        out.push(',');
    }
    out.push('"');
    push_escaped(out, key);
    out.push_str("\":\"");
    push_escaped(out, value);
    out.push('"');
}

fn push_escaped(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const DIGIT_STRS: [&str; 16] = [
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
    ];

    fn fresh_attrs(stage: TraceStage) -> BTreeMap<&'static str, &'static str> {
        let mut m = BTreeMap::new();
        m.insert("host", "test-node-01");
        m.insert("stage_idx", TraceStage::STAGE_INDEX_STRS[stage.index() as usize]);
        m.insert("service", "mox-graph-service");
        m
    }

    /// [1/8] Stage Emit
    #[test]
    fn test_stage_emit() {
        let tracer = Tracer::new();
        let before = tracer.span_count();
        let _res: Result<(), TraceError> =
            tracer.emit_span(TraceStage::Emit, "trace-test-1", fresh_attrs(TraceStage::Emit));
        _res.unwrap();
        assert_eq!(tracer.span_count(), before + 1);
        assert_eq!(tracer.len().unwrap(), 1);
        let json = tracer.export_json();
        assert!(json.contains("\"stage\":\"Emit\""));
    }

    /// [2/8] Stage CdcNext
    #[test]
    fn test_stage_cdcnext() {
        let tracer = Tracer::new();
        let r: Result<(), TraceError> = tracer.emit_span(
            TraceStage::CdcNext,
            "trace-test-2",
            fresh_attrs(TraceStage::CdcNext),
        );
        r.unwrap();
        assert_eq!(tracer.span_count(), 1);
        assert!(tracer.export_json().contains("\"stage\":\"CdcNext\""));
    }

    /// [3/8] Stage Dedup
    #[test]
    fn test_stage_dedup() {
        let tracer = Tracer::new();
        let r: Result<(), TraceError> = tracer.emit_span(
            TraceStage::Dedup,
            "trace-test-3",
            fresh_attrs(TraceStage::Dedup),
        );
        r.unwrap();
        assert_eq!(tracer.span_count(), 1);
        assert!(tracer.export_json().contains("\"stage\":\"Dedup\""));
    }

    /// [4/8] Stage SparkWrite
    #[test]
    fn test_stage_sparkwrite() {
        let tracer = Tracer::new();
        let r: Result<(), TraceError> = tracer.emit_span(
            TraceStage::SparkWrite,
            "trace-test-4",
            fresh_attrs(TraceStage::SparkWrite),
        );
        r.unwrap();
        assert_eq!(tracer.span_count(), 1);
        assert!(tracer.export_json().contains("\"stage\":\"SparkWrite\""));
    }

    /// [5/8] Stage Projection
    #[test]
    fn test_stage_projection() {
        let tracer = Tracer::new();
        let r: Result<(), TraceError> = tracer.emit_span(
            TraceStage::Projection,
            "trace-test-5",
            fresh_attrs(TraceStage::Projection),
        );
        r.unwrap();
        assert_eq!(tracer.span_count(), 1);
        assert!(tracer.export_json().contains("\"stage\":\"Projection\""));
    }

    /// [6/8] Stage Audit
    #[test]
    fn test_stage_audit() {
        let tracer = Tracer::new();
        let r: Result<(), TraceError> = tracer.emit_span(
            TraceStage::Audit,
            "trace-test-6",
            fresh_attrs(TraceStage::Audit),
        );
        r.unwrap();
        assert_eq!(tracer.span_count(), 1);
        assert!(tracer.export_json().contains("\"stage\":\"Audit\""));
    }

    /// [7/8] Stage CircuitBreaker
    #[test]
    fn test_stage_circuitbreaker() {
        let tracer = Tracer::new();
        let r: Result<(), TraceError> = tracer.emit_span(
            TraceStage::CircuitBreaker,
            "trace-test-7",
            fresh_attrs(TraceStage::CircuitBreaker),
        );
        r.unwrap();
        assert_eq!(tracer.span_count(), 1);
        assert!(tracer.export_json().contains("\"stage\":\"CircuitBreaker\""));
    }

    /// [8/8] Stage Sink
    #[test]
    fn test_stage_sink() {
        let tracer = Tracer::new();
        let r: Result<(), TraceError> = tracer.emit_span(
            TraceStage::Sink,
            "trace-test-8",
            fresh_attrs(TraceStage::Sink),
        );
        r.unwrap();
        assert_eq!(tracer.span_count(), 1);
        assert!(tracer.export_json().contains("\"stage\":\"Sink\""));
    }

    /// 综合: 8 阶段各 emit 1 条 + export 全 8 条 + span_count==8 + 字段齐全
    #[test]
    fn test_all_8_stages_exported_and_fields_complete() {
        let tracer = Tracer::new();
        let trace_id = "trace-all-8";
        for stage in TraceStage::ALL.iter() {
            let mut attrs: BTreeMap<&str, &str> = BTreeMap::new();
            attrs.insert("user", "alice");
            attrs.insert("op", stage.as_str());
            let r: Result<(), TraceError> = tracer.emit_span(*stage, trace_id, attrs);
            assert!(r.is_ok(), "emit span for {:?} failed", stage);
        }

        assert_eq!(tracer.span_count(), 8);
        assert_eq!(tracer.len().unwrap(), 8);

        let json = tracer.export_json();

        for stage in TraceStage::ALL.iter() {
            let needle = format!("\"stage\":\"{}\"", stage.as_str());
            assert!(
                json.contains(&needle),
                "JSON missing stage={}: {}",
                stage,
                &json
            );
        }

        for field in &[
            "\"trace_id\"",
            "\"span_id\"",
            "\"start_ms\"",
            "\"end_ms\"",
            "\"duration_ms\"",
            "\"attrs\"",
        ] {
            let n = byte_count(&json, field);
            assert!(n >= 8, "field {} occurrences {} < 8", field, n);
        }

        let tid_count = byte_count(&json, trace_id);
        assert!(tid_count >= 8, "trace_id {} count {} < 8", trace_id, tid_count);
    }

    /// span_id 唯一性
    #[test]
    fn test_span_ids_are_unique() {
        let tracer = Tracer::new();
        for _ in 0..100 {
            let attrs: BTreeMap<&str, &str> = BTreeMap::new();
            let r: Result<(), TraceError> =
                tracer.emit_span(TraceStage::Emit, "same-trace", attrs);
            r.unwrap();
        }
        assert_eq!(tracer.span_count(), 100);
        let json = tracer.export_json();
        let ids = extract_json_string_fields(&json, "span_id");
        let mut dedup = ids.clone();
        dedup.sort();
        dedup.dedup();
        assert_eq!(ids.len(), dedup.len(), "duplicate span_id detected");
    }

    /// 空 trace_id 报错
    #[test]
    fn test_invalid_trace_id_returns_error() {
        let tracer = Tracer::new();
        let attrs: BTreeMap<&str, &str> = BTreeMap::new();
        let res: Result<(), TraceError> = tracer.emit_span(TraceStage::Emit, "", attrs);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), TraceError::InvalidTraceId);
        assert_eq!(tracer.span_count(), 0);
    }

    /// clear 清零
    #[test]
    fn test_clear_resets_count() {
        let tracer = Tracer::new();
        for (i, s) in TraceStage::ALL.iter().enumerate() {
            let mut a: BTreeMap<&str, &str> = BTreeMap::new();
            a.insert("i", DIGIT_STRS[i]);
            a.insert("s", s.as_str());
            let r: Result<(), TraceError> = tracer.emit_span(*s, "t-clear", a);
            r.unwrap();
        }
        assert_eq!(tracer.span_count(), 8);
        tracer.clear().unwrap();
        assert_eq!(tracer.span_count(), 0);
        assert!(tracer.is_empty().unwrap());
        assert_eq!(tracer.export_json(), "[]");
    }

    /// 泛型错误转换: 自定义 MyErr: From<TraceError>
    #[derive(Debug)]
    enum MyErr {
        Trace(TraceError),
    }
    impl From<TraceError> for MyErr {
        fn from(e: TraceError) -> Self {
            MyErr::Trace(e)
        }
    }
    #[test]
    fn test_emit_span_generic_error_conversion() {
        let tracer = Tracer::new();
        let attrs: BTreeMap<&str, &str> = BTreeMap::new();
        let r: Result<(), MyErr> = tracer.emit_span(TraceStage::Audit, "ok-1", attrs);
        assert!(r.is_ok());
        let empty: BTreeMap<&str, &str> = BTreeMap::new();
        let r2: Result<(), MyErr> = tracer.emit_span(TraceStage::Audit, "", empty);
        assert!(matches!(r2, Err(MyErr::Trace(TraceError::InvalidTraceId))));
    }

    // ---- helpers ----

    fn byte_count(haystack: &str, needle: &str) -> usize {
        haystack.matches(needle).count()
    }

    fn extract_json_string_fields(json: &str, field: &str) -> Vec<String> {
        let pattern = format!("\"{}\":\"", field);
        let mut out = Vec::new();
        let bytes = json.as_bytes();
        let pb = pattern.as_bytes();
        let mut i = 0;
        while i + pb.len() <= bytes.len() {
            if bytes[i..i + pb.len()] == *pb {
                let start = i + pb.len();
                let mut j = start;
                while j < bytes.len() {
                    if bytes[j] == b'"' && (j == 0 || bytes[j - 1] != b'\\') {
                        break;
                    }
                    j += 1;
                }
                out.push(json[start..j].to_string());
                i = j + 1;
            } else {
                i += 1;
            }
        }
        out
    }
}

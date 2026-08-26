//! Distributed tracing context propagation.
//!
//! Provides trace_id/span_id generation and propagation across async boundaries.
//! Uses UUID v7 for time-ordered trace IDs.

use std::sync::Arc;
use tokio::task_local;

task_local! {
    static TRACE_CONTEXT: Arc<TraceContext>;
}

#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
    pub sampled: bool,
}

impl TraceContext {
    /// Create a new root trace context.
    pub fn new_root(service_name: &str) -> Self {
        Self {
            trace_id: uuid::Uuid::now_v7().to_string(),
            span_id: uuid::Uuid::now_v7().to_string()[..16].to_string(),
            parent_span_id: None,
            service_name: service_name.to_string(),
            sampled: true,
        }
    }

    /// Create a child span from this context.
    pub fn child_span(&self, service_name: &str) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: uuid::Uuid::now_v7().to_string()[..16].to_string(),
            parent_span_id: Some(self.span_id.clone()),
            service_name: service_name.to_string(),
            sampled: self.sampled,
        }
    }

    /// Parse trace context from HTTP headers (W3C TraceContext compatible subset).
    pub fn from_headers(headers: &http::HeaderMap, service_name: &str) -> Self {
        if let Some(traceparent) = headers.get("traceparent").and_then(|v| v.to_str().ok()) {
            // W3C format: version-traceid-spanid-traceflags
            let parts: Vec<&str> = traceparent.split('-').collect();
            if parts.len() >= 4 {
                return Self {
                    trace_id: parts[1].to_string(),
                    span_id: uuid::Uuid::now_v7().to_string()[..16].to_string(),
                    parent_span_id: Some(parts[2].to_string()),
                    service_name: service_name.to_string(),
                    sampled: parts[3] == "01",
                };
            }
        }
        // Fallback: x-trace-id header
        if let Some(trace_id) = headers.get("x-trace-id").and_then(|v| v.to_str().ok()) {
            return Self {
                trace_id: trace_id.to_string(),
                span_id: uuid::Uuid::now_v7().to_string()[..16].to_string(),
                parent_span_id: None,
                service_name: service_name.to_string(),
                sampled: true,
            };
        }
        Self::new_root(service_name)
    }

    /// Serialize to W3C traceparent header value.
    pub fn to_traceparent(&self) -> String {
        format!(
            "00-{}-{}-{}",
            self.trace_id.replace('-', ""),
            self.span_id,
            if self.sampled { "01" } else { "00" }
        )
    }
}

/// Get the current trace_id from the task-local context, or generate a new one.
pub fn current_trace_id() -> String {
    TRACE_CONTEXT
        .try_with(|ctx| ctx.trace_id.clone())
        .unwrap_or_else(|_| uuid::Uuid::now_v7().to_string())
}

/// Execute a future with the given trace context.
pub async fn with_trace_context<F, T>(ctx: Arc<TraceContext>, f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    TRACE_CONTEXT.scope(ctx, f).await
}

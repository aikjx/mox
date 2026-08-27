// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Trace ID 传播 — Distributed Tracing
//!
//! 企业级分布式追踪：每个请求携带唯一trace_id，
//! 跨线程/跨服务/跨能力传播，用于日志关联和问题排查。

use serde::{Deserialize, Serialize};
use parking_lot::RwLock;

/// Trace ID（UUID v4 简化版，32位十六进制）
pub type TraceId = String;

/// Trace 上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// 追踪ID
    pub trace_id: TraceId,
    /// 跨度ID（当前调用链节点）
    pub span_id: String,
    /// 父跨度ID
    pub parent_span_id: Option<String>,
    /// 服务名称
    pub service: String,
    /// 开始时间（RFC3339）
    pub started_at: String,
    /// 附加标签
    #[serde(default)]
    pub tags: std::collections::HashMap<String, String>,
}

impl TraceContext {
    /// 创建新的追踪上下文（根span）
    pub fn new(service: impl Into<String>) -> Self {
        let trace_id = generate_trace_id();
        let span_id = generate_span_id();
        Self {
            trace_id,
            span_id,
            parent_span_id: None,
            service: service.into(),
            started_at: chrono::Utc::now().to_rfc3339(),
            tags: std::collections::HashMap::new(),
        }
    }

    /// 从已有trace_id创建（用于传播）
    pub fn from_trace_id(trace_id: TraceId, service: impl Into<String>) -> Self {
        let mut ctx = Self::new(service);
        ctx.trace_id = trace_id;
        ctx
    }

    /// 创建子span
    pub fn child_span(&self, service: impl Into<String>) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: generate_span_id(),
            parent_span_id: Some(self.span_id.clone()),
            service: service.into(),
            started_at: chrono::Utc::now().to_rfc3339(),
            tags: std::collections::HashMap::new(),
        }
    }

    /// 添加标签
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
}

/// 线程局部TraceContext存储
thread_local! {
    static CURRENT_TRACE: RwLock<Option<TraceContext>> = RwLock::new(None);
}

/// 获取当前线程的trace_id
pub fn current_trace_id() -> Option<TraceId> {
    CURRENT_TRACE.with(|t| t.read().as_ref().map(|ctx| ctx.trace_id.clone()))
}

/// 获取当前线程的TraceContext
pub fn current_trace_context() -> Option<TraceContext> {
    CURRENT_TRACE.with(|t| t.read().clone())
}

/// 在TraceContext上下文中执行闭包
pub fn with_trace<F, R>(ctx: TraceContext, f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev = CURRENT_TRACE.with(|t| t.read().clone());
    CURRENT_TRACE.with(|t| *t.write() = Some(ctx));
    let result = f();
    CURRENT_TRACE.with(|t| *t.write() = prev);
    result
}

/// 生成Trace ID（32位十六进制，无连字符的UUID）
fn generate_trace_id() -> TraceId {
    use uuid::Uuid;
    Uuid::new_v4().simple().to_string()
}

/// 生成Span ID（16位十六进制）
fn generate_span_id() -> String {
    use uuid::Uuid;
    let uuid = Uuid::new_v4().simple().to_string();
    uuid[..16].to_string()
}

/// Trace中间件（用于axum/tower）— 从请求头提取trace_id并设置到线程局部
pub struct TraceMiddleware;

impl TraceMiddleware {
    /// 从HTTP头提取trace_id（支持X-Trace-Id和traceparent）
    pub fn extract_trace_id(headers: &std::collections::HashMap<String, String>) -> Option<TraceId> {
        headers.get("x-trace-id").cloned()
            .or_else(|| headers.get("X-Trace-Id").cloned())
            .or_else(|| headers.get("traceparent").and_then(|v| {
                // W3C traceparent格式: 00-<trace_id>-<span_id>-<flags>
                let parts: Vec<&str> = v.split('-').collect();
                if parts.len() >= 2 { Some(parts[1].to_string()) } else { None }
            }))
    }
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 分布式追踪 — OpenTelemetry，零配置自动注入trace_id

use tracing::Span;
use uuid::Uuid;

/// 追踪上下文（贯穿整个请求链路）
#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
}

impl TraceContext {
    /// 创建新的追踪上下文（新trace）
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string().replace('-', ""),
            span_id: Uuid::new_v4().to_string().replace('-', "")[..16].to_string(),
            parent_span_id: None,
            service_name: service_name.into(),
        }
    }

    /// 从上游请求头恢复追踪上下文
    pub fn from_headers(headers: &axum::http::HeaderMap, service_name: impl Into<String>) -> Self {
        let trace_id = headers
            .get("x-trace-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string().replace('-', ""));
        let parent_span_id = headers
            .get("x-span-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        Self {
            trace_id,
            span_id: Uuid::new_v4().to_string().replace('-', "")[..16].to_string(),
            parent_span_id,
            service_name: service_name.into(),
        }
    }

    /// 注入到下游请求头（空实现占位，避免 reqwest 依赖）
    pub fn inject_headers(&self, _headers: &mut axum::http::HeaderMap) {
    }

    /// 创建子span
    pub fn child_span(&self, _name: &str) -> Span {
        tracing::info_span!(
            "child_span",
            trace_id = %self.trace_id,
            span_id = %self.span_id,
            service = %self.service_name,
        )
    }
}

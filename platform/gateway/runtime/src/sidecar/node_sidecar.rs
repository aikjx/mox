//! Node sidecar 客户端：本地 127.0.0.1:3010 的内部 endpoints（/internal/intent, /internal/graph-algo, /internal/graph/list, /internal/file/list ...）
//!
//! 特性：
//!   - reqwest 10s 超时；失败返回 SidecarError（带诊断）
//!   - metrics：每次调用对 sidecar_success / sidecar_fail 累加（进程内原子计数，供 /ai/engine/metrics 对外暴露）
//!   - fail-open 可选：当 sidecar 不可达时，对某些调用可返回 mock/fallback（通过 `enable_fallback=true`）。

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct SidecarMetrics {
    pub calls: Arc<AtomicU64>,
    pub success: Arc<AtomicU64>,
    pub fail: Arc<AtomicU64>,
    pub fallback_used: Arc<AtomicU64>,
}

impl SidecarMetrics {
    pub fn snapshot(&self) -> SidecarMetricsSnapshot {
        SidecarMetricsSnapshot {
            calls: self.calls.load(Ordering::Relaxed),
            success: self.success.load(Ordering::Relaxed),
            fail: self.fail.load(Ordering::Relaxed),
            fallback_used: self.fallback_used.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SidecarMetricsSnapshot {
    pub calls: u64,
    pub success: u64,
    pub fail: u64,
    pub fallback_used: u64,
}

#[derive(Debug, Clone)]
pub struct NodeSidecarClient {
    base_url: String,
    timeout_ms: u64,
    pub enable_fallback: bool,
    pub metrics: SidecarMetrics,
}

#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("sidecar unavailable: base={base} err={msg}")]
    Unavailable { base: String, msg: String },
    #[error("sidecar HTTP {status}: {body}")]
    Http { status: u16, body: String },
    #[error("serde: {0}")]
    Serde(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntentReq {
    pub query: String,
    #[serde(default)]
    pub context: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntentResp {
    pub ok: bool,
    pub intent: String,
    pub confidence: f64,
    pub capability: Option<String>,
    pub explain: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphAlgoReq {
    pub algorithm: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphAlgoResp {
    pub ok: bool,
    pub algorithm: String,
    pub result: serde_json::Value,
    pub timing_ms: Option<u64>,
}

impl NodeSidecarClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            timeout_ms: 10_000,
            enable_fallback: true,
            metrics: SidecarMetrics::default(),
        }
    }

    #[allow(dead_code)] // 作为 sidecar SDK 预留扩展 API（测试使用；lint 放行）
    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }
    #[allow(dead_code)] // 同上
    pub fn with_fallback(mut self, v: bool) -> Self {
        self.enable_fallback = v;
        self
    }
    #[allow(dead_code)] // 同上
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// 调 /internal/intent（真实 HTTP）。失败时：若 enable_fallback → 生成本地兜底（关键词匹配）；否则返回 Err。
    pub async fn intent(&self, req: IntentReq) -> Result<IntentResp, SidecarError> {
        self.metrics.calls.fetch_add(1, Ordering::Relaxed);
        let url = format!("{}/internal/intent", self.base_url.trim_end_matches('/'));
        let res = self._post_json(&url, &req).await;
        match res {
            Ok(body) => {
                let parsed: IntentResp =
                    serde_json::from_str(&body).map_err(|e| SidecarError::Serde(e.to_string()))?;
                self.metrics.success.fetch_add(1, Ordering::Relaxed);
                Ok(parsed)
            }
            Err(_e) if self.enable_fallback => {
                self.metrics.fail.fetch_add(1, Ordering::Relaxed);
                self.metrics.fallback_used.fetch_add(1, Ordering::Relaxed);
                Ok(fallback_intent(&req))
            }
            Err(e) => {
                self.metrics.fail.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    /// 调 /internal/graph-algo（真实 HTTP）。失败时：若 enable_fallback → 空 ok=false；否则 Err。
    pub async fn graph_algo(&self, req: GraphAlgoReq) -> Result<GraphAlgoResp, SidecarError> {
        self.metrics.calls.fetch_add(1, Ordering::Relaxed);
        let url = format!(
            "{}/internal/graph-algo",
            self.base_url.trim_end_matches('/')
        );
        match self._post_json(&url, &req).await {
            Ok(body) => {
                let parsed: GraphAlgoResp =
                    serde_json::from_str(&body).map_err(|e| SidecarError::Serde(e.to_string()))?;
                self.metrics.success.fetch_add(1, Ordering::Relaxed);
                Ok(parsed)
            }
            Err(_e) if self.enable_fallback => {
                self.metrics.fail.fetch_add(1, Ordering::Relaxed);
                self.metrics.fallback_used.fetch_add(1, Ordering::Relaxed);
                Ok(GraphAlgoResp {
                    ok: false,
                    algorithm: req.algorithm,
                    result: serde_json::Value::Null,
                    timing_ms: None,
                })
            }
            Err(e) => {
                self.metrics.fail.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    async fn _post_json<T: Serialize>(&self, url: &str, body: &T) -> Result<String, SidecarError> {
        // reqwest 初始化；运行时测试已挂 reqwest workspace dep
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .build()
            .map_err(|e| SidecarError::Unavailable {
                base: self.base_url.clone(),
                msg: format!("client build: {e}"),
            })?;
        let resp =
            client
                .post(url)
                .json(body)
                .send()
                .await
                .map_err(|e| SidecarError::Unavailable {
                    base: self.base_url.clone(),
                    msg: e.to_string(),
                })?;
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        if !(200..=299).contains(&status) {
            return Err(SidecarError::Http { status, body: text });
        }
        Ok(text)
    }

    /// 通用 POST passthrough：把任意 JSON body 透传到 sidecar 指定相对 path，原样返回 JSON Value。
    /// 用于 workflow/execute 等 Node 侧业务端点（在 Rust 只做薄代理）。
    pub async fn post_passthrough(
        &self,
        rel_path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, SidecarError> {
        self.metrics.calls.fetch_add(1, Ordering::Relaxed);
        let base = self.base_url.trim_end_matches('/');
        let p = rel_path.trim_start_matches('/');
        let url = format!("{base}/{p}");
        match self._post_json(&url, &body).await {
            Ok(text) => {
                let parsed: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| SidecarError::Serde(e.to_string()))?;
                self.metrics.success.fetch_add(1, Ordering::Relaxed);
                Ok(parsed)
            }
            Err(e) => {
                self.metrics.fail.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    /// 通用 GET passthrough：用于只读端点透传（verify/health）。
    pub async fn get_passthrough(&self, rel_path: &str) -> Result<serde_json::Value, SidecarError> {
        self.metrics.calls.fetch_add(1, Ordering::Relaxed);
        let base = self.base_url.trim_end_matches('/');
        let p = rel_path.trim_start_matches('/');
        let url = format!("{base}/{p}");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .build()
            .map_err(|e| SidecarError::Unavailable {
                base: self.base_url.clone(),
                msg: format!("client build: {e}"),
            })?;
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| SidecarError::Unavailable {
                base: self.base_url.clone(),
                msg: e.to_string(),
            })?;
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        if !(200..=299).contains(&status) {
            self.metrics.fail.fetch_add(1, Ordering::Relaxed);
            return Err(SidecarError::Http { status, body: text });
        }
        let parsed: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| SidecarError::Serde(e.to_string()))?;
        self.metrics.success.fetch_add(1, Ordering::Relaxed);
        Ok(parsed)
    }
}

fn fallback_intent(req: &IntentReq) -> IntentResp {
    // 非常轻量的关键词兜底，用于 sidecar 挂掉时仍能路由。不做复杂意图识别。
    let q = req.query.to_lowercase();
    let score_chat: u32 = 1;
    let mut score_file: u32 = 0;
    let mut score_graph: u32 = 0;
    let mut score_kb: u32 = 0;
    if q.contains("文档") || q.contains("文件") || q.contains("doc") || q.contains("上传") {
        score_file += 3;
    }
    if q.contains("节点") || q.contains("边") || q.contains("图谱") || q.contains("graph") {
        score_graph += 3;
    }
    if q.contains("知识") || q.contains("检索") || q.contains("需求") {
        score_kb += 2;
    }
    let best = [
        ("chat", score_chat),
        ("file_search", score_file),
        ("graph_query", score_graph),
        ("kb_search", score_kb),
    ]
    .into_iter()
    .max_by_key(|x| x.1)
    .unwrap_or(("chat", 1));
    IntentResp {
        ok: true,
        intent: best.0.to_string(),
        confidence: best.1 as f64 / 5.0,
        capability: Some(match best.0 {
            "chat" => "llm_chat".to_string(),
            "file_search" => "file_graph_search".to_string(),
            "graph_query" => "graph_query".to_string(),
            "kb_search" => "kb_search".to_string(),
            _ => "llm_chat".to_string(),
        }),
        explain: vec![
            "[sidecar-fallback] node sidecar 不可达，网关使用本地关键词兜底意图识别".to_string(),
            format!("keyword scored intent `{}` with raw {}", best.0, best.1),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unreachable_sidecar_fallback_ok_and_metrics_increment_fail() {
        // 监听端口 1：保证必然连不上；启用 fallback → 仍能拿到 ok=true 响应 + sidecar_fail +1
        let client = NodeSidecarClient::new("http://127.0.0.1:1")
            .with_timeout(200)
            .with_fallback(true);
        let before = client.metrics.snapshot();
        let resp = client
            .intent(IntentReq {
                query: "列出所有 Project 节点".to_string(),
                context: Default::default(),
            })
            .await
            .unwrap();
        assert!(resp.ok);
        assert_eq!(resp.intent, "graph_query");
        let after = client.metrics.snapshot();
        assert_eq!(after.calls, before.calls + 1);
        assert_eq!(after.fail, before.fail + 1);
        assert_eq!(after.fallback_used, before.fallback_used + 1);
        assert_eq!(after.success, before.success);
    }

    #[tokio::test]
    async fn unreachable_sidecar_no_fallback_err_diagnostic() {
        let client = NodeSidecarClient::new("http://127.0.0.1:1")
            .with_timeout(100)
            .with_fallback(false);
        let before = client.metrics.snapshot();
        let err = client
            .intent(IntentReq {
                query: "hi".into(),
                context: Default::default(),
            })
            .await
            .unwrap_err();
        match err {
            SidecarError::Unavailable { base, msg } => {
                assert!(base.contains("127.0.0.1:1"));
                assert!(!msg.is_empty(), "诊断消息不能为空");
            }
            other => panic!("预期 Unavailable，实际 {other:?}"),
        }
        let after = client.metrics.snapshot();
        assert_eq!(after.calls, before.calls + 1);
        assert_eq!(after.fail, before.fail + 1);
        assert_eq!(after.success, before.success);
    }
}

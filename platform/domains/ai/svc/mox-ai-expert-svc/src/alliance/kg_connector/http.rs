// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! HTTP 连接器实现
//!
//! 通过 HTTP JSON 调用 kg-hub 服务的 REST API。
//! 生产环境默认实现，符合微服务架构解耦原则。
//!
//! 对应 kg-hub API：
//!   - GET  /api/kg/search/quick?q=&top_k=&hops=  → 快速检索（含图扩散）
//!   - POST /api/kg/search                           → 混合检索
//!   - GET  /api/kg/health                           → 健康检查
//!
//! 重构要点：
//!   - 使用 reqwest 的 `.query()` 方法自动 URL 编码，消除手写 urlencode
//!   - 保留与原有 HttpKgHubConnector 完全兼容的构造 API

use std::time::Duration;

use super::traits::KgConnector;
use super::types::{ApiResp, GraphSearchHit};

/// 基于 HTTP 的 kg-hub 连接器
///
/// 调用 kg-hub 的 axum HTTP 接口（api.rs 中定义的路由）。
/// 使用 reqwest blocking 客户端（mox-expert 已有依赖）。
pub struct HttpKgHubConnector {
    base_url: String,
    client: reqwest::blocking::Client,
    timeout_ms: u64,
}

impl HttpKgHubConnector {
    /// 创建 HTTP 连接器
    ///
    /// - `base_url`: kg-hub 服务地址，如 "http://localhost:8080"
    /// - `timeout_ms`: 请求超时（默认 3000ms，激活扩散应快速返回）
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_timeout(base_url, 3000)
    }

    pub fn with_timeout(base_url: impl Into<String>, timeout_ms: u64) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
            timeout_ms,
        }
    }

    /// 快速检索（含图扩散）
    ///
    /// GET /api/kg/search/quick?q={q}&top_k={k}&hops={hops}
    ///
    /// 使用 reqwest 的 `.query()` 方法自动处理 URL 编码，
    /// 替代之前手写的 urlencode 函数，消除重复工具代码。
    fn quick_search(&self, q: &str, top_k: usize, hops: usize) -> Result<Vec<GraphSearchHit>, String> {
        let url = format!("{}/api/kg/search/quick", self.base_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("q", q), ("top_k", &top_k.to_string()), ("hops", &hops.to_string())])
            .send()
            .map_err(|e| format!("kg-hub request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("kg-hub HTTP {}", resp.status()));
        }

        let body: ApiResp<Vec<GraphSearchHit>> = resp
            .json()
            .map_err(|e| format!("kg-hub response parse failed: {}", e))?;

        if !body.ok {
            return Err(body.error.unwrap_or_else(|| "unknown kg-hub error".to_string()));
        }

        Ok(body.data.unwrap_or_default())
    }

    /// 内部访问 base_url（用于测试断言）
    #[cfg(test)]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// 内部访问 timeout_ms（用于测试断言）
    #[cfg(test)]
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }
}

impl KgConnector for HttpKgHubConnector {
    fn spread(
        &self,
        seeds: &[String],
        _damping: f64,
        rounds: u32,
    ) -> Result<std::collections::BTreeMap<String, f64>, String> {
        // 将 seeds 拼接为查询文本（kg-hub quick_search 接受文本查询）
        // rounds 映射为扩散跳数 hops
        let query = seeds.join(" ");
        let hops = rounds.min(5) as usize; // 最多 5 跳，防止过度扩散
        let top_k = 50;

        let hits = self.quick_search(&query, top_k, hops)?;

        // 将搜索结果转为 {node_id_or_name: score} 映射
        // intent.rs 的 classify_intent 会用 key 包含类名来归一到 7 类
        let mut result = std::collections::BTreeMap::new();
        for hit in hits {
            // 同时用 id 和 name 作为 key（提高 intent 归一命中率）
            let score = hit.score.max(hit.graph_score);
            if score > 0.0 {
                result.insert(hit.id.clone(), score);
                result.insert(hit.name.clone(), score);
            }
        }
        Ok(result)
    }

    fn search(&self, query: &str, top_k: usize) -> Result<Vec<GraphSearchHit>, String> {
        self.quick_search(query, top_k, 0)
    }

    fn available(&self) -> bool {
        let url = format!("{}/api/kg/health", self.base_url);
        self.client
            .get(&url)
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    fn name(&self) -> &str {
        "http-kg-hub"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// HttpKgHubConnector 构造正确（不实际发请求）
    #[test]
    fn http_connector_constructs_correctly() {
        let c = HttpKgHubConnector::new("http://localhost:8080/");
        assert_eq!(c.base_url(), "http://localhost:8080");
        assert_eq!(c.timeout_ms(), 3000);

        let c2 = HttpKgHubConnector::with_timeout("http://kg:9000", 5000);
        assert_eq!(c2.base_url(), "http://kg:9000");
        assert_eq!(c2.timeout_ms(), 5000);
    }

    /// 验证连接器名称
    #[test]
    fn http_connector_name_is_correct() {
        let c = HttpKgHubConnector::new("http://localhost:8080");
        assert_eq!(c.name(), "http-kg-hub");
    }

    /// 验证 KgConnector trait 实现
    #[test]
    fn http_connector_implements_kg_connector() {
        fn check<C: KgConnector>(_: &C) {}
        let c = HttpKgHubConnector::new("http://localhost:8080");
        check(&c);
    }
}

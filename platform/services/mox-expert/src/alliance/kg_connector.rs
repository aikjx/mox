//! KG-Hub 图谱连接器（v3 M1 交付）
//!
//! 职责：
//!   1. 将 kg-hub 统一图谱中枢的能力封装为 scheduler 可直接调用的 Connector
//!   2. 提供 `spread_fn()` 适配器，直接接入 `intent::classify_intent()` 的 `graph_spread_fn` 参数
//!   3. 提供 `enhance_expert_matching()`，用图谱 PageRank/关联度增强专家排序
//!
//! 设计原则：
//!   - 与 kg-hub 解耦：通过 HTTP JSON 通信，不直接依赖 kg-hub crate（符合 v3 微服务架构）
//!   - 降级安全：kg-hub 不可用时自动返回 Err，由 intent.rs 标记 degraded=true
//!   - 可测试：提供 `MockKgHubConnector` 用于单元测试
//!
//! 对应 kg-hub API：
//!   - GET  /api/kg/search/quick?q=&top_k=&hops=  → 快速检索（含图扩散）
//!   - POST /api/kg/search                           → 混合检索
//!   - GET  /api/kg/health                           → 健康检查

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

// ================== 公共类型 ==================

/// 图谱搜索命中（与 kg-hub SearchHit 对齐，但本地定义以解耦）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSearchHit {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub layer: String,
    pub path: String,
    pub summary: String,
    pub score: f64,
    pub keyword_score: f64,
    pub vector_score: f64,
    pub graph_score: f64,
    pub matched_by: Vec<String>,
}

/// kg-hub 统一响应包装
#[derive(Debug, Deserialize)]
struct ApiResp<T> {
    ok: bool,
    data: Option<T>,
    error: Option<String>,
}

// ================== Connector Trait ==================

/// KG-Hub 连接器抽象（便于 mock 测试和多实现）
pub trait KgHubConnector: Send + Sync {
    /// 激活扩散：从 seeds 出发，按 damping 衰减、rounds 轮扩散
    /// 返回 {node_label: score} 映射（intent.rs 会归一到 7 类）
    fn spread(
        &self,
        seeds: &[String],
        damping: f64,
        rounds: u32,
    ) -> Result<BTreeMap<String, f64>, String>;

    /// 混合检索：返回 top_k 条命中
    fn search(&self, query: &str, top_k: usize) -> Result<Vec<GraphSearchHit>, String>;

    /// 图谱是否可用（健康检查）
    fn available(&self) -> bool;

    /// 连接器名称（用于日志/trace）
    fn name(&self) -> &str;
}

// ================== HTTP 连接器（生产环境） ==================

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
    fn quick_search(&self, q: &str, top_k: usize, hops: usize) -> Result<Vec<GraphSearchHit>, String> {
        let url = format!(
            "{}/api/kg/search/quick?q={}&top_k={}&hops={}",
            self.base_url,
            urlencode(q),
            top_k,
            hops
        );
        let resp = self
            .client
            .get(&url)
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
}

impl KgHubConnector for HttpKgHubConnector {
    fn spread(
        &self,
        seeds: &[String],
        _damping: f64,
        rounds: u32,
    ) -> Result<BTreeMap<String, f64>, String> {
        // 将 seeds 拼接为查询文本（kg-hub quick_search 接受文本查询）
        // rounds 映射为扩散跳数 hops
        let query = seeds.join(" ");
        let hops = rounds.min(5) as usize; // 最多 5 跳，防止过度扩散
        let top_k = 50;

        let hits = self.quick_search(&query, top_k, hops)?;

        // 将搜索结果转为 {node_id_or_name: score} 映射
        // intent.rs 的 classify_intent 会用 key 包含类名来归一到 7 类
        let mut result: BTreeMap<String, f64> = BTreeMap::new();
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

// ================== Mock 连接器（测试用） ==================

/// 内存 Mock 连接器，用于单元测试（不依赖外部 kg-hub 服务）
pub struct MockKgHubConnector {
    spread_result: BTreeMap<String, f64>,
    search_result: Vec<GraphSearchHit>,
    available: bool,
}

impl MockKgHubConnector {
    pub fn new() -> Self {
        Self {
            spread_result: BTreeMap::new(),
            search_result: Vec::new(),
            available: true,
        }
    }

    pub fn with_spread(mut self, result: BTreeMap<String, f64>) -> Self {
        self.spread_result = result;
        self
    }

    pub fn with_search(mut self, hits: Vec<GraphSearchHit>) -> Self {
        self.search_result = hits;
        self
    }

    pub fn unavailable(mut self) -> Self {
        self.available = false;
        self
    }
}

impl Default for MockKgHubConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl KgHubConnector for MockKgHubConnector {
    fn spread(
        &self,
        _seeds: &[String],
        _damping: f64,
        _rounds: u32,
    ) -> Result<BTreeMap<String, f64>, String> {
        if !self.available {
            return Err("mock kg-hub unavailable".to_string());
        }
        Ok(self.spread_result.clone())
    }

    fn search(&self, _query: &str, top_k: usize) -> Result<Vec<GraphSearchHit>, String> {
        if !self.available {
            return Err("mock kg-hub unavailable".to_string());
        }
        Ok(self.search_result.iter().take(top_k).cloned().collect())
    }

    fn available(&self) -> bool {
        self.available
    }

    fn name(&self) -> &str {
        "mock-kg-hub"
    }
}

// ================== 适配器：接入 intent::classify_intent ==================

/// 将 KgHubConnector 适配为 `classify_intent()` 需要的 `graph_spread_fn` 闭包
///
/// 用法：
/// ```ignore
/// let connector = HttpKgHubConnector::new("http://localhost:8080");
/// let intent = classify_intent(query, Some(spread_fn(&connector)));
/// ```
///
/// 注意：返回的是 `FnOnce` 闭包，因为 classify_intent 内部只调用一次扩散。
/// 闭包捕获 connector 的引用，调用方需保证 connector 生命周期覆盖闭包使用。
pub fn spread_fn<'a, C: KgHubConnector + ?Sized>(
    connector: &'a C,
) -> impl FnOnce(&[String], f64, u32) -> Result<BTreeMap<String, f64>, String> + 'a {
    move |seeds, damping, rounds| connector.spread(seeds, damping, rounds)
}

// ================== 专家匹配增强 ==================

/// 专家匹配增强结果
#[derive(Debug, Clone)]
pub struct ExpertGraphBoost {
    /// 专家 ID → 图谱增强分（0..1，越高表示该专家与查询的图谱关联度越高）
    pub boosts: BTreeMap<String, f64>,
    /// 是否使用了图谱（false 表示降级，boosts 全为 0）
    pub graph_used: bool,
}

/// 用 kg-hub 图谱增强专家匹配排序
///
/// 流程：
///   1. 用 query 搜索图谱，获取相关节点
///   2. 对每个专家，计算其与搜索结果的关联度（专家 dimension/name 出现在搜索结果中的频率和分数）
///   3. 返回 {expert_id: boost_score}，可叠加到 team.rs 的 total 分数中
///
/// - `connector`: kg-hub 连接器
/// - `query`: 用户查询
/// - `expert_ids`: 待增强的专家 ID 列表
/// - `expert_dimensions`: 专家 ID → 维度名映射（用于图谱匹配）
pub fn enhance_expert_matching<C: KgHubConnector + ?Sized>(
    connector: &C,
    query: &str,
    expert_ids: &[String],
    expert_dimensions: &BTreeMap<String, String>,
) -> ExpertGraphBoost {
    let mut boosts: BTreeMap<String, f64> = BTreeMap::new();
    for id in expert_ids {
        boosts.insert(id.clone(), 0.0);
    }

    let hits = match connector.search(query, 30) {
        Ok(h) if !h.is_empty() => h,
        _ => {
            return ExpertGraphBoost {
                boosts,
                graph_used: false,
            };
        }
    };

    // 计算每个专家的图谱增强分
    // 策略：专家的 dimension/name 出现在搜索结果的 name/path/summary 中，
    // 按命中次数和分数加权
    for expert_id in expert_ids {
        let dim = expert_dimensions
            .get(expert_id)
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        let expert_key = expert_id.to_lowercase();

        let mut total_score = 0.0_f64;
        let mut hit_count = 0;

        for hit in &hits {
            let text = format!(
                "{} {} {} {}",
                hit.name.to_lowercase(),
                hit.path.to_lowercase(),
                hit.summary.to_lowercase(),
                hit.kind.to_lowercase()
            );
            if text.contains(&expert_key) || text.contains(&dim) {
                total_score += hit.score;
                hit_count += 1;
            }
        }

        if hit_count > 0 {
            // 归一化：平均分数 × 命中次数衰减（避免单一高分节点过度影响）
            let avg = total_score / hit_count as f64;
            let count_factor = 1.0 - (-hit_count as f64 * 0.5).exp(); // 0..1 饱和
            let boost = (avg * count_factor).min(1.0);
            boosts.insert(expert_id.clone(), boost);
        }
    }

    ExpertGraphBoost {
        boosts,
        graph_used: true,
    }
}

// ================== 工具函数 ==================

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
            out.push(ch);
        } else {
            for byte in ch.to_string().as_bytes() {
                out.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    out
}

// ================== 测试 ==================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alliance::intent::classify_intent;

    /// TDD 1: spread_fn 适配器能正确接入 classify_intent
    #[test]
    fn spread_fn_adapter_works_with_classify_intent() {
        let mut spread_result = BTreeMap::new();
        spread_result.insert("code".to_string(), 0.95);
        spread_result.insert("rust".to_string(), 0.80);
        spread_result.insert("programming".to_string(), 0.70);

        let connector = MockKgHubConnector::new().with_spread(spread_result);
        let result = classify_intent(
            "帮我写一个 Rust 函数",
            Some(spread_fn(&connector)),
        );

        // 图谱可用时 degraded 应为 false
        assert!(!result.degraded, "graph available should not be degraded");
        // spread_scores 应包含 code 类的高分
        assert!(
            result.spread_scores.get("code").copied().unwrap_or(0.0) > 0.0,
            "code spread score should be > 0, got {:?}",
            result.spread_scores
        );
    }

    /// TDD 2: kg-hub 不可用时自动降级
    #[test]
    fn unavailable_kg_hub_triggers_degraded() {
        let connector = MockKgHubConnector::new().unavailable();
        let result = classify_intent(
            "测试查询",
            Some(spread_fn(&connector)),
        );
        assert!(result.degraded, "unavailable kg-hub should trigger degraded");
        assert!(result.degrade_reason.is_some());
    }

    /// TDD 3: enhance_expert_matching 返回正确的增强分
    #[test]
    fn enhance_expert_matching_scores_relevant_experts() {
        let hits = vec![
            GraphSearchHit {
                id: "node1".into(),
                name: "Rust 代码质量分析".into(),
                kind: "Function".into(),
                layer: "L3".into(),
                path: "analysis/code_quality".into(),
                summary: "对 Rust 代码进行质量和性能分析".into(),
                score: 0.9,
                keyword_score: 0.8,
                vector_score: 0.7,
                graph_score: 0.6,
                matched_by: vec!["keyword".into()],
            },
            GraphSearchHit {
                id: "node2".into(),
                name: "安全审计".into(),
                kind: "Function".into(),
                layer: "L3".into(),
                path: "security/audit".into(),
                summary: "权限和安全漏洞检测".into(),
                score: 0.7,
                keyword_score: 0.6,
                vector_score: 0.5,
                graph_score: 0.4,
                matched_by: vec!["keyword".into()],
            },
        ];

        let connector = MockKgHubConnector::new().with_search(hits);
        let expert_ids = vec!["code_quality".to_string(), "security".to_string(), "business".to_string()];
        let mut dimensions = BTreeMap::new();
        dimensions.insert("code_quality".to_string(), "CodeQuality".to_string());
        dimensions.insert("security".to_string(), "Security".to_string());
        dimensions.insert("business".to_string(), "Business".to_string());

        let boost = enhance_expert_matching(&connector, "Rust 代码质量", &expert_ids, &dimensions);

        assert!(boost.graph_used, "should use graph");
        // code_quality 应该有较高增强分（命中 node1）
        assert!(
            boost.boosts.get("code_quality").copied().unwrap_or(0.0) > 0.0,
            "code_quality should get boost, got {:?}",
            boost.boosts
        );
        // security 应该有增强分（命中 node2）
        assert!(
            boost.boosts.get("security").copied().unwrap_or(0.0) > 0.0,
            "security should get boost"
        );
        // business 没有命中，应该为 0
        assert_eq!(
            boost.boosts.get("business").copied().unwrap_or(0.0),
            0.0,
            "business should have no boost"
        );
    }

    /// TDD 4: 图谱搜索为空时增强返回全 0
    #[test]
    fn empty_search_returns_zero_boosts() {
        let connector = MockKgHubConnector::new(); // 空搜索结果
        let expert_ids = vec!["code".to_string()];
        let dimensions = BTreeMap::new();
        let boost = enhance_expert_matching(&connector, "test", &expert_ids, &dimensions);
        assert!(!boost.graph_used);
        assert_eq!(boost.boosts.get("code").copied().unwrap_or(-1.0), 0.0);
    }

    /// TDD 5: urlencode 正确处理中文和特殊字符
    #[test]
    fn urlencode_handles_chinese_and_special() {
        assert_eq!(urlencode("hello"), "hello");
        assert!(urlencode("你好").contains("%E4%BD%A0"));
        assert!(urlencode("a b").contains("%20"));
    }

    /// TDD 6: HttpKgHubConnector 构造正确（不实际发请求）
    #[test]
    fn http_connector_constructs_correctly() {
        let c = HttpKgHubConnector::new("http://localhost:8080/");
        assert_eq!(c.base_url, "http://localhost:8080");
        assert_eq!(c.timeout_ms, 3000);

        let c2 = HttpKgHubConnector::with_timeout("http://kg:9000", 5000);
        assert_eq!(c2.base_url, "http://kg:9000");
        assert_eq!(c2.timeout_ms, 5000);
    }
}

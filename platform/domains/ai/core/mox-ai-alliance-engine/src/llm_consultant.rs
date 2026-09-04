// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! LLM 专家咨询器（FR-CORE-04 扩展）：
//!   - 通过 HTTP 调用 OpenAI 兼容的 LLM API，生成真实专家观点
//!   - 每位专家使用独立的 system prompt，注入维度、能力、历史通过率
//!   - 解析 LLM 输出，提取观点文本、质量分、置信度
//!   - 失败时自动降级回退到 LocalRuleConsultant，保证可用性
//!   - 支持超时控制、重试、请求/响应审计日志
//!
//! # 设计
//! - `LLMConfig` — LLM 连接配置（api_base / api_key / model / timeout）
//! - `HttpLLMConsultant` — HTTP LLM 咨询器，实现 ExpertConsultant trait
//! - `LLMResponseParser` — 解析 LLM 结构化输出（JSON 格式）
//! - `FallbackConsultant` — 包装器，LLM 失败时回退到本地规则

use crate::debate::{ExpertConsultant, ExpertOpinion, LocalRuleConsultant};
use crate::team::ExpertMeta;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// =============================================================================
// LLM 配置
// =============================================================================

/// LLM 连接配置
///
/// 支持所有 OpenAI 兼容的 API（OpenAI / Azure / vLLM / Ollama / 通义千问 / 豆包等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// API 基础地址（如 https://api.openai.com/v1）
    pub api_base: String,
    /// API 密钥
    pub api_key: String,
    /// 模型名称（如 gpt-4o / qwen-max / doubao-pro）
    pub model: String,
    /// 请求超时（秒），默认 30
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// 最大重试次数，默认 1
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// 温度参数，默认 0.3（专家分析需要低温度保证稳定性）
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// 最大输出 token 数，默认 1024
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_timeout_secs() -> u64 {
    30
}
fn default_max_retries() -> u32 {
    1
}
fn default_temperature() -> f32 {
    0.3
}
fn default_max_tokens() -> u32 {
    1024
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            api_base: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4o".to_string(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

impl LLMConfig {
    /// 从环境变量创建配置
    ///
    /// 支持的环境变量：
    /// - `LLM_API_BASE` — API 基础地址
    /// - `LLM_API_KEY` — API 密钥
    /// - `LLM_MODEL` — 模型名称
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("LLM_API_KEY").ok()?;
        if api_key.is_empty() {
            return None;
        }
        Some(Self {
            api_base: std::env::var("LLM_API_BASE")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            api_key,
            model: std::env::var("LLM_MODEL").unwrap_or_else(|_| "gpt-4o".to_string()),
            ..Default::default()
        })
    }

    /// 验证配置完整性
    pub fn validate(&self) -> Result<(), String> {
        if self.api_base.is_empty() {
            return Err("api_base 不能为空".to_string());
        }
        if self.api_key.is_empty() {
            return Err("api_key 不能为空（使用 LocalRuleConsultant 无需配置 LLM）".to_string());
        }
        if self.model.is_empty() {
            return Err("model 不能为空".to_string());
        }
        if self.timeout_secs == 0 {
            return Err("timeout_secs 必须大于 0".to_string());
        }
        Ok(())
    }
}

// =============================================================================
// OpenAI 兼容 API 类型
// =============================================================================

/// Chat 消息角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// Chat 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// Chat Completion 请求
#[derive(Debug, Clone, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

/// 响应格式（强制 JSON 输出）
#[derive(Debug, Clone, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

/// Chat Completion 响应
#[derive(Debug, Clone, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
    #[allow(dead_code)]
    usage: Option<Usage>,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Usage {
    #[allow(dead_code)]
    prompt_tokens: u32,
    #[allow(dead_code)]
    completion_tokens: u32,
    #[allow(dead_code)]
    total_tokens: u32,
}

/// LLM 结构化输出（专家观点）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertOpinionJSON {
    /// 专家观点正文（Markdown 格式）
    pub answer: String,
    /// 质量评分（0.0 - 1.0）
    pub score: f64,
    /// 置信度（0.0 - 1.0）
    pub confidence: f64,
    /// 关键风险点（可选）
    #[serde(default)]
    pub risks: Vec<String>,
    /// 建议行动项（可选）
    #[serde(default)]
    pub recommendations: Vec<String>,
}

// =============================================================================
// HttpLLMConsultant
// =============================================================================

/// HTTP LLM 专家咨询器
///
/// 通过 OpenAI 兼容 API 调用 LLM，为每位专家生成真实观点。
///
/// # 示例
///
/// ```rust,ignore
/// use mox_ai_alliance_engine::debate::HttpLLMConsultant;
///
/// let config = LLMConfig {
///     api_base: "https://api.openai.com/v1".into(),
///     api_key: "sk-xxx".into(),
///     model: "gpt-4o".into(),
///     ..Default::default()
/// };
/// let consultant = HttpLLMConsultant::new(config);
/// let engine = DebateEngine::with_consultant(consultant);
/// ```
#[derive(Clone)]
pub struct HttpLLMConsultant {
    config: LLMConfig,
    client: reqwest::Client,
    fallback: LocalRuleConsultant,
}

impl std::fmt::Debug for HttpLLMConsultant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpLLMConsultant")
            .field("model", &self.config.model)
            .field("api_base", &self.config.api_base)
            .field("timeout_secs", &self.config.timeout_secs)
            .finish()
    }
}

impl HttpLLMConsultant {
    /// 创建新的 LLM 咨询器
    pub fn new(config: LLMConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            config,
            client,
            fallback: LocalRuleConsultant::new(),
        }
    }

    /// 从环境变量创建（如果 LLM_API_KEY 未设置则返回 None）
    pub fn from_env() -> Option<Self> {
        LLMConfig::from_env().map(Self::new)
    }

    /// 构建专家的 system prompt
    fn build_system_prompt(&self, expert: &ExpertMeta) -> String {
        let dimension = format!("{:?}", expert.dimension);
        let capabilities = expert
            .supported_classes
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("、");

        format!(
            r#"你是「{}」领域的资深专家，维度标识：{}。

## 你的专业能力
支持的意图类别：{}

## 你的历史表现
- 近30天 A 级通过率：{:.1}%
- 平均单次分析延迟：{}ms
- 维度优先级：{}/100

## 你的任务
针对用户的查询，从你的专业维度进行深度分析，输出结构化的专家观点。

## 输出要求（必须严格遵守 JSON 格式）
输出必须是一个 JSON 对象，包含以下字段：
- "answer"：你的专家观点正文，使用 Markdown 格式，包含核心观点、分析过程、风险提示
- "score"：你对本次分析质量的自评分数，0.0 到 1.0 之间
- "confidence"：你对本次分析的置信度，0.0 到 1.0 之间
- "risks"：关键风险点数组（字符串数组，可为空）
- "recommendations"：建议行动项数组（字符串数组，可为空）

## 分析原则
1. 只从你的专业维度出发，不越界评论其他维度
2. 客观、严谨、可验证，不使用模糊表述
3. 涉及代码时给出具体建议和示例
4. 涉及安全/权限时必须强调最小权限原则
5. 如果查询与你的维度无关，score 设为 0.3 以下并说明原因

只输出 JSON，不要输出任何其他文字或 Markdown 代码块标记。"#,
            expert.description,
            dimension,
            if capabilities.is_empty() {
                "（未配置具体能力标签）".to_string()
            } else {
                capabilities
            },
            expert.gate_a_rate_30d * 100.0,
            expert.avg_latency_ms,
            expert.priority
        )
    }

    /// 调用 LLM API（带重试）
    async fn call_llm(&self, messages: Vec<ChatMessage>) -> Result<String, String> {
        let url = format!("{}/chat/completions", self.config.api_base.trim_end_matches('/'));

        let request_body = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            response_format: Some(ResponseFormat {
                format_type: "json_object".to_string(),
            }),
        };

        let mut last_error = String::new();

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                // 指数退避：1s, 2s, 4s...
                let delay = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                tokio::time::sleep(delay).await;
                tracing::warn!(attempt = attempt, "LLM 请求重试中...");
            }

            match self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        let body = resp.text().await.unwrap_or_default();
                        last_error = format!("HTTP {}: {}", status, body);
                        tracing::error!(status = %status, body = %body, "LLM API 请求失败");
                        continue;
                    }

                    match resp.json::<ChatCompletionResponse>().await {
                        Ok(data) => {
                            if let Some(choice) = data.choices.first() {
                                return Ok(choice.message.content.clone());
                            }
                            last_error = "LLM 返回空 choices".to_string();
                        }
                        Err(e) => {
                            last_error = format!("解析 LLM 响应失败: {}", e);
                            tracing::error!(error = %e, "解析 LLM 响应失败");
                        }
                    }
                }
                Err(e) => {
                    last_error = format!("网络错误: {}", e);
                    tracing::error!(error = %e, "LLM API 网络错误");
                }
            }
        }

        Err(last_error)
    }

    /// 解析 LLM 输出为 ExpertOpinionJSON
    ///
    /// 支持标准 JSON、带代码块的 JSON、越界分数自动 clamp
    pub fn parse_opinion(&self, raw: &str) -> Result<ExpertOpinionJSON, String> {
        // 尝试直接解析 JSON
        let cleaned = raw.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();

        match serde_json::from_str::<ExpertOpinionJSON>(cleaned) {
            Ok(mut opinion) => {
                // 校验范围
                opinion.score = opinion.score.clamp(0.0, 1.0);
                opinion.confidence = opinion.confidence.clamp(0.0, 1.0);
                if opinion.answer.trim().is_empty() {
                    return Err("LLM 输出的 answer 为空".to_string());
                }
                Ok(opinion)
            }
            Err(e) => {
                // 尝试提取 JSON 子串
                if let Some(start) = cleaned.find('{') {
                    if let Some(end) = cleaned.rfind('}') {
                        if end > start {
                            let json_str = &cleaned[start..=end];
                            if let Ok(mut opinion) = serde_json::from_str::<ExpertOpinionJSON>(json_str) {
                                opinion.score = opinion.score.clamp(0.0, 1.0);
                                opinion.confidence = opinion.confidence.clamp(0.0, 1.0);
                                if !opinion.answer.trim().is_empty() {
                                    return Ok(opinion);
                                }
                            }
                        }
                    }
                }
                Err(format!("无法解析 LLM 输出为 JSON: {}", e))
            }
        }
    }
}

#[async_trait]
impl ExpertConsultant for HttpLLMConsultant {
    async fn consult(&self, query: &str, expert: &ExpertMeta) -> ExpertOpinion {
        let t0 = std::time::Instant::now();
        let dim_str = format!("{:?}", expert.dimension);

        // 1. 构建消息
        let system_prompt = self.build_system_prompt(expert);
        let messages = vec![
            ChatMessage {
                role: ChatRole::System,
                content: system_prompt,
            },
            ChatMessage {
                role: ChatRole::User,
                content: query.to_string(),
            },
        ];

        // 2. 调用 LLM
        match self.call_llm(messages).await {
            Ok(raw) => {
                // 3. 解析输出
                match self.parse_opinion(&raw) {
                    Ok(parsed) => {
                        let latency_ms = t0.elapsed().as_millis().min(u64::MAX as u128) as u64;
                        let tokens_approx = estimate_tokens(&parsed.answer);

                        tracing::info!(
                            expert = %expert.expert_id,
                            score = parsed.score,
                            confidence = parsed.confidence,
                            latency_ms = latency_ms,
                            "LLM 专家观点生成成功"
                        );

                        ExpertOpinion {
                            expert_id: expert.expert_id.clone(),
                            dimension: dim_str.to_lowercase(),
                            answer: parsed.answer,
                            score: parsed.score,
                            confidence: parsed.confidence,
                            latency_ms,
                            timed_out: false,
                            tokens_approx,
                        }
                    }
                    Err(e) => {
                        tracing::warn!(expert = %expert.expert_id, error = %e, "LLM 输出解析失败，降级到本地规则");
                        self.fallback_to_local(query, expert, &dim_str, t0).await
                    }
                }
            }
            Err(e) => {
                tracing::warn!(expert = %expert.expert_id, error = %e, "LLM 调用失败，降级到本地规则");
                self.fallback_to_local(query, expert, &dim_str, t0).await
            }
        }
    }

    fn is_llm_mode(&self) -> bool {
        true
    }
}

impl HttpLLMConsultant {
    /// 降级到本地规则咨询器
    async fn fallback_to_local(
        &self,
        query: &str,
        expert: &ExpertMeta,
        dim_str: &str,
        t0: std::time::Instant,
    ) -> ExpertOpinion {
        let mut op = self.fallback.consult(query, expert).await;
        // 标注这是降级结果
        op.answer = format!(
            "> ⚠️ **降级提示**：LLM 调用失败，以下为本地规则生成的参考观点（非真实 AI 分析）。\n\n{}",
            op.answer
        );
        op.dimension = dim_str.to_lowercase();
        op.latency_ms = t0.elapsed().as_millis().min(u64::MAX as u128) as u64;
        op
    }
}

// =============================================================================
// FallbackConsultant — 包装器，支持运行时切换 LLM / 本地规则
// =============================================================================

/// 可切换的咨询器包装器
///
/// 运行时根据配置决定使用 LLM 还是本地规则，无需重建 DebateEngine。
#[derive(Debug, Clone)]
pub enum SwitchableConsultant {
    /// 本地规则（默认，无外部依赖）
    Local(LocalRuleConsultant),
    /// LLM 驱动（需要 API 配置）
    LLM(HttpLLMConsultant),
}

impl SwitchableConsultant {
    /// 从配置创建：有 LLM 配置则用 LLM，否则用本地规则
    pub fn from_config(config: Option<LLMConfig>) -> Self {
        match config {
            Some(cfg) if cfg.validate().is_ok() => {
                Self::LLM(HttpLLMConsultant::new(cfg))
            }
            _ => Self::Local(LocalRuleConsultant::new()),
        }
    }

    /// 是否为 LLM 模式
    pub fn is_llm(&self) -> bool {
        matches!(self, Self::LLM(_))
    }
}

#[async_trait]
impl ExpertConsultant for SwitchableConsultant {
    async fn consult(&self, query: &str, expert: &ExpertMeta) -> ExpertOpinion {
        match self {
            Self::Local(c) => c.consult(query, expert).await,
            Self::LLM(c) => c.consult(query, expert).await,
        }
    }

    fn is_llm_mode(&self) -> bool {
        self.is_llm()
    }
}

// =============================================================================
// 工具函数
// =============================================================================

/// 估算文本的 token 数（粗略估算：英文 1 token ≈ 4 chars，中文 1 token ≈ 1.5 chars）
fn estimate_tokens(text: &str) -> usize {
    let ascii_chars = text.chars().filter(|c| c.is_ascii()).count() as f64;
    let cjk_chars = text.chars().filter(|c| !c.is_ascii()).count() as f64;
    ((ascii_chars / 4.0) + (cjk_chars / 1.5)) as usize
}

// =============================================================================
// TDD 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_config_default_values() {
        let cfg = LLMConfig::default();
        assert_eq!(cfg.timeout_secs, 30);
        assert_eq!(cfg.max_retries, 1);
        assert!((cfg.temperature - 0.3).abs() < f32::EPSILON);
        assert_eq!(cfg.max_tokens, 1024);
    }

    #[test]
    fn llm_config_validate_empty_api_key() {
        let cfg = LLMConfig {
            api_key: String::new(),
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn llm_config_validate_valid() {
        let cfg = LLMConfig {
            api_base: "https://api.example.com/v1".to_string(),
            api_key: "sk-test".to_string(),
            model: "test-model".to_string(),
            ..Default::default()
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn parse_opinion_valid_json() {
        let consultant = HttpLLMConsultant::new(LLMConfig {
            api_key: "test".to_string(),
            ..Default::default()
        });
        let raw = r#"{"answer": "测试观点", "score": 0.85, "confidence": 0.9, "risks": [], "recommendations": []}"#;
        let result = consultant.parse_opinion(raw);
        assert!(result.is_ok());
        let opinion = result.unwrap();
        assert_eq!(opinion.answer, "测试观点");
        assert!((opinion.score - 0.85).abs() < f64::EPSILON);
        assert!((opinion.confidence - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_opinion_with_code_fence() {
        let consultant = HttpLLMConsultant::new(LLMConfig {
            api_key: "test".to_string(),
            ..Default::default()
        });
        let raw = "```json\n{\"answer\": \"带代码块的观点\", \"score\": 0.7, \"confidence\": 0.8, \"risks\": [], \"recommendations\": []}\n```";
        let result = consultant.parse_opinion(raw);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().answer, "带代码块的观点");
    }

    #[test]
    fn parse_opinion_clamps_out_of_range() {
        let consultant = HttpLLMConsultant::new(LLMConfig {
            api_key: "test".to_string(),
            ..Default::default()
        });
        let raw = r#"{"answer": "越界测试", "score": 1.5, "confidence": -0.5, "risks": [], "recommendations": []}"#;
        let result = consultant.parse_opinion(raw).unwrap();
        assert!((result.score - 1.0).abs() < f64::EPSILON);
        assert!((result.confidence - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_opinion_empty_answer_rejected() {
        let consultant = HttpLLMConsultant::new(LLMConfig {
            api_key: "test".to_string(),
            ..Default::default()
        });
        let raw = r#"{"answer": "", "score": 0.5, "confidence": 0.5, "risks": [], "recommendations": []}"#;
        assert!(consultant.parse_opinion(raw).is_err());
    }

    #[test]
    fn estimate_tokens_mixed_text() {
        let text = "Hello 世界";
        let tokens = estimate_tokens(text);
        // Hello = 5 ascii chars ≈ 1.25 tokens, 世界 = 2 cjk ≈ 1.33 tokens
        assert!(tokens >= 1 && tokens <= 5);
    }

    #[test]
    fn switchable_consultant_local_mode() {
        let consultant = SwitchableConsultant::from_config(None);
        assert!(!consultant.is_llm());
        assert!(!consultant.is_llm_mode());
    }

    #[test]
    fn switchable_consultant_invalid_config_falls_back_local() {
        let cfg = LLMConfig {
            api_key: String::new(),
            ..Default::default()
        };
        let consultant = SwitchableConsultant::from_config(Some(cfg));
        assert!(!consultant.is_llm(), "无效配置应回退到本地模式");
    }

    #[test]
    fn build_system_prompt_contains_expert_info() {
        use crate::team::{build_expert_registry, ExpertMeta};
        let consultant = HttpLLMConsultant::new(LLMConfig {
            api_key: "test".to_string(),
            ..Default::default()
        });
        let registry = build_expert_registry();
        let security = registry.get("security").unwrap();
        let prompt = consultant.build_system_prompt(security);
        assert!(prompt.contains("security") || prompt.contains("Security") || prompt.contains("安全"));
        assert!(prompt.contains("JSON"));
        assert!(prompt.contains("score"));
        assert!(prompt.contains("confidence"));
    }

    // 注意：实际 LLM 调用测试需要真实 API Key，在 CI 中跳过
    // 可通过设置 LLM_API_KEY 环境变量后手动运行：
    // cargo test --features llm-live-test llm_live_test
}

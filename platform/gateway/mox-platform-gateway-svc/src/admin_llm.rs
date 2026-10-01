// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! LLM 网关管理面路由（AdminLlm 面板·只读族）：`/api/llm/providers*` `/api/llm/health` `/api/llm/routing`
//!
//! # 能力来源（禁桩）
//! 本模块全部读视图直接投影自 [`mox_ai_expert_svc::llm::LlmConfig::from_env()`]——
//! 即网关/专家链路真实使用的 LLM 配置（env 驱动，运行时实时读取）：
//!
//! - 多 Provider：`MOX_LLM_PROVIDERS=id1,id2` + `MOX_LLM_{ID}_BASE_URL/_API_KEY/_MODEL`
//! - 单 Provider：`MOX_LLM_API_KEY/_BASE_URL/_MODEL`，兼容 `DEEPSEEK_API_KEY` / `OPENAI_API_KEY`
//! - 路由：`MOX_LLM_ROUTING_STRATEGY`（priority/round_robin/latency_first/cost_first）
//!
//! **安全**：API Key 绝不回传，仅以 `has_key: bool` 表达"是否已配置"。
//!
//! # 边界（如实报告，不造桩）
//! - 配置来源是**环境变量**，进程启动时即固定，无持久化存储（DB/文件）。
//!   因此 Provider 的增删改/启停/设默认/连通性测试/模型自动发现 等**写操作族**
//!   需要"LLM 配置持久化层 + 运行时热加载"前置工程，本模块不伪造这些写端点。
//! - `/api/llm/usage` `/api/llm/stats` `/api/llm/logs` 需要逐次调用遥测管线，
//!   网关当前无 LLM 调用日志采集，属缺口，不返回全零模板。
//! - `/api/web-search/*` 无任何互联网搜索引擎客户端实现（无 DuckDuckGo/SearXNG/Tavily），
//!   属缺口，不在此伪造。

use axum::{routing::get, Router};
use mox_ai_expert_svc::llm::{LlmConfig, RoutingStrategy};
use mox_api_protocol::{api_ok, ApiResponse};
use serde_json::{json, Value};

/// 路由策略枚举 → 前端约定字符串（与 AdminLlm.vue 下拉值对齐）
fn strategy_name(s: RoutingStrategy) -> &'static str {
    match s {
        RoutingStrategy::Priority => "priority",
        RoutingStrategy::RoundRobin => "round_robin",
        RoutingStrategy::LatencyFirst => "latency_first",
        RoutingStrategy::CostFirst => "cost_first",
    }
}

/// GET /api/llm/providers —— 当前真实配置的 Provider 清单（API Key 脱敏）
///
/// 只回传 env 中**已配置（有 Key）**的 Provider；未配置 Key 的渠道不会被
/// `LlmConfig::from_env()` 收录，因此不出现在此清单（这是事实，不是过滤）。
async fn llm_providers() -> ApiResponse<Value> {
    let mut list: Vec<Value> = Vec::new();
    if let Some(c) = LlmConfig::from_env() {
        for (i, p) in c.providers.iter().enumerate() {
            list.push(json!({
                "id": p.provider_id,
                "name": p.provider_id,
                "type": p.provider_id,
                "base_url": p.base_url,
                "model": p.model,
                "enabled": p.enabled,
                // 第一个 provider 即 env 配置的"当前使用"（与 LlmConfig 兼容视图 providers[0] 一致）
                "active": i == 0,
                "has_key": !p.api_key.is_empty(),
                "temperature": c.temperature,
                "max_tokens": c.max_tokens,
                "price_per_1k_tokens": p.price_per_1k_tokens,
                // 注意：不返回 api_key 明文
            }));
        }
    }
    api_ok(json!(list))
}

/// 已知公开 Provider 预设（参考数据：官方公开 Base URL / 模型清单）
///
/// 这是**静态参考元数据**（类似国家码/币种表），用于"快速添加渠道"自动填充表单；
/// 不代表已配置或已连通。真实连通状态以 env 配置 + 人工测试为准。
fn llm_presets() -> Vec<Value> {
    vec![
        json!({
            "id": "deepseek",
            "name": "DeepSeek",
            "base_url": "https://api.deepseek.com",
            "models": ["deepseek-chat", "deepseek-reasoner"],
            "description": "深度求索（OpenAI 兼容，国内直连）"
        }),
        json!({
            "id": "openai",
            "name": "OpenAI",
            "base_url": "https://api.openai.com/v1",
            "models": ["gpt-4o-mini", "gpt-4o", "o4-mini"],
            "description": "OpenAI 官方（Chat Completions 兼容）"
        }),
        json!({
            "id": "qwen",
            "name": "通义千问",
            "base_url": "https://dashscope.aliyuncs.com/compatible-mode/v1",
            "models": ["qwen-plus", "qwen-max", "qwen-turbo"],
            "description": "阿里云 DashScope（OpenAI 兼容模式）"
        }),
        json!({
            "id": "zhipu",
            "name": "智谱 GLM",
            "base_url": "https://open.bigmodel.cn/api/paas/v4",
            "models": ["glm-4-plus", "glm-4-air", "glm-4-flash"],
            "description": "智谱 AI BigModel（GLM 系列）"
        }),
        json!({
            "id": "volcengine",
            "name": "火山方舟",
            "base_url": "https://ark.cn-beijing.volces.com/api/v3",
            "models": ["doubao-pro-4k", "doubao-pro-32k", "doubao-lite-4k"],
            "description": "字节火山引擎方舟（豆包，OpenAI 兼容）"
        }),
        json!({
            "id": "anthropic",
            "name": "Anthropic",
            "base_url": "https://api.anthropic.com",
            "models": ["claude-opus-4", "claude-sonnet-4", "claude-haiku-3-5"],
            "description": "Anthropic Claude（Messages API）"
        }),
        json!({
            "id": "google",
            "name": "Google Gemini",
            "base_url": "https://generativelanguage.googleapis.com/v1beta",
            "models": ["gemini-2.0-flash", "gemini-1.5-pro", "gemini-1.5-flash"],
            "description": "Google Gemini（Generative Language API）"
        }),
        json!({
            "id": "ollama",
            "name": "Ollama 本地",
            "base_url": "http://127.0.0.1:11434/v1",
            "models": ["llama3.1", "qwen2.5", "deepseek-r1"],
            "description": "本地 Ollama（OpenAI 兼容端点，无需 Key）"
        }),
    ]
}

/// GET /api/llm/providers/presets —— 已知公开 Provider 预设（静态参考数据）
async fn llm_provider_presets() -> ApiResponse<Value> {
    api_ok(json!(llm_presets()))
}

/// GET /api/llm/health —— LLM 网关健康/配置概况（env 实时投影，不发网络请求）
async fn llm_health() -> ApiResponse<Value> {
    let cfg = LlmConfig::from_env();
    let (configured, providers, enabled, with_key, strategy) = match &cfg {
        Some(c) => (
            true,
            c.providers.len(),
            c.providers.iter().filter(|p| p.enabled).count(),
            c.providers.iter().filter(|p| p.enabled && !p.api_key.is_empty()).count(),
            strategy_name(c.routing_strategy),
        ),
        None => (false, 0, 0, 0, "priority"),
    };
    api_ok(json!({
        "configured": configured,
        "providers": providers,
        "enabled": enabled,
        "with_key": with_key,
        "routing_strategy": strategy,
        "engine": if configured { "openai_compatible" } else { "local_rules_fallback" },
        // 真实语义：未配置 Key 时专家链路回退本地 mox_optimize 规则引擎（见 experts_collaboration.rs）
        "note": if configured {
            "已配置真实 LLM Provider（env 驱动）"
        } else {
            "未配置 MOX_LLM_* 凭据，专家链路回退本地规则引擎"
        }
    }))
}

/// GET /api/llm/routing —— 当前路由策略配置（env 投影）
async fn llm_routing() -> ApiResponse<Value> {
    match LlmConfig::from_env() {
        Some(c) => {
            let ids: Vec<&str> = c.providers.iter().map(|p| p.provider_id.as_str()).collect();
            api_ok(json!({
                "strategy": strategy_name(c.routing_strategy),
                "providers": ids,
                "fallback": true,
                "load_balance": matches!(c.routing_strategy, RoutingStrategy::RoundRobin | RoutingStrategy::CostFirst),
                "circuit_break_threshold": c.circuit_break_threshold,
                "circuit_break_cooldown_ms": c.circuit_break_cooldown_ms,
                "timeout_ms": c.timeout_ms,
            }))
        }
        None => api_ok(json!({
            "strategy": "priority",
            "providers": Vec::<String>::new(),
            "fallback": true,
            "load_balance": false,
            "note": "未配置 LLM Provider，路由策略为默认 priority"
        })),
    }
}

/// 装配 LLM 管理面只读路由（无状态：配置实时读 env）
pub fn build_llm_admin_router() -> Router<()> {
    Router::new()
        .route("/api/llm/providers", get(llm_providers))
        .route("/api/llm/providers/presets", get(llm_provider_presets))
        .route("/api/llm/health", get(llm_health))
        .route("/api/llm/routing", get(llm_routing))
}

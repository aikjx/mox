// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 真实大模型专家咨询实现：`LlmExpertConsultant`
//!
//! 实现 `ExpertConsultant` trait，把「真实 LLM（ReAct + 工具调用）」接入专家节点：
//! 1. 从 `ConsultQuery.ctx.prefer_expert` 推导专家角色，构建系统提示；
//! 2. 运行 ReAct 循环（工具：calculate / now / expert_lookup），得到推理轨迹与最终答案；
//! 3. 把最终答案归一化为 `ConsultReport`（steps=推理轨迹、score=模型自评、vetoed=是否否决）；
//! 4. 无 API Key 或 LLM 调用失败时回退本地 `ExpertServiceImpl`（保证离线可用、优雅降级）。

use super::chat::{ChatClient, LlmConfig, OpenAiChatClient};
#[cfg(test)]
use super::chat::ChatMessage;
use super::react::{run_react, ReactConfig, ReactResult};
use super::tools::{ExpertLookupTool, ToolRegistry};
use crate::expert_traits::ExpertConsultant;
use crate::services::ExpertServiceImpl;
use crate::types::{ConsultQuery, ConsultReport, Result};
use async_trait::async_trait;
use std::sync::Arc;

/// 真实 LLM 专家咨询器
pub struct LlmExpertConsultant {
    client: Arc<dyn ChatClient>,
    tools: ToolRegistry,
    react: ReactConfig,
    /// 模型显示名（写入步骤）
    model: String,
    /// 本地回退咨询器
    local: ExpertServiceImpl,
    /// 专家信息查询回调（供 expert_lookup 工具）
    expert_lookup: Option<Arc<dyn Fn(&str) -> Option<(String, String, String, Vec<String>)> + Send + Sync>>,
}

impl LlmExpertConsultant {
    /// 构造（本地回退为全新 ExpertServiceImpl）
    pub fn new(client: Arc<dyn ChatClient>, model: impl Into<String>) -> Self {
        Self {
            client,
            tools: ToolRegistry::with_builtins(),
            react: ReactConfig::from_env(),
            model: model.into(),
            local: ExpertServiceImpl::new(),
            expert_lookup: None,
        }
    }

    pub fn with_react_config(mut self, config: ReactConfig) -> Self {
        self.react = config;
        self
    }

    /// 注册 expert_lookup 工具（用注册表查询回调）
    pub fn with_expert_lookup<F>(mut self, lookup: F) -> Self
    where
        F: Fn(&str) -> Option<(String, String, String, Vec<String>)> + Send + Sync + 'static,
    {
        let tool = Arc::new(ExpertLookupTool::new(lookup)) as Arc<dyn super::tools::Tool>;
        self.tools.register(tool);
        self.expert_lookup = None;
        self
    }

    /// 构造系统提示
    fn build_system_prompt(&self, expert_id: Option<&str>) -> String {
        let role = expert_role(expert_id);
        format!(
            "你是「{}」，一个基于真实大模型的专家智能体。\n\
             你的任务：针对用户的专家咨询请求，给出专业、可执行的分析结论。\n\n\
             可用工具：\n{}\n\n\
             使用规则：\n\
             1. 需要精确计算或查证时，必须先输出 <tool_call>{{\"name\":\"...\",\"arguments\":{{...}}}}</tool_call>，\n\
                收到观察结果后再继续推理；严禁凭空编造数值。\n\
             2. 最终答案必须以纯文本结论收尾，并在最后一行输出：\n\
                「结论评分：0.75」与「是否否决：否」（评分 0~1，1=完全可靠；\n\
                仅当确属无法处理或方案不可行时才输出「是否否决：是」）。",
            role,
            self.tools.tool_descriptions()
        )
    }

    /// 构造用户消息
    fn build_user_message(&self, query: &ConsultQuery) -> String {
        let input = query.ctx.get("input_data").cloned().unwrap_or_default();
        let context = query.ctx.get("context").cloned().unwrap_or_default();
        let mut s = format!("【专家请求】{}", query.query);
        if !input.is_empty() {
            s.push_str(&format!("\n【输入数据】{}", input));
        }
        if !context.is_empty() {
            s.push_str(&format!("\n【附加上下文】{}", context));
        }
        s.push_str("\n请给出最终专家结论。");
        s
    }

    /// 同步执行 LLM 咨询（内部：ReAct → Report）
    fn consult_sync(&self, query: &ConsultQuery) -> Result<ConsultReport> {
        let expert_id = query.ctx.get("prefer_expert").cloned();
        let system = self.build_system_prompt(expert_id.as_deref());
        let user = self.build_user_message(query);

        let react_result = run_react(
            self.client.as_ref(),
            &system,
            &user,
            &self.tools,
            &self.react,
        )
        .map_err(|e| anyhow::anyhow!("真实 LLM 咨询失败: {}", e))?;

        Ok(react_to_report(&query.id, &react_result, &self.model))
    }
}

/// 把 ReAct 结果归一化为 ConsultReport
pub fn react_to_report(id: &str, r: &ReactResult, model: &str) -> ConsultReport {
    let score = parse_score(&r.final_answer);
    let (vetoed, reason) = parse_veto(&r.final_answer);
    let mut steps = r.to_steps(model);
    // 末尾附最终答案首行（去评分控制行）
    let answer_line = r
        .final_answer
        .lines()
        .map(|l| l.trim()).find(|l| !l.is_empty() && !l.starts_with("结论评分") && !l.starts_with("是否否决"))
        .unwrap_or("")
        .to_string();
    if !answer_line.is_empty() && !steps.contains(&answer_line) {
        steps.push(format!("[结论] {}", answer_line));
    }
    ConsultReport {
        report_id: id.to_string(),
        steps,
        score,
        vetoed,
        reason,
    }
}

/// 从最终答案解析自评分数
pub fn parse_score(answer: &str) -> f64 {
    // 优先级：结论评分：x | "score": x | 置信度：x | 评分：x
    for pat in ["结论评分", "评分", "置信度"] {
        if let Some(idx) = answer.find(pat) {
            let rest = &answer[idx + pat.len()..];
            // 跳过 "：" 或 ":" 或空格
            let rest = rest.trim_start_matches([':', '：', ' ', '\t', '"', '\'']);
            // 取前导数字
            let num: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(v) = num.parse::<f64>() {
                return v.clamp(0.0, 1.0);
            }
        }
    }
    // 兜底：默认 0.75（介于中性到可靠之间）
    0.75
}

/// 从最终答案解析是否否决
pub fn parse_veto(answer: &str) -> (bool, Option<String>) {
    // 显式控制行（行首），例如「是否否决：否 / 是否否决: 是」
    for line in answer.lines() {
        let line = line.trim();
        if line.starts_with("是否否决") {
            let rest = line
                .trim_start_matches("是否否决")
                .trim_start_matches(['：', ':', ' ', '\t']);
            if rest.starts_with('否') || rest.starts_with('0') {
                return (false, None);
            }
            return (true, Some(snippet(answer)));
        }
    }
    // 语义兜底：仅正文中出现明确否决语义词
    if answer.contains("否决") || answer.contains("不可行") || answer.contains("无法处理") {
        (true, Some(snippet(answer)))
    } else {
        (false, None)
    }
}

fn snippet(answer: &str) -> String {
    let s: String = answer.chars().take(160).collect();
    if answer.chars().count() > 160 {
        format!("{}…", s)
    } else {
        s
    }
}

/// 专家 id → 角色名
pub fn expert_role(expert_id: Option<&str>) -> &'static str {
    let id = expert_id.unwrap_or("");
    let lower = id.to_lowercase();
    if lower.contains("finance") {
        "金融分析与投资专家"
    } else if lower.contains("math") {
        "数学与逻辑专家"
    } else if lower.contains("code") || lower.contains("program") {
        "软件工程与代码专家"
    } else if lower.contains("medical") || lower.contains("health") {
        "医学健康专家"
    } else if lower.contains("law") || lower.contains("legal") {
        "法律合规专家"
    } else if lower.contains("creative") || lower.contains("copy") {
        "创意文案专家"
    } else if lower.contains("vision") || lower.contains("visual") || lower.contains("image") {
        "视觉设计专家"
    } else if lower.contains("translat") {
        "翻译与语言专家"
    } else if lower.contains("research") || lower.contains("academic") {
        "学术研究专家"
    } else if lower.contains("arch") {
        "系统架构专家"
    } else if lower.contains("schedul") {
        "任务调度专家"
    } else {
        "领域专家"
    }
}

#[async_trait]
impl ExpertConsultant for LlmExpertConsultant {
    async fn consult(&self, query: &ConsultQuery) -> Result<ConsultReport> {
        // 复制闭包需要的最小状态，经 spawn_blocking 桥接阻塞式 LLM 客户端
        let q = query.clone();
        let id = q.id.clone();
        let system = self.build_system_prompt(q.ctx.get("prefer_expert").map(|s| s.as_str()));
        let user = self.build_user_message(&q);
        let client: Arc<dyn ChatClient> = self.client.clone();
        let tools = self.tools.clone();
        let react = self.react.clone();
        let model = self.model.clone();

        let result = tokio::task::spawn_blocking(move || {
            run_react(client.as_ref(), &system, &user, &tools, &react)
                .map(|r| react_to_report(&id, &r, &model))
        })
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking join error: {}", e))?;

        match result {
            Ok(report) => Ok(report),
            Err(e) => {
                if is_all_providers_broken(&e) {
                    log_warn(format!("所有 LLM Provider 不可用/熔断，立即回退本地引擎: {}", e));
                } else {
                    log_warn(format!("真实 LLM 咨询失败，回退本地引擎: {}", e));
                }
                self.local.consult_blocking(&q)
            }
        }
    }

    fn consult_blocking(&self, query: &ConsultQuery) -> Result<ConsultReport> {
        match self.consult_sync(query) {
            Ok(r) => Ok(r),
            Err(e) => {
                if is_all_providers_broken(&e) {
                    log_warn(format!("所有 LLM Provider 不可用/熔断，立即回退本地引擎: {}", e));
                } else {
                    log_warn(format!("真实 LLM 咨询失败，回退本地引擎: {}", e));
                }
                self.local.consult_sync(query)
            }
        }
    }
}

fn log_warn(msg: String) {
    if std::env::var("RUST_LOG").is_ok() {
        eprintln!("[llm-expert] {}", msg);
    }
}

/// 判断是否为「所有 Provider 不可用/熔断」错误（应立即回退本地，不重试）
fn is_all_providers_broken(e: &anyhow::Error) -> bool {
    let msg = e.to_string().to_lowercase();
    msg.contains("all llm providers") || msg.contains("circuit-broken")
}

/// 从环境变量构造真实 LLM 咨询器；未配置 Key 时返回 None
///
/// 环境变量：
/// - `MOX_LLM_API_KEY`（缺省回退 `OPENAI_API_KEY`）；缺省则返回 None（走本地）
/// - `MOX_LLM_BASE_URL` / `MOX_LLM_MODEL` / `MOX_LLM_MAX_ROUNDS` / `MOX_LLM_TIMEOUT_MS`
/// - `MOX_LLM_ENABLED=false` 可显式禁用
/// - 多 Provider 模式：`MOX_LLM_PROVIDERS=id1,id2` + 每个 provider 的
///   `MOX_LLM_{ID}_BASE_URL` / `MOX_LLM_{ID}_API_KEY` / `MOX_LLM_{ID}_MODEL`，
///   自动启用路由器（路由策略 + 熔断降级）
pub fn llm_consultant_from_env() -> Option<Arc<dyn ExpertConsultant>> {
    if std::env::var("MOX_LLM_ENABLED")
        .ok()
        .map(|v| v == "0" || v == "false" || v == "FALSE")
        .unwrap_or(false)
    {
        return None;
    }
    let (config, router) = LlmConfig::from_env_with_router()?;
    let client = if let Some(router) = router {
        // 多 Provider 模式：注入路由器
        Arc::new(OpenAiChatClient::new(config.clone()).with_router(router)) as Arc<dyn ChatClient>
    } else {
        // 单 Provider 模式：保持原有行为
        Arc::new(OpenAiChatClient::new(config.clone())) as Arc<dyn ChatClient>
    };
    let consultant = LlmExpertConsultant::new(client, config.model.clone())
        .with_expert_lookup(builtin_expert_lookup);
    Some(Arc::new(consultant) as Arc<dyn ExpertConsultant>)
}

/// 内置联盟专家静态表（10 个）：expert_id → (name, domain, capabilities)
fn builtin_expert_lookup(
    id: &str,
) -> Option<(String, String, String, Vec<String>)> {
    let row: (&str, &str, &[&str]) = match id {
        "code-expert-001" => ("代码开发专家", "code", &["code", "programming", "软件", "开发"]),
        "math-expert-001" => ("数学专家", "math", &["math", "mathematics", "数学", "算法"]),
        "medical-expert-001" => ("医学专家", "medical", &["medical", "health", "医学", "健康"]),
        "law-expert-001" => ("法律专家", "law", &["law", "legal", "法律", "合规"]),
        "finance-expert-001" => ("金融分析专家", "finance", &["finance", "investment", "金融", "投资"]),
        "creative-expert-001" => ("创意文案专家", "creative", &["creative", "copywriting", "创意", "文案"]),
        "vision-expert-001" => ("视觉设计专家", "vision", &["vision", "visual", "设计", "图像"]),
        "translation-expert-001" => ("翻译专家", "translation", &["translation", "语言", "翻译"]),
        "research-expert-001" => ("学术研究专家", "research", &["research", "学术", "综述", "论文"]),
        "arch-expert-001" => ("系统架构专家", "arch", &["architecture", "架构", "设计"]),
        _ => return None,
    };
    Some((
        id.to_string(),
        row.0.to_string(),
        row.1.to_string(),
        row.2.iter().map(|s| s.to_string()).collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::chat::MockChatClient;
    use std::collections::HashMap;

    /// 测试用：始终返回错误的 Mock 客户端（用于验证回退逻辑）
    struct ErrorMockClient {
        error_msg: String,
    }
    impl ErrorMockClient {
        fn new(msg: impl Into<String>) -> Self {
            Self { error_msg: msg.into() }
        }
    }
    impl ChatClient for ErrorMockClient {
        fn complete(&self, _messages: &[ChatMessage]) -> anyhow::Result<String> {
            Err(anyhow::anyhow!("{}", self.error_msg))
        }
    }

    fn query(id: &str, prefer: &str, input: &str) -> ConsultQuery {
        let mut ctx = HashMap::new();
        if !prefer.is_empty() {
            ctx.insert("prefer_expert".into(), prefer.into());
        }
        if !input.is_empty() {
            ctx.insert("input_data".into(), input.into());
        }
        ConsultQuery {
            id: id.into(),
            query: "6*7 等于多少？请给出结论".into(),
            ctx,
        }
    }

    #[test]
    fn parse_score_variants() {
        assert!((parse_score("结论评分：0.9") - 0.9).abs() < 1e-9);
        assert!((parse_score("评分: 0.85") - 0.85).abs() < 1e-9);
        assert!((parse_score("置信度：0.62") - 0.62).abs() < 1e-9);
        assert!((parse_score("无评分行") - 0.75).abs() < 1e-9);
        assert!((parse_score("结论评分：1.5") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn parse_veto_variants() {
        assert!(parse_veto("结论评分：0.5\n是否否决：是\n原因…").0);
        assert!(!parse_veto("结论评分：0.5\n是否否决：否").0);
        assert!(parse_veto("该方案不可行，无法落地").0);
        assert!(!parse_veto("结论正常。").0);
    }

    #[test]
    fn expert_role_mapping() {
        assert!(expert_role(Some("finance-expert-001")).contains("金融"));
        assert!(expert_role(Some("medical-expert-001")).contains("医学"));
        assert!(expert_role(Some("code-expert-001")).contains("代码"));
        assert!(expert_role(None).contains("领域"));
    }

    #[test]
    fn consultant_reacts_with_tool_and_falls_back_on_error() {
        // 脚本：第1轮工具调用 → 第2轮最终答案（含评分）
        let client = Arc::new(MockChatClient::new(vec![
            "<tool_call>{\"name\":\"calculate\",\"arguments\":{\"expression\":\"6*7\"}}</tool_call>".into(),
            "6*7=42。结论评分：0.9\n是否否决：否".into(),
        ])) as Arc<dyn ChatClient>;
        let c = LlmExpertConsultant::new(client, "mock-gpt");
        let rep = c.consult_blocking(&query("q1", "finance-expert-001", "")).unwrap();
        assert_eq!(rep.report_id, "q1");
        assert!((rep.score - 0.9).abs() < 1e-9);
        assert!(!rep.vetoed);
        assert!(rep.steps.iter().any(|s| s.contains("[工具]")));
        assert!(rep.steps.iter().any(|s| s.contains("42")));
    }

    #[test]
    fn consultant_veto_propagates() {
        let client = Arc::new(MockChatClient::new(vec![
            "该请求缺少必要信息，无法处理。结论评分：0.2\n是否否决：是".into(),
        ])) as Arc<dyn ChatClient>;
        let c = LlmExpertConsultant::new(client, "mock");
        let rep = c.consult_blocking(&query("q2", "", "")).unwrap();
        assert!(rep.vetoed);
        assert!(rep.reason.is_some());
        assert!((rep.score - 0.2).abs() < 1e-9);
    }

    #[test]
    fn test_consultant_falls_back_on_all_providers_broken() {
        // Mock 客户端返回 "all LLM providers circuit-broken" 错误
        let client = Arc::new(ErrorMockClient::new(
            "all LLM providers unavailable or circuit-broken",
        )) as Arc<dyn ChatClient>;
        let c = LlmExpertConsultant::new(client, "mock");
        // 应回退本地引擎，不返回错误
        let rep = c.consult_blocking(&query("q-fallback", "finance-expert-001", "")).unwrap();
        assert_eq!(rep.report_id, "q-fallback");
        // 本地引擎应产生非空 steps
        assert!(!rep.steps.is_empty());
    }

    #[test]
    fn test_consultant_falls_back_on_generic_error() {
        // 普通错误也应回退本地引擎
        let client = Arc::new(ErrorMockClient::new(
            "LLM request failed: connection refused",
        )) as Arc<dyn ChatClient>;
        let c = LlmExpertConsultant::new(client, "mock");
        let rep = c.consult_blocking(&query("q-generic", "", "")).unwrap();
        assert_eq!(rep.report_id, "q-generic");
        assert!(!rep.steps.is_empty());
    }

    #[test]
    fn test_is_all_providers_broken_detection() {
        assert!(is_all_providers_broken(&anyhow::anyhow!(
            "all LLM providers unavailable or circuit-broken"
        )));
        assert!(is_all_providers_broken(&anyhow::anyhow!(
            "circuit-broken: provider deepseek"
        )));
        assert!(!is_all_providers_broken(&anyhow::anyhow!(
            "LLM request failed: timeout"
        )));
        assert!(!is_all_providers_broken(&anyhow::anyhow!(
            "LLM API error (500): internal error"
        )));
    }
}

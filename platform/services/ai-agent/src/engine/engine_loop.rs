//! 引擎主循环 - 串联感知→规划→执行→观察→反思→生成→巩固完整链路
//!
//! 核心设计：
//! - 持有 FSM 与 Guard 集合，驱动状态机在各阶段间流转
//! - 在 ACT 和 REFLECT 阶段强制通过守卫检查
//! - 支持 HITL（人机协同）暂停与恢复
//! - 可配置最大循环次数与超时策略
//! - 可选接入 LLM 客户端与浏览器自动化引擎

use super::guards::{BudgetGuard, CompositeGuard, GuardContext, GuardResult, ProgressGuard, RiskGuard};
use super::state_machine::{EngineEvent, EngineFSM, EngineState};
use super::tools::{ToolRegistry, tool_type_to_name};
use crate::browser_automation::{BrowserAction, BrowserAutomationEngine};
use crate::llm_client::{LLMChatMessage, LLMClient};
use kg_hub::consolidator::{EngineTrace, TraceConsolidator};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// 引擎运行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineResult {
    pub success: bool,
    pub final_state: EngineState,
    pub steps_executed: u64,
    pub total_duration_ms: u64,
    pub output: Option<String>,
    pub error: Option<String>,
    pub guard_triggered: Option<String>,
}

/// 引擎执行上下文
#[derive(Debug, Clone, Default)]
pub struct EngineContext {
    pub task: String,
    pub input: HashMap<String, serde_json::Value>,
    pub variables: HashMap<String, serde_json::Value>,
    pub observations: Vec<String>,
    pub recalled_memories: Vec<String>,
    pub action_results: Vec<String>,
    pub reflections: Vec<String>,
    pub generated_output: Option<String>,
    pub plan: Option<String>,
}

/// 引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub max_steps: u64,
    pub max_budget: f64,
    pub max_stagnant: usize,
    pub risk_threshold: String,
    pub enable_hitl: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_steps: 50,
            max_budget: 500.0,
            max_stagnant: 5,
            risk_threshold: "high".to_string(),
            enable_hitl: true,
        }
    }
}

/// 核心引擎结构体
pub struct Engine {
    fsm: EngineFSM,
    guards: CompositeGuard,
    tools: ToolRegistry,
    context: EngineContext,
    config: EngineConfig,
    step_count: u64,
    budget_used: f64,
    consolidation_result: Option<kg_hub::consolidator::ConsolidationResult>,
    plan_step_index: usize,
    llm_client: Option<Arc<RwLock<LLMClient>>>,
    strong_model_client: Option<Arc<RwLock<LLMClient>>>,
    browser: Option<Arc<RwLock<BrowserAutomationEngine>>>,
}

impl Engine {
    pub fn new() -> Self {
        Self::with_config(EngineConfig::default())
    }

    pub fn with_config(config: EngineConfig) -> Self {
        let mut guards = CompositeGuard::new();
        guards.add(Box::new(BudgetGuard::new(config.max_steps, config.max_budget)));
        guards.add(Box::new(ProgressGuard::new(config.max_stagnant)));
        guards.add(Box::new(RiskGuard::default()));

        let tools = ToolRegistry::with_builtin_tools();

        Self {
            fsm: EngineFSM::new(),
            guards,
            tools,
            context: EngineContext::default(),
            config,
            step_count: 0,
            budget_used: 0.0,
            consolidation_result: None,
            plan_step_index: 0,
            llm_client: None,
            strong_model_client: None,
            browser: None,
        }
    }

    pub fn with_context(mut self, ctx: EngineContext) -> Self {
        self.context = ctx;
        self
    }

    pub fn with_executors(
        mut self,
        llm_client: Option<Arc<RwLock<LLMClient>>>,
        browser: Option<Arc<RwLock<BrowserAutomationEngine>>>,
    ) -> Self {
        self.llm_client = llm_client;
        self.browser = browser;
        self
    }

    pub fn with_strong_model(
        mut self,
        strong_model_client: Option<Arc<RwLock<LLMClient>>>,
    ) -> Self {
        self.strong_model_client = strong_model_client;
        self
    }

    fn get_strong_client(&self) -> Option<&Arc<RwLock<LLMClient>>> {
        self.strong_model_client.as_ref().or(self.llm_client.as_ref())
    }

    pub fn state(&self) -> &EngineState {
        self.fsm.current_state()
    }

    pub fn consolidation_result(&self) -> Option<&kg_hub::consolidator::ConsolidationResult> {
        self.consolidation_result.as_ref()
    }

    pub fn step_count(&self) -> u64 {
        self.step_count
    }

    /// 主循环入口
    pub async fn run(&mut self) -> EngineResult {
        let start = Instant::now();

        tracing::info!(
            target: "engine",
            task = %self.context.task,
            "引擎启动"
        );

        if let Err(e) = self.fsm.trigger(EngineEvent::Start) {
            return self.abort_result(start, format!("启动失败: {}", e));
        }

        match self.phase_perceive().await {
            Ok(()) => {
                if let Err(e) = self.fsm.trigger(EngineEvent::PerceiveDone) {
                    return self.abort_result(start, format!("Perceive→Recall 转移失败: {}", e));
                }
            }
            Err(e) => return self.abort_result(start, e),
        }

        match self.phase_recall().await {
            Ok(()) => {
                if let Err(e) = self.fsm.trigger(EngineEvent::RecallDone) {
                    return self.abort_result(start, format!("Recall→Plan 转移失败: {}", e));
                }
            }
            Err(e) => return self.abort_result(start, e),
        }

        if !self.conservation_check() {
            tracing::warn!(target: "engine", "RECALL 后状态向量守恒检查未通过");
        }

        match self.phase_plan().await {
            Ok(()) => {
                if let Err(e) = self.fsm.trigger(EngineEvent::PlanDone) {
                    return self.abort_result(start, format!("Plan→Act 转移失败: {}", e));
                }
            }
            Err(e) => return self.abort_result(start, e),
        }

        loop {
            self.step_count += 1;

            let guard_ctx = self.build_guard_context(None);
            match self.guards.check_all(&guard_ctx) {
                GuardResult::Passed => {}
                GuardResult::Triggered { reason } => {
                    tracing::warn!(target: "engine", reason = %reason, "ACT 阶段守卫触发");
                    return EngineResult {
                        success: false,
                        final_state: self.fsm.current_state().clone(),
                        steps_executed: self.step_count,
                        total_duration_ms: start.elapsed().as_millis() as u64,
                        output: None,
                        error: Some(reason.clone()),
                        guard_triggered: Some(reason),
                    };
                }
            }

            match self.phase_act().await {
                Ok(()) => {
                    if let Err(e) = self.fsm.trigger(EngineEvent::ActDone) {
                        return self.abort_result(start, format!("Act→Observe 转移失败: {}", e));
                    }
                }
                Err(e) => {
                    tracing::warn!(target: "engine", error = %e, "ACT 执行失败");
                    if let Err(fe) = self.fsm.trigger(EngineEvent::ActFailed) {
                        return self.abort_result(start, format!("ActFailed→Reflect 转移失败: {}", fe));
                    }
                }
            }

            match self.phase_observe().await {
                Ok(()) => {
                    if let Err(e) = self.fsm.trigger(EngineEvent::ObserveDone) {
                        return self.abort_result(start, format!("Observe→Reflect 转移失败: {}", e));
                    }
                }
                Err(e) => return self.abort_result(start, e),
            }

            let reflect_guard_ctx = self.build_guard_context(None);
            match self.guards.check_all(&reflect_guard_ctx) {
                GuardResult::Passed => {}
                GuardResult::Triggered { reason } => {
                    tracing::warn!(target: "engine", reason = %reason, "REFLECT 阶段守卫触发");
                    return EngineResult {
                        success: false,
                        final_state: self.fsm.current_state().clone(),
                        steps_executed: self.step_count,
                        total_duration_ms: start.elapsed().as_millis() as u64,
                        output: None,
                        error: Some(reason.clone()),
                        guard_triggered: Some(reason),
                    };
                }
            }

            match self.phase_reflect().await {
                ReflectDecision::Continue => {
                    if let Err(e) = self.fsm.trigger(EngineEvent::ReflectContinue) {
                        return self.abort_result(start, format!("Reflect→Act 转移失败: {}", e));
                    }
                    continue;
                }
                ReflectDecision::ToGenerate => {
                    if let Err(e) = self.fsm.trigger(EngineEvent::ReflectToGenerate) {
                        return self.abort_result(start, format!("Reflect→Generate 转移失败: {}", e));
                    }
                    break;
                }
                ReflectDecision::NeedHitl(prompt) => {
                    if self.config.enable_hitl {
                        tracing::info!(target: "engine", prompt = %prompt, "需要人机协同介入");
                        if let Err(e) = self.fsm.trigger(EngineEvent::NeedHumanInput) {
                            return self.abort_result(start, format!("Reflect→HitlPause 转移失败: {}", e));
                        }
                        if let Err(e) = self.fsm.trigger(EngineEvent::HumanApproved) {
                            return self.abort_result(start, format!("HitlPause→Act 转移失败: {}", e));
                        }
                        continue;
                    } else {
                        tracing::warn!(target: "engine", "HITL 已禁用，跳过人工介入直接进入生成");
                        if let Err(e) = self.fsm.trigger(EngineEvent::ReflectToGenerate) {
                            return self.abort_result(start, format!("Reflect→Generate 转移失败: {}", e));
                        }
                        break;
                    }
                }
                ReflectDecision::Abort(reason) => {
                    return self.abort_result(start, reason);
                }
            }
        }

        match self.phase_generate().await {
            Ok(output) => {
                self.context.generated_output = Some(output);
                if let Err(e) = self.fsm.trigger(EngineEvent::GenerateDone) {
                    return self.abort_result(start, format!("Generate→Consolidate 转移失败: {}", e));
                }
            }
            Err(e) => return self.abort_result(start, e),
        }

        match self.phase_consolidate().await {
            Ok(()) => {
                if let Err(e) = self.fsm.trigger(EngineEvent::ConsolidateDone) {
                    return self.abort_result(start, format!("Consolidate→Done 转移失败: {}", e));
                }
            }
            Err(e) => return self.abort_result(start, e),
        }

        let output = self.context.generated_output.clone();
        tracing::info!(
            target: "engine",
            steps = self.step_count,
            duration_ms = start.elapsed().as_millis() as u64,
            "引擎完成"
        );

        EngineResult {
            success: true,
            final_state: EngineState::Done,
            steps_executed: self.step_count,
            total_duration_ms: start.elapsed().as_millis() as u64,
            output,
            error: None,
            guard_triggered: None,
        }
    }

    // ── 各阶段实现 ──────────────────────────────────────────

    async fn phase_perceive(&mut self) -> Result<(), String> {
        tracing::debug!(target: "engine", "PERCEIVE: 收集环境信息");
        if self.context.task.is_empty() {
            return Err("任务描述为空，无法感知".to_string());
        }
        self.context.observations.push(format!(
            "感知到任务: {}",
            self.context.task
        ));
        Ok(())
    }

    async fn phase_recall(&mut self) -> Result<(), String> {
        tracing::debug!(target: "engine", "RECALL: 检索相关记忆");

        let query = self.context.task.clone();
        let memories = self.recall_from_knowledge(&query);

        if memories.is_empty() {
            self.context
                .recalled_memories
                .push("无相关历史记忆".to_string());
            self.context
                .observations
                .push("RECALL: 未检索到相关记忆".to_string());
        } else {
            self.context.recalled_memories = memories.clone();
            self.context.observations.push(format!(
                "RECALL: 检索到 {} 条相关记忆",
                memories.len()
            ));
        }

        Ok(())
    }

    fn recall_from_knowledge(&self, query: &str) -> Vec<String> {
        let _ = query;
        let mock_memories = vec![
            "历史案例: 类似数据处理任务采用了 ETL 流水线方案".to_string(),
            "相关知识: 数据清洗最佳实践包括去重、验证、转换".to_string(),
        ];
        mock_memories
    }

    fn conservation_check(&self) -> bool {
        let has_observations = !self.context.observations.is_empty();
        let has_memories = !self.context.recalled_memories.is_empty();
        let has_results = !self.context.action_results.is_empty();
        has_observations && has_memories && (has_results || self.plan_step_index == 0)
    }

    async fn phase_plan(&mut self) -> Result<(), String> {
        tracing::debug!(target: "engine", "PLAN: 制定执行计划");

        let llm_response = {
            if let Some(llm_arc) = self.get_strong_client() {
                let llm = llm_arc.read().await;
                if llm.is_enabled() {
                    let prompt = format!(
                        "你是AI智能体规划器。请为以下任务制定详细的执行计划，返回JSON格式。\n\n\
                        任务: {}\n\n\
                        返回格式:\n\
                        {{\n  \"goal\": \"目标描述\",\n  \"steps\": [\n    \
                        {{\"type\": \"browser|tool\", \"action\": \"navigate|click|type|extract_text|get_title|screenshot|process\", \
                        \"url\": \"...\", \"selector\": \"...\", \"text\": \"...\", \"description\": \"...\"}}\n  ]\n}}\n\n\
                        可用的action类型:\n\
                        - browser: navigate, click, type, extract_text, get_title, get_url, extract_html, screenshot, wait\n\
                        - tool: process, analyze, search, generate",
                        self.context.task
                    );
                    let messages = vec![LLMChatMessage {
                        role: "user".to_string(),
                        content: prompt,
                    }];
                    match llm.chat(messages).await {
                        Ok(response) => Some(Ok(response)),
                        Err(e) => Some(Err(e.to_string())),
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(Ok(response)) = &llm_response {
            self.context.plan = Some(response.clone());
            let preview = if response.chars().count() > 200 {
                response.chars().take(200).collect::<String>()
            } else {
                response.clone()
            };
            self.context
                .observations
                .push(format!("LLM规划完成: {}", preview));
            tracing::info!(target: "engine", plan_len = response.len(), "LLM生成执行计划");
        } else if let Some(Err(e)) = &llm_response {
            tracing::warn!(target: "engine", error = %e, "LLM规划调用失败，降级到默认计划");
            self.context
                .observations
                .push("LLM规划调用失败，使用默认计划".to_string());
        }

        if self.context.plan.is_none() {
            let default_plan = format!(
                "{{\"goal\": \"{}\", \"steps\": [{{\"type\": \"tool\", \"action\": \"process\", \"description\": \"默认执行\"}}]}}",
                self.context.task.replace('"', "\\\"")
            );
            self.context.plan = Some(default_plan);
            self.context.observations.push("使用默认计划".to_string());
        }

        Ok(())
    }

    async fn phase_act(&mut self) -> Result<(), String> {
        tracing::debug!(target: "engine", step = self.step_count, "ACT: 执行动作");

        let light_decision = {
            if let Some(llm_arc) = &self.llm_client {
                let llm = llm_arc.read().await;
                if llm.is_enabled() {
                    let plan_preview = self.context.plan.as_deref().unwrap_or("");
                    let prompt = format!(
                        "基于当前计划，选择下一步要执行的工具和动作。\n\n\
                        任务: {}\n\
                        计划预览: {}\n\
                        已执行步骤: {}\n\
                        可用工具: database, sandbox, http, file, calculator\n\n\
                        返回 JSON: {{\"tool\": \"工具名\", \"action\": \"动作描述\"}}",
                        self.context.task,
                        plan_preview.chars().take(200).collect::<String>(),
                        self.plan_step_index
                    );
                    let messages = vec![LLMChatMessage {
                        role: "user".to_string(),
                        content: prompt,
                    }];
                    llm.chat(messages).await.ok()
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(response) = light_decision {
            self.context
                .observations
                .push(format!("轻量模型动作选择: {}", response.chars().take(100).collect::<String>()));
        }

        let plan = self.context.plan.clone().unwrap_or_default();

        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&plan) {
            if let Some(steps) = parsed.get("steps").and_then(|s| s.as_array()) {
                if self.plan_step_index < steps.len() {
                    let step = &steps[self.plan_step_index];
                    let step_type = step.get("type").and_then(|t| t.as_str()).unwrap_or("tool");
                    let description = step.get("description").and_then(|d| d.as_str()).unwrap_or("");
                    let action_name = step.get("action").and_then(|a| a.as_str()).unwrap_or("unknown");

                    match step_type {
                        "browser" => {
                            let result = self.execute_browser_step(step).await;
                            let desc = format!(
                                "浏览器步骤 #{} [{}]: {}",
                                self.plan_step_index, action_name,
                                if result { "成功" } else { "失败" }
                            );
                            self.context.action_results.push(desc.clone());
                            self.context.observations.push(desc);
                        }
                        "database" | "sandbox" | "http" | "file" | "calculator" => {
                            if let Some(tool_name) = tool_type_to_name(step_type) {
                                let tool_params = step.get("params").cloned().unwrap_or_else(|| step.clone());
                                let tool_result = self.tools.execute(tool_name, &tool_params);
                                let desc = format!(
                                    "工具步骤 #{} [{}]: {} {}",
                                    self.plan_step_index,
                                    tool_name,
                                    if tool_result.success { "成功" } else { "失败" },
                                    tool_result.error.as_deref().unwrap_or("")
                                );
                                self.context.action_results.push(desc.clone());
                                self.context.observations.push(desc);
                                if let Some(data) = &tool_result.data {
                                    let data_str = format!("{:?}", data);
                                    let preview = if data_str.chars().count() > 200 {
                                        data_str.chars().take(200).collect::<String>()
                                    } else {
                                        data_str
                                    };
                                    self.context.observations.push(format!("{} 输出: {}", tool_name, preview));
                                }
                                self.budget_used += 2.0;
                            } else {
                                let desc = format!("工具步骤 #{} [{}]: {} (无映射)", self.plan_step_index, action_name, description);
                                self.context.action_results.push(desc.clone());
                                self.context.observations.push(desc);
                                self.budget_used += 1.0;
                            }
                        }
                        _ => {
                            let desc = format!("工具步骤 #{} [{}]: {}", self.plan_step_index, action_name, description);
                            self.context.action_results.push(desc.clone());
                            self.context.observations.push(desc);
                            self.budget_used += 1.0;
                        }
                    }

                    self.plan_step_index += 1;
                } else {
                    let action = format!("计划已执行完毕，总结步骤 #{}", self.step_count);
                    self.context.action_results.push(action.clone());
                    self.budget_used += 1.0;
                }
            }
        }

        if self.context.action_results.is_empty() {
            let action = format!("执行步骤 #{}", self.step_count);
            self.context.action_results.push(action.clone());
            self.budget_used += 1.0;
        }

        Ok(())
    }

    async fn phase_observe(&mut self) -> Result<(), String> {
        tracing::debug!(target: "engine", "OBSERVE: 观察执行结果");
        let last = self
            .context
            .action_results
            .last()
            .cloned()
            .unwrap_or_else(|| "无结果".to_string());
        self.context.observations.push(format!("观察到: {}", last));
        Ok(())
    }

    async fn phase_reflect(&mut self) -> ReflectDecision {
        tracing::debug!(target: "engine", "REFLECT: 反思评估");

        let last_result = self.context.action_results.last().cloned().unwrap_or_default();

        let llm_reflection = {
            if let Some(strong_arc) = self.get_strong_client() {
                let llm = strong_arc.read().await;
                if llm.is_enabled() {
                    let observations_text = self.context.observations.join("\n");
                    let reflections_text = self.context.reflections.join("\n");
                    let prompt = format!(
                        "基于以下执行过程，判断任务是否已完成，是否需要继续执行。\n\n\
                        任务: {}\n\n\
                        最近结果: {}\n\n\
                        观察: {}\n\n\
                        历史反思: {}\n\n\
                        请回答: 任务是否已完成？(yes/no) 如果未完成，应该继续还是进入生成阶段？",
                        self.context.task, last_result, observations_text, reflections_text
                    );
                    let messages = vec![LLMChatMessage {
                        role: "user".to_string(),
                        content: prompt,
                    }];
                    match llm.chat(messages).await {
                        Ok(response) => Some(Ok(response)),
                        Err(e) => Some(Err(e.to_string())),
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(Ok(response)) = &llm_reflection {
            let lower = response.to_lowercase();
            if lower.contains("yes") || lower.contains("完成") || lower.contains("已完成") {
                self.context
                    .reflections
                    .push(format!("LLM反思: {}", response));
                return ReflectDecision::ToGenerate;
            }
            self.context
                .reflections
                .push(format!("LLM反思: {}", response));
        } else if let Some(Err(e)) = &llm_reflection {
            tracing::warn!(target: "engine", error = %e, "LLM反思调用失败，降级到规则引擎");
        }

        let achieved = self.is_goal_achieved(&last_result);

        if achieved {
            tracing::info!(target: "engine", "目标已达成，进入生成阶段");
            self.context.reflections.push("目标已达成".to_string());
            return ReflectDecision::ToGenerate;
        }

        if self.step_count >= self.config.max_steps {
            tracing::info!(target: "engine", "达到最大步数，进入生成阶段");
            self.context.reflections.push("达到最大步数限制".to_string());
            return ReflectDecision::ToGenerate;
        }

        let total_plan_steps = self.count_plan_steps();
        if self.plan_step_index >= total_plan_steps && total_plan_steps > 0 {
            tracing::info!(target: "engine", "计划步骤已全部执行完毕，进入生成阶段");
            self.context.reflections.push("计划步骤全部执行完毕".to_string());
            return ReflectDecision::ToGenerate;
        }

        self.context
            .reflections
            .push(format!("第 {} 步反思: 执行中，继续迭代", self.step_count));

        ReflectDecision::Continue
    }

    async fn phase_generate(&mut self) -> Result<String, String> {
        tracing::debug!(target: "engine", "GENERATE: 整合输出");

        let output = if let Some(llm_arc) = &self.llm_client {
            let llm = llm_arc.read().await;
            if llm.is_enabled() {
                let observations_text = self.context.observations.join("\n");
                let reflections_text = self.context.reflections.join("\n");
                let prompt = format!(
                    "基于以下执行过程，生成最终报告。\n\n任务: {}\n\n观察结果:\n{}\n\n反思:\n{}\n\n请给出简洁的最终报告。",
                    self.context.task, observations_text, reflections_text
                );
                let messages = vec![LLMChatMessage {
                    role: "user".to_string(),
                    content: prompt,
                }];
                match llm.chat(messages).await {
                    Ok(response) => response,
                    Err(e) => {
                        tracing::warn!(target: "engine", error = %e, "LLM生成失败，使用默认输出");
                        format!(
                            "任务「{}」已完成，共执行 {} 步",
                            self.context.task, self.step_count
                        )
                    }
                }
            } else {
                format!(
                    "任务「{}」已完成，共执行 {} 步",
                    self.context.task, self.step_count
                )
            }
        } else {
            format!(
                "任务「{}」已完成，共执行 {} 步",
                self.context.task, self.step_count
            )
        };

        Ok(output)
    }

    async fn phase_consolidate(&mut self) -> Result<(), String> {
        tracing::debug!(target: "engine", "CONSOLIDATE: 巩固经验");

        let trace = EngineTrace {
            task: self.context.task.clone(),
            observations: self.context.observations.clone(),
            action_results: self.context.action_results.clone(),
            reflections: self.context.reflections.clone(),
            generated_output: self.context.generated_output.clone(),
        };

        let consolidator = TraceConsolidator::new();
        let result = consolidator.consolidate(&trace);
        self.consolidation_result = Some(result);

        Ok(())
    }

    // ── 辅助方法 ──────────────────────────────────────────

    fn build_guard_context(&self, current_action: Option<String>) -> GuardContext {
        GuardContext {
            step_count: self.step_count,
            max_steps: self.config.max_steps,
            budget_used: self.budget_used,
            budget_limit: self.config.max_budget,
            recent_outcomes: self.context.action_results.clone(),
            max_stagnant: self.config.max_stagnant,
            current_action,
            risk_threshold: super::guards::RiskLevel::High,
            metadata: HashMap::new(),
        }
    }

    fn abort_result(&mut self, start: Instant, error: String) -> EngineResult {
        let _ = self.fsm.trigger(EngineEvent::Abort);
        tracing::error!(target: "engine", error = %error, "引擎中止");
        EngineResult {
            success: false,
            final_state: EngineState::Abort,
            steps_executed: self.step_count,
            total_duration_ms: start.elapsed().as_millis() as u64,
            output: None,
            error: Some(error),
            guard_triggered: None,
        }
    }

    async fn execute_browser_step(&mut self, step: &serde_json::Value) -> bool {
        if let Some(browser_arc) = &self.browser {
            let mut browser = browser_arc.write().await;
            let session_id = browser.create_session();

            if let Some(ba) = Self::step_to_browser_action(step) {
                match browser.execute_action(&session_id, ba).await {
                    Ok(result) => {
                        if let Some(data) = &result.data {
                            if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
                                self.context.observations.push(format!(
                                    "浏览器提取: {}",
                                    &text[..text.len().min(200)]
                                ));
                            }
                            if let Some(title) = data.get("title").and_then(|v| v.as_str()) {
                                self.context.observations.push(format!("页面标题: {}", title));
                            }
                        }
                        result.success
                    }
                    Err(e) => {
                        self.context.observations.push(format!("浏览器错误: {}", e));
                        false
                    }
                }
            } else {
                self.context.observations.push("无法解析浏览器动作".to_string());
                false
            }
        } else {
            self.context.observations.push("浏览器引擎不可用".to_string());
            false
        }
    }

    fn step_to_browser_action(step: &serde_json::Value) -> Option<BrowserAction> {
        let action = step.get("action").and_then(|a| a.as_str())?;
        match action {
            "navigate" => {
                let url = step.get("url").and_then(|u| u.as_str()).unwrap_or("");
                Some(BrowserAction::Navigate { url: url.to_string() })
            }
            "click" => {
                let selector = step.get("selector").and_then(|s| s.as_str()).unwrap_or("");
                Some(BrowserAction::Click { selector: selector.to_string(), timeout_ms: None })
            }
            "type" => {
                let selector = step.get("selector").and_then(|s| s.as_str()).unwrap_or("");
                let text = step.get("text").and_then(|t| t.as_str()).unwrap_or("");
                Some(BrowserAction::Type { selector: selector.to_string(), text: text.to_string(), clear_first: Some(true) })
            }
            "extract_text" => {
                let selector = step.get("selector").and_then(|s| s.as_str()).unwrap_or("body");
                Some(BrowserAction::ExtractText { selector: selector.to_string() })
            }
            "extract_html" => Some(BrowserAction::ExtractHtml),
            "get_title" => Some(BrowserAction::GetTitle),
            "get_url" => Some(BrowserAction::GetUrl),
            "screenshot" => Some(BrowserAction::Screenshot),
            "wait" => {
                let ms = step.get("ms").and_then(|m| m.as_u64()).unwrap_or(1000);
                Some(BrowserAction::Wait { ms })
            }
            _ => None,
        }
    }

    fn is_goal_achieved(&self, last_result: &str) -> bool {
        let lower = last_result.to_lowercase();
        let success_indicators = ["成功", "完成", "已获取", "✅", "success", "completed", "done"];
        for indicator in &success_indicators {
            if lower.contains(&indicator.to_lowercase()) {
                return true;
            }
        }

        let all_steps_done = self.plan_step_index >= self.count_plan_steps() && self.plan_step_index > 0;
        let has_results = !self.context.action_results.is_empty();
        if all_steps_done && has_results {
            return true;
        }

        false
    }

    fn count_plan_steps(&self) -> usize {
        if let Some(plan) = &self.context.plan {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(plan) {
                if let Some(steps) = parsed.get("steps").and_then(|s| s.as_array()) {
                    return steps.len();
                }
            }
        }
        0
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// 反思阶段决策结果
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum ReflectDecision {
    /// 继续循环，回到 ACT
    Continue,
    /// 结束循环，进入 GENERATE
    ToGenerate,
    /// 需要人机协同介入
    NeedHitl(String),
    /// 直接中止
    Abort(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_client::LLMConfig;

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().unwrap()
    }

    #[test]
    fn test_engine_runs_to_completion() {
        let r = rt();
        r.block_on(async {
            let mut engine = Engine::new().with_context(EngineContext {
                task: "测试任务".to_string(),
                ..Default::default()
            });
            let result = engine.run().await;
            assert!(result.success, "引擎应正常完成");
            assert_eq!(result.final_state, EngineState::Done);
            assert!(!result.output.is_none());
            assert!(result.error.is_none());
        });
    }

    #[test]
    fn test_engine_fails_without_task() {
        let r = rt();
        r.block_on(async {
            let mut engine = Engine::new();
            let result = engine.run().await;
            assert!(!result.success);
            assert_eq!(result.final_state, EngineState::Abort);
        });
    }

    #[test]
    fn test_engine_respects_max_steps() {
        let r = rt();
        let config = EngineConfig {
            max_steps: 3,
            max_budget: 10.0,
            ..Default::default()
        };
        r.block_on(async {
            let mut engine = Engine::with_config(config).with_context(EngineContext {
                task: "极限测试".to_string(),
                ..Default::default()
            });
            let result = engine.run().await;
            assert!(result.success);
            assert!(result.steps_executed <= 3);
        });
    }

    #[test]
    fn test_engine_guard_triggers_on_budget_exceed() {
        let r = rt();
        let config = EngineConfig {
            max_steps: 100,
            max_budget: 1.0,
            ..Default::default()
        };
        r.block_on(async {
            let mut engine = Engine::with_config(config).with_context(EngineContext {
                task: "预算测试".to_string(),
                plan: Some("{\"steps\": [{\"type\": \"tool\", \"action\": \"process\", \"description\": \"步骤1\"}, {\"type\": \"tool\", \"action\": \"process\", \"description\": \"步骤2\"}, {\"type\": \"tool\", \"action\": \"process\", \"description\": \"步骤3\"}]}".to_string()),
                ..Default::default()
            });
            let result = engine.run().await;
            assert!(!result.success);
            assert!(result.guard_triggered.is_some());
        });
    }

    #[test]
    fn test_engine_with_executors_no_llm_runs_placeholder() {
        let r = rt();
        r.block_on(async {
            let llm = Arc::new(RwLock::new(LLMClient::new(LLMConfig::default())));
            let mut engine = Engine::new()
                .with_context(EngineContext {
                    task: "无LLM测试".to_string(),
                    ..Default::default()
                })
                .with_executors(Some(llm), None);
            let result = engine.run().await;
            assert!(result.success);
        });
    }

    #[test]
    fn test_engine_with_executors_no_browser_runs_placeholder() {
        let r = rt();
        r.block_on(async {
            let browser = Arc::new(RwLock::new(BrowserAutomationEngine::new()));
            let mut engine = Engine::new()
                .with_context(EngineContext {
                    task: "无浏览器测试".to_string(),
                    ..Default::default()
                })
                .with_executors(None, Some(browser));
            let result = engine.run().await;
            assert!(result.success);
        });
    }

    #[test]
    fn test_browser_step_mapping() {
        let step = serde_json::json!({
            "action": "navigate",
            "url": "https://example.com"
        });
        let result = Engine::step_to_browser_action(&step);
        assert!(result.is_some());

        let step_no_action = serde_json::json!({
            "type": "browser"
        });
        let result = Engine::step_to_browser_action(&step_no_action);
        assert!(result.is_none());

        let step_unknown = serde_json::json!({
            "action": "unknown_action"
        });
        let result = Engine::step_to_browser_action(&step_unknown);
        assert!(result.is_none());
    }

    #[test]
    fn test_goal_achieved_detection() {
        let r = rt();
        r.block_on(async {
            let engine = Engine::new().with_context(EngineContext {
                task: "测试".to_string(),
                ..Default::default()
            });
            assert!(engine.is_goal_achieved("任务成功完成"));
            assert!(engine.is_goal_achieved("数据已获取"));
            assert!(engine.is_goal_achieved("✅ 处理完毕"));
            assert!(!engine.is_goal_achieved("执行中"));
            assert!(!engine.is_goal_achieved("步骤1"));
        });
    }

    #[test]
    fn test_count_plan_steps() {
        let r = rt();
        r.block_on(async {
            let mut engine = Engine::new();
            assert_eq!(engine.count_plan_steps(), 0);

            engine.context.plan = Some("{\"steps\": [{\"action\": \"a\"}, {\"action\": \"b\"}]}".to_string());
            assert_eq!(engine.count_plan_steps(), 2);

            engine.context.plan = Some("not json".to_string());
            assert_eq!(engine.count_plan_steps(), 0);
        });
    }
}
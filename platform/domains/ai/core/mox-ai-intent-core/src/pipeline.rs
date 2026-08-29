// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 端到端意图理解管道：用户输入 → 分类 → 实体提取 → 任务拆解 → 输出结构化结果。
//!
//! ## 管道阶段
//! ```text
//! 用户输入
//!   │
//!   ▼
//! ┌─────────────┐
//! │ 1. 预处理   │  去除空白、规范化、敏感词过滤
//! └──────┬──────┘
//!        │
//!        ▼
//! ┌─────────────┐
//! │ 2. 意图分类 │  Aho-Corasick + 评分排序
//! └──────┬──────┘
//!        │
//!        ▼
//! ┌─────────────┐
//! │ 3. 实体提取 │  时间/数字/参数/领域实体
//! └──────┬──────┘
//!        │
//!        ▼
//! ┌─────────────┐
//! │ 4. 任务拆解 │  基于意图模板生成执行计划
//! └──────┬──────┘
//!        │
//!        ▼
//! ┌─────────────┐
//! │ 5. Agent匹配│  专家联盟打分，推荐执行者
//! └──────┬──────┘
//!        │
//!        ▼
//! 结构化意图结果
//! ```
//!
//! ## 设计原则
//! - 纯同步、零 I/O、单线程 1ms 内完成（P1 目标）
//! - 每阶段独立可替换，P2 可接入 LLM 做语义增强
//! - 结果可序列化，方便跨进程 / FFI 传递

use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::alliance::{AllianceScorer, ExpertCandidate, ScoredExpert};
use crate::builtins::IntentRegistry;
use crate::classifier::{IntentClassifier, IntentPattern, IntentResult};
use crate::entity::{Entity, EntityExtractor, EntityType};
use crate::task_decomp::{RiskLevel, TaskDecomposer, TaskPlan};

// ─── 管道输出 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentUnderstanding {
    /// 请求 ID
    pub request_id: String,
    /// 原始用户输入
    pub query: String,
    /// 预处理后的文本
    pub cleaned_query: String,
    /// 意图分类结果
    pub intent: IntentResult,
    /// 提取到的实体
    pub entities: Vec<Entity>,
    /// 任务计划
    pub task_plan: TaskPlan,
    /// 推荐的 Agent / 专家（TOP-K）
    pub recommended_agents: Vec<ScoredExpert>,
    /// 人机协同建议
    pub collaboration: CollaborationSuggestion,
    /// 各阶段耗时（ms）
    pub timing: PipelineTiming,
    /// 整体置信度 0..1
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTiming {
    pub preprocess_ms: u64,
    pub classify_ms: u64,
    pub entity_extract_ms: u64,
    pub task_decompose_ms: u64,
    pub agent_match_ms: u64,
    pub total_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSuggestion {
    /// 最高风险等级
    pub max_risk: String,
    /// 是否需要用户确认
    pub needs_confirmation: bool,
    /// 需要确认的步骤列表
    pub confirmation_steps: Vec<String>,
    /// 建议的弹框方向（用于四向弹框体系）
    pub suggested_panel: PanelDirection,
    /// 建议的交互方式
    pub interaction_mode: InteractionMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelDirection {
    /// 右侧抽屉：详情、列表、配置
    Right,
    /// 顶部下拉：搜索、快速命令
    Top,
    /// 底部上滑：参数确认、进度
    Bottom,
    /// 居中弹框：敏感确认
    Center,
    /// 内嵌：直接在对话中展示
    Inline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionMode {
    /// 直接执行（低风险只读）
    AutoExecute,
    /// 一键确认（中风险）
    OneClickConfirm,
    /// 二次确认（高风险）
    DoubleConfirm,
    /// 多轮澄清（信息不足）
    MultiTurnClarify,
}

// ─── 管道配置 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// 低置信度阈值（低于此值进入多轮澄清）
    pub low_confidence_threshold: f32,
    /// TOP-K Agent 推荐数量
    pub top_k_agents: usize,
    /// 是否启用内置意图
    pub enable_builtin_intents: bool,
    /// 是否启用实体提取
    pub enable_entity_extraction: bool,
    /// 是否启用任务拆解
    pub enable_task_decomposition: bool,
    /// 是否启用 Agent 匹配
    pub enable_agent_matching: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            low_confidence_threshold: 0.3,
            top_k_agents: 3,
            enable_builtin_intents: true,
            enable_entity_extraction: true,
            enable_task_decomposition: true,
            enable_agent_matching: false, // P1 默认关闭，需注册 experts 后启用
        }
    }
}

// ─── 管道实现 ────────────────────────────────────────────────────────────────

pub struct IntentPipeline {
    config: PipelineConfig,
    classifier: IntentClassifier,
    entity_extractor: EntityExtractor,
    task_decomposer: TaskDecomposer,
    /// Agent / 专家候选池（可选）
    experts: Vec<ExpertCandidate>,
    expert_scorer: Option<AllianceScorer>,
}

impl IntentPipeline {
    /// 使用默认配置 + 内置意图创建管道
    pub fn new() -> Self {
        let config = PipelineConfig::default();
        Self::with_config(config)
    }

    pub fn with_config(config: PipelineConfig) -> Self {
        let patterns = if config.enable_builtin_intents {
            IntentRegistry::all_patterns()
        } else {
            vec![]
        };

        let classifier = IntentClassifier::new(patterns);
        let entity_extractor = EntityExtractor::new();
        let task_decomposer = TaskDecomposer::new();

        Self {
            config,
            classifier,
            entity_extractor,
            task_decomposer,
            experts: vec![],
            expert_scorer: None,
        }
    }

    /// 注册自定义意图模式
    pub fn add_intent_patterns(&mut self, patterns: Vec<IntentPattern>) {
        // 重建分类器（P1 简单实现，P2 可做增量）
        let mut all_patterns = IntentRegistry::all_patterns();
        all_patterns.extend(patterns);
        self.classifier = IntentClassifier::new(all_patterns);
    }

    /// 注册领域实体（项目/图谱/数据集/Agent 名）
    pub fn register_domain_entity(&mut self, etype: EntityType, name: &str, aliases: &[&str]) {
        self.entity_extractor.register_domain_entity(etype, name, aliases);
    }

    /// 注册专家 / Agent（用于匹配推荐）
    pub fn register_experts(&mut self, experts: Vec<ExpertCandidate>) {
        self.experts = experts;
        if !self.experts.is_empty() {
            self.expert_scorer = Some(AllianceScorer::new(self.experts.clone()));
            self.config.enable_agent_matching = true;
        }
    }

    // ── 主入口 ────────────────────────────────────────────────────────────

    pub fn process(&self, query: &str) -> IntentUnderstanding {
        let total_start = Instant::now();
        let mut timing = PipelineTiming {
            preprocess_ms: 0,
            classify_ms: 0,
            entity_extract_ms: 0,
            task_decompose_ms: 0,
            agent_match_ms: 0,
            total_ms: 0,
        };

        // 阶段 1: 预处理
        let t0 = Instant::now();
        let cleaned = self.preprocess(query);
        timing.preprocess_ms = t0.elapsed().as_micros() as u64 / 1000;

        // 阶段 2: 意图分类
        let t0 = Instant::now();
        let intent = self.classifier.classify(&cleaned);
        timing.classify_ms = t0.elapsed().as_micros() as u64 / 1000;

        // 阶段 3: 实体提取
        let t0 = Instant::now();
        let entities = if self.config.enable_entity_extraction {
            self.entity_extractor.extract(&cleaned)
        } else {
            vec![]
        };
        timing.entity_extract_ms = t0.elapsed().as_micros() as u64 / 1000;

        // 阶段 4: 任务拆解
        let t0 = Instant::now();
        let task_plan = if self.config.enable_task_decomposition {
            self.task_decomposer.decompose(&intent.primary, &entities, &cleaned)
        } else {
            // fallback: 空计划
            TaskPlan {
                plan_id: uuid::Uuid::now_v7().to_string(),
                intent: intent.primary.clone(),
                user_query: cleaned.clone(),
                steps: vec![],
                requires_overall_confirmation: false,
                parallel_groups: vec![],
                total_est_duration_sec: 0,
            }
        };
        timing.task_decompose_ms = t0.elapsed().as_micros() as u64 / 1000;

        // 阶段 5: Agent 匹配
        let t0 = Instant::now();
        let recommended_agents = if self.config.enable_agent_matching {
            self.match_agents(&cleaned, &intent)
        } else {
            vec![]
        };
        timing.agent_match_ms = t0.elapsed().as_micros() as u64 / 1000;

        // 人机协同建议
        let collaboration = self.suggest_collaboration(&intent, &entities, &task_plan);

        // 整体置信度
        let confidence = intent.confidence;

        timing.total_ms = total_start.elapsed().as_micros() as u64 / 1000;

        IntentUnderstanding {
            request_id: uuid::Uuid::now_v7().to_string(),
            query: query.to_string(),
            cleaned_query: cleaned,
            intent,
            entities,
            task_plan,
            recommended_agents,
            collaboration,
            timing,
            confidence,
        }
    }

    // ── 预处理 ────────────────────────────────────────────────────────────

    fn preprocess(&self, text: &str) -> String {
        // 去除首尾空白
        let trimmed = text.trim();
        // 合并多个空白为单个
        let mut result = String::with_capacity(trimmed.len());
        let mut prev_space = false;
        for c in trimmed.chars() {
            if c.is_whitespace() {
                if !prev_space {
                    result.push(' ');
                    prev_space = true;
                }
            } else {
                result.push(c);
                prev_space = false;
            }
        }
        result
    }

    // ── Agent 匹配 ────────────────────────────────────────────────────────

    fn match_agents(&self, query: &str, intent: &IntentResult) -> Vec<ScoredExpert> {
        let scorer = match &self.expert_scorer {
            Some(s) => s,
            None => return vec![],
        };

        let secondary: Vec<String> = intent.secondary.clone();
        let mut scored = scorer.score(
            query,
            &intent.primary,
            &secondary,
            &intent.matched_keywords,
            |_id| (1.0, 0.7), // 默认统计值，P2 接入真实统计
        );
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(self.config.top_k_agents).collect()
    }

    // ── 人机协同建议 ──────────────────────────────────────────────────────

    fn suggest_collaboration(
        &self,
        intent: &IntentResult,
        _entities: &[Entity],
        plan: &TaskPlan,
    ) -> CollaborationSuggestion {
        // 计算最高风险
        let max_risk = plan.steps.iter()
            .map(|s| s.risk)
            .max_by_key(|r| match r {
                RiskLevel::Low => 0,
                RiskLevel::Medium => 1,
                RiskLevel::High => 2,
            })
            .unwrap_or(RiskLevel::Low);

        let needs_confirmation = max_risk.requires_confirmation();

        let confirmation_steps: Vec<String> = plan.steps
            .iter()
            .filter(|s| s.risk.requires_confirmation())
            .map(|s| s.name.clone())
            .collect();

        // 低置信度 → 多轮澄清
        if intent.confidence < self.config.low_confidence_threshold {
            return CollaborationSuggestion {
                max_risk: max_risk.label().to_string(),
                needs_confirmation: false,
                confirmation_steps: vec![],
                suggested_panel: PanelDirection::Inline,
                interaction_mode: InteractionMode::MultiTurnClarify,
            };
        }

        // 按风险等级决定交互模式和弹框方向
        let (interaction_mode, panel) = match max_risk {
            RiskLevel::Low => (InteractionMode::AutoExecute, PanelDirection::Inline),
            RiskLevel::Medium => (InteractionMode::OneClickConfirm, PanelDirection::Bottom),
            RiskLevel::High => (InteractionMode::DoubleConfirm, PanelDirection::Center),
        };

        CollaborationSuggestion {
            max_risk: max_risk.label().to_string(),
            needs_confirmation,
            confirmation_steps,
            suggested_panel: panel,
            interaction_mode,
        }
    }

    // ── 便捷访问 ──────────────────────────────────────────────────────────

    pub fn intent_count(&self) -> usize {
        // 近似：分类器 pattern 数
        IntentRegistry::all().len()
    }

    pub fn entity_extractor(&self) -> &EntityExtractor {
        &self.entity_extractor
    }

    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }
}

impl Default for IntentPipeline {
    fn default() -> Self { Self::new() }
}

// ─── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_processes_simple_query() {
        let pipe = IntentPipeline::new();
        let result = pipe.process("帮我分析上个月的销售数据");
        assert!(!result.intent.primary.is_empty());
        assert!(result.confidence >= 0.0);
        assert!(result.timing.total_ms < 1000); // 1 秒内完成
    }

    #[test]
    fn pipeline_extracts_entities() {
        let pipe = IntentPipeline::new();
        let result = pipe.process("明天生成 PPT 发给销售总监");
        assert!(!result.entities.is_empty());
        // 应至少有时间 + 输出格式
        let has_time = result.entities.iter().any(|e|
            matches!(e.etype, EntityType::TimePoint | EntityType::TimeRange));
        let has_format = result.entities.iter().any(|e|
            matches!(e.etype, EntityType::OutputFormat));
        assert!(has_time, "应提取到时间实体");
        assert!(has_format, "应提取到输出格式实体");
    }

    #[test]
    fn pipeline_generates_task_plan() {
        let pipe = IntentPipeline::new();
        let result = pipe.process("生成上周的销售报告");
        assert!(!result.task_plan.steps.is_empty());
    }

    #[test]
    fn collaboration_suggestion_for_low_risk() {
        let pipe = IntentPipeline::new();
        let result = pipe.process("查一下图谱数据");
        // 低风险 → 自动执行 + 内嵌
        assert_eq!(
            result.collaboration.interaction_mode,
            InteractionMode::AutoExecute
        );
        assert!(!result.collaboration.needs_confirmation);
    }

    #[test]
    fn collaboration_suggestion_for_high_risk() {
        let pipe = IntentPipeline::new();
        let result = pipe.process("生成报告发给销售总监");
        // 高风险（发送邮件）→ 二次确认 + 居中弹框
        if result.task_plan.steps.iter().any(|s| matches!(s.risk, RiskLevel::High)) {
            assert!(result.collaboration.needs_confirmation);
            assert_eq!(
                result.collaboration.interaction_mode,
                InteractionMode::DoubleConfirm
            );
            assert_eq!(
                result.collaboration.suggested_panel,
                PanelDirection::Center
            );
        }
    }

    #[test]
    fn low_confidence_triggers_clarify() {
        let pipe = IntentPipeline::new();
        let result = pipe.process("随便说点什么吧");
        // 极低置信度应进入澄清模式
        if result.confidence < 0.3 {
            assert_eq!(
                result.collaboration.interaction_mode,
                InteractionMode::MultiTurnClarify
            );
        }
    }

    #[test]
    fn custom_domain_entities_work() {
        let mut pipe = IntentPipeline::new();
        pipe.register_domain_entity(
            EntityType::Project,
            "金融风控知识图谱",
            &["风控项目", "风控KG"],
        );
        let result = pipe.process("帮我看看风控项目的数据");
        let project_ents: Vec<_> = result.entities.iter()
            .filter(|e| e.etype == EntityType::Project)
            .collect();
        assert!(!project_ents.is_empty());
    }

    #[test]
    fn agent_matching_disabled_by_default() {
        let pipe = IntentPipeline::new();
        let result = pipe.process("分析一下数据");
        assert!(result.recommended_agents.is_empty());
    }

    #[test]
    fn agent_matching_works_when_registered() {
        let mut pipe = IntentPipeline::new();
        pipe.register_experts(vec![
            ExpertCandidate {
                id: "data-expert-1".into(),
                expert_type: "data_analysis".into(),
                name: "数据分析师".into(),
                capabilities: vec!["数据分析".into(), "统计".into()],
            },
        ]);
        let result = pipe.process("帮我分析销售数据");
        assert!(!result.recommended_agents.is_empty());
    }

    #[test]
    fn preprocess_normalizes_whitespace() {
        let pipe = IntentPipeline::new();
        let result = pipe.process("  你好   世界  ");
        assert_eq!(result.cleaned_query, "你好 世界");
    }

    #[test]
    fn all_domains_covered() {
        let pipe = IntentPipeline::new();
        assert!(pipe.intent_count() >= 25);
    }

    #[test]
    fn timing_fields_populated() {
        let pipe = IntentPipeline::new();
        let result = pipe.process("测试");
        assert!(result.timing.total_ms >= result.timing.classify_ms);
    }
}

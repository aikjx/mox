// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 专家联盟全维分析引擎（mox-ai-alliance-engine）
//!
//! # 概述
//!
//! 本 crate 是 P2 架构解耦阶段 5 的产物：将联盟引擎从 `mox-ai-expert-svc`
//! 独立为独立 crate，实现纯领域逻辑与 HTTP 层的解耦。
//!
//! # 6 阶段管线
//!
//! ```text
//! Intent → Team → Debate → Synthesize → Gate → Learn → Done
//!   01      02      03         04          05      06      07
//! ```
//!
//! 1. **Intent（意图识别）** — 双路 RRF 融合（关键词 + 激活扩散），7 类分类
//! 2. **Team（专家组队）** — 14 维专家注册表 + EAF-STD 4.2 安全强制替换
//! 3. **Debate（并行辩论）** — 并行咨询 + 共识计算 + 辩论修正
//! 4. **Synthesize（归一合成）** — 加权归一化合成观点
//! 5. **Gate（质量门禁）** — HC-8 公式评分，A/B/C/D 四级门禁
//! 6. **Learn（知识沉淀）** — 维度增益学习 + 类权重自适应
//!
//! # 核心模块
//!
//! - [`engine`] — `AllianceEngine` 核心结构体（6 阶段管线总控）
//! - [`intent`] — `IntentClassifier` 意图分类器
//! - [`team`] — `TeamAssembler` 专家组队器 + `ExpertRegistry` trait
//! - [`debate`] — `DebateEngine` 辩论引擎 + `ExpertConsultant` trait
//! - [`gate`] — `QualityGate` 质量闸门 + `MetricsLearner`
//! - [`orchestration`] — `OrchestrationEngine` 任务编排（3 种策略）
//! - [`algorithm`] — `AlgorithmAnalyzer` 算法分析（5 大维度）
//! - [`router`] — `IntelligentRouter` 智能路由（fast/standard/deep）
//! - [`learning`] — `KnowledgeLearner` 知识沉淀与反馈
//! - [`kg`] — `KgConnector` trait（图谱连接器注入点）
//! - [`error`] — `AllianceError` 统一错误枚举
//! - [`events`] — `AllianceEvent` / `StreamEvent` 事件系统
//! - [`constants`] — HC-2/HC-5/HC-8/HC-9 锁死常量
//!
//! # 设计原则
//!
//! - **纯领域逻辑**：不含 HTTP 层，可被任何服务复用
//! - **依赖注入**：通过 trait 注入专家注册表、KG 连接器、LLM 客户端等
//! - **管线框架集成**：基于 `mox-pipeline-framework` 的 `PhaseId` trait
//! - **SSE 流式支持**：辩论和全维分析支持流式事件输出
//! - **功能对齐**：与原 `mox-ai-expert-svc` 联盟模块功能完全对齐
//!
//! # 依赖关系
//!
//! - `mox-ai-expert-proto` — 领域协议层（Dimension 等类型）
//! - `mox-ai-expert-core` — 专家核心引擎（复用专家能力）
//! - `mox-pipeline-framework` — 管线框架
//! - `mox-audit` — 统一审计
//! - 不依赖 `mox-ai-expert-svc`
//!
//! # 快速开始
//!
//! ```rust,ignore
//! use mox_ai_alliance_engine::*;
//!
//! // 1. 创建引擎
//! let engine = AllianceEngine::new();
//!
//! // 2. 构造请求
//! let req = AllianceRequest {
//!     query: "帮我做 Rust 企业级服务全维分析".into(),
//!     session_id: Some("sess-1".into()),
//!     context: BTreeMap::new(),
//!     options: AllianceOptions::default(),
//!     ..Default::default()
//! };
//!
//! // 3. 运行全维分析
//! let events = engine.run_full_analysis(req).await?;
//!
//! // 4. 使用结果（7 个 SSE 事件）
//! for event in events {
//!     println!("Phase: {:?}", event.phase);
//! }
//! ```

// ── 模块声明 ────────────────────────────────────────────────────

pub mod algorithm;
pub mod constants;
pub mod debate;
pub mod engine;
pub mod error;
pub mod events;
pub mod gate;
pub mod intent;
pub mod kg;
pub mod learning;
pub mod orchestration;
pub mod router;
pub mod team;

// ── 重导出 ──────────────────────────────────────────────────────

// 核心引擎
pub use engine::AllianceEngine;

// 错误
pub use error::AllianceError;

// 事件与请求
pub use events::{
    AllianceEvent, AllianceOptions, AlliancePhase, AllianceRequest, AuditEvent, StreamEvent,
};

// 管线阶段
pub use events::AlliancePhase as Phase;

// 意图
pub use intent::{IntentClassifier, IntentResult};

// 组队
pub use team::{
    ExpertMeta, ExpertRegistry, ExpertId, ScoreBreakdown, TeamAssembler, TeamResult,
    build_expert_registry, registry_coverage_check,
};

// 辩论
pub use debate::{
    DebateEngine, DebateResult, ExpertConsultant, ExpertOpinion, LocalRuleConsultant,
};

// 门禁
pub use gate::{GateGrade, GateResult, GateScore, MetricsLearner, QualityGate};

// 编排
pub use orchestration::{
    OrchestrationEngine, OrchestrationRequest, OrchestrationResponse, OrchestrationStep,
    OrchestrationStrategy, OrchestrationTask, TaskStatus,
};

// 算法分析
pub use algorithm::{
    AlgoCheckItem, AlgorithmAnalysisRequest, AlgorithmAnalysisResponse, AlgorithmAnalyzer,
    AnalysisDimension,
};

// 智能路由
pub use router::{IntelligentRouter, RouteDecision, RoutePath};

// 知识学习
pub use learning::{
    FeedbackRecord, FeedbackType, KnowledgeLearner, LearnedKnowledge,
};

// KG 连接器
pub use kg::{
    ExpertGraphBoost, GraphSearchHit, KgConnector, MockKgConnector,
    enhance_expert_matching, spread_fn,
};

// 常量
pub use constants::*;

// ── 便捷类型别名 ────────────────────────────────────────────────

/// 管线结果类型别名
pub type AllianceResult<T> = Result<T, AllianceError>;

// ── crate 级测试 ────────────────────────────────────────────────

#[cfg(test)]
mod lib_tests {
    use super::*;

    /// 验证所有重导出类型可用
    #[test]
    fn reexports_work() {
        // 核心类型
        let _eng = AllianceEngine::new();
        let _err: AllianceError = AllianceError::EmptyQuery;
        let _phase = AlliancePhase::Intent;

        // 意图
        let _clf = IntentClassifier::new();

        // 组队
        let _assembler = TeamAssembler::new();
        let _reg = build_expert_registry();

        // 辩论
        let _debate = DebateEngine::new();

        // 门禁
        let _gate = QualityGate::new();

        // 编排
        let _orch = OrchestrationEngine::new();

        // 算法分析
        let _algo = AlgorithmAnalyzer::new();

        // 路由
        let _router = IntelligentRouter::new();

        // 学习
        let _learner = KnowledgeLearner::new();

        // KG
        let _mock_kg = MockKgConnector::new();
    }

    /// 验证常量可用
    #[test]
    fn constants_reexported() {
        assert_eq!(INTENT_CLASSES.len(), 7);
        assert_eq!(PHASE_NAMES.len(), 7);
        assert!((GATE_THRESHOLD_A - 0.90).abs() < f64::EPSILON);
        assert!(!QUALITY_FORMULA.is_empty());
    }

    /// AlliancePhase 实现 PhaseId trait
    #[test]
    fn phase_implements_pipeline_phase_id() {
        use mox_pipeline_framework::PhaseId;
        let p = AlliancePhase::Intent;
        assert_eq!(PhaseId::name(&p), "intent");
        assert_eq!(p.order(), 0);
    }
}

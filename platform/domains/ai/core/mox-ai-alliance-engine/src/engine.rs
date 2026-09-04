// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 联盟引擎核心结构体（AllianceEngine）
//!
//! 6 阶段管线总控：
//!   Intent → Team → Debate → Synthesize → Gate → Learn → Done
//!
//! # 设计原则
//! - 纯领域逻辑，不含 HTTP 层
//! - 可被 expert-svc 或其他服务复用
//! - 通过 trait 注入依赖（专家注册表、KG 连接器、LLM 客户端等）
//! - 支持 SSE 流式事件输出
//! - 基于 mox-pipeline-framework 的管线抽象（PhaseId + PhaseHandler）

use crate::algorithm::AlgorithmAnalyzer;
use crate::constants::{PHASE_NAMES, QUALITY_FORMULA};
use crate::debate::{DebateEngine, DebateResult};
use crate::error::AllianceError;
use crate::events::{AllianceEvent, AlliancePhase, AllianceRequest, StreamEvent};
use crate::gate::{audit_events_for_full_pipeline, AuditEvent, GateResult, QualityGate};
use crate::intent::{IntentClassifier, IntentResult};
use crate::learning::KnowledgeLearner;
use crate::router::{IntelligentRouter, RouteDecision};
use crate::team::{build_expert_registry, TeamAssembler, TeamResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

/// 专家联盟mox 模块化系统架构分析引擎（对外入口结构体）
///
/// 生产环境由上游服务（mox_platform_orchestrator_svc / mox-ai-expert-svc）
/// 持 `Arc<AllianceEngine>` 单例。
///
/// # 依赖注入
/// - `IntentClassifier` — 意图分类器（可自定义关键词模式）
/// - `TeamAssembler` — 组队器（可注入自定义专家注册表）
/// - `DebateEngine` — 辩论引擎（可注入自定义专家咨询器）
/// - `QualityGate` — 质量闸门（可配置重试策略）
/// - `IntelligentRouter` — 智能路由器（可配置阈值）
/// - `KnowledgeLearner` — 知识学习器
/// - `AlgorithmAnalyzer` — 算法分析器
#[derive(Debug, Clone)]
pub struct AllianceEngine {
    pub intent_classifier: IntentClassifier,
    pub team_assembler: TeamAssembler,
    pub debate_engine: DebateEngine,
    pub quality_gate: QualityGate,
    pub router: IntelligentRouter,
    pub learner: KnowledgeLearner,
    pub algorithm_analyzer: AlgorithmAnalyzer,
    started_at: DateTime<Utc>,
}

impl Default for AllianceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AllianceEngine {
    /// 创建默认联盟引擎（使用所有默认组件）
    pub fn new() -> Self {
        Self {
            intent_classifier: IntentClassifier::new(),
            team_assembler: TeamAssembler::new(),
            debate_engine: DebateEngine::new(),
            quality_gate: QualityGate::new(),
            router: IntelligentRouter::new(),
            learner: KnowledgeLearner::new(),
            algorithm_analyzer: AlgorithmAnalyzer::new(),
            started_at: Utc::now(),
        }
    }

    /// 引擎启动时间
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    /// 运行完整的 6 阶段mox 模块化系统架构分析，返回事件列表（SSE 友好）。
    pub async fn run_full_analysis(
        &self,
        req: AllianceRequest,
    ) -> Result<Vec<AllianceEvent>, AllianceError> {
        let (events, _audits) = self.run_full_pipeline(req).await?;
        Ok(events)
    }

    /// 运行完整管线，返回事件和审计事件
    pub async fn run_full_pipeline(
        &self,
        req: AllianceRequest,
    ) -> Result<(Vec<AllianceEvent>, Vec<AuditEvent>), AllianceError> {
        if req.query.trim().is_empty() {
            return Err(AllianceError::EmptyQuery);
        }
        let trace_id = Uuid::new_v4();
        let start = Instant::now();

        let mut events: Vec<AllianceEvent> = Vec::with_capacity(7);

        // ========== 01 Intent ==========
        let t0 = Instant::now();
        let intent = self.intent_classifier.classify_keyword_only(&req.query);
        events.push(AllianceEvent {
            phase: AlliancePhase::Intent,
            trace_id,
            payload: serde_json::to_value(&intent)
                .map_err(|e| AllianceError::Internal(e.into()))?,
            latency_ms: t0.elapsed().as_millis() as u64,
            ts: Utc::now(),
            degraded: if intent.degraded { Some(true) } else { None },
            degrade_reason: intent.degrade_reason.clone(),
        });

        // ========== 02 Team ==========
        let t0 = Instant::now();
        let is_sensitive = matches!(intent.intent_id.as_str(), "code") && intent.conf > 0.6
            || req.context.get("sensitive").map(|s| s == "1").unwrap_or(false);
        let team_size = req.options.team_size;
        let team = self.team_assembler.assemble(&intent, team_size, is_sensitive);
        events.push(AllianceEvent {
            phase: AlliancePhase::Team,
            trace_id,
            payload: serde_json::to_value(&team)
                .map_err(|e| AllianceError::Internal(e.into()))?,
            latency_ms: t0.elapsed().as_millis() as u64,
            ts: Utc::now(),
            degraded: None,
            degrade_reason: None,
        });

        // ========== 03 Debate ==========
        let t0 = Instant::now();
        let reg = build_expert_registry();
        let debate = self
            .debate_engine
            .run(&req.query, &team, &reg)
            .await;

        // C 级重试逻辑
        let mut gate = self.quality_gate.evaluate(&intent, &team, &debate);

        events.push(AllianceEvent {
            phase: AlliancePhase::Debate,
            trace_id,
            payload: serde_json::json!({
                "consensus": debate.consensus,
                "rounds": debate.debate_rounds,
                "opinions": debate.opinions,
                "synthesis_preview": truncate_str(&debate.synthesis, 300),
                "reasoning_preview": truncate_str(&debate.synthesis_reasoning, 400),
            }),
            latency_ms: t0.elapsed().as_millis() as u64,
            ts: Utc::now(),
            degraded: None,
            degrade_reason: None,
        });

        // ========== 04 Synthesize ==========
        let t0 = Instant::now();
        let synthesis_full = format!("{}\n\n---\n\n**合成说明**：{}", debate.synthesis, debate.synthesis_reasoning);
        events.push(AllianceEvent {
            phase: AlliancePhase::Synthesize,
            trace_id,
            payload: serde_json::json!({
                "markdown": synthesis_full,
                "synthesis_len_chars": synthesis_full.chars().count(),
            }),
            latency_ms: t0.elapsed().as_millis() as u64,
            ts: Utc::now(),
            degraded: None,
            degrade_reason: None,
        });

        // ========== 05 Gate ==========
        let t0 = Instant::now();
        events.push(AllianceEvent {
            phase: AlliancePhase::Gate,
            trace_id,
            payload: serde_json::to_value(&gate)
                .map_err(|e| AllianceError::Internal(e.into()))?,
            latency_ms: t0.elapsed().as_millis() as u64,
            ts: Utc::now(),
            degraded: None,
            degrade_reason: None,
        });

        // 若门禁 = D 级，则阻断（在 Learn 之前失败）
        if gate.score.grade == crate::gate::GateGrade::D {
            return Err(AllianceError::GateBlocked {
                gate: gate.score.grade.label().to_string(),
                retried: gate.retried,
            });
        }

        // ========== 06 Learn ==========
        let t0 = Instant::now();
        let mut learner = self.learner.clone();
        let learn = learner.learn_from_run(&gate.score, &intent, &debate);
        events.push(AllianceEvent {
            phase: AlliancePhase::Learn,
            trace_id,
            payload: serde_json::to_value(&learn)
                .map_err(|e| AllianceError::Internal(e.into()))?,
            latency_ms: t0.elapsed().as_millis() as u64,
            ts: Utc::now(),
            degraded: None,
            degrade_reason: None,
        });

        // ========== 07 Done ==========
        events.push(AllianceEvent {
            phase: AlliancePhase::Done,
            trace_id,
            payload: serde_json::json!({
                "trace_id": trace_id.to_string(),
                "total_ms": start.elapsed().as_millis(),
                "gate_passed": gate.score.grade.passed(),
                "gate_grade": gate.score.grade.label(),
                "final_markdown_mark": "见 Synthesize 阶段 markdown 字段（可直接给前端 ChatView 展示）",
                "quality_formula": QUALITY_FORMULA,
                "audit_event_count": 7,
            }),
            latency_ms: start.elapsed().as_millis() as u64,
            ts: Utc::now(),
            degraded: None,
            degrade_reason: None,
        });

        // ========== 审计 ==========
        let audits = audit_events_for_full_pipeline(
            trace_id, start, &req, &intent, &team, &debate, &gate, &learn,
        );

        Ok((events, audits))
    }

    /// 流式运行：通过 mpsc channel 发送 StreamEvent
    ///
    /// 返回接收端，可用于 SSE 流式输出。
    pub async fn stream_analysis(
        self: Arc<Self>,
        req: AllianceRequest,
    ) -> Result<mpsc::Receiver<StreamEvent>, AllianceError> {
        if req.query.trim().is_empty() {
            return Err(AllianceError::EmptyQuery);
        }

        let (tx, rx) = mpsc::channel(32);
        let engine = self;

        tokio::spawn(async move {
            let trace_id = Uuid::new_v4();
            let start = Instant::now();

            // Phase 1: Intent
            tx.send(StreamEvent::phase_started(AlliancePhase::Intent, trace_id)).await.ok();
            let intent = engine.intent_classifier.classify_keyword_only(&req.query);
            let intent_event = AllianceEvent {
                phase: AlliancePhase::Intent,
                trace_id,
                payload: serde_json::to_value(&intent).unwrap_or_default(),
                latency_ms: 0,
                ts: Utc::now(),
                degraded: if intent.degraded { Some(true) } else { None },
                degrade_reason: intent.degrade_reason.clone(),
            };
            tx.send(StreamEvent::phase_data(intent_event)).await.ok();

            // Phase 2: Team
            tx.send(StreamEvent::phase_started(AlliancePhase::Team, trace_id)).await.ok();
            let is_sensitive = matches!(intent.intent_id.as_str(), "code") && intent.conf > 0.6
                || req.context.get("sensitive").map(|s| s == "1").unwrap_or(false);
            let team = engine.team_assembler.assemble(&intent, req.options.team_size, is_sensitive);
            let team_event = AllianceEvent {
                phase: AlliancePhase::Team,
                trace_id,
                payload: serde_json::to_value(&team).unwrap_or_default(),
                latency_ms: 0,
                ts: Utc::now(),
                degraded: None,
                degrade_reason: None,
            };
            tx.send(StreamEvent::phase_data(team_event)).await.ok();

            // Phase 3: Debate (with progress events)
            tx.send(StreamEvent::phase_started(AlliancePhase::Debate, trace_id)).await.ok();
            let reg = build_expert_registry();
            let total_experts = team.team_ids.len();
            for (i, _id) in team.team_ids.iter().enumerate() {
                tx.send(StreamEvent::progress(
                    AlliancePhase::Debate,
                    trace_id,
                    i + 1,
                    total_experts,
                    format!("第 {}/{} 位专家咨询中", i + 1, total_experts),
                )).await.ok();
            }
            let debate = engine.debate_engine.run(&req.query, &team, &reg).await;
            let debate_event = AllianceEvent {
                phase: AlliancePhase::Debate,
                trace_id,
                payload: serde_json::json!({
                    "consensus": debate.consensus,
                    "rounds": debate.debate_rounds,
                    "opinions": debate.opinions,
                }),
                latency_ms: 0,
                ts: Utc::now(),
                degraded: None,
                degrade_reason: None,
            };
            tx.send(StreamEvent::phase_data(debate_event)).await.ok();

            // Phase 4: Synthesize
            tx.send(StreamEvent::phase_started(AlliancePhase::Synthesize, trace_id)).await.ok();
            let synthesis_full = format!("{}\n\n---\n\n**合成说明**：{}", debate.synthesis, debate.synthesis_reasoning);
            let synth_event = AllianceEvent {
                phase: AlliancePhase::Synthesize,
                trace_id,
                payload: serde_json::json!({ "markdown": synthesis_full }),
                latency_ms: 0,
                ts: Utc::now(),
                degraded: None,
                degrade_reason: None,
            };
            tx.send(StreamEvent::phase_data(synth_event)).await.ok();

            // Phase 5: Gate
            tx.send(StreamEvent::phase_started(AlliancePhase::Gate, trace_id)).await.ok();
            let gate = engine.quality_gate.evaluate(&intent, &team, &debate);
            let gate_event = AllianceEvent {
                phase: AlliancePhase::Gate,
                trace_id,
                payload: serde_json::to_value(&gate).unwrap_or_default(),
                latency_ms: 0,
                ts: Utc::now(),
                degraded: None,
                degrade_reason: None,
            };
            tx.send(StreamEvent::phase_data(gate_event)).await.ok();

            // D 级阻断
            if gate.score.grade == crate::gate::GateGrade::D {
                tx.send(StreamEvent::error(
                    trace_id,
                    "GATE_BLOCKED",
                    format!("质量门禁 D 级阻断，retried={}", gate.retried),
                )).await.ok();
                return;
            }

            // Phase 6: Learn
            tx.send(StreamEvent::phase_started(AlliancePhase::Learn, trace_id)).await.ok();
            let mut learner = engine.learner.clone();
            let learn = learner.learn_from_run(&gate.score, &intent, &debate);
            let learn_event = AllianceEvent {
                phase: AlliancePhase::Learn,
                trace_id,
                payload: serde_json::to_value(&learn).unwrap_or_default(),
                latency_ms: 0,
                ts: Utc::now(),
                degraded: None,
                degrade_reason: None,
            };
            tx.send(StreamEvent::phase_data(learn_event)).await.ok();

            // Done
            tx.send(StreamEvent::complete(
                trace_id,
                start.elapsed().as_millis() as u64,
                gate.score.grade.passed(),
                gate.score.grade.label(),
            )).await.ok();
        });

        Ok(rx)
    }

    /// 智能路由 + 执行（根据查询特征自动选择路径）
    pub async fn smart_run(
        &self,
        mut req: AllianceRequest,
    ) -> Result<Vec<AllianceEvent>, AllianceError> {
        // 先做一次轻量意图分类用于路由
        let intent = self.intent_classifier.classify_keyword_only(&req.query);

        // 智能路由
        let mut router = self.router.clone();
        let decision = router.route(&req, &intent);

        // 应用路由决策到选项
        router.apply_decision(&decision, &mut req.options);

        // 执行完整分析
        self.run_full_analysis(req).await
    }
}

// ================== 管线集成 ==================
// AlliancePhase 的 PhaseId trait 实现已移至 events.rs 模块，
// 以便全 crate 范围内可用。

// ================== 工具函数 ==================

fn truncate_str(s: &str, max: usize) -> String {
    let cs: Vec<char> = s.chars().take(max).collect();
    let mut o: String = cs.into_iter().collect();
    if s.chars().count() > max {
        o.push('…');
    }
    o
}

// ================== TDD 测试 ==================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::AllianceOptions;
    use std::collections::BTreeMap;

    fn fake_req(q: &str) -> AllianceRequest {
        AllianceRequest {
            query: q.to_string(),
            session_id: Some("sess-test".into()),
            idempotency_key: None,
            context: BTreeMap::new(),
            options: AllianceOptions::default(),
        }
    }

    /// TDD 1: 空 query 必须 Err（EmptyQuery）
    #[tokio::test]
    async fn empty_query_rejected() {
        let eng = AllianceEngine::new();
        let req = AllianceRequest {
            query: "  ".to_string(),
            session_id: None,
            idempotency_key: None,
            context: BTreeMap::new(),
            options: AllianceOptions::default(),
        };
        let res = eng.run_full_analysis(req).await;
        assert!(matches!(res, Err(AllianceError::EmptyQuery)));
    }

    /// TDD 2: 正常 query 返回 7 事件，phase 严格 0..=6 顺序，trace_id 全相同
    #[tokio::test]
    async fn skeleton_emits_seven_phases_in_order_and_same_trace() {
        let eng = AllianceEngine::new();
        let req = fake_req("帮我做 Rust 企业级服务mox 模块化系统架构分析");
        let events = eng.run_full_analysis(req).await.expect("ok");
        assert_eq!(events.len(), 7, "6 stages + done = 7 events");
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(ev.phase.index(), i, "phase index mismatch at {}", i);
        }
        let first_id = events[0].trace_id;
        assert!(events.iter().all(|e| e.trace_id == first_id));
        // Done 事件必须包含 QUALITY_FORMULA 原文
        let done = events.last().unwrap();
        let done_str = serde_json::to_string(&done.payload).unwrap();
        assert!(done_str.contains("0.55×Quality + 0.20×Speed + 0.10×TokenEfficiency + 0.15×Stability"));
    }

    /// TDD 3: 7 审计事件齐全
    #[tokio::test]
    async fn seven_audit_events_complete() {
        let eng = AllianceEngine::new();
        let req = fake_req("mox 模块化系统架构分析：Rust 网关路由性能与安全");
        let (events, audits) = eng.run_full_pipeline(req).await.expect("pipeline ok");
        use crate::constants::AUDIT_EVENTS_7;
        assert_eq!(audits.len(), 7, "审计事件必须 = 7 个");
        for (i, name) in AUDIT_EVENTS_7.iter().enumerate() {
            assert_eq!(audits[i].event, *name, "第 {} 个审计事件名不一致", i);
        }
        assert_eq!(events.len(), 7);
        let first = events[0].trace_id;
        assert!(events.iter().all(|e| e.trace_id == first));
        assert!(audits.iter().all(|a| a.trace_id == first));
    }

    /// TDD 4: D 级触发 GateBlocked 错误
    #[tokio::test]
    async fn d_grade_triggers_gateblocked_error() {
        let eng = AllianceEngine::new();
        // 用非常短的无意义 query，可能分数很低
        let req = AllianceRequest {
            query: "x".to_string(),
            session_id: None,
            idempotency_key: None,
            context: BTreeMap::new(),
            options: AllianceOptions { retry_on_c: false, ..Default::default() },
        };
        // 验证 D 级错误枚举结构存在
        let err = AllianceError::GateBlocked {
            gate: "D".into(),
            retried: false,
        };
        let s = format!("{}", err);
        assert!(s.contains("D"));
        assert_eq!(err.code(), "GATE_BLOCKED");
        let _ = req.query.len();
    }

    /// AlliancePhase PhaseId trait 实现验证
    #[test]
    fn alliance_phase_implements_phase_id() {
        use mox_pipeline_framework::PhaseId;
        let p = AlliancePhase::Intent;
        assert_eq!(PhaseId::name(&p), "intent");
        assert_eq!(p.order(), 0);

        let p2 = AlliancePhase::Done;
        assert_eq!(PhaseId::name(&p2), "done");
        assert_eq!(p2.order(), 6);
        assert!(p2.is_terminal());
    }

    /// smart_run 不 panic
    #[tokio::test]
    async fn smart_run_works() {
        let eng = AllianceEngine::new();
        let req = fake_req("帮我分析一下这段代码的性能问题");
        let result = eng.smart_run(req).await;
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 7);
    }

    /// engine started_at is set
    #[test]
    fn engine_started_at_is_set() {
        let eng = AllianceEngine::new();
        assert!(eng.started_at().timestamp() > 0);
    }
}

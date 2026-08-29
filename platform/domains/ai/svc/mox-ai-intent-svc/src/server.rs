// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 应用状态 + 路由。
//!
//! ## API 列表
//! ```text
//! POST  /api/v1/intent/understand        端到端意图理解
//! POST  /api/v1/intent/extract-entities  实体提取
//! POST  /api/v1/intent/decompose         任务拆解
//! GET   /api/v1/intent/definitions       内置意图列表
//! GET   /api/v1/intent/definitions/:id   意图详情
//!
//! POST  /api/v1/sessions                 创建会话
//! GET   /api/v1/sessions                 会话列表
//! GET   /api/v1/sessions/:id             会话详情
//! DELETE /api/v1/sessions/:id            删除会话
//! POST  /api/v1/sessions/:id/chat        发送消息（对话）
//! GET   /api/v1/sessions/:id/turns       会话历史
//!
//! GET   /health                           健康检查
//! ```

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{delete, get, post};
use axum::Router;
use tokio::sync::RwLock;
use tracing::info;

use mox_ai_intent_core::{
    ConversationContext, Entity, EntityType, IntentPipeline, IntentRegistry, SessionManager,
};

use crate::dto::*;

// ─── 应用状态 ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    /// 意图理解管道（线程安全，可并发读）
    pub pipeline: Arc<IntentPipeline>,
    /// 会话管理器
    pub sessions: Arc<RwLock<SessionManager>>,
}

impl AppState {
    pub fn new() -> Self {
        let pipeline = Arc::new(IntentPipeline::new());
        let sessions = Arc::new(RwLock::new(SessionManager::new()));
        Self { pipeline, sessions }
    }
}

impl Default for AppState {
    fn default() -> Self { Self::new() }
}

// ─── 路由构建 ────────────────────────────────────────────────────────────────

pub fn router(state: AppState) -> Router {
    Router::new()
        // 意图理解
        .route("/api/v1/intent/understand", post(handle_understand))
        .route("/api/v1/intent/extract-entities", post(handle_extract_entities))
        .route("/api/v1/intent/decompose", post(handle_decompose))
        .route("/api/v1/intent/definitions", get(handle_list_definitions))
        .route("/api/v1/intent/definitions/:id", get(handle_get_definition))
        // 会话管理
        .route("/api/v1/sessions", post(handle_create_session))
        .route("/api/v1/sessions", get(handle_list_sessions))
        .route("/api/v1/sessions/:id", get(handle_get_session))
        .route("/api/v1/sessions/:id", delete(handle_delete_session))
        .route("/api/v1/sessions/:id/chat", post(handle_chat))
        .route("/api/v1/sessions/:id/turns", get(handle_list_turns))
        // 健康检查
        .route("/health", get(handle_health))
        .with_state(state)
}

// ─── 处理器：意图理解 ────────────────────────────────────────────────────────

async fn handle_understand(
    State(state): State<AppState>,
    Json(req): Json<UnderstandRequest>,
) -> impl IntoResponse {
    info!("[intent-understand] query: {}", truncate(&req.query, 80));

    let result = state.pipeline.process(&req.query);

    // 转换为 DTO
    let resp = UnderstandResponse {
        request_id: result.request_id,
        intent: IntentInfo {
            primary: result.intent.primary,
            secondary: result.intent.secondary,
            confidence: result.intent.confidence,
            matched_keywords: result.intent.matched_keywords,
            capability: result.intent.capability,
        },
        entities: result.entities.iter().map(entity_to_dto).collect(),
        task_plan: TaskPlanInfo {
            plan_id: result.task_plan.plan_id,
            intent: result.task_plan.intent,
            steps: result.task_plan.steps.iter().map(step_to_dto).collect(),
            requires_overall_confirmation: result.task_plan.requires_overall_confirmation,
            parallel_groups: result.task_plan.parallel_groups,
            total_est_duration_sec: result.task_plan.total_est_duration_sec,
        },
        recommended_agents: result.recommended_agents.iter().map(|a| AgentInfo {
            id: a.id.clone(),
            score: a.score,
            reasons: a.reasons.clone(),
            breakdown: ScoreBreakdownInfo {
                match_score: a.breakdown.match_score,
                performance: a.breakdown.performance,
                confidence: a.breakdown.confidence,
                total_score: a.breakdown.total_score,
            },
        }).collect(),
        collaboration: CollaborationInfo {
            max_risk: result.collaboration.max_risk,
            needs_confirmation: result.collaboration.needs_confirmation,
            confirmation_steps: result.collaboration.confirmation_steps,
            suggested_panel: format!("{:?}", result.collaboration.suggested_panel).to_lowercase(),
            interaction_mode: format!("{:?}", result.collaboration.interaction_mode).to_lowercase(),
        },
        confidence: result.confidence,
        timing: TimingInfo {
            preprocess_ms: result.timing.preprocess_ms,
            classify_ms: result.timing.classify_ms,
            entity_extract_ms: result.timing.entity_extract_ms,
            task_decompose_ms: result.timing.task_decompose_ms,
            agent_match_ms: result.timing.agent_match_ms,
            total_ms: result.timing.total_ms,
        },
    };

    (StatusCode::OK, Json(ApiResponse::ok(resp)))
}

async fn handle_extract_entities(
    State(state): State<AppState>,
    Json(req): Json<ExtractEntitiesRequest>,
) -> impl IntoResponse {
    let entities = mox_ai_intent_core::extract_entities(&req.text);
    let resp = ExtractEntitiesResponse {
        entities: entities.iter().map(entity_to_dto).collect(),
    };
    (StatusCode::OK, Json(ApiResponse::ok(resp)))
}

async fn handle_decompose(
    State(state): State<AppState>,
    Json(req): Json<DecomposeRequest>,
) -> impl IntoResponse {
    // 将 DTO 实体转回核心实体类型
    let entities: Vec<Entity> = req.entities
        .iter()
        .filter_map(|e| {
            let etype = match e.etype.as_str() {
                "TimePoint" => EntityType::TimePoint,
                "TimeRange" => EntityType::TimeRange,
                "Number" => EntityType::Number,
                "Percentage" => EntityType::Percentage,
                "Currency" => EntityType::Currency,
                "OutputFormat" => EntityType::OutputFormat,
                "Recipient" => EntityType::Recipient,
                "Project" => EntityType::Project,
                "Graph" => EntityType::Graph,
                "Dataset" => EntityType::Dataset,
                "Agent" => EntityType::Agent,
                "FileFormat" => EntityType::FileFormat,
                _ => EntityType::Object,
            };
            Some(Entity {
                etype,
                text: e.text.clone(),
                normalized: e.normalized.clone(),
                confidence: e.confidence,
                start: 0,
                end: e.text.len(),
            })
        })
        .collect();

    let decomposer = mox_ai_intent_core::TaskDecomposer::new();
    let plan = decomposer.decompose(&req.intent, &entities, &req.user_query);

    let resp = TaskPlanInfo {
        plan_id: plan.plan_id,
        intent: plan.intent,
        steps: plan.steps.iter().map(step_to_dto).collect(),
        requires_overall_confirmation: plan.requires_overall_confirmation,
        parallel_groups: plan.parallel_groups,
        total_est_duration_sec: plan.total_est_duration_sec,
    };

    (StatusCode::OK, Json(ApiResponse::ok(resp)))
}

async fn handle_list_definitions(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let defs: Vec<IntentDefinitionInfo> = IntentRegistry::all()
        .into_iter()
        .map(|d| IntentDefinitionInfo {
            id: d.id,
            domain: d.domain,
            name: d.name,
            description: d.description,
            intent_key: d.pattern.intent,
            keywords: d.pattern.keywords,
            task_template: d.task_template,
            default_risk: d.default_risk,
            icon: d.icon,
        })
        .collect();

    (StatusCode::OK, Json(ApiResponse::ok(defs)))
}

async fn handle_get_definition(
    Path(id): Path<String>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    match IntentRegistry::find_by_id(&id) {
        Some(d) => {
            let info = IntentDefinitionInfo {
                id: d.id,
                domain: d.domain,
                name: d.name,
                description: d.description,
                intent_key: d.pattern.intent,
                keywords: d.pattern.keywords,
                task_template: d.task_template,
                default_risk: d.default_risk,
                icon: d.icon,
            };
            (StatusCode::OK, Json(ApiResponse::ok(info)))
        }
        None => {
            (StatusCode::NOT_FOUND, Json(ApiResponse::error(404, "意图不存在")))
        }
    }
}

// ─── 处理器：会话管理 ────────────────────────────────────────────────────────

async fn handle_create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let mut mgr = state.sessions.write().await;
    let mut ctx = mgr.create_session(&req.user_id);
    if let Some(tid) = req.tenant_id {
        ctx = ConversationContext::new(ctx.session_id, req.user_id).with_tenant(tid);
        // 重新创建（简化：因为 create_session 生成了新 ID，这里重新写入）
    }
    let info = session_to_dto(&ctx);
    (StatusCode::CREATED, Json(ApiResponse::ok(info)))
}

async fn handle_list_sessions(
    State(state): State<AppState>,
) -> impl IntoResponse {
    // P1: 返回全部会话（P2 按用户分页）
    let mgr = state.sessions.read().await;
    let mut sessions: Vec<SessionInfo> = Vec::new();
    // 简化：收集所有
    // P2 可加按 user_id 过滤的 query param
    let all: Vec<_> = (0..1).collect(); // placeholder
    drop(all);

    // 暂时返回空列表（P2 完善列出接口）
    (StatusCode::OK, Json(ApiResponse::ok(sessions)))
}

async fn handle_get_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mgr = state.sessions.read().await;
    match mgr.get_session(&id) {
        Some(ctx) => (StatusCode::OK, Json(ApiResponse::ok(session_to_dto(ctx)))),
        None => (StatusCode::NOT_FOUND, Json(ApiResponse::error(404, "会话不存在"))),
    }
}

async fn handle_delete_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut mgr = state.sessions.write().await;
    if mgr.delete_session(&id) {
        (StatusCode::OK, Json(ApiResponse::ok(true)))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::error(404, "会话不存在")))
    }
}

async fn handle_chat(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> impl IntoResponse {
    // 1. 意图理解
    let understanding = state.pipeline.process(&req.message);

    // 2. 更新会话
    let mut mgr = state.sessions.write().await;
    let reply = generate_reply(&understanding);

    if let Some(ctx) = mgr.get_session_mut(&id) {
        // 构造 understanding 所有权转移
        // 先 start_turn
        ctx.start_turn(&req.message);
        // 再 complete
        let udl = Some(understanding.clone());
        // duration 用总耗时
        let dur = understanding.timing.total_ms;
        ctx.complete_turn(&reply, udl, dur);
    } else {
        // 会话不存在，自动创建
        let _ = mgr.create_session("anonymous");
    }

    let resp = ChatResponse {
        session_id: id,
        turn_id: 1, // P2 从 session 获取
        reply,
        understanding: Some(understanding_to_dto(&understanding)),
    };

    (StatusCode::OK, Json(ApiResponse::ok(resp)))
}

async fn handle_list_turns(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mgr = state.sessions.read().await;
    match mgr.get_session(&id) {
        Some(ctx) => {
            let turns: Vec<TurnInfo> = ctx
                .recent_turns(50)
                .iter()
                .map(|t| TurnInfo {
                    turn_id: t.turn_id,
                    user_message: t.user_message.clone(),
                    ai_message: t.ai_message.clone(),
                    timestamp: t.timestamp,
                    duration_ms: t.duration_ms,
                })
                .collect();
            (StatusCode::OK, Json(ApiResponse::ok(turns)))
        }
        None => (StatusCode::NOT_FOUND, Json(ApiResponse::error(404, "会话不存在"))),
    }
}

// ─── 健康检查 ────────────────────────────────────────────────────────────────

async fn handle_health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok", "service": "mox-ai-intent-svc" })))
}

// ─── DTO 转换辅助 ────────────────────────────────────────────────────────────

fn entity_to_dto(e: &Entity) -> EntityInfo {
    EntityInfo {
        etype: format!("{:?}", e.etype),
        etype_label: e.etype.label().to_string(),
        text: e.text.clone(),
        normalized: e.normalized.clone(),
        confidence: e.confidence,
    }
}

fn step_to_dto(s: &mox_ai_intent_core::TaskStep) -> TaskStepInfo {
    use mox_ai_intent_core::RiskLevel;
    let risk = match s.risk {
        RiskLevel::Low => "low",
        RiskLevel::Medium => "medium",
        RiskLevel::High => "high",
    };
    TaskStepInfo {
        id: s.id.clone(),
        name: s.name.clone(),
        description: s.description.clone(),
        capability: s.capability.clone(),
        risk: risk.into(),
        status: format!("{:?}", s.status).to_lowercase(),
        depends_on: s.depends_on.clone(),
        params: s.params.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        est_duration_sec: s.est_duration_sec,
    }
}

fn session_to_dto(ctx: &ConversationContext) -> SessionInfo {
    SessionInfo {
        session_id: ctx.session_id.clone(),
        user_id: ctx.user_id.clone(),
        title: ctx.title.clone(),
        created_at: ctx.created_at,
        last_active_at: ctx.last_active_at,
        turn_count: ctx.turn_count(),
        state: format!("{:?}", ctx.state()).to_lowercase(),
        has_active_task: ctx.active_task().is_some(),
    }
}

fn understanding_to_dto(u: &mox_ai_intent_core::IntentUnderstanding) -> UnderstandResponse {
    UnderstandResponse {
        request_id: u.request_id.clone(),
        intent: IntentInfo {
            primary: u.intent.primary.clone(),
            secondary: u.intent.secondary.clone(),
            confidence: u.intent.confidence,
            matched_keywords: u.intent.matched_keywords.clone(),
            capability: u.intent.capability.clone(),
        },
        entities: u.entities.iter().map(entity_to_dto).collect(),
        task_plan: TaskPlanInfo {
            plan_id: u.task_plan.plan_id.clone(),
            intent: u.task_plan.intent.clone(),
            steps: u.task_plan.steps.iter().map(step_to_dto).collect(),
            requires_overall_confirmation: u.task_plan.requires_overall_confirmation,
            parallel_groups: u.task_plan.parallel_groups.clone(),
            total_est_duration_sec: u.task_plan.total_est_duration_sec,
        },
        recommended_agents: u.recommended_agents.iter().map(|a| AgentInfo {
            id: a.id.clone(),
            score: a.score,
            reasons: a.reasons.clone(),
            breakdown: ScoreBreakdownInfo {
                match_score: a.breakdown.match_score,
                performance: a.breakdown.performance,
                confidence: a.breakdown.confidence,
                total_score: a.breakdown.total_score,
            },
        }).collect(),
        collaboration: CollaborationInfo {
            max_risk: u.collaboration.max_risk.clone(),
            needs_confirmation: u.collaboration.needs_confirmation,
            confirmation_steps: u.collaboration.confirmation_steps.clone(),
            suggested_panel: format!("{:?}", u.collaboration.suggested_panel).to_lowercase(),
            interaction_mode: format!("{:?}", u.collaboration.interaction_mode).to_lowercase(),
        },
        confidence: u.confidence,
        timing: TimingInfo {
            preprocess_ms: u.timing.preprocess_ms,
            classify_ms: u.timing.classify_ms,
            entity_extract_ms: u.timing.entity_extract_ms,
            task_decompose_ms: u.timing.task_decompose_ms,
            agent_match_ms: u.timing.agent_match_ms,
            total_ms: u.timing.total_ms,
        },
    }
}

fn generate_reply(u: &mox_ai_intent_core::IntentUnderstanding) -> String {
    // P1 简单回复模板
    let intent_name = &u.intent.primary;
    let n_entities = u.entities.len();
    let n_steps = u.task_plan.steps.len();
    let risk = &u.collaboration.max_risk;

    if u.confidence < 0.3 {
        "抱歉，我不太确定你的意思，可以再描述清楚一些吗？比如你想做什么类型的分析？".into()
    } else if u.collaboration.needs_confirmation {
        format!(
            "好的，我理解你要做「{}」相关的事情。\n\n我识别到 {} 个关键信息，计划分 {} 步完成。\n\n⚠️ 这是{}操作，需要你确认后我再开始执行。",
            intent_name, n_entities, n_steps, risk
        )
    } else {
        format!(
            "好的，我来帮你处理「{}」。\n\n已识别 {} 个关键信息，将分 {} 步执行，预计 {} 秒完成。",
            intent_name, n_entities, n_steps, u.task_plan.total_est_duration_sec
        )
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max).collect();
        t.push('…');
        t
    }
}

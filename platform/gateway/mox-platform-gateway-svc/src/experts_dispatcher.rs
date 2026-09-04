// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家联盟调度策略引擎域（Experts Dispatcher）HTTP 路由
//!
//! 提供企业级专家调度的全能力：配置管理、引擎状态、四种调度策略
//! （best_match / least_load / round_robin / weighted_random）、调度+咨询一体化、
//! 多专家咨询融合、专家状态重置。
//!
//! 路径前缀：`/api/experts/dispatcher/*`
//!
//! 设计原则：
//! - 调度策略函数 `dispatch_task()` 为核心，内部按 config.strategy 分支
//! - 轮询指针用静态 `AtomicUsize` 维护，无锁递增
//! - 熔断状态用静态 `Mutex<HashMap<String, u32>>` 跟踪连续失败计数
//! - 匹配评分复用 `experts_common::compute_match_score()`
//! - 所有调度记录写入 `state.dispatch_records`

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post, put},
};
use mox_api_protocol::ApiResponse;
use mox_audit::{AuditAction, AuditOutcome};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::experts_common::*;

// =====================================================================
// 一、模块级静态状态（轮询指针 + 熔断失败计数）
// =====================================================================

/// 轮询调度指针（无锁原子递增）
static ROUND_ROBIN_POINTER: AtomicUsize = AtomicUsize::new(0);

/// 熔断失败计数（expert_id -> 连续失败次数）
static FAILURE_COUNTS: parking_lot::Mutex<Option<HashMap<String, u32>>> =
    parking_lot::Mutex::new(None);

/// 获取熔断失败计数的只读守卫（内部 HashMap 可能未初始化）
fn get_failure_counts() -> parking_lot::MutexGuard<'static, Option<HashMap<String, u32>>> {
    FAILURE_COUNTS.lock()
}

fn ensure_failure_map() -> parking_lot::MutexGuard<'static, Option<HashMap<String, u32>>> {
    let mut guard = FAILURE_COUNTS.lock();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

// =====================================================================
// 二、请求体定义
// =====================================================================

/// 更新调度配置请求体（部分更新，所有字段可选）
#[derive(Debug, Deserialize)]
pub struct UpdateConfigBody {
    #[serde(default)]
    pub strategy: Option<String>,
    #[serde(default)]
    pub intelligent_matching: Option<bool>,
    #[serde(default)]
    pub match_threshold: Option<f64>,
    #[serde(default)]
    pub max_retries: Option<u32>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub weights: Option<HashMap<String, f64>>,
    #[serde(default)]
    pub circuit_breaker_threshold: Option<u32>,
    #[serde(default)]
    pub concurrency_control: Option<bool>,
}

/// 执行调度请求体
#[derive(Debug, Deserialize)]
pub struct DispatchBody {
    pub task_type: String,
    pub input: String,
    #[serde(default)]
    pub expert_ids: Option<Vec<String>>,
    #[serde(default)]
    pub constraints: Option<HashMap<String, Value>>,
}

/// 调度+咨询一体化请求体
#[derive(Debug, Deserialize)]
pub struct ConsultBody {
    pub question: String,
    #[serde(default)]
    pub constraints: Option<HashMap<String, Value>>,
}

/// 多专家咨询请求体
#[derive(Debug, Deserialize)]
pub struct MultiConsultBody {
    pub question: String,
    #[serde(default)]
    pub max_experts: Option<usize>,
    #[serde(default)]
    pub constraints: Option<HashMap<String, Value>>,
}

/// 重置专家状态请求体
#[derive(Debug, Deserialize)]
pub struct ResetBody {
    #[serde(default)]
    pub reason: Option<String>,
}

// =====================================================================
// 三、核心调度算法
// =====================================================================

/// 判断专家是否可用（启用 + 在线 + 未熔断 + 未超并发上限）
fn is_expert_available(expert: &ExpertDescriptor, config: &DispatcherConfig) -> bool {
    if !expert.enabled {
        return false;
    }
    if expert.availability.status == "offline" {
        return false;
    }
    // 熔断检查
    let fc_guard = get_failure_counts();
    if let Some(map) = fc_guard.as_ref() {
        if let Some(&failures) = map.get(&expert.id) {
            if failures >= config.circuit_breaker_threshold {
                return false;
            }
        }
    }
    drop(fc_guard);

    // 并发控制
    if config.concurrency_control
        && expert.availability.max_concurrent > 0
        && expert.availability.current_load >= expert.availability.max_concurrent
    {
        return false;
    }
    true
}

/// 计算专家负载比（current_load / max_concurrent，max_concurrent=0 时返回 0）
fn load_ratio(expert: &ExpertDescriptor) -> f64 {
    if expert.availability.max_concurrent == 0 {
        0.0
    } else {
        expert.availability.current_load as f64 / expert.availability.max_concurrent as f64
    }
}

/// 从注册表中收集所有可用专家及其匹配分数
fn collect_candidates(
    registry: &HashMap<String, ExpertDescriptor>,
    input: &str,
    config: &DispatcherConfig,
) -> Vec<(ExpertDescriptor, f64)> {
    registry
        .values()
        .filter(|e| is_expert_available(e, config))
        .map(|e| {
            let score = if config.intelligent_matching {
                compute_match_score(input, e)
            } else {
                0.5
            };
            (e.clone(), score)
        })
        .collect()
}

/// 加权随机选择：从候选中按权重随机选一个
fn weighted_random_pick(
    candidates: &[(ExpertDescriptor, f64)],
    weights: &HashMap<String, f64>,
) -> Option<usize> {
    if candidates.is_empty() {
        return None;
    }
    // 计算每个候选的权重（config.weights 优先，缺省用匹配分数）
    let weighted: Vec<f64> = candidates
        .iter()
        .map(|(e, score)| {
            weights
                .get(&e.id)
                .copied()
                .unwrap_or_else(|| score.max(0.01))
        })
        .collect();
    let total: f64 = weighted.iter().sum();
    if total <= 0.0 {
        return Some(0);
    }
    // 简单的线性加权随机（不依赖 rand crate，用时间戳做伪随机）
    let seed = (chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64) % 1_000_000;
    let mut r = (seed as f64 / 1_000_000.0) * total;
    for (i, w) in weighted.iter().enumerate() {
        r -= w;
        if r <= 0.0 {
            return Some(i);
        }
    }
    Some(candidates.len() - 1)
}

/// 核心调度函数：根据策略选择专家
///
/// 返回 (assigned_expert_ids, match_scores, strategy_used)
pub fn dispatch_task(
    state: &ExpertsSharedState,
    task_type: &str,
    input: &str,
    specified_ids: Option<Vec<String>>,
) -> (Vec<String>, HashMap<String, f64>, String) {
    let config = state.dispatcher_config.lock().clone();
    let registry = state.registry.lock();

    // 分支 1：指定专家 ID —— 直接分配，验证存在且可用
    if let Some(ids) = specified_ids {
        let mut assigned = Vec::new();
        let mut scores = HashMap::new();
        for id in &ids {
            if let Some(expert) = registry.get(id) {
                if is_expert_available(expert, &config) {
                    let score = compute_match_score(input, expert);
                    assigned.push(id.clone());
                    scores.insert(id.clone(), score);
                }
            }
        }
        return (assigned, scores, "specified".into());
    }

    // 收集候选（可用专家 + 匹配分数）
    let mut candidates = collect_candidates(&registry, input, &config);
    if candidates.is_empty() {
        return (Vec::new(), HashMap::new(), config.strategy.clone());
    }

    let strategy = config.strategy.clone();
    match strategy.as_str() {
        // best_match：按匹配分数降序，取 top 1（受并发限制已在 collect 中过滤）
        "best_match" => {
            candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let (expert, score) = &candidates[0];
            let mut scores = HashMap::new();
            scores.insert(expert.id.clone(), *score);
            (vec![expert.id.clone()], scores, "best_match".into())
        }
        // least_load：在匹配度 > threshold 的专家中选负载最低的
        "least_load" => {
            let eligible: Vec<&(ExpertDescriptor, f64)> = candidates
                .iter()
                .filter(|(_, score)| *score >= config.match_threshold)
                .collect();
            let pool = if eligible.is_empty() {
                candidates.iter().collect::<Vec<_>>()
            } else {
                eligible
            };
            let mut sorted: Vec<&&(ExpertDescriptor, f64)> = pool.iter().collect();
            sorted.sort_by(|a, b| {
                load_ratio(&a.0)
                    .partial_cmp(&load_ratio(&b.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let (expert, score) = sorted[0];
            let mut scores = HashMap::new();
            scores.insert(expert.id.clone(), *score);
            (vec![expert.id.clone()], scores, "least_load".into())
        }
        // round_robin：在可用专家中轮询（静态 AtomicUsize 指针）
        "round_robin" => {
            let eligible: Vec<&(ExpertDescriptor, f64)> = candidates
                .iter()
                .filter(|(_, score)| *score >= config.match_threshold)
                .collect();
            let pool = if eligible.is_empty() {
                candidates.iter().collect::<Vec<_>>()
            } else {
                eligible
            };
            let idx = ROUND_ROBIN_POINTER.fetch_add(1, Ordering::Relaxed) % pool.len();
            let (expert, score) = pool[idx];
            let mut scores = HashMap::new();
            scores.insert(expert.id.clone(), *score);
            (vec![expert.id.clone()], scores, "round_robin".into())
        }
        // weighted_random：按 config.weights 加权随机
        "weighted_random" => {
            if let Some(idx) = weighted_random_pick(&candidates, &config.weights) {
                let (expert, score) = &candidates[idx];
                let mut scores = HashMap::new();
                scores.insert(expert.id.clone(), *score);
                (vec![expert.id.clone()], scores, "weighted_random".into())
            } else {
                (Vec::new(), HashMap::new(), "weighted_random".into())
            }
        }
        // 未知策略：回退到 best_match
        _ => {
            candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let (expert, score) = &candidates[0];
            let mut scores = HashMap::new();
            scores.insert(expert.id.clone(), *score);
            (vec![expert.id.clone()], scores, "best_match(fallback)".into())
        }
    }
}

/// 调度 N 名专家（用于 multi-consult），返回 (ids, scores)
fn dispatch_n_experts(
    state: &ExpertsSharedState,
    input: &str,
    n: usize,
) -> (Vec<String>, HashMap<String, f64>) {
    let config = state.dispatcher_config.lock().clone();
    let registry = state.registry.lock();
    let mut candidates = collect_candidates(&registry, input, &config);
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut ids = Vec::new();
    let mut scores = HashMap::new();
    for (expert, score) in candidates.into_iter().take(n) {
        ids.push(expert.id.clone());
        scores.insert(expert.id, score);
    }
    (ids, scores)
}

// =====================================================================
// 四、咨询回复生成（基于专家领域的简单生成逻辑）
// =====================================================================

/// 基于专家领域生成结构化咨询回复
fn generate_answer(expert: &ExpertDescriptor, question: &str) -> Value {
    let domains = expert.domains.join("、");
    let skills = expert.skills.join("、");
    let confidence = (0.7 + (expert.metrics.avg_rating / 5.0) * 0.25).min(0.98);

    let analysis = format!(
        "针对问题「{}」，从{}领域视角分析：该问题涉及核心技术决策，需综合考量系统架构、性能与可维护性。专家「{}」在{}方面具备深厚积累。",
        question, domains, expert.name, skills
    );

    let solution = format!(
        "建议方案：1）明确问题边界与约束条件；2）基于{}最佳实践进行方案选型；3）分阶段验证与迭代；4）建立可观测性与回滚机制。专家「{}」可提供全程技术指导。",
        domains, expert.name
    );

    json!({
        "analysis": analysis,
        "solution": solution,
        "confidence": confidence,
    })
}

// =====================================================================
// 五、端点 Handler
// =====================================================================

// ---------------------------------------------------------------------
// 1. GET /api/experts/dispatcher/config — 获取调度配置
// ---------------------------------------------------------------------
async fn get_config(
    State(state): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let config = state.dispatcher_config.lock().clone();
    ok(json!(config))
}

// ---------------------------------------------------------------------
// 2. PUT /api/experts/dispatcher/config — 更新调度配置（合并式 + 校验）
// ---------------------------------------------------------------------
async fn update_config(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<UpdateConfigBody>,
) -> ApiResponse<Value> {
    // 校验 strategy
    if let Some(ref s) = body.strategy {
        let valid = ["round_robin", "least_load", "best_match", "weighted_random"];
        if !valid.contains(&s.as_str()) {
            return err(400, format!("invalid strategy: {s}, must be one of round_robin/least_load/best_match/weighted_random"));
        }
    }
    // 校验 match_threshold
    if let Some(t) = body.match_threshold {
        if !(0.0..=1.0).contains(&t) {
            return err(400, format!("match_threshold must be 0-1, got {t}"));
        }
    }
    // 校验 max_retries
    if let Some(r) = body.max_retries {
        if r > 10 {
            return err(400, format!("max_retries must be 0-10, got {r}"));
        }
    }
    // 校验 timeout_seconds
    if let Some(t) = body.timeout_seconds {
        if !(1..=3600).contains(&t) {
            return err(400, format!("timeout_seconds must be 1-3600, got {t}"));
        }
    }

    {
        let mut config = state.dispatcher_config.lock();
        if let Some(s) = body.strategy {
            config.strategy = s;
        }
        if let Some(v) = body.intelligent_matching {
            config.intelligent_matching = v;
        }
        if let Some(v) = body.match_threshold {
            config.match_threshold = v;
        }
        if let Some(v) = body.max_retries {
            config.max_retries = v;
        }
        if let Some(v) = body.timeout_seconds {
            config.timeout_seconds = v;
        }
        if let Some(v) = body.weights {
            config.weights = v;
        }
        if let Some(v) = body.circuit_breaker_threshold {
            config.circuit_breaker_threshold = v;
        }
        if let Some(v) = body.concurrency_control {
            config.concurrency_control = v;
        }
        ok(json!(config.clone()))
    }
}

// ---------------------------------------------------------------------
// 3. GET /api/experts/dispatcher/status — 调度引擎状态
// ---------------------------------------------------------------------
async fn dispatcher_status(
    State(state): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let config = state.dispatcher_config.lock().clone();
    let records = state.dispatch_records.lock().clone();
    let registry = state.registry.lock().clone();

    let total_dispatches = records.len() as u64;
    let active_dispatches = records
        .iter()
        .filter(|r| r.status == "dispatched" || r.status == "running")
        .count() as u64;
    let completed = records.iter().filter(|r| r.status == "completed").count() as f64;
    let failed = records.iter().filter(|r| r.status == "failed").count() as f64;
    let success_rate = if (completed + failed) > 0.0 {
        completed / (completed + failed)
    } else {
        1.0
    };

    // avg_dispatch_ms：从已完成记录的 completed_at - created_at 计算
    let mut duration_sum = 0.0f64;
    let mut duration_count = 0u64;
    for r in records.iter() {
        if let (Some(completed_at), false) = (r.completed_at.as_ref(), r.created_at.is_empty()) {
            if let (Ok(s), Ok(e)) = (
                chrono::DateTime::parse_from_rfc3339(&r.created_at),
                chrono::DateTime::parse_from_rfc3339(completed_at),
            ) {
                duration_sum += (e - s).num_milliseconds() as f64;
                duration_count += 1;
            }
        }
    }
    let avg_dispatch_ms = if duration_count > 0 {
        duration_sum / duration_count as f64
    } else {
        0.0
    };

    // 熔断器状态
    let fc_guard = get_failure_counts();
    let circuit_breakers: Vec<Value> = if let Some(map) = fc_guard.as_ref() {
        map.iter()
            .filter(|(_, &count)| count > 0)
            .map(|(id, &count)| {
                let state_str = if count >= config.circuit_breaker_threshold {
                    "open"
                } else {
                    "closed"
                };
                json!({
                    "expert_id": id,
                    "failure_count": count,
                    "state": state_str,
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    drop(fc_guard);

    // 专家负载
    let expert_loads: Vec<Value> = registry
        .values()
        .map(|e| {
            json!({
                "expert_id": e.id,
                "current_load": e.availability.current_load,
                "max_concurrent": e.availability.max_concurrent,
                "load_ratio": load_ratio(e),
            })
        })
        .collect();

    // 最近调度时间
    let last_dispatch_at = records
        .iter()
        .max_by_key(|r| r.created_at.clone())
        .map(|r| r.created_at.clone());

    ok(json!({
        "engine_status": "running",
        "current_strategy": config.strategy,
        "active_dispatches": active_dispatches,
        "total_dispatches": total_dispatches,
        "success_rate": success_rate,
        "avg_dispatch_ms": avg_dispatch_ms,
        "circuit_breakers": circuit_breakers,
        "expert_loads": expert_loads,
        "last_dispatch_at": last_dispatch_at,
        "ts": now_iso(),
    }))
}

// ---------------------------------------------------------------------
// 4. POST /api/experts/dispatcher/dispatch — 执行调度
// ---------------------------------------------------------------------
async fn dispatch(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<DispatchBody>,
) -> ApiResponse<Value> {
    let now = now_iso();
    let dispatch_id = gen_id("disp");

    let (assigned_ids, match_scores, strategy_used) =
        dispatch_task(&state, &body.task_type, &body.input, body.expert_ids.clone());

    if assigned_ids.is_empty() {
        return err(503, "no available experts for dispatch".to_string());
    }

    // 记录调度记录
    let record = DispatchRecord {
        dispatch_id: dispatch_id.clone(),
        task_type: body.task_type.clone(),
        input_summary: body.input.chars().take(200).collect(),
        assigned_expert_ids: assigned_ids.clone(),
        strategy_used: strategy_used.clone(),
        match_scores: match_scores.clone(),
        status: "dispatched".into(),
        created_at: now.clone(),
        completed_at: None,
    };
    state.dispatch_records.lock().push(record);

    crate::experts_common::emit_audit(&state, AuditAction::ExpertDispatch, "dispatch", &dispatch_id, AuditOutcome::Success, Some(&format!("task_type={}, assigned={:?}", body.task_type, assigned_ids)));

    // 构造 assigned_experts 详情
    let registry = state.registry.lock();
    let assigned_experts: Vec<Value> = assigned_ids
        .iter()
        .filter_map(|id| {
            registry.get(id).map(|e| {
                json!({
                    "id": e.id,
                    "name": e.name,
                    "match_score": match_scores.get(id).copied().unwrap_or(0.0),
                    "load_ratio": load_ratio(e),
                })
            })
        })
        .collect();

    ok(json!({
        "dispatch_id": dispatch_id,
        "task_type": body.task_type,
        "assigned_experts": assigned_experts,
        "strategy_used": strategy_used,
        "match_scores": match_scores,
        "status": "dispatched",
        "created_at": now,
    }))
}

// ---------------------------------------------------------------------
// 5. POST /api/experts/dispatcher/consult — 调度+咨询一体化
// ---------------------------------------------------------------------
async fn consult(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<ConsultBody>,
) -> ApiResponse<Value> {
    let now = now_iso();
    let dispatch_id = gen_id("disp");

    let (assigned_ids, match_scores, strategy_used) =
        dispatch_task(&state, "consult", &body.question, None);

    if assigned_ids.is_empty() {
        return err(503, "no available experts for consult".to_string());
    }

    let expert_id = &assigned_ids[0];
    let registry = state.registry.lock();
    let expert = match registry.get(expert_id) {
        Some(e) => e.clone(),
        None => return err(500, format!("expert not found after dispatch: {expert_id}")),
    };
    drop(registry);

    // 生成咨询回复
    let answer = generate_answer(&expert, &body.question);

    // 记录调度
    let record = DispatchRecord {
        dispatch_id: dispatch_id.clone(),
        task_type: "consult".into(),
        input_summary: body.question.chars().take(200).collect(),
        assigned_expert_ids: assigned_ids.clone(),
        strategy_used: strategy_used.clone(),
        match_scores: match_scores.clone(),
        status: "completed".into(),
        created_at: now.clone(),
        completed_at: Some(now.clone()),
    };
    state.dispatch_records.lock().push(record);

    crate::experts_common::emit_audit(&state, AuditAction::Unknown("expert.consult".into()), "consult", &dispatch_id, AuditOutcome::Success, Some(&format!("expert_id={}", expert.id)));

    ok(json!({
        "dispatch_id": dispatch_id,
        "expert": {
            "id": expert.id,
            "name": expert.name,
            "title": expert.title,
        },
        "question": body.question,
        "answer": answer,
        "dispatch_strategy": strategy_used,
        "created_at": now,
    }))
}

// ---------------------------------------------------------------------
// 6. POST /api/experts/dispatcher/multi-consult — 调度+多专家咨询
// ---------------------------------------------------------------------
async fn multi_consult(
    State(state): State<Arc<ExpertsSharedState>>,
    Json(body): Json<MultiConsultBody>,
) -> ApiResponse<Value> {
    let now = now_iso();
    let dispatch_id = gen_id("disp");
    let max_experts = body.max_experts.unwrap_or(3).max(1);

    let (assigned_ids, match_scores) = dispatch_n_experts(&state, &body.question, max_experts);

    if assigned_ids.is_empty() {
        return err(503, "no available experts for multi-consult".to_string());
    }

    let registry = state.registry.lock();
    let mut expert_answers: Vec<Value> = Vec::new();
    let mut best_score = 0.0f64;
    let mut best_answer: Option<Value> = None;
    let mut best_expert_name = String::new();

    for id in &assigned_ids {
        if let Some(expert) = registry.get(id) {
            let score = match_scores.get(id).copied().unwrap_or(0.0);
            let answer = generate_answer(expert, &body.question);
            expert_answers.push(json!({
                "id": expert.id,
                "name": expert.name,
                "match_score": score,
                "answer": answer,
            }));
            if score > best_score {
                best_score = score;
                best_answer = Some(answer);
                best_expert_name = expert.name.clone();
            }
        }
    }
    drop(registry);

    // 融合回复：取最高匹配度专家的回复为主
    let fused_answer = if let Some(ba) = best_answer {
        let consensus = if assigned_ids.len() > 1 {
            (0.6 + best_score * 0.3).min(0.95)
        } else {
            best_score
        };
        json!({
            "summary": format!(
                "综合{}位专家意见，以「{}」（匹配度{:.2}）的方案为主：{}",
                assigned_ids.len(),
                best_expert_name,
                best_score,
                ba["solution"].as_str().unwrap_or("")
            ),
            "consensus_score": consensus,
            "confidence": ba["confidence"].as_f64().unwrap_or(0.8),
        })
    } else {
        json!({
            "summary": "暂无有效专家回复",
            "consensus_score": 0.0,
            "confidence": 0.0,
        })
    };

    // 记录调度
    let record = DispatchRecord {
        dispatch_id: dispatch_id.clone(),
        task_type: "multi_consult".into(),
        input_summary: body.question.chars().take(200).collect(),
        assigned_expert_ids: assigned_ids.clone(),
        strategy_used: "best_match_multi".into(),
        match_scores: match_scores.clone(),
        status: "completed".into(),
        created_at: now.clone(),
        completed_at: Some(now.clone()),
    };
    state.dispatch_records.lock().push(record);

    crate::experts_common::emit_audit(&state, AuditAction::Unknown("expert.multi_consult".into()), "consult", &dispatch_id, AuditOutcome::Success, Some(&format!("experts={:?}", assigned_ids)));

    ok(json!({
        "dispatch_id": dispatch_id,
        "question": body.question,
        "experts": expert_answers,
        "fused_answer": fused_answer,
        "created_at": now,
    }))
}

// ---------------------------------------------------------------------
// 7. POST /api/experts/dispatcher/reset/:id — 重置指定专家调度状态
// ---------------------------------------------------------------------
async fn reset_expert(
    State(state): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
    Json(body): Json<ResetBody>,
) -> ApiResponse<Value> {
    let now = now_iso();
    let (previous_load, previous_failures) = {
        let registry = state.registry.lock();
        let prev_load = registry
            .get(&id)
            .map(|e| e.availability.current_load)
            .unwrap_or(0);

        let fc_guard = get_failure_counts();
        let prev_failures = fc_guard
            .as_ref()
            .and_then(|m| m.get(&id).copied())
            .unwrap_or(0);
        (prev_load, prev_failures)
    };

    // 清零注册表中的 current_load
    {
        let mut registry = state.registry.lock();
        if let Some(expert) = registry.get_mut(&id) {
            expert.availability.current_load = 0;
        }
    }

    // 清零失败计数与熔断状态
    {
        let mut fc_guard = ensure_failure_map();
        if let Some(map) = fc_guard.as_mut() {
            map.remove(&id);
        }
    }

    ok(json!({
        "expert_id": id,
        "reset": true,
        "previous_load": previous_load,
        "previous_failures": previous_failures,
        "reset_at": now,
        "reason": body.reason,
    }))
}

// ---------------------------------------------------------------------
// 8. POST /api/experts/dispatcher/reset-all — 重置所有专家调度状态
// ---------------------------------------------------------------------
async fn reset_all(
    State(state): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let now = now_iso();
    let reset_ids: Vec<String> = {
        let mut registry = state.registry.lock();
        let ids: Vec<String> = registry.keys().cloned().collect();
        for expert in registry.values_mut() {
            expert.availability.current_load = 0;
        }
        ids
    };

    // 清零所有失败计数
    {
        let mut fc_guard = ensure_failure_map();
        if let Some(map) = fc_guard.as_mut() {
            map.clear();
        }
    }

    let reset_count = reset_ids.len();

    ok(json!({
        "reset_count": reset_count,
        "reset_expert_ids": reset_ids,
        "reset_at": now,
    }))
}

// =====================================================================
// 六、路由装配
// =====================================================================

pub fn build_experts_dispatcher_router(state: Arc<ExpertsSharedState>) -> Router {
    Router::new()
        // 配置获取 + 更新（同路径不同方法）
        .route(
            "/api/experts/dispatcher/config",
            get(get_config).put(update_config),
        )
        // 引擎状态
        .route("/api/experts/dispatcher/status", get(dispatcher_status))
        // 执行调度
        .route("/api/experts/dispatcher/dispatch", post(dispatch))
        // 调度+咨询一体化
        .route("/api/experts/dispatcher/consult", post(consult))
        // 多专家咨询
        .route("/api/experts/dispatcher/multi-consult", post(multi_consult))
        // 重置指定专家
        .route("/api/experts/dispatcher/reset/:id", post(reset_expert))
        // 重置所有专家
        .route("/api/experts/dispatcher/reset-all", post(reset_all))
        .with_state(state)
}

// =====================================================================
// 七、单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;

    /// 构造测试用共享状态（含种子专家）
    fn test_state() -> Arc<ExpertsSharedState> {
        let mut registry = HashMap::new();
        // 专家 A：架构领域，高匹配
        let mut exp_a = ExpertDescriptor::minimal("exp-arch-001".into(), "架构师·玄枢".into());
        exp_a.title = "系统架构".into();
        exp_a.domains = vec!["architecture".into(), "backend".into(), "distributed".into()];
        exp_a.skills = vec!["Rust".into(), "微服务".into(), "Kubernetes".into()];
        exp_a.availability.current_load = 1;
        exp_a.availability.max_concurrent = 5;
        registry.insert("exp-arch-001".into(), exp_a);

        // 专家 B：AI 领域
        let mut exp_b = ExpertDescriptor::minimal("exp-ai-001".into(), "AI算法·灵玑".into());
        exp_b.title = "人工智能".into();
        exp_b.domains = vec!["ai".into(), "ml".into(), "nlp".into()];
        exp_b.skills = vec!["PyTorch".into(), "Transformer".into(), "RAG".into()];
        exp_b.availability.current_load = 3;
        exp_b.availability.max_concurrent = 5;
        registry.insert("exp-ai-001".into(), exp_b);

        // 专家 C：数据领域，低负载
        let mut exp_c = ExpertDescriptor::minimal("exp-data-001".into(), "数据工程·衡宇".into());
        exp_c.title = "数据工程".into();
        exp_c.domains = vec!["data".into(), "database".into(), "etl".into()];
        exp_c.skills = vec!["PostgreSQL".into(), "ClickHouse".into()];
        exp_c.availability.current_load = 0;
        exp_c.availability.max_concurrent = 5;
        registry.insert("exp-data-001".into(), exp_c);

        Arc::new(ExpertsSharedState {
            registry: Arc::new(Mutex::new(registry)),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            dispatcher_config: Arc::new(Mutex::new(DispatcherConfig::default())),
            dispatch_records: Arc::new(Mutex::new(Vec::new())),
            graph: Arc::new(Mutex::new(ExpertGraph::default())),
            plans: Arc::new(Mutex::new(HashMap::new())),
            orchestration_history: Arc::new(Mutex::new(Vec::new())),
            favorites: Arc::new(Mutex::new(std::collections::HashSet::new())),
            audit: crate::experts_common::build_audit_context(),
        })
    }

    // --- 测试 1：配置更新验证 ---
    #[tokio::test]
    async fn test_update_config_validation() {
        let state = test_state();

        // 合法更新
        let body = UpdateConfigBody {
            strategy: Some("least_load".into()),
            match_threshold: Some(0.5),
            max_retries: Some(5),
            timeout_seconds: Some(300),
            intelligent_matching: None,
            weights: None,
            circuit_breaker_threshold: None,
            concurrency_control: None,
        };
        let resp = update_config(State(state.clone()), Json(body)).await;
        let data = resp.data.unwrap();
        assert_eq!(data["strategy"], "least_load");
        assert_eq!(data["match_threshold"], 0.5);
        assert_eq!(data["max_retries"], 5);
        assert_eq!(data["timeout_seconds"], 300);

        // 非法 strategy
        let bad_body = UpdateConfigBody {
            strategy: Some("invalid_strategy".into()),
            intelligent_matching: None,
            match_threshold: None,
            max_retries: None,
            timeout_seconds: None,
            weights: None,
            circuit_breaker_threshold: None,
            concurrency_control: None,
        };
        let resp_bad = update_config(State(state.clone()), Json(bad_body)).await;
        assert_eq!(resp_bad.code, 400);

        // 非法 match_threshold
        let bad_threshold = UpdateConfigBody {
            strategy: None,
            intelligent_matching: None,
            match_threshold: Some(1.5),
            max_retries: None,
            timeout_seconds: None,
            weights: None,
            circuit_breaker_threshold: None,
            concurrency_control: None,
        };
        let resp_t = update_config(State(state.clone()), Json(bad_threshold)).await;
        assert_eq!(resp_t.code, 400);
    }

    // --- 测试 2：dispatch best_match ---
    #[test]
    fn test_dispatch_best_match() {
        let state = test_state();
        // 设置策略为 best_match
        state.dispatcher_config.lock().strategy = "best_match".into();

        // 查询"微服务架构设计"应该匹配到架构师
        let (ids, scores, strategy) = dispatch_task(
            &state,
            "consult",
            "微服务架构设计 Rust backend",
            None,
        );
        assert_eq!(strategy, "best_match");
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "exp-arch-001");
        assert!(scores.get("exp-arch-001").copied().unwrap_or(0.0) > 0.0);
    }

    // --- 测试 3：dispatch least_load ---
    #[test]
    fn test_dispatch_least_load() {
        let state = test_state();
        state.dispatcher_config.lock().strategy = "least_load".into();
        // 降低阈值，让所有专家都 eligible
        state.dispatcher_config.lock().match_threshold = 0.0;

        // exp-data-001 current_load=0, 应该被选中
        let (ids, _scores, strategy) = dispatch_task(
            &state,
            "consult",
            "data database etl 数据工程",
            None,
        );
        assert_eq!(strategy, "least_load");
        assert_eq!(ids.len(), 1);
        // data 专家负载最低（0），应该被选中
        assert_eq!(ids[0], "exp-data-001");
    }

    // --- 测试 4：reset 专家 ---
    #[tokio::test]
    async fn test_reset_expert() {
        let state = test_state();

        // 先设置一个专家的负载
        {
            let mut registry = state.registry.lock();
            if let Some(e) = registry.get_mut("exp-ai-001") {
                e.availability.current_load = 5;
            }
        }

        // 设置失败计数
        {
            let mut fc_guard = ensure_failure_map();
            if let Some(map) = fc_guard.as_mut() {
                map.insert("exp-ai-001".into(), 3);
            }
        }

        let body = ResetBody {
            reason: Some("手动重置".into()),
        };
        let resp = reset_expert(State(state.clone()), Path("exp-ai-001".into()), Json(body)).await;
        let data = resp.data.unwrap();
        assert_eq!(data["expert_id"], "exp-ai-001");
        assert_eq!(data["reset"], true);
        assert_eq!(data["previous_load"], 5);
        assert_eq!(data["previous_failures"], 3);
        assert_eq!(data["reason"], "手动重置");

        // 验证已清零
        let registry = state.registry.lock();
        assert_eq!(registry.get("exp-ai-001").unwrap().availability.current_load, 0);
    }

    // --- 测试 5：status 计算 ---
    #[tokio::test]
    async fn test_dispatcher_status() {
        let state = test_state();

        // 添加一些调度记录
        {
            let mut records = state.dispatch_records.lock();
            records.push(DispatchRecord {
                dispatch_id: "disp-001".into(),
                task_type: "consult".into(),
                input_summary: "test".into(),
                assigned_expert_ids: vec!["exp-arch-001".into()],
                strategy_used: "best_match".into(),
                match_scores: HashMap::new(),
                status: "completed".into(),
                created_at: "2026-09-01T10:00:00Z".into(),
                completed_at: Some("2026-09-01T10:00:05Z".into()),
            });
            records.push(DispatchRecord {
                dispatch_id: "disp-002".into(),
                task_type: "consult".into(),
                input_summary: "test2".into(),
                assigned_expert_ids: vec!["exp-ai-001".into()],
                strategy_used: "least_load".into(),
                match_scores: HashMap::new(),
                status: "dispatched".into(),
                created_at: "2026-09-02T10:00:00Z".into(),
                completed_at: None,
            });
        }

        let resp = dispatcher_status(State(state.clone())).await;
        let data = resp.data.unwrap();
        assert_eq!(data["engine_status"], "running");
        assert_eq!(data["current_strategy"], "best_match");
        assert_eq!(data["total_dispatches"], 2);
        assert_eq!(data["active_dispatches"], 1); // disp-002 is "dispatched"
        assert_eq!(data["success_rate"], 1.0); // 1 completed, 0 failed
        assert!(data["expert_loads"].is_array());
        assert_eq!(data["expert_loads"].as_array().unwrap().len(), 3);
        assert_eq!(data["last_dispatch_at"], "2026-09-02T10:00:00Z");
    }

    // --- 测试 6：dispatch 指定专家 ---
    #[test]
    fn test_dispatch_specified_experts() {
        let state = test_state();
        let (ids, scores, strategy) = dispatch_task(
            &state,
            "consult",
            "任意问题",
            Some(vec!["exp-arch-001".into(), "exp-ai-001".into()]),
        );
        assert_eq!(strategy, "specified");
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"exp-arch-001".to_string()));
        assert!(ids.contains(&"exp-ai-001".to_string()));
        assert!(scores.contains_key("exp-arch-001"));
    }

    // --- 测试 7：round_robin 轮询 ---
    #[test]
    fn test_dispatch_round_robin() {
        let state = test_state();
        state.dispatcher_config.lock().strategy = "round_robin".into();
        state.dispatcher_config.lock().match_threshold = 0.0;

        // 连续调度多次，验证轮询不重复（在 3 个专家间循环）
        let mut first_ids = Vec::new();
        for _ in 0..3 {
            let (ids, _, _) = dispatch_task(&state, "consult", "test", None);
            first_ids.push(ids[0].clone());
        }
        // 3 次应该覆盖不同专家（轮询）
        let unique: std::collections::HashSet<String> = first_ids.iter().cloned().collect();
        assert_eq!(unique.len(), 3);
    }

    // --- 测试 8：consult 一体化 ---
    #[tokio::test]
    async fn test_consult_integration() {
        let state = test_state();
        state.dispatcher_config.lock().strategy = "best_match".into();

        let body = ConsultBody {
            question: "如何设计微服务架构".into(),
            constraints: None,
        };
        let resp = consult(State(state.clone()), Json(body)).await;
        let data = resp.data.unwrap();
        assert!(data["dispatch_id"].as_str().unwrap().starts_with("disp-"));
        assert_eq!(data["expert"]["id"], "exp-arch-001");
        assert!(data["answer"]["analysis"].is_string());
        assert!(data["answer"]["solution"].is_string());
        assert!(data["answer"]["confidence"].as_f64().unwrap() > 0.0);
        assert_eq!(data["question"], "如何设计微服务架构");
    }

    // --- 测试 9：reset-all ---
    #[tokio::test]
    async fn test_reset_all() {
        let state = test_state();
        // 设置所有专家负载
        {
            let mut registry = state.registry.lock();
            for e in registry.values_mut() {
                e.availability.current_load = 10;
            }
        }

        let resp = reset_all(State(state.clone())).await;
        let data = resp.data.unwrap();
        assert_eq!(data["reset_count"], 3);
        assert_eq!(data["reset_expert_ids"].as_array().unwrap().len(), 3);

        // 验证全部清零
        let registry = state.registry.lock();
        for e in registry.values() {
            assert_eq!(e.availability.current_load, 0);
        }
    }
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家注册中心域（Experts Registry）HTTP 路由
//!
//! 提供专家注册中心全域端点：
//! - 专家 CRUD（列表/详情/注册/更新/软删除）
//! - 能力目录聚合
//! - 平台级指标与概览仪表盘
//! - 单个专家衍生指标
//! - 广场扩展端点的真实化版本（stats / consult-room / team / consult-now）
//!
//! 路径前缀：`/api/experts/*`
//! 共享基础：`super::experts_common::*`

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
};
use base64::Engine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

use super::experts_common::*;
use mox_api_protocol::ApiResponse;
use mox_audit::{AuditAction, AuditOutcome};

// =====================================================================
// 一、私有辅助函数
// =====================================================================

/// 从 JSON 请求体合并字段到专家描述符（用于 POST 创建和 PUT 合并式更新）
fn merge_expert_from_value(exp: &mut ExpertDescriptor, body: &Value) {
    if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
        exp.name = name.to_string();
    }
    if let Some(avatar) = body.get("avatar").and_then(|v| v.as_str()) {
        exp.avatar = avatar.to_string();
    }
    if let Some(title) = body.get("title").and_then(|v| v.as_str()) {
        exp.title = title.to_string();
    }
    if let Some(org) = body.get("organization").and_then(|v| v.as_str()) {
        exp.organization = org.to_string();
    }
    if let Some(bio) = body.get("bio").and_then(|v| v.as_str()) {
        exp.bio = bio.to_string();
    }
    if let Some(domains) = body.get("domains").and_then(|v| v.as_array()) {
        exp.domains = domains.iter().filter_map(|v| v.as_str().map(String::from)).collect();
    }
    if let Some(skills) = body.get("skills").and_then(|v| v.as_array()) {
        exp.skills = skills.iter().filter_map(|v| v.as_str().map(String::from)).collect();
    }
    if let Some(caps) = body.get("capabilities").and_then(|v| v.as_array()) {
        exp.capabilities = caps.iter()
            .filter_map(|v| serde_json::from_value::<ExpertCapability>(v.clone()).ok())
            .collect();
    }
    if let Some(et) = body.get("expert_type").and_then(|v| v.as_str()) {
        exp.expert_type = et.to_string();
    }
    if let Some(pm) = body.get("pricing_model").and_then(|v| v.as_str()) {
        exp.pricing_model = pm.to_string();
    }
    if let Some(rate) = body.get("hourly_rate_cents").and_then(|v| v.as_u64()) {
        exp.hourly_rate_cents = rate as u32;
    }
    if let Some(langs) = body.get("languages").and_then(|v| v.as_array()) {
        exp.languages = langs.iter().filter_map(|v| v.as_str().map(String::from)).collect();
    }
    if let Some(tz) = body.get("timezone").and_then(|v| v.as_str()) {
        exp.timezone = tz.to_string();
    }
    if let Some(vs) = body.get("verification_status").and_then(|v| v.as_str()) {
        exp.verification_status = vs.to_string();
    }
    if let Some(tags) = body.get("tags").and_then(|v| v.as_array()) {
        exp.tags = tags.iter().filter_map(|v| v.as_str().map(String::from)).collect();
    }
    // availability 子字段合并
    if let Some(av) = body.get("availability").and_then(|v| v.as_object()) {
        if let Some(s) = av.get("status").and_then(|v| v.as_str()) {
            exp.availability.status = s.to_string();
        }
        if let Some(la) = av.get("last_active").and_then(|v| v.as_str()) {
            exp.availability.last_active = la.to_string();
        }
        if let Some(arm) = av.get("avg_response_minutes").and_then(|v| v.as_f64()) {
            exp.availability.avg_response_minutes = arm;
        }
        if let Some(cl) = av.get("current_load").and_then(|v| v.as_u64()) {
            exp.availability.current_load = cl as u32;
        }
        if let Some(mc) = av.get("max_concurrent").and_then(|v| v.as_u64()) {
            exp.availability.max_concurrent = mc as u32;
        }
    }
    // metrics 子字段合并
    if let Some(mt) = body.get("metrics").and_then(|v| v.as_object()) {
        if let Some(v) = mt.get("total_consultations").and_then(|x| x.as_u64()) {
            exp.metrics.total_consultations = v;
        }
        if let Some(v) = mt.get("today_consultations").and_then(|x| x.as_u64()) {
            exp.metrics.today_consultations = v;
        }
        if let Some(v) = mt.get("avg_rating").and_then(|x| x.as_f64()) {
            exp.metrics.avg_rating = v;
        }
        if let Some(v) = mt.get("rating_count").and_then(|x| x.as_u64()) {
            exp.metrics.rating_count = v;
        }
        if let Some(v) = mt.get("resolution_rate").and_then(|x| x.as_f64()) {
            exp.metrics.resolution_rate = v;
        }
        if let Some(v) = mt.get("first_response_accuracy").and_then(|x| x.as_f64()) {
            exp.metrics.first_response_accuracy = v;
        }
        if let Some(v) = mt.get("total_service_minutes").and_then(|x| x.as_u64()) {
            exp.metrics.total_service_minutes = v;
        }
    }
    // metadata 合并（增量 KV）
    if let Some(meta) = body.get("metadata").and_then(|v| v.as_object()) {
        for (k, v) in meta {
            exp.metadata.insert(k.clone(), v.clone());
        }
    }
}

/// 从注册表计算平台级聚合指标（供 /metrics 与 /stats_real 共享）
fn compute_platform_metrics(registry: &HashMap<String, ExpertDescriptor>) -> Value {
    let enabled: Vec<&ExpertDescriptor> = registry.values().filter(|e| e.enabled).collect();
    let total = enabled.len();

    let online = enabled.iter().filter(|e| e.availability.status == "online").count();
    let busy = enabled.iter().filter(|e| e.availability.status == "busy").count();
    let offline = enabled.iter().filter(|e| e.availability.status == "offline").count();

    let total_consultations: u64 = enabled.iter().map(|e| e.metrics.total_consultations).sum();
    let today_consultations: u64 = enabled.iter().map(|e| e.metrics.today_consultations).sum();

    let avg_rating = if total > 0 {
        enabled.iter().map(|e| e.metrics.avg_rating).sum::<f64>() / total as f64
    } else { 0.0 };

    let avg_response = if total > 0 {
        enabled.iter().map(|e| e.availability.avg_response_minutes).sum::<f64>() / total as f64
    } else { 0.0 };

    // 领域分布
    let mut domain_dist: HashMap<String, u64> = HashMap::new();
    for e in &enabled {
        for d in &e.domains {
            *domain_dist.entry(d.clone()).or_insert(0) += 1;
        }
    }

    let satisfaction_rate = if total > 0 {
        enabled.iter().map(|e| e.metrics.resolution_rate).sum::<f64>() / total as f64
    } else { 0.0 };

    json!({
        "total_experts": total,
        "online_experts": online,
        "busy_experts": busy,
        "offline_experts": offline,
        "total_consultations": total_consultations,
        "today_consultations": today_consultations,
        "avg_rating": avg_rating,
        "avg_response_minutes": avg_response,
        "domain_distribution": domain_dist,
        "satisfaction_rate": satisfaction_rate,
    })
}

/// 生成 JWT-like base64 咨询室令牌
fn make_room_token(room_id: &str, expert_id: &str) -> String {
    let header = json!({"alg":"HS256","typ":"JWT"}).to_string();
    let payload = json!({
        "room_id": room_id,
        "expert_id": expert_id,
        "iat": now_iso(),
        "exp": 3600,
    }).to_string();
    let enc = &base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let h = enc.encode(header.as_bytes());
    let p = enc.encode(payload.as_bytes());
    let sig = hex::encode(uuid::Uuid::new_v4().as_bytes());
    format!("{}.{}.{}", h, p, sig)
}

// =====================================================================
// 二、专家 CRUD（5 个端点）
// =====================================================================

/// GET /api/experts — 专家列表（分页 + 多维度过滤 + 搜索匹配）
async fn list_experts(
    State(s): State<Arc<ExpertsSharedState>>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();
    let (offset, page_size) = parse_pagination(&params);

    let domain_filter = params.get("domain").map(|v| v.to_lowercase());
    let skill_filter = params.get("skill").map(|v| v.to_lowercase());
    let status_filter = params.get("status").map(|v| v.to_lowercase());
    let type_filter = params.get("expert_type").map(|v| v.to_lowercase());
    let search = params.get("search").cloned();
    let sort = params.get("sort").cloned();

    // 基础过滤：仅启用专家 + 各维度过滤
    let mut candidates: Vec<&ExpertDescriptor> = reg.values()
        .filter(|e| e.enabled)
        .filter(|e| {
            if let Some(df) = &domain_filter {
                if !e.domains.iter().any(|d| d.to_lowercase().contains(df)) {
                    return false;
                }
            }
            if let Some(sf) = &skill_filter {
                if !e.skills.iter().any(|sk| sk.to_lowercase().contains(sf)) {
                    return false;
                }
            }
            if let Some(stf) = &status_filter {
                if e.availability.status.to_lowercase() != *stf {
                    return false;
                }
            }
            if let Some(tf) = &type_filter {
                if e.expert_type.to_lowercase() != *tf {
                    return false;
                }
            }
            true
        })
        .collect();

    // 搜索匹配：使用 compute_match_score 过滤并按分数排序
    if let Some(q) = &search {
        let mut scored: Vec<(f64, &ExpertDescriptor)> = candidates.iter()
            .map(|e| (compute_match_score(q, e), *e))
            .filter(|(score, _)| *score > 0.3)
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        candidates = scored.into_iter().map(|(_, e)| e).collect();
    } else if let Some(sort_key) = &sort {
        match sort_key.as_str() {
            "rating" => candidates.sort_by(|a, b| {
                b.metrics.avg_rating.partial_cmp(&a.metrics.avg_rating).unwrap_or(std::cmp::Ordering::Equal)
            }),
            "consultations" => candidates.sort_by(|a, b| {
                b.metrics.total_consultations.cmp(&a.metrics.total_consultations)
            }),
            "name" => candidates.sort_by(|a, b| a.name.cmp(&b.name)),
            _ => {}
        }
    }

    let total = candidates.len();
    let page_items: Vec<&ExpertDescriptor> = candidates.into_iter()
        .skip(offset)
        .take(page_size)
        .collect();

    ok(json!({
        "experts": page_items,
        "total": total,
        "page": (offset / page_size) + 1,
        "page_size": page_size,
    }))
}

/// GET /api/experts/:id — 单个专家详情
async fn get_expert(
    State(s): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();
    match reg.get(&id) {
        Some(exp) if exp.enabled => ok(json!(exp)),
        _ => err(404, format!("expert not found: {}", id)),
    }
}

/// POST /api/experts — 注册专家
async fn create_expert(
    State(s): State<Arc<ExpertsSharedState>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let name = match body.get("name").and_then(|v| v.as_str()) {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => return err(400, "expert name is required"),
    };

    let id = match body.get("id").and_then(|v| v.as_str()) {
        Some(existing_id) if !existing_id.is_empty() => existing_id.to_string(),
        _ => gen_id("exp"),
    };

    let mut reg = s.registry.lock();
    if reg.contains_key(&id) {
        return err(400, format!("expert id already exists: {}", id));
    }

    let mut exp = ExpertDescriptor::minimal(id.clone(), name);
    merge_expert_from_value(&mut exp, &body);
    exp.created_at = now_iso();
    exp.updated_at = exp.created_at.clone();

    reg.insert(id.clone(), exp.clone());
    save_registry(&reg);

    emit_audit(&s, AuditAction::Unknown("expert.register".into()), "expert", &id, AuditOutcome::Success, Some(&format!("name={}", exp.name)));

    ok(json!({
        "expert": exp,
        "created": true,
        "id": id,
    }))
}

/// PUT /api/experts/:id — 合并式更新专家信息
async fn update_expert(
    State(s): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let mut reg = s.registry.lock();
    match reg.get_mut(&id) {
        Some(exp) if exp.enabled => {
            merge_expert_from_value(exp, &body);
            exp.updated_at = now_iso();
            let updated = exp.clone();
            save_registry(&reg);
            emit_audit(&s, AuditAction::Unknown("expert.update".into()), "expert", &id, AuditOutcome::Success, Some(&format!("name={}", updated.name)));
            ok(json!({
                "expert": updated,
                "updated": true,
            }))
        }
        _ => err(404, format!("expert not found: {}", id)),
    }
}

/// DELETE /api/experts/:id — 软删除专家（enabled=false + deleted_at）
async fn delete_expert(
    State(s): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let mut reg = s.registry.lock();
    match reg.get_mut(&id) {
        Some(exp) => {
            exp.enabled = false;
            exp.updated_at = now_iso();
            exp.metadata.insert("deleted_at".into(), json!(now_iso()));
            save_registry(&reg);
            emit_audit(&s, AuditAction::Unknown("expert.disable".into()), "expert", &id, AuditOutcome::Success, Some("soft_delete"));
            ok(json!({
                "id": id,
                "deleted": true,
                "soft_delete": true,
                "message": "expert has been soft-deleted",
            }))
        }
        None => err(404, format!("expert not found: {}", id)),
    }
}

// =====================================================================
// 三、能力目录（1 个端点）
// =====================================================================

/// GET /api/experts/capabilities — 从注册表聚合去重后的能力目录
async fn list_capabilities(
    State(s): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();

    // capability_id -> (name, domain, expert_count, proficiency_sum)
    let mut cap_map: HashMap<String, (String, String, u64, f64)> = HashMap::new();
    let mut domains: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for exp in reg.values().filter(|e| e.enabled) {
        for cap in &exp.capabilities {
            domains.insert(cap.domain.clone());
            let entry = cap_map.entry(cap.id.clone()).or_insert_with(|| {
                (cap.name.clone(), cap.domain.clone(), 0, 0.0)
            });
            entry.2 += 1;
            entry.3 += cap.proficiency as f64;
        }
    }

    let mut capabilities: Vec<Value> = cap_map.into_iter()
        .map(|(id, (name, domain, count, prof_sum))| {
            json!({
                "id": id,
                "name": name,
                "domain": domain,
                "expert_count": count,
                "avg_proficiency": if count > 0 { prof_sum / count as f64 } else { 0.0 },
            })
        })
        .collect();
    capabilities.sort_by(|a, b| {
        a.get("id").and_then(|v| v.as_str()).cmp(&b.get("id").and_then(|v| v.as_str()))
    });

    let domain_list: Vec<String> = domains.into_iter().collect();

    ok(json!({
        "capabilities": capabilities,
        "total": capabilities.len(),
        "domains": domain_list,
    }))
}

// =====================================================================
// 四、指标与概览（3 个端点）
// =====================================================================

/// GET /api/experts/metrics — 平台级专家指标聚合（实时计算，非零值 stub）
async fn platform_metrics(
    State(s): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();
    let metrics = compute_platform_metrics(&reg);
    ok(metrics)
}

/// GET /api/experts/overview — 概览仪表盘
async fn platform_overview(
    State(s): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();
    let sessions = s.sessions.lock();

    let enabled: Vec<&ExpertDescriptor> = reg.values().filter(|e| e.enabled).collect();
    let experts_count = enabled.len();

    let active_sessions_count = sessions.values()
        .filter(|sess| sess.status == "active")
        .count();

    let today_consultations: u64 = enabled.iter().map(|e| e.metrics.today_consultations).sum();
    let avg_rating = if experts_count > 0 {
        enabled.iter().map(|e| e.metrics.avg_rating).sum::<f64>() / experts_count as f64
    } else { 0.0 };

    // 评分前 5
    let mut top_rated: Vec<&ExpertDescriptor> = enabled.clone();
    top_rated.sort_by(|a, b| {
        b.metrics.avg_rating.partial_cmp(&a.metrics.avg_rating).unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_rated_experts: Vec<Value> = top_rated.iter().take(5)
        .map(|e| json!({
            "id": e.id,
            "name": e.name,
            "title": e.title,
            "avg_rating": e.metrics.avg_rating,
            "rating_count": e.metrics.rating_count,
        }))
        .collect();

    // 咨询量前 5
    let mut most_active: Vec<&ExpertDescriptor> = enabled.clone();
    most_active.sort_by(|a, b| b.metrics.total_consultations.cmp(&a.metrics.total_consultations));
    let most_active_experts: Vec<Value> = most_active.iter().take(5)
        .map(|e| json!({
            "id": e.id,
            "name": e.name,
            "title": e.title,
            "total_consultations": e.metrics.total_consultations,
        }))
        .collect();

    // 领域分布
    let mut domain_breakdown: HashMap<String, u64> = HashMap::new();
    for e in &enabled {
        for d in &e.domains {
            *domain_breakdown.entry(d.clone()).or_insert(0) += 1;
        }
    }

    ok(json!({
        "experts_count": experts_count,
        "active_sessions_count": active_sessions_count,
        "today_consultations": today_consultations,
        "avg_rating": avg_rating,
        "top_rated_experts": top_rated_experts,
        "most_active_experts": most_active_experts,
        "domain_breakdown": domain_breakdown,
    }))
}

/// GET /api/experts/:id/metrics — 单个专家指标 + 衍生指标
async fn expert_metrics(
    State(s): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();
    let exp = match reg.get(&id) {
        Some(e) if e.enabled => e.clone(),
        _ => return err(404, format!("expert not found: {}", id)),
    };

    // 计算排名百分位：该专家评分高于多少比例的专家
    let all_enabled: Vec<&ExpertDescriptor> = reg.values().filter(|e| e.enabled).collect();
    let total = all_enabled.len() as f64;
    let better_count = all_enabled.iter()
        .filter(|e| e.metrics.avg_rating < exp.metrics.avg_rating)
        .count() as f64;
    let rank_percentile = if total > 0.0 { (better_count / total) * 100.0 } else { 0.0 };

    // 负载比
    let load_ratio = if exp.availability.max_concurrent > 0 {
        exp.availability.current_load as f64 / exp.availability.max_concurrent as f64
    } else { 0.0 };

    // 效率评分：评分 40% + 解决率 30% + 空闲度 30%
    let efficiency_score = (exp.metrics.avg_rating / 5.0).min(1.0) * 0.4
        + exp.metrics.resolution_rate.min(1.0) * 0.3
        + (1.0 - load_ratio.min(1.0)) * 0.3;

    ok(json!({
        "expert_id": exp.id,
        "metrics": exp.metrics,
        "availability": exp.availability,
        "derived": {
            "rank_percentile": rank_percentile,
            "load_ratio": load_ratio,
            "efficiency_score": efficiency_score,
        },
    }))
}

// =====================================================================
// 五、广场扩展 stub 真实化增强（4 个端点，函数名加 _real 后缀）
// =====================================================================

/// GET /api/experts/stats — 真实统计（字段对齐前端期望，含 domains 对象与 ts）
async fn experts_stats_real(
    State(s): State<Arc<ExpertsSharedState>>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();
    let m = compute_platform_metrics(&reg);

    // 将 domain_distribution 转为前端期望的 domains 对象（固定键 + 动态补充）
    let domain_dist = m.get("domain_distribution")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut domains = json!({
        "architecture": 0,
        "data": 0,
        "ai": 0,
        "cloud": 0,
        "security": 0,
        "devops": 0,
        "product": 0,
        "other": 0,
    });
    if let Some(dom_obj) = domains.as_object_mut() {
        for (k, v) in &domain_dist {
            dom_obj.insert(k.clone(), v.clone());
        }
    }

    ok(json!({
        "total_experts": m["total_experts"],
        "online_experts": m["online_experts"],
        "busy_experts": m["busy_experts"],
        "offline_experts": m["offline_experts"],
        "total_consultations": m["total_consultations"],
        "today_consultations": m["today_consultations"],
        "avg_rating": m["avg_rating"],
        "avg_response_minutes": m["avg_response_minutes"],
        "domains": domains,
        "satisfaction_rate": m["satisfaction_rate"],
        "ts": now_iso(),
    }))
}

/// GET /api/experts/bookings/:id/consult-room — 真实咨询室（生成令牌与 WebRTC 配置）
async fn consult_room_real(
    State(s): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    let reg = s.registry.lock();
    let expert = reg.get(&id).cloned();

    let room_id = gen_id("room");
    let room_token = make_room_token(&room_id, &id);
    let join_url = format!("/consult/room/{}", room_id);

    let expert_info = expert.as_ref().map(|e| {
        json!({
            "id": e.id,
            "name": e.name,
            "title": e.title,
            "avatar": e.avatar,
            "online": e.availability.status == "online",
        })
    });

    let status = if expert.as_ref().map(|e| e.availability.status == "online").unwrap_or(false) {
        "available"
    } else {
        "waiting"
    };

    ok(json!({
        "booking_id": id,
        "room_id": room_id,
        "room_token": room_token,
        "join_url": join_url,
        "webrtc_config": {
            "ice_servers": [
                { "urls": "stun:stun.l.google.com:19302" },
                { "urls": "stun:stun1.l.google.com:19302" },
            ],
        },
        "expert_info": expert_info,
        "status": status,
        "expires_in": 3600,
        "created_at": now_iso(),
    }))
}

/// POST /api/experts/team — 真实团队加入（验证专家存在性，已验证则自动批准）
async fn join_team_real(
    State(s): State<Arc<ExpertsSharedState>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let team_id = match body.get("team_id").and_then(|v| v.as_str()) {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => return err(400, "team_id is required"),
    };
    let expert_id = body.get("expert_id").and_then(|v| v.as_str()).map(String::from);
    let role = body.get("role").and_then(|v| v.as_str()).unwrap_or("member").to_string();

    let (status, verified_expert_id) = if let Some(eid) = &expert_id {
        let reg = s.registry.lock();
        match reg.get(eid) {
            Some(exp) if exp.enabled => {
                if exp.verification_status == "verified" || exp.verification_status == "certified" {
                    ("approved", eid.clone())
                } else {
                    ("pending_approval", eid.clone())
                }
            }
            _ => return err(404, format!("expert not found: {}", eid)),
        }
    } else {
        ("pending_approval", "expert-current".to_string())
    };

    let application_id = gen_id("app");
    let estimated_review_hours = if status == "approved" { 0 } else { 24 };

    ok(json!({
        "application_id": application_id,
        "status": status,
        "team_id": team_id,
        "expert_id": verified_expert_id,
        "role": role,
        "applied_at": now_iso(),
        "estimated_review_hours": estimated_review_hours,
        "message": if status == "approved" {
            "专家已验证，自动批准加入团队"
        } else {
            "申请已提交，等待团队管理员审批"
        },
    }))
}

/// POST /api/experts/:id/consult-now — 真实即时咨询（验证在线 + 创建会话）
async fn consult_now_real(
    State(s): State<Arc<ExpertsSharedState>>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let topic = body.get("topic").and_then(|v| v.as_str()).unwrap_or("即时咨询").to_string();
    let question = body.get("question").and_then(|v| v.as_str()).map(String::from);
    let channel = body.get("channel").and_then(|v| v.as_str()).unwrap_or("text").to_string();

    // 验证专家存在且在线
    let expert_online = {
        let reg = s.registry.lock();
        match reg.get(&id) {
            Some(exp) if exp.enabled => exp.availability.status == "online",
            _ => return err(404, format!("expert not found: {}", id)),
        }
    };

    if !expert_online {
        return ok(json!({
            "expert_id": id,
            "session_id": null,
            "status": "unavailable",
            "channel": channel,
            "topic": topic,
            "question": question,
            "expert_online": false,
            "chat_url": null,
            "created_at": now_iso(),
            "message": "专家当前不在线，请稍后重试或预约",
        }));
    }

    // 创建会话写入共享状态
    let session_id = gen_id("sess");
    let now = now_iso();
    {
        let mut sessions = s.sessions.lock();
        let mut meta = HashMap::new();
        meta.insert("channel".into(), json!(channel));
        if let Some(q) = &question {
            meta.insert("question".into(), json!(q));
        }
        let session = ExpertSession {
            id: session_id.clone(),
            title: topic.clone(),
            expert_ids: vec![id.clone()],
            user_id: "guest-user".into(),
            session_type: "single".into(),
            status: "active".into(),
            topic: topic.clone(),
            messages: Vec::new(),
            tags: Vec::new(),
            metadata: meta,
            created_at: now.clone(),
            last_active_at: now.clone(),
            archived_at: None,
        };
        sessions.insert(session_id.clone(), session);
    }

    //  increment expert consultation counters
    {
        let mut reg = s.registry.lock();
        if let Some(exp) = reg.get_mut(&id) {
            exp.metrics.total_consultations += 1;
            exp.metrics.today_consultations += 1;
            exp.availability.current_load += 1;
            exp.availability.last_active = now.clone();
            save_registry(&reg);
        }
    }

    emit_audit(&s, AuditAction::Unknown("expert.consult_now".into()), "session", &session_id, AuditOutcome::Success, Some(&format!("expert_id={}, topic={}", id, topic)));

    let chat_url = format!("/chat/{}", session_id);

    ok(json!({
        "expert_id": id,
        "session_id": session_id,
        "status": "connected",
        "channel": channel,
        "topic": topic,
        "question": question,
        "expert_online": true,
        "chat_url": chat_url,
        "created_at": now,
    }))
}

// =====================================================================
// 六、路由装配
// =====================================================================

/// 构建专家注册中心域路由（由调用方传入共享状态，确保全域状态一致）
pub fn build_experts_registry_router(state: Arc<ExpertsSharedState>) -> Router {
    Router::new()
        // —— 专家 CRUD ——
        .route("/api/experts", get(list_experts).post(create_expert))
        .route("/api/experts/capabilities", get(list_capabilities))
        .route("/api/experts/metrics", get(platform_metrics))
        .route("/api/experts/overview", get(platform_overview))
        .route("/api/experts/stats", get(experts_stats_real))
        .route("/api/experts/:id", get(get_expert).put(update_expert).delete(delete_expert))
        .route("/api/experts/:id/metrics", get(expert_metrics))
        // —— 广场扩展真实化端点 ——
        .route("/api/experts/bookings/:id/consult-room", get(consult_room_real))
        .route("/api/experts/team", post(join_team_real))
        .route("/api/experts/:id/consult-now", post(consult_now_real))
        .with_state(state)
}

// =====================================================================
// 七、单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;

    fn make_test_state() -> Arc<ExpertsSharedState> {
        Arc::new(ExpertsSharedState {
            registry: Arc::new(Mutex::new(HashMap::new())),
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

    fn seed_expert(state: &Arc<ExpertsSharedState>, id: &str, name: &str, domains: Vec<&str>) {
        let mut reg = state.registry.lock();
        let mut exp = ExpertDescriptor::minimal(id.into(), name.into());
        exp.domains = domains.into_iter().map(String::from).collect();
        exp.skills = vec!["Rust".into(), "Python".into()];
        exp.metrics.avg_rating = 4.5;
        exp.metrics.total_consultations = 100;
        exp.metrics.resolution_rate = 0.9;
        reg.insert(id.into(), exp);
    }

    // 测试 1：创建专家（POST /api/experts）
    #[tokio::test]
    async fn test_create_expert() {
        let state = make_test_state();
        let body = json!({
            "name": "测试专家·甲",
            "title": "高级架构师",
            "domains": ["architecture", "backend"],
            "skills": ["Rust", "Go"],
            "expert_type": "ai",
        });
        let resp = create_expert(State(state.clone()), Json(body)).await;
        assert!(resp.data.is_some());
        let d = resp.data.unwrap();
        assert_eq!(d["created"], true);
        assert!(d["expert"]["name"].as_str().unwrap().contains("测试专家"));
        assert_eq!(d["expert"]["expert_type"], "ai");

        // 验证持久化到注册表
        let reg = state.registry.lock();
        let id = d["id"].as_str().unwrap();
        assert!(reg.contains_key(id));
    }

    // 测试 2：获取不存在的专家返回 404
    #[tokio::test]
    async fn test_get_expert_not_found() {
        let state = make_test_state();
        let resp = get_expert(State(state), Path("nonexistent-999".into())).await;
        assert!(resp.data.is_none());
        assert_eq!(resp.code, 404);
    }

    // 测试 3：合并式更新专家（PUT /api/experts/:id）
    #[tokio::test]
    async fn test_update_expert_merge() {
        let state = make_test_state();
        seed_expert(&state, "exp-update-001", "原名称", vec!["ai"]);

        let body = json!({
            "title": "更新后的头衔",
            "hourly_rate_cents": 5000,
            "metrics": { "avg_rating": 4.9 },
        });
        let resp = update_expert(State(state.clone()), Path("exp-update-001".into()), Json(body)).await;
        let data = resp.data.unwrap();
        assert_eq!(data["updated"], true);
        assert_eq!(data["expert"]["title"], "更新后的头衔");
        assert_eq!(data["expert"]["hourly_rate_cents"], 5000);
        assert_eq!(data["expert"]["metrics"]["avg_rating"], 4.9);
        // 未提供的字段保持不变
        assert_eq!(data["expert"]["name"], "原名称");
        assert!(data["expert"]["domains"].as_array().unwrap().iter().any(|d| d == "ai"));
    }

    // 测试 4：软删除专家（DELETE /api/experts/:id）
    #[tokio::test]
    async fn test_soft_delete_expert() {
        let state = make_test_state();
        seed_expert(&state, "exp-del-001", "待删除专家", vec!["data"]);

        let resp = delete_expert(State(state.clone()), Path("exp-del-001".into())).await;
        let data = resp.data.unwrap();
        assert_eq!(data["deleted"], true);
        assert_eq!(data["soft_delete"], true);

        // 验证 enabled=false 且 deleted_at 已记录
        let reg = state.registry.lock();
        let exp = reg.get("exp-del-001").unwrap();
        assert!(!exp.enabled);
        assert!(exp.metadata.contains_key("deleted_at"));

        // 验证列表中不再出现
        drop(reg);
        let mut params = HashMap::new();
        let list_resp = list_experts(State(state), Query(params)).await;
        let list_data = list_resp.data.unwrap();
        let experts = list_data["experts"].as_array().unwrap();
        assert!(experts.iter().all(|e| e["id"] != "exp-del-001"));
    }

    // 测试 5：搜索匹配过滤与排序
    #[tokio::test]
    async fn test_match_search_filtering() {
        let state = make_test_state();
        seed_expert(&state, "exp-search-001", "架构师·玄枢", vec!["architecture", "backend"]);
        seed_expert(&state, "exp-search-002", "AI算法·灵玑", vec!["ai", "ml"]);
        seed_expert(&state, "exp-search-003", "数据工程·衡宇", vec!["data", "database"]);

        let mut params = HashMap::new();
        params.insert("search".into(), "架构 backend Kubernetes".into());
        let resp = list_experts(State(state), Query(params)).await;
        let data = resp.data.unwrap();
        let experts = data["experts"].as_array().unwrap();
        // 架构师应排在第一位（匹配度最高）
        assert!(!experts.is_empty());
        assert_eq!(experts[0]["id"], "exp-search-001");
        // 搜索结果应过滤掉完全不匹配的
        assert!(experts.iter().all(|e| {
            let id = e["id"].as_str().unwrap();
            id != "exp-search-003" // 数据工程与"架构 backend Rust"不匹配
        }));
    }

    // 测试 6：平台指标聚合（非零值 stub）
    #[tokio::test]
    async fn test_metrics_aggregation() {
        let state = make_test_state();
        seed_expert(&state, "exp-metric-001", "专家A", vec!["ai"]);
        seed_expert(&state, "exp-metric-002", "专家B", vec!["architecture", "ai"]);

        let resp = platform_metrics(State(state)).await;
        let data = resp.data.unwrap();
        assert_eq!(data["total_experts"], 2);
        assert_eq!(data["total_consultations"], 200); // 每个 100
        assert_eq!(data["avg_rating"], 4.5);
        // 领域分布：ai 出现 2 次，architecture 出现 1 次
        let dist = data["domain_distribution"].as_object().unwrap();
        assert_eq!(dist["ai"], 2);
        assert_eq!(dist["architecture"], 1);
        assert!(data["satisfaction_rate"].as_f64().unwrap() > 0.0);
    }
}

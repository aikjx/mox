// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家广场扩展域（Experts Ext）HTTP 路由
//!
//! 提供专家平台统计、预约管理、收藏、咨询室、团队加入、即时咨询等专家广场能力。
//!
//! 路径前缀：`/experts/*`

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post, put},
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use mox_api_protocol::{ApiResponse, api_ok, api_error};

// =====================================================================
// 共享状态
// =====================================================================

#[derive(Debug, Clone, Serialize)]
struct Booking {
    id: String,
    expert_id: String,
    expert_name: String,
    user_id: String,
    topic: String,
    scheduled_at: String,
    duration_minutes: i64,
    status: String,
    created_at: String,
}

#[derive(Clone)]
struct ExpertsState {
    bookings: Arc<Mutex<Vec<Booking>>>,
    favorites: Arc<Mutex<std::collections::HashSet<String>>>,
}

impl ExpertsState {
    fn new() -> Self {
        Self {
            bookings: Arc::new(Mutex::new(Vec::new())),
            favorites: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn ok(data: Value) -> ApiResponse<Value> {
    api_ok(data)
}

// =====================================================================
// 1. GET /experts/stats — 平台统计
// =====================================================================
async fn experts_stats() -> ApiResponse<Value> {
    ok(json!({
        "total_experts": 0,
        "online_experts": 0,
        "busy_experts": 0,
        "offline_experts": 0,
        "total_consultations": 0,
        "today_consultations": 0,
        "avg_rating": 0.0,
        "avg_response_minutes": 0.0,
        "domains": {
            "architecture": 0,
            "data": 0,
            "ai": 0,
            "cloud": 0,
            "security": 0,
            "devops": 0,
            "product": 0,
            "other": 0,
        },
        "satisfaction_rate": 0.0,
        "ts": now_iso(),
    }))
}

// =====================================================================
// 2. GET /experts/bookings/mine — 我的预约列表
// =====================================================================
async fn my_bookings(State(s): State<Arc<ExpertsState>>) -> ApiResponse<Value> {
    let bookings = s.bookings.lock().clone();
    ok(json!({
        "bookings": bookings,
        "total": bookings.len(),
        "pending": bookings.iter().filter(|b| b.status == "pending").count(),
        "confirmed": bookings.iter().filter(|b| b.status == "confirmed").count(),
        "completed": bookings.iter().filter(|b| b.status == "completed").count(),
        "cancelled": bookings.iter().filter(|b| b.status == "cancelled").count(),
    }))
}

// =====================================================================
// 3. POST /experts/{id}/favorite — 专家收藏切换
// =====================================================================
async fn toggle_expert_favorite(
    Path(id): Path<String>,
    State(s): State<Arc<ExpertsState>>,
) -> ApiResponse<Value> {
    let mut favs = s.favorites.lock();
    let is_fav = favs.contains(&id);
    if is_fav {
        favs.remove(&id);
    } else {
        favs.insert(id.clone());
    }
    ok(json!({
        "expert_id": id,
        "favorite": !is_fav,
        "action": if is_fav { "unfavorited" } else { "favorited" },
        "updated_at": now_iso(),
    }))
}

// =====================================================================
// 4. POST /experts/bookings — 预约创建
// =====================================================================

#[derive(Debug, Deserialize)]
struct CreateBookingBody {
    expert_id: String,
    topic: String,
    scheduled_at: Option<String>,
    duration_minutes: Option<i64>,
    description: Option<String>,
}

async fn create_booking(
    State(s): State<Arc<ExpertsState>>,
    Json(body): Json<CreateBookingBody>,
) -> ApiResponse<Value> {
    let booking_id = format!("booking-{}", uuid::Uuid::new_v4().simple());
    let scheduled = body.scheduled_at.unwrap_or_else(|| {
        (chrono::Utc::now() + chrono::Duration::hours(24))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    });
    let duration = body.duration_minutes.unwrap_or(60);
    let booking = Booking {
        id: booking_id.clone(),
        expert_id: body.expert_id.clone(),
        expert_name: format!("专家-{}", &body.expert_id),
        user_id: "admin-user".into(),
        topic: body.topic.clone(),
        scheduled_at: scheduled.clone(),
        duration_minutes: duration,
        status: "pending".into(),
        created_at: now_iso(),
    };
    s.bookings.lock().push(booking.clone());
    ok(json!(booking))
}

// =====================================================================
// 5. PUT /experts/bookings/{id}/cancel — 预约取消
// =====================================================================
async fn cancel_booking(
    Path(id): Path<String>,
    State(s): State<Arc<ExpertsState>>,
) -> ApiResponse<Value> {
    let mut bookings = s.bookings.lock();
    if let Some(b) = bookings.iter_mut().find(|b| b.id == id) {
        if b.status == "cancelled" || b.status == "completed" {
            return api_error(400, format!("预约 {} 当前状态为 {}，无法取消", id, b.status));
        }
        b.status = "cancelled".into();
        return ok(json!({
            "booking_id": id,
            "status": "cancelled",
            "cancelled_at": now_iso(),
            "message": "预约已取消",
        }));
    }
    api_error(404, format!("booking not found: {id}"))
}

// =====================================================================
// 6. GET /experts/bookings/{id}/consult-room — 咨询室进入
// =====================================================================
async fn consult_room(Path(id): Path<String>) -> ApiResponse<Value> {
    ok(json!({
        "booking_id": id,
        "room_id": format!("room-{}", id),
        "room_token": null,
        "join_url": null,
        "webrtc_config": {
            "ice_servers": [],
        },
        "expert_info": null,
        "status": "unavailable",
        "expires_in": 0,
        "created_at": now_iso(),
    }))
}

// =====================================================================
// 7. POST /experts/team — 加入团队
// =====================================================================

#[derive(Debug, Deserialize)]
struct JoinTeamBody {
    team_id: String,
    expert_id: Option<String>,
    role: Option<String>,
    message: Option<String>,
}

async fn join_team(Json(body): Json<JoinTeamBody>) -> ApiResponse<Value> {
    let role = body.role.unwrap_or_else(|| "member".into());
    ok(json!({
        "team_id": body.team_id,
        "expert_id": body.expert_id.unwrap_or_else(|| "expert-current".into()),
        "role": role,
        "status": "pending_approval",
        "application_id": format!("app-{}", uuid::Uuid::new_v4().simple()),
        "message": body.message.unwrap_or_else(|| "申请加入团队".into()),
        "applied_at": now_iso(),
        "estimated_review_hours": 0,
    }))
}

// =====================================================================
// 8. POST /experts/{id}/consult-now — 即时咨询
// =====================================================================

#[derive(Debug, Deserialize)]
struct ConsultNowBody {
    topic: Option<String>,
    question: Option<String>,
    channel: Option<String>,
}

async fn consult_now(
    Path(id): Path<String>,
    Json(body): Json<ConsultNowBody>,
) -> ApiResponse<Value> {
    ok(json!({
        "expert_id": id,
        "session_id": null,
        "status": "unavailable",
        "channel": body.channel.unwrap_or_else(|| "text".into()),
        "topic": body.topic.unwrap_or_else(|| "即时咨询".into()),
        "question": body.question,
        "estimated_wait_seconds": 0,
        "chat_url": null,
        "expert_online": false,
        "created_at": now_iso(),
    }))
}

// =====================================================================
// 路由装配
// =====================================================================

pub fn build_experts_ext_router() -> Router {
    let state = Arc::new(ExpertsState::new());
    Router::new()
        .route("/experts/stats", get(experts_stats))
        .route("/experts/bookings/mine", get(my_bookings))
        .route("/experts/:id/favorite", post(toggle_expert_favorite))
        .route("/experts/bookings", post(create_booking))
        .route("/experts/bookings/:id/cancel", put(cancel_booking))
        .route("/experts/bookings/:id/consult-room", get(consult_room))
        .route("/experts/team", post(join_team))
        .route("/experts/:id/consult-now", post(consult_now))
        .with_state(state)
}

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
        let now = chrono::Utc::now();
        let seed = vec![
            Booking {
                id: "booking-001".into(),
                expert_id: "expert-arch-001".into(),
                expert_name: "架构优化专家".into(),
                user_id: "admin-user".into(),
                topic: "微服务架构拆分咨询".into(),
                scheduled_at: (now + chrono::Duration::hours(48))
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                duration_minutes: 60,
                status: "confirmed".into(),
                created_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            },
            Booking {
                id: "booking-002".into(),
                expert_id: "expert-data-001".into(),
                expert_name: "数据工程专家".into(),
                user_id: "admin-user".into(),
                topic: "数据管道性能优化".into(),
                scheduled_at: (now - chrono::Duration::hours(24))
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                duration_minutes: 90,
                status: "completed".into(),
                created_at: (now - chrono::Duration::hours(72))
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            },
            Booking {
                id: "booking-003".into(),
                expert_id: "expert-ai-001".into(),
                expert_name: "AI 算法专家".into(),
                user_id: "admin-user".into(),
                topic: "推荐系统方案评审".into(),
                scheduled_at: (now + chrono::Duration::hours(24))
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                duration_minutes: 45,
                status: "pending".into(),
                created_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            },
        ];
        Self {
            bookings: Arc::new(Mutex::new(seed)),
            favorites: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn ok(data: Value) -> Json<Value> {
    Json(json!({ "success": true, "data": data }))
}

// =====================================================================
// 1. GET /experts/stats — 平台统计
// =====================================================================
async fn experts_stats() -> Json<Value> {
    ok(json!({
        "total_experts": 342,
        "online_experts": 128,
        "busy_experts": 45,
        "offline_experts": 169,
        "total_consultations": 12847,
        "today_consultations": 86,
        "avg_rating": 4.7,
        "avg_response_minutes": 8.5,
        "domains": {
            "architecture": 58,
            "data": 72,
            "ai": 45,
            "cloud": 38,
            "security": 32,
            "devops": 28,
            "product": 41,
            "other": 28,
        },
        "satisfaction_rate": 96.8,
        "ts": now_iso(),
    }))
}

// =====================================================================
// 2. GET /experts/bookings/mine — 我的预约列表
// =====================================================================
async fn my_bookings(State(s): State<Arc<ExpertsState>>) -> Json<Value> {
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
) -> Json<Value> {
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
) -> Json<Value> {
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
) -> Json<Value> {
    let mut bookings = s.bookings.lock();
    if let Some(b) = bookings.iter_mut().find(|b| b.id == id) {
        if b.status == "cancelled" || b.status == "completed" {
            return Json(json!({
                "success": false,
                "error": format!("预约 {} 当前状态为 {}，无法取消", id, b.status),
            }));
        }
        b.status = "cancelled".into();
        return ok(json!({
            "booking_id": id,
            "status": "cancelled",
            "cancelled_at": now_iso(),
            "message": "预约已取消",
        }));
    }
    Json(json!({ "success": false, "error": format!("booking not found: {id}") }))
}

// =====================================================================
// 6. GET /experts/bookings/{id}/consult-room — 咨询室进入
// =====================================================================
async fn consult_room(Path(id): Path<String>) -> Json<Value> {
    let room_token = uuid::Uuid::new_v4().simple().to_string();
    ok(json!({
        "booking_id": id,
        "room_id": format!("room-{}", id),
        "room_token": room_token,
        "join_url": format!("/consult/rooms/{}?token={}", id, room_token),
        "webrtc_config": {
            "ice_servers": [
                { "urls": "stun:stun.l.google.com:19302" },
                { "urls": "turn:turn.mox.example:3478", "username": "mox", "credential": "***" },
            ],
        },
        "expert_info": {
            "expert_id": "expert-arch-001",
            "name": "架构优化专家",
            "avatar": "/avatars/expert-arch-001.png",
            "online": true,
        },
        "status": "ready",
        "expires_in": 3600,
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

async fn join_team(Json(body): Json<JoinTeamBody>) -> Json<Value> {
    let role = body.role.unwrap_or_else(|| "member".into());
    ok(json!({
        "team_id": body.team_id,
        "expert_id": body.expert_id.unwrap_or_else(|| "expert-current".into()),
        "role": role,
        "status": "pending_approval",
        "application_id": format!("app-{}", uuid::Uuid::new_v4().simple()),
        "message": body.message.unwrap_or_else(|| "申请加入团队".into()),
        "applied_at": now_iso(),
        "estimated_review_hours": 24,
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
) -> Json<Value> {
    let session_id = format!("session-{}", uuid::Uuid::new_v4().simple());
    ok(json!({
        "expert_id": id,
        "session_id": session_id,
        "status": "connecting",
        "channel": body.channel.unwrap_or_else(|| "text".into()),
        "topic": body.topic.unwrap_or_else(|| "即时咨询".into()),
        "question": body.question,
        "estimated_wait_seconds": 45,
        "chat_url": format!("/consult/chat/{}", session_id),
        "expert_online": true,
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

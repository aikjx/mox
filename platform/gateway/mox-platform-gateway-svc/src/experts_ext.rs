// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家广场扩展域（Experts Ext）HTTP 路由
//!
//! 提供专家预约管理与专家收藏能力，路由前缀 `/api/experts/*`。
//!
//! ## 归一化收口（2026-09-05）
//! 本模块与注册中心域（`experts_registry`）、智能协作域（`experts_collaboration`）等
//! 共用同一份 `ExpertsSharedState`：
//! - **收藏集合**统一落在 `ExpertsSharedState.favorites`，不再在本模块内另存一份，
//!   消除「广场端收藏 → 注册中心端读不到」的数据分裂；
//! - 平台统计 `/api/experts/stats`、咨询室 `/api/experts/bookings/:id/consult-room`、
//!   加入团队 `/api/experts/team`、即时咨询 `/api/experts/:id/consult-now` 四个端点
//!   已由 `experts_registry` 提供真实实现，本模块的同义占位实现已移除（原为死代码）。
//!
//! 预约（`Booking`）经 `experts_db` 落 SQLite（`data/experts.db`，WAL + 事务），
//! 历史 `data/experts_bookings.json` 在启动时自动导入并归档。

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

use crate::experts_common::ExpertsSharedState;

// =====================================================================
// 领域模型 + 持久化
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// 预约读取：经 experts_db 从 SQLite 载入
fn load_experts_bookings() -> Vec<Booking> {
    crate::experts_db::load_bookings()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect()
}

/// 预约写入：经 experts_db 事务化全量同步落 SQLite
fn save_experts_bookings(bookings: &[Booking]) {
    let rows: Vec<Value> = bookings
        .iter()
        .filter_map(|b| serde_json::to_value(b).ok())
        .collect();
    crate::experts_db::save_bookings(&rows);
}

// =====================================================================
// 共享状态
// =====================================================================

/// 专家广场扩展域状态
///
/// 仅持有本域独有的预约集合；收藏集合复用全域共享状态，避免双份存储。
#[derive(Clone)]
struct ExpertsState {
    bookings: Arc<Mutex<Vec<Booking>>>,
    shared: Arc<ExpertsSharedState>,
}

impl ExpertsState {
    fn new(shared: Arc<ExpertsSharedState>) -> Self {
        Self {
            bookings: Arc::new(Mutex::new(load_experts_bookings())),
            shared,
        }
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn ok(data: Value) -> ApiResponse<Value> {
    api_ok(data)
}

/// 从注册中心解析专家展示名
///
/// 预约必须指向真实存在的专家：未注册（或已软删除）的专家一律拒绝，
/// 防止产生 `expert_name` 为占位串、专家 ID 悬空的脏数据。
fn resolve_expert_name(shared: &ExpertsSharedState, expert_id: &str) -> Result<String, String> {
    let reg = shared.registry.lock();
    match reg.get(expert_id) {
        Some(e) if e.enabled => Ok(e.name.clone()),
        Some(_) => Err(format!("expert disabled: {expert_id}")),
        None => Err(format!("expert not found: {expert_id}")),
    }
}

// =====================================================================
// 1. GET /experts/bookings/mine — 我的预约列表
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
// 2. POST /experts/{id}/favorite — 专家收藏切换
// =====================================================================
async fn toggle_expert_favorite(
    Path(id): Path<String>,
    State(s): State<Arc<ExpertsState>>,
) -> ApiResponse<Value> {
    // 收藏集合归一化：全域唯一真源为 ExpertsSharedState.favorites
    let mut favs = s.shared.favorites.lock();
    let is_fav = favs.contains(&id);
    if is_fav {
        favs.remove(&id);
    } else {
        favs.insert(id.clone());
    }
    drop(favs);
    ok(json!({
        "expert_id": id,
        "favorite": !is_fav,
        "action": if is_fav { "unfavorited" } else { "favorited" },
        "updated_at": now_iso(),
    }))
}

// =====================================================================
// 3. POST /experts/bookings — 预约创建
// =====================================================================

#[derive(Debug, Deserialize)]
struct CreateBookingBody {
    expert_id: String,
    topic: String,
    scheduled_at: Option<String>,
    duration_minutes: Option<i64>,
    #[allow(dead_code)]
    description: Option<String>,
}

async fn create_booking(
    State(s): State<Arc<ExpertsState>>,
    Json(body): Json<CreateBookingBody>,
) -> ApiResponse<Value> {
    // 专家名取注册中心真实数据；未注册专家直接拒绝，避免产生指向空专家的脏预约
    let expert_name = match resolve_expert_name(&s.shared, &body.expert_id) {
        Ok(name) => name,
        Err(msg) => return api_error(404, msg),
    };

    let booking_id = format!("booking-{}", uuid::Uuid::new_v4().simple());
    let scheduled = body.scheduled_at.unwrap_or_else(|| {
        (chrono::Utc::now() + chrono::Duration::hours(24))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    });
    let duration = body.duration_minutes.unwrap_or(60);

    let booking = Booking {
        id: booking_id.clone(),
        expert_id: body.expert_id.clone(),
        expert_name,
        user_id: "admin-user".into(),
        topic: body.topic.clone(),
        scheduled_at: scheduled,
        duration_minutes: duration,
        status: "pending".into(),
        created_at: now_iso(),
    };

    // 单次加锁完成「内存态更新 + 持久化」，避免同一临界区被拆成两次加锁
    let mut bookings = s.bookings.lock();
    bookings.push(booking.clone());
    save_experts_bookings(&bookings);

    ok(json!(booking))
}

// =====================================================================
// 4. PUT /experts/bookings/{id}/cancel — 预约取消
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
        save_experts_bookings(&bookings);
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
// 路由装配
// =====================================================================

/// 构建专家广场扩展域路由
///
/// `shared` 为专家联盟全域共享状态；收藏集合复用其中的 `favorites`，
/// 专家名校验复用其中的 `registry`。
pub fn build_experts_ext_router(shared: Arc<ExpertsSharedState>) -> Router {
    let state = Arc::new(ExpertsState::new(shared));
    Router::new()
        .route("/api/experts/bookings/mine", get(my_bookings))
        .route("/api/experts/:id/favorite", post(toggle_expert_favorite))
        .route("/api/experts/bookings", post(create_booking))
        .route("/api/experts/bookings/:id/cancel", put(cancel_booking))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experts_common::*;
    use mox_audit::{AuditContext, MultiSink, NoopSink};
    use std::collections::{HashMap, HashSet};

    /// 构造无 IO 的全域共享状态（审计走 NoopSink，不落盘）
    fn test_shared(experts: Vec<ExpertDescriptor>) -> Arc<ExpertsSharedState> {
        let mut registry = HashMap::new();
        for e in experts {
            registry.insert(e.id.clone(), e);
        }
        Arc::new(ExpertsSharedState {
            registry: Arc::new(Mutex::new(registry)),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            dispatcher_config: Arc::new(Mutex::new(DispatcherConfig::default())),
            dispatch_records: Arc::new(Mutex::new(Vec::new())),
            graph: Arc::new(Mutex::new(ExpertGraph::default())),
            plans: Arc::new(Mutex::new(HashMap::new())),
            orchestration_history: Arc::new(Mutex::new(Vec::new())),
            favorites: Arc::new(Mutex::new(HashSet::new())),
            audit: Arc::new(
                AuditContext::new(Arc::new(MultiSink::new().with_sink(Box::new(NoopSink)))),
            ),
        })
    }

    #[test]
    fn test_resolve_expert_name() {
        let shared = test_shared(vec![ExpertDescriptor::minimal(
            "exp-1".into(),
            "架构师·玄枢".into(),
        )]);
        assert_eq!(resolve_expert_name(&shared, "exp-1").unwrap(), "架构师·玄枢");
        assert!(resolve_expert_name(&shared, "exp-not-exist")
            .unwrap_err()
            .contains("not found"));
    }

    #[test]
    fn test_resolve_expert_name_rejects_disabled() {
        let mut disabled = ExpertDescriptor::minimal("exp-2".into(), "已禁用专家".into());
        disabled.enabled = false;
        let shared = test_shared(vec![disabled]);
        assert!(resolve_expert_name(&shared, "exp-2")
            .unwrap_err()
            .contains("disabled"));
    }

    /// 回归：收藏必须落在全域共享态，而不是本模块私有集合
    ///
    /// 直接构造 `ExpertsState`（不经 `new()`），避免单元测试触碰真实 SQLite。
    #[tokio::test]
    async fn test_toggle_favorite_writes_shared_state() {
        let shared = test_shared(vec![ExpertDescriptor::minimal("exp-1".into(), "甲".into())]);
        let s = Arc::new(ExpertsState {
            bookings: Arc::new(Mutex::new(Vec::new())),
            shared: shared.clone(),
        });
        assert!(s.shared.favorites.lock().is_empty());

        toggle_expert_favorite(Path("exp-1".to_string()), State(s.clone())).await;
        assert!(s.shared.favorites.lock().contains("exp-1"));

        // 再次调用为取消
        toggle_expert_favorite(Path("exp-1".to_string()), State(s.clone())).await;
        assert!(s.shared.favorites.lock().is_empty());
    }
}

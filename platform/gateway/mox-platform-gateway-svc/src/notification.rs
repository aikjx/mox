// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 通知中心域（Notification）HTTP 路由
//!
//! 提供通知列表分页查询、单条已读标记、全部已读标记等通知中心能力。
//! 通知数据使用 JSON 文件持久化（data/notifications.json），启动时加载，变更时写回。
//!
//! 路径：`/notifications` · `/notifications/:id/read` · `/notifications/read-all`

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, put},
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use mox_api_protocol::{ApiResponse, api_ok, api_error};

// =====================================================================
// JSON 持久化（data/notifications.json）
// =====================================================================

const NOTIFICATIONS_PATH: &str = "data/notifications.json";

fn load_notifications() -> Vec<Notification> {
    match std::fs::read_to_string(NOTIFICATIONS_PATH) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_notifications(notifications: &[Notification]) {
    if let Some(parent) = std::path::Path::new(NOTIFICATIONS_PATH).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json_str) = serde_json::to_string_pretty(notifications) {
        let _ = std::fs::write(NOTIFICATIONS_PATH, json_str);
    }
}

// =====================================================================
// 共享状态
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Notification {
    id: String,
    title: String,
    content: String,
    #[serde(rename = "type")]
    notification_type: String,
    read: bool,
    created_at: i64,
}

#[derive(Clone)]
struct NotificationState {
    notifications: Arc<Mutex<Vec<Notification>>>,
}

impl NotificationState {
    fn new() -> Self {
        Self { notifications: Arc::new(Mutex::new(load_notifications())) }
    }
}

fn ok(data: Value) -> ApiResponse<Value> {
    api_ok(data)
}

// =====================================================================
// 1. GET /notifications — 通知列表（分页）
// =====================================================================

#[derive(Debug, Deserialize)]
struct NotificationListQuery {
    page: Option<usize>,
    page_size: Option<usize>,
    unread_only: Option<bool>,
}

async fn list_notifications(
    State(s): State<Arc<NotificationState>>,
    Query(q): Query<NotificationListQuery>,
) -> ApiResponse<Value> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    let unread_only = q.unread_only.unwrap_or(false);

    let all = s.notifications.lock().clone();
    let filtered: Vec<&Notification> = if unread_only {
        all.iter().filter(|n| !n.read).collect()
    } else {
        all.iter().collect()
    };

    let total = filtered.len();
    let start = (page - 1) * page_size;
    let items: Vec<&Notification> = filtered.iter().skip(start).take(page_size).copied().collect();

    ok(json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))
}

// =====================================================================
// 2. PUT /notifications/{id}/read — 标记单条已读
// =====================================================================

async fn mark_notification_read(
    Path(id): Path<String>,
    State(s): State<Arc<NotificationState>>,
) -> ApiResponse<Value> {
    let mut notifications = s.notifications.lock();
    if let Some(idx) = notifications.iter().position(|n| n.id == id) {
        notifications[idx].read = true;
        save_notifications(&notifications);
        return ok(json!({
            "id": id,
            "read": true,
        }));
    }
    api_error(404, format!("notification not found: {id}"))
}

// =====================================================================
// 3. PUT /notifications/read-all — 全部已读
// =====================================================================

async fn mark_all_notifications_read(
    State(s): State<Arc<NotificationState>>,
) -> ApiResponse<Value> {
    let mut notifications = s.notifications.lock();
    let mut updated_count: u64 = 0;
    for n in notifications.iter_mut() {
        if !n.read {
            n.read = true;
            updated_count += 1;
        }
    }
    if updated_count > 0 {
        save_notifications(&notifications);
    }
    ok(json!({
        "updated_count": updated_count,
    }))
}

// =====================================================================
// 路由装配
// =====================================================================

pub fn build_notification_router() -> Router {
    let state = Arc::new(NotificationState::new());
    Router::new()
        .route("/notifications", get(list_notifications))
        .route("/notifications/:id/read", put(mark_notification_read))
        .route("/notifications/read-all", put(mark_all_notifications_read))
        .with_state(state)
}

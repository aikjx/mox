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

fn load_notifications() -> Vec<Notification> {
    crate::store_json::try_migrate_json::<Notification>("notification.notifications", "data/notifications.json")
}

fn save_notifications(notifications: &[Notification]) {
    if let Err(e) = crate::store_json::save_collection("notification.notifications", notifications) {
        eprintln!("[notification] 通知持久化失败: {}", e);
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
pub struct NotificationState {
    notifications: Arc<Mutex<Vec<Notification>>>,
}

impl NotificationState {
    pub fn new() -> Self {
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
// 4. GET /notifications/unread-count — 未读通知数（真实数据）
// =====================================================================
//
// 归一化（RC-6/RC-7）：此端点原由 workspace.rs 以硬编码全零 stub 提供，
// 与通知域真实数据源（data/notifications.json）脱节，导致前端未读数恒为 0。
// 现归位到通知域，直接统计 NotificationState，消除 stub 与双源不一致。

async fn unread_count(
    State(s): State<Arc<NotificationState>>,
) -> ApiResponse<Value> {
    let all = s.notifications.lock().clone();
    let unread: Vec<&Notification> = all.iter().filter(|n| !n.read).collect();

    // 按类型聚合（保持与旧 stub 相同的 by_type 键集，前端零改动）
    let mut by_type: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    for k in ["task_assigned", "mention", "comment", "system", "alert"] {
        by_type.insert(k.to_string(), 0);
    }
    for n in unread.iter() {
        *by_type.entry(n.notification_type.clone()).or_insert(0) += 1;
    }

    // 最新一条未读通知（按 created_at 降序）
    let latest = unread
        .iter()
        .max_by_key(|n| n.created_at)
        .map(|n| {
            json!({
                "id": n.id,
                "title": n.title,
                "type": n.notification_type,
                "created_at": n.created_at,
            })
        });

    ok(json!({
        "total": unread.len(),
        "by_type": by_type,
        "latest_notification": latest,
    }))
}

// =====================================================================
// 路由装配
// =====================================================================

pub fn build_notification_router(state: Arc<NotificationState>) -> Router {
    Router::new()
        .route("/api/notifications", get(list_notifications))
        .route("/api/notifications/unread-count", get(unread_count))
        .route("/api/notifications/:id/read", put(mark_notification_read))
        .route("/api/notifications/read-all", put(mark_all_notifications_read))
        .with_state(state)
}

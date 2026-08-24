//! HTTP / WebSocket 运行时（接入层 + 鉴权中间件 + 实时通信 + 企业级加固）
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::{
    extract::{Extension, Path, Query, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tower_http::cors::CorsLayer;

use crate::error::*;
use crate::model::*;
use crate::orchestrator::XuanjiSystem;
use crate::rbac::{Permission, Role, RoleBinding, Scope};

/// 服务版本（来自 Cargo.toml），用于健康检查与可观测性
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 已认证用户（由鉴权中间件注入）
#[derive(Clone)]
pub struct AuthUser(pub String);

// ---------------- 请求体 ----------------
#[derive(Deserialize)]
pub struct InviteReq {
    xuanji_id: String,
    name: String,
    email: String,
    title: String,
    expertise: Vec<String>,
    tier: Option<String>,
}
#[derive(Deserialize)]
pub struct CreateTaskReq {
    xuanji_id: String,
    title: String,
    description: String,
    priority: Option<String>,
}
#[derive(Deserialize)]
pub struct AssignReq {
    assignees: Vec<String>,
}
#[derive(Deserialize)]
pub struct TransitionReq {
    to: String,
}
#[derive(Deserialize)]
pub struct CommentReq {
    body: String,
}
#[derive(Deserialize)]
pub struct MsgReq {
    body: String,
}
#[derive(Deserialize)]
pub struct CreateChannelReq {
    xuanji_id: String,
    kind: String, // "xuanji" | "task:<id>" | "direct"
    name: String,
    members: Vec<String>,
}

fn parse_tier(s: &Option<String>) -> Tier {
    match s.as_deref() {
        Some("Senior") => Tier::Senior,
        Some("Lead") => Tier::Lead,
        Some("Principal") => Tier::Principal,
        _ => Tier::Associate,
    }
}
fn parse_priority(s: &Option<String>) -> Priority {
    match s.as_deref() {
        Some("High") => Priority::High,
        Some("Critical") => Priority::Critical,
        Some("Low") => Priority::Low,
        _ => Priority::Medium,
    }
}
fn parse_status(s: &str) -> Result<TaskStatus> {
    match s {
        "Draft" => Ok(TaskStatus::Draft),
        "Assigned" => Ok(TaskStatus::Assigned),
        "InProgress" => Ok(TaskStatus::InProgress),
        "InReview" => Ok(TaskStatus::InReview),
        "Done" => Ok(TaskStatus::Done),
        "Cancelled" => Ok(TaskStatus::Cancelled),
        _ => Err(AppError::BadRequest(format!("未知状态 {s}"))),
    }
}

// ---------------- 企业级中间件 ----------------
/// 限流中间件（安全防护 I-04）：以「令牌 / 匿名」为键，窗口内超额即拒绝（429）
async fn rate_limit_mw(State(sys): State<Arc<XuanjiSystem>>, req: Request, next: Next) -> Response {
    let key = extract_token_key(&req).unwrap_or_else(|| "anonymous".to_string());
    if sys.ratelimiter.check(&key) {
        next.run(req).await
    } else {
        (StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded").into_response()
    }
}

/// 可观测性中间件（I-04）：统计请求数、延迟与错误数
async fn metrics_mw(State(sys): State<Arc<XuanjiSystem>>, req: Request, next: Next) -> Response {
    let start = Instant::now();
    sys.metrics.inc_requests();
    let resp = next.run(req).await;
    let ms = start.elapsed().as_millis() as u64;
    sys.metrics.add_latency_ms(ms);
    if resp.status().is_client_error() || resp.status().is_server_error() {
        sys.metrics.inc_errors();
    }
    resp
}

/// 从请求中提取限流键：优先用认证令牌，其次用 X-Forwarded-For / 远端地址
fn extract_token_key(req: &Request) -> Option<String> {
    if let Some(t) = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        return Some(format!("tok:{}", t));
    }
    if let Some(t) = req
        .headers()
        .get("x-auth-token")
        .and_then(|v| v.to_str().ok())
    {
        return Some(format!("tok:{}", t));
    }
    req.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| format!("ip:{}", s.split(',').next().unwrap_or(s)))
}

// ---------------- 鉴权中间件 ----------------
async fn auth_mw(State(sys): State<Arc<XuanjiSystem>>, mut req: Request, next: Next) -> Response {
    let token = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .or_else(|| {
            req.headers()
                .get("x-auth-token")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        });
    let token = match token {
        Some(t) => t,
        None => return (StatusCode::UNAUTHORIZED, "missing token").into_response(),
    };
    match sys.store.member_by_token(&token).await {
        Some(mid) => {
            req.extensions_mut().insert(AuthUser(mid));
            next.run(req).await
        }
        None => (StatusCode::UNAUTHORIZED, "invalid token").into_response(),
    }
}

// ---------------- 路由处理器 ----------------
async fn me(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
) -> Result<Json<Member>> {
    Ok(Json(sys.member.get(&id).await?))
}

async fn list_members(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Member>>> {
    let xuanji_id = q
        .get("xuanji_id")
        .cloned()
        .ok_or_else(|| AppError::BadRequest("缺少 xuanji_id".into()))?;
    sys.require(
        &id,
        Permission::AuditView,
        &crate::rbac::ResourceCtx {
            xuanji_id: xuanji_id.clone(),
            task: None,
        },
    )
    .await?;
    Ok(Json(sys.member.list(&xuanji_id).await))
}

async fn invite(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Json(req): Json<InviteReq>,
) -> Result<Json<Member>> {
    let m = sys
        .invite_member(
            &id,
            &InviteInput {
                xuanji_id: req.xuanji_id.clone(),
                name: req.name.clone(),
                email: req.email.clone(),
                title: req.title.clone(),
                expertise: req.expertise.clone(),
                tier: parse_tier(&req.tier),
            },
        )
        .await?;
    Ok(Json(m))
}

async fn create_task(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Json(req): Json<CreateTaskReq>,
) -> Result<Json<Task>> {
    let t = sys
        .create_task(
            &id,
            &req.xuanji_id,
            &req.title,
            &req.description,
            parse_priority(&req.priority),
        )
        .await?;
    Ok(Json(t))
}

async fn list_tasks(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Task>>> {
    let xuanji_id = q
        .get("xuanji_id")
        .cloned()
        .ok_or_else(|| AppError::BadRequest("缺少 xuanji_id".into()))?;
    sys.require(
        &id,
        Permission::TaskViewAll,
        &crate::rbac::ResourceCtx {
            xuanji_id: xuanji_id.clone(),
            task: None,
        },
    )
    .await?;
    Ok(Json(sys.task.list(&xuanji_id).await))
}

async fn assign(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Path(task_id): Path<String>,
    Json(req): Json<AssignReq>,
) -> Result<Json<Task>> {
    let t = sys.assign_task(&id, &task_id, req.assignees).await?;
    Ok(Json(t))
}

async fn transition(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Path(task_id): Path<String>,
    Json(req): Json<TransitionReq>,
) -> Result<Json<Task>> {
    let to = parse_status(&req.to)?;
    let t = sys.transition_task(&id, &task_id, to).await?;
    Ok(Json(t))
}

async fn comment(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Path(task_id): Path<String>,
    Json(req): Json<CommentReq>,
) -> Result<Json<Message>> {
    let m = sys.comment_task(&id, &task_id, &req.body).await?;
    Ok(Json(m))
}

async fn watch(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Path(task_id): Path<String>,
) -> Result<Json<Task>> {
    let t = sys.watch_task(&id, &task_id).await?;
    Ok(Json(t))
}

async fn create_channel(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Json(req): Json<CreateChannelReq>,
) -> Result<Json<Channel>> {
    let kind = match req.kind.as_str() {
        "xuanji" => ChannelKind::Xuanji,
        s if s.starts_with("task:") => ChannelKind::Task(s["task:".len()..].to_string()),
        "direct" | "dm" => ChannelKind::Direct(req.members.clone()),
        _ => return Err(AppError::BadRequest("未知频道类型".into())),
    };
    sys.require(
        &id,
        Permission::CommSendXuanji,
        &crate::rbac::ResourceCtx {
            xuanji_id: req.xuanji_id.clone(),
            task: None,
        },
    )
    .await?;
    let ch = sys
        .comm
        .create_channel(&req.xuanji_id, kind, &req.name, req.members)
        .await;
    Ok(Json(ch))
}

async fn list_channels(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Channel>>> {
    let xuanji_id = q
        .get("xuanji_id")
        .cloned()
        .ok_or_else(|| AppError::BadRequest("缺少 xuanji_id".into()))?;
    sys.require(
        &id,
        Permission::AuditView,
        &crate::rbac::ResourceCtx {
            xuanji_id: xuanji_id.clone(),
            task: None,
        },
    )
    .await?;
    Ok(Json(sys.store.list_channels(&xuanji_id).await))
}

async fn send_msg(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Path(channel_id): Path<String>,
    Json(req): Json<MsgReq>,
) -> Result<Json<Message>> {
    let m = sys
        .send_channel_message(&id, &channel_id, &req.body)
        .await?;
    Ok(Json(m))
}

async fn list_messages(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<Message>>> {
    // 频道成员或审计员可见
    let ch = sys
        .store
        .get_channel(&channel_id)
        .await
        .ok_or_else(|| AppError::NotFound("频道不存在".into()))?;
    let is_member = ch.members.iter().any(|m| m == &id);
    if !is_member {
        sys.require(
            &id,
            Permission::AuditView,
            &crate::rbac::ResourceCtx {
                xuanji_id: ch.xuanji_id.clone(),
                task: None,
            },
        )
        .await?;
    }
    Ok(Json(sys.comm.list_messages(&channel_id).await))
}

async fn my_notifications(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
) -> Result<Json<Vec<Notification>>> {
    sys.require(
        &id,
        Permission::AuditView,
        &crate::rbac::ResourceCtx {
            xuanji_id: String::new(),
            task: None,
        },
    )
    .await?;
    Ok(Json(sys.comm.list_notifications(&id).await))
}

async fn read_notification(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Path(ntf_id): Path<String>,
) -> Result<Json<()>> {
    sys.comm.mark_read(&ntf_id, &id).await?;
    Ok(Json(()))
}

async fn grant_role(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Json(req): Json<GrantReq>,
) -> Result<Json<Vec<RoleBinding>>> {
    sys.require(
        &id,
        Permission::MemberManage,
        &crate::rbac::ResourceCtx {
            xuanji_id: req.xuanji_id.clone(),
            task: None,
        },
    )
    .await?;
    let role = match req.role.as_str() {
        "XuanjiAdmin" => Role::XuanjiAdmin,
        "Coordinator" => Role::Coordinator,
        "Expert" => Role::Expert,
        "Member" => Role::Member,
        "Auditor" => Role::Auditor,
        _ => return Err(AppError::BadRequest("未知角色".into())),
    };
    let scope = match &req.scope {
        Some(s) if s.starts_with("task:") => Scope::Task(s["task:".len()..].to_string()),
        Some(s) if s.starts_with("xuanji:") => Scope::Xuanji(s["xuanji:".len()..].to_string()),
        _ => Scope::Global,
    };
    sys.perm
        .assign_role(RoleBinding {
            member_id: req.member_id.clone(),
            role,
            scope,
        })
        .await;
    Ok(Json(sys.perm.bindings_of(&req.member_id).await))
}

#[derive(Deserialize)]
pub struct GrantReq {
    member_id: String,
    role: String,
    xuanji_id: String,
    scope: Option<String>,
}

/// 审计流查询（BR-18）：需 `audit:view` 权限，支持按动作类别过滤
async fn list_audit(
    Extension(AuthUser(id)): Extension<AuthUser>,
    State(sys): State<Arc<XuanjiSystem>>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<Vec<AuditRecord>>> {
    let xuanji_id = q
        .get("xuanji_id")
        .cloned()
        .ok_or_else(|| AppError::BadRequest("缺少 xuanji_id".into()))?;
    let mut records = sys.list_audit(&id, &xuanji_id).await?;
    if let Some(action) = q.get("action") {
        records.retain(|r| r.action.as_str() == action);
    }
    Ok(Json(records))
}

async fn health(State(sys): State<Arc<XuanjiSystem>>) -> Response {
    (
        StatusCode::OK,
        serde_json::json!({
            "status": "ok",
            "version": VERSION,
            "persistent": sys.store.is_persistent(),
        })
        .to_string(),
    )
        .into_response()
}

/// Prometheus 指标导出端点（I-04）：实时聚合进程内指标
async fn metrics_handler(State(sys): State<Arc<XuanjiSystem>>) -> Response {
    // 动态刷新「活跃成员」瞬时量
    let active = {
        let s = sys.store.state.read().await;
        s.members
            .values()
            .filter(|m| m.status == MemberStatus::Active)
            .count() as u64
    };
    sys.metrics.set_active_members(active);
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4")],
        sys.metrics.render_prometheus(),
    )
        .into_response()
}

// ---------------- WebSocket 实时通知 ----------------
async fn ws_handler(
    State(sys): State<Arc<XuanjiSystem>>,
    ws: WebSocketUpgrade,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let token = q.get("token").cloned().unwrap_or_default();
    let member_id = match sys.store.member_by_token(&token).await {
        Some(m) => m,
        None => return (StatusCode::UNAUTHORIZED, "invalid token").into_response(),
    };
    ws.on_upgrade(move |socket| handle_socket(sys, socket, member_id))
}

async fn handle_socket(sys: Arc<XuanjiSystem>, socket: WebSocket, member_id: String) {
    let (mut sender, mut receiver) = socket.split();
    // 初始推送：未读通知
    let notes = sys.comm.list_notifications(&member_id).await;
    let payload = serde_json::json!({ "type": "init", "notifications": notes }).to_string();
    let _ = sender.send(WsMessage::Text(payload)).await;
    let mut rx = sys.bus.subscribe();
    let member_clone = member_id.clone();
    let forward = tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            if ev.interested_members().contains(&member_clone) {
                if let Ok(payload) = serde_json::to_string(&ev) {
                    if sender.send(WsMessage::Text(payload)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });
    // 保持连接：读取客户端消息（目前忽略，仅维持心跳）
    while let Some(Ok(msg)) = receiver.next().await {
        if let WsMessage::Close(_) = msg {
            break;
        }
    }
    forward.abort();
}

// ---------------- 应用装配 ----------------
pub fn app(sys: Arc<XuanjiSystem>) -> Router {
    // CORS：仅允许配置的来源列表；未配置则完全禁止跨域（默认安全）
    let cors = if sys.config.cors_enabled() {
        let origins = sys
            .config
            .cors_allowed_origins
            .iter()
            .filter_map(|o| o.parse::<axum::http::HeaderValue>().ok())
            .collect::<Vec<_>>();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
            ])
    } else {
        // 未配置允许来源时，默认拒绝一切跨域请求（默认安全）
        CorsLayer::new()
    };

    let protected = Router::new()
        .route("/api/me", get(me))
        .route("/api/members", get(list_members))
        .route("/api/members/invite", post(invite))
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/:id/assign", post(assign))
        .route("/api/tasks/:id/transition", post(transition))
        .route("/api/tasks/:id/comments", post(comment))
        .route("/api/tasks/:id/watch", post(watch))
        .route("/api/channels", get(list_channels).post(create_channel))
        .route(
            "/api/channels/:id/messages",
            get(list_messages).post(send_msg),
        )
        .route("/api/notifications", get(my_notifications))
        .route("/api/notifications/:id/read", post(read_notification))
        .route("/api/roles/grant", post(grant_role))
        .route("/api/audit", get(list_audit))
        .layer(middleware::from_fn_with_state(sys.clone(), auth_mw));

    Router::new()
        .route("/api/health", get(health))
        .route("/api/metrics", get(metrics_handler))
        .route("/api/ws", get(ws_handler))
        .merge(protected)
        // 限流与可观测性作用于全站（含健康/指标探测）
        .layer(middleware::from_fn_with_state(sys.clone(), rate_limit_mw))
        .layer(middleware::from_fn_with_state(sys.clone(), metrics_mw))
        .layer(cors)
        .with_state(sys)
}

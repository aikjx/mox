// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 人机协同（HITL - Human-in-the-Loop）Websocket Handler
//!
//! 提供实时人工审批能力：
//!
//! - **HITLRequest** — 客户端 → 服务端的请求 / 指令
//!     - `subscribe`      订阅 HITL 事件（带可选过滤条件）
//!     - `unsubscribe`    取消订阅
//!     - `action`         审批动作（APPROVE / DENY / MODIFY_APPROVE）
//!     - `list_pending`  查询当前待审批列表
//! - **HITLResponse** — 服务端 → 客户端的响应 / 事件推送
//!     - `connected`      连接成功
//!     - `subscribed`    订阅确认
//!     - `hitl_event`     新的待审批事项广播
//!     - `action_result`  审批动作的执行结果
//!     - `pending_list`   待审批列表响应
//!     - `error`          通用错误
//!
//! ## 三种审批动作
//!
//! | 动作 | 说明 |
//! |------|------|
//! | `APPROVE`          | 批准（直接放行） |
//! | `DENY`             | 拒绝（直接驳回） |
//! | `MODIFY_APPROVE`   | 修改后批准（需带 `modified_payload`，合并到原 payload 后放行） |

use axum::extract::{ws::Message, ws::WebSocket, ws::WebSocketUpgrade, Extension, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::rbac_middleware::Principal;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, Mutex, RwLock};
use mox_api_protocol::{ApiResponse, api_ok, api_error, api_ok_empty};

// ============================================================================
// HITL 共享状态
// ============================================================================

/// HITL 全局状态（挂载到 AppState）
#[derive(Clone)]
pub struct HitlState {
    /// 待审批事项（按 id 索引）
    pub pending: Arc<RwLock<HashMap<String, HitlEvent>>>,
    /// 已处理（批准/拒绝/修改批准）的历史
    pub history: Arc<Mutex<Vec<HitlDecisionRecord>>>,
    /// WebSocket 广播通道（新的待审批事项）
    pub event_broadcast: broadcast::Sender<HitlEvent>,
    /// 审批结果广播通道
    pub decision_broadcast: broadcast::Sender<HitlDecisionRecord>,
}

impl Default for HitlState {
    fn default() -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let (decision_tx, _) = broadcast::channel(256);
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            event_broadcast: event_tx,
            decision_broadcast: decision_tx,
        }
    }
}

impl HitlState {
    /// 注册一条新的待审批事项并广播通知所有订阅客户端
    #[allow(dead_code)]
    pub async fn submit_event(&self, event: HitlEvent) {
        let mut pending = self.pending.write().await;
        pending.insert(event.id.clone(), event.clone());
        drop(pending);
        let _ = self.event_broadcast.send(event);
    }

    /// 处理一次审批动作（APPROVE / DENY / MODIFY_APPROVE）
    pub async fn handle_action(
        &self,
        event_id: &str,
        action: HitlAction,
        actor: &str,
        comment: Option<String>,
        modified_payload: Option<Value>,
    ) -> Result<HitlDecisionRecord, String> {
        let mut pending = self.pending.write().await;
        let event = pending
            .get(event_id)
            .cloned()
            .ok_or_else(|| format!("event_id `{event_id}` 不存在或已处理"))?;

        let mut final_payload = event.payload.clone();

        if let Some(ref patch) = modified_payload {
            // MODIFY_APPROVE 合并 patch 到原 payload（浅合并）
            if let (Some(orig), Some(patch)) = (final_payload.as_object_mut(), patch.as_object()) {
                for (k, v) in patch {
                    orig.insert(k.clone(), v.clone());
                }
            }
            final_payload = patch.clone();
        }

        let record = HitlDecisionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            event_id: event.id.clone(),
            flow_id: event.flow_id.clone(),
            action,
            actor: actor.to_string(),
            comment,
            modified_payload,
            final_payload,
            ts: unix_ts(),
        };

        // 移出待审批列表
        pending.remove(event_id);
        drop(pending);

        // 写入历史
        {
            let mut history = self.history.lock().await;
            history.push(record.clone());
        }

        // 广播结果
        let _ = self.decision_broadcast.send(record.clone());

        Ok(record)
    }
}

// ============================================================================
// 数据模型
// ============================================================================

/// HITL 审批动作
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HitlAction {
    Approve,
    Deny,
    #[serde(rename = "MODIFY_APPROVE")]
    ModifyApprove,
}

impl std::fmt::Display for HitlAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HitlAction::Approve => write!(f, "APPROVE"),
            HitlAction::Deny => write!(f, "DENY"),
            HitlAction::ModifyApprove => write!(f, "MODIFY_APPROVE"),
        }
    }
}

/// 待审批事项（由主流程触发进入 HITL 审批队列）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HitlEvent {
    pub id: String,
    pub flow_id: String,
    pub flow_name: String,
    pub kind: String,
    pub description: String,
    pub payload: Value,
    pub requester: String,
    pub ts: i64,
}

/// 审批动作记录（历史）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HitlDecisionRecord {
    pub id: String,
    pub event_id: String,
    pub flow_id: String,
    pub action: HitlAction,
    pub actor: String,
    pub comment: Option<String>,
    pub modified_payload: Option<Value>,
    pub final_payload: Value,
    pub ts: i64,
}

/// 客户端 → 服务端请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HitlRequest {
    /// 订阅 HITL 事件（可选过滤条件：flow_id / kind）
    Subscribe { filters: Option<HitlFilter> },
    /// 取消订阅
    Unsubscribe,
    /// 发送审批动作
    Action {
        event_id: String,
        action: HitlAction,
        actor: String,
        comment: Option<String>,
        /// MODIFY_APPROVE 必填；其他动作可忽略
        modified_payload: Option<Value>,
    },
    /// 查询当前待审批列表
    ListPending { flow_id: Option<String> },
}

/// 订阅过滤条件
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HitlFilter {
    pub flow_id: Option<String>,
    pub kind: Option<String>,
}

/// 服务端 → 客户端响应 / 事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HitlResponse {
    /// 连接成功
    Connected { timestamp: i64, message: String },
    /// 订阅确认
    Subscribed {
        filters: Option<HitlFilter>,
        pending_count: usize,
    },
    /// 新的待审批事项推送
    HitlEvent { data: HitlEvent },
    /// 审批动作结果
    ActionResult {
        success: bool,
        record: Option<HitlDecisionRecord>,
        error: Option<String>,
    },
    /// 待审批列表响应
    PendingList { items: Vec<HitlEvent> },
    /// 通用错误
    Error { code: String, message: String },
}

/// UNIX 时间戳（秒）
pub fn unix_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ============================================================================
// WebSocket Handler
// ============================================================================

/// GET /ws/hitl
///
/// WebSocket 端点，支持：
/// - 客户端订阅待审批事件（`subscribe` 消息，可带 `flow_id` / `kind` 过滤）
/// - 客户端发送审批动作（`action` 消息：APPROVE / DENY / MODIFY_APPROVE）
/// - 客户端查询待审批列表（`list_pending`）
/// - 服务端主动广播：`hitl_event`（新事项）与 `action_result`（动作完成）
///
/// 连接建立后立即推送 `connected` 消息。
pub async fn hitl_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<HitlState>>,
    Extension(principal): Extension<Principal>,
) -> impl IntoResponse {
    // 连接已由网关鉴权（/ws/hitl 现要求 Bearer 或查询参数 token）；以「经鉴权主体」作为审批动作的
    // 实际 actor 落痕，不信任客户端自报 actor（原默认 'admin' 可被任意关联者冒充）。
    ws.on_upgrade(move |socket| async move {
        // 以「经鉴权主体」作为审批动作的落痕 actor，不信任客户端自报 actor。
        let actor = principal.token_id.clone();
        // 全双工 WebSocket 直接用 Arc<Mutex> 包裹，写半部被多个广播任务共享。
        let write = std::sync::Arc::new(tokio::sync::Mutex::new(socket));

        // 发送连接成功消息
        {
            let msg = Message::Text(
                serde_json::to_string(&HitlResponse::Connected {
                    timestamp: unix_ts(),
                    message: "HITL WebSocket 已连接".to_string(),
                })
                .unwrap_or_else(|_| {
                    r#"{"type":"error","code":"SERDE","message":"serialize failed"}"#.to_string()
                }),
            );
            let mut w = write.lock().await;
            let _ = w.send(msg).await;
            drop(w);
        }

        // 广播通道订阅
        let mut event_rx = state.event_broadcast.subscribe();
        let mut decision_rx = state.decision_broadcast.subscribe();

        // 最近一次订阅过滤条件（用于服务端过滤广播）
        let subscribed: Arc<Mutex<Option<HitlFilter>>> = Arc::new(Mutex::new(None));

        // ===== 客户端消息读取任务 =====
        let read_task = {
            let state = state.clone();
            let w = write.clone();
            let subscribed = subscribed.clone();
            let actor = actor.clone();
            tokio::spawn(async move {
                loop {
                    let msg = {
                        let mut s = w.lock().await;
                        s.recv().await
                    };
                    // WebSocket::recv() -> Option<Result<Message, Error>>
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            match serde_json::from_str::<HitlRequest>(text.as_ref()) {
                                Ok(req) => {
                                    handle_client_request(&state, &w, &subscribed, &actor, req).await;
                                }
                                Err(e) => {
                                    write_text(&w, HitlResponse::Error {
                                        code: "BAD_REQUEST".to_string(),
                                        message: format!("无法解析请求: {e}"),
                                    })
                                    .await;
                                }
                            }
                        }
                        Some(Ok(Message::Binary(_))) => {
                            write_text(
                                &w,
                                HitlResponse::Error {
                                    code: "BAD_REQUEST".to_string(),
                                    message: "仅支持 JSON 文本消息".to_string(),
                                },
                            )
                            .await;
                        }
                        Some(Ok(Message::Ping(_) | Message::Pong(_))) => {
                            // 由 axum/tokio 自动处理，忽略
                        }
                        Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    }
                }
            })
        };

        // ===== 广播监听任务 =====
        let broadcast_task = {
            let w = write.clone();
            let subscribed = subscribed.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        result = event_rx.recv() => {
                            match result {
                                Ok(event) => {
                                    let filter = subscribed.lock().await.clone();
                                    if hitl_event_matches(&event, filter.as_ref()) {
                                        write_text(&w, HitlResponse::HitlEvent { data: event }).await;
                                    }
                                }
                                Err(broadcast::error::RecvError::Lagged(n)) => {
                                    tracing::warn!("HITL event broadcast lagged {n} messages");
                                }
                                Err(broadcast::error::RecvError::Closed) => break,
                            }
                        }
                        result = decision_rx.recv() => {
                            match result {
                                Ok(record) => {
                                    write_text(
                                        &w,
                                        HitlResponse::ActionResult {
                                            success: true,
                                            record: Some(record),
                                            error: None,
                                        },
                                    )
                                    .await;
                                }
                                Err(broadcast::error::RecvError::Lagged(n)) => {
                                    tracing::warn!("HITL decision broadcast lagged {n} messages");
                                }
                                Err(broadcast::error::RecvError::Closed) => break,
                            }
                        }
                    }
                }
            })
        };

        // ===== 心跳任务（30s） =====
        let ping_task = {
            let w = write.clone();
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30));
                loop {
                    ticker.tick().await;
                    let mut w = w.lock().await;
                    if w.send(Message::Ping(vec![])).await.is_err() {
                        break;
                    }
                }
            })
        };

        // 任一任务结束则关闭整个连接
        tokio::select! {
            _ = read_task => {}
            _ = broadcast_task => {}
            _ = ping_task => {}
        }
    })
}

// ============================================================================
// 内部辅助
// ============================================================================

/// 将 HITLResponse 序列化并通过 WebSocket 写半部发送
async fn write_text(write: &Arc<Mutex<WebSocket>>, resp: HitlResponse) {
    let text = serde_json::to_string(&resp).unwrap_or_else(|_| {
        r#"{"type":"error","code":"SERDE","message":"serialize failed"}"#.to_string()
    });
    let mut w = write.lock().await;
    let _ = w.send(Message::Text(text)).await;
}

/// 处理来自客户端的单条请求
async fn handle_client_request(
    state: &Arc<HitlState>,
    write: &Arc<Mutex<WebSocket>>,
    subscribed: &Arc<Mutex<Option<HitlFilter>>>,
    actor: &str,
    req: HitlRequest,
) {
    match req {
        HitlRequest::Subscribe { filters } => {
            *subscribed.lock().await = filters.clone();
            let pending = state.pending.read().await;
            let count = pending
                .values()
                .filter(|e| hitl_event_matches(e, filters.as_ref()))
                .count();
            drop(pending);
            write_text(
                write,
                HitlResponse::Subscribed {
                    filters,
                    pending_count: count,
                },
            )
            .await;
        }
        HitlRequest::Unsubscribe => {
            *subscribed.lock().await = None;
            write_text(
                write,
                HitlResponse::Subscribed {
                    filters: None,
                    pending_count: 0,
                },
            )
            .await;
        }
        HitlRequest::Action {
            event_id,
            action,
            actor: _, // 客户端自报 actor 不再采信；落痕 actor 为经鉴权的主体（见 hitl_ws_handler）
            comment,
            modified_payload,
        } => {
            // MODIFY_APPROVE 必须提供 modified_payload
            if action == HitlAction::ModifyApprove && modified_payload.is_none() {
                write_text(
                    write,
                    HitlResponse::ActionResult {
                        success: false,
                        record: None,
                        error: Some("MODIFY_APPROVE 必须提供 modified_payload".to_string()),
                    },
                )
                .await;
                return;
            }

            match state
                .handle_action(&event_id, action, actor, comment, modified_payload)
                .await
            {
                Ok(record) => {
                    write_text(
                        write,
                        HitlResponse::ActionResult {
                            success: true,
                            record: Some(record),
                            error: None,
                        },
                    )
                    .await;
                }
                Err(e) => {
                    write_text(
                        write,
                        HitlResponse::ActionResult {
                            success: false,
                            record: None,
                            error: Some(e),
                        },
                    )
                    .await;
                }
            }
        }
        HitlRequest::ListPending { flow_id } => {
            let pending = state.pending.read().await;
            let items: Vec<HitlEvent> = pending
                .values()
                .filter(|e| flow_id.as_ref().is_none_or(|f| &e.flow_id == f))
                .cloned()
                .collect();
            drop(pending);
            write_text(write, HitlResponse::PendingList { items }).await;
        }
    }
}

/// 判断事件是否匹配订阅过滤条件（过滤条件为 None 时视为全部匹配）
fn hitl_event_matches(event: &HitlEvent, filter: Option<&HitlFilter>) -> bool {
    let Some(f) = filter else {
        return true;
    };
    if f.flow_id.as_ref().is_some_and(|id| id != &event.flow_id) {
        return false;
    }
    if f.kind.as_ref().is_some_and(|k| k != &event.kind) {
        return false;
    }
    true
}

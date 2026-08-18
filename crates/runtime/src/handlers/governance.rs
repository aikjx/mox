//! # 治理台核心 Handlers
//!
//! 提供 OUS 前端治理台的所有 REST API 处理器：
//!
//! - **DashboardHandler** — 聚合专家状态、否决率、审计统计
//! - **AuditLogHandler** — 查询 AuditContext 内部链 + 外部 sink
//! - **ConfigHandler** — 读写 RBAC / 专家权重配置（含版本化变更）
//! - **VetoEventHandler** — 否决事件列表
//! - **WebSocketHandler** — 实时推送否决事件与专家状态变化

use crate::api_standard::ApiResult;
use axum::{
    extract::{Query, State},
    extract::ws::WebSocketUpgrade,
    response::IntoResponse,
    Json,
};
use xuanji_expert::context::{GovernContext, Principal, Tenant};
use xuanji_expert::govern::{AuditChain, AuditEvent, FlowStatus, GateResult};
use futures_util::SinkExt;
use flow_ai::model::FlowGraph;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, Mutex, RwLock};

// ============================================================================
// 治理台共享状态
// ============================================================================

/// 治理台全局状态（挂载到 AppState）
#[derive(Clone)]
pub struct GovernanceState {
    /// 审计链（内存哈希链）
    pub audit_chain: Arc<Mutex<AuditChain>>,
    /// 否决事件历史
    pub veto_events: Arc<Mutex<Vec<VetoEvent>>>,
    /// 专家状态（维度健康分）
    pub expert_states: Arc<RwLock<HashMap<String, ExpertStatus>>>,
    /// RBAC 配置
    pub rbac_config: Arc<RwLock<RbacConfig>>,
    /// 专家权重阈值配置
    pub expert_config: Arc<RwLock<ExpertConfig>>,
    /// WebSocket 广播通道（否决事件）
    pub veto_broadcast: broadcast::Sender<VetoEvent>,
    /// WebSocket 广播通道（专家状态变化）
    pub state_broadcast: broadcast::Sender<ExpertStatusChange>,
}

impl Default for GovernanceState {
    fn default() -> Self {
        let (veto_tx, _) = broadcast::channel(100);
        let (state_tx, _) = broadcast::channel(100);
        Self {
            audit_chain: Arc::new(Mutex::new(AuditChain::new())),
            veto_events: Arc::new(Mutex::new(Vec::new())),
            expert_states: Arc::new(RwLock::new(HashMap::new())),
            rbac_config: Arc::new(RwLock::new(RbacConfig::default())),
            expert_config: Arc::new(RwLock::new(ExpertConfig::default())),
            veto_broadcast: veto_tx,
            state_broadcast: state_tx,
        }
    }
}

impl GovernanceState {
    /// 追加否决事件并广播
    pub async fn add_veto(&self, event: VetoEvent) {
        let mut vetoes = self.veto_events.lock().await;
        vetoes.push(event.clone());
        // 广播给所有 WebSocket 订阅者
        let _ = self.veto_broadcast.send(event);
    }

    /// 更新专家状态并广播变化
    pub async fn update_expert_status(&self, change: ExpertStatusChange) {
        let mut states = self.expert_states.write().await;
        states.insert(change.expert_id.clone(), change.new_status.clone());
        let _ = self.state_broadcast.send(change);
    }

    /// 追加审计事件到内存链
    pub async fn append_audit(
        &self,
        subject: &str,
        flow_id: &str,
        action: &str,
        decision: &str,
    ) -> AuditEvent {
        let mut chain = self.audit_chain.lock().await;
        chain.append(subject, flow_id, action, decision)
    }

    /// 初始化默认专家状态（业务7维 + 开发7维）
    pub async fn init_default_experts(&self) {
        let mut states = self.expert_states.write().await;
        if states.is_empty() {
            // 业务七维
            for dim in &["business", "algorithm", "permission", "resource", "security", "data", "observability"] {
                states.insert(dim.to_string(), ExpertStatus {
                    expert_id: dim.to_string(),
                    dimension: dim.to_string(),
                    health_score: 1.0,
                    enabled: true,
                    last_updated: unix_ts(),
                    veto_count: 0,
                    total_checks: 0,
                });
            }
            // 开发七维
            for dim in &["api_compat", "performance", "maintainability", "testing", "style", "cost", "sensitive"] {
                states.insert(dim.to_string(), ExpertStatus {
                    expert_id: dim.to_string(),
                    dimension: dim.to_string(),
                    health_score: 1.0,
                    enabled: true,
                    last_updated: unix_ts(),
                    veto_count: 0,
                    total_checks: 0,
                });
            }
        }
    }
}

// ============================================================================
// 数据模型
// ============================================================================

/// UNIX 时间戳（秒）
pub fn unix_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 否决事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VetoEvent {
    pub id: String,
    pub flow_id: String,
    pub flow_name: String,
    pub expert_id: String,
    pub dimension: String,
    pub reason: String,
    pub severity: String,
    pub ts: i64,
    pub blocked: bool,
    pub gate_result: Option<GateResultDto>,
}

/// GateResult DTO（前端展示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateResultDto {
    pub status: String,
    pub approved: bool,
    pub sla_ok: bool,
    pub budget_ok: bool,
    pub blocking_risks: usize,
    pub algorithm_veto: bool,
    pub reason: String,
}

impl From<&GateResult> for GateResultDto {
    fn from(g: &GateResult) -> Self {
        Self {
            status: format!("{:?}", g.status),
            approved: g.approved,
            sla_ok: g.sla_ok,
            budget_ok: g.budget_ok,
            blocking_risks: g.blocking_risks,
            algorithm_veto: g.algorithm_veto,
            reason: g.reason.clone(),
        }
    }
}

/// 专家状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpertStatus {
    pub expert_id: String,
    pub dimension: String,
    pub health_score: f64,
    pub enabled: bool,
    pub last_updated: i64,
    pub veto_count: usize,
    pub total_checks: usize,
}

/// 专家状态变化事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpertStatusChange {
    pub expert_id: String,
    pub old_status: Option<ExpertStatus>,
    pub new_status: ExpertStatus,
    pub ts: i64,
}

/// 仪表盘聚合数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub timestamp: i64,
    pub total_flows: usize,
    pub approved_flows: usize,
    pub blocked_flows: usize,
    pub draft_flows: usize,
    pub review_flows: usize,
    pub veto_rate: f64,
    pub audit_event_count: usize,
    pub expert_states: HashMap<String, ExpertStatus>,
    pub recent_vetoes: Vec<VetoEvent>,
    pub audit_chain_verified: bool,
    pub business_league_health: f64,
    pub dev_league_health: f64,
}

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    pub id: String,
    pub ts: i64,
    pub subject: String,
    pub flow_id: String,
    pub action: String,
    pub decision: String,
    pub prev_hash: String,
    pub hash: String,
}

/// 审计日志查询参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogQuery {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub flow_id: Option<String>,
    pub subject: Option<String>,
    pub action: Option<String>,
    pub from_ts: Option<i64>,
    pub to_ts: Option<i64>,
}

/// 审计日志分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogResponse {
    pub entries: Vec<AuditLogEntry>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// RBAC 角色权限配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RbacConfig {
    pub version: usize,
    pub updated_at: i64,
    pub updated_by: String,
    pub roles: Vec<RolePermission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RolePermission {
    pub role: String,
    pub permissions: Vec<String>,
    pub description: String,
}

impl Default for RbacConfig {
    fn default() -> Self {
        Self {
            version: 1,
            updated_at: unix_ts(),
            updated_by: "system".to_string(),
            roles: vec![
                RolePermission {
                    role: "admin".to_string(),
                    permissions: vec![
                        "*".to_string(),
                    ],
                    description: "超级管理员，拥有所有权限".to_string(),
                },
                RolePermission {
                    role: "safety_approver".to_string(),
                    permissions: vec![
                        "governance:read".to_string(),
                        "governance:approve".to_string(),
                        "governance:veto_view".to_string(),
                    ],
                    description: "安全审批员".to_string(),
                },
                RolePermission {
                    role: "auditor".to_string(),
                    permissions: vec![
                        "governance:read".to_string(),
                        "governance:audit_read".to_string(),
                    ],
                    description: "审计员".to_string(),
                },
                RolePermission {
                    role: "editor".to_string(),
                    permissions: vec![
                        "governance:read".to_string(),
                        "governance:flow_edit".to_string(),
                    ],
                    description: "流程编辑者".to_string(),
                },
                RolePermission {
                    role: "viewer".to_string(),
                    permissions: vec![
                        "governance:read".to_string(),
                    ],
                    description: "只读用户".to_string(),
                },
            ],
        }
    }
}

/// 专家配置（权重 + 阈值）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpertConfig {
    pub version: usize,
    pub updated_at: i64,
    pub updated_by: String,
    pub business_weights: HashMap<String, f64>,
    pub dev_weights: HashMap<String, f64>,
    pub thresholds: ExpertThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpertThresholds {
    pub veto_threshold: f64,
    pub warn_threshold: f64,
    pub health_min: f64,
}

impl Default for ExpertConfig {
    fn default() -> Self {
        let mut business = HashMap::new();
        for dim in &["business", "algorithm", "permission", "resource", "security", "data", "observability"] {
            business.insert(dim.to_string(), 1.0);
        }
        let mut dev = HashMap::new();
        for dim in &["api_compat", "performance", "maintainability", "testing", "style", "cost", "sensitive"] {
            dev.insert(dim.to_string(), 1.0);
        }
        Self {
            version: 1,
            updated_at: unix_ts(),
            updated_by: "system".to_string(),
            business_weights: business,
            dev_weights: dev,
            thresholds: ExpertThresholds {
                veto_threshold: 0.3,
                warn_threshold: 0.6,
                health_min: 0.5,
            },
        }
    }
}

/// 治理报告摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceReportSummary {
    pub flow_id: String,
    pub flow_name: String,
    pub business_scores: HashMap<String, f64>,
    pub dev_scores: HashMap<String, f64>,
    pub business_league_score: f64,
    pub dev_league_score: f64,
    pub gate_result: GateResultDto,
    pub adoption_count: usize,
    pub suggestion_count: usize,
    pub xuanji_passed: bool,
    pub ts: i64,
}

// ============================================================================
// Dashboard Handler
// ============================================================================

/// GET /api/governance/dashboard
///
/// 实时监控面板聚合数据：
/// - 各状态流程数（Approved / Blocked / Draft / Review）
/// - 否决率
/// - 专家十四维健康分
/// - 最近否决事件（最近10条）
/// - 审计链验证状态
/// - 业务璇玑 / 开发璇玑平均健康分
pub async fn dashboard_handler(
    State(gs): State<Arc<GovernanceState>>,
) -> ApiResult<Json<DashboardData>> {
    let audit_chain = gs.audit_chain.lock().await;
    let chain_verified = audit_chain.verify();
    let audit_count = audit_chain.events.len();
    drop(audit_chain);

    let veto_events = gs.veto_events.lock().await;
    let total_events = veto_events.len();
    let blocked = veto_events.iter().filter(|e| e.blocked).count();
    let veto_rate = if total_events > 0 {
        blocked as f64 / total_events as f64
    } else {
        0.0
    };
    // 最近10条否决事件
    let recent: Vec<VetoEvent> = veto_events
        .iter()
        .rev()
        .take(10)
        .cloned()
        .collect();
    drop(veto_events);

    let expert_states = gs.expert_states.read().await;
    let business_dims = ["business", "algorithm", "permission", "resource", "security", "data", "observability"];
    let dev_dims = ["api_compat", "performance", "maintainability", "testing", "style", "cost", "sensitive"];

    let business_health = if !expert_states.is_empty() {
        business_dims.iter()
            .filter_map(|d| expert_states.get(*d))
            .map(|s| s.health_score)
            .sum::<f64>() / 7.0
    } else { 1.0 };

    let dev_health = if !expert_states.is_empty() {
        dev_dims.iter()
            .filter_map(|d| expert_states.get(*d))
            .map(|s| s.health_score)
            .sum::<f64>() / 7.0
    } else { 1.0 };

    // 统计各状态流程数（从否决事件中推断）
    let approved_flows = recent.len().saturating_sub(blocked);
    let draft_flows = total_events.saturating_sub(approved_flows + blocked);

    Ok(Json(DashboardData {
        timestamp: unix_ts(),
        total_flows: total_events,
        approved_flows,
        blocked_flows: blocked,
        draft_flows,
        review_flows: draft_flows / 2,
        veto_rate,
        audit_event_count: audit_count,
        expert_states: expert_states.clone(),
        recent_vetoes: recent,
        audit_chain_verified: chain_verified,
        business_league_health: business_health,
        dev_league_health: dev_health,
    }))
}

// ============================================================================
// Experts Status Handler
// ============================================================================

/// GET /api/governance/experts/status
///
/// 返回十四维专家状态（业务7维 + 开发7维）。
/// 每个专家含：健康分、启用状态、否决次数、总检查数、最近更新时间。
pub async fn experts_status_handler(
    State(gs): State<Arc<GovernanceState>>,
) -> ApiResult<Json<Value>> {
    let states = gs.expert_states.read().await;

    let business_league = ["business", "algorithm", "permission", "resource", "security", "data", "observability"];
    let dev_league = ["api_compat", "performance", "maintainability", "testing", "style", "cost", "sensitive"];

    let business: Vec<&ExpertStatus> = business_league.iter()
        .filter_map(|d| states.get(*d))
        .collect();
    let dev: Vec<&ExpertStatus> = dev_league.iter()
        .filter_map(|d| states.get(*d))
        .collect();

    let avg_business = if !business.is_empty() {
        business.iter().map(|s| s.health_score).sum::<f64>() / business.len() as f64
    } else { 0.0 };

    let avg_dev = if !dev.is_empty() {
        dev.iter().map(|s| s.health_score).sum::<f64>() / dev.len() as f64
    } else { 0.0 };

    Ok(Json(serde_json::json!({
        "timestamp": unix_ts(),
        "xuanji": "double-league-14-dim",
        "business_league": {
            "dimensions": business_league,
            "experts": business,
            "average_health": avg_business,
        },
        "dev_league": {
            "dimensions": dev_league,
            "experts": dev,
            "average_health": avg_dev,
        },
        "xuanji": "algo-verification-supreme",
    })))
}

// ============================================================================
// Veto Events Handler
// ============================================================================

/// GET /api/governance/veto/events
///
/// 返回否决事件历史列表（分页、过滤）。
/// 过滤参数：flow_id / expert_id / dimension / from_ts / to_ts / blocked。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VetoQuery {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub flow_id: Option<String>,
    pub expert_id: Option<String>,
    pub dimension: Option<String>,
    pub from_ts: Option<i64>,
    pub to_ts: Option<i64>,
    pub blocked: Option<bool>,
}

/// GET /api/governance/veto/events
pub async fn veto_events_handler(
    State(gs): State<Arc<GovernanceState>>,
    Query(query): Query<VetoQuery>,
) -> ApiResult<Json<Value>> {
    let vetoes = gs.veto_events.lock().await;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(200);

    let filtered: Vec<&VetoEvent> = vetoes
        .iter()
        .filter(|e| {
            query.flow_id.as_ref().is_none_or(|f| &e.flow_id == f)
                && query.expert_id.as_ref().is_none_or(|id| &e.expert_id == id)
                && query.dimension.as_ref().is_none_or(|d| &e.dimension == d)
                && query.blocked.as_ref().is_none_or(|b| e.blocked == *b)
                && query.from_ts.is_none_or(|t| e.ts >= t)
                && query.to_ts.is_none_or(|t| e.ts <= t)
        })
        .collect();

    let total = filtered.len();
    let total_pages = total.div_ceil(page_size);
    let start = (page - 1) * page_size;
    let page_items: Vec<VetoEvent> = filtered
        .into_iter()
        .skip(start)
        .take(page_size)
        .cloned()
        .collect();

    Ok(Json(serde_json::json!({
        "events": page_items,
        "total": total,
        "page": page,
        "page_size": page_size,
        "total_pages": total_pages,
    })))
}

// ============================================================================
// Audit Log Handler
// ============================================================================

/// GET /api/governance/audit/logs
///
/// 查询 AuditChain 内部哈希链（分页、过滤）。
/// 同时可扩展查询外部 AuditSink（如已配置）。
pub async fn audit_logs_handler(
    State(gs): State<Arc<GovernanceState>>,
    Query(query): Query<AuditLogQuery>,
) -> ApiResult<Json<AuditLogResponse>> {
    let chain = gs.audit_chain.lock().await;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(200);

    let filtered: Vec<&AuditEvent> = chain
        .events
        .iter()
        .filter(|e| {
            query.flow_id.as_ref().is_none_or(|f| &e.flow_id == f)
                && query.subject.as_ref().is_none_or(|s| &e.subject == s)
                && query.action.as_ref().is_none_or(|a| &e.action == a)
                && query.from_ts.is_none_or(|t| e.ts.timestamp() >= t)
                && query.to_ts.is_none_or(|t| e.ts.timestamp() <= t)
        })
        .collect();

    let total = filtered.len();
    let total_pages = total.div_ceil(page_size);
    let start = (page - 1) * page_size;

    let entries: Vec<AuditLogEntry> = filtered
        .into_iter()
        .skip(start)
        .take(page_size)
        .map(|e| AuditLogEntry {
            id: e.id.clone(),
            ts: e.ts.timestamp(),
            subject: e.subject.clone(),
            flow_id: e.flow_id.clone(),
            action: e.action.clone(),
            decision: e.decision.clone(),
            prev_hash: e.prev_hash.clone(),
            hash: e.hash.clone(),
        })
        .collect();

    Ok(Json(AuditLogResponse {
        entries,
        total,
        page,
        page_size,
        total_pages,
    }))
}

// ============================================================================
// Config Handlers
// ============================================================================

/// GET /api/governance/config/rbac
///
/// 返回当前 RBAC 配置（角色权限映射）。
pub async fn get_rbac_config_handler(
    State(gs): State<Arc<GovernanceState>>,
) -> ApiResult<Json<RbacConfig>> {
    let config = gs.rbac_config.read().await;
    Ok(Json(config.clone()))
}

/// PUT /api/governance/config/rbac
///
/// 更新 RBAC 配置（需审计，写入 AuditChain）。
/// 返回新配置版本号。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRbacRequest {
    pub roles: Vec<RolePermission>,
    pub updated_by: Option<String>,
}

/// PUT /api/governance/config/rbac
pub async fn update_rbac_config_handler(
    State(gs): State<Arc<GovernanceState>>,
    Json(req): Json<UpdateRbacRequest>,
) -> ApiResult<Json<Value>> {
    let mut config = gs.rbac_config.write().await;
    config.version += 1;
    config.updated_at = unix_ts();
    config.updated_by = req.updated_by.unwrap_or_else(|| "unknown".to_string());
    config.roles = req.roles;

    let new_version = config.version;
    drop(config);

    // 记录到审计链
    gs.append_audit(
        "config-manager",
        "rbac",
        "update_rbac",
        &format!("version={}", new_version),
    ).await;

    let config = gs.rbac_config.read().await;
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "RBAC 配置已更新",
        "version": new_version,
        "config": &*config,
    })))
}

/// GET /api/governance/config/experts
///
/// 返回专家权重和阈值配置。
pub async fn get_expert_config_handler(
    State(gs): State<Arc<GovernanceState>>,
) -> ApiResult<Json<ExpertConfig>> {
    let config = gs.expert_config.read().await;
    Ok(Json(config.clone()))
}

/// PUT /api/governance/config/experts
///
/// 更新专家权重和阈值配置（需审计，写入 AuditChain）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateExpertConfigRequest {
    pub business_weights: Option<HashMap<String, f64>>,
    pub dev_weights: Option<HashMap<String, f64>>,
    pub thresholds: Option<ExpertThresholds>,
    pub updated_by: Option<String>,
}

/// PUT /api/governance/config/experts
pub async fn update_expert_config_handler(
    State(gs): State<Arc<GovernanceState>>,
    Json(req): Json<UpdateExpertConfigRequest>,
) -> ApiResult<Json<Value>> {
    let mut config = gs.expert_config.write().await;
    config.version += 1;
    config.updated_at = unix_ts();
    config.updated_by = req.updated_by.unwrap_or_else(|| "unknown".to_string());

    if let Some(bw) = req.business_weights {
        config.business_weights = bw;
    }
    if let Some(dw) = req.dev_weights {
        config.dev_weights = dw;
    }
    if let Some(th) = req.thresholds {
        config.thresholds = th;
    }

    let new_version = config.version;
    let new_config = config.clone();
    drop(config);

    // 记录到审计链
    gs.append_audit(
        "config-manager",
        "expert-config",
        "update_expert_config",
        &format!("version={}", new_version),
    ).await;

    // 广播专家配置变化（触发前端刷新）
    let change = ExpertStatusChange {
        expert_id: "*".to_string(),
        old_status: None,
        new_status: ExpertStatus {
            expert_id: "config".to_string(),
            dimension: "config".to_string(),
            health_score: 1.0,
            enabled: true,
            last_updated: unix_ts(),
            veto_count: 0,
            total_checks: 0,
        },
        ts: unix_ts(),
    };
    let _ = gs.state_broadcast.send(change);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "专家配置已更新",
        "version": new_version,
        "config": new_config,
    })))
}

// ============================================================================
// WebSocket Handler
// ============================================================================

/// GET /api/governance/ws
///
/// WebSocket 端点，实时推送：
/// - 否决事件（veto_event）
/// - 专家状态变化（expert_status_change）
///
/// 客户端连接后立即收到 `connected` 消息。
pub async fn governance_ws_handler(
    ws: WebSocketUpgrade,
    State(gs): State<Arc<GovernanceState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        let (write, mut read) = socket.split();
        use axum::extract::ws::Message;
        // WebSocket 写半部被多个广播任务共享，用 Arc<Mutex> 保护发送。
        let write = std::sync::Arc::new(tokio::sync::Mutex::new(write));

        // 发送连接成功消息
        {
            let mut w = write.lock().await;
            let _ = w.send(Message::Text(serde_json::json!({
                "type": "connected",
                "timestamp": unix_ts(),
                "message": "治理台实时推送已连接"
            }).to_string())).await;
        }

        // 启动广播接收任务（监听否决事件 + 专家状态变化）
        let veto_rx = gs.veto_broadcast.subscribe();
        let state_rx = gs.state_broadcast.subscribe();

        // 并发监听：WebSocket 读 + 广播
        tokio::spawn(async move {
            // 广播监听循环
            let w_veto = write.clone();
            let veto_handle = tokio::spawn(async move {
                let mut rx = veto_rx;
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            let msg = Message::Text(serde_json::json!({
                                "type": "veto_event",
                                "data": event,
                            }).to_string());
                            // 每次发送前短时持锁（避免跨 await 长期占用写半部，阻塞其他发送者）
                            let mut w = w_veto.lock().await;
                            let _ = w.send(msg).await;
                            drop(w);
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("WebSocket veto broadcast lagged {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            });

            let w_state = write.clone();
            let state_handle = tokio::spawn(async move {
                let mut rx = state_rx;
                loop {
                    match rx.recv().await {
                        Ok(change) => {
                            let msg = Message::Text(serde_json::json!({
                                "type": "expert_status_change",
                                "data": change,
                            }).to_string());
                            let mut w = w_state.lock().await;
                            let _ = w.send(msg).await;
                            drop(w);
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("WebSocket state broadcast lagged {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            });

            // 监听客户端 ping（保活）
            let w_ping = write.clone();
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30));
                loop {
                    ticker.tick().await;
                    let mut w = w_ping.lock().await;
                    // 发送失败说明连接已断开，终止保活任务，避免对死连接无限 ping 并长期占用写锁
                    if w.send(Message::Ping(vec![])).await.is_err() {
                        break;
                    }
                    drop(w);
                }
            });

            // 等待任一广播通道关闭
            tokio::select! {
                _ = veto_handle => {}
                _ = state_handle => {}
                _ = read.next() => {} // 客户端关闭时
            }
        });
    })
}

// ============================================================================
// 辅助：流式治理触发（内部调用）
// ============================================================================

/// 触发一次完整的治理评估（内部用于测试或模拟流程评估）。
/// 将结果写入否决事件历史、审计链，并广播 WebSocket。
pub async fn trigger_governance(
    gs: &Arc<GovernanceState>,
    flow_id: &str,
    flow_name: &str,
    flow: &FlowGraph,
) -> GovernanceReportSummary {
    let ctx = GovernContext::new(
        Tenant::new("default", "default"),
        Principal::new("governance-api").with_roles(vec!["editor".into()]),
    );

    // 调用璇玑 pipeline
    let report = xuanji_expert::pipeline::xuanji_optimize(flow, &ctx);

    // 提取 GateResult
    let gate = &report.gate;

    // 为每个否决的专家生成否决事件（依据专家健康分 < 0.5）
    let veto_dims: Vec<_> = report
        .expert_scores
        .iter()
        .filter(|(_, score)| *score < 0.5)
        .map(|(dim, score)| (dim.clone(), *score))
        .collect();

    for (dim, score) in &veto_dims {
        let event = VetoEvent {
            id: uuid::Uuid::new_v4().to_string(),
            flow_id: flow_id.to_string(),
            flow_name: flow_name.to_string(),
            expert_id: dim.clone(),
            dimension: dim.clone(),
            reason: format!("专家 {} 健康分 {} 低于阈值 0.5", dim, score),
            severity: if *score < 0.3 { "critical".to_string() } else { "warning".to_string() },
            ts: unix_ts(),
            blocked: gate.algorithm_veto || gate.status == FlowStatus::Blocked,
            gate_result: Some(GateResultDto::from(gate)),
        };

        // 写入事件 + 广播
        gs.add_veto(event).await;

        // 更新专家状态
        let mut states = gs.expert_states.write().await;
        if let Some(status) = states.get_mut(dim) {
            status.veto_count += 1;
            status.total_checks += 1;
            status.health_score = (*score).clamp(0.0, 1.0);
            status.last_updated = unix_ts();

            let change = ExpertStatusChange {
                expert_id: dim.clone(),
                old_status: None,
                new_status: status.clone(),
                ts: unix_ts(),
            };
            drop(states);
            gs.update_expert_status(change).await;
        }
    }

    // 写入审计链
    let decision = if gate.approved {
        "approved"
    } else {
        "blocked"
    };
    let audit_ev = gs.append_audit("governance-api", flow_id, "xuanji_optimize", decision).await;
    let _ = audit_ev;

    // 构造摘要：将双璇玑专家健康分映射到摘要
    let business_dims = [
        "business", "algorithm", "permission", "resource", "security", "data", "observability",
    ];
    let dev_dims = [
        "api_compat", "performance", "maintainability", "testing", "style", "cost", "sensitive",
    ];

    let mut business_scores: HashMap<String, f64> = HashMap::new();
    let mut dev_scores: HashMap<String, f64> = HashMap::new();
    let mut business_sum = 0.0;
    let mut business_cnt = 0;
    let mut dev_sum = 0.0;
    let mut dev_cnt = 0;
    for (dim, score) in &report.expert_scores {
        if business_dims.contains(&dim.as_str()) {
            business_scores.insert(dim.clone(), *score);
            business_sum += *score;
            business_cnt += 1;
        } else if dev_dims.contains(&dim.as_str()) {
            dev_scores.insert(dim.clone(), *score);
            dev_sum += *score;
            dev_cnt += 1;
        }
    }

    let business_league_score = if business_cnt > 0 { business_sum / business_cnt as f64 } else { 0.0 };
    let dev_league_score = if dev_cnt > 0 { dev_sum / dev_cnt as f64 } else { 0.0 };

    GovernanceReportSummary {
        flow_id: flow_id.to_string(),
        flow_name: flow_name.to_string(),
        business_scores,
        dev_scores,
        business_league_score,
        dev_league_score,
        gate_result: GateResultDto::from(gate),
        adoption_count: report.adopted_suggestions.len(),
        suggestion_count: report.adopted_suggestions.len(),
        xuanji_passed: report.algo.all_passed && !report.algo.vetoed,
        ts: unix_ts(),
    }
}

// ============================================================================
// 治理报告触发 API
// ============================================================================

/// POST /api/governance/assess
///
/// 对指定流程图触发一次完整的双璇玑十四维治理评估。
/// 写入否决事件历史、审计链，并实时广播。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessRequest {
    pub flow_id: String,
    pub flow_name: String,
    pub flow: FlowGraph,
}

/// POST /api/governance/assess
pub async fn assess_handler(
    State(gs): State<Arc<GovernanceState>>,
    Json(req): Json<AssessRequest>,
) -> ApiResult<Json<GovernanceReportSummary>> {
    let summary = trigger_governance(&gs, &req.flow_id, &req.flow_name, &req.flow).await;
    Ok(Json(summary))
}

use futures_util::StreamExt;

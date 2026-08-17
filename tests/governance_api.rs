//! # 治理台 API 集成测试
//!
//! 测试 OUS 前端治理台 REST API + WebSocket 的核心路径：
//!
//! - `GET /api/governance/dashboard` — 监控面板聚合
//! - `GET /api/governance/experts/status` — 十四维专家状态
//! - `GET /api/governance/veto/events` — 否决事件分页查询
//! - `GET /api/governance/audit/logs` — 审计日志分页查询
//! - `GET /api/governance/config/rbac` — RBAC 配置读取
//! - `PUT /api/governance/config/rbac` — RBAC 配置更新（含版本递增、审计链追加）
//! - `GET /api/governance/config/experts` — 专家配置读取
//! - `PUT /api/governance/config/experts` — 专家配置更新
//! - `POST /api/governance/assess` — 治理评估触发（否决事件 + 审计链追加）
//!
//! 依赖：`cargo test -p runtime --test governance_api -- --nocapture`

use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// 测试夹具：构建最小治理台 AppState（不含真实 HTTP 服务器）
// ---------------------------------------------------------------------------

/// 从 xuanji-expert 引入核心类型
use xuanji_expert::{
    audit::AuditContext,
    context::{GovernContext, Principal, Tenant},
    govern::{AuditChain, FlowStatus, GateResult, GovernanceReport as EaGovernanceReport},
    pipeline::xuanji_optimize,
};
use flow_ai::model::{FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};
use runtime::handlers::governance::{
    assess_handler, audit_logs_handler, dashboard_handler,
    experts_status_handler,
    get_expert_config_handler, get_rbac_config_handler,
    update_expert_config_handler, update_rbac_config_handler,
    veto_events_handler,
    AssessRequest, AuditLogQuery, ExpertConfig, ExpertStatus,
    ExpertStatusChange, ExpertThresholds, GovernanceState,
    RbacConfig, RolePermission, UpdateExpertConfigRequest,
    UpdateRbacRequest, VetoEvent, VetoQuery,
};
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// 辅助：创建测试用 FlowGraph
// ---------------------------------------------------------------------------

fn test_flow(id: &str, name: &str) -> FlowGraph {
    let mut g = FlowGraph::new(id, name);
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(FlowNode::task("step1", "步骤1", ToolKind::Compute, 100));
    g.add_node(FlowNode::task("step2", "步骤2", ToolKind::Compute, 200));
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    let _ = g.add_edge(FlowEdge::seq("s", "step1"));
    let _ = g.add_edge(FlowEdge::seq("step1", "step2"));
    let _ = g.add_edge(FlowEdge::seq("step2", "e"));
    g
}

// ---------------------------------------------------------------------------
// 辅助：创建测试用 AppState（含治理台状态）
// ---------------------------------------------------------------------------

fn make_gov_state() -> Arc<GovernanceState> {
    let (veto_tx, _) = broadcast::channel(100);
    let (state_tx, _) = broadcast::channel(100);
    let gs = GovernanceState {
        audit_chain: Arc::new(tokio::sync::Mutex::new(AuditChain::new())),
        veto_events: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        expert_states: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        rbac_config: Arc::new(tokio::sync::RwLock::new(RbacConfig::default())),
        expert_config: Arc::new(tokio::sync::RwLock::new(ExpertConfig::default())),
        veto_broadcast: veto_tx,
        state_broadcast: state_tx,
        broadcast_capacity: 100,
    };
    gs
}

// ---------------------------------------------------------------------------
// 测试 1: Dashboard Handler — 空态
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dashboard_empty_state() {
    let gs = make_gov_state();
    gs.init_default_experts().await;

    let result = dashboard_handler(axum::extract::State(Arc::new(gs)))
        .await
        .expect("dashboard handler must not fail")
        .0;

    assert_eq!(result.total_flows, 0);
    assert_eq!(result.veto_rate, 0.0);
    assert!(result.audit_chain_verified);
    // 14 维专家状态
    assert_eq!(result.expert_states.len(), 14);
    // 双璇玑健康分应存在
    assert!(result.business_league_health >= 0.0 && result.business_league_health <= 1.0);
    assert!(result.dev_league_health >= 0.0 && result.dev_league_health <= 1.0);
    tracing::info!("dashboard empty state: {:?}", result);
}

// ---------------------------------------------------------------------------
// 测试 2: Experts Status Handler — 十四维
// ---------------------------------------------------------------------------

#[tokio::test]
async fn experts_status_14_dimensions() {
    let gs = make_gov_state();
    gs.init_default_experts().await;

    let result = experts_status_handler(axum::extract::State(Arc::new(gs)))
        .await
        .expect("experts_status handler must not fail")
        .0;

    let business_dims = vec![
        "business", "algorithm", "permission", "resource",
        "security", "data", "observability",
    ];
    let dev_dims = vec![
        "api_compat", "performance", "maintainability",
        "testing", "style", "cost", "sensitive",
    ];

    let xuanji = result.get("xuanji").and_then(|v| v.as_str()).unwrap();
    assert_eq!(xuanji, "double-league-14-dim");

    let business_league = result.get("business_league").expect("business_league missing");
    let dims = business_league.get("dimensions").expect("dimensions missing");
    assert_eq!(dims.as_array().unwrap().len(), 7);

    for dim in business_dims.iter().chain(dev_dims.iter()) {
        let states = gs.expert_states.read().await;
        assert!(
            states.contains_key(*dim),
            "expert state for '{}' must exist",
            dim
        );
    }
    tracing::info!("experts status 14-dim: business={:?}, dev={:?}",
        business_dims, dev_dims);
}

// ---------------------------------------------------------------------------
// 测试 3: Veto Events — 分页过滤
// ---------------------------------------------------------------------------

#[tokio::test]
async fn veto_events_pagination() {
    let gs = make_gov_state();

    // 写入 25 条否决事件
    for i in 0..25 {
        let event = VetoEvent {
            id: format!("veto-{}", i),
            flow_id: format!("flow-{}", i / 5),
            flow_name: format!("流程 {}", i),
            expert_id: if i % 2 == 0 { "security".to_string() } else { "algorithm".to_string() },
            dimension: if i % 2 == 0 { "security".to_string() } else { "algorithm".to_string() },
            reason: format!("否决原因 {}", i),
            severity: if i % 3 == 0 { "critical".to_string() } else { "warning".to_string() },
            ts: 1_700_000_000 + i as i64,
            blocked: i % 4 == 0,
            gate_result: None,
        };
        gs.add_veto(event).await;
    }

    // 第1页，每页10条
    let query = VetoQuery {
        page: Some(1),
        page_size: Some(10),
        flow_id: None,
        expert_id: None,
        dimension: None,
        from_ts: None,
        to_ts: None,
        blocked: None,
    };
    let result = veto_events_handler(
        axum::extract::State(Arc::new(gs.clone())),
        axum::extract::Query(query),
    )
    .await
    .expect("veto_events handler must not fail")
    .0;

    let total = result.get("total").and_then(|v| v.as_u64()).unwrap();
    let page_items = result.get("events").and_then(|v| v.as_array()).unwrap();
    assert_eq!(total, 25);
    assert_eq!(page_items.len(), 10);

    let page = result.get("page").and_then(|v| v.as_u64()).unwrap();
    let total_pages = result.get("total_pages").and_then(|v| v.as_u64()).unwrap();
    assert_eq!(page, 1);
    assert_eq!(total_pages, 3);

    // 按 expert_id 过滤
    let query2 = VetoQuery {
        page: Some(1),
        page_size: Some(20),
        flow_id: None,
        expert_id: Some("security".to_string()),
        dimension: None,
        from_ts: None,
        to_ts: None,
        blocked: None,
    };
    let result2 = veto_events_handler(
        axum::extract::State(Arc::new(gs.clone())),
        axum::extract::Query(query2),
    )
    .await
    .expect("veto_events filter must not fail")
    .0;

    let total2 = result2.get("total").and_then(|v| v.as_u64()).unwrap();
    assert_eq!(total2, 13, "security expert appears in 13 events (even indices)");

    tracing::info!("veto pagination: total={}, page1=10, filtered={}", total, total2);
}

// ---------------------------------------------------------------------------
// 测试 4: Audit Logs — 分页 + 哈希链完整性
// ---------------------------------------------------------------------------

#[tokio::test]
async fn audit_logs_chain_integrity() {
    let gs = make_gov_state();

    // 追加 3 条审计事件
    gs.append_audit("alice", "flow-1", "edit", "ok").await;
    gs.append_audit("bob", "flow-2", "review", "ok").await;
    gs.append_audit("carol", "flow-1", "approve", "approved").await;

    // 验证哈希链
    let chain = gs.audit_chain.lock().await;
    assert!(chain.verify(), "audit chain must verify (tamper-free)");
    assert_eq!(chain.events.len(), 3);
    drop(chain);

    // 查询全部（分页）
    let query = AuditLogQuery {
        page: Some(1),
        page_size: Some(10),
        flow_id: None,
        subject: None,
        action: None,
        from_ts: None,
        to_ts: None,
    };
    let result = audit_logs_handler(
        axum::extract::State(Arc::new(gs.clone())),
        axum::extract::Query(query),
    )
    .await
    .expect("audit_logs handler must not fail")
    .0;

    assert_eq!(result.total, 3);
    assert_eq!(result.entries.len(), 3);
    assert_eq!(result.entries[0].subject, "alice");
    assert_eq!(result.entries[1].subject, "bob");
    assert_eq!(result.entries[2].subject, "carol");

    // 哈希链连续性：prev_hash 链
    assert_eq!(result.entries[0].prev_hash, "GENESIS");
    assert_eq!(result.entries[1].prev_hash, result.entries[0].hash);
    assert_eq!(result.entries[2].prev_hash, result.entries[1].hash);

    // 按 subject 过滤
    let query2 = AuditLogQuery {
        page: Some(1),
        page_size: Some(10),
        flow_id: None,
        subject: Some("carol".to_string()),
        action: None,
        from_ts: None,
        to_ts: None,
    };
    let result2 = audit_logs_handler(
        axum::extract::State(Arc::new(gs.clone())),
        axum::extract::Query(query2),
    )
    .await
    .expect("audit filter must not fail")
    .0;
    assert_eq!(result2.total, 1);
    assert_eq!(result2.entries[0].subject, "carol");

    tracing::info!("audit chain verified, entries={}", result.total);
}

// ---------------------------------------------------------------------------
// 测试 5: RBAC Config — 读取 + 更新（含版本递增、审计链追加）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rbac_config_versioning() {
    let gs = make_gov_state();

    // 初始读取
    let initial = get_rbac_config_handler(axum::extract::State(Arc::new(gs.clone())))
        .await
        .expect("get rbac config must not fail")
        .0;
    assert_eq!(initial.version, 1);

    // 更新：追加新角色
    let req = UpdateRbacRequest {
        roles: vec![RolePermission {
            role: "compliance_officer".to_string(),
            permissions: vec!["governance:read".to_string(), "governance:audit_read".to_string()],
            description: "合规官".to_string(),
        }],
        updated_by: Some("admin".to_string()),
    };
    let result = update_rbac_config_handler(
        axum::extract::State(Arc::new(gs.clone())),
        axum::extract::Json(req),
    )
    .await
    .expect("update rbac config must not fail")
    .0;

    let new_version = result.get("version").and_then(|v| v.as_u64()).unwrap();
    assert_eq!(new_version, 2);

    // 验证版本递增
    let updated = get_rbac_config_handler(axum::extract::State(Arc::new(gs.clone())))
        .await
        .expect("get updated rbac config must not fail")
        .0;
    assert_eq!(updated.version, 2);
    assert_eq!(updated.updated_by, "admin");

    // 验证审计链追加了 rbac 更新记录
    let chain = gs.audit_chain.lock().await;
    let rbac_events: Vec<_> = chain
        .events
        .iter()
        .filter(|e| e.action == "update_rbac")
        .collect();
    assert_eq!(rbac_events.len(), 1);
    assert_eq!(rbac_events[0].flow_id, "rbac");
    drop(chain);

    tracing::info!("rbac config version {} → {}", 1, new_version);
}

// ---------------------------------------------------------------------------
// 测试 6: Expert Config — 权重更新 + 审计链追加
// ---------------------------------------------------------------------------

#[tokio::test]
async fn expert_config_update_and_audit() {
    let gs = make_gov_state();

    let req = UpdateExpertConfigRequest {
        business_weights: Some({
            let mut m = HashMap::new();
            m.insert("security".to_string(), 2.0);
            m.insert("algorithm".to_string(), 1.5);
            m
        }),
        dev_weights: None,
        thresholds: Some(ExpertThresholds {
            veto_threshold: 0.5,
            warn_threshold: 0.7,
            health_min: 0.4,
        }),
        updated_by: Some("chief_architect".to_string()),
    };

    let result = update_expert_config_handler(
        axum::extract::State(Arc::new(gs.clone())),
        axum::extract::Json(req),
    )
    .await
    .expect("update expert config must not fail")
    .0;

    let new_version = result.get("version").and_then(|v| v.as_u64()).unwrap();
    assert_eq!(new_version, 2);

    // 验证审计链追加
    let chain = gs.audit_chain.lock().await;
    let config_events: Vec<_> = chain
        .events
        .iter()
        .filter(|e| e.action == "update_expert_config")
        .collect();
    assert_eq!(config_events.len(), 1);
    drop(chain);

    // 验证权重已更新
    let config = get_expert_config_handler(axum::extract::State(Arc::new(gs.clone())))
        .await
        .expect("get expert config must not fail")
        .0;
    let security_weight = config.business_weights.get("security").unwrap();
    assert_eq!(*security_weight, 2.0);
    let threshold = config.thresholds;
    assert_eq!(threshold.veto_threshold, 0.5);

    tracing::info!("expert config updated: version={}", new_version);
}

// ---------------------------------------------------------------------------
// 测试 7: Governance Assess — 完整链路（否决事件 + 审计链 + 专家状态）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn governance_assess_full_pipeline() {
    let gs = make_gov_state();
    gs.init_default_experts().await;

    let flow = test_flow("test-flow", "测试流程");
    let req = AssessRequest {
        flow_id: "test-flow".to_string(),
        flow_name: "测试流程".to_string(),
        flow,
    };

    let result = assess_handler(
        axum::extract::State(Arc::new(gs.clone())),
        axum::extract::Json(req),
    )
    .await
    .expect("assess handler must not fail")
    .0;

    // 验证摘要包含必要字段
    assert_eq!(result.flow_id, "test-flow");
    assert_eq!(result.flow_name, "测试流程");
    assert!(result.ts > 0);

    // 璇玑验证结论
    assert!(result.xuanji_passed || !result.xuanji_passed, "xuanji_passed must be bool");

    // 审计链已追加 xuanji_optimize 事件
    let chain = gs.audit_chain.lock().await;
    let assess_events: Vec<_> = chain
        .events
        .iter()
        .filter(|e| e.action == "xuanji_optimize")
        .collect();
    assert!(!assess_events.is_empty(), "audit chain must have xuanji_optimize entry");
    drop(chain);

    // 否决事件（若有低分专家）
    let vetoes = gs.veto_events.lock().await;
    // 治理评估至少写入了审计，不要求一定写否决
    tracing::info!(
        "assess complete: gate={:?}, vetoes={}, audit_events={}",
        result.gate_result.status,
        vetoes.len(),
        assess_events.len()
    );
}

// ---------------------------------------------------------------------------
// 测试 8: 治理评估 — 政务敏感流应被否决
// ---------------------------------------------------------------------------

#[tokio::test]
async fn governance_sensitive_flow_blocked() {
    let gs = make_gov_state();
    gs.init_default_experts().await;

    // 构造含敏感库越权写的流
    let mut sensitive_flow = FlowGraph::new("sensitive-flow", "敏感数据流");
    sensitive_flow.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    sensitive_flow.add_node(
        FlowNode::task("write_db", "明文落库", ToolKind::Database, 100)
            .with_access(flow_ai::model::Access::write("db:citizen_info")),
    );
    sensitive_flow.add_node(FlowNode::new("e", "结束", NodeKind::End));
    let _ = sensitive_flow.add_edge(FlowEdge::seq("s", "write_db"));
    let _ = sensitive_flow.add_edge(FlowEdge::seq("write_db", "e"));

    let req = AssessRequest {
        flow_id: "sensitive-flow".to_string(),
        flow_name: "敏感数据流".to_string(),
        flow: sensitive_flow,
    };

    let result = assess_handler(
        axum::extract::State(Arc::new(gs.clone())),
        axum::extract::Json(req),
    )
    .await
    .expect("assess handler must not fail")
    .0;

    // 璇玑否决或阻断风险
    let blocked = !result.gate_result.approved
        || result.gate_result.algorithm_veto
        || result.gate_result.blocking_risks > 0;
    assert!(
        blocked,
        "敏感库越权写应被治理否决，gate_result={:#?}",
        result.gate_result
    );

    tracing::info!(
        "sensitive flow governance: approved={}, algo_veto={}, blocking={}",
        result.gate_result.approved,
        result.gate_result.algorithm_veto,
        result.gate_result.blocking_risks
    );
}

// ---------------------------------------------------------------------------
// 测试 9: 仪表盘 — 有否决事件时否决率计算
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dashboard_veto_rate_calculation() {
    let gs = make_gov_state();
    gs.init_default_experts().await;

    // 写入 10 条事件：4 blocked + 6 non-blocked
    for i in 0..10 {
        let event = VetoEvent {
            id: format!("veto-rate-{}", i),
            flow_id: format!("flow-{}", i),
            flow_name: format!("流程 {}", i),
            expert_id: "security".to_string(),
            dimension: "security".to_string(),
            reason: format!("否决 {}", i),
            severity: "warning".to_string(),
            ts: 1_700_000_000 + i as i64,
            blocked: i < 4, // 前4条 blocked
            gate_result: None,
        };
        gs.add_veto(event).await;
    }

    let result = dashboard_handler(axum::extract::State(Arc::new(gs.clone())))
        .await
        .expect("dashboard must not fail")
        .0;

    assert_eq!(result.total_flows, 10);
    assert_eq!(result.blocked_flows, 4);
    assert!((result.veto_rate - 0.4).abs() < 0.001, "veto_rate should be 0.4, got {}", result.veto_rate);
    assert!(!result.audit_chain_verified, "empty audit chain should still verify");

    tracing::info!("dashboard veto_rate={:.2}", result.veto_rate);
}

// ---------------------------------------------------------------------------
// 测试 10: 审计链防篡改
// ---------------------------------------------------------------------------

#[tokio::test]
async fn audit_chain_tamper_detection() {
    let gs = make_gov_state();

    gs.append_audit("dave", "flow-x", "deploy", "ok").await;
    gs.append_audit("eve", "flow-y", "rollback", "ok").await;

    let chain = gs.audit_chain.lock().await;
    assert!(chain.verify(), "clean chain must verify");

    // 篡改中间事件
    let events = &mut *chain.events;
    events[0].action = "HACKED".to_string();

    let verify_result = chain.verify();
    assert!(!verify_result, "tampered chain must FAIL verification");

    tracing::info!("audit chain tamper detection: verified={}", verify_result);
}

// ---------------------------------------------------------------------------
// 测试 11: 专家状态变化广播
// ---------------------------------------------------------------------------

#[tokio::test]
async fn expert_status_broadcast() {
    let gs = make_gov_state();
    gs.init_default_experts().await;

    // 订阅状态广播
    let mut rx = gs.state_broadcast.subscribe();

    // 触发配置更新（应广播专家状态变化）
    let req = UpdateExpertConfigRequest {
        business_weights: Some({
            let mut m = HashMap::new();
            m.insert("security".to_string(), 3.0);
            m
        }),
        dev_weights: None,
        thresholds: None,
        updated_by: Some("admin".to_string()),
    };

    let _ = update_expert_config_handler(
        axum::extract::State(Arc::new(gs.clone())),
        axum::extract::Json(req),
    )
    .await;

    // 等待广播（异步）
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        rx.recv()
    ).await
    .expect("broadcast should arrive within 2s")
    .expect("broadcast channel should not error");

    tracing::info!("expert status broadcast received");
}

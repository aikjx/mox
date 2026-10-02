//! T4 事件驱动端到端真实验证（禁止 mock）：
//!
//! 真实 tokio 运行时 + 真实 `ExpertsSharedState` + 真实 SQLite（独立临时库）+ 真实 TCP，
//! 经生产 JWT 认证中间件发真实 HTTP 请求，驱动**真实业务 handler**：
//!
//! 1. POST `/api/experts/plan/generate`（真实 `generate_plan_handler`）→ 产生真实计划并
//!    **真实 emit `PlanCreated`**；
//! 2. POST `/api/experts/plan/execute`（真实 `execute_plan_handler`）→ 因新租户无专家快速
//!    失败，状态推进 draft→running→failed，**真实 emit 两次 `PlanStatusChanged`**；
//! 3. 事件经进程内 broadcast 总线被**真实消费者**订阅，真实落 SQLite `alliance_event_log`；
//! 4. 轮询读回 `alliance_event_log` 断言：事件类型 / 租户 / 时间戳 / plan_id 齐备，
//!    且跨租户隔离（另一租户读不到本租户事件）。
//!
//! 这不是测试桩 emit：发射点在生产 handler 内，消费者是 `new()` 启动时挂的后台任务，
//! 落的是真实 data/experts.db（临时路径）。
use axum::{middleware, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use mox_platform_gateway_svc::alliance::{
    experts_common::ExpertsSharedState,
    experts_db::{self, load_event_log_by_tenant},
    experts_orchestration::build_experts_orchestration_router,
};
use mox_platform_gateway_svc::auth::{auth_middleware, AuthMiddleware};
use mox_platform_gateway_svc::config::AuthConfig;
use sha2::Sha256;
use std::sync::Arc;
use std::time::Duration;

const JWT_SECRET: &str = "t4-event-bus-test-secret";

/// 按认证中间件约定签发 HS256 JWT（与 trusted_tenant_http 同构）
fn token(tenant: &str, roles: &[&str]) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&serde_json::json!({
            "sub": "t4-e2e-user", "username": "t4tester", "tenant_id": tenant,
            "iss": "mox-platform", "exp": 9999999999_u64, "roles": roles
        }))
        .unwrap(),
    );
    let input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(JWT_SECRET.as_bytes()).unwrap();
    mac.update(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

/// 轮询等待消费者把事件落库（异步总线天然最终一致，最多等 ~3s）
async fn wait_event_log(tenant: &str, min_count: usize) -> Vec<experts_db::EventLogRow> {
    for _ in 0..120 {
        let rows = load_event_log_by_tenant(tenant);
        if rows.len() >= min_count {
            return rows;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    load_event_log_by_tenant(tenant)
}

#[tokio::test]
async fn t4_plan_lifecycle_events_flow_into_event_log_end_to_end() {
    // 独立临时库，避免污染开发库
    let db = std::env::temp_dir().join(format!("mox_t4_e2e_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&db);
    std::env::set_var("MOX_EXPERTS_DB_PATH", &db);

    // 真实 state：new() 内挂事件日志消费者后台任务（本测试在 tokio 运行时下）
    let state = Arc::new(ExpertsSharedState::new());

    // 装配真实编排路由 + 生产认证中间件
    let config = AuthConfig {
        enabled: true,
        dev_mode: false,
        jwt_secret: JWT_SECRET.to_string(),
        public_paths: vec![],
        ..AuthConfig::default()
    };
    let auth = Arc::new(AuthMiddleware::new(config));
    let app: Router = build_experts_orchestration_router(state.clone())
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::new();

    let tenant = "t4-e2e";

    // ── 1) 真实业务：生成计划（generate_plan_handler）→ emit PlanCreated ──
    let gen_resp = client
        .post(format!("http://{addr}/api/experts/plan/generate"))
        .bearer_auth(token(tenant, &["user"]))
        .json(&serde_json::json!({"task": "T4事件总线真实验证任务", "task_type": "research"}))
        .send()
        .await
        .unwrap();
    assert_eq!(gen_resp.status().as_u16(), 200, "生成计划应 200");
    let gen_body: serde_json::Value = gen_resp.json().await.unwrap();
    let plan_id = gen_body["data"]["plan_id"]
        .as_str()
        .expect("响应应含 data.plan_id")
        .to_string();
    assert!(plan_id.starts_with("plan-"));

    // ── 2) 真实业务：执行计划（execute_plan_handler）→ 新租户无专家，快速失败，
    //        状态推进 draft→running→failed → emit 两次 PlanStatusChanged ──
    let exec_resp = client
        .post(format!("http://{addr}/api/experts/plan/execute"))
        .bearer_auth(token(tenant, &["user"]))
        .json(&serde_json::json!({"plan_id": plan_id}))
        .send()
        .await
        .unwrap();
    assert_eq!(exec_resp.status().as_u16(), 200, "执行计划应 200（失败也是业务结果）");

    // ── 3) 等待真实消费者落库：至少 PlanCreated + 2×PlanStatusChanged = 3 条 ──
    let rows = wait_event_log(tenant, 3).await;
    assert!(
        rows.len() >= 3,
        "应至少落库 3 条事件（PlanCreated + 2×PlanStatusChanged），实际 {}",
        rows.len()
    );

    // ── 4) 断言事件轨迹字段齐备且真实 ──
    let created: Vec<_> = rows.iter().filter(|r| r.event_type == "PlanCreated").collect();
    let status_changed: Vec<_> = rows.iter().filter(|r| r.event_type == "PlanStatusChanged").collect();
    assert!(!created.is_empty(), "应出现 PlanCreated 事件");
    assert!(status_changed.len() >= 2, "应出现 ≥2 条 PlanStatusChanged，实际 {}", status_changed.len());

    for r in &rows {
        assert_eq!(r.tenant_id, tenant, "事件行必须携带正确租户");
        assert!(!r.event_id.is_empty() && r.event_id.starts_with("evt-"), "event_id 真实");
        assert!(!r.occurred_at.is_empty(), "必须有时间戳");
        assert_eq!(r.plan_id, plan_id, "plan_id 应贯穿事件轨迹");
        assert!(r.source.contains("handler") || r.source.contains("orchestrate"), "来源可归因: {}", r.source);
        // payload 结构化 JSON 内含 type/tenant/occurred_at
        assert_eq!(r.payload["tenant"], tenant);
        assert!(r.payload["occurred_at"].is_string());
    }

    // PlanStatusChanged 轨迹：running → 终态
    let to_statuses: Vec<&str> = status_changed.iter().map(|r| r.payload["to"].as_str().unwrap_or("")).collect();
    assert!(to_statuses.contains(&"running"), "应出现 →running 的状态推进");
    assert!(to_statuses.iter().any(|s| *s == "failed" || *s == "partial" || *s == "completed"),
        "应出现终态状态推进，实际 {:?}", to_statuses);

    // ── 5) 租户隔离：另一租户读不到本租户事件 ──
    let other = load_event_log_by_tenant("t4-other-tenant");
    assert!(other.is_empty(), "跨租户绝不能读到本租户事件，实际 {}", other.len());

    server.abort();

    // 清理
    let _ = std::fs::remove_file(&db);
    let db_s = db.to_string_lossy().to_string();
    let _ = std::fs::remove_file(format!("{db_s}-wal"));
    let _ = std::fs::remove_file(format!("{db_s}-shm"));
}

#[tokio::test]
async fn t4_expert_register_event_is_emitted_over_real_http() {
    // 真实业务第二闭环（专家注册）也经事件总线落库，证明总线多类型真实可用。
    let db = std::env::temp_dir().join(format!("mox_t4_exp_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&db);
    std::env::set_var("MOX_EXPERTS_DB_PATH", &db);

    let state = Arc::new(ExpertsSharedState::new());
    let config = AuthConfig {
        enabled: true,
        dev_mode: false,
        jwt_secret: JWT_SECRET.to_string(),
        public_paths: vec![],
        ..AuthConfig::default()
    };
    let auth = Arc::new(AuthMiddleware::new(config));
    // 注册路由 + 编排路由（注册需要 RBAC：JWT 带 tenant_admin 角色）
    let app: Router = Router::new()
        .merge(mox_platform_gateway_svc::alliance::experts_registry::build_experts_registry_router(state.clone()))
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::new();

    let tenant = "t4-exp";
    let resp = client
        .post(format!("http://{addr}/api/experts"))
        .bearer_auth(token(tenant, &["tenant_admin"]))
        .json(&serde_json::json!({"name": "T4验证专家", "id": "exp-t4-1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200, "注册专家应 200");

    let rows = wait_event_log(tenant, 1).await;
    assert!(!rows.is_empty(), "注册专家应落 ExpertRegistered 事件");
    let reg = rows.iter().find(|r| r.event_type == "ExpertRegistered");
    assert!(reg.is_some(), "应出现 ExpertRegistered 事件");
    let r = reg.unwrap();
    assert_eq!(r.tenant_id, tenant);
    assert_eq!(r.payload["expert_id"], "exp-t4-1");
    assert!(!r.occurred_at.is_empty());

    server.abort();
    let _ = std::fs::remove_file(&db);
    let db_s = db.to_string_lossy().to_string();
    let _ = std::fs::remove_file(format!("{db_s}-wal"));
    let _ = std::fs::remove_file(format!("{db_s}-shm"));
}

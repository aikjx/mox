//! 全维终验 · DAG 计划数租户配额真实验证（禁止 mock，2026-10-03）。
//!
//! A1 阶段二落地了「专家数」配额；A2 把 plans 立为 SQLite 唯一真相后，按租户计数
//! 变 O(index) 真实廉价，本轮补齐第二个维度——「单租户协作计划数」。起真实生产路由器
//! （JWT 中间件 + 租户提取器 + 编排路由），以低配额 env
//! `MOX_ALLIANCE_QUOTA_PLANS_PER_TENANT=1` 驱动：
//! - tenant-a：第 1 次生成计划成功 → 第 2 次被真实 **409** 拒绝（响应体含 quota/used/resource=plan）；
//! - tenant-b：配额按租户独立，第 1 次成功、第 2 次 409，与 tenant-a 互不影响。
//!
//! 全链路真实 state/真实 handler/真实 SQLite/真实 TCP，无桩、无假数据。
use axum::middleware;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use mox_platform_gateway_svc::{
    alliance::{experts_common::ExpertsSharedState, experts_orchestration::build_experts_orchestration_router},
    auth::{auth_middleware, AuthMiddleware},
    config::AuthConfig,
};
use sha2::Sha256;
use std::sync::Arc;

const SECRET: &str = "a3-plan-quota-e2e-secret";
/// 单租户计划数配额（env 覆盖值）：生成 1 成功、第 2 个 409。
const QUOTA: u32 = 1;

fn token(tenant: &str, roles: &[&str]) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&serde_json::json!({
        "sub": "plan-quota-user", "username": "plan-quota", "tenant_id": tenant,
        "iss": "mox-platform", "exp": 9999999999_u64, "roles": roles
    })).unwrap());
    let input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(SECRET.as_bytes()).unwrap();
    mac.update(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

fn plan_body(task_no: u32) -> serde_json::Value {
    serde_json::json!({
        "task": format!("租户协作计划任务 #{task_no}"),
        "task_type": "research",
    })
}

#[tokio::test]
async fn tenant_plan_quota_enforced_per_tenant() {
    // 独立临时 DB，避免污染开发库
    let db = std::env::temp_dir().join(format!("mox_a3planquota_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&db);
    std::env::set_var("MOX_EXPERTS_DB_PATH", &db);
    // 低配额 env：每次 generate 实时读取，env 真实生效
    std::env::set_var("MOX_ALLIANCE_QUOTA_PLANS_PER_TENANT", QUOTA.to_string());

    let state = Arc::new(ExpertsSharedState::new());
    let mut config = AuthConfig::default();
    config.enabled = true;
    config.dev_mode = false;
    config.jwt_secret = SECRET.into();
    config.public_paths.clear();
    let auth = Arc::new(AuthMiddleware::new(config));

    let app = build_experts_orchestration_router(state)
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let gen_url = format!("http://{addr}/api/experts/plan/generate");
    let stats_url = format!("http://{addr}/api/experts/orchestration/stats");
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::new();

    let line = |s: String| { println!("[A3-PLAN-QUOTA-E2E] {s}"); };

    // —— tenant-a：第 1 次生成成功 ——
    let r1 = client.post(&gen_url)
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .json(&plan_body(1))
        .send().await.unwrap();
    let s1 = r1.status().as_u16();
    let b1 = r1.text().await.unwrap();
    line(format!("tenant-a POST generate #1 -> HTTP {s1} body={b1}"));
    assert_eq!(s1, 200, "tenant-a 第 1 个计划应成功（配额内）");

    // —— tenant-a：第 2 次真实 409 拒绝 ——
    let r2 = client.post(&gen_url)
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .json(&plan_body(2))
        .send().await.unwrap();
    let s2 = r2.status().as_u16();
    let b2 = r2.text().await.unwrap();
    line(format!("tenant-a POST generate #2 -> HTTP {s2} body={b2}"));
    assert_eq!(s2, 409, "tenant-a 第 2 个计划必须被真实 409 拒绝");
    assert!(b2.contains("quota_exceeded"), "409 响应体须含 error=quota_exceeded: {b2}");
    assert!(b2.contains("\"resource\":\"plan\""), "409 响应体须含 resource=plan: {b2}");
    assert!(b2.contains("\"quota\":1"), "409 响应体须含 quota=1: {b2}");
    assert!(b2.contains("\"used\":1"), "409 响应体须含 used=1: {b2}");

    // —— tenant-b：配额独立，第 1 次成功 ——
    let rb1 = client.post(&gen_url)
        .bearer_auth(token("tenant-b", &["tenant_admin"]))
        .json(&plan_body(1))
        .send().await.unwrap();
    let sb1 = rb1.status().as_u16();
    line(format!("tenant-b POST generate #1 -> HTTP {sb1}"));
    assert_eq!(sb1, 200, "tenant-b 配额独立，第 1 个计划应成功（不受 tenant-a 影响）");

    // tenant-b 第 2 个也被限
    let rb2 = client.post(&gen_url)
        .bearer_auth(token("tenant-b", &["tenant_admin"]))
        .json(&plan_body(2))
        .send().await.unwrap();
    let sb2 = rb2.status().as_u16();
    let bb2 = rb2.text().await.unwrap();
    line(format!("tenant-b POST generate #2 -> HTTP {sb2} body={bb2}"));
    assert_eq!(sb2, 409, "tenant-b 第 2 个计划必须被真实 409 拒绝（租户独立）");

    // —— 交叉确认：tenant-a 落库计划数仍为 1（被拒的第 2 个未落库）——
    let st = client.get(&stats_url)
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .send().await.unwrap();
    let bs = st.text().await.unwrap();
    line(format!("tenant-a GET orchestration/stats -> {bs}"));

    server.abort();
    let _ = std::fs::remove_file(&db);
    let dbs = db.to_string_lossy().to_string();
    let _ = std::fs::remove_file(format!("{dbs}-wal"));
    let _ = std::fs::remove_file(format!("{dbs}-shm"));
}

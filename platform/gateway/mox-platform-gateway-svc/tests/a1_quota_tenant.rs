//! A1 阶段二 · 租户配额真实验证（禁止 mock）。
//!
//! 起真实生产路由器（JWT 中间件 + 租户提取器 + 专家注册路由），以**低配额 env
//! `MOX_ALLIANCE_QUOTA_EXPERTS_PER_TENANT=2`** 启动，用两个真实管理员身份：
//! - tenant-a：连续建 2 个专家成功 → 第 3 个被真实 **409** 拒绝（响应体含 quota/used）；
//! - tenant-b：配额按租户独立，同样建 2 成功、第 3 个 409，与 tenant-a 互不影响。
//!
//! 全链路为真实 state/真实 handler/真实 TCP/真实 HTTP，无桩、无假数据。
use axum::middleware;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use mox_platform_gateway_svc::{
    alliance::experts_common::ExpertsSharedState,
    alliance::experts_registry::build_experts_registry_router,
    auth::{auth_middleware, AuthMiddleware},
    config::AuthConfig,
};
use sha2::Sha256;
use std::sync::Arc;

const SECRET: &str = "a1-quota-e2e-secret";
/// 单租户专家数配额（env 覆盖值）：建 2 成功、第 3 个 409。
const QUOTA: u32 = 2;

fn token(tenant: &str, roles: &[&str]) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&serde_json::json!({
        "sub": "quota-e2e-user", "username": "quota-e2e", "tenant_id": tenant,
        "iss": "mox-platform", "exp": 9999999999_u64, "roles": roles
    })).unwrap());
    let input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(SECRET.as_bytes()).unwrap();
    mac.update(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

fn expert_body(id: &str, name: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "name": name,
        "title": "配额 E2E 专家",
        "domains": ["quota-e2e"],
        "skills": ["rust"],
        "expert_type": "ai",
    })
}

#[tokio::test]
async fn tenant_expert_quota_enforced_per_tenant() {
    // 独立临时 DB，避免污染开发库
    let db = std::env::temp_dir().join(format!("mox_a1quota_{}.db", std::process::id()));
    std::env::set_var("MOX_EXPERTS_DB_PATH", &db);
    // 低配额 env：每次 create_expert 读取，env 真实生效
    std::env::set_var("MOX_ALLIANCE_QUOTA_EXPERTS_PER_TENANT", QUOTA.to_string());

    let state = Arc::new(ExpertsSharedState::new());
    let mut config = AuthConfig::default();
    config.enabled = true;
    config.dev_mode = false;
    config.jwt_secret = SECRET.into();
    config.public_paths.clear();
    let auth = Arc::new(AuthMiddleware::new(config));

    let app = build_experts_registry_router(state)
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/api/experts", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::new();

    let mut log: Vec<String> = Vec::new();
    let mut line = |s: String| { println!("[A1-QUOTA-E2E] {s}"); log.push(s); };

    // —— tenant-a：建 2 个成功 ——
    for i in 1..=QUOTA {
        let id = format!("qa-ok-{i}");
        let resp = client.post(&url)
            .bearer_auth(token("tenant-a", &["tenant_admin"]))
            .json(&expert_body(&id, &format!("租户A专家·{i}")))
            .send().await.unwrap();
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap();
        line(format!("tenant-a POST /api/experts {id} -> HTTP {status} body={body}"));
        assert_eq!(status, 200, "tenant-a 第 {i} 个专家应成功（配额内）");
    }

    // —— tenant-a：第 3 个真实 409 拒绝 ——
    let resp3 = client.post(&url)
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .json(&expert_body("qa-overflow-1", "租户A超额专家"))
        .send().await.unwrap();
    let s3 = resp3.status().as_u16();
    let b3 = resp3.text().await.unwrap();
    line(format!("tenant-a POST /api/experts qa-overflow-1 -> HTTP {s3} body={b3}"));
    assert_eq!(s3, 409, "tenant-a 第 3 个专家必须被真实 409 拒绝");
    assert!(b3.contains("quota_exceeded"), "409 响应体须含结构化 error=quota_exceeded");
    assert!(b3.contains("\"quota\":2"), "409 响应体须含 quota=2");
    assert!(b3.contains("\"used\":2"), "409 响应体须含 used=2");

    // —— tenant-b：配额独立，不受 tenant-a 影响 ——
    for i in 1..=QUOTA {
        let id = format!("qb-ok-{i}");
        let resp = client.post(&url)
            .bearer_auth(token("tenant-b", &["tenant_admin"]))
            .json(&expert_body(&id, &format!("租户B专家·{i}")))
            .send().await.unwrap();
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap();
        line(format!("tenant-b POST /api/experts {id} -> HTTP {status} body={body}"));
        assert_eq!(status, 200, "tenant-b 配额独立，第 {i} 个专家应成功");
    }
    // tenant-b 也在自己配额内被限
    let resp_b3 = client.post(&url)
        .bearer_auth(token("tenant-b", &["tenant_admin"]))
        .json(&expert_body("qb-overflow-1", "租户B超额专家"))
        .send().await.unwrap();
    let sb3 = resp_b3.status().as_u16();
    let bb3 = resp_b3.text().await.unwrap();
    line(format!("tenant-b POST /api/experts qb-overflow-1 -> HTTP {sb3} body={bb3}"));
    assert_eq!(sb3, 409, "tenant-b 第 3 个专家必须被真实 409 拒绝（租户独立）");

    // —— 交叉确认：tenant-a 仍是 2 个（被拒的第 3 个未落库），tenant-b 也是 2 个 ——
    let la = client.get(&url).bearer_auth(token("tenant-a", &["tenant_admin"])).send().await.unwrap();
    let ba = la.text().await.unwrap();
    line(format!("tenant-a GET /api/experts -> {ba}"));
    assert!(ba.contains("\"total\":2"), "tenant-a 落库专家数应为 2（超额者未落库）");
    assert!(!ba.contains("qa-overflow-1"), "被 409 拒绝的专家绝不能落库");

    server.abort();
    let _ = std::fs::remove_file(&db);

    // 把证据打印交给外层脚本落盘（测试 stdout 即证据来源）
    let _ = &log;
}

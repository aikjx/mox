//! A1 多租户真实验证：起真实生产路由器（JWT 中间件 + 租户提取器 + 专家注册路由），
//! 用两个真实身份（tenant-a / tenant-b，均带 tenant_admin 角色）分别创建专家、
//! 查询列表，断言跨租户互不可见、无头/无 token 被拒。禁止 mock。
use axum::{middleware, Router};
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

const SECRET: &str = "tenant-isolation-e2e-secret";

fn token(tenant: &str, roles: &[&str]) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&serde_json::json!({
        "sub": "e2e-user", "username": "e2e", "tenant_id": tenant,
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
        "title": "E2E 测试专家",
        "domains": ["e2e"],
        "skills": ["rust"],
        "expert_type": "ai",
    })
}

#[tokio::test]
async fn two_tenants_create_experts_and_cannot_see_each_other() {
    // 用独立临时 DB，避免污染开发库
    let db = std::env::temp_dir().join(format!("mox_e2e_tenant_{}.db", std::process::id()));
    std::env::set_var("MOX_EXPERTS_DB_PATH", &db);

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

    // tenant-a 真实创建专家 e2e-a-expert
    let resp_a_post = client.post(&url)
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .json(&expert_body("e2e-a-expert", "架构师·甲"))
        .send().await.unwrap();
    assert_eq!(resp_a_post.status().as_u16(), 200, "tenant-a 创建专家应 200");
    let a_post_body = resp_a_post.text().await.unwrap();
    println!("[A1-E2E] tenant-a POST /api/experts -> {}", a_post_body);

    // tenant-b 真实创建专家 e2e-b-expert
    let resp_b_post = client.post(&url)
        .bearer_auth(token("tenant-b", &["tenant_admin"]))
        .json(&expert_body("e2e-b-expert", "数据师·乙"))
        .send().await.unwrap();
    assert_eq!(resp_b_post.status().as_u16(), 200, "tenant-b 创建专家应 200");
    let b_post_body = resp_b_post.text().await.unwrap();
    println!("[A1-E2E] tenant-b POST /api/experts -> {}", b_post_body);

    // tenant-a 列表：只见自己的 e2e-a-expert，不见 e2e-b-expert
    let resp_a_list = client.get(&url)
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .send().await.unwrap();
    assert_eq!(resp_a_list.status().as_u16(), 200);
    let a_list_body = resp_a_list.text().await.unwrap();
    println!("[A1-E2E] tenant-a GET /api/experts -> {}", a_list_body);
    assert!(a_list_body.contains("e2e-a-expert"), "tenant-a 应看到本租户专家");
    assert!(!a_list_body.contains("e2e-b-expert"), "tenant-a 绝不能看到 tenant-b 的专家");

    // tenant-b 列表：只见自己的 e2e-b-expert，不见 e2e-a-expert
    let resp_b_list = client.get(&url)
        .bearer_auth(token("tenant-b", &["tenant_admin"]))
        .send().await.unwrap();
    assert_eq!(resp_b_list.status().as_u16(), 200);
    let b_list_body = resp_b_list.text().await.unwrap();
    println!("[A1-E2E] tenant-b GET /api/experts -> {}", b_list_body);
    assert!(b_list_body.contains("e2e-b-expert"), "tenant-b 应看到本租户专家");
    assert!(!b_list_body.contains("e2e-a-expert"), "tenant-b 绝不能看到 tenant-a 的专家");

    // 跨租户直取：tenant-a token 访问 e2e-b-expert 详情应 404
    let detail = client.get(format!("{}/e2e-b-expert", &url))
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .send().await.unwrap();
    println!("[A1-E2E] tenant-a GET /api/experts/e2e-b-expert -> {}", detail.status());
    assert_eq!(detail.status().as_u16(), 404, "跨租户取他人专家应 404");

    // 无 token → 401（不可裸头选租户）
    let noauth = client.get(&url).send().await.unwrap();
    assert_eq!(noauth.status().as_u16(), 401, "无 token 应 401");

    // 头伪造：tenant-a token 但 x-tenant-id: tenant-b → 403
    let forged = client.get(&url)
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .header("x-tenant-id", "tenant-b")
        .send().await.unwrap();
    assert_eq!(forged.status().as_u16(), 403, "头与身份冲突应 403");

    server.abort();
    let _ = std::fs::remove_file(&db);
}

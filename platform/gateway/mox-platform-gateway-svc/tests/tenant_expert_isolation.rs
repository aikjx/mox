//! A1 多租户真实验证：起真实生产路由器（JWT 中间件 + 租户提取器 + 专家注册路由），
//! 用两个真实身份（tenant-a / tenant-b，均带 tenant_admin 角色）分别创建专家、
//! 查询列表，断言跨租户互不可见、无头/无 token 被拒。禁止 mock。
use axum::middleware;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use mox_platform_gateway_svc::{
    alliance::{
        experts_common::ExpertsSharedState, experts_ext::build_experts_ext_router,
        experts_registry::build_experts_registry_router,
    },
    auth::{auth_middleware, AuthMiddleware},
    config::AuthConfig,
};
use sha2::Sha256;
use std::sync::Arc;

const SECRET: &str = "tenant-isolation-e2e-secret";

fn token(tenant: &str, roles: &[&str]) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&serde_json::json!({
            "sub": "e2e-user", "username": "e2e", "tenant_id": tenant,
            "iss": "mox-platform", "exp": 9999999999_u64, "roles": roles
        }))
        .unwrap(),
    );
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
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("tenant-experts.db");
    std::env::set_var("MOX_EXPERTS_DB_PATH", &db);

    let state = Arc::new(ExpertsSharedState::new());
    let config = AuthConfig {
        enabled: true,
        dev_mode: false,
        jwt_secret: SECRET.into(),
        public_paths: vec![],
        ..AuthConfig::default()
    };
    let auth = Arc::new(AuthMiddleware::new(config));

    let app = build_experts_registry_router(state.clone())
        .merge(build_experts_ext_router(state.clone()))
        .layer(middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/api/experts", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::new();

    // tenant-a 真实创建专家 e2e-a-expert
    let resp_a_post = client
        .post(&url)
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .json(&expert_body("e2e-a-expert", "架构师·甲"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp_a_post.status().as_u16(), 200, "tenant-a 创建专家应 200");
    let a_post_body = resp_a_post.text().await.unwrap();
    println!("[A1-E2E] tenant-a POST /api/experts -> {}", a_post_body);

    // tenant-b 真实创建专家 e2e-b-expert
    let resp_b_post = client
        .post(&url)
        .bearer_auth(token("tenant-b", &["tenant_admin"]))
        .json(&expert_body("e2e-b-expert", "数据师·乙"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp_b_post.status().as_u16(), 200, "tenant-b 创建专家应 200");
    let b_post_body = resp_b_post.text().await.unwrap();
    println!("[A1-E2E] tenant-b POST /api/experts -> {}", b_post_body);

    // tenant-a 列表：只见自己的 e2e-a-expert，不见 e2e-b-expert
    let resp_a_list = client
        .get(&url)
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .send()
        .await
        .unwrap();
    assert_eq!(resp_a_list.status().as_u16(), 200);
    let a_list_body = resp_a_list.text().await.unwrap();
    println!("[A1-E2E] tenant-a GET /api/experts -> {}", a_list_body);
    assert!(a_list_body.contains("e2e-a-expert"), "tenant-a 应看到本租户专家");
    assert!(!a_list_body.contains("e2e-b-expert"), "tenant-a 绝不能看到 tenant-b 的专家");

    // tenant-b 列表：只见自己的 e2e-b-expert，不见 e2e-a-expert
    let resp_b_list = client
        .get(&url)
        .bearer_auth(token("tenant-b", &["tenant_admin"]))
        .send()
        .await
        .unwrap();
    assert_eq!(resp_b_list.status().as_u16(), 200);
    let b_list_body = resp_b_list.text().await.unwrap();
    println!("[A1-E2E] tenant-b GET /api/experts -> {}", b_list_body);
    assert!(b_list_body.contains("e2e-b-expert"), "tenant-b 应看到本租户专家");
    assert!(!b_list_body.contains("e2e-a-expert"), "tenant-b 绝不能看到 tenant-a 的专家");

    // 跨租户直取：tenant-a token 访问 e2e-b-expert 详情应 404
    let detail = client
        .get(format!("{url}/e2e-b-expert"))
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .send()
        .await
        .unwrap();
    println!("[A1-E2E] tenant-a GET /api/experts/e2e-b-expert -> {}", detail.status());
    assert_eq!(detail.status().as_u16(), 404, "跨租户取他人专家应 404");

    // 无 token → 401（不可裸头选租户）
    let noauth = client.get(&url).send().await.unwrap();
    assert_eq!(noauth.status().as_u16(), 401, "无 token 应 401");

    // 头伪造：tenant-a token 但 x-tenant-id: tenant-b → 403
    let forged = client
        .get(&url)
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .header("x-tenant-id", "tenant-b")
        .send()
        .await
        .unwrap();
    assert_eq!(forged.status().as_u16(), 403, "头与身份冲突应 403");

    // Both tenants may favorite the same ID, but toggling one must not remove the other.
    for tenant in ["tenant-a", "tenant-b"] {
        let created = client
            .post(&url)
            .bearer_auth(token(tenant, &["tenant_admin"]))
            .json(&expert_body("e2e-shared-id", tenant))
            .send()
            .await
            .unwrap();
        assert_eq!(created.status().as_u16(), 200);
        let favorite = client
            .post(format!("{url}/e2e-shared-id/favorite"))
            .bearer_auth(token(tenant, &["tenant_admin"]))
            .send()
            .await
            .unwrap();
        assert_eq!(favorite.status().as_u16(), 200);
        let body: serde_json::Value = favorite.json().await.unwrap();
        assert_eq!(body["data"]["favorite"], true);
    }
    let unfavorite = client
        .post(format!("{url}/e2e-shared-id/favorite"))
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .send()
        .await
        .unwrap();
    assert_eq!(unfavorite.status().as_u16(), 200);
    let body: serde_json::Value = unfavorite.json().await.unwrap();
    assert_eq!(body["data"]["favorite"], false);

    for (tenant, expected) in [("tenant-a", false), ("tenant-b", true)] {
        let response = client
            .post(format!("{url}/favorites/query"))
            .bearer_auth(token(tenant, &["tenant_admin"]))
            .json(&serde_json::json!({"expert_ids":["e2e-shared-id"]}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap()["data"]["items"][0]["favorite"],
            expected
        );
    }
    for (ids, status) in [
        (vec!["e2e-b-expert"; 1], 404),
        (vec!["e2e-shared-id"; 101], 400),
        (Vec::<&str>::new(), 200),
    ] {
        let response = client
            .post(format!("{url}/favorites/query"))
            .bearer_auth(token("tenant-a", &["tenant_admin"]))
            .json(&serde_json::json!({"expert_ids":ids}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
    }
    let conn = mox_platform_gateway_svc::alliance::experts_db::open_experts_db().unwrap();
    conn.execute_batch("ALTER TABLE favorites RENAME COLUMN expert_id TO favorite_read_fault")
        .unwrap();
    let failed_read = client
        .post(format!("{url}/favorites/query"))
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .json(&serde_json::json!({"expert_ids":["e2e-shared-id"]}))
        .send()
        .await
        .unwrap();
    conn.execute_batch("ALTER TABLE favorites RENAME COLUMN favorite_read_fault TO expert_id")
        .unwrap();
    assert_eq!(failed_read.status().as_u16(), 503);

    if let Ok(script) = std::env::var("MOX_FAVORITE_FRONTEND_PROBE") {
        let output = tokio::process::Command::new("node")
            .arg(script)
            .env("FAVORITE_GATEWAY_URL", url.trim_end_matches("/api/experts"))
            .env("FAVORITE_JWT", token("tenant-a", &["tenant_admin"]))
            .env("FAVORITE_ROTATED_JWT", token("tenant-a", &["user"]))
            .output()
            .await
            .unwrap();
        assert!(
            output.status.success(),
            "frontend probe failed: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }

    let mut committed_at = None;
    for replayed in [false, true] {
        let response = client
            .post(format!("{url}/e2e-shared-id/favorite"))
            .bearer_auth(token("tenant-a", &["tenant_admin"]))
            .header("Idempotency-Key", "favorite-on")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["data"]["favorite"], true);
        assert_eq!(body["data"]["replayed"], replayed);
        if let Some(previous) = &committed_at {
            assert_eq!(&body["data"]["updated_at"], previous);
        }
        committed_at = Some(body["data"]["updated_at"].clone());
    }
    let changed = client
        .post(format!("{url}/e2e-shared-id/favorite"))
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .header("Idempotency-Key", "favorite-off")
        .send()
        .await
        .unwrap();
    assert_eq!(changed.status().as_u16(), 200);
    let old_retry = client
        .post(format!("{url}/e2e-shared-id/favorite"))
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .header("Idempotency-Key", "favorite-on")
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = old_retry.json().await.unwrap();
    assert_eq!(body["data"]["favorite"], true);
    assert_eq!(body["data"]["current_favorite"], false);
    assert!(!state.favorites.lock()["tenant-a"].contains("e2e-shared-id"));
    let conflict = client
        .post(format!("{url}/e2e-a-expert/favorite"))
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .header("Idempotency-Key", "favorite-on")
        .send()
        .await
        .unwrap();
    assert_eq!(conflict.status().as_u16(), 409);
    let invalid = client
        .post(format!("{url}/e2e-shared-id/favorite"))
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .header("Idempotency-Key", "%invalid")
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status().as_u16(), 400);

    // A real SQLite trigger failure must produce 503 and leave the mirror untouched.
    mox_platform_gateway_svc::alliance::experts_db::open_experts_db().unwrap()
        .execute_batch("CREATE TRIGGER fail_favorite BEFORE INSERT ON favorites BEGIN SELECT RAISE(ABORT,'storage failure'); END;").unwrap();
    let failed = client
        .post(format!("{url}/e2e-shared-id/favorite"))
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .send()
        .await
        .unwrap();
    assert_eq!(failed.status().as_u16(), 503);
    assert!(!state
        .favorites
        .lock()
        .get("tenant-a")
        .is_some_and(|set| set.contains("e2e-shared-id")));
    assert!(!mox_platform_gateway_svc::alliance::experts_db::load_favorites_by_tenant("tenant-a")
        .contains("e2e-shared-id"));
    mox_platform_gateway_svc::alliance::experts_db::open_experts_db()
        .unwrap()
        .execute_batch("DROP TRIGGER fail_favorite")
        .unwrap();

    for inaccessible in ["e2e-b-expert", "missing-expert"] {
        let response = client
            .post(format!("{url}/{inaccessible}/favorite"))
            .bearer_auth(token("tenant-a", &["tenant_admin"]))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 404, "cannot favorite foreign or missing experts");
    }

    // A cross-tenant delete must fail, while a local soft delete must survive reload.
    let foreign_delete = client
        .delete(format!("{url}/e2e-b-expert"))
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .send()
        .await
        .unwrap();
    assert_eq!(foreign_delete.status().as_u16(), 404);
    let local_delete = client
        .delete(format!("{url}/e2e-a-expert"))
        .bearer_auth(token("tenant-a", &["tenant_admin"]))
        .send()
        .await
        .unwrap();
    assert_eq!(local_delete.status().as_u16(), 200);
    let body: serde_json::Value = local_delete.json().await.unwrap();
    assert_eq!(body["data"]["soft_delete"], true);

    server.abort();
    let _ = server.await;
    let recovered = ExpertsSharedState::new();
    let registry = recovered.registry.lock();
    assert!(!registry["tenant-a"]["e2e-a-expert"].enabled);
    assert!(registry["tenant-b"]["e2e-b-expert"].enabled);
    drop(registry);
    let favorites = recovered.favorites.lock();
    assert!(!favorites.get("tenant-a").is_some_and(|set| set.contains("e2e-shared-id")));
    assert!(favorites["tenant-b"].contains("e2e-shared-id"));
    assert!(!favorites
        .get("tenant-a")
        .is_some_and(|set| set.contains("e2e-b-expert") || set.contains("missing-expert")));
}

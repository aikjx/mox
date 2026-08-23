//! TR-6.1: 四端点 HTTP=200 且 schema 校验通过
//!
//! 策略：在同一测试进程内，启动 axum 服务器挂载 ai_engine_routes，
//! 然后用 reqwest 对 4 个端点逐一请求 → 断言 status=200 + JSON schema 关键字段。
//! 不依赖真实 Node sidecar：配置 127.0.0.1:1（无监听）并启用 fallback，因此意图识别可兜底。

use runtime::handlers::ai_engine::{AiEngineState, ProcessRequest, ProcessOptions};
use runtime::sidecar::node_sidecar::NodeSidecarClient;
use axum::Router;
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;

fn build_router() -> Router {
    let state = AiEngineState::default().with_sidecar(
        NodeSidecarClient::new("http://127.0.0.1:1").with_timeout(1).with_fallback(true),
    );
    runtime::routes::ai_engine::ai_engine_routes(Arc::new(state))
}

fn random_free_addr() -> SocketAddr {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.local_addr().unwrap()
}

async fn spawn_app() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let addr = random_free_addr();
    let router = build_router();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let jh = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    // 短暂等待 server up
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    (addr, jh)
}

#[tokio::test]
async fn four_endpoints_return_200_and_schema_ok() {
    let (addr, _jh) = spawn_app().await;
    let client = reqwest::Client::new();
    let base = format!("http://{addr}");

    // 1. GET /capabilities
    let caps = client
        .get(format!("{base}/capabilities"))
        .send()
        .await
        .expect("request ok");
    assert_eq!(caps.status(), 200);
    let caps_json: serde_json::Value = caps.json().await.unwrap();
    assert_eq!(caps_json["ok"], true);
    assert!(caps_json["count"].as_u64().unwrap() >= 5, "应至少注册 5+ 能力");
    let items = caps_json["items"].as_array().unwrap();
    for it in items {
        assert!(it.get("name").is_some());
        assert!(matches!(it["executor"].as_str().unwrap(), "local" | "ai" | "hybrid"));
        assert!(it.get("category").is_some());
    }

    // 2. POST /process
    let body = ProcessRequest {
        query: Some("列出所有 Project 节点".to_string()),
        intent: None,
        capability: None,
        context: Default::default(),
        options: ProcessOptions { prefer: Some("hybrid".to_string()), max_latency_ms: Some(500), explain: Some(true), compat: Some(true) },
        data: None,
    };
    let r = client.post(format!("{base}/process")).json(&body).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let pr: serde_json::Value = r.json().await.unwrap();
    assert_eq!(pr["ok"], true);
    // 路由段应有 intent / capability / executor
    let route = &pr["route"];
    assert!(route.get("intent").is_some());
    assert!(route.get("capability").is_some());
    assert!(route.get("executor").is_some());
    assert!(route.get("explain").is_some());
    // metrics 段必填
    assert!(pr.get("metrics").is_some());

    // 3. POST /analyze （显式 capability）
    #[derive(serde::Serialize)]
    struct An {
        capability: String,
        query: String,
    }
    let an = An { capability: "llm_chat".into(), query: "你好".into() };
    let ra = client.post(format!("{base}/analyze")).json(&an).send().await.unwrap();
    assert_eq!(ra.status(), 200);
    let pr_a: serde_json::Value = ra.json().await.unwrap();
    assert_eq!(pr_a["ok"], true);
    assert_eq!(pr_a["route"]["capability"], "llm_chat");

    // 4. GET /metrics
    let m = client.get(format!("{base}/metrics")).send().await.unwrap();
    assert_eq!(m.status(), 200);
    let mj: serde_json::Value = m.json().await.unwrap();
    assert_eq!(mj["ok"], true);
    assert!(mj.get("requests_total").is_some());
    assert!(mj.get("sidecar").is_some());
    assert!(mj.get("p95_latency_ms").is_some());
}

//! 层3 API 服务层 · 企业级分步验证（L5）
//!
//! 用 `reqwest` 真正启动 HTTP 服务并驱动 REST 契约全链路：
//! 提交需求 → 查询拓扑 → 冻结资产 → 检索知识库 → 重跑正则化。
//! 覆盖 `gen/c5.rs` 定义的所有端点。

use mox_flow_primiflow_svc::persistence::Persistence;
use mox_flow_primiflow_svc::server::{new_state, spawn_serve};
use serde_json::Value;

/// 启动服务并返回 (client, base_url)
async fn boot() -> (reqwest::Client, String) {
    let out_dir = std::env::temp_dir().join(format!("pf_api_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&out_dir).ok();
    let state = new_state(out_dir, Persistence::memory());
    let addr = spawn_serve(state, "127.0.0.1:0").await.unwrap();
    let base = format!("http://{addr}");
    // 等待服务就绪（最多 ~1s）
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client.get(&base).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    (client, base)
}

#[tokio::test]
async fn l5_create_project_runs_full_loop() {
    let (c, base) = boot().await;
    let resp = c
        .post(format!("{base}/api/projects"))
        .json(&serde_json::json!({"name":"经营分析","description":"每天抓取销售数据，清洗对账后生成图表报告。对接 PostgreSQL。"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: Value = resp.json().await.unwrap();
    assert!(v["id"].is_string());
    assert!(v["topology_id"].is_string());
    assert!(v["report"]["conserved"].is_boolean());
    assert!(v["report"]["acyclic"].is_boolean());
}

#[tokio::test]
async fn l5_get_topology_returns_mermaid() {
    let (c, base) = boot().await;
    let r = c
        .post(format!("{base}/api/projects"))
        .json(&serde_json::json!({"name":"工单聚类","description":"抓取工单，文本向量化，聚类分析，生成图表报告。"}))
        .send()
        .await
        .unwrap();
    let v: Value = r.json().await.unwrap();
    let id = v["topology_id"].as_str().unwrap();
    let topo = c
        .get(format!("{base}/api/topologies/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(topo.status(), 200);
    let md = topo.text().await.unwrap();
    assert!(
        md.contains("graph") || md.contains("flowchart"),
        "拓扑应为 Mermaid 图：{md}"
    );
}

#[tokio::test]
async fn l5_freeze_increments_kb_assets() {
    let (c, base) = boot().await;
    let r = c
        .post(format!("{base}/api/projects"))
        .json(&serde_json::json!({"name":"周报","description":"抓取销售数据，生成图表报告。"}))
        .send()
        .await
        .unwrap();
    let v: Value = r.json().await.unwrap();
    let id = v["id"].as_str().unwrap();
    let f = c
        .post(format!("{base}/api/topologies/{id}/freeze"))
        .send()
        .await
        .unwrap();
    assert_eq!(f.status(), 200);
    let fj: Value = f.json().await.unwrap();
    assert!(
        fj["kb_assets"].as_u64().unwrap() >= 1,
        "冻结后应至少沉淀 1 个资产"
    );
}

#[tokio::test]
async fn l5_assets_search_filters() {
    let (c, base) = boot().await;
    c.post(format!("{base}/api/projects"))
        .json(&serde_json::json!({"name":"库存预警","description":"抓取销售数据，库存核算，生成图表报告。"}))
        .send()
        .await
        .unwrap();
    let a = c
        .get(format!("{base}/api/assets?q=report"))
        .send()
        .await
        .unwrap();
    assert_eq!(a.status(), 200);
    let aj: Value = a.json().await.unwrap();
    assert!(aj["total"].as_u64().unwrap() >= 1, "知识库应有资产");
}

#[tokio::test]
async fn l5_regularize_reruns() {
    let (c, base) = boot().await;
    let r = c
        .post(format!("{base}/api/projects"))
        .json(&serde_json::json!({"name":"风控","description":"接入流数据，特征计算，模型推理，告警下发。"}))
        .send()
        .await
        .unwrap();
    let v: Value = r.json().await.unwrap();
    let id = v["id"].as_str().unwrap();
    let rr = c
        .post(format!("{base}/api/topologies/{id}/regularize"))
        .send()
        .await
        .unwrap();
    assert_eq!(rr.status(), 200);
    let rj: Value = rr.json().await.unwrap();
    assert!(rj["report"]["conserved"].is_boolean());
}

#[tokio::test]
async fn l5_list_projects_after_create() {
    let (c, base) = boot().await;
    c.post(format!("{base}/api/projects"))
        .json(&serde_json::json!({"name":"经营分析","description":"每天抓取销售数据，清洗对账后生成图表报告。对接 PostgreSQL。"}))
        .send()
        .await
        .unwrap();
    let r = c.get(format!("{base}/api/projects")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let v: Value = r.json().await.unwrap();
    assert!(v["total"].as_u64().unwrap() >= 1, "应至少列出 1 个项目");
    assert!(!v["projects"].as_array().unwrap().is_empty());
    let p0 = &v["projects"][0];
    assert!(p0["id"].is_string());
    assert!(p0["kappa"].is_number());
    assert!(p0["conserved"].is_boolean());
}

#[tokio::test]
async fn l5_get_project_detail() {
    let (c, base) = boot().await;
    let r = c
        .post(format!("{base}/api/projects"))
        .json(&serde_json::json!({"name":"工单聚类","description":"抓取工单，文本向量化，聚类分析，生成图表报告。"}))
        .send()
        .await
        .unwrap();
    let v: Value = r.json().await.unwrap();
    let id = v["id"].as_str().unwrap();
    let d = c
        .get(format!("{base}/api/projects/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(d.status(), 200);
    let dj: Value = d.json().await.unwrap();
    assert_eq!(dj["project"]["id"].as_str().unwrap(), id);
    assert!(dj["project"]["kappa"].is_number());
    assert!(dj["project"]["q_after"].is_number());
    // 不存在的项目应 404
    let miss = c
        .get(format!("{base}/api/projects/nope"))
        .send()
        .await
        .unwrap();
    assert_eq!(miss.status(), 404);
}

/// 跨重启复现 Q：第一次进程把资产落盘到 SQLite，第二次进程启动时重放，
/// 引擎应直接继承历史资产（无需从零探索），且项目清单可恢复。
#[tokio::test]
async fn l5_replay_across_restart_continues_q() {
    let dir = std::env::temp_dir().join(format!("pf_replay_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).ok();
    let db = dir.join("primiflow.db").to_string_lossy().to_string();
    let _ = std::fs::remove_file(&db);
    let client = reqwest::Client::new();

    // 第一次启动：SQLite 落盘 + 跑一个需求，固化资产
    let out1 = dir.join("run1");
    std::fs::create_dir_all(&out1).ok();
    let state1 = new_state(out1, Persistence::sqlite(&db).unwrap());
    state1.replay_from_store().await;
    let addr1 = spawn_serve(state1, "127.0.0.1:0").await.unwrap();
    let base1 = format!("http://{addr1}");
    for _ in 0..50 {
        if client.get(&base1).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let r = client
        .post(format!("{base1}/api/projects"))
        .json(&serde_json::json!({"name":"零售预警","description":"每天抓取销售数据，清洗对账后生成图表报告。对接 PostgreSQL。"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    // 第二次启动：同一 SQLite 文件，启动重放应恢复资产
    let out2 = dir.join("run2");
    std::fs::create_dir_all(&out2).ok();
    let state2 = new_state(out2, Persistence::sqlite(&db).unwrap());
    state2.replay_from_store().await;
    let addr2 = spawn_serve(state2, "127.0.0.1:0").await.unwrap();
    let base2 = format!("http://{addr2}");
    for _ in 0..50 {
        if client.get(&base2).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let a = client
        .get(format!("{base2}/api/assets"))
        .send()
        .await
        .unwrap();
    assert_eq!(a.status(), 200);
    let aj: Value = a.json().await.unwrap();
    assert!(
        aj["total"].as_u64().unwrap() >= 1,
        "重启后应通过重放恢复资产 Q"
    );
    let p = client
        .get(format!("{base2}/api/projects"))
        .send()
        .await
        .unwrap();
    let pj: Value = p.json().await.unwrap();
    assert!(pj["total"].as_u64().unwrap() >= 1, "重启后应恢复项目记录");

    let _ = std::fs::remove_file(&db);
}

#[tokio::test]
async fn l5_health_returns_status_ok() {
    let (c, base) = boot().await;
    let r = c.get(format!("{base}/api/health")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let v: Value = r.json().await.unwrap();
    assert_eq!(v["status"], "ok");
    assert_eq!(v["service"], "primiflow");
    assert!(v["version"].is_string());
    assert!(v["q"].is_number());
    assert!(v["kb_assets"].is_number());
    assert!(v["projects_total"].is_number());
}

#[tokio::test]
async fn l5_cors_headers_present_on_get() {
    let (c, base) = boot().await;
    let r = c
        .get(format!("{base}/api/health"))
        .header("origin", "http://example.com")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(
        r.headers().get("access-control-allow-origin").unwrap(),
        "*",
        "普通 GET 响应应带 CORS 头"
    );
}

#[tokio::test]
async fn l5_options_preflight_returns_204() {
    let (c, base) = boot().await;
    let r = c
        .request(reqwest::Method::OPTIONS, format!("{base}/api/projects"))
        .header("origin", "http://example.com")
        .header("access-control-request-method", "POST")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204, "预检应直接 204");
    assert_eq!(r.headers().get("access-control-allow-origin").unwrap(), "*");
    assert!(r
        .headers()
        .get("access-control-allow-methods")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("POST"));
}

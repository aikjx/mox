// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 知识库 HTTP 集成测试：直接驱动 `build_kb_router()`，端到端验证全部 `/kb/*` 接口。
//!
//! 对齐 legacy API 面：成功 `{ "success": true, "data": ... }`，失败 `{ "success": false, ... }`。
//! 存储走真实 FS 后端（tempdir），分析走本地引擎（无 LLM 时自动降级）。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use mox_kb_svc::handlers::build_kb_router_with_dir;
use serde_json::Value;
use tower::ServiceExt;

/// 独立临时数据目录（避免污染默认 ./data/store；生命周期由 tempfile 持有，测试内有效）
fn temp_data_dir() -> std::path::PathBuf {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().to_path_buf();
    std::mem::forget(dir);
    path
}

fn app() -> axum::Router {
    build_kb_router_with_dir(temp_data_dir())
}

/// 发起请求并解析 JSON
async fn call(router: &axum::Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let builder = Request::builder().method(method).uri(uri);
    let req = match body {
        Some(b) => builder
            .header("content-type", "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let value: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

#[tokio::test]
async fn kb_full_lifecycle() {
    let router = app();

    // 1. 创建文档
    let (status, body) = call(
        &router,
        "POST",
        "/kb/documents",
        Some(serde_json::json!({
            "title": "云盘混合架构",
            "content": "内容寻址去重配合纠删码与分片技术，S3 协议层自研 SigV4 客户端。\n图谱挂图将文档落为节点边。\n内容寻址去重提升云盘效率。",
            "category": "cat-tech",
            "tags": ["云盘", "去重"],
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["success"], true, "{body}");
    let doc_id = body["data"]["id"].as_str().expect("应返回 id").to_string();
    let doc = body["data"]["document"].clone();
    assert_eq!(doc["title"], "云盘混合架构");
    assert_eq!(doc["current_version"], "v1");

    // 2. 读取文档
    let (status, body) = call(&router, "GET", &format!("/kb/documents/{doc_id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["title"], "云盘混合架构");

    // 3. 更新文档
    let (status, body) = call(
        &router,
        "PUT",
        &format!("/kb/documents/{doc_id}"),
        Some(serde_json::json!({ "title": "云盘混合架构 v2" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["document"]["title"], "云盘混合架构 v2");

    // 4. 分析（专家联盟）
    let (status, body) = call(&router, "POST", &format!("/kb/documents/{doc_id}/analyze"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let data = &body["data"];
    assert_eq!(data["status"], "analyzed", "{body}");
    assert!(!data["entities"].as_array().unwrap().is_empty(), "应抽取实体: {body}");
    assert!(!data["summary"].as_str().unwrap().is_empty());
    assert!(data["expert_score"].as_f64().unwrap() >= 0.0);
    assert!(data["expert_score"].as_f64().unwrap() <= 1.0);
    let entity_count = data["entities"].as_array().unwrap().len();

    // 5. 图谱挂图
    let (status, body) = call(&router, "POST", &format!("/kb/documents/{doc_id}/graph-link"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let data = &body["data"];
    assert_eq!(data["status"], "linked", "{body}");
    assert!(data["graph_total_nodes"].as_u64().unwrap() >= 3);
    assert!(data["graph_total_edges"].as_u64().unwrap() >= 1);

    // 6. 检索（标题命中最高）
    let (status, body) = call(
        &router,
        "POST",
        "/kb/search",
        Some(serde_json::json!({ "query": "云盘", "limit": 10 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["total"].as_u64().unwrap(), 1, "{body}");
    assert_eq!(body["data"]["results"][0]["title"], "云盘混合架构 v2");
    // 图谱命中非空（挂图过）
    assert!(!body["data"]["graph_hits"].as_array().unwrap().is_empty());

    // 7. 版本：创建 v2 快照 → 对比 → 回滚
    let (status, body) = call(
        &router,
        "POST",
        &format!("/kb/documents/{doc_id}/versions"),
        Some(serde_json::json!({ "note": "初版评审" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = call(
        &router,
        "POST",
        &format!("/kb/documents/{doc_id}/versions"),
        Some(serde_json::json!({ "note": "二版修订" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = call(&router, "GET", &format!("/kb/documents/{doc_id}/versions"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["versions"].as_array().unwrap().len() >= 3, "{body}");

    let (status, body) = call(
        &router,
        "POST",
        &format!("/kb/documents/{doc_id}/versions/compare"),
        Some(serde_json::json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["data"]["diff"].is_object(), "{body}");

    let (status, body) = call(
        &router,
        "POST",
        &format!("/kb/documents/{doc_id}/versions/revert"),
        Some(serde_json::json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // 8. 实体端点
    let (status, body) = call(&router, "GET", &format!("/kb/documents/{doc_id}/entities"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["entities"].as_array().unwrap().len(), entity_count);

    // 9. 统计
    let (status, body) = call(&router, "GET", "/kb/stats", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["documents"], 1);
    assert!(body["data"]["graph_nodes"].as_u64().unwrap() >= 3);

    // 10. 分类 / 标签
    let (status, body) = call(&router, "GET", "/kb/categories", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 3);
    let (status, body) = call(&router, "GET", "/kb/tags", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body["data"].as_array().unwrap().is_empty(), "{body}");

    // 11. 删除 → 404
    let (status, _) = call(&router, "DELETE", &format!("/kb/documents/{doc_id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(&router, "GET", &format!("/kb/documents/{doc_id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // 删除后图谱反挂图：文档节点消失
    let (status, body) = call(
        &router,
        "POST",
        "/kb/search",
        Some(serde_json::json!({ "query": "云盘", "limit": 10 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"].as_u64().unwrap(), 0, "删除后不应命中: {body}");
}

#[tokio::test]
async fn kb_batch_analyze_and_errors() {
    let router = app();

    // 批量分析（全量）
    let (status, body) = call(&router, "POST", "/kb/documents", Some(serde_json::json!({ "title": "D1", "content": "图谱检索与知识库分析" }))).await;
    assert_eq!(status, StatusCode::OK);
    let d1 = body["data"]["id"].as_str().unwrap().to_string();
    let (status, body) = call(&router, "POST", "/kb/documents", Some(serde_json::json!({ "title": "D2", "content": "存储分层与快照恢复" }))).await;
    assert_eq!(status, StatusCode::OK);
    let d2 = body["data"]["id"].as_str().unwrap().to_string();

    let (status, body) = call(&router, "POST", "/kb/batch-analyze", Some(serde_json::json!({ "ids": [d1, d2] }))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["analyzed"], 2, "{body}");

    // 不存在的文档 → 404
    let (status, body) = call(&router, "GET", "/kb/documents/not-exist", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["success"], false);

    // 删除不存在 → 404
    let (status, _) = call(&router, "DELETE", "/kb/documents/not-exist", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn kb_graph_unlink() {
    let router = app();

    let (status, body) = call(&router, "POST", "/kb/documents", Some(serde_json::json!({ "title": "U1", "content": "图谱挂图与反挂图语义关系" }))).await;
    assert_eq!(status, StatusCode::OK);
    let id = body["data"]["id"].as_str().unwrap().to_string();

    let (status, body) = call(&router, "POST", &format!("/kb/documents/{id}/graph-link"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let nodes_before = body["data"]["graph_total_nodes"].as_u64().unwrap();

    // DELETE 反挂图（前端 kbGraphUnlink 语义）
    let (status, body) = call(&router, "DELETE", &format!("/kb/documents/{id}/graph-link"), Some(serde_json::json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["status"], "unlinked");
    assert!(body["data"]["nodes_removed"].as_u64().unwrap() >= 1, "{body}");

    // 反挂图后图谱检索不再命中文档节点
    let (status, body) = call(&router, "POST", "/kb/search", Some(serde_json::json!({ "query": "图谱", "limit": 10 }))).await;
    assert_eq!(status, StatusCode::OK);
    let graph_hits = body["data"]["graph_hits"].as_array().unwrap();
    assert!(graph_hits.iter().all(|n| n["id"].as_str().unwrap_or("") != format!("kb-{id}")), "文档节点应已移除: {body}");
    // 图规模下降
    let (status, body) = call(&router, "GET", "/kb/stats", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["graph_nodes"].as_u64().unwrap() < nodes_before, "{body}");
}



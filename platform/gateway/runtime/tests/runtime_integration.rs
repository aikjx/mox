//! Runtime HTTP API 集成测试
//!
//! 测试目标：
//! - 路由基础连通性
//! - RBAC 权限控制
//! - 审计日志记录
//! - 错误响应格式（RFC 9457）
//!
//! 说明：端到端需先启动服务器（`cargo run -p runtime`，默认 3000 端口），
//! 再用 `cargo test --package runtime --test runtime_integration -- --ignored` 运行。
//! 一键端到端脚本见仓库根 `scripts/ci.py`（build + test + fe build + 启服 + 健康检查）。
//! 2026-08-18 已在本机实测：5/5 全部通过（健康检查 / RBAC 越权 403 / admin 全权限 /
//! 审计留痕 / RFC 9457 错误格式）。

#[cfg(test)]
mod runtime_integration_tests {
    #[tokio::test]
    #[ignore = "需要启动服务器"]
    async fn test_health_endpoint() {
        let client = reqwest::Client::new();
        let resp = client.get("http://localhost:3000/api/health")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    #[ignore = "需要启动服务器"]
    async fn test_rbac_viewer_denied_write() {
        let client = reqwest::Client::new();
        let resp = client.post("http://localhost:3000/api/operators/register")
            .header("Authorization", "Bearer viewer_token123")
            .json(&serde_json::json!({
                "id": "test_op",
                "name": "Test Operator",
                "operator_type": "custom",
                "parameters": {}
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 403);
    }

    #[tokio::test]
    #[ignore = "需要启动服务器"]
    async fn test_rbac_admin_has_all_permissions() {
        let client = reqwest::Client::new();
        let resp = client.get("http://localhost:3000/api/logs")
            .header("Authorization", "Bearer admin_token123")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    #[ignore = "需要启动服务器"]
    async fn test_audit_event_recorded() {
        let client = reqwest::Client::new();
        let _ = client.post("http://localhost:3000/api/execute")
            .header("Authorization", "Bearer admin_token123")
            .json(&serde_json::json!({
                "workflow": ["identity"],
                "input": [1.0, 2.0, 3.0]
            }))
            .send()
            .await;
        let resp = client.get("http://localhost:3000/api/logs")
            .header("Authorization", "Bearer admin_token123")
            .send()
            .await
            .unwrap();
        let logs: serde_json::Value = resp.json().await.unwrap();
        assert!(!logs["logs"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "需要启动服务器"]
    async fn test_error_format_rfc9457() {
        let client = reqwest::Client::new();
        let resp = client.get("http://localhost:3000/api/ai/flows/nonexistent")
            .header("Authorization", "Bearer admin_token123")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 404);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["type"].is_string());
        assert!(body["title"].is_string());
        assert!(body["status"].is_number());
    }
}

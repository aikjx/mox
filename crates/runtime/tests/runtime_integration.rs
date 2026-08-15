//! Runtime HTTP API 集成测试
//!
//! 测试目标：
//! - 55 路由基础连通性
//! - RBAC 权限控制
//! - 审计日志记录
//! - 错误响应格式（RFC 9457）

#[cfg(test)]
mod runtime_integration_tests {
    // 注意：此测试需要运行时服务器启动后才能执行
    // 使用 `cargo test --package runtime --test runtime_integration -- --ignored` 运行

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
        
        // 使用 viewer token 尝试写操作
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
        
        // Viewer 无写权限，应返回 403
        assert_eq!(resp.status(), 403);
    }

    #[tokio::test]
    #[ignore = "需要启动服务器"]
    async fn test_rbac_admin_has_all_permissions() {
        let client = reqwest::Client::new();
        
        // Admin 可以查看审计日志
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
        
        // 执行操作
        let _ = client.post("http://localhost:3000/api/execute")
            .header("Authorization", "Bearer admin_token123")
            .json(&serde_json::json!({
                "operators": ["identity"],
                "input": [1.0, 2.0, 3.0]
            }))
            .send()
            .await;
        
        // 查看审计日志
        let resp = client.get("http://localhost:3000/api/logs")
            .header("Authorization", "Bearer admin_token123")
            .send()
            .await
            .unwrap();
        
        let logs: serde_json::Value = resp.json().await.unwrap();
        assert!(logs["logs"].as_array().unwrap().len() > 0);
    }

    #[tokio::test]
    #[ignore = "需要启动服务器"]
    async fn test_error_format_rfc9457() {
        let client = reqwest::Client::new();
        
        // 不存在的节点
        let resp = client.get("http://localhost:3000/api/ai/flows/nonexistent")
            .header("Authorization", "Bearer admin_token123")
            .send()
            .await
            .unwrap();
        
        // 应返回 404 + Problem+JSON
        assert_eq!(resp.status(), 404);
        
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["type"].is_string());
        assert!(body["title"].is_string());
        assert!(body["status"].is_number());
    }
}

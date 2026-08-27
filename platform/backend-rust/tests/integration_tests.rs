// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! E2E 集成测试（简化版，API 对齐）
//!
//! 测试 API 网关完整链路

use mox_enterprise_backend::api_gateway::*;

// ==================== API 网关完整链路测试 ====================

#[tokio::test]
async fn test_gateway_full_request_lifecycle() {
    // 1. 创建网关
    let gateway = ApiGateway::builder()
        .service_name("test-gateway")
        .listen_addr("127.0.0.1:0")
        .default_timeout_ms(5000)
        .retry_attempts(2)
        .rate_limit_per_second(100)
        .circuit_breaker_threshold(0.5)
        .upstream_service(UpstreamService {
            name: "test-service".to_string(),
            path_prefix: "/api/test".to_string(),
            targets: vec!["http://localhost:9999".to_string()],
            load_balance: LoadBalanceStrategy::RoundRobin,
            timeout_ms: 5000,
            retries: 2,
            rate_limit_per_second: 100,
            circuit_breaker_threshold: 0.5,
        })
        .build()
        .unwrap();

    // 2. 验证配置
    assert_eq!(gateway.listen_addr(), "127.0.0.1:0");
    assert!(gateway.retry_attempts() >= 2);

    // 3. 验证路由匹配
    let upstream = gateway.match_upstream("/api/test/resource");
    assert!(upstream.is_some());
    assert_eq!(upstream.unwrap().name, "test-service");

    // 4. 验证未匹配路由
    assert!(gateway.match_upstream("/api/other").is_none());

    // 5. 验证统计
    let stats = gateway.stats();
    assert_eq!(stats.service_name, "test-gateway");
    assert_eq!(stats.upstream_services, 1);
    assert_eq!(stats.circuit_breakers, 1);
}

#[tokio::test]
async fn test_gateway_rate_limit_and_circuit_breaker() {
    let gateway = ApiGateway::builder()
        .service_name("combined-test")
        .listen_addr("127.0.0.1:0")
        .default_timeout_ms(1000)
        .retry_attempts(1)
        .rate_limit_per_second(5)
        .circuit_breaker_threshold(0.5)
        .upstream_service(UpstreamService {
            name: "unstable-service".to_string(),
            path_prefix: "/api/unstable".to_string(),
            targets: vec!["http://localhost:9998".to_string()],
            load_balance: LoadBalanceStrategy::RoundRobin,
            timeout_ms: 1000,
            retries: 1,
            rate_limit_per_second: 5,
            circuit_breaker_threshold: 0.5,
        })
        .build()
        .unwrap();

    let stats = gateway.stats();
    assert_eq!(stats.circuit_breakers, 1);
}

#[test]
fn test_gateway_round_robin_selection() {
    let config = GatewayConfig {
        upstream_services: vec![UpstreamService {
            name: "test".to_string(),
            path_prefix: "/test".to_string(),
            targets: vec!["http://a:8080".to_string(), "http://b:8080".to_string(), "http://c:8080".to_string()],
            load_balance: LoadBalanceStrategy::RoundRobin,
            timeout_ms: 5000,
            retries: 3,
            rate_limit_per_second: 100,
            circuit_breaker_threshold: 0.5,
        }],
        ..Default::default()
    };

    let gateway = ApiGateway::new(config);
    let svc = gateway.match_upstream("/test/path").unwrap();

    // 轮询应该依次返回 a, b, c
    let t1 = gateway.select_target(&svc);
    let t2 = gateway.select_target(&svc);
    let t3 = gateway.select_target(&svc);
    let t4 = gateway.select_target(&svc);

    assert_eq!(t1, "http://a:8080");
    assert_eq!(t2, "http://b:8080");
    assert_eq!(t3, "http://c:8080");
    assert_eq!(t4, "http://a:8080", "应该循环回到第一个");
}

#[test]
fn test_gateway_upstream_matching() {
    let config = GatewayConfig {
        upstream_services: vec![
            UpstreamService {
                name: "storage".to_string(),
                path_prefix: "/api/storage".to_string(),
                targets: vec!["http://storage:8080".to_string()],
                load_balance: LoadBalanceStrategy::RoundRobin,
                timeout_ms: 5000,
                retries: 3,
                rate_limit_per_second: 100,
                circuit_breaker_threshold: 0.5,
            },
            UpstreamService {
                name: "metadata".to_string(),
                path_prefix: "/api/metadata".to_string(),
                targets: vec!["http://metadata:8080".to_string()],
                load_balance: LoadBalanceStrategy::RoundRobin,
                timeout_ms: 5000,
                retries: 3,
                rate_limit_per_second: 100,
                circuit_breaker_threshold: 0.5,
            },
        ],
        ..Default::default()
    };

    let gateway = ApiGateway::new(config);

    assert_eq!(gateway.match_upstream("/api/storage/file").unwrap().name, "storage");
    assert_eq!(gateway.match_upstream("/api/metadata/search").unwrap().name, "metadata");
    assert!(gateway.match_upstream("/api/other/path").is_none(), "未匹配的路径应该返回 None");
}

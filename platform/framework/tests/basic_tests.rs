// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! mox-framework 基础测试套件

use mox_framework::*;
use mox_framework::error::*;
use mox_framework::config::*;
use mox_framework::tenant::*;
use mox_framework::health::*;
use mox_framework::auth::*;
use mox_framework::resilience::*;
use mox_framework::metrics::*;
use mox_framework::tracing::*;
use std::time::Duration;

// ============================================================
// 1. 框架根模块测试
// ============================================================

#[test]
fn test_framework_constants() {
    assert_eq!(NAME, "mox-framework");
    assert!(!VERSION.is_empty(), "VERSION should not be empty");
}

#[test]
fn test_framework_result_type() {
    // 验证 FrameworkResult 类型别名正常工作
    let ok_result: FrameworkResult<i32> = Ok(42);
    assert_eq!(ok_result.unwrap(), 42);

    let err_result: FrameworkResult<i32> = Err(FrameworkError::validation("test error"));
    assert!(err_result.is_err());
}

#[test]
fn test_reexported_types() {
    // 验证重导出类型可用
    let _config = FrameworkConfig::default();
    let _err = FrameworkError::new(100002, "test");
    let _server = FrameworkServer::new("test-service");
    let _tenant = TenantContext::system();
}

// ============================================================
// 2. 配置模块测试
// ============================================================

#[test]
fn test_config_default_construction() {
    let config = FrameworkConfig::default();
    assert_eq!(config.service_name, "mox-service");
    assert_eq!(config.service_version, "1.0.0");
    assert_eq!(config.listen_addr, "0.0.0.0:8080");
    assert_eq!(config.grpc_addr, "0.0.0.0:50051");
    assert_eq!(config.log_level, "info");
    assert_eq!(config.log_format, "json");
    assert_eq!(config.environment, "dev");
    assert_eq!(config.tenant_mode, "logical");
}

#[test]
fn test_auth_config_default() {
    let auth = AuthConfig::default();
    assert_eq!(auth.jwt_secret, "change-me-in-production");
    assert_eq!(auth.jwt_expiry_secs, 86400);
    assert!(auth.enabled);
}

#[test]
fn test_resilience_config_default() {
    let resilience = ResilienceConfig::default();
    assert_eq!(resilience.timeout_secs, 30);
    assert_eq!(resilience.max_retries, 3);
    assert_eq!(resilience.rate_limit_per_sec, 1000);
    assert!((resilience.circuit_breaker_threshold - 0.5).abs() < f64::EPSILON);
}

#[test]
fn test_observability_config_default() {
    let obs = ObservabilityConfig::default();
    assert!(obs.metrics_enabled);
    assert!(obs.tracing_enabled);
    assert!(obs.health_enabled);
    assert!(obs.otel_endpoint.is_none());
}

#[test]
fn test_config_from_env() {
    // from_env 应该总是返回一个配置（使用默认值+环境变量覆盖）
    let config = FrameworkConfig::from_env();
    // 至少默认值应该被填充
    assert!(!config.service_name.is_empty());
    assert!(!config.service_version.is_empty());
}

#[test]
fn test_config_clone() {
    let config = FrameworkConfig::default();
    let cloned = config.clone();
    assert_eq!(config.service_name, cloned.service_name);
    assert_eq!(config.listen_addr, cloned.listen_addr);
}

// ============================================================
// 3. 错误模块测试
// ============================================================

#[test]
fn test_error_construction() {
    let err = FrameworkError::new(100002, "test error");
    assert_eq!(err.code, 100002);
    assert_eq!(err.message, "test error");
    assert_eq!(err.severity, Severity::Error);
    assert!(err.source_text.is_none());
    assert!(err.details.is_none());
}

#[test]
fn test_error_builder_methods() {
    let err = FrameworkError::new(200003, "internal error")
        .with_severity(Severity::Critical)
        .with_source("database")
        .with_details(serde_json::json!({"retry": true}));

    assert_eq!(err.severity, Severity::Critical);
    assert_eq!(err.source_text.as_deref(), Some("database"));
    assert!(err.details.is_some());
    assert_eq!(err.details.unwrap()["retry"], true);
}

#[test]
fn test_error_convenience_constructors() {
    let validation_err = FrameworkError::validation("bad input");
    assert_eq!(validation_err.code, 100002);
    assert_eq!(validation_err.severity, Severity::Warning);

    let unauthorized_err = FrameworkError::unauthorized("no token");
    assert_eq!(unauthorized_err.code, 110002);

    let forbidden_err = FrameworkError::forbidden("no access");
    assert_eq!(forbidden_err.code, 120002);

    let not_found_err = FrameworkError::not_found("resource missing");
    assert_eq!(not_found_err.code, 130002);

    let conflict_err = FrameworkError::conflict("duplicate");
    assert_eq!(conflict_err.code, 140002);

    let rate_limited_err = FrameworkError::rate_limited("too many");
    assert_eq!(rate_limited_err.code, 150002);

    let internal_err = FrameworkError::internal("crash");
    assert_eq!(internal_err.code, 200003);
    assert_eq!(internal_err.severity, Severity::Error);

    let timeout_err = FrameworkError::timeout("slow");
    assert_eq!(timeout_err.code, 230003);

    let unavailable_err = FrameworkError::unavailable("down");
    assert_eq!(unavailable_err.code, 220003);
    assert_eq!(unavailable_err.severity, Severity::Critical);
}

#[test]
fn test_error_http_status() {
    assert_eq!(FrameworkError::validation("x").http_status(), 400);
    assert_eq!(FrameworkError::unauthorized("x").http_status(), 401);
    assert_eq!(FrameworkError::forbidden("x").http_status(), 403);
    assert_eq!(FrameworkError::not_found("x").http_status(), 404);
    assert_eq!(FrameworkError::conflict("x").http_status(), 409);
    assert_eq!(FrameworkError::rate_limited("x").http_status(), 429);
    assert_eq!(FrameworkError::internal("x").http_status(), 500);
    assert_eq!(FrameworkError::unavailable("x").http_status(), 503);
    assert_eq!(FrameworkError::timeout("x").http_status(), 504);

    // 未知错误码默认 500
    assert_eq!(FrameworkError::new(999999, "x").http_status(), 500);
}

#[test]
fn test_severity_enum() {
    assert_eq!(Severity::Info as u8, 0);
    assert_eq!(Severity::Warning as u8, 1);
    assert_eq!(Severity::Error as u8, 2);
    assert_eq!(Severity::Critical as u8, 3);
}

#[test]
fn test_error_display() {
    let err = FrameworkError::new(100002, "bad input");
    let display = format!("{}", err);
    assert!(display.contains("100002"));
    assert!(display.contains("bad input"));
}

#[test]
fn test_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let fw_err: FrameworkError = io_err.into();
    assert_eq!(fw_err.code, 200003); // internal
    assert!(fw_err.message.contains("IO error"));
}

#[test]
fn test_error_from_json_error() {
    let json_err = serde_json::from_str::<i32>("not a number").unwrap_err();
    let fw_err: FrameworkError = json_err.into();
    assert_eq!(fw_err.code, 100002); // validation
    assert!(fw_err.message.contains("JSON error"));
}

// ============================================================
// 4. 租户模块测试
// ============================================================

#[test]
fn test_tenant_context_system() {
    let ctx = TenantContext::system();
    assert_eq!(ctx.tenant_id, "system");
    assert_eq!(ctx.tenant_name, "System");
    assert_eq!(ctx.mode, TenantMode::Logical);
    assert_eq!(ctx.plan, "enterprise");
}

#[test]
fn test_tenant_prefix() {
    let ctx = TenantContext::system();
    assert_eq!(ctx.prefix(), "system:");
    assert_eq!(ctx.with_prefix("mykey"), "system:mykey");
}

#[test]
fn test_tenant_strip_prefix() {
    let ctx = TenantContext::system();
    assert_eq!(ctx.strip_prefix("system:mykey"), "mykey");
    // 无前缀时返回原字符串
    assert_eq!(ctx.strip_prefix("other:mykey"), "other:mykey");
    assert_eq!(ctx.strip_prefix("plainkey"), "plainkey");
}

#[test]
fn test_tenant_mode_enum() {
    assert_ne!(TenantMode::None, TenantMode::Logical);
    assert_ne!(TenantMode::Logical, TenantMode::Schema);
    assert_ne!(TenantMode::Schema, TenantMode::Cluster);
}

#[test]
fn test_tenant_state_construction() {
    let state = TenantState::new(TenantMode::Logical, "default-tenant");
    assert_eq!(state.mode, TenantMode::Logical);
    assert_eq!(*state.default_tenant, "default-tenant");
}

// ============================================================
// 5. 健康检查模块测试
// ============================================================

#[tokio::test]
async fn test_health_checker_new() {
    let checker = HealthChecker::new("test-service", "1.0.0");
    let report = checker.report().await;
    assert_eq!(report.service, "test-service");
    assert_eq!(report.version, "1.0.0");
    assert_eq!(report.status, HealthStatus::Up);
    assert!(report.components.is_empty());
}

#[tokio::test]
async fn test_health_register_component() {
    let checker = HealthChecker::new("test-service", "1.0.0");
    checker.register_component("database").await;
    checker.register_component("cache").await;

    let report = checker.report().await;
    assert_eq!(report.components.len(), 2);
    assert_eq!(report.components[0].name, "database");
    assert_eq!(report.components[0].status, HealthStatus::Up);
    assert_eq!(report.components[1].name, "cache");
}

#[tokio::test]
async fn test_health_update_component() {
    let checker = HealthChecker::new("test-service", "1.0.0");
    checker.register_component("database").await;

    checker.update_component("database", HealthStatus::Degraded, Some("slow queries".into())).await;

    let report = checker.report().await;
    assert_eq!(report.status, HealthStatus::Degraded);
    assert_eq!(report.components[0].status, HealthStatus::Degraded);
    assert_eq!(report.components[0].message.as_deref(), Some("slow queries"));
}

#[tokio::test]
async fn test_health_status_down_propagation() {
    let checker = HealthChecker::new("test-service", "1.0.0");
    checker.register_component("database").await;
    checker.register_component("cache").await;

    checker.update_component("database", HealthStatus::Down, None).await;

    let report = checker.report().await;
    // 只要有一个 Down，整体状态就是 Down
    assert_eq!(report.status, HealthStatus::Down);
}

#[test]
fn test_health_status_enum() {
    assert_ne!(HealthStatus::Up, HealthStatus::Down);
    assert_ne!(HealthStatus::Up, HealthStatus::Degraded);
}

#[tokio::test]
async fn test_health_routes() {
    let checker = HealthChecker::new("test-service", "1.0.0");
    let _router = checker.routes();
    // 路由应该能成功构建，不 panic
}

// ============================================================
// 6. 认证模块测试
// ============================================================

#[test]
fn test_auth_state_construction() {
    let state = AuthState::new("my-secret", true);
    assert_eq!(*state.jwt_secret, "my-secret");
    assert!(state.enabled);

    let state_disabled = AuthState::new("my-secret", false);
    assert!(!state_disabled.enabled);
}

#[test]
fn test_jwt_token_roundtrip() {
    let secret = "test-secret-key-for-jwt";
    let token = generate_token(
        secret,
        "user123",
        "tenant456",
        vec!["admin".to_string(), "user".to_string()],
        vec!["read".to_string(), "write".to_string()],
        3600,
    ).expect("token generation should succeed");

    assert!(!token.is_empty());

    let claims = verify_token(secret, &token).expect("token verification should succeed");
    assert_eq!(claims.sub, "user123");
    assert_eq!(claims.tenant_id, "tenant456");
    assert_eq!(claims.roles, vec!["admin", "user"]);
    assert_eq!(claims.permissions, vec!["read", "write"]);
    assert!(claims.exp > claims.iat);
}

#[test]
fn test_jwt_token_wrong_secret() {
    let token = generate_token(
        "correct-secret",
        "user123",
        "tenant456",
        vec![],
        vec![],
        3600,
    ).unwrap();

    let result = verify_token("wrong-secret", &token);
    assert!(result.is_err());
}

#[test]
fn test_jwt_token_invalid() {
    let result = verify_token("secret", "not-a-valid-token");
    assert!(result.is_err());
}

#[test]
fn test_rbac_permission_check() {
    let claims = Claims {
        sub: "user1".into(),
        tenant_id: "t1".into(),
        roles: vec!["user".to_string()],
        permissions: vec!["read".to_string()],
        exp: 0,
        iat: 0,
    };

    assert!(has_permission(&claims, "read"));
    assert!(!has_permission(&claims, "write"));

    // admin 角色拥有所有权限
    let admin_claims = Claims {
        sub: "admin".into(),
        tenant_id: "t1".into(),
        roles: vec!["admin".to_string()],
        permissions: vec![],
        exp: 0,
        iat: 0,
    };
    assert!(has_permission(&admin_claims, "anything"));
}

#[test]
fn test_rbac_role_check() {
    let claims = Claims {
        sub: "user1".into(),
        tenant_id: "t1".into(),
        roles: vec!["editor".to_string()],
        permissions: vec![],
        exp: 0,
        iat: 0,
    };

    assert!(has_role(&claims, "editor"));
    assert!(!has_role(&claims, "viewer"));

    // admin 角色满足所有角色检查
    let admin_claims = Claims {
        sub: "admin".into(),
        tenant_id: "t1".into(),
        roles: vec!["admin".to_string()],
        permissions: vec![],
        exp: 0,
        iat: 0,
    };
    assert!(has_role(&admin_claims, "editor"));
}

// ============================================================
// 7. 弹性容错模块测试
// ============================================================

#[test]
fn test_circuit_breaker_initial_state() {
    let cb = CircuitBreaker::new(0.5, 5, Duration::from_secs(30));
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.allow());
}

#[test]
fn test_circuit_breaker_opens_on_failures() {
    let cb = CircuitBreaker::new(0.5, 4, Duration::from_secs(30));

    // 先记录一些成功，达到 min_requests
    cb.record_success();
    cb.record_success();

    // 然后记录失败，使失败率超过阈值
    cb.record_failure();
    cb.record_failure();
    cb.record_failure();

    // 此时应该已经熔断
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(!cb.allow());
}

#[test]
fn test_circuit_breaker_closed_allows_requests() {
    let cb = CircuitBreaker::new(0.99, 100, Duration::from_secs(30));
    // 在 Closed 状态应该总是允许
    for _ in 0..10 {
        assert!(cb.allow());
        cb.record_success();
    }
    assert_eq!(cb.state(), CircuitState::Closed);
}

#[test]
fn test_rate_limiter_acquire() {
    let rl = RateLimiter::new(5, 1);
    // 初始有 5 个令牌
    for i in 0..5 {
        assert!(rl.try_acquire(), "should acquire token {}", i);
    }
    // 第 6 个应该失败
    assert!(!rl.try_acquire(), "should be rate limited");
}

#[test]
fn test_rate_limiter_zero_capacity() {
    let rl = RateLimiter::new(0, 0);
    assert!(!rl.try_acquire());
}

#[tokio::test]
async fn test_bulkhead_acquire() {
    let bh = Bulkhead::new(3);
    assert_eq!(bh.max_concurrent(), 3);
    assert_eq!(bh.available(), 3);

    let permit = bh.acquire().await.unwrap();
    assert_eq!(bh.available(), 2);

    drop(permit);
    assert_eq!(bh.available(), 3);
}

#[test]
fn test_retry_policy_default() {
    let policy = RetryPolicy::default();
    assert_eq!(policy.max_retries, 3);
    assert_eq!(policy.base_delay, Duration::from_millis(100));
    assert_eq!(policy.max_delay, Duration::from_secs(5));
    assert!((policy.multiplier - 2.0).abs() < f64::EPSILON);
}

#[test]
fn test_retry_policy_delay_increases() {
    let policy = RetryPolicy::default();
    let delay0 = policy.delay_for(0);
    let delay1 = policy.delay_for(1);
    let delay2 = policy.delay_for(2);

    // 指数退避，延迟应该递增
    assert!(delay0 < delay1);
    assert!(delay1 < delay2);
    // 不超过 max_delay
    assert!(delay2 <= policy.max_delay);
}

#[test]
fn test_circuit_state_enum() {
    assert_ne!(CircuitState::Closed, CircuitState::Open);
    assert_ne!(CircuitState::Open, CircuitState::HalfOpen);
}

// ============================================================
// 8. 指标模块测试
// ============================================================

#[test]
fn test_metrics_collector_new() {
    let mc = MetricsCollector::new("test-service");
    assert_eq!(mc.service_name, "test-service");
}

#[test]
fn test_metrics_record_request() {
    let mc = MetricsCollector::new("test-service");

    mc.record_request("GET", "/api/test", 200, 50);
    mc.record_request("POST", "/api/data", 500, 100);

    // request_count 应该是 2
    assert_eq!(mc.request_count.load(std::sync::atomic::Ordering::Relaxed), 2);
    // error_count 应该是 1（500 错误）
    assert_eq!(mc.error_count.load(std::sync::atomic::Ordering::Relaxed), 1);
}

#[test]
fn test_histogram_bucket() {
    let bucket = HistogramBucket::default();
    bucket.record(10);
    bucket.record(20);
    bucket.record(30);

    assert_eq!(bucket.count.load(std::sync::atomic::Ordering::Relaxed), 3);
    assert_eq!(bucket.sum.load(std::sync::atomic::Ordering::Relaxed), 60);
}

#[test]
fn test_gauge_value() {
    let gauge = GaugeValue::default();
    gauge.set_f64(3.14);
    let val = gauge.get_f64();
    assert!((val - 3.14).abs() < 0.001);
}

#[test]
fn test_metrics_routes() {
    let mc = MetricsCollector::new("test-service");
    let _router = mc.routes();
    // 路由应该能成功构建
}

// ============================================================
// 9. 追踪模块测试
// ============================================================

#[test]
fn test_trace_context_new() {
    let ctx = TraceContext::new("my-service");
    assert_eq!(ctx.service_name, "my-service");
    assert!(!ctx.trace_id.is_empty());
    assert!(!ctx.span_id.is_empty());
    assert!(ctx.parent_span_id.is_none());

    // trace_id 应该是 32 字符（去掉了连字符的 UUID）
    assert_eq!(ctx.trace_id.len(), 32);
    // span_id 应该是 16 字符
    assert_eq!(ctx.span_id.len(), 16);
}

#[test]
fn test_trace_context_from_headers() {
    use axum::http::HeaderMap;

    let mut headers = HeaderMap::new();
    headers.insert("x-trace-id", "abc123def456".parse().unwrap());
    headers.insert("x-span-id", "span789".parse().unwrap());

    let ctx = TraceContext::from_headers(&headers, "my-service");
    assert_eq!(ctx.trace_id, "abc123def456");
    assert_eq!(ctx.parent_span_id.as_deref(), Some("span789"));
    assert_eq!(ctx.service_name, "my-service");
}

#[test]
fn test_trace_context_no_headers() {
    use axum::http::HeaderMap;

    let headers = HeaderMap::new();
    let ctx = TraceContext::from_headers(&headers, "my-service");
    // 没有 trace-id 头时应该生成新的
    assert!(!ctx.trace_id.is_empty());
    assert_eq!(ctx.trace_id.len(), 32);
    assert!(ctx.parent_span_id.is_none());
}

#[test]
fn test_trace_child_span() {
    let ctx = TraceContext::new("my-service");
    let _span = ctx.child_span("test-operation");
    // 应该能创建 span 而不 panic
}

// ============================================================
// 10. 服务器模块测试
// ============================================================

#[test]
fn test_framework_server_new() {
    let server = FrameworkServer::new("test-service");
    assert_eq!(server.config().service_name, "test-service");
    assert!(!server.config().service_version.is_empty());
}

#[test]
fn test_framework_server_from_config() {
    let config = FrameworkConfig::default();
    let server = FrameworkServer::from_config(config);
    assert_eq!(server.config().service_name, "mox-service");
}

#[test]
fn test_framework_server_with_router() {
    use axum::Router;

    let server = FrameworkServer::new("test-service");
    let app = Router::new();
    let server = server.with_router(app);
    // 应该能设置路由而不 panic
    let _config = server.config();
}

#[tokio::test]
async fn test_framework_server_health_component() {
    let server = FrameworkServer::new("test-service");
    server.register_health_component("database").await;

    let report = server.health().report().await;
    assert_eq!(report.components.len(), 1);
    assert_eq!(report.components[0].name, "database");
}

#[test]
fn test_framework_server_accessors() {
    let server = FrameworkServer::new("test-service");

    // 验证各个访问器
    let _config = server.config();
    let _health = server.health();
    let _metrics = server.metrics();
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Gateway configuration.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Whether authentication is enabled.
    pub enabled: bool,
    /// JWT secret key for token validation.
    pub jwt_secret: String,
    /// Token issuer to validate.
    pub token_issuer: String,
    /// Public paths that don't require authentication.
    pub public_paths: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            jwt_secret: "change-me-in-production".into(),
            token_issuer: "mox-platform".into(),
            public_paths: vec![
                "/health".into(),
                "/api/auth/login".into(),
                // 迁移期：/system/* 与 /security/* 暂为公开（前端 dev 令牌非合法 JWT，
                // 置于 protected 会直接 401 且阻断管理面板）。生产环境须移出 public_paths，
                // 待 auth 对接 IAM JWT 校验后回收为受保护路由。
                "/api/system".into(),
                "/api/security".into(),
                // 迁移期：/kg/v1/* 与 /ai/engine/* 暂为公开（知识图谱端点刚桥接真实算法，
                // 前端 dev 与 E2E 测试需直接访问；生产环境待 auth 对接 IAM JWT 后回收）。
                "/kg/v1".into(),
                "/ai/engine".into(),
                // 迁移期：/kb/* 云盘知识库暂为公开（mox-kb-svc 100% 自研，对齐 legacy API 面
                // 前端零改动；生产环境待 auth 对接 IAM JWT 后回收为受保护路由）。
                "/kb".into(),
            ],
        }
    }
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled.
    pub enabled: bool,
    /// Maximum requests per window per client.
    pub max_requests: u32,
    /// Rate limit window duration.
    pub window_secs: u64,
    /// Burst allowance above max_requests.
    pub burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_requests: 100,
            window_secs: 60,
            burst: 20,
        }
    }
}

/// Routing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Default upstream timeout in seconds.
    pub upstream_timeout_secs: u64,
    /// Whether to enable path-based routing.
    pub path_routing: bool,
    /// Whether to enable header-based routing.
    pub header_routing: bool,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            upstream_timeout_secs: 30,
            path_routing: true,
            header_routing: false,
        }
    }
}

/// Complete gateway configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Host to bind to.
    pub host: String,
    /// Port to listen on.
    pub port: u16,
    /// Request timeout.
    #[serde(skip, default = "default_timeout")]
    pub request_timeout: Duration,
    /// Authentication configuration.
    pub auth: AuthConfig,
    /// Rate limiting configuration.
    pub rate_limit: RateLimitConfig,
    /// Routing configuration.
    pub routing: RoutingConfig,
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 8080,
            request_timeout: Duration::from_secs(30),
            auth: AuthConfig::default(),
            rate_limit: RateLimitConfig::default(),
            routing: RoutingConfig::default(),
        }
    }
}

impl GatewayConfig {
    /// Load configuration from a JSON file.
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Get the bind address.
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

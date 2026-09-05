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
    /// 生产环境仅保留必要的认证端点和健康检查（≤3 个）。
    pub public_paths: Vec<String>,
    /// Dev mode: when true, allows frontend dev token to bypass strict JWT
    /// signature validation. Enabled by default in debug builds, must be
    /// explicitly set via MOX_DEV_MODE=1 in release.
    #[serde(default = "default_dev_mode")]
    pub dev_mode: bool,
}

fn default_dev_mode() -> bool {
    cfg!(debug_assertions)
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            // 占位值：启动时由 resolve_jwt_secret() 从 JWT_SECRET 环境变量覆盖。
            // release 模式下若环境变量未设置则 panic，禁止使用此默认值。
            jwt_secret: "change-me-in-production".into(),
            token_issuer: "mox-platform".into(),
            // 生产收紧：仅保留健康检查 + 登录 + 注册 三个公开路径。
            // /api/system、/api/security、/kg/v1、/ai/engine、/alliance/v1、/kb
            // 均已回收为受保护路由，须携带有效 JWT 访问。
            public_paths: vec![
                "/health".into(),
                "/api/auth/login".into(),
                "/api/auth/register".into(),
                // 管理面探针端点（k8s / docker liveness & readiness）允许匿名访问，
                // 其余 /actuator/*（env 配置泄露 / logs 日志泄露 / api 启停等）强制鉴权（见 lib.rs P0-2）。
                "/actuator/health".into(),
                "/actuator/info".into(),
            ],
            dev_mode: default_dev_mode(),
        }
    }
}

impl AuthConfig {
    /// Resolve JWT secret from environment variable.
    ///
    /// - `JWT_SECRET` 环境变量优先；
    /// - debug 模式下未设置时使用 dev 密钥并打印警告（前端 dev 令牌可继续使用）；
    /// - release 模式下未设置时 panic，禁止使用默认值启动。
    pub fn resolve_jwt_secret(&mut self) {
        match std::env::var("JWT_SECRET") {
            Ok(secret) if !secret.is_empty() => {
                self.jwt_secret = secret;
            }
            _ => {
                if cfg!(debug_assertions) {
                    eprintln!(
                        "[WARN] JWT_SECRET 环境变量未设置，debug 模式使用 dev 密钥。\n\
                         生产环境必须设置 JWT_SECRET，否则将拒绝启动。"
                    );
                    self.jwt_secret = "dev-only-insecure-secret".into();
                    self.dev_mode = true;
                } else {
                    panic!(
                        "JWT_SECRET 环境变量未设置！生产环境禁止使用默认密钥启动。\n\
                         请设置: export JWT_SECRET=<your-strong-secret>"
                    );
                }
            }
        }
        // MOX_DEV_MODE=1 可在 release 中显式开启 dev 模式（仅限临时调试）
        if std::env::var("MOX_DEV_MODE").unwrap_or_default() == "1" {
            self.dev_mode = true;
            eprintln!("[WARN] MOX_DEV_MODE=1 已启用，dev 令牌可绕过严格 JWT 校验。仅限临时调试！");
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
    /// CORS allowed origins. 从 CORS_ALLOWED_ORIGINS 环境变量读取（逗号分隔），
    /// 默认包含 localhost:3000 / localhost:5173 开发地址。
    #[serde(default = "default_cors_origins")]
    pub cors_allowed_origins: Vec<String>,
}

fn default_cors_origins() -> Vec<String> {
    vec![
        "http://localhost:3000".into(),
        "http://localhost:5173".into(),
        "http://127.0.0.1:3000".into(),
        "http://127.0.0.1:5173".into(),
    ]
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
            cors_allowed_origins: default_cors_origins(),
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

    /// Resolve CORS allowed origins from `CORS_ALLOWED_ORIGINS` env var (comma-separated).
    /// 生产环境应设置具体域名，禁止使用通配符 `*`。
    pub fn resolve_cors_origins(&mut self) {
        if let Ok(env_origins) = std::env::var("CORS_ALLOWED_ORIGINS") {
            let origins: Vec<String> = env_origins
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && s != "*")
                .collect();
            if !origins.is_empty() {
                self.cors_allowed_origins = origins;
            }
        }
    }
}

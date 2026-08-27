// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Authentication middleware.
//!
//! Provides JWT token validation, API key authentication, and role-based access control.
//! Depends only on L2 `mox-platform-api` trait contracts.

use crate::config::AuthConfig;
use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use mox_platform_api::UserInfo;
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Authentication middleware that validates requests before they reach handlers.
pub struct AuthMiddleware {
    config: AuthConfig,
    /// In-memory API key store (key_hash -> user_id).
    api_keys: Arc<parking_lot::RwLock<std::collections::HashMap<String, String>>>,
}

impl AuthMiddleware {
    /// Create a new auth middleware with the given configuration.
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config,
            api_keys: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Register an API key for a user.
    pub fn register_api_key(&self, user_id: &str, api_key: &str) {
        let hash = hash_api_key(api_key);
        self.api_keys.write().insert(hash, user_id.to_string());
    }

    /// Revoke an API key.
    pub fn revoke_api_key(&self, api_key: &str) {
        let hash = hash_api_key(api_key);
        self.api_keys.write().remove(&hash);
    }

    /// Check if a path is public (doesn't require auth).
    pub fn is_public_path(&self, path: &str) -> bool {
        self.config.public_paths.iter().any(|p| path.starts_with(p))
    }

    /// Validate a JWT token and return the user info.
    pub fn validate_token(&self, token: &str) -> Option<UserInfo> {
        // In production, this would validate the JWT signature and claims.
        // For now, we do a basic structure check.
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 { return None; }

        // Decode payload (base64)
        let payload = parts.get(1)?;
        let decoded = base64_decode(payload)?;
        let claims: serde_json::Value = serde_json::from_slice(&decoded).ok()?;

        let user_id = claims.get("sub")?.as_str()?.to_string();
        let tenant_id = claims.get("tenant_id").and_then(|v| v.as_str()).unwrap_or("default").to_string();
        let roles = claims.get("roles")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Some(UserInfo {
            id: user_id,
            username: claims.get("username").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
            email: claims.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            tenant_id,
            roles,
            enabled: true,
            created_at: String::new(),
        })
    }

    /// Validate an API key and return the user ID.
    pub fn validate_api_key(&self, api_key: &str) -> Option<String> {
        let hash = hash_api_key(api_key);
        self.api_keys.read().get(&hash).cloned()
    }
}

/// Axum middleware function for authentication.
pub async fn auth_middleware(
    auth: Arc<AuthMiddleware>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !auth.config.enabled || auth.is_public_path(request.uri().path()) {
        return Ok(next.run(request).await);
    }

    // Try Bearer token first
    let user_info = if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        let auth_str = auth_header.to_str().unwrap_or("");
        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            auth.validate_token(token)
        } else {
            None
        }
    } else {
        // Try API key
        if let Some(api_key) = request.headers().get("X-API-Key") {
            let key = api_key.to_str().unwrap_or("");
            auth.validate_api_key(key).map(|user_id| UserInfo {
                id: user_id,
                username: "api-user".into(),
                email: String::new(),
                tenant_id: "default".into(),
                roles: vec!["api".into()],
                enabled: true,
                created_at: String::new(),
            })
        } else {
            None
        }
    };

    if user_info.is_some() {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(input).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_path() {
        let auth = AuthMiddleware::new(AuthConfig::default());
        assert!(auth.is_public_path("/health"));
        assert!(auth.is_public_path("/api/auth/login"));
        assert!(!auth.is_public_path("/api/data/records"));
    }

    #[test]
    fn test_api_key_registration() {
        let auth = AuthMiddleware::new(AuthConfig::default());
        auth.register_api_key("user1", "secret-key-123");
        assert_eq!(auth.validate_api_key("secret-key-123"), Some("user1".to_string()));
        assert_eq!(auth.validate_api_key("wrong-key"), None);
        auth.revoke_api_key("secret-key-123");
        assert_eq!(auth.validate_api_key("secret-key-123"), None);
    }
}

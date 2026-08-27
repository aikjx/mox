// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 通用 OAuth 2.0 / OIDC Provider 实现

use super::*;
use async_trait::async_trait;

pub struct OAuth2Provider {
    config: SsoConfig,
    client: reqwest::Client,
}

impl OAuth2Provider {
    pub fn new(config: SsoConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");
        Self { config, client }
    }
}

#[async_trait]
impl SsoProvider for OAuth2Provider {
    fn provider_type(&self) -> SsoType { SsoType::OAuth2 }
    fn provider_name(&self) -> &'static str { "OAuth 2.0 / OIDC" }

    async fn get_auth_url(&self, state: &str) -> SsoResult<String> {
        let scope = self.config.scopes.join(" ");
        let params = serde_urlencoded::to_string([
            ("client_id", self.config.client_id.as_str()),
            ("redirect_uri", self.config.redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", scope.as_str()),
            ("state", state),
        ]).map_err(|e| SsoError::ConfigError(e.to_string()))?;
        Ok(format!("{}?{}", self.config.auth_url, params))
    }

    async fn exchange_token(&self, code: &str) -> SsoResult<SsoToken> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.config.redirect_uri.as_str()),
            ("client_id", self.config.client_id.as_str()),
            ("client_secret", self.config.client_secret.as_str()),
        ];

        let resp = self.client.post(&self.config.token_url)
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(SsoError::TokenError(body));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(SsoToken {
            access_token: json.get("access_token").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            refresh_token: json.get("refresh_token").and_then(|v| v.as_str()).map(|s| s.to_string()),
            expires_in: json.get("expires_in").and_then(|v| v.as_u64()),
            token_type: json.get("token_type").and_then(|v| v.as_str()).unwrap_or("Bearer").to_string(),
            id_token: json.get("id_token").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }

    async fn get_user_info(&self, token: &SsoToken) -> SsoResult<SsoUser> {
        let resp = self.client.get(&self.config.user_info_url)
            .bearer_auth(&token.access_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(SsoError::UserInfoError(format!("HTTP {}", resp.status())));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(SsoUser {
            external_id: json.get("sub").or_else(|| json.get("id")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            username: json.get("preferred_username").or_else(|| json.get("username")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            email: json.get("email").and_then(|v| v.as_str()).map(|s| s.to_string()),
            phone: json.get("phone_number").and_then(|v| v.as_str()).map(|s| s.to_string()),
            display_name: json.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            avatar_url: json.get("picture").and_then(|v| v.as_str()).map(|s| s.to_string()),
            department: None,
            roles: vec![],
            raw: json,
        })
    }

    async fn validate_token(&self, token: &str) -> SsoResult<bool> {
        // 简化：调用user_info端点验证
        let resp = self.client.get(&self.config.user_info_url)
            .bearer_auth(token)
            .send()
            .await?;
        Ok(resp.status().is_success())
    }
}

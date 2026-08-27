// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 企业微信 SSO Provider

use super::*;
use async_trait::async_trait;

pub struct WeComProvider {
    config: SsoConfig,
    client: reqwest::Client,
}

impl WeComProvider {
    pub fn new(config: SsoConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");
        Self { config, client }
    }

    fn corp_id(&self) -> &str {
        self.config.extra.get("corp_id").map(|s| s.as_str()).unwrap_or(&self.config.client_id)
    }

    fn agent_id(&self) -> &str {
        self.config.extra.get("agent_id").map(|s| s.as_str()).unwrap_or("")
    }
}

#[async_trait]
impl SsoProvider for WeComProvider {
    fn provider_type(&self) -> SsoType { SsoType::WeCom }
    fn provider_name(&self) -> &'static str { "企业微信" }

    async fn get_auth_url(&self, state: &str) -> SsoResult<String> {
        // 企业微信网页授权URL
        let params = serde_urlencoded::to_string([
            ("appid", self.corp_id()),
            ("redirect_uri", self.config.redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", "snsapi_base"),
            ("state", state),
            ("agentid", self.agent_id()),
        ]).map_err(|e| SsoError::ConfigError(e.to_string()))?;
        Ok(format!("{}?{}#wechat_redirect", self.config.auth_url, params))
    }

    async fn exchange_token(&self, code: &str) -> SsoResult<SsoToken> {
        // 企业微信：获取access_token（企业级，非用户级）
        let url = format!(
            "{}?corpid={}&corpsecret={}",
            self.config.token_url, self.corp_id(), self.config.client_secret
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(SsoError::TokenError(format!("HTTP {}", resp.status())));
        }
        let json: serde_json::Value = resp.json().await?;
        let access_token = json.get("access_token").and_then(|v| v.as_str()).unwrap_or("").to_string();

        // 用code获取用户userid
        let user_url = format!(
            "{}?access_token={}&code={}",
            self.config.extra.get("user_info_url").map(|s| s.as_str()).unwrap_or(""),
            access_token, code
        );
        let user_resp = self.client.get(&user_url).send().await?;
        let user_json: serde_json::Value = user_resp.json().await.unwrap_or_default();

        Ok(SsoToken {
            access_token,
            refresh_token: None,
            expires_in: json.get("expires_in").and_then(|v| v.as_u64()),
            token_type: "Bearer".into(),
            id_token: user_json.get("UserId").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }

    async fn get_user_info(&self, token: &SsoToken) -> SsoResult<SsoUser> {
        let user_id = token.id_token.as_deref().unwrap_or("");
        // 企业微信：读取成员详情
        let url = format!(
            "{}?access_token={}&userid={}",
            self.config.user_info_url, token.access_token, user_id
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(SsoError::UserInfoError(format!("HTTP {}", resp.status())));
        }
        let json: serde_json::Value = resp.json().await?;
        Ok(SsoUser {
            external_id: json.get("userid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            username: json.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            email: json.get("email").and_then(|v| v.as_str()).map(|s| s.to_string()),
            phone: json.get("mobile").and_then(|v| v.as_str()).map(|s| s.to_string()),
            display_name: json.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            avatar_url: json.get("avatar").and_then(|v| v.as_str()).map(|s| s.to_string()),
            department: json.get("department").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(",")),
            roles: vec![],
            raw: json,
        })
    }

    async fn validate_token(&self, _token: &str) -> SsoResult<bool> {
        Ok(true)
    }
}

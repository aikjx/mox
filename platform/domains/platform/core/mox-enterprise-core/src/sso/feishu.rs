//! 飞书 SSO Provider

use super::*;
use async_trait::async_trait;

pub struct FeishuProvider {
    config: SsoConfig,
    client: reqwest::Client,
}

impl FeishuProvider {
    pub fn new(config: SsoConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");
        Self { config, client }
    }

    fn app_id(&self) -> &str {
        self.config.extra.get("app_id").map(|s| s.as_str()).unwrap_or(&self.config.client_id)
    }
}

#[async_trait]
impl SsoProvider for FeishuProvider {
    fn provider_type(&self) -> SsoType { SsoType::Feishu }
    fn provider_name(&self) -> &'static str { "飞书" }

    async fn get_auth_url(&self, state: &str) -> SsoResult<String> {
        let params = serde_urlencoded::to_string([
            ("app_id", self.app_id()),
            ("redirect_uri", self.config.redirect_uri.as_str()),
            ("response_type", "code"),
            ("state", state),
        ]).map_err(|e| SsoError::ConfigError(e.to_string()))?;
        Ok(format!("{}?{}", self.config.auth_url, params))
    }

    async fn exchange_token(&self, code: &str) -> SsoResult<SsoToken> {
        // 飞书：先获取tenant_access_token，再用code获取用户access_token
        let tenant_body = serde_json::json!({
            "app_id": self.app_id(),
            "app_secret": self.config.client_secret,
        });
        let tenant_resp = self.client.post(&self.config.token_url)
            .json(&tenant_body)
            .send()
            .await?;
        let tenant_json: serde_json::Value = tenant_resp.json().await.unwrap_or_default();
        let tenant_token = tenant_json.get("tenant_access_token").and_then(|v| v.as_str()).unwrap_or("");

        // 用code获取用户信息
        let user_url = self.config.extra.get("user_token_url").map(|s| s.as_str()).unwrap_or("");
        let user_resp = self.client.post(user_url)
            .header("Authorization", format!("Bearer {}", tenant_token))
            .json(&serde_json::json!({ "grant_type": "authorization_code", "code": code }))
            .send()
            .await?;
        let user_json: serde_json::Value = user_resp.json().await.unwrap_or_default();

        Ok(SsoToken {
            access_token: user_json.get("data").and_then(|d| d.get("access_token")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            refresh_token: user_json.get("data").and_then(|d| d.get("refresh_token")).and_then(|v| v.as_str()).map(|s| s.to_string()),
            expires_in: user_json.get("data").and_then(|d| d.get("expires_in")).and_then(|v| v.as_u64()),
            token_type: "Bearer".into(),
            id_token: None,
        })
    }

    async fn get_user_info(&self, token: &SsoToken) -> SsoResult<SsoUser> {
        let resp = self.client.get(&self.config.user_info_url)
            .header("Authorization", format!("Bearer {}", token.access_token))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(SsoError::UserInfoError(format!("HTTP {}", resp.status())));
        }
        let json: serde_json::Value = resp.json().await?;
        let data = json.get("data").and_then(|d| d.get("user")).unwrap_or(&json);
        Ok(SsoUser {
            external_id: data.get("open_id").or_else(|| data.get("user_id")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            username: data.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            email: data.get("email").and_then(|v| v.as_str()).map(|s| s.to_string()),
            phone: data.get("mobile").and_then(|v| v.as_str()).map(|s| s.to_string()),
            display_name: data.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            avatar_url: data.get("avatar_url").and_then(|v| v.as_str()).map(|s| s.to_string()),
            department: None,
            roles: vec![],
            raw: json,
        })
    }

    async fn validate_token(&self, _token: &str) -> SsoResult<bool> {
        Ok(true)
    }
}

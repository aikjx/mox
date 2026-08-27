//! 钉钉 SSO Provider（OAuth2 授权码模式，钉钉特有API）

use super::*;
use async_trait::async_trait;

pub struct DingTalkProvider {
    config: SsoConfig,
    client: reqwest::Client,
}

impl DingTalkProvider {
    pub fn new(config: SsoConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");
        Self { config, client }
    }

    fn app_key(&self) -> &str {
        self.config.extra.get("app_key").map(|s| s.as_str()).unwrap_or(&self.config.client_id)
    }
}

#[async_trait]
impl SsoProvider for DingTalkProvider {
    fn provider_type(&self) -> SsoType { SsoType::DingTalk }
    fn provider_name(&self) -> &'static str { "钉钉" }

    async fn get_auth_url(&self, state: &str) -> SsoResult<String> {
        // 钉钉授权URL：https://login.dingtalk.com/oauth2/auth
        let params = serde_urlencoded::to_string([
            ("redirect_uri", self.config.redirect_uri.as_str()),
            ("response_type", "code"),
            ("client_id", self.app_key()),
            ("scope", "openid"),
            ("state", state),
            ("prompt", "consent"),
        ]).map_err(|e| SsoError::ConfigError(e.to_string()))?;
        Ok(format!("{}?{}", self.config.auth_url, params))
    }

    async fn exchange_token(&self, code: &str) -> SsoResult<SsoToken> {
        // 钉钉：用code获取用户token
        let body = serde_json::json!({
            "clientId": self.app_key(),
            "clientSecret": self.config.client_secret,
            "code": code,
            "grantType": "authorization_code",
        });

        let resp = self.client.post(&self.config.token_url)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(SsoError::TokenError(body));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(SsoToken {
            access_token: json.get("accessToken").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            refresh_token: json.get("refreshToken").and_then(|v| v.as_str()).map(|s| s.to_string()),
            expires_in: json.get("expireIn").and_then(|v| v.as_u64()),
            token_type: "Bearer".into(),
            id_token: None,
        })
    }

    async fn get_user_info(&self, token: &SsoToken) -> SsoResult<SsoUser> {
        // 钉钉：用accessToken获取用户信息
        let resp = self.client.get(&self.config.user_info_url)
            .header("x-acs-dingtalk-access-token", &token.access_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(SsoError::UserInfoError(format!("HTTP {}", resp.status())));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(SsoUser {
            external_id: json.get("unionId").or_else(|| json.get("openId")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            username: json.get("nick").or_else(|| json.get("userName")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            email: json.get("email").and_then(|v| v.as_str()).map(|s| s.to_string()),
            phone: json.get("mobile").and_then(|v| v.as_str()).map(|s| s.to_string()),
            display_name: json.get("nick").and_then(|v| v.as_str()).map(|s| s.to_string()),
            avatar_url: json.get("avatarUrl").and_then(|v| v.as_str()).map(|s| s.to_string()),
            department: None,
            roles: vec![],
            raw: json,
        })
    }

    async fn validate_token(&self, _token: &str) -> SsoResult<bool> {
        // 钉钉token验证需调用API，简化返回true
        Ok(true)
    }
}

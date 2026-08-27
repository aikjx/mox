// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 内置SSO Factory — OAuth2/OIDC

use crate::enterprise::sso::oauth2::OAuth2Provider;
use crate::enterprise::sso::{SsoConfig, SsoProvider, SsoType};
use crate::factory::{FactoryConfig, SsoFactory};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// OAuth2 SSO Factory
pub struct OAuth2SsoFactory;

#[async_trait]
impl SsoFactory for OAuth2SsoFactory {
    fn factory_type(&self) -> &'static str { "oauth2" }

    async fn create(&self, config: &FactoryConfig) -> anyhow::Result<Arc<dyn SsoProvider>> {
        let client_id = config.get_str("client_id").unwrap_or("").to_string();
        let client_secret = config.get_str("client_secret").unwrap_or("").to_string();
        let redirect_uri = config.get_str("redirect_uri").unwrap_or("").to_string();
        let auth_url = config.get_str("auth_url").unwrap_or("").to_string();
        let token_url = config.get_str("token_url").unwrap_or("").to_string();
        let user_info_url = config.get_str("user_info_url").unwrap_or("").to_string();

        // 解析scopes
        let scopes: Vec<String> = config.config.get("scopes")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_else(|| vec!["openid".into(), "profile".into(), "email".into()]);

        // 解析extra
        let extra: HashMap<String, String> = config.config.get("extra")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect())
            .unwrap_or_default();

        let sso_config = SsoConfig {
            sso_type: SsoType::OAuth2,
            client_id,
            client_secret,
            redirect_uri,
            auth_url,
            token_url,
            user_info_url,
            scopes,
            extra,
        };

        let provider = OAuth2Provider::new(sso_config);
        Ok(Arc::new(provider))
    }
}

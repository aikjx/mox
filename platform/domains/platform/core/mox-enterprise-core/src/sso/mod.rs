// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! SSO 单点登录 — 统一抽象 + 多协议实现
//!
//! 支持：OAuth2/OIDC、SAML 2.0、CAS、钉钉、企业微信、飞书
//! 新增SSO提供商：实现SsoProvider trait + 注册到SsoManager

pub mod dingtalk;
pub mod feishu;
pub mod oauth2;
pub mod wecom;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// SSO 类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SsoType {
    /// OAuth 2.0 / OIDC（通用）
    OAuth2,
    /// SAML 2.0（政企常用）
    Saml,
    /// CAS（高校/政务）
    Cas,
    /// LDAP / AD域
    Ldap,
    /// 钉钉
    DingTalk,
    /// 企业微信
    WeCom,
    /// 飞书
    Feishu,
}

impl SsoType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SsoType::OAuth2 => "oauth2",
            SsoType::Saml => "saml",
            SsoType::Cas => "cas",
            SsoType::Ldap => "ldap",
            SsoType::DingTalk => "dingtalk",
            SsoType::WeCom => "wecom",
            SsoType::Feishu => "feishu",
        }
    }
}

/// SSO 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoUser {
    /// 第三方用户唯一ID
    pub external_id: String,
    /// 用户名
    pub username: String,
    /// 邮箱
    pub email: Option<String>,
    /// 手机号
    pub phone: Option<String>,
    /// 姓名
    pub display_name: Option<String>,
    /// 头像URL
    pub avatar_url: Option<String>,
    /// 部门/组织
    pub department: Option<String>,
    /// 角色列表
    pub roles: Vec<String>,
    /// 原始响应（透传）
    pub raw: serde_json::Value,
}

/// SSO Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub token_type: String,
    pub id_token: Option<String>,
}

/// SSO 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoConfig {
    pub sso_type: SsoType,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub auth_url: String,
    pub token_url: String,
    pub user_info_url: String,
    pub scopes: Vec<String>,
    /// 额外配置（如钉钉的app_key、企业微信的corp_id）
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

/// SSO 错误
#[derive(Debug, thiserror::Error)]
pub enum SsoError {
    #[error("配置错误: {0}")]
    ConfigError(String),
    #[error("认证失败: {0}")]
    AuthError(String),
    #[error("Token交换失败: {0}")]
    TokenError(String),
    #[error("用户信息获取失败: {0}")]
    UserInfoError(String),
    #[error("网络错误: {0}")]
    NetworkError(String),
    #[error("不支持的SSO类型: {0}")]
    UnsupportedType(String),
    #[error("Provider未找到: {0}")]
    ProviderNotFound(String),
    #[error("其他错误: {0}")]
    Other(String),
}

impl From<reqwest::Error> for SsoError {
    fn from(e: reqwest::Error) -> Self {
        SsoError::NetworkError(e.to_string())
    }
}

pub type SsoResult<T> = Result<T, SsoError>;

/// SSO Provider 统一 trait
#[async_trait]
pub trait SsoProvider: Send + Sync {
    fn provider_type(&self) -> SsoType;
    fn provider_name(&self) -> &'static str;

    /// 获取授权URL
    async fn get_auth_url(&self, state: &str) -> SsoResult<String>;

    /// 用授权码交换Token
    async fn exchange_token(&self, code: &str) -> SsoResult<SsoToken>;

    /// 获取用户信息
    async fn get_user_info(&self, token: &SsoToken) -> SsoResult<SsoUser>;

    /// 验证Token有效性
    async fn validate_token(&self, token: &str) -> SsoResult<bool>;

    /// 完整登录流程（授权码→Token→用户信息）
    async fn login_with_code(&self, code: &str) -> SsoResult<(SsoToken, SsoUser)> {
        let token = self.exchange_token(code).await?;
        let user = self.get_user_info(&token).await?;
        Ok((token, user))
    }
}

/// SSO 管理器 — 按租户管理多个SSO Provider
pub struct SsoManager {
    /// tenant_id -> provider
    providers: RwLock<HashMap<String, Arc<dyn SsoProvider>>>,
    /// 默认SSO类型
    default_type: RwLock<SsoType>,
}

impl SsoManager {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            default_type: RwLock::new(SsoType::OAuth2),
        }
    }

    /// 注册租户SSO
    pub fn register(&self, tenant_id: &str, provider: Arc<dyn SsoProvider>) {
        tracing::info!("register SSO for tenant {}: {}", tenant_id, provider.provider_name());
        self.providers.write().insert(tenant_id.into(), provider);
    }

    /// 从配置创建并注册SSO
    pub fn register_from_config(&self, tenant_id: &str, config: SsoConfig) -> SsoResult<()> {
        let provider: Arc<dyn SsoProvider> = match config.sso_type {
            SsoType::OAuth2 => Arc::new(oauth2::OAuth2Provider::new(config)),
            SsoType::DingTalk => Arc::new(dingtalk::DingTalkProvider::new(config)),
            SsoType::WeCom => Arc::new(wecom::WeComProvider::new(config)),
            SsoType::Feishu => Arc::new(feishu::FeishuProvider::new(config)),
            _ => return Err(SsoError::UnsupportedType(config.sso_type.as_str().into())),
        };
        self.register(tenant_id, provider);
        Ok(())
    }

    /// 获取租户SSO Provider
    pub fn get(&self, tenant_id: &str) -> SsoResult<Arc<dyn SsoProvider>> {
        self.providers.read()
            .get(tenant_id)
            .cloned()
            .ok_or_else(|| SsoError::ProviderNotFound(tenant_id.into()))
    }

    /// 注销租户SSO
    pub fn unregister(&self, tenant_id: &str) -> Option<Arc<dyn SsoProvider>> {
        self.providers.write().remove(tenant_id)
    }

    /// 已注册租户数量
    pub fn len(&self) -> usize {
        self.providers.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.providers.read().is_empty()
    }

    pub fn set_default_type(&self, sso_type: SsoType) {
        *self.default_type.write() = sso_type;
    }

    pub fn default_type(&self) -> SsoType {
        *self.default_type.read()
    }
}

impl Default for SsoManager {
    fn default() -> Self { Self::new() }
}

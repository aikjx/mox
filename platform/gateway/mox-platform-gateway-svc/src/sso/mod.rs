//! SSO 单点登录模块
//!
//! 支持 OAuth2 / SAML / CAS / OIDC 等主流单点登录协议

pub mod api;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// SSO 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoProvider {
    /// 提供商 ID
    pub provider_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 提供商名称
    pub name: String,
    /// 协议类型：oauth2 / oidc / saml / cas / ldap
    pub protocol: String,
    /// 状态：enabled / disabled
    pub status: String,
    /// 客户端 ID
    pub client_id: String,
    /// 客户端密钥（加密存储）
    pub client_secret: String,
    /// 授权端点
    pub auth_endpoint: String,
    /// Token 端点
    pub token_endpoint: String,
    /// 用户信息端点
    pub userinfo_endpoint: Option<String>,
    /// 登出端点
    pub logout_endpoint: Option<String>,
    /// 重定向 URI
    pub redirect_uri: String,
    /// 作用域
    pub scopes: Vec<String>,
    /// 字段映射（提供商字段 -> 系统字段）
    pub field_mapping: HashMap<String, String>,
    /// 额外配置
    pub extra_config: HashMap<String, String>,
    /// 是否默认提供商
    pub is_default: bool,
    /// 排序
    pub sort_order: i32,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// SSO 登录请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoLoginRequest {
    /// 提供商 ID
    pub provider_id: String,
    /// 重定向 URI（可选，覆盖默认）
    pub redirect_uri: Option<String>,
    /// 状态参数（防 CSRF）
    pub state: Option<String>,
}

/// SSO 登录响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoLoginResponse {
    /// 授权 URL（前端跳转）
    pub auth_url: String,
    /// 状态参数
    pub state: String,
}

/// SSO 回调请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoCallbackRequest {
    /// 提供商 ID
    pub provider_id: String,
    /// 授权码
    pub code: String,
    /// 状态参数
    pub state: String,
}

/// SSO 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoUserInfo {
    /// 外部用户 ID
    pub external_user_id: String,
    /// 用户名
    pub username: String,
    /// 真实姓名
    pub real_name: Option<String>,
    /// 邮箱
    pub email: Option<String>,
    /// 手机
    pub phone: Option<String>,
    /// 部门
    pub department: Option<String>,
    /// 职位
    pub position: Option<String>,
    /// 头像
    pub avatar: Option<String>,
    /// 角色列表
    pub roles: Vec<String>,
    /// 原始数据
    pub raw: HashMap<String, serde_json::Value>,
}

/// SSO Token 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoTokenInfo {
    /// 访问令牌
    pub access_token: String,
    /// 刷新令牌
    pub refresh_token: Option<String>,
    /// 令牌类型
    pub token_type: String,
    /// 过期时间（秒）
    pub expires_in: Option<i64>,
    /// 作用域
    pub scope: Option<String>,
    /// ID Token（OIDC）
    pub id_token: Option<String>,
}

/// SSO 会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoSession {
    /// 会话 ID
    pub session_id: String,
    /// 提供商 ID
    pub provider_id: String,
    /// 用户 ID
    pub user_id: String,
    /// 外部用户 ID
    pub external_user_id: String,
    /// 访问令牌
    pub access_token: String,
    /// 刷新令牌
    pub refresh_token: Option<String>,
    /// 登录时间
    pub login_at: String,
    /// 过期时间
    pub expires_at: Option<String>,
    /// IP 地址
    pub ip_address: Option<String>,
    /// User Agent
    pub user_agent: Option<String>,
    /// 状态：active / expired / revoked
    pub status: String,
}

/// 创建 SSO 提供商请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSsoProviderRequest {
    pub name: String,
    pub protocol: String,
    pub client_id: String,
    pub client_secret: String,
    pub auth_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: Option<String>,
    pub logout_endpoint: Option<String>,
    pub redirect_uri: String,
    pub scopes: Option<Vec<String>>,
    pub field_mapping: Option<HashMap<String, String>>,
    pub extra_config: Option<HashMap<String, String>>,
    pub is_default: Option<bool>,
    pub sort_order: Option<i32>,
}

/// 更新 SSO 提供商请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSsoProviderRequest {
    pub name: Option<String>,
    pub status: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub auth_endpoint: Option<String>,
    pub token_endpoint: Option<String>,
    pub userinfo_endpoint: Option<String>,
    pub logout_endpoint: Option<String>,
    pub redirect_uri: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub field_mapping: Option<HashMap<String, String>>,
    pub extra_config: Option<HashMap<String, String>>,
    pub is_default: Option<bool>,
    pub sort_order: Option<i32>,
}

/// 支持的 SSO 协议列表
pub fn supported_protocols() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("oauth2", "OAuth 2.0", "通用 OAuth2 协议"),
        ("oidc", "OpenID Connect", "基于 OAuth2 的身份层协议"),
        ("saml", "SAML 2.0", "安全断言标记语言，企业级 SSO"),
        ("cas", "CAS", "中央认证服务，高校常用"),
        ("ldap", "LDAP", "轻量级目录访问协议"),
    ]
}

/// 内置 SSO 提供商模板
pub fn builtin_provider_templates() -> Vec<SsoProvider> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        SsoProvider {
            provider_id: "tpl_feishu".to_string(),
            tenant_id: "default".to_string(),
            name: "飞书登录".to_string(),
            protocol: "oauth2".to_string(),
            status: "disabled".to_string(),
            client_id: "".to_string(),
            client_secret: "".to_string(),
            auth_endpoint: "https://open.feishu.cn/open-apis/authen/v1/index".to_string(),
            token_endpoint: "https://open.feishu.cn/open-apis/authen/v1/oidc/access_token".to_string(),
            userinfo_endpoint: Some("https://open.feishu.cn/open-apis/authen/v1/user_info".to_string()),
            logout_endpoint: None,
            redirect_uri: "".to_string(),
            scopes: vec!["contact:user.base:readonly".to_string()],
            field_mapping: HashMap::new(),
            extra_config: HashMap::new(),
            is_default: false,
            sort_order: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        SsoProvider {
            provider_id: "tpl_dingtalk".to_string(),
            tenant_id: "default".to_string(),
            name: "钉钉登录".to_string(),
            protocol: "oauth2".to_string(),
            status: "disabled".to_string(),
            client_id: "".to_string(),
            client_secret: "".to_string(),
            auth_endpoint: "https://login.dingtalk.com/oauth2/auth".to_string(),
            token_endpoint: "https://api.dingtalk.com/v1.0/oauth2/userAccessToken".to_string(),
            userinfo_endpoint: Some("https://api.dingtalk.com/v1.0/contact/users/me".to_string()),
            logout_endpoint: None,
            redirect_uri: "".to_string(),
            scopes: vec!["openid".to_string(), "corpid".to_string()],
            field_mapping: HashMap::new(),
            extra_config: HashMap::new(),
            is_default: false,
            sort_order: 2,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        SsoProvider {
            provider_id: "tpl_wecom".to_string(),
            tenant_id: "default".to_string(),
            name: "企业微信登录".to_string(),
            protocol: "oauth2".to_string(),
            status: "disabled".to_string(),
            client_id: "".to_string(),
            client_secret: "".to_string(),
            auth_endpoint: "https://open.weixin.qq.com/connect/oauth2/authorize".to_string(),
            token_endpoint: "https://qyapi.weixin.qq.com/cgi-bin/gettoken".to_string(),
            userinfo_endpoint: Some("https://qyapi.weixin.qq.com/cgi-bin/user/getuserinfo".to_string()),
            logout_endpoint: None,
            redirect_uri: "".to_string(),
            scopes: vec!["snsapi_base".to_string()],
            field_mapping: HashMap::new(),
            extra_config: HashMap::new(),
            is_default: false,
            sort_order: 3,
            created_at: now.clone(),
            updated_at: now,
        },
    ]
}

// =============================================================================
// 安全头中间件（SecurityHeaders）
// =============================================================================
//
// 统一的 HTTP 安全响应头中间件，参考 OWASP Secure Headers Project。
//
// 包含以下安全头：
// - X-Frame-Options: 防止点击劫持
// - X-Content-Type-Options: 防止 MIME 类型嗅探
// - X-XSS-Protection: 启用浏览器 XSS 过滤
// - Content-Security-Policy: 内容安全策略，防止 XSS 和数据注入
// - Strict-Transport-Security: 强制 HTTPS 连接
// - Referrer-Policy: 控制 Referer 信息泄露
// - Permissions-Policy: 控制浏览器功能权限
// - Cross-Origin-Opener-Policy: 跨域窗口隔离
// - Cross-Origin-Resource-Policy: 跨域资源隔离
//
// 使用方式：
// ```ignore
// use mox_server_runtime::security::SecurityHeadersLayer;
//
// let app = Router::new()
//     .layer(SecurityHeadersLayer::default());
// ```
// =============================================================================

use axum::{
    http::{HeaderMap, HeaderName, HeaderValue},
    response::Response,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tower::{Layer, Service};

/// 安全头配置
#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    /// X-Frame-Options（DENY / SAMEORIGIN）
    pub x_frame_options: Option<&'static str>,
    /// X-Content-Type-Options（nosniff）
    pub x_content_type_options: Option<&'static str>,
    /// X-XSS-Protection（1; mode=block）
    pub x_xss_protection: Option<&'static str>,
    /// Content-Security-Policy
    pub content_security_policy: Option<&'static str>,
    /// Strict-Transport-Security
    pub strict_transport_security: Option<&'static str>,
    /// Referrer-Policy
    pub referrer_policy: Option<&'static str>,
    /// Permissions-Policy
    pub permissions_policy: Option<&'static str>,
    /// Cross-Origin-Opener-Policy
    pub cross_origin_opener_policy: Option<&'static str>,
    /// Cross-Origin-Resource-Policy
    pub cross_origin_resource_policy: Option<&'static str>,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            x_frame_options: Some("DENY"),
            x_content_type_options: Some("nosniff"),
            x_xss_protection: Some("1; mode=block"),
            content_security_policy: Some("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self' https:; frame-ancestors 'none'"),
            strict_transport_security: Some("max-age=31536000; includeSubDomains; preload"),
            referrer_policy: Some("strict-origin-when-cross-origin"),
            permissions_policy: Some("geolocation=(), microphone=(), camera=(), payment=(), usb=()"),
            cross_origin_opener_policy: Some("same-origin"),
            cross_origin_resource_policy: Some("same-origin"),
        }
    }
}

impl SecurityHeadersConfig {
    /// 创建宽松配置（适合开发环境）
    pub fn permissive() -> Self {
        Self {
            x_frame_options: Some("SAMEORIGIN"),
            x_content_type_options: Some("nosniff"),
            x_xss_protection: Some("1; mode=block"),
            content_security_policy: Some("default-src 'self' 'unsafe-inline' 'unsafe-eval'; img-src * data:; connect-src *; font-src * data:; style-src * 'unsafe-inline'"),
            strict_transport_security: None, // 开发环境不强制 HTTPS
            referrer_policy: Some("no-referrer-when-downgrade"),
            permissions_policy: Some("geolocation=*, microphone=*, camera=*"),
            cross_origin_opener_policy: None,
            cross_origin_resource_policy: None,
        }
    }

    /// 创建严格配置（适合生产环境，最高安全等级）
    pub fn strict() -> Self {
        Self {
            x_frame_options: Some("DENY"),
            x_content_type_options: Some("nosniff"),
            x_xss_protection: Some("1; mode=block"),
            content_security_policy: Some("default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'"),
            strict_transport_security: Some("max-age=63072000; includeSubDomains; preload"),
            referrer_policy: Some("no-referrer"),
            permissions_policy: Some("geolocation=(), microphone=(), camera=(), payment=(), usb=(), accelerometer=(), gyroscope=(), magnetometer=()"),
            cross_origin_opener_policy: Some("same-origin"),
            cross_origin_resource_policy: Some("same-origin"),
        }
    }

    /// 转换为 HeaderMap
    pub fn to_header_map(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        if let Some(value) = self.x_frame_options {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(b"X-Frame-Options"),
                HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        if let Some(value) = self.x_content_type_options {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(b"X-Content-Type-Options"),
                HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        if let Some(value) = self.x_xss_protection {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(b"X-XSS-Protection"),
                HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        if let Some(value) = self.content_security_policy {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(b"Content-Security-Policy"),
                HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        if let Some(value) = self.strict_transport_security {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(b"Strict-Transport-Security"),
                HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        if let Some(value) = self.referrer_policy {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(b"Referrer-Policy"),
                HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        if let Some(value) = self.permissions_policy {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(b"Permissions-Policy"),
                HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        if let Some(value) = self.cross_origin_opener_policy {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(b"Cross-Origin-Opener-Policy"),
                HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        if let Some(value) = self.cross_origin_resource_policy {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(b"Cross-Origin-Resource-Policy"),
                HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        headers
    }
}

/// 安全头中间件 Layer
#[derive(Debug, Clone)]
pub struct SecurityHeadersLayer {
    config: Arc<SecurityHeadersConfig>,
}

impl SecurityHeadersLayer {
    /// 创建安全头中间件
    pub fn new(config: SecurityHeadersConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// 使用默认配置创建
    pub fn default() -> Self {
        Self::new(SecurityHeadersConfig::default())
    }

    /// 使用严格配置创建（生产环境推荐）
    pub fn strict() -> Self {
        Self::new(SecurityHeadersConfig::strict())
    }

    /// 使用宽松配置创建（开发环境）
    pub fn permissive() -> Self {
        Self::new(SecurityHeadersConfig::permissive())
    }
}

impl Default for SecurityHeadersLayer {
    fn default() -> Self {
        Self::default()
    }
}

impl<S> Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeadersService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityHeadersService {
            inner,
            config: self.config.clone(),
        }
    }
}

/// 安全头服务
#[derive(Debug, Clone)]
pub struct SecurityHeadersService<S> {
    inner: S,
    config: Arc<SecurityHeadersConfig>,
}

impl<S, ReqBody, ResBody> Service<axum::http::Request<ReqBody>> for SecurityHeadersService<S>
where
    S: Service<axum::http::Request<ReqBody>, Response = Response<ResBody>> + Send + 'static,
    S::Future: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::http::Request<ReqBody>) -> Self::Future {
        let config = self.config.clone();
        let future = self.inner.call(req);

        Box::pin(async move {
            let mut response = future.await?;

            // 注入安全头
            let headers = config.to_header_map();
            let response_headers = response.headers_mut();
            for (name, value) in headers.iter() {
                // 不覆盖已有的头（允许应用自定义）
                if !response_headers.contains_key(name) {
                    response_headers.insert(name.clone(), value.clone());
                }
            }

            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_headers() {
        let config = SecurityHeadersConfig::default();
        let headers = config.to_header_map();

        assert!(headers.contains_key("x-frame-options"));
        assert!(headers.contains_key("x-content-type-options"));
        assert!(headers.contains_key("x-xss-protection"));
        assert!(headers.contains_key("content-security-policy"));
        assert!(headers.contains_key("strict-transport-security"));
        assert!(headers.contains_key("referrer-policy"));
        assert!(headers.contains_key("permissions-policy"));
        assert!(headers.contains_key("cross-origin-opener-policy"));
        assert!(headers.contains_key("cross-origin-resource-policy"));
    }

    #[test]
    fn test_strict_config() {
        let config = SecurityHeadersConfig::strict();
        let headers = config.to_header_map();

        let csp = headers.get("content-security-policy").unwrap();
        assert!(csp.to_str().unwrap().contains("default-src 'none'"));

        let hsts = headers.get("strict-transport-security").unwrap();
        assert!(hsts.to_str().unwrap().contains("max-age=63072000"));
    }

    #[test]
    fn test_permissive_config() {
        let config = SecurityHeadersConfig::permissive();
        let headers = config.to_header_map();

        // 宽松配置不强制 HTTPS
        assert!(!headers.contains_key("strict-transport-security"));

        let xfo = headers.get("x-frame-options").unwrap();
        assert_eq!(xfo.to_str().unwrap(), "SAMEORIGIN");
    }

    #[test]
    fn test_security_headers_layer_creation() {
        let layer = SecurityHeadersLayer::default();
        let strict_layer = SecurityHeadersLayer::strict();
        let permissive_layer = SecurityHeadersLayer::permissive();

        assert!(layer.config.x_frame_options.is_some());
        assert!(strict_layer.config.x_frame_options.is_some());
        assert!(permissive_layer.config.x_frame_options.is_some());
    }

    #[test]
    fn test_header_values() {
        let config = SecurityHeadersConfig::default();
        let headers = config.to_header_map();

        assert_eq!(
            headers.get("x-frame-options").unwrap().to_str().unwrap(),
            "DENY"
        );
        assert_eq!(
            headers.get("x-content-type-options").unwrap().to_str().unwrap(),
            "nosniff"
        );
        assert_eq!(
            headers.get("x-xss-protection").unwrap().to_str().unwrap(),
            "1; mode=block"
        );
    }
}

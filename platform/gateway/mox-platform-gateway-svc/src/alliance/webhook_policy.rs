//! Operator-controlled exact origins. Default denies all outbound subscriptions.
use reqwest::Url;

pub(super) fn allowed_target(value: &str) -> Result<Url, &'static str> {
    if value.len() > 2048 {
        return Err("Webhook 地址长度不得超过 2048 字节");
    }
    let url = Url::parse(value).map_err(|_| "Webhook 地址无效")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.query().is_some()
    {
        return Err("Webhook 仅接受无账号、密码、查询参数和片段的 HTTP(S) 地址");
    }
    let configured = std::env::var("MOX_WEBHOOK_ALLOWED_ORIGINS").unwrap_or_default();
    let allowed =
        configured
            .split(',')
            .filter_map(|origin| Url::parse(origin.trim()).ok())
            .any(|origin| {
                origin.path() == "/"
                    && origin.query().is_none()
                    && origin.fragment().is_none()
                    && origin.username().is_empty()
                    && origin.password().is_none()
                    && origin.origin() == url.origin()
            });
    if !allowed {
        return Err("Webhook 目标源尚未获运维授权，请联系运维配置接收地址");
    }
    Ok(url)
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 白标配置 — 政企品牌定制

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;

/// 白标配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelabelConfig {
    /// 品牌名称
    pub brand_name: String,
    /// 品牌简称
    pub brand_short_name: Option<String>,
    /// Logo URL（浅色模式）
    pub logo_url: Option<String>,
    /// Logo URL（深色模式）
    pub logo_dark_url: Option<String>,
    /// Favicon URL
    pub favicon_url: Option<String>,
    /// 登录页背景图URL
    pub login_background_url: Option<String>,
    /// 系统域名
    pub domain: Option<String>,
    /// 客服邮箱
    pub support_email: Option<String>,
    /// 客服电话
    pub support_phone: Option<String>,
    /// 官网URL
    pub website_url: Option<String>,
    /// 备案号
    pub icp_number: Option<String>,
    /// 版权信息
    pub copyright: Option<String>,
    /// 登录页欢迎语
    pub login_welcome: Option<String>,
    /// 登录页副标题
    pub login_subtitle: Option<String>,
    /// 是否显示"由MOX提供技术支持"
    pub show_powered_by: bool,
    /// 自定义页脚HTML
    pub custom_footer_html: Option<String>,
    /// 额外元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Default for WhitelabelConfig {
    fn default() -> Self {
        Self {
            brand_name: "MOX 平台".into(),
            brand_short_name: Some("MOX".into()),
            logo_url: None,
            logo_dark_url: None,
            favicon_url: None,
            login_background_url: None,
            domain: None,
            support_email: Some("support@mox.local".into()),
            support_phone: None,
            website_url: None,
            icp_number: None,
            copyright: Some("© 2026 MOX. All rights reserved.".into()),
            login_welcome: Some("欢迎使用".into()),
            login_subtitle: Some("AI驱动的全维度突破平台".into()),
            show_powered_by: true,
            custom_footer_html: None,
            metadata: HashMap::new(),
        }
    }
}

/// 白标管理器 — 按租户管理白标配置
pub struct WhitelabelManager {
    configs: RwLock<HashMap<String, WhitelabelConfig>>,
    default_config: RwLock<WhitelabelConfig>,
}

impl WhitelabelManager {
    pub fn new() -> Self {
        Self {
            configs: RwLock::new(HashMap::new()),
            default_config: RwLock::new(WhitelabelConfig::default()),
        }
    }

    pub fn set_default(&self, config: WhitelabelConfig) {
        *self.default_config.write() = config;
    }

    pub fn set_tenant(&self, tenant_id: &str, config: WhitelabelConfig) {
        self.configs.write().insert(tenant_id.into(), config);
    }

    pub fn get(&self, tenant_id: &str) -> WhitelabelConfig {
        self.configs.read()
            .get(tenant_id)
            .cloned()
            .unwrap_or_else(|| self.default_config.read().clone())
    }

    pub fn remove_tenant(&self, tenant_id: &str) -> Option<WhitelabelConfig> {
        self.configs.write().remove(tenant_id)
    }

    pub fn tenant_count(&self) -> usize {
        self.configs.read().len()
    }
}

impl Default for WhitelabelManager {
    fn default() -> Self { Self::new() }
}

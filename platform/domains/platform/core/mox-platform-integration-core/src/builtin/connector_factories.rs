// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 内置Connector Factory — Webhook

use crate::connector::connectors::webhook::WebhookConnector;
use crate::connector::traits::{Connector, ConnectorConfig, ConnectorType};
use crate::factory::{ConnectorFactory, FactoryConfig};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Webhook Connector Factory
pub struct WebhookConnectorFactory;

#[async_trait]
impl ConnectorFactory for WebhookConnectorFactory {
    fn factory_type(&self) -> &'static str { "webhook" }

    async fn create(&self, config: &FactoryConfig) -> anyhow::Result<Arc<dyn Connector>> {
        let endpoint = config.get_str("endpoint").unwrap_or("").to_string();
        let protocol = config.get_str("protocol").unwrap_or("rest").to_string();
        let auth_type = config.get_str("auth_type").unwrap_or("none").to_string();

        // 解析credentials
        let credentials: HashMap<String, String> = config.config.get("credentials")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect())
            .unwrap_or_default();

        // 解析headers
        let headers: HashMap<String, String> = config.config.get("headers")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect())
            .unwrap_or_default();

        let connector_config = ConnectorConfig {
            id: config.id.clone(),
            name: config.name.clone(),
            connector_type: ConnectorType::Webhook,
            protocol,
            endpoint,
            auth_type,
            credentials,
            headers,
            timeout_secs: 30,
            max_retries: 2,
            extra: HashMap::new(),
            enabled: config.enabled,
        };

        let connector = WebhookConnector::new(connector_config);
        Ok(Arc::new(connector))
    }
}

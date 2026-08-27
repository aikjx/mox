// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 内置Factory实现 — 开箱即用
//!
//! 提供OpenAI/Qwen/Webhook/OAuth2等常用Factory实现，
//! 调用`register_all_builtin_factories()`即可全部注册。

pub mod ai_factories;
pub mod connector_factories;
pub mod sso_factories;

use crate::factory::FactoryRegistry;
use std::sync::Arc;

/// 注册所有内置Factory（开箱即用）
pub fn register_all_builtin_factories(registry: &FactoryRegistry) {
    // AI Provider Factory
    registry.register_ai_factory(Arc::new(ai_factories::OpenAiFactory));
    registry.register_ai_factory(Arc::new(ai_factories::QwenFactory));
    registry.register_ai_factory(Arc::new(ai_factories::AnthropicFactory));

    // Connector Factory
    registry.register_connector_factory(Arc::new(connector_factories::WebhookConnectorFactory));

    // SSO Factory
    registry.register_sso_factory(Arc::new(sso_factories::OAuth2SsoFactory));

    tracing::info!("all builtin factories registered: AI(3) + Connector(1) + SSO(1)");
}

/// 仅注册AI内置Factory
pub fn register_ai_factories(registry: &FactoryRegistry) {
    registry.register_ai_factory(Arc::new(ai_factories::OpenAiFactory));
    registry.register_ai_factory(Arc::new(ai_factories::QwenFactory));
    registry.register_ai_factory(Arc::new(ai_factories::AnthropicFactory));
}

/// 仅注册Connector内置Factory
pub fn register_connector_factories(registry: &FactoryRegistry) {
    registry.register_connector_factory(Arc::new(connector_factories::WebhookConnectorFactory));
}

/// 仅注册SSO内置Factory
pub fn register_sso_factories(registry: &FactoryRegistry) {
    registry.register_sso_factory(Arc::new(sso_factories::OAuth2SsoFactory));
}

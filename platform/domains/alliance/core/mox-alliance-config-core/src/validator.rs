// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 配置验证器

use mox_alliance_common_proto::{
    ApiKeySource, ExpertModuleConfig, GlobalLlmConfig, GraphEngineType, ModuleGraphConfig,
    ModuleLlmConfig,
};

use crate::error::{ConfigError, ConfigResult};

/// 配置验证器
pub struct ConfigValidator;

impl ConfigValidator {
    /// 验证模块完整配置
    pub fn validate_module_config(config: &ExpertModuleConfig) -> ConfigResult<()> {
        // 验证 module_id 非空
        if config.module_id.trim().is_empty() {
            return Err(ConfigError::ValidationFailed(
                "module_id must not be empty".to_string(),
            ));
        }

        // 验证 expert_id 非空
        if config.expert_id.trim().is_empty() {
            return Err(ConfigError::ValidationFailed(
                "expert_id must not be empty".to_string(),
            ));
        }

        // 验证名称非空
        if config.name.trim().is_empty() {
            return Err(ConfigError::ValidationFailed(
                "name must not be empty".to_string(),
            ));
        }

        // 验证 LLM 配置
        Self::validate_llm_config(&config.llm_config)?;

        // 验证 Graph 配置
        Self::validate_graph_config(&config.graph_config)?;

        // 验证能力权重在合理范围
        for (cap_id, weight) in &config.capability_weights {
            if !(0.0..=1.0).contains(weight) {
                return Err(ConfigError::ValidationFailed(format!(
                    "Capability weight for {} must be between 0.0 and 1.0, got {}",
                    cap_id, weight
                )));
            }
        }

        // 验证匹配权重之和约等于 1.0
        let mw = &config.matching_weights;
        let total = mw.domain + mw.capability + mw.rating + mw.performance + mw.health;
        if (total - 1.0).abs() > 0.01 {
            return Err(ConfigError::ValidationFailed(format!(
                "Matching weights must sum to 1.0, got {:.2}",
                total
            )));
        }

        Ok(())
    }

    /// 验证 LLM 配置
    pub fn validate_llm_config(config: &ModuleLlmConfig) -> ConfigResult<()> {
        // module_id 非空
        if config.module_id.trim().is_empty() {
            return Err(ConfigError::ValidationFailed(
                "module_id must not be empty".to_string(),
            ));
        }

        // primary_provider 非空
        if config.primary_provider.trim().is_empty() {
            return Err(ConfigError::ValidationFailed(
                "primary_provider must not be empty".to_string(),
            ));
        }

        // primary_model 非空
        if config.primary_model.trim().is_empty() {
            return Err(ConfigError::ValidationFailed(
                "primary_model must not be empty".to_string(),
            ));
        }

        // 验证模型参数范围
        let mc = &config.model_config;
        if !(0.0..=2.0).contains(&mc.temperature) {
            return Err(ConfigError::ValidationFailed(format!(
                "temperature must be between 0.0 and 2.0, got {}",
                mc.temperature
            )));
        }
        if !(0.0..=1.0).contains(&mc.top_p) {
            return Err(ConfigError::ValidationFailed(format!(
                "top_p must be between 0.0 and 1.0, got {}",
                mc.top_p
            )));
        }
        if mc.max_tokens == 0 {
            return Err(ConfigError::ValidationFailed(
                "max_tokens must be greater than 0".to_string(),
            ));
        }
        if !(-2.0..=2.0).contains(&mc.frequency_penalty) {
            return Err(ConfigError::ValidationFailed(format!(
                "frequency_penalty must be between -2.0 and 2.0, got {}",
                mc.frequency_penalty
            )));
        }
        if !(-2.0..=2.0).contains(&mc.presence_penalty) {
            return Err(ConfigError::ValidationFailed(format!(
                "presence_penalty must be between -2.0 and 2.0, got {}",
                mc.presence_penalty
            )));
        }

        // 验证 provider 选项
        for provider in &config.provider_options {
            if provider.provider_id.trim().is_empty() {
                return Err(ConfigError::ValidationFailed(
                    "provider_id must not be empty".to_string(),
                ));
            }
            // 验证 API Key 来源配置
            match &provider.api_key_source {
                ApiKeySource::EnvVar { env_name } => {
                    if env_name.trim().is_empty() {
                        return Err(ConfigError::ValidationFailed(format!(
                            "env_name for provider {} must not be empty",
                            provider.provider_id
                        )));
                    }
                }
                ApiKeySource::PlainText { api_key } => {
                    if api_key.trim().is_empty() {
                        return Err(ConfigError::ValidationFailed(format!(
                            "api_key for provider {} must not be empty",
                            provider.provider_id
                        )));
                    }
                }
                ApiKeySource::SecretRef { secret_id, .. } => {
                    if secret_id.trim().is_empty() {
                        return Err(ConfigError::ValidationFailed(format!(
                            "secret_id for provider {} must not be empty",
                            provider.provider_id
                        )));
                    }
                }
                ApiKeySource::Inherit => {
                    // 继承模式是允许的（模块级可以继承全局配置）
                }
            }
        }

        // 验证 primary_provider 在 provider_options 中存在且启用
        let primary_exists = config
            .provider_options
            .iter()
            .any(|p| p.provider_id == config.primary_provider && p.enabled);
        if !primary_exists && !config.provider_options.is_empty() {
            return Err(ConfigError::ValidationFailed(format!(
                "Primary provider '{}' not found in enabled provider options",
                config.primary_provider
            )));
        }

        // 验证 fallback_chain 中的 provider 都存在
        for fallback in &config.fallback_chain {
            let exists = config
                .provider_options
                .iter()
                .any(|p| &p.provider_id == fallback && p.enabled);
            if !exists && !config.provider_options.is_empty() {
                return Err(ConfigError::ValidationFailed(format!(
                    "Fallback provider '{}' not found in enabled provider options",
                    fallback
                )));
            }
        }

        Ok(())
    }

    /// 验证 Graph 配置
    pub fn validate_graph_config(config: &ModuleGraphConfig) -> ConfigResult<()> {
        // module_id 非空
        if config.module_id.trim().is_empty() {
            return Err(ConfigError::ValidationFailed(
                "module_id must not be empty".to_string(),
            ));
        }

        // 验证连接配置
        if config.connection.uri_env.trim().is_empty() {
            return Err(ConfigError::ValidationFailed(
                "connection.uri_env must not be empty".to_string(),
            ));
        }

        // 验证查询参数
        if config.query_config.timeout_ms == 0 {
            return Err(ConfigError::ValidationFailed(
                "query_config.timeout_ms must be greater than 0".to_string(),
            ));
        }
        if config.query_config.max_results == 0 {
            return Err(ConfigError::ValidationFailed(
                "query_config.max_results must be greater than 0".to_string(),
            ));
        }

        // Custom 引擎类型必须提供端点
        if config.engine_type == GraphEngineType::Custom
            && config.custom_endpoint.as_deref().unwrap_or("").is_empty()
        {
            return Err(ConfigError::ValidationFailed(
                "custom_endpoint must be provided when engine_type is 'custom'".to_string(),
            ));
        }

        Ok(())
    }

    /// 验证全局默认 LLM 配置
    pub fn validate_global_llm_config(config: &GlobalLlmConfig) -> ConfigResult<()> {
        // primary_provider 非空
        if config.primary_provider.trim().is_empty() {
            return Err(ConfigError::ValidationFailed(
                "primary_provider must not be empty".to_string(),
            ));
        }

        // primary_model 非空
        if config.primary_model.trim().is_empty() {
            return Err(ConfigError::ValidationFailed(
                "primary_model must not be empty".to_string(),
            ));
        }

        // 验证模型参数范围
        let mc = &config.model_config;
        if !(0.0..=2.0).contains(&mc.temperature) {
            return Err(ConfigError::ValidationFailed(format!(
                "temperature must be between 0.0 and 2.0, got {}",
                mc.temperature
            )));
        }
        if !(0.0..=1.0).contains(&mc.top_p) {
            return Err(ConfigError::ValidationFailed(format!(
                "top_p must be between 0.0 and 1.0, got {}",
                mc.top_p
            )));
        }
        if mc.max_tokens == 0 {
            return Err(ConfigError::ValidationFailed(
                "max_tokens must be greater than 0".to_string(),
            ));
        }
        if !(-2.0..=2.0).contains(&mc.frequency_penalty) {
            return Err(ConfigError::ValidationFailed(format!(
                "frequency_penalty must be between -2.0 and 2.0, got {}",
                mc.frequency_penalty
            )));
        }
        if !(-2.0..=2.0).contains(&mc.presence_penalty) {
            return Err(ConfigError::ValidationFailed(format!(
                "presence_penalty must be between -2.0 and 2.0, got {}",
                mc.presence_penalty
            )));
        }

        // 验证 provider 选项
        for provider in &config.provider_options {
            if provider.provider_id.trim().is_empty() {
                return Err(ConfigError::ValidationFailed(
                    "provider_id must not be empty".to_string(),
                ));
            }
            // 全局配置中不允许 Inherit 模式（全局是最顶层）
            if matches!(provider.api_key_source, ApiKeySource::Inherit) {
                return Err(ConfigError::ValidationFailed(format!(
                    "Global provider '{}' cannot use 'inherit' api_key_source",
                    provider.provider_id
                )));
            }
            // 验证 API Key 来源
            match &provider.api_key_source {
                ApiKeySource::EnvVar { env_name } => {
                    if env_name.trim().is_empty() {
                        return Err(ConfigError::ValidationFailed(format!(
                            "env_name for provider {} must not be empty",
                            provider.provider_id
                        )));
                    }
                }
                ApiKeySource::PlainText { api_key } => {
                    if api_key.trim().is_empty() {
                        return Err(ConfigError::ValidationFailed(format!(
                            "api_key for provider {} must not be empty",
                            provider.provider_id
                        )));
                    }
                }
                ApiKeySource::SecretRef { secret_id, .. } => {
                    if secret_id.trim().is_empty() {
                        return Err(ConfigError::ValidationFailed(format!(
                            "secret_id for provider {} must not be empty",
                            provider.provider_id
                        )));
                    }
                }
                ApiKeySource::Inherit => unreachable!(),
            }
        }

        // 验证 primary_provider 在 provider_options 中存在且启用
        let primary_exists = config
            .provider_options
            .iter()
            .any(|p| p.provider_id == config.primary_provider && p.enabled);
        if !primary_exists && !config.provider_options.is_empty() {
            return Err(ConfigError::ValidationFailed(format!(
                "Primary provider '{}' not found in enabled provider options",
                config.primary_provider
            )));
        }

        // 验证 fallback_chain 中的 provider 都存在
        for fallback in &config.fallback_chain {
            let exists = config
                .provider_options
                .iter()
                .any(|p| &p.provider_id == fallback && p.enabled);
            if !exists && !config.provider_options.is_empty() {
                return Err(ConfigError::ValidationFailed(format!(
                    "Fallback provider '{}' not found in enabled provider options",
                    fallback
                )));
            }
        }

        Ok(())
    }
}

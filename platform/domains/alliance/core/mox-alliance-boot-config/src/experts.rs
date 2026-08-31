// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家配置外部化（Experts Boot Config）
//!
//! 将专家联盟的**专家模块配置**（全局 LLM 配置 + 领域专家模块）从代码写死
//! 改为 **yml 可加载 + 按模块合并**，是配置外部化（Nacos 演进）的专家维度闭环。
//!
//! ## 设计：覆盖式合并（Overlay）
//! - `global_llm`：**局部覆盖**内置全局 LLM 配置（只写想改的字段，其余继承）
//! - `modules`：按 `module_id` 与内置 10 大领域专家**合并**（yml 提及的字段覆盖，
//!   未提及的保留内置默认；yml 未列出的模块保持不变）
//! - 时间戳（`created_at`/`updated_at`）由系统生成，yml 无需填写
//!
//! ## 配置文件
//! `config/alliance-experts.yml`（路径可用 `MOX_ALLIANCE_EXPERTS_FILE` 覆盖）

use std::collections::HashMap;

use mox_alliance_common_proto::{
    ExpertModuleConfig, GlobalLlmConfig, LlmRoutingStrategy, MatchingWeights, ModelConfig,
    ModuleGraphConfig, ModuleLlmConfig,
};
use serde::Deserialize;

/// 全局 LLM 配置的局部覆盖（只覆盖提供的字段，其余继承内置）
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct GlobalLlmOverlay {
    /// 主 provider（如 openai / anthropic / qwen / deepseek）
    pub primary_provider: Option<String>,
    /// 主模型（如 gpt-4o）
    pub primary_model: Option<String>,
    /// 回退链（provider 列表，按优先级）
    pub fallback_chain: Option<Vec<String>>,
    /// 路由策略
    pub routing_strategy: Option<LlmRoutingStrategy>,
    /// 采样参数（temperature / top_p / max_tokens 等）
    pub model_config: Option<ModelConfig>,
}

impl GlobalLlmOverlay {
    /// 将覆盖应用到内置全局配置，返回合并结果
    pub fn apply_to(&self, base: &GlobalLlmConfig) -> GlobalLlmConfig {
        GlobalLlmConfig {
            primary_provider: self.primary_provider.clone().unwrap_or_else(|| base.primary_provider.clone()),
            primary_model: self.primary_model.clone().unwrap_or_else(|| base.primary_model.clone()),
            fallback_chain: self
                .fallback_chain
                .clone()
                .unwrap_or_else(|| base.fallback_chain.clone()),
            routing_strategy: self.routing_strategy.unwrap_or(base.routing_strategy),
            model_config: self.model_config.clone().unwrap_or_else(|| base.model_config.clone()),
            provider_options: base.provider_options.clone(),
            global_system_prompt_prefix: base.global_system_prompt_prefix.clone(),
            version: base.version,
            updated_at: chrono::Utc::now(),
        }
    }
}

/// 单个专家模块的覆盖（按 `module_id` 合并到内置模块）
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ExpertModuleOverlay {
    /// 模块 ID（必填，匹配内置模块）
    pub module_id: String,
    /// 专家 ID
    pub expert_id: Option<String>,
    /// 模块名称
    pub name: Option<String>,
    /// 模块版本
    pub version: Option<String>,
    /// LLM 配置
    pub llm_config: Option<ModuleLlmConfig>,
    /// Graph 配置
    pub graph_config: Option<ModuleGraphConfig>,
    /// 能力权重 (capability_id -> weight)
    pub capability_weights: Option<HashMap<String, f32>>,
    /// 匹配权重
    pub matching_weights: Option<MatchingWeights>,
    /// 是否启用
    pub enabled: Option<bool>,
    /// 标签
    pub tags: Option<Vec<String>>,
}

/// 专家配置整体（对应 `config/alliance-experts.yml`）
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ExpertsBootConfig {
    /// 全局 LLM 局部覆盖（None = 使用内置）
    pub global_llm: Option<GlobalLlmOverlay>,
    /// 模块覆盖列表（按 module_id 合并）
    pub modules: Vec<ExpertModuleOverlay>,
}

impl ExpertsBootConfig {
    /// 计算生效的全局 LLM 配置
    pub fn effective_global(&self, builtin: &GlobalLlmConfig) -> GlobalLlmConfig {
        match &self.global_llm {
            Some(overlay) => overlay.apply_to(builtin),
            None => builtin.clone(),
        }
    }

    /// 将模块覆盖合并到内置模块列表
    ///
    /// - 内置模块被覆盖时：保留其 `created_at`/`updated_at`，其余字段按覆盖合并
    /// - yml 引入内置不存在的模块：以默认值 + 覆盖字段创建
    pub fn merge_into(&self, builtin: Vec<ExpertModuleConfig>) -> Vec<ExpertModuleConfig> {
        let mut map: HashMap<String, ExpertModuleConfig> =
            builtin.into_iter().map(|m| (m.module_id.clone(), m)).collect();

        for overlay in &self.modules {
            if overlay.module_id.is_empty() {
                continue;
            }
            match map.get_mut(&overlay.module_id) {
                Some(module) => {
                    overlay.apply_to(module);
                }
                None => {
                    // yml 新增模块：以默认 + 覆盖字段创建
                    let mut module = ExpertModuleConfig {
                        module_id: overlay.module_id.clone(),
                        expert_id: overlay.expert_id.clone().unwrap_or_default(),
                        name: overlay.name.clone().unwrap_or_else(|| overlay.module_id.clone()),
                        version: overlay.version.clone().unwrap_or_else(|| "1.0.0".to_string()),
                        llm_config: overlay
                            .llm_config
                            .clone()
                            .unwrap_or_else(ModuleLlmConfig::default),
                        graph_config: overlay
                            .graph_config
                            .clone()
                            .unwrap_or_else(ModuleGraphConfig::default),
                        capability_weights: overlay.capability_weights.clone().unwrap_or_default(),
                        matching_weights: overlay
                            .matching_weights
                            .clone()
                            .unwrap_or_default(),
                        enabled: overlay.enabled.unwrap_or(true),
                        tags: overlay.tags.clone().unwrap_or_default(),
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                    };
                    overlay.apply_to(&mut module);
                    map.insert(overlay.module_id.clone(), module);
                }
            }
        }

        // 保持内置顺序稳定
        let mut out: Vec<ExpertModuleConfig> = map.into_values().collect();
        out.sort_by_key(|m| m.module_id.clone());
        out
    }
}

impl ExpertModuleOverlay {
    /// 将覆盖字段合并进目标模块（保留 created_at/updated_at）
    fn apply_to(&self, module: &mut ExpertModuleConfig) {
        if let Some(v) = &self.expert_id {
            module.expert_id = v.clone();
        }
        if let Some(v) = &self.name {
            module.name = v.clone();
        }
        if let Some(v) = &self.version {
            module.version = v.clone();
        }
        if let Some(v) = &self.llm_config {
            module.llm_config = v.clone();
        }
        if let Some(v) = &self.graph_config {
            module.graph_config = v.clone();
        }
        if let Some(v) = &self.capability_weights {
            module.capability_weights = v.clone();
        }
        if let Some(v) = &self.matching_weights {
            module.matching_weights = v.clone();
        }
        if let Some(v) = self.enabled {
            module.enabled = v;
        }
        if let Some(v) = &self.tags {
            module.tags = v.clone();
        }
        module.updated_at = chrono::Utc::now();
    }
}

/// 从 yml 文件加载专家配置覆盖
///
/// - 文件不存在 → 空覆盖（全部使用内置默认）
/// - 解析失败 → 返回错误（配置错误必须显式暴露）
pub fn load_experts(path: &str) -> anyhow::Result<ExpertsBootConfig> {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let cfg: ExpertsBootConfig = serde_yaml::from_str(&content).map_err(|e| {
                anyhow::anyhow!("专家配置 {path} 解析失败: {e}")
            })?;
            tracing::info!("加载专家配置覆盖: {path}（{} 个模块覆盖）", cfg.modules.len());
            Ok(cfg)
        }
        Err(_) => {
            tracing::warn!("专家配置文件 {path} 不存在，使用内置 10 大领域专家默认配置。");
            Ok(ExpertsBootConfig::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_common_proto::{
        ApiKeySource, LlmProviderOption, LlmRoutingStrategy, ModelConfig,
    };

    fn sample_global() -> GlobalLlmConfig {
        GlobalLlmConfig {
            primary_provider: "openai".to_string(),
            primary_model: "gpt-4o".to_string(),
            fallback_chain: vec!["anthropic".to_string()],
            routing_strategy: LlmRoutingStrategy::Priority,
            model_config: ModelConfig {
                temperature: 0.7,
                top_p: 0.9,
                max_tokens: 4096,
                frequency_penalty: 0.0,
                presence_penalty: 0.0,
                stop_sequences: vec![],
            },
            provider_options: vec![LlmProviderOption {
                provider_id: "openai".to_string(),
                display_name: None,
                api_key_source: ApiKeySource::Inherit,
                base_url: None,
                default_model: None,
                supported_models: vec![],
                price_per_1k_tokens: None,
                rpm_limit: None,
                tpm_limit: None,
                enabled: true,
            }],
            global_system_prompt_prefix: None,
            version: 1,
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn global_llm_overlay_partial() {
        let base = sample_global();
        let overlay = GlobalLlmOverlay {
            primary_model: Some("gpt-4o-mini".to_string()),
            ..Default::default()
        };
        let merged = overlay.apply_to(&base);
        // 覆盖生效
        assert_eq!(merged.primary_model, "gpt-4o-mini");
        // 未覆盖字段继承
        assert_eq!(merged.primary_provider, "openai");
        assert_eq!(merged.model_config.max_tokens, 4096);
        // provider_options 保留内置
        assert_eq!(merged.provider_options.len(), 1);
    }

    #[test]
    fn merge_module_by_id() {
        let builtin = vec![
            ExpertModuleConfig {
                module_id: "expert-code".to_string(),
                expert_id: "code-expert-001".to_string(),
                name: "代码编程专家".to_string(),
                version: "1.0.0".to_string(),
                llm_config: ModuleLlmConfig::default(),
                graph_config: ModuleGraphConfig::default(),
                capability_weights: Default::default(),
                matching_weights: MatchingWeights::default(),
                enabled: true,
                tags: vec!["code".to_string()],
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        ];
        let cfg = ExpertsBootConfig {
            modules: vec![ExpertModuleOverlay {
                module_id: "expert-code".to_string(),
                name: Some("代码编程专家(覆盖)".to_string()),
                enabled: Some(false),
                ..Default::default()
            }],
            ..Default::default()
        };
        let merged = cfg.merge_into(builtin);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "代码编程专家(覆盖)");
        assert_eq!(merged[0].enabled, false);
        // 未覆盖字段保留
        assert_eq!(merged[0].expert_id, "code-expert-001");
        assert_eq!(merged[0].tags, vec!["code".to_string()]);
    }

    #[test]
    fn empty_config_keeps_builtin() {
        let builtin = vec![
            ExpertModuleConfig {
                module_id: "expert-math".to_string(),
                expert_id: "math-expert-001".to_string(),
                name: "数学专家".to_string(),
                version: "1.0.0".to_string(),
                llm_config: ModuleLlmConfig::default(),
                graph_config: ModuleGraphConfig::default(),
                capability_weights: Default::default(),
                matching_weights: MatchingWeights::default(),
                enabled: true,
                tags: vec![],
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        ];
        let cfg = ExpertsBootConfig::default();
        let merged = cfg.merge_into(builtin);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "数学专家");
    }
}

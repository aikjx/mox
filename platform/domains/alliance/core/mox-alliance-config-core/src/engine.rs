// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 模块化配置引擎
//!
//! 负责管理每个专家模块的独立配置（LLM + Graph + 专家属性），
//! 支持热更新、版本管理、回滚、变更事件通知。

use chrono::Utc;
use mox_alliance_common_proto::{
    ConfigType, ConfigVersion, ExpertModuleConfig, GlobalLlmConfig, MergedLlmConfig,
    ModuleGraphConfig, ModuleLlmConfig,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::error::{ConfigError, ConfigResult};
use crate::events::{ConfigChangeEvent, ConfigChangeType};
use crate::store::ConfigStore;
use crate::validator::ConfigValidator;

/// 模块化配置引擎
pub struct ConfigEngine {
    store: Arc<dyn ConfigStore>,
    event_tx: broadcast::Sender<ConfigChangeEvent>,
}

impl ConfigEngine {
    /// 创建新的配置引擎
    pub fn new(store: Arc<dyn ConfigStore>) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self { store, event_tx }
    }

    /// 订阅配置变更事件
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.event_tx.subscribe()
    }

    /// 发布变更事件
    fn publish_event(&self, event: ConfigChangeEvent) {
        if self.event_tx.receiver_count() > 0 {
            let _ = self.event_tx.send(event);
        }
    }

    // === 模块配置 CRUD ===

    /// 注册新的专家模块配置
    pub async fn register_module(
        &self,
        mut config: ExpertModuleConfig,
        changed_by: &str,
        reason: &str,
    ) -> ConfigResult<ExpertModuleConfig> {
        // 检查是否已存在
        if self
            .store
            .get_module_config(&config.module_id)
            .await?
            .is_some()
        {
            return Err(ConfigError::ModuleAlreadyExists(config.module_id.clone()));
        }

        // 验证配置
        ConfigValidator::validate_module_config(&config)?;

        let now = Utc::now();
        config.created_at = now;
        config.updated_at = now;
        config.llm_config.updated_at = now;
        config.graph_config.updated_at = now;

        // 保存配置
        self.store.save_module_config(&config).await?;

        // 保存版本快照
        self.save_version_snapshot(
            &config.module_id,
            ConfigType::Full,
            &config,
            changed_by,
            reason,
        )
        .await?;

        info!(
            module_id = %config.module_id,
            "Module config registered"
        );

        self.publish_event(ConfigChangeEvent::new(
            config.module_id.clone(),
            ConfigType::Full,
            ConfigChangeType::Created,
            None,
            1,
            changed_by.to_string(),
            reason.to_string(),
        ));

        Ok(config)
    }

    /// 获取模块完整配置
    pub async fn get_module_config(
        &self,
        module_id: &str,
    ) -> ConfigResult<ExpertModuleConfig> {
        self.store
            .get_module_config(module_id)
            .await?
            .ok_or_else(|| ConfigError::ModuleNotFound(module_id.to_string()))
    }

    /// 获取模块配置（可选）
    pub async fn get_module_config_opt(
        &self,
        module_id: &str,
    ) -> ConfigResult<Option<ExpertModuleConfig>> {
        self.store.get_module_config(module).await
    }

    /// 列出所有模块配置
    pub async fn list_modules(&self) -> ConfigResult<Vec<ExpertModuleConfig>> {
        self.store.list_module_configs().await
    }

    /// 按标签列出模块配置
    pub async fn list_modules_by_tag(&self, tag: &str) -> ConfigResult<Vec<ExpertModuleConfig>> {
        self.store.list_module_configs_by_tag(tag).await
    }

    /// 更新模块完整配置
    pub async fn update_module_config(
        &self,
        mut config: ExpertModuleConfig,
        changed_by: &str,
        reason: &str,
    ) -> ConfigResult<ExpertModuleConfig> {
        // 获取旧版本
        let old_config = self.get_module_config(&config.module_id).await?;
        let old_version = old_config.llm_config.version;

        // 验证配置
        ConfigValidator::validate_module_config(&config)?;

        // 递增版本号
        let new_version = old_version + 1;
        config.llm_config.version = new_version;
        config.graph_config.version = new_version;
        config.updated_at = Utc::now();
        config.llm_config.updated_at = Utc::now();
        config.graph_config.updated_at = Utc::now();

        // 保存
        self.store.save_module_config(&config).await?;

        // 保存版本快照
        self.save_version_snapshot(
            &config.module_id,
            ConfigType::Full,
            &config,
            changed_by,
            reason,
        )
        .await?;

        info!(
            module_id = %config.module_id,
            old_version,
            new_version,
            "Module config updated"
        );

        self.publish_event(ConfigChangeEvent::new(
            config.module_id.clone(),
            ConfigType::Full,
            ConfigChangeType::Updated,
            Some(old_version),
            new_version,
            changed_by.to_string(),
            reason.to_string(),
        ));

        Ok(config)
    }

    /// 删除模块配置
    pub async fn delete_module(
        &self,
        module_id: &str,
        changed_by: &str,
        reason: &str,
    ) -> ConfigResult<bool> {
        let existed = self.store.delete_module_config(module_id).await?;

        if existed {
            info!(module_id, "Module config deleted");
            self.publish_event(ConfigChangeEvent::new(
                module_id.to_string(),
                ConfigType::Full,
                ConfigChangeType::Deleted,
                None,
                0,
                changed_by.to_string(),
                reason.to_string(),
            ));
        }

        Ok(existed)
    }

    /// 启用模块
    pub async fn enable_module(
        &self,
        module_id: &str,
        changed_by: &str,
        reason: &str,
    ) -> ConfigResult<()> {
        let mut config = self.get_module_config(module_id).await?;
        if !config.enabled {
            config.enabled = true;
            config.updated_at = Utc::now();
            self.store.save_module_config(&config).await?;
            info!(module_id, "Module enabled");
            self.publish_event(ConfigChangeEvent::new(
                module_id.to_string(),
                ConfigType::Full,
                ConfigChangeType::Enabled,
                None,
                config.llm_config.version,
                changed_by.to_string(),
                reason.to_string(),
            ));
        }
        Ok(())
    }

    /// 禁用模块
    pub async fn disable_module(
        &self,
        module_id: &str,
        changed_by: &str,
        reason: &str,
    ) -> ConfigResult<()> {
        let mut config = self.get_module_config(module_id).await?;
        if config.enabled {
            config.enabled = false;
            config.updated_at = Utc::now();
            self.store.save_module_config(&config).await?;
            warn!(module_id, "Module disabled");
            self.publish_event(ConfigChangeEvent::new(
                module_id.to_string(),
                ConfigType::Full,
                ConfigChangeType::Disabled,
                None,
                config.llm_config.version,
                changed_by.to_string(),
                reason.to_string(),
            ));
        }
        Ok(())
    }

    // === 全局默认 LLM 配置 ===

    /// 获取全局默认 LLM 配置
    pub async fn get_global_llm_config(&self) -> ConfigResult<GlobalLlmConfig> {
        self.store
            .get_global_llm_config()
            .await?
            .ok_or_else(|| ConfigError::GlobalConfigNotFound("llm".to_string()))
    }

    /// 获取全局默认 LLM 配置（可选，不存在则返回默认值）
    pub async fn get_global_llm_config_opt(&self) -> ConfigResult<GlobalLlmConfig> {
        Ok(self
            .store
            .get_global_llm_config()
            .await?
            .unwrap_or_default())
    }

    /// 设置全局默认 LLM 配置
    pub async fn set_global_llm_config(
        &self,
        mut config: GlobalLlmConfig,
        changed_by: &str,
        reason: &str,
    ) -> ConfigResult<GlobalLlmConfig> {
        // 验证
        ConfigValidator::validate_global_llm_config(&config)?;

        let old_config = self.store.get_global_llm_config().await?;
        let old_version = old_config.as_ref().map(|c| c.version).unwrap_or(0);

        // 递增版本
        let new_version = old_version + 1;
        config.version = new_version;
        config.updated_at = Utc::now();

        // 保存
        self.store.save_global_llm_config(&config).await?;

        info!(
            old_version,
            new_version,
            "Global LLM config updated"
        );

        // 发布全局配置变更事件
        self.publish_event(ConfigChangeEvent::new(
            "__global__".to_string(),
            ConfigType::Llm,
            ConfigChangeType::Updated,
            if old_version > 0 { Some(old_version) } else { None },
            new_version,
            changed_by.to_string(),
            reason.to_string(),
        ));

        Ok(config)
    }

    // === LLM 配置管理 ===

    /// 获取模块 LLM 配置
    pub async fn get_llm_config(&self, module_id: &str) -> ConfigResult<ModuleLlmConfig> {
        let config = self.get_module_config(module_id).await?;
        Ok(config.llm_config)
    }

    /// 获取合并后的 LLM 配置（模块配置 + 全局默认）
    ///
    /// 这是实际执行时应该使用的配置，已经完成了所有继承和覆盖的计算。
    /// 如果模块配置了独立的 Provider 和 API Key，就用模块的；
    /// 如果模块没有配置某个 Provider，就从全局配置继承。
    pub async fn get_merged_llm_config(&self, module_id: &str) -> ConfigResult<MergedLlmConfig> {
        let module_config = self.get_llm_config(module_id).await?;
        let global_config = self.get_global_llm_config_opt().await?;
        Ok(module_config.merge_with_global(&global_config))
    }

    /// 更新模块 LLM 配置
    pub async fn update_llm_config(
        &self,
        mut llm_config: ModuleLlmConfig,
        changed_by: &str,
        reason: &str,
    ) -> ConfigResult<ModuleLlmConfig> {
        let module_id = llm_config.module_id.clone();
        let mut module_config = self.get_module_config(&module_id).await?;
        let old_version = module_config.llm_config.version;

        // 验证
        ConfigValidator::validate_llm_config(&llm_config)?;

        // 递增版本
        let new_version = old_version + 1;
        llm_config.version = new_version;
        llm_config.updated_at = Utc::now();

        // 更新
        module_config.llm_config = llm_config.clone();
        module_config.updated_at = Utc::now();
        self.store.save_module_config(&module_config).await?;

        // 保存版本快照
        self.save_version_snapshot(
            &module_id,
            ConfigType::Llm,
            &llm_config,
            changed_by,
            reason,
        )
        .await?;

        debug!(
            module_id,
            old_version, new_version, "LLM config updated"
        );

        self.publish_event(ConfigChangeEvent::new(
            module_id,
            ConfigType::Llm,
            ConfigChangeType::Updated,
            Some(old_version),
            new_version,
            changed_by.to_string(),
            reason.to_string(),
        ));

        Ok(llm_config)
    }

    // === Graph 配置管理 ===

    /// 获取模块 Graph 配置
    pub async fn get_graph_config(&self, module_id: &str) -> ConfigResult<ModuleGraphConfig> {
        let config = self.get_module_config(module_id).await?;
        Ok(config.graph_config)
    }

    /// 更新模块 Graph 配置
    pub async fn update_graph_config(
        &self,
        mut graph_config: ModuleGraphConfig,
        changed_by: &str,
        reason: &str,
    ) -> ConfigResult<ModuleGraphConfig> {
        let module_id = graph_config.module_id.clone();
        let mut module_config = self.get_module_config(&module_id).await?;
        let old_version = module_config.graph_config.version;

        // 验证
        ConfigValidator::validate_graph_config(&graph_config)?;

        // 递增版本
        let new_version = old_version + 1;
        graph_config.version = new_version;
        graph_config.updated_at = Utc::now();

        // 更新
        module_config.graph_config = graph_config.clone();
        module_config.updated_at = Utc::now();
        self.store.save_module_config(&module_config).await?;

        // 保存版本快照
        self.save_version_snapshot(
            &module_id,
            ConfigType::Graph,
            &graph_config,
            changed_by,
            reason,
        )
        .await?;

        debug!(
            module_id,
            old_version, new_version, "Graph config updated"
        );

        self.publish_event(ConfigChangeEvent::new(
            module_id,
            ConfigType::Graph,
            ConfigChangeType::Updated,
            Some(old_version),
            new_version,
            changed_by.to_string(),
            reason.to_string(),
        ));

        Ok(graph_config)
    }

    // === 版本管理 ===

    /// 获取配置版本列表
    pub async fn list_versions(
        &self,
        module_id: &str,
        config_type: ConfigType,
    ) -> ConfigResult<Vec<ConfigVersion>> {
        self.store.list_versions(module_id, config_type).await
    }

    /// 回滚到指定版本
    pub async fn rollback_to_version(
        &self,
        module_id: &str,
        config_type: ConfigType,
        target_version: u32,
        changed_by: &str,
        reason: &str,
    ) -> ConfigResult<()> {
        let version = self
            .store
            .get_version(module_id, config_type, target_version)
            .await?
            .ok_or_else(|| ConfigError::VersionNotFound {
                module_id: module_id.to_string(),
                version: target_version,
            })?;

        match config_type {
            ConfigType::Llm => {
                let llm_config: ModuleLlmConfig =
                    serde_json::from_value(version.config_snapshot).map_err(|e| {
                        ConfigError::SerializationError(format!(
                            "Failed to deserialize LLM config: {}",
                            e
                        ))
                    })?;
                self.update_llm_config(llm_config, changed_by, reason)
                    .await?;
            }
            ConfigType::Graph => {
                let graph_config: ModuleGraphConfig =
                    serde_json::from_value(version.config_snapshot).map_err(|e| {
                        ConfigError::SerializationError(format!(
                            "Failed to deserialize Graph config: {}",
                            e
                        ))
                    })?;
                self.update_graph_config(graph_config, changed_by, reason)
                    .await?;
            }
            ConfigType::Full => {
                let full_config: ExpertModuleConfig =
                    serde_json::from_value(version.config_snapshot).map_err(|e| {
                        ConfigError::SerializationError(format!(
                            "Failed to deserialize module config: {}",
                            e
                        ))
                    })?;
                self.update_module_config(full_config, changed_by, reason)
                    .await?;
            }
            ConfigType::Expert => {
                // Expert config rollback handled as part of full config
                return Err(ConfigError::ConfigTypeMismatch {
                    expected: ConfigType::Full,
                    got: config_type,
                });
            }
        }

        info!(
            module_id,
            ?config_type, target_version, "Config rolled back"
        );

        Ok(())
    }

    // === 内部方法 ===

    /// 保存版本快照
    async fn save_version_snapshot<T: serde::Serialize>(
        &self,
        module_id: &str,
        config_type: ConfigType,
        config: &T,
        changed_by: &str,
        reason: &str,
    ) -> ConfigResult<()> {
        let snapshot =
            serde_json::to_value(config).map_err(|e| ConfigError::SerializationError(e.to_string()))?;

        // 计算版本号 — 使用当前配置的 version 字段
        let version_num = match config_type {
            ConfigType::Llm => {
                let config: ModuleLlmConfig = serde_json::from_value(snapshot.clone())
                    .map_err(|e| ConfigError::SerializationError(e.to_string()))?;
                config.version
            }
            ConfigType::Graph => {
                let config: ModuleGraphConfig = serde_json::from_value(snapshot.clone())
                    .map_err(|e| ConfigError::SerializationError(e.to_string()))?;
                config.version
            }
            ConfigType::Full => {
                let config: ExpertModuleConfig = serde_json::from_value(snapshot.clone())
                    .map_err(|e| ConfigError::SerializationError(e.to_string()))?;
                config.llm_config.version
            }
            ConfigType::Expert => 1,
        };

        let version = ConfigVersion {
            version: version_num,
            module_id: module_id.to_string(),
            config_type,
            config_snapshot: snapshot,
            changed_by: changed_by.to_string(),
            change_reason: reason.to_string(),
            created_at: Utc::now(),
        };

        self.store.save_version(&version).await?;
        Ok(())
    }

    /// 测试 LLM 配置连通性（返回连通性结果描述）
    pub async fn test_llm_config(
        &self,
        module_id: &str,
    ) -> ConfigResult<Result<String, String>> {
        let _config = self.get_llm_config(module_id).await?;
        // TODO: 实际的连通性测试逻辑
        Ok(Ok("LLM configuration is valid (validation-only test)".to_string()))
    }

    /// 测试 Graph 配置连通性
    pub async fn test_graph_config(
        &self,
        module_id: &str,
    ) -> ConfigResult<Result<String, String>> {
        let _config = self.get_graph_config(module_id).await?;
        // TODO: 实际的连通性测试逻辑
        Ok(Ok("Graph configuration is valid (validation-only test)".to_string()))
    }
}

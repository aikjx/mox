// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 配置存储抽象与内存实现

use async_trait::async_trait;
use mox_alliance_common_proto::{
    ConfigType, ConfigVersion, ExpertModuleConfig, GlobalLlmConfig, ModuleGraphConfig,
    ModuleLlmConfig,
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{ConfigError, ConfigResult};

/// 配置存储 trait — 抽象配置的持久化存储
#[async_trait]
pub trait ConfigStore: Send + Sync {
    // === 全局默认 LLM 配置 ===

    /// 获取全局默认 LLM 配置
    async fn get_global_llm_config(&self) -> ConfigResult<Option<GlobalLlmConfig>>;

    /// 保存全局默认 LLM 配置
    async fn save_global_llm_config(&self, config: &GlobalLlmConfig) -> ConfigResult<()>;

    // === 专家模块完整配置 ===

    /// 获取专家模块完整配置
    async fn get_module_config(&self, module_id: &str) -> ConfigResult<Option<ExpertModuleConfig>>;

    /// 保存专家模块完整配置
    async fn save_module_config(&self, config: &ExpertModuleConfig) -> ConfigResult<()>;

    /// 删除专家模块配置
    async fn delete_module_config(&self, module_id: &str) -> ConfigResult<bool>;

    /// 列出所有模块配置
    async fn list_module_configs(&self) -> ConfigResult<Vec<ExpertModuleConfig>>;

    /// 按标签筛选模块配置
    async fn list_module_configs_by_tag(&self, tag: &str) -> ConfigResult<Vec<ExpertModuleConfig>>;

    // === LLM 配置 ===

    /// 获取模块 LLM 配置
    async fn get_llm_config(&self, module_id: &str) -> ConfigResult<Option<ModuleLlmConfig>>;

    /// 更新模块 LLM 配置
    async fn update_llm_config(&self, config: &ModuleLlmConfig) -> ConfigResult<()>;

    // === Graph 配置 ===

    /// 获取模块 Graph 配置
    async fn get_graph_config(&self, module_id: &str) -> ConfigResult<Option<ModuleGraphConfig>>;

    /// 更新模块 Graph 配置
    async fn update_graph_config(&self, config: &ModuleGraphConfig) -> ConfigResult<()>;

    // === 版本管理 ===

    /// 保存配置版本快照
    async fn save_version(&self, version: &ConfigVersion) -> ConfigResult<()>;

    /// 获取指定模块的配置版本列表
    async fn list_versions(
        &self,
        module_id: &str,
        config_type: ConfigType,
    ) -> ConfigResult<Vec<ConfigVersion>>;

    /// 获取指定版本的配置快照
    async fn get_version(
        &self,
        module_id: &str,
        config_type: ConfigType,
        version: u32,
    ) -> ConfigResult<Option<ConfigVersion>>;
}

/// 内存配置存储 — 用于测试和轻量部署场景
pub struct MemoryConfigStore {
    global_llm: parking_lot::RwLock<Option<GlobalLlmConfig>>,
    modules: parking_lot::RwLock<HashMap<String, ExpertModuleConfig>>,
    versions: parking_lot::RwLock<Vec<ConfigVersion>>,
}

impl MemoryConfigStore {
    pub fn new() -> Self {
        Self {
            global_llm: parking_lot::RwLock::new(None),
            modules: parking_lot::RwLock::new(HashMap::new()),
            versions: parking_lot::RwLock::new(Vec::new()),
        }
    }

    pub fn arc() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl Default for MemoryConfigStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConfigStore for MemoryConfigStore {
    async fn get_global_llm_config(&self) -> ConfigResult<Option<GlobalLlmConfig>> {
        Ok(self.global_llm.read().clone())
    }

    async fn save_global_llm_config(&self, config: &GlobalLlmConfig) -> ConfigResult<()> {
        *self.global_llm.write() = Some(config.clone());
        Ok(())
    }

    async fn get_module_config(&self, module_id: &str) -> ConfigResult<Option<ExpertModuleConfig>> {
        Ok(self.modules.read().get(module_id).cloned())
    }

    async fn save_module_config(&self, config: &ExpertModuleConfig) -> ConfigResult<()> {
        self.modules
            .write()
            .insert(config.module_id.clone(), config.clone());
        Ok(())
    }

    async fn delete_module_config(&self, module_id: &str) -> ConfigResult<bool> {
        Ok(self.modules.write().remove(module_id).is_some())
    }

    async fn list_module_configs(&self) -> ConfigResult<Vec<ExpertModuleConfig>> {
        Ok(self.modules.read().values().cloned().collect())
    }

    async fn list_module_configs_by_tag(
        &self,
        tag: &str,
    ) -> ConfigResult<Vec<ExpertModuleConfig>> {
        Ok(self
            .modules
            .read()
            .values()
            .filter(|c| c.tags.iter().any(|t| t == tag))
            .cloned()
            .collect())
    }

    async fn get_llm_config(&self, module_id: &str) -> ConfigResult<Option<ModuleLlmConfig>> {
        Ok(self
            .modules
            .read()
            .get(module_id)
            .map(|c| c.llm_config.clone()))
    }

    async fn update_llm_config(&self, config: &ModuleLlmConfig) -> ConfigResult<()> {
        let mut modules = self.modules.write();
        if let Some(module) = modules.get_mut(&config.module_id) {
            module.llm_config = config.clone();
            Ok(())
        } else {
            Err(ConfigError::ModuleNotFound(config.module_id.clone()))
        }
    }

    async fn get_graph_config(&self, module_id: &str) -> ConfigResult<Option<ModuleGraphConfig>> {
        Ok(self
            .modules
            .read()
            .get(module_id)
            .map(|c| c.graph_config.clone()))
    }

    async fn update_graph_config(&self, config: &ModuleGraphConfig) -> ConfigResult<()> {
        let mut modules = self.modules.write();
        if let Some(module) = modules.get_mut(&config.module_id) {
            module.graph_config = config.clone();
            Ok(())
        } else {
            Err(ConfigError::ModuleNotFound(config.module_id.clone()))
        }
    }

    async fn save_version(&self, version: &ConfigVersion) -> ConfigResult<()> {
        self.versions.write().push(version.clone());
        Ok(())
    }

    async fn list_versions(
        &self,
        module_id: &str,
        config_type: ConfigType,
    ) -> ConfigResult<Vec<ConfigVersion>> {
        let mut result: Vec<ConfigVersion> = self
            .versions
            .read()
            .iter()
            .filter(|v| v.module_id == module_id && v.config_type == config_type)
            .cloned()
            .collect();
        result.sort_by_key(|v| std::cmp::Reverse(v.version));
        Ok(result)
    }

    async fn get_version(
        &self,
        module_id: &str,
        config_type: ConfigType,
        version: u32,
    ) -> ConfigResult<Option<ConfigVersion>> {
        Ok(self
            .versions
            .read()
            .iter()
            .find(|v| v.module_id == module_id && v.config_type == config_type && v.version == version)
            .cloned())
    }
}

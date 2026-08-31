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
use std::time::Duration;
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

        // 继承全局 provider（模块未显式声明的 provider 回退到全局默认，符合配置继承语义）
        let global = self.get_global_llm_config_opt().await?;
        inherit_global_providers(&mut config.llm_config, &global);

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
        self.store.get_module_config(module_id).await
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
    ///
    /// 真实探测逻辑（非校验占位）：
    /// 1. **配置完整性**：主 Provider 必须存在，API Key 必须可解析（EnvVar 已设 / PlainText 非空）
    /// 2. **端点合法性**：base_url（若配置）必须是合法 http(s) URL
    /// 3. **网络可达性**：对 base_url 的 host:port 发起真实 TCP 连接探测（3s 超时）
    ///    未配置 base_url 时跳过网络探测（使用 SDK 内建默认端点，仅做配置校验）
    ///
    /// 返回值内层：`Ok(通过描述)` 或 `Err(失败原因)`。
    pub async fn test_llm_config(
        &self,
        module_id: &str,
    ) -> ConfigResult<Result<String, String>> {
        let config = self.get_llm_config(module_id).await?;
        let primary = &config.primary_provider;

        // 主 Provider 必须存在
        let Some(provider) = config
            .provider_options
            .iter()
            .find(|p| &p.provider_id == primary)
        else {
            return Ok(Err(format!(
                "主 Provider '{primary}' 未在 provider_options 中配置（已配置: {}）",
                config
                    .provider_options
                    .iter()
                    .map(|p| p.provider_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        };

        let api_key_ok = provider.api_key_source.resolve_api_key().is_some();
        let header = format!("[{}] ", provider.provider_id);

        match &provider.base_url {
            Some(url) => match probe_url_connectivity(url).await {
                Ok(detail) => {
                    if api_key_ok {
                        Ok(Ok(format!("{header}API Key ✓；端点 {url} 可达（{detail}）")))
                    } else {
                        Ok(Ok(format!(
                            "{header}端点 {url} 可达（{detail}），但 API Key 未解析到（环境变量未设置或非 PlainText）——功能不可用"
                        )))
                    }
                }
                Err(e) => Ok(Err(format!("{header}端点 {url} 不可达：{e}"))),
            },
            None => {
                // 无自定义 base_url：SDK 内建默认端点，跳过网络探测
                if api_key_ok {
                    Ok(Ok(format!(
                        "{header}配置完整（API Key ✓，使用 SDK 内建默认端点，跳过网络探测）"
                    )))
                } else {
                    Ok(Err(format!(
                        "{header}API Key 未解析到（环境变量未设置或非 PlainText）——请先配置密钥"
                    )))
                }
            }
        }
    }

    /// 测试 Graph 配置连通性
    ///
    /// 真实探测逻辑：
    /// 1. **连接 URI 解析**：优先 `custom_endpoint`，其次读取 `connection.uri_env` 环境变量
    /// 2. **凭据完整性**：声明了 user/password 环境变量时必须已设置
    /// 3. **网络可达性**：对 URI 的 host:port 发起真实 TCP 连接探测（3s 超时）
    ///
    /// 返回值内层：`Ok(通过描述)` 或 `Err(失败原因)`。
    pub async fn test_graph_config(
        &self,
        module_id: &str,
    ) -> ConfigResult<Result<String, String>> {
        let config = self.get_graph_config(module_id).await?;

        // 1) 解析连接 URI
        let uri = if let Some(endpoint) = &config.custom_endpoint {
            Some(endpoint.clone())
        } else {
            std::env::var(&config.connection.uri_env).ok()
        };
        let Some(uri) = uri else {
            return Ok(Err(format!(
                "Graph 连接 URI 未解析到（custom_endpoint 未配置，且环境变量 '{}' 未设置）",
                config.connection.uri_env
            )));
        };

        // 2) 凭据完整性
        let mut missing = Vec::new();
        if let Some(env) = &config.connection.user_env {
            if std::env::var(env).is_err() {
                missing.push(format!("用户环境变量 '{env}'"));
            }
        }
        if let Some(env) = &config.connection.password_env {
            if std::env::var(env).is_err() {
                missing.push(format!("密码环境变量 '{env}'"));
            }
        }

        // 3) 网络可达性
        match probe_url_connectivity(&uri).await {
            Ok(detail) => {
                let mut report = format!("Graph 端点 {uri} 可达（{detail}）");
                if !missing.is_empty() {
                    report.push_str(&format!("；但缺少凭据：{}", missing.join("、")));
                }
                Ok(Ok(report))
            }
            Err(e) => Ok(Err(format!("Graph 端点 {uri} 不可达：{e}"))),
        }
    }
}

/// 对 URL 的 host:port 发起真实 TCP 连接探测（默认端口：https=443 / http=80 / bolt=7687；3s 超时）
async fn probe_url_connectivity(url: &str) -> Result<String, String> {
    let (host, port) = parse_host_port(url)?;
    let addr = format!("{host}:{port}");
    let connect = tokio::net::TcpStream::connect(&addr);
    match tokio::time::timeout(Duration::from_secs(3), connect).await {
        Ok(Ok(_)) => Ok(format!("TCP {addr} 连接成功")),
        Ok(Err(e)) => Err(format!("TCP {addr} 连接失败：{e}")),
        Err(_) => Err(format!("TCP {addr} 连接超时（3s）")),
    }
}

/// 从 URL 解析 host 与端口
///
/// 支持：`https://host:port/path`、`http://host`（默认 80）、`bolt://host:7687`、裸 `host:port`。
/// 无协议前缀且无显式端口时返回错误（无法确定端口）。
fn parse_host_port(url: &str) -> Result<(String, u16), String> {
    let trimmed = url.trim();
    let (scheme, rest) = match trimmed.split_once("://") {
        Some((s, r)) => (s, r),
        None => ("", trimmed),
    };
    let default_port = match scheme {
        "https" => Some(443u16),
        "http" => Some(80u16),
        "bolt" | "neo4j" => Some(7687u16),
        "" => None, // 裸 host:port，无默认端口
        other => {
            return Err(format!(
                "不支持的协议 '{other}'（仅支持 http/https/bolt/neo4j 或裸 host:port）"
            ))
        }
    };
    let host_port = rest.split(['/', '?', '#']).next().unwrap_or("");
    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => (
            h.to_string(),
            Some(p.parse::<u16>().map_err(|_| format!("端口无效：{p}"))?),
        ),
        _ => (host_port.to_string(), None),
    };
    let port = match port.or(default_port) {
        Some(p) => p,
        None => return Err(format!("无法从 '{url}' 确定端口（请显式指定 host:port）")),
    };
    if host.is_empty() {
        return Err(format!("URL '{url}' 缺少主机名"));
    }
    Ok((host, port))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_common_proto::{
        ApiKeySource, ExpertModuleConfig, GraphConnectionConfig, GraphEngineType,
        GraphQueryConfig, GraphSchemaConfig, LlmProviderOption, LlmRoutingStrategy,
        MatchingWeights, ModelConfig, ModuleGraphConfig, ModuleLlmConfig,
    };
    use std::net::TcpListener;
    use std::sync::Arc;

    use crate::store::MemoryConfigStore;

    fn make_llm_module(
        module_id: &str,
        provider_id: &str,
        api_key_source: ApiKeySource,
        base_url: Option<String>,
    ) -> ExpertModuleConfig {
        ExpertModuleConfig {
            module_id: module_id.to_string(),
            expert_id: format!("expert-{module_id}"),
            name: module_id.to_string(),
            version: "1.0.0".to_string(),
            llm_config: ModuleLlmConfig {
                module_id: module_id.to_string(),
                primary_provider: provider_id.to_string(),
                primary_model: "test-model".to_string(),
                fallback_chain: vec![],
                routing_strategy: LlmRoutingStrategy::Priority,
                model_config: ModelConfig::default(),
                provider_options: vec![LlmProviderOption {
                    provider_id: provider_id.to_string(),
                    display_name: Some(provider_id.to_string()),
                    api_key_source,
                    base_url,
                    default_model: Some("test-model".to_string()),
                    supported_models: vec![],
                    price_per_1k_tokens: None,
                    rpm_limit: None,
                    tpm_limit: None,
                    enabled: true,
                }],
                system_prompt_template: None,
                use_global_prompt_prefix: true,
                version: 1,
                updated_at: chrono::Utc::now(),
            },
            graph_config: ModuleGraphConfig {
                module_id: module_id.to_string(),
                engine_type: GraphEngineType::RelGraph,
                connection: GraphConnectionConfig {
                    uri_env: "RELGRAPH_URI".to_string(),
                    user_env: None,
                    password_env: None,
                    database: None,
                },
                query_config: GraphQueryConfig::default(),
                schema: GraphSchemaConfig::default(),
                custom_endpoint: None,
                version: 1,
                updated_at: chrono::Utc::now(),
            },
            capability_weights: std::collections::HashMap::new(),
            matching_weights: MatchingWeights::default(),
            enabled: true,
            tags: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn listen_addr() -> (TcpListener, String, u16) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let addr = listener.local_addr().expect("local addr");
        // 返回 listener 并保持存活，端口在整个测试期间持续监听
        (listener, addr.ip().to_string(), addr.port())
    }

    #[test]
    fn parse_host_port_supports_common_urls() {
        assert_eq!(parse_host_port("https://api.openai.com/v1").unwrap(), ("api.openai.com".into(), 443));
        assert_eq!(parse_host_port("http://127.0.0.1:8080").unwrap(), ("127.0.0.1".into(), 8080));
        assert_eq!(parse_host_port("bolt://graph.local:7687").unwrap(), ("graph.local".into(), 7687));
        assert_eq!(parse_host_port("graph.local:7687").unwrap(), ("graph.local".into(), 7687));
        // 无协议且无端口 → 无法确定端口
        assert!(parse_host_port("graph.local").is_err());
        // 非法协议 → 报错
        assert!(parse_host_port("ftp://host:21").is_err());
    }

    #[tokio::test]
    async fn probe_detects_reachable_and_unreachable() {
        let (_listener, host, port) = listen_addr();
        let url = format!("http://{host}:{port}");
        assert!(probe_url_connectivity(&url).await.is_ok(), "监听中的端口应可达");

        // 未监听端口（取一个几乎不可能被占用的高位端口）
        let url = format!("http://{host}:65530");
        assert!(probe_url_connectivity(&url).await.is_err(), "未监听端口应不可达");
    }

    #[tokio::test]
    async fn llm_config_probe_reachable_with_plain_key() {
        let (_listener, host, port) = listen_addr();
        let engine = ConfigEngine::new(Arc::new(MemoryConfigStore::default()));
        let cfg = make_llm_module(
            "test-llm-ok",
            "local",
            ApiKeySource::from_plain("sk-test"),
            Some(format!("http://{host}:{port}")),
        );
        engine.register_module(cfg, "tester", "test").await.unwrap();

        let result = engine.test_llm_config("test-llm-ok").await.unwrap();
        let msg = result.unwrap_or_else(|e| panic!("应通过连通性测试，实际失败：{e}"));
        assert!(msg.contains("API Key ✓"), "消息应含 API Key ✓，实际：{msg}");
        assert!(msg.contains("可达"), "消息应含可达，实际：{msg}");
    }

    #[tokio::test]
    async fn llm_config_probe_missing_key_no_endpoint() {
        let engine = ConfigEngine::new(Arc::new(MemoryConfigStore::default()));
        // 无 base_url + EnvVar(未设置) → 内层 Err
        let cfg = make_llm_module(
            "test-llm-nokey",
            "openai",
            ApiKeySource::from_env("CONFIG_ENGINE_TEST_UNSET_KEY"),
            None,
        );
        engine.register_module(cfg, "tester", "test").await.unwrap();

        let result = engine.test_llm_config("test-llm-nokey").await.unwrap();
        assert!(result.is_err(), "缺少 API Key 时应报告失败，实际：{result:?}");
    }

    #[tokio::test]
    async fn graph_config_probe_custom_endpoint_reachable() {
        let (_listener, host, port) = listen_addr();
        let engine = ConfigEngine::new(Arc::new(MemoryConfigStore::default()));
        let mut cfg = make_llm_module("test-graph-ok", "local", ApiKeySource::from_plain("k"), None);
        cfg.graph_config.custom_endpoint = Some(format!("http://{host}:{port}"));
        engine.register_module(cfg, "tester", "test").await.unwrap();

        let result = engine.test_graph_config("test-graph-ok").await.unwrap();
        let msg = result.unwrap_or_else(|e| panic!("Graph 连通性应通过，实际失败：{e}"));
        assert!(msg.contains("可达"), "消息应含可达，实际：{msg}");
    }



}

/// 将全局配置中模块未显式声明的 provider 继承到模块配置（回退到全局默认语义）。
///
/// 模块的 primary_provider 与 allback_chain 可引用全局默认 provider（如 anthropic/qwen），
/// 此时从全局 provider_options 补齐到模块级，使模块配置自洽且可独立校验。
fn inherit_global_providers(llm: &mut ModuleLlmConfig, global: &GlobalLlmConfig) {
    let mut needed: Vec<String> = Vec::new();
    if !llm.primary_provider.is_empty() {
        needed.push(llm.primary_provider.clone());
    }
    for f in &llm.fallback_chain {
        needed.push(f.clone());
    }
    for pid in needed {
        if llm.provider_options.iter().any(|p| p.provider_id == pid) {
            continue;
        }
        if let Some(gp) = global.provider_options.iter().find(|p| p.provider_id == pid) {
            llm.provider_options.push(gp.clone());
        }
    }
}

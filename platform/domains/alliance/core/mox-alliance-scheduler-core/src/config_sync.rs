// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 配置同步桥接器
//!
//! 将模块化配置引擎（ConfigEngine）与专家匹配器连接起来，
//! 实现配置变更的自动同步和动态生效。
//!
//! ## 功能
//! - 从配置引擎订阅配置变更事件
//! - 将匹配权重（MatchingWeights）同步到模块化匹配器
//! - 支持全量同步和增量同步
//! - 配置变更后匹配算法立即生效

use mox_alliance_common_proto::{AllianceResult, ExpertModuleConfig, MatchingWeights};
use mox_alliance_config_core::{ConfigChangeEvent, ConfigChangeType, ConfigEngine, ConfigType};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::modular_matcher::ModularWeightMatcher;

/// 配置同步器
///
/// 监听配置引擎的变更事件，将匹配权重等配置同步到模块化匹配器。
/// 这是连接配置层和匹配层的关键桥梁。
pub struct ConfigSynchronizer {
    /// 配置引擎引用
    config_engine: Arc<ConfigEngine>,
    /// 模块化匹配器引用
    matcher: Arc<ModularWeightMatcher>,
    /// 模块 ID 到专家 ID 的映射缓存 (module_id -> expert_id)
    module_to_expert: Arc<RwLock<HashMap<String, String>>>,
    /// 是否正在运行
    running: Arc<RwLock<bool>>,
}

impl ConfigSynchronizer {
    /// 创建配置同步器
    pub fn new(config_engine: Arc<ConfigEngine>, matcher: Arc<ModularWeightMatcher>) -> Self {
        Self {
            config_engine,
            matcher,
            module_to_expert: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 全量同步：从配置引擎加载所有模块配置并同步到匹配器
    ///
    /// 启动时调用，确保匹配器拥有最新的权重配置。
    pub async fn full_sync(&self) -> AllianceResult<usize> {
        info!("Starting full config sync...");

        // 获取所有模块配置
        let modules = self.config_engine.list_modules().await.map_err(|e| {
            error!("Failed to list modules from config engine: {}", e);
            mox_alliance_common_proto::AllianceError::internal(format!(
                "Config engine error: {}",
                e
            ))
        })?;

        let mut synced = 0;
        let mut module_map = self.module_to_expert.write();
        module_map.clear();

        for module in &modules {
            if !module.enabled {
                debug!("Skipping disabled module: {}", module.module_id);
                continue;
            }

            // 同步匹配权重
            self.matcher
                .set_expert_weights(&module.expert_id, module.matching_weights.clone());

            // 记录模块到专家的映射
            module_map.insert(module.module_id.clone(), module.expert_id.clone());

            synced += 1;
            debug!(
                "Synced weights for expert {} (module: {})",
                module.expert_id, module.module_id
            );
        }

        info!("Full sync completed: {} modules synced", synced);
        Ok(synced)
    }

    /// 启动事件监听循环（异步任务）
    ///
    /// 订阅配置引擎的变更事件，实时同步到匹配器。
    /// 返回的 JoinHandle 可用于取消监听。
    pub fn start_listening(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let mut rx = self.config_engine.subscribe();
        let sync = self.clone();
        *sync.running.write() = true;

        info!("Config synchronizer event listener started");

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if !*sync.running.read() {
                            break;
                        }
                        if let Err(e) = sync.handle_config_event(event).await {
                            error!("Error handling config event: {}", e);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Config event channel lagged, {} events missed", n);
                        // 触发全量同步以恢复一致性
                        if let Err(e) = sync.full_sync().await {
                            error!("Full sync recovery failed: {}", e);
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        warn!("Config event channel closed, stopping listener");
                        break;
                    }
                }
            }
            info!("Config synchronizer event listener stopped");
        })
    }

    /// 停止事件监听
    pub fn stop(&self) {
        *self.running.write() = false;
        info!("Config synchronizer stopping...");
    }

    /// 处理单个配置变更事件
    async fn handle_config_event(&self, event: ConfigChangeEvent) -> AllianceResult<()> {
        debug!(
            "Handling config event: {:?} for module {}, type {:?}",
            event.change_type, event.module_id, event.config_type
        );

        match event.change_type {
            ConfigChangeType::Created | ConfigChangeType::Updated | ConfigChangeType::Enabled => {
                self.sync_single_module(&event.module_id).await?;
            }
            ConfigChangeType::Deleted | ConfigChangeType::Disabled => {
                self.remove_module_weights(&event.module_id);
            }
            ConfigChangeType::RolledBack => {
                // 回滚后重新同步
                self.sync_single_module(&event.module_id).await?;
            }
        }

        Ok(())
    }

    /// 同步单个模块的配置到匹配器
    async fn sync_single_module(&self, module_id: &str) -> AllianceResult<()> {
        let config = self
            .config_engine
            .get_module_config(module_id)
            .await
            .map_err(|e| {
                mox_alliance_common_proto::AllianceError::internal(format!(
                    "Failed to get module config: {}",
                    e
                ))
            })?;

        match config {
            Some(module) => {
                if module.enabled {
                    self.matcher
                        .set_expert_weights(&module.expert_id, module.matching_weights.clone());

                    self.module_to_expert
                        .write()
                        .insert(module.module_id.clone(), module.expert_id.clone());

                    debug!(
                        "Synced weights for expert {} from module {}",
                        module.expert_id, module.module_id
                    );
                } else {
                    self.remove_module_weights(module_id);
                }
            }
            None => {
                warn!("Module config not found for sync: {}", module_id);
            }
        }

        Ok(())
    }

    /// 移除模块对应的专家权重（恢复为默认权重）
    fn remove_module_weights(&self, module_id: &str) {
        let mut module_map = self.module_to_expert.write();
        if let Some(expert_id) = module_map.remove(module_id) {
            self.matcher.reset_expert_weights(&expert_id);
            debug!(
                "Removed weights for expert {} (module {} disabled/deleted)",
                expert_id, module_id
            );
        }
    }

    /// 获取模块到专家的映射（用于调试）
    pub fn module_expert_map(&self) -> HashMap<String, String> {
        self.module_to_expert.read().clone()
    }

    /// 获取当前同步的模块数量
    pub fn synced_count(&self) -> usize {
        self.module_to_expert.read().len()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use mox_alliance_common_proto::{
        ExpertModuleConfig, GraphEngineType, MatchingWeights, ModuleGraphConfig, ModuleLlmConfig,
    };
    use mox_alliance_config_core::{ConfigEngine, MemoryConfigStore};

    fn make_test_module_config(
        module_id: &str,
        expert_id: &str,
        weights: MatchingWeights,
    ) -> ExpertModuleConfig {
        let now = Utc::now();
        ExpertModuleConfig {
            module_id: module_id.to_string(),
            expert_id: expert_id.to_string(),
            name: format!("Test Module {}", module_id),
            version: "1.0.0".to_string(),
            llm_config: ModuleLlmConfig {
                module_id: module_id.to_string(),
                primary_provider: "openai".to_string(),
                primary_model: "gpt-4".to_string(),
                fallback_chain: vec![],
                routing_strategy: Default::default(),
                model_config: Default::default(),
                provider_options: vec![],
                system_prompt_template: None,
                version: 1,
                updated_at: now,
            },
            graph_config: ModuleGraphConfig {
                module_id: module_id.to_string(),
                engine_type: GraphEngineType::RelGraph,
                connection: mox_alliance_common_proto::GraphConnectionConfig {
                    uri_env: "RELGRAPH_URI".to_string(),
                    user_env: None,
                    password_env: None,
                    database: None,
                },
                query_config: Default::default(),
                schema: Default::default(),
                custom_endpoint: None,
                version: 1,
                updated_at: now,
            },
            capability_weights: Default::default(),
            matching_weights: weights,
            enabled: true,
            tags: vec![],
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn full_sync_works() {
        let store = MemoryConfigStore::arc();
        let engine = Arc::new(ConfigEngine::new(store.clone()));
        let matcher = Arc::new(ModularWeightMatcher::new());
        let sync = ConfigSynchronizer::new(engine.clone(), matcher.clone());

        // 注册两个模块
        let weights1 = MatchingWeights {
            domain: 0.5,
            capability: 0.3,
            rating: 0.1,
            performance: 0.05,
            health: 0.05,
        };
        let weights2 = MatchingWeights {
            domain: 0.2,
            capability: 0.5,
            rating: 0.2,
            performance: 0.05,
            health: 0.05,
        };

        engine
            .register_module(
                make_test_module_config("mod1", "expert1", weights1.clone()),
                "test",
                "init",
            )
            .await
            .unwrap();
        engine
            .register_module(
                make_test_module_config("mod2", "expert2", weights2.clone()),
                "test",
                "init",
            )
            .await
            .unwrap();

        // 全量同步
        let count = sync.full_sync().await.unwrap();
        assert_eq!(count, 2);

        // 验证权重已同步
        let synced_w1 = matcher.get_expert_weights("expert1");
        assert!((synced_w1.domain - weights1.domain).abs() < 0.001);
        assert!((synced_w1.capability - weights1.capability).abs() < 0.001);

        let synced_w2 = matcher.get_expert_weights("expert2");
        assert!((synced_w2.domain - weights2.domain).abs() < 0.001);
        assert!((synced_w2.capability - weights2.capability).abs() < 0.001);
    }

    #[tokio::test]
    async fn disabled_modules_skipped_in_sync() {
        let store = MemoryConfigStore::arc();
        let engine = Arc::new(ConfigEngine::new(store.clone()));
        let matcher = Arc::new(ModularWeightMatcher::new());
        let sync = ConfigSynchronizer::new(engine.clone(), matcher.clone());

        // 注册一个禁用的模块
        let mut config = make_test_module_config("mod-disabled", "expert-disabled", MatchingWeights::default());
        config.enabled = false;
        engine
            .register_module(config, "test", "init")
            .await
            .unwrap();

        // 全量同步
        let count = sync.full_sync().await.unwrap();
        assert_eq!(count, 0); // 禁用的模块不应被同步

        // 验证专家使用默认权重
        let w = matcher.get_expert_weights("expert-disabled");
        let default = MatchingWeights::default();
        assert!((w.domain - default.domain).abs() < 0.001);
    }

    #[tokio::test]
    async fn module_to_expert_mapping() {
        let store = MemoryConfigStore::arc();
        let engine = Arc::new(ConfigEngine::new(store.clone()));
        let matcher = Arc::new(ModularWeightMatcher::new());
        let sync = ConfigSynchronizer::new(engine.clone(), matcher.clone());

        engine
            .register_module(
                make_test_module_config("mod1", "expert1", MatchingWeights::default()),
                "test",
                "init",
            )
            .await
            .unwrap();

        sync.full_sync().await.unwrap();

        let map = sync.module_expert_map();
        assert_eq!(map.get("mod1"), Some(&"expert1".to_string()));
        assert_eq!(sync.synced_count(), 1);
    }
}

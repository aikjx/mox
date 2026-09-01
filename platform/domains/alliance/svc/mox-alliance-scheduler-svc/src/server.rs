// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 调度器服务器

use std::net::SocketAddr;
use std::sync::Arc;

use mox_alliance_common_proto::{Capability, Expert, ExpertHealth, ExpertModuleConfig, ExpertStatus};
use mox_alliance_config_core::examples::domain_experts::{
    build_domain_experts, build_global_default_config,
};
use mox_alliance_config_core::{ConfigEngine, MemoryConfigStore};
use mox_alliance_executor_proto::DagEngine;
use mox_alliance_boot_config::{ExpertServiceSection, ExpertsBootConfig};
use mox_alliance_scheduler_core::{
    ConfigSynchronizer, HttpExecutorBridge, HttpExecutorBridgeConfig, InProcessExecutorBridge,
    ModularWeightMatcher, TaskSchedulerImpl,
};
use mox_alliance_scheduler_proto::types::SchedulerConfig;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::app_state::SchedulerAppState;
use crate::routes::build_router;

/// 调度器运行模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerMode {
    /// 独立模式（通过 HTTP 调用远程执行器）
    Standalone,
    /// 内嵌模式（执行器在同进程内，进程内调用）
    Embedded,
}

/// 调度器服务器
pub struct SchedulerServer {
    config: SchedulerConfig,
    listen_addr: SocketAddr,
    mode: SchedulerMode,
    /// 执行器服务地址（Standalone 模式下使用）
    executor_url: Option<String>,
    /// 进程内执行器引擎（Embedded 模式下使用，依赖注入）
    embedded_engine: Option<Arc<dyn DagEngine>>,
    /// 任务仓库（持久化可插拔；默认按环境变量决定）
    task_repository: Option<Arc<dyn mox_alliance_scheduler_core::TaskRepository>>,
    /// 专家配置外部化覆盖（来自 config/alliance-experts.yml，按 module_id 合并）
    experts: Option<ExpertsBootConfig>,
    /// HTTP 专家桥接配置（生产专家服务；enabled=true 时启动拉取远程专家）
    expert_service: Option<ExpertServiceSection>,
}

impl SchedulerServer {
    pub fn new(config: SchedulerConfig, listen_addr: SocketAddr) -> Self {
        Self {
            config,
            listen_addr,
            mode: SchedulerMode::Standalone,
            executor_url: None,
            embedded_engine: None,
            task_repository: None,
            experts: None,
            expert_service: None,
        }
    }

    /// 设置运行模式
    pub fn with_mode(mut self, mode: SchedulerMode) -> Self {
        self.mode = mode;
        self
    }

    /// 设置执行器服务地址
    pub fn with_executor_url(mut self, url: impl Into<String>) -> Self {
        self.executor_url = Some(url.into());
        self
    }

    /// 注入进程内执行器引擎（Embedded 模式必需）
    pub fn with_embedded_engine(mut self, engine: Arc<dyn DagEngine>) -> Self {
        self.embedded_engine = Some(engine);
        self
    }

    /// 注入自定义任务仓库（企业级持久化）
    pub fn with_task_repository(
        mut self,
        repository: Arc<dyn mox_alliance_scheduler_core::TaskRepository>,
    ) -> Self {
        self.task_repository = Some(repository);
        self
    }

    /// 注入专家配置外部化覆盖（config/alliance-experts.yml 合并）
    pub fn with_experts(mut self, experts: ExpertsBootConfig) -> Self {
        self.experts = Some(experts);
        self
    }

    /// 注入 HTTP 专家桥接配置（生产专家服务，enabled=true 时 build_app 会拉取远程专家）
    pub fn with_expert_service(mut self, expert_service: ExpertServiceSection) -> Self {
        self.expert_service = Some(expert_service);
        self
    }

    /// 解析任务仓库：显式注入优先，否则按环境变量 MOX_ALLIANCE_STORAGE_MODE
    /// - "file"（或未设置时默认 "file"）：文件快照持久化到 ./data/alliance_tasks.json
    /// - "memory"：纯内存
    /// - 旧环境变量 `ALLIANCE_TASK_STORE` 保留兼容（deprecated，命中即告警）
    fn resolve_task_repository(
        &self,
    ) -> anyhow::Result<Arc<dyn mox_alliance_scheduler_core::TaskRepository>> {
        if let Some(repo) = &self.task_repository {
            return Ok(repo.clone());
        }

        // 归一化：统一 `MOX_ALLIANCE_STORAGE_MODE`；旧 `ALLIANCE_TASK_STORE` 兼容
        let mode = match std::env::var("MOX_ALLIANCE_STORAGE_MODE") {
            Ok(v) if !v.is_empty() => v,
            _ => match std::env::var("ALLIANCE_TASK_STORE") {
                Ok(old) if !old.is_empty() => {
                    warn!("环境变量 ALLIANCE_TASK_STORE 已废弃，请改用 MOX_ALLIANCE_STORAGE_MODE（旧值已生效兼容）");
                    old
                }
                _ => "file".to_string(),
            },
        };
        match mode.as_str() {
            "memory" => {
                info!("Using in-memory task repository");
                Ok(Arc::new(
                    mox_alliance_scheduler_core::InMemoryTaskRepository::new(),
                ))
            }
            _ => {
                let path = std::path::Path::new("data").join("alliance_tasks.json");
                let repo = mox_alliance_scheduler_core::FileTaskRepository::new(&path)?;
                info!("Using file task repository at {}", path.display());
                Ok(Arc::new(repo))
            }
        }
    }

    /// 创建执行器桥接
    fn create_executor_bridge(
        &self,
    ) -> anyhow::Result<Arc<dyn mox_alliance_scheduler_core::ExecutorBridge>> {
        match self.mode {
            SchedulerMode::Standalone => {
                // 默认指向执行器服务（executor-svc 监听 3200），修复了原先指向自身 3100 的错配
                let base_url = self
                    .executor_url
                    .clone()
                    .unwrap_or_else(|| "http://localhost:3200".to_string());

                let config = HttpExecutorBridgeConfig {
                    base_url: base_url.clone(),
                    timeout_ms: 30_000,
                };

                let bridge = HttpExecutorBridge::new(config)?;
                info!("Using HTTP executor bridge at {}", base_url);
                Ok(Arc::new(bridge))
            }
            SchedulerMode::Embedded => {
                let engine = self.embedded_engine.clone().ok_or_else(|| {
                    anyhow::anyhow!(
                        "Embedded mode requires an in-process DagEngine. \
                         Call with_embedded_engine() before run()"
                    )
                })?;
                let bridge = InProcessExecutorBridge::new(engine);
                info!("Using in-process executor bridge");
                Ok(Arc::new(bridge))
            }
        }
    }

    /// 由模块配置派生专家并注册到模块化匹配器
    ///
    /// 全链路接线：模块配置(ConfigEngine) → 专家注册(ModularWeightMatcher)
    /// → 权重同步(ConfigSynchronizer) → 调度匹配。
    fn expert_from_module(module: &ExpertModuleConfig) -> Expert {
        let now = chrono::Utc::now();
        let capabilities: Vec<Capability> = module
            .capability_weights
            .keys()
            .map(|name| Capability {
                capability_id: format!("{}-{}", module.module_id, name),
                name: name.clone(),
                description: name.clone(),
                domain: "general".to_string(),
                version: "1.0.0".to_string(),
            })
            .collect();

        Expert {
            expert_id: module.expert_id.clone(),
            tenant_id: "system".to_string(),
            name: module.name.clone(),
            version: module.version.clone(),
            // 用模块的系统提示词模板承载领域能力词库（原实现只存 name，导致描述匹配失效）
            description: match &module.llm_config.system_prompt_template {
                Some(tpl) => format!("{}. {}", module.name, tpl),
                None => module.name.clone(),
            },
            domains: module.tags.clone(),
            capabilities,
            tools: vec![],
            status: ExpertStatus::Active,
            health: ExpertHealth::default(),
            priority: 5,
            created_at: now,
            updated_at: now,
        }
    }

    /// 构建应用（将构建逻辑与网络监听分离，便于测试注入与复用）
    pub async fn build_app(&self) -> anyhow::Result<axum::Router> {
        // ── 构建模块化配置子系统（全链路接线）──
        // 1) 配置引擎（内存存储，可替换为持久化实现）
        let config_engine = Arc::new(ConfigEngine::new(Arc::new(MemoryConfigStore::new())));

        // 1.1) 引导全局 LLM 默认配置（模块未显式声明的 provider 继承自此）
        //      支持 config/alliance-experts.yml 局部覆盖（global_llm 字段级覆盖）
        let builtin_global = build_global_default_config();
        let global_config = self
            .experts
            .as_ref()
            .map(|e| e.effective_global(&builtin_global))
            .unwrap_or(builtin_global);
        config_engine
            .set_global_llm_config(global_config, "system", "bootstrap global llm config")
            .await
            .map_err(|e| anyhow::anyhow!("Failed to set global llm config: {}", e))?;

        // 2) 注册领域专家模块配置（内置 10 大专家 + yml 按 module_id 覆盖/新增）
        let builtin_modules = build_domain_experts();
        let modules = match &self.experts {
            Some(e) => e.merge_into(builtin_modules),
            None => builtin_modules,
        };
        for module in modules {
            config_engine
                .register_module(module.clone(), "system", "bootstrap builtin domain experts")
                .await
                .map_err(|e| anyhow::anyhow!("Failed to register module {}: {}", module.module_id, e))?;
        }

        // 3) 模块化权重匹配器：由模块配置派生专家并注册
        let matcher = Arc::new(ModularWeightMatcher::new());
        let modules = config_engine
            .list_modules()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list modules: {}", e))?;
        let experts: Vec<Expert> = modules
            .iter()
            .map(Self::expert_from_module)
            .collect();
        matcher.register_experts(experts);
        info!(
            "Registered {} builtin domain experts from module config",
            modules.len()
        );

        // 3.1) HTTP 专家桥接（生产专家服务）：expert_service.enabled=true 时，
        //      启动从远程 AI 专家服务拉取专家并入匹配器；拉取失败优雅降级到内置（不崩溃）。
        if let Some(es) = &self.expert_service {
            if es.enabled {
                let http_bridge = mox_alliance_scheduler_core::HttpExpertRegistryBridge::new(
                    mox_alliance_scheduler_core::HttpBridgeConfig {
                        base_url: es.base_url.clone(),
                        timeout_ms: es.timeout_ms,
                        tenant_id: "system".to_string(),
                    },
                );
                match http_bridge.fetch_experts().await {
                    Ok(remote) => {
                        matcher.register_experts(remote.clone());
                        info!(
                            "HTTP 专家桥接启用：从 {} 拉取 {} 位专家并入匹配器",
                            es.base_url,
                            remote.len()
                        );
                    }
                    Err(e) => {
                        warn!(
                            "HTTP 专家桥接拉取失败（{}），使用内置领域专家继续。原因：{}",
                            es.base_url, e
                        );
                    }
                }
            }
        }

        // 4) 配置同步器：模块权重 → 匹配器（运行时可热更新）
        let synchronizer = Arc::new(ConfigSynchronizer::new(config_engine.clone(), matcher.clone()));
        let synced = synchronizer
            .full_sync()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to sync module config: {}", e))?;
        info!("Config synchronizer synced {} module weights", synced);

        // ── 创建执行器桥接 ──
        let executor_bridge = self.create_executor_bridge()?;

        // ── 解析任务仓库（持久化可插拔）──
        let task_repository = self.resolve_task_repository()?;

        // ── 初始化调度器（使用模块化匹配器 + 可插拔存储）──
        let scheduler = Arc::new(
            TaskSchedulerImpl::new_with_bridge(
                self.config.clone(),
                matcher.clone(),
                executor_bridge.clone(),
            )
            .with_task_repository(task_repository),
        );

        // 构建应用状态
        let executor_base_url = self
            .executor_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:3200".to_string());
        let state = SchedulerAppState::new_with_bridge(
            self.config.clone(),
            scheduler,
            matcher,
            executor_bridge,
        )
        .with_executor_base_url(executor_base_url);

        // 构建路由
        Ok(build_router(state))
    }

    /// 启动服务器
    pub async fn run(&self) -> anyhow::Result<()> {
        let app = self.build_app().await?;

        info!(
            "Scheduler server starting on {} (mode: {:?})",
            self.listen_addr, self.mode
        );

        // 启动服务
        let listener = tokio::net::TcpListener::bind(self.listen_addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

// 保留旧的兼容性代码（不使用 bridge 的版本）
#[allow(dead_code)]
fn _create_legacy_scheduler(
    config: SchedulerConfig,
    matcher: Arc<mox_alliance_scheduler_core::RuleBasedExpertMatcher>,
) -> (Arc<TaskSchedulerImpl>, mpsc::UnboundedSender<mox_alliance_common_proto::Task>) {
    let (dispatch_tx, _dispatch_rx) = mpsc::unbounded_channel::<mox_alliance_common_proto::Task>();
    let scheduler = Arc::new(TaskSchedulerImpl::new(
        config,
        matcher,
        dispatch_tx.clone(),
    ));
    (scheduler, dispatch_tx)
}

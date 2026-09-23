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

    /// 多活前置条件：开启 HA 时任务库必须是共享权威源（sqlite）。
    /// 调用方自行注入仓库时豁免——共享与否由宿主保证（测试/内嵌宿主即此路径）。
    fn ha_storage_ok(ha_on: bool, mode: &str, injected_repo: bool) -> bool {
        !ha_on || injected_repo || mode == "sqlite"
    }

    /// 解析存储档位（归一化：统一 `MOX_ALLIANCE_STORAGE_MODE`；旧 `ALLIANCE_TASK_STORE` 兼容）
    fn storage_mode() -> String {
        match std::env::var("MOX_ALLIANCE_STORAGE_MODE") {
            Ok(v) if !v.is_empty() => v,
            _ => match std::env::var("ALLIANCE_TASK_STORE") {
                Ok(old) if !old.is_empty() => {
                    warn!("环境变量 ALLIANCE_TASK_STORE 已废弃，请改用 MOX_ALLIANCE_STORAGE_MODE（旧值已生效兼容）");
                    old
                }
                _ => "file".to_string(),
            },
        }
    }

    /// 解析任务仓库：显式注入优先，否则按 `mode`（见 [`Self::storage_mode`]）
    /// - "file"（默认）：文件快照持久化到 ./data/alliance_tasks.json
    /// - "memory"：纯内存
    /// - "sqlite"：SQLite 增量落盘（WAL）到 ./data/alliance_tasks.db
    ///
    /// `shared = true`（多活副本）时开库不做启动清扫：那会把别的副本正在跑的
    /// 任务改写成 interrupted。异常态的收敛改由 leader 的 `reconcile_active_tasks` 负责。
    fn resolve_task_repository(
        &self,
        mode: &str,
        shared: bool,
    ) -> anyhow::Result<Arc<dyn mox_alliance_scheduler_core::TaskRepository>> {
        if let Some(repo) = &self.task_repository {
            return Ok(repo.clone());
        }

        match mode {
            "memory" => {
                info!("Using in-memory task repository");
                Ok(Arc::new(
                    mox_alliance_scheduler_core::InMemoryTaskRepository::new(),
                ))
            }
            "sqlite" => {
                let path = std::path::Path::new("data").join("alliance_tasks.db");
                let repo = if shared {
                    let repo = mox_alliance_scheduler_core::SqliteTaskRepository::new_shared(&path)?;
                    info!(
                        "Using shared sqlite task repository (WAL, 多活：跳过启动清扫) at {}",
                        path.display()
                    );
                    repo
                } else {
                    let repo = mox_alliance_scheduler_core::SqliteTaskRepository::new(&path)?;
                    info!("Using sqlite task repository (WAL) at {}", path.display());
                    repo
                };
                Ok(Arc::new(repo))
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
    ///
    /// 不含 HA 句柄；需要优雅让位请用 [`Self::build_app_with_ha`]（`run()` 即用此路径）。
    pub async fn build_app(&self) -> anyhow::Result<axum::Router> {
        Ok(self.build_app_with_ha().await?.0)
    }

    /// 构建应用并装配多活运行面。
    ///
    /// 返回 `(Router, Option<(选主状态机, 后台循环)>)`；`None` 表示
    /// `MOX_ALLIANCE_HA_MODE` 未开启——此时请求路径与周期职责都只由本副本承担，
    /// 行为与单副本完全一致。
    pub async fn build_app_with_ha(
        &self,
    ) -> anyhow::Result<(
        axum::Router,
        Option<(crate::ha::SharedElector, tokio::task::JoinHandle<()>)>,
    )> {
        // ── 多活前置校验：先拒掉"假多活"，再建重型子系统 ──
        // 任务状态本身必须是所有副本共享的权威源：`file` 是全量快照单写者（并发副本互相覆盖）、
        // `memory` 各副本私有（选出的 leader 只看得见自己那份表）。
        // 这两种档位下选主只是自娱自乐，故拒绝启动而不是勉强跑起来。
        let ha_cfg = crate::ha::HaConfig::from_env();
        let storage_mode = Self::storage_mode();
        anyhow::ensure!(
            Self::ha_storage_ok(ha_cfg.is_some(), &storage_mode, self.task_repository.is_some()),
            "MOX_ALLIANCE_HA_MODE 要求 MOX_ALLIANCE_STORAGE_MODE=sqlite（当前 {storage_mode}）：\
             多活副本必须共享同一个任务库，否则租约选出来的 leader 对账的是只有自己看得见的状态"
        );

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

        // ── 解析任务仓库（持久化可插拔；多活时按共享库打开，不做启动清扫）──
        let task_repository = self.resolve_task_repository(&storage_mode, ha_cfg.is_some())?;

        // ── 初始化调度器（使用模块化匹配器 + 可插拔存储）──
        let scheduler = Arc::new(
            TaskSchedulerImpl::new_with_bridge(
                self.config.clone(),
                matcher.clone(),
                executor_bridge.clone(),
            )
            .with_task_repository(task_repository),
        );

        // ── 初始化共享指标收集器（/metrics 暴露，/experts/search 实时 record）──
        let metrics = Arc::new(mox_alliance_scheduler_core::AllianceMetrics::new());

        // 构建应用状态
        let executor_base_url = self
            .executor_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:3200".to_string());
        let mut state = SchedulerAppState::new_with_bridge(
            self.config.clone(),
            scheduler,
            matcher,
            executor_bridge,
        )
        .with_executor_base_url(executor_base_url)
        .with_metrics(metrics);

        // ── 多活（HA）：租约选主 + leader 专属周期对账 ──
        // 未开启时完全不介入：不起线程、不开租约库、不抢主。
        let ha = match ha_cfg {
            None => None,
            Some(cfg) => {
                let (elector, task) =
                    crate::ha::start(&cfg, state.scheduler.clone()).map_err(|e| {
                        anyhow::anyhow!(
                            "HA 装配失败（租约库 {}，holder {}）: {e}",
                            cfg.db.display(),
                            cfg.holder
                        )
                    })?;
                state = state.with_leadership(elector.clone());
                Some((elector, task))
            }
        };

        // 构建路由
        Ok((build_router(state), ha))
    }

    /// 启动服务器
    pub async fn run(&self) -> anyhow::Result<()> {
        let (app, ha) = self.build_app_with_ha().await?;

        info!(
            "Scheduler server starting on {} (mode: {:?})",
            self.listen_addr, self.mode
        );

        // 启动服务
        let listener = tokio::net::TcpListener::bind(self.listen_addr).await?;
        info!(service = "alliance-scheduler", addr = %self.listen_addr, "TCP listener bound");
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
                info!("alliance-scheduler shutdown signal received");
            })
            .await?;

        // HA 收尾：先停后台循环、等它真正退出，再主动让位。
        // 顺序不能倒——循环若还在续约，会把刚交出去的租约又抢回来，
        // standby 反而要等满一个租约周期。
        if let Some((elector, task)) = ha {
            task.abort();
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                task,
            )
            .await;
            crate::ha::resign(&elector);
        }

        info!("alliance-scheduler stopped gracefully");
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

#[cfg(test)]
mod tests {
    use super::SchedulerServer;

    #[test]
    fn ha_refuses_storage_that_replicas_do_not_share() {
        // 关闭 HA：任何档位放行（默认行为与单副本一致）
        assert!(SchedulerServer::ha_storage_ok(false, "file", false));
        assert!(SchedulerServer::ha_storage_ok(false, "memory", false));
        // 开启 HA：只有共享权威源放行
        assert!(SchedulerServer::ha_storage_ok(true, "sqlite", false));
        assert!(
            !SchedulerServer::ha_storage_ok(true, "file", false),
            "file 是全量快照单写者：双副本会互相覆盖"
        );
        assert!(
            !SchedulerServer::ha_storage_ok(true, "memory", false),
            "memory 各副本私有：选出的 leader 只看得见自己的表"
        );
        // 注入式仓库由宿主自行保证共享性，不受此门禁约束
        assert!(SchedulerServer::ha_storage_ok(true, "file", true));
    }
}

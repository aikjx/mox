// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家同步器
//!
//! 定时从 AI 专家服务同步专家列表到联盟调度器的匹配器中。
//! 支持全量同步、增量同步和健康状态更新。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use mox_alliance_common_proto::{AllianceError, AllianceResult, Expert, ExpertHealth};

use crate::registry::{ExpertRegistryBridge, SyncStats};

// ============================================================================
// 同步器配置
// ============================================================================

/// 专家同步器配置
#[derive(Debug, Clone)]
pub struct SynchronizerConfig {
    /// 全量同步间隔（秒）
    pub full_sync_interval_secs: u64,
    /// 增量同步间隔（秒），0 表示不启用增量同步
    pub incremental_sync_interval_secs: u64,
    /// 健康状态更新间隔（秒），0 表示不启用健康更新
    pub health_update_interval_secs: u64,
    /// 初始同步延迟（秒），启动后等待多久开始第一次同步
    pub initial_delay_secs: u64,
    /// 同步失败最大重试次数
    pub max_retries: u32,
    /// 重试间隔（秒）
    pub retry_interval_secs: u64,
    /// 是否启用自动同步
    pub auto_sync_enabled: bool,
}

impl Default for SynchronizerConfig {
    fn default() -> Self {
        Self {
            full_sync_interval_secs: 300, // 5 分钟
            incremental_sync_interval_secs: 60, // 1 分钟
            health_update_interval_secs: 30, // 30 秒
            initial_delay_secs: 5,
            max_retries: 3,
            retry_interval_secs: 10,
            auto_sync_enabled: true,
        }
    }
}

// ============================================================================
// 同步模式
// ============================================================================

/// 同步模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// 全量同步：拉取全部专家，替换本地
    Full,
    /// 增量同步：只同步变更的专家
    Incremental,
    /// 仅更新健康状态
    HealthOnly,
}

// ============================================================================
// 同步状态
// ============================================================================

/// 同步器运行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynchronizerState {
    /// 未启动
    Idle,
    /// 运行中
    Running,
    /// 同步中
    Syncing,
    /// 已停止
    Stopped,
}

/// 同步结果
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// 同步模式
    pub mode: SyncMode,
    /// 是否成功
    pub success: bool,
    /// 同步统计（成功时有值）
    pub stats: Option<SyncStats>,
    /// 错误信息（失败时有值）
    pub error: Option<String>,
    /// 同步时间
    pub sync_time: DateTime<Utc>,
    /// 耗时（毫秒）
    pub duration_ms: u64,
}

impl Default for SyncResult {
    fn default() -> Self {
        Self {
            mode: SyncMode::Full,
            success: false,
            stats: None,
            error: None,
            sync_time: Utc::now(),
            duration_ms: 0,
        }
    }
}

// ============================================================================
// 专家数据源 trait
// ============================================================================

/// 专家数据源 trait
///
/// 定义从外部系统获取专家列表的接口。
/// 可以是 HTTP API、数据库、配置文件等。
#[async_trait]
pub trait ExpertDataSource: Send + Sync {
    /// 拉取全部专家列表
    async fn fetch_all_experts(&self) -> AllianceResult<Vec<Expert>>;

    /// 拉取指定时间后变更的专家（增量同步）
    ///
    /// 默认实现：返回全部专家（不支持增量时回退为全量）
    async fn fetch_updated_experts(&self, _since: DateTime<Utc>) -> AllianceResult<Vec<Expert>> {
        self.fetch_all_experts().await
    }

    /// 获取专家健康状态
    ///
    /// 默认实现：返回空列表（不支持健康查询）
    async fn fetch_health_status(&self) -> AllianceResult<Vec<(String, ExpertHealth)>> {
        Ok(vec![])
    }
}

// ============================================================================
// ExpertSynchronizer
// ============================================================================

/// 专家同步器
///
/// 定时从 `ExpertDataSource` 拉取专家列表，同步到 `ExpertRegistryBridge`。
/// 支持全量同步、增量同步和健康状态更新三种模式。
///
/// ## 使用方式
/// ```rust,ignore
/// let data_source = Arc::new(HttpExpertDataSource::new(config));
/// let registry = Arc::new(InMemoryExpertRegistry::new());
/// let synchronizer = ExpertSynchronizer::new(config, data_source, registry);
/// synchronizer.start().await;
/// ```
pub struct ExpertSynchronizer {
    config: SynchronizerConfig,
    data_source: Arc<dyn ExpertDataSource>,
    registry: Arc<dyn ExpertRegistryBridge>,
    state: Arc<parking_lot::RwLock<SynchronizerState>>,
    last_sync_time: Arc<parking_lot::RwLock<Option<DateTime<Utc>>>>,
    last_full_sync: Arc<parking_lot::RwLock<Option<DateTime<Utc>>>>,
    last_result: Arc<parking_lot::RwLock<Option<SyncResult>>>,
    /// 停止信号发送端
    stop_tx: Arc<parking_lot::Mutex<Option<mpsc::Sender<()>>>>,
    /// 后台任务句柄
    task_handle: Arc<parking_lot::Mutex<Option<JoinHandle<()>>>>,
}

impl ExpertSynchronizer {
    /// 创建新的同步器
    pub fn new(
        config: SynchronizerConfig,
        data_source: Arc<dyn ExpertDataSource>,
        registry: Arc<dyn ExpertRegistryBridge>,
    ) -> Self {
        Self {
            config,
            data_source,
            registry,
            state: Arc::new(parking_lot::RwLock::new(SynchronizerState::Idle)),
            last_sync_time: Arc::new(parking_lot::RwLock::new(None)),
            last_full_sync: Arc::new(parking_lot::RwLock::new(None)),
            last_result: Arc::new(parking_lot::RwLock::new(None)),
            stop_tx: Arc::new(parking_lot::Mutex::new(None)),
            task_handle: Arc::new(parking_lot::Mutex::new(None)),
        }
    }

    /// 启动同步器（后台定时任务）
    pub async fn start(&self) -> AllianceResult<()> {
        if !self.config.auto_sync_enabled {
            info!("Expert synchronizer auto-sync is disabled");
            return Ok(());
        }

        let mut state = self.state.write();
        if *state == SynchronizerState::Running {
            warn!("Expert synchronizer is already running");
            return Ok(());
        }
        *state = SynchronizerState::Running;
        drop(state);

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        *self.stop_tx.lock() = Some(stop_tx);

        let config = self.config.clone();
        let data_source = self.data_source.clone();
        let registry = self.registry.clone();
        let state_arc = self.state.clone();
        let last_sync_arc = self.last_sync_time.clone();
        let last_full_arc = self.last_full_sync.clone();
        let last_result_arc = self.last_result.clone();

        let handle = tokio::spawn(async move {
            // 初始延迟
            if config.initial_delay_secs > 0 {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(config.initial_delay_secs)) => {}
                    _ = stop_rx.recv() => {
                        info!("Expert synchronizer stopped during initial delay");
                        return;
                    }
                }
            }

            // 首次全量同步
            info!("Starting initial full expert sync...");
            let result = do_full_sync(&data_source, &registry, &config).await;
            record_sync_result(&last_result_arc, result.clone());
            if result.success {
                *last_sync_arc.write() = Some(Utc::now());
                *last_full_arc.write() = Some(Utc::now());
                info!(
                    "Initial full sync completed: {} experts total",
                    result.stats.as_ref().map(|s| s.total).unwrap_or(0)
                );
            } else {
                error!(
                    "Initial full sync failed: {}",
                    result.error.unwrap_or_else(|| "unknown".into())
                );
            }

            // 主循环
            let mut full_sync_tick = tokio::time::interval(Duration::from_secs(
                config.full_sync_interval_secs.max(1),
            ));
            let mut inc_sync_tick = if config.incremental_sync_interval_secs > 0 {
                Some(tokio::time::interval(Duration::from_secs(
                    config.incremental_sync_interval_secs,
                )))
            } else {
                None
            };
            let mut health_tick = if config.health_update_interval_secs > 0 {
                Some(tokio::time::interval(Duration::from_secs(
                    config.health_update_interval_secs,
                )))
            } else {
                None
            };

            loop {
                tokio::select! {
                    _ = full_sync_tick.tick() => {
                        debug!("Triggering full expert sync...");
                        *state_arc.write() = SynchronizerState::Syncing;
                        let result = do_full_sync(&data_source, &registry, &config).await;
                        record_sync_result(&last_result_arc, result.clone());
                        if result.success {
                            *last_sync_arc.write() = Some(Utc::now());
                            *last_full_arc.write() = Some(Utc::now());
                            debug!(
                                "Full sync completed: {} experts",
                                result.stats.as_ref().map(|s| s.total).unwrap_or(0)
                            );
                        } else {
                            warn!(
                                "Full sync failed: {}",
                                result.error.unwrap_or_else(|| "unknown".into())
                            );
                        }
                        *state_arc.write() = SynchronizerState::Running;
                    }
                    _ = async {
                        if let Some(tick) = inc_sync_tick.as_mut() {
                            tick.tick().await;
                        } else {
                            std::future::pending::<()>().await;
                        }
                    } => {
                        debug!("Triggering incremental expert sync...");
                        *state_arc.write() = SynchronizerState::Syncing;
                        let since = last_full_arc.read().unwrap_or_else(Utc::now);
                        let result = do_incremental_sync(&data_source, &registry, &config, since).await;
                        record_sync_result(&last_result_arc, result.clone());
                        if result.success {
                            *last_sync_arc.write() = Some(Utc::now());
                            debug!("Incremental sync completed");
                        } else {
                            warn!("Incremental sync failed");
                        }
                        *state_arc.write() = SynchronizerState::Running;
                    }
                    _ = async {
                        if let Some(tick) = health_tick.as_mut() {
                            tick.tick().await;
                        } else {
                            std::future::pending::<()>().await;
                        }
                    } => {
                        debug!("Triggering health status update...");
                        let result = do_health_update(&data_source, &registry).await;
                        if result.success {
                            debug!("Health update completed");
                        } else {
                            debug!("Health update skipped or failed");
                        }
                    }
                    _ = stop_rx.recv() => {
                        info!("Expert synchronizer stopping...");
                        break;
                    }
                }
            }

            *state_arc.write() = SynchronizerState::Stopped;
            info!("Expert synchronizer stopped");
        });

        *self.task_handle.lock() = Some(handle);
        info!("Expert synchronizer started");
        Ok(())
    }

    /// 停止同步器
    pub async fn stop(&self) -> AllianceResult<()> {
        // 发送停止信号（先取出 Sender，释放锁后再 await，避免持有 MutexGuard 跨 await）
        let stop_tx = self.stop_tx.lock().take();
        if let Some(tx) = stop_tx {
            let _ = tx.send(()).await;
        }

        // 等待任务结束
        let handle = self.task_handle.lock().take();
        if let Some(handle) = handle {
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }

        *self.state.write() = SynchronizerState::Stopped;
        Ok(())
    }

    /// 手动触发一次全量同步
    pub async fn trigger_full_sync(&self) -> AllianceResult<SyncStats> {
        let result = do_full_sync(&self.data_source, &self.registry, &self.config).await;
        self.record_result(result.clone());
        if result.success {
            *self.last_sync_time.write() = Some(Utc::now());
            *self.last_full_sync.write() = Some(Utc::now());
            Ok(result.stats.unwrap_or_default())
        } else {
            Err(AllianceError::internal(format!(
                "Full sync failed: {}",
                result.error.unwrap_or_else(|| "unknown".into())
            )))
        }
    }

    /// 手动触发一次增量同步
    pub async fn trigger_incremental_sync(&self) -> AllianceResult<SyncStats> {
        let since = self.last_full_sync.read().unwrap_or_else(Utc::now);
        let result = do_incremental_sync(&self.data_source, &self.registry, &self.config, since).await;
        self.record_result(result.clone());
        if result.success {
            *self.last_sync_time.write() = Some(Utc::now());
            Ok(result.stats.unwrap_or_default())
        } else {
            Err(AllianceError::internal(format!(
                "Incremental sync failed: {}",
                result.error.unwrap_or_else(|| "unknown".into())
            )))
        }
    }

    /// 获取当前状态
    pub fn state(&self) -> SynchronizerState {
        *self.state.read()
    }

    /// 获取最后同步时间
    pub fn last_sync_time(&self) -> Option<DateTime<Utc>> {
        *self.last_sync_time.read()
    }

    /// 获取最后全量同步时间
    pub fn last_full_sync_time(&self) -> Option<DateTime<Utc>> {
        *self.last_full_sync.read()
    }

    /// 获取最后同步结果
    pub fn last_result(&self) -> Option<SyncResult> {
        self.last_result.read().clone()
    }

    fn record_result(&self, result: SyncResult) {
        *self.last_result.write() = Some(result);
    }
}

// ============================================================================
// 同步执行函数
// ============================================================================

/// 执行全量同步
async fn do_full_sync<D, R>(
    data_source: &Arc<D>,
    registry: &Arc<R>,
    config: &SynchronizerConfig,
) -> SyncResult
where
    D: ExpertDataSource + ?Sized,
    R: ExpertRegistryBridge + ?Sized,
{
    let start = std::time::Instant::now();
    let mut result = SyncResult {
        mode: SyncMode::Full,
        sync_time: Utc::now(),
        ..Default::default()
    };

    let mut attempt = 0;
    loop {
        attempt += 1;
        match data_source.fetch_all_experts().await {
            Ok(experts) => {
                match registry.sync_experts(experts).await {
                    Ok(count) => {
                        result.success = true;
                        result.stats = Some(SyncStats {
                            added: count,
                            updated: 0,
                            removed: 0,
                            total: count,
                        });
                        result.duration_ms = start.elapsed().as_millis() as u64;
                        return result;
                    }
                    Err(e) => {
                        result.error = Some(format!("Registry sync error: {}", e));
                    }
                }
            }
            Err(e) => {
                result.error = Some(format!("Fetch error: {}", e));
            }
        }

        if attempt >= config.max_retries {
            break;
        }
        warn!(
            "Full sync attempt {}/{} failed, retrying in {}s...",
            attempt, config.max_retries, config.retry_interval_secs
        );
        tokio::time::sleep(Duration::from_secs(config.retry_interval_secs)).await;
    }

    result.duration_ms = start.elapsed().as_millis() as u64;
    result
}

/// 执行增量同步
async fn do_incremental_sync<D, R>(
    data_source: &Arc<D>,
    registry: &Arc<R>,
    config: &SynchronizerConfig,
    since: DateTime<Utc>,
) -> SyncResult
where
    D: ExpertDataSource + ?Sized,
    R: ExpertRegistryBridge + ?Sized,
{
    let start = std::time::Instant::now();
    let mut result = SyncResult {
        mode: SyncMode::Incremental,
        sync_time: Utc::now(),
        ..Default::default()
    };

    let mut attempt = 0;
    loop {
        attempt += 1;
        match data_source.fetch_updated_experts(since).await {
            Ok(experts) => {
                // 对于增量同步，我们需要逐个注册/更新专家
                let mut added = 0usize;
                let mut updated = 0usize;
                for expert in experts {
                    let exists = registry.get_expert(&expert.expert_id).await
                        .ok()
                        .flatten()
                        .is_some();
                    if exists {
                        updated += 1;
                    } else {
                        added += 1;
                    }
                    if let Err(e) = registry.register_expert(expert).await {
                        result.error = Some(format!("Register error: {}", e));
                    }
                }
                result.success = true;
                let total = registry.expert_count().await;
                result.stats = Some(SyncStats {
                    added,
                    updated,
                    removed: 0,
                    total,
                });
                result.duration_ms = start.elapsed().as_millis() as u64;
                return result;
            }
            Err(e) => {
                result.error = Some(format!("Fetch error: {}", e));
            }
        }

        if attempt >= config.max_retries {
            break;
        }
        warn!(
            "Incremental sync attempt {}/{} failed, retrying in {}s...",
            attempt, config.max_retries, config.retry_interval_secs
        );
        tokio::time::sleep(Duration::from_secs(config.retry_interval_secs)).await;
    }

    result.duration_ms = start.elapsed().as_millis() as u64;
    result
}

/// 执行健康状态更新
async fn do_health_update<D, R>(
    data_source: &Arc<D>,
    registry: &Arc<R>,
) -> SyncResult
where
    D: ExpertDataSource + ?Sized,
    R: ExpertRegistryBridge + ?Sized,
{
    let start = std::time::Instant::now();
    let mut result = SyncResult {
        mode: SyncMode::HealthOnly,
        sync_time: Utc::now(),
        ..Default::default()
    };

    match data_source.fetch_health_status().await {
        Ok(health_list) => {
            let mut updated = 0usize;
            for (expert_id, health) in health_list {
                if registry.update_expert_health(&expert_id, health).await.is_ok() {
                    updated += 1;
                }
            }
            result.success = true;
            result.stats = Some(SyncStats {
                added: 0,
                updated,
                removed: 0,
                total: registry.expert_count().await,
            });
            result.duration_ms = start.elapsed().as_millis() as u64;
        }
        Err(e) => {
            result.error = Some(format!("Health fetch error: {}", e));
            result.duration_ms = start.elapsed().as_millis() as u64;
        }
    }

    result
}

/// 记录同步结果
fn record_sync_result(
    last_result_arc: &Arc<parking_lot::RwLock<Option<SyncResult>>>,
    result: SyncResult,
) {
    *last_result_arc.write() = Some(result);
}

// ============================================================================
// 内置数据源：内存数据源（用于测试）
// ============================================================================

/// 内存版专家数据源（用于测试）
pub struct InMemoryExpertDataSource {
    experts: parking_lot::RwLock<Vec<Expert>>,
}

impl InMemoryExpertDataSource {
    pub fn new() -> Self {
        Self {
            experts: parking_lot::RwLock::new(Vec::new()),
        }
    }

    pub fn with_experts(experts: Vec<Expert>) -> Self {
        Self {
            experts: parking_lot::RwLock::new(experts),
        }
    }

    pub fn set_experts(&self, experts: Vec<Expert>) {
        *self.experts.write() = experts;
    }

    pub fn add_expert(&self, expert: Expert) {
        self.experts.write().push(expert);
    }
}

impl Default for InMemoryExpertDataSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExpertDataSource for InMemoryExpertDataSource {
    async fn fetch_all_experts(&self) -> AllianceResult<Vec<Expert>> {
        Ok(self.experts.read().clone())
    }

    async fn fetch_updated_experts(&self, _since: DateTime<Utc>) -> AllianceResult<Vec<Expert>> {
        Ok(self.experts.read().clone())
    }

    async fn fetch_health_status(&self) -> AllianceResult<Vec<(String, ExpertHealth)>> {
        Ok(vec![])
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::InMemoryExpertRegistry;

    fn make_test_expert(id: &str, name: &str) -> Expert {
        let mut e = Expert::new_system(name.to_string(), format!("Expert {}", name));
        e.expert_id = id.to_string();
        e
    }

    #[tokio::test]
    async fn full_sync_basic() {
        let data_source = Arc::new(InMemoryExpertDataSource::with_experts(vec![
            make_test_expert("e1", "Expert1"),
            make_test_expert("e2", "Expert2"),
            make_test_expert("e3", "Expert3"),
        ]));
        let registry = Arc::new(InMemoryExpertRegistry::new());
        let config = SynchronizerConfig {
            max_retries: 1,
            ..Default::default()
        };

        let result = do_full_sync(&data_source, &registry, &config).await;
        assert!(result.success);
        assert_eq!(result.stats.as_ref().unwrap().total, 3);
        assert_eq!(registry.expert_count().await, 3);
    }

    #[tokio::test]
    async fn full_sync_replaces_existing() {
        let registry = Arc::new(InMemoryExpertRegistry::new());
        registry
            .register_expert(make_test_expert("old", "OldExpert"))
            .await
            .unwrap();
        assert_eq!(registry.expert_count().await, 1);

        let data_source = Arc::new(InMemoryExpertDataSource::with_experts(vec![
            make_test_expert("new1", "New1"),
            make_test_expert("new2", "New2"),
        ]));
        let config = SynchronizerConfig {
            max_retries: 1,
            ..Default::default()
        };

        let result = do_full_sync(&data_source, &registry, &config).await;
        assert!(result.success);
        assert_eq!(registry.expert_count().await, 2);
        assert!(registry.get_expert("old").await.unwrap().is_none());
        assert!(registry.get_expert("new1").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn incremental_sync_adds_new() {
        let registry = Arc::new(InMemoryExpertRegistry::new());
        registry
            .register_expert(make_test_expert("e1", "E1"))
            .await
            .unwrap();

        let data_source = Arc::new(InMemoryExpertDataSource::with_experts(vec![
            make_test_expert("e1", "E1"),
            make_test_expert("e2", "E2"),
        ]));
        let config = SynchronizerConfig {
            max_retries: 1,
            ..Default::default()
        };

        let result =
            do_incremental_sync(&data_source, &registry, &config, Utc::now()).await;
        assert!(result.success);
        let stats = result.stats.unwrap();
        assert_eq!(stats.added, 1);
        assert_eq!(stats.updated, 1);
        assert_eq!(stats.total, 2);
    }

    #[tokio::test]
    async fn synchronizer_manual_full_sync() {
        let data_source = Arc::new(InMemoryExpertDataSource::with_experts(vec![
            make_test_expert("e1", "E1"),
            make_test_expert("e2", "E2"),
        ]));
        let registry = Arc::new(InMemoryExpertRegistry::new());
        let config = SynchronizerConfig {
            auto_sync_enabled: false,
            max_retries: 1,
            ..Default::default()
        };

        let sync = ExpertSynchronizer::new(config, data_source, registry.clone());
        let stats = sync.trigger_full_sync().await.unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(registry.expert_count().await, 2);
    }

    #[tokio::test]
    async fn synchronizer_start_stop() {
        let data_source = Arc::new(InMemoryExpertDataSource::with_experts(vec![
            make_test_expert("e1", "E1"),
        ]));
        let registry = Arc::new(InMemoryExpertRegistry::new());
        let config = SynchronizerConfig {
            auto_sync_enabled: true,
            full_sync_interval_secs: 3600, // 长间隔，避免自动触发
            incremental_sync_interval_secs: 0,
            health_update_interval_secs: 0,
            initial_delay_secs: 0,
            max_retries: 1,
            ..Default::default()
        };

        let sync = ExpertSynchronizer::new(config, data_source, registry.clone());

        // 启动
        sync.start().await.unwrap();

        // 等待初始同步完成
        tokio::time::sleep(Duration::from_millis(200)).await;

        // 验证已同步
        assert_eq!(registry.expert_count().await, 1);
        assert!(sync.last_sync_time().is_some());

        // 停止
        sync.stop().await.unwrap();
        assert_eq!(sync.state(), SynchronizerState::Stopped);
    }

    #[tokio::test]
    async fn health_update_updates_existing() {
        let registry = Arc::new(InMemoryExpertRegistry::new());
        let mut expert = make_test_expert("e1", "E1");
        expert.health.is_healthy = true;
        registry.register_expert(expert).await.unwrap();

        // 创建带健康状态的数据源
        struct HealthDataSource {
            health_data: parking_lot::RwLock<Vec<(String, ExpertHealth)>>,
        }

        #[async_trait]
        impl ExpertDataSource for HealthDataSource {
            async fn fetch_all_experts(&self) -> AllianceResult<Vec<Expert>> {
                Ok(vec![])
            }
            async fn fetch_health_status(&self) -> AllianceResult<Vec<(String, ExpertHealth)>> {
                Ok(self.health_data.read().clone())
            }
        }

        let health = ExpertHealth {
            is_healthy: false,
            success_rate: 0.5,
            ..Default::default()
        };

        let data_source = Arc::new(HealthDataSource {
            health_data: parking_lot::RwLock::new(vec![("e1".to_string(), health)]),
        });

        let result = do_health_update(&data_source, &registry).await;
        assert!(result.success);

        let updated = registry.get_expert("e1").await.unwrap().unwrap();
        assert!(!updated.health.is_healthy);
        assert!((updated.health.success_rate - 0.5).abs() < 1e-9);
    }

    #[test]
    fn sync_config_default_values() {
        let config = SynchronizerConfig::default();
        assert!(config.auto_sync_enabled);
        assert_eq!(config.full_sync_interval_secs, 300);
        assert_eq!(config.incremental_sync_interval_secs, 60);
        assert_eq!(config.health_update_interval_secs, 30);
        assert_eq!(config.max_retries, 3);
    }
}

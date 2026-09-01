// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Boot Config — 服务引导配置加载器
//!
//! 为专家联盟核心服务（scheduler-svc / executor-svc）提供 **yml 文件 + 环境变量覆盖**
//! 的引导配置加载能力，是"配置外部化 → Nacos 配置中心"演进的地基。
//!
//! ## 加载优先级（从低到高）
//! 1. **内置默认值**（`Default` 实现，与服务启动硬编码默认一致）
//! 2. **yml 文件**（`config/alliance-scheduler.yml` / `config/alliance-executor.yml`）
//! 3. **环境变量**（`MOX_ALLIANCE_*`，如 `MOX_ALLIANCE_SERVER_PORT`）
//!
//! ## 环境变量命名规则
//! `MOX_ALLIANCE_` + 配置路径全大写蛇形：如 `scheduler.max_concurrent_tasks` →
//! `MOX_ALLIANCE_SCHEDULER_MAX_CONCURRENT_TASKS`。
//!
//! ## 与 PORT-NORM-001 的一致性
//! 端口默认值遵循 `docs/standards/expert-alliance-port-norm.md`：
//! scheduler=3100、executor=3200、AI 专家桥接=3300（核心服务 3000-3999 段）。

use std::str::FromStr;

use serde::Deserialize;

pub mod config_store;
pub mod experts;

#[cfg(feature = "nacos")]
pub mod nacos_config;

#[cfg(feature = "naming")]
pub mod naming;

pub use config_store::{
    ConfigStore, ConfigStoreChain, ConfigStoreError, FileConfigStore, MemoryConfigStore,
};

pub use experts::{
    load_experts, ExpertModuleOverlay, ExpertsBootConfig, GlobalLlmOverlay,
};

#[cfg(feature = "naming")]
pub use naming::{NamingRegistry, NamingSection};

/// 环境变量前缀（统一命名空间）
const ENV_PREFIX: &str = "MOX_ALLIANCE_";

// ─────────────────────────── 服务器段 ───────────────────────────

/// 服务器监听配置
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerSection {
    /// 监听地址（默认 0.0.0.0）
    pub host: String,
    /// 监听端口（核心服务 3000-3999 段，PORT-NORM-001）
    pub port: u16,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3100,
        }
    }
}

// ─────────────────────────── 存储段（共享） ───────────────────────────

/// 任务仓库存储配置
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct StorageSection {
    /// 存储模式：file（文件快照）| memory（纯内存）
    pub mode: String,
    /// 文件快照路径（mode=file 时生效）
    pub path: String,
}

impl Default for StorageSection {
    fn default() -> Self {
        Self {
            mode: "file".to_string(),
            path: "data/alliance_tasks.json".to_string(),
        }
    }
}

// ─────────────────────────── 调度器配置 ───────────────────────────

/// 调度器业务配置（对应 `SchedulerConfig`）
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SchedulerSection {
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    /// 任务队列容量
    pub queue_capacity: usize,
    /// 默认任务优先级（normal | low | high | critical）
    pub default_priority: String,
    /// 默认协作模式（parallel | sequential | consult | debate）
    pub default_mode: String,
    /// 默认融合策略（weighted | voting | sequential | none）
    pub default_fusion_strategy: String,
    /// 计划生成超时（毫秒）
    pub plan_generation_timeout_ms: u64,
}

impl Default for SchedulerSection {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 100,
            queue_capacity: 1000,
            default_priority: "normal".to_string(),
            default_mode: "parallel".to_string(),
            default_fusion_strategy: "weighted".to_string(),
            plan_generation_timeout_ms: 30_000,
        }
    }
}

/// 执行器桥接配置（scheduler → executor-svc）
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ExecutorBridgeSection {
    /// 执行器服务基地址（默认 http://localhost:3200）
    pub base_url: String,
    /// 桥接请求超时（毫秒）
    pub timeout_ms: u64,
}

impl Default for ExecutorBridgeSection {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3200".to_string(),
            timeout_ms: 30_000,
        }
    }
}

/// AI 专家服务配置（scheduler 内部桥接的专家服务）
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ExpertServiceSection {
    /// AI 专家服务基地址（默认 http://localhost:3300）
    pub base_url: String,
    /// 专家服务请求超时（毫秒）
    pub timeout_ms: u64,
    /// 是否启用 HTTP 专家桥接（生产专家服务）。默认关闭；
    /// 启用后 scheduler 启动时从远程拉取专家并入匹配器，拉取失败优雅降级到内置（不崩溃）。
    pub enabled: bool,
}

impl Default for ExpertServiceSection {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3300".to_string(),
            timeout_ms: 5_000,
            enabled: false,
        }
    }
}

/// 调度器完整引导配置
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SchedulerBootConfig {
    pub server: ServerSection,
    pub scheduler: SchedulerSection,
    pub executor_bridge: ExecutorBridgeSection,
    pub expert_service: ExpertServiceSection,
    pub storage: StorageSection,
    pub nacos: NacosSection,
    #[cfg(feature = "naming")]
    pub naming: NamingSection,
}

impl Default for SchedulerBootConfig {
    fn default() -> Self {
        Self {
            server: ServerSection {
                port: 3100, // 调度编排核心端口（PORT-NORM-001）
                ..ServerSection::default()
            },
            scheduler: SchedulerSection::default(),
            executor_bridge: ExecutorBridgeSection::default(),
            expert_service: ExpertServiceSection::default(),
            storage: StorageSection::default(),
            nacos: NacosSection::default(),
            #[cfg(feature = "naming")]
            naming: NamingSection::default(),
        }
    }
}

// ─────────────────────────── 执行器配置 ───────────────────────────

/// 执行器业务配置（对应 `ExecutorConfig`）
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ExecutorSection {
    /// 执行器模式：expert（生产）| mock（开发/测试）
    pub mode: String,
    /// 最大并发执行节点数
    pub max_concurrent_nodes: usize,
    /// 节点默认超时（毫秒）
    pub default_node_timeout_ms: u64,
    /// 默认最大重试次数
    pub default_max_retries: u32,
    /// 调度轮询间隔（毫秒）
    pub poll_interval_ms: u64,
    /// 进度更新最小间隔（毫秒）
    pub progress_update_interval_ms: u64,
}

impl Default for ExecutorSection {
    fn default() -> Self {
        Self {
            mode: "expert".to_string(),
            max_concurrent_nodes: 50,
            default_node_timeout_ms: 300_000,
            default_max_retries: 3,
            poll_interval_ms: 100,
            progress_update_interval_ms: 500,
        }
    }
}

/// 执行器完整引导配置
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ExecutorBootConfig {
    pub server: ServerSection,
    pub executor: ExecutorSection,
    pub storage: StorageSection,
    pub nacos: NacosSection,
    #[cfg(feature = "naming")]
    pub naming: NamingSection,
}

impl Default for ExecutorBootConfig {
    fn default() -> Self {
        Self {
            server: ServerSection {
                port: 3200, // 执行核心端口（PORT-NORM-001）
                ..ServerSection::default()
            },
            executor: ExecutorSection::default(),
            storage: StorageSection::default(),
            nacos: NacosSection::default(),
            #[cfg(feature = "naming")]
            naming: NamingSection::default(),
        }
    }
}

// ─────────────────────────── Nacos 配置中心引导段 ───────────────────────────

/// Nacos 配置中心引导段（bootstrap）。
///
/// 本结构**始终可解析**（不依赖 nacos-sdk）；仅在 `features = ["nacos"]` 下，
/// `load_scheduler_with_nacos` / `load_executor_with_nacos` 才会真正连接 Nacos。
///
/// 语义：本地 yml 是**引导 + 兜底**；`enabled=true` 时，启动从 Nacos 拉取
/// `data_id` 对应的远程完整配置，**整体覆盖**本地 yml（远程优先、失败降级）。
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(default)]
pub struct NacosSection {
    /// 是否启用 Nacos 配置中心（默认关闭，保持本地 yml 优先）
    pub enabled: bool,
    /// Nacos 服务地址，如 `127.0.0.1:8848`
    pub server_addr: String,
    /// 命名空间（空 = public）
    pub namespace: String,
    /// 认证用户名（空 = 无鉴权）
    pub username: String,
    /// 认证密码
    pub password: String,
    /// 配置分组（默认 `DEFAULT_GROUP`）
    pub group: String,
    /// 当前服务绑定的 dataId（如 `mox-alliance-scheduler.yml`）
    pub data_id: String,
}

impl Default for NacosSection {
    fn default() -> Self {
        Self {
            enabled: false,
            server_addr: "127.0.0.1:8848".to_string(),
            namespace: String::new(),
            username: String::new(),
            password: String::new(),
            group: "DEFAULT_GROUP".to_string(),
            data_id: String::new(),
        }
    }
}

// ─────────────────────────── 加载与覆盖 ───────────────────────────

/// 从 yml 文件 + 环境变量加载调度器引导配置
///
/// - 文件不存在 → 使用内置默认（并输出警告）
/// - 文件存在但解析失败 → 返回错误（配置错误必须显式暴露，禁止静默吞掉）
/// - 环境变量 `MOX_ALLIANCE_*` 覆盖 yml 中对应字段
pub fn load_scheduler(path: &str) -> anyhow::Result<SchedulerBootConfig> {
    let content = load_from_file_raw(path)?;
    parse_scheduler_yaml(&content)
}

/// 从配置源链（ConfigStore）加载调度器引导配置。
///
/// `key` 为配置源中的配置名（如 `alliance-scheduler`）。链全部未命中 → 使用内置默认。
pub async fn load_scheduler_from_store(
    store: &dyn ConfigStore,
    key: &str,
) -> anyhow::Result<SchedulerBootConfig> {
    let content = store
        .load_raw(key)
        .await
        .map_err(|e| anyhow::anyhow!("配置源[{}]读取失败: {e}", store.name()))?;
    match content {
        Some(text) => parse_scheduler_yaml(&text),
        None => {
            tracing::warn!(
                store = store.name(),
                key = key,
                "配置源未命中，使用内置默认值。"
            );
            Ok(SchedulerBootConfig::default())
        }
    }
}

/// 从 yml 文本解析调度器配置（反序列化 + 内置默认合并 + env 覆盖）
fn parse_scheduler_yaml(content: &str) -> anyhow::Result<SchedulerBootConfig> {
    let cfg = parse_yaml::<SchedulerBootConfig>(content)?;
    let mut cfg = cfg.unwrap_or_default();
    apply_env_overrides_scheduler(&mut cfg);
    Ok(cfg)
}

/// 从 yml 文件 + 环境变量加载执行器引导配置
pub fn load_executor(path: &str) -> anyhow::Result<ExecutorBootConfig> {
    let content = load_from_file_raw(path)?;
    parse_executor_yaml(&content)
}

/// 从配置源链（ConfigStore）加载执行器引导配置。
pub async fn load_executor_from_store(
    store: &dyn ConfigStore,
    key: &str,
) -> anyhow::Result<ExecutorBootConfig> {
    let content = store
        .load_raw(key)
        .await
        .map_err(|e| anyhow::anyhow!("配置源[{}]读取失败: {e}", store.name()))?;
    match content {
        Some(text) => parse_executor_yaml(&text),
        None => {
            tracing::warn!(
                store = store.name(),
                key = key,
                "配置源未命中，使用内置默认值。"
            );
            Ok(ExecutorBootConfig::default())
        }
    }
}

/// 从 yml 文本解析执行器配置（反序列化 + 内置默认合并 + env 覆盖）
fn parse_executor_yaml(content: &str) -> anyhow::Result<ExecutorBootConfig> {
    let cfg = parse_yaml::<ExecutorBootConfig>(content)?;
    let mut cfg = cfg.unwrap_or_default();
    apply_env_overrides_executor(&mut cfg);
    Ok(cfg)
}

/// Nacos 模式加载调度器配置（bootstrap）。
///
/// 读取本地 yml（引导）→ 若 `nacos.enabled=true` 且 dataId 非空 → 从 Nacos 拉取
/// 远程完整配置**整体覆盖**本地；Nacos 不可达 → 告警并降级本地 yml。
#[cfg(feature = "nacos")]
pub async fn load_scheduler_with_nacos(path: &str) -> anyhow::Result<SchedulerBootConfig> {
    let local = load_scheduler(path)?;
    if let Some(remote) = fetch_remote_config::<SchedulerBootConfig>(&local.nacos, parse_scheduler_yaml)
        .await?
    {
        return Ok(remote);
    }
    Ok(local)
}

/// Nacos 模式加载执行器配置（bootstrap）。
#[cfg(feature = "nacos")]
pub async fn load_executor_with_nacos(path: &str) -> anyhow::Result<ExecutorBootConfig> {
    let local = load_executor(path)?;
    if let Some(remote) = fetch_remote_config::<ExecutorBootConfig>(&local.nacos, parse_executor_yaml)
        .await?
    {
        return Ok(remote);
    }
    Ok(local)
}

/// 尝试从 Nacos 拉取远程配置并解析；未启用 / 拉取失败返回 `Ok(None)`（由调用方用本地）。
#[cfg(feature = "nacos")]
async fn fetch_remote_config<T>(
    section: &NacosSection,
    parse: impl Fn(&str) -> anyhow::Result<T>,
) -> anyhow::Result<Option<T>> {
    use crate::nacos_config::NacosConfigStore;
    match NacosConfigStore::connect(section).await? {
        Some(store) => match store.load_raw(&section.data_id).await {
            Ok(Some(text)) => {
                tracing::info!(
                    data_id = %section.data_id,
                    "已从 Nacos 拉取远程配置，整体覆盖本地 yml"
                );
                Ok(Some(parse(&text)?))
            }
            Ok(None) => {
                tracing::warn!(
                    data_id = %section.data_id,
                    "Nacos 中无该配置，使用本地 yml"
                );
                Ok(None)
            }
            Err(e) => {
                tracing::warn!(err = %e, "Nacos 拉取失败，降级本地 yml");
                Ok(None)
            }
        },
        None => Ok(None),
    }
}

/// 读取文件原文（不存在 → Err 由调用方处理；存在读取失败 → 显式报错）
fn load_from_file_raw(path: &str) -> anyhow::Result<String> {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            tracing::info!("加载配置文件: {path}");
            Ok(content)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!("配置文件 {path} 不存在，使用内置默认值。");
            Ok(String::new())
        }
        Err(e) => Err(anyhow::anyhow!("配置文件 {path} 读取失败：{e}")),
    }
}

/// 从 yml 文本反序列化：空文本 → Ok(None)；解析失败 → 显式报错（fail-fast）
fn parse_yaml<T: for<'de> Deserialize<'de>>(content: &str) -> anyhow::Result<Option<T>> {
    if content.trim().is_empty() {
        return Ok(None);
    }
    match serde_yaml::from_str::<T>(content) {
        Ok(cfg) => Ok(Some(cfg)),
        Err(e) => Err(anyhow::anyhow!(
            "配置解析失败：{e}（配置错误必须显式暴露，禁止静默降级为默认值）"
        )),
    }
}

/// 环境变量覆盖工具：`MOX_ALLIANCE_<PATH>`
fn env_value(key: &str) -> Option<String> {
    std::env::var(format!("{ENV_PREFIX}{key}")).ok()
}

/// 用环境变量覆盖字段（解析失败保持原值并告警）
fn over_str(base: &mut String, key: &str) {
    if let Some(v) = env_value(key) {
        if !v.is_empty() {
            *base = v;
        }
    }
}

fn over_num<T: FromStr + Copy>(base: &mut T, key: &str) {
    if let Some(v) = env_value(key) {
        match v.parse::<T>() {
            Ok(n) => *base = n,
            Err(_) => tracing::warn!("环境变量 {ENV_PREFIX}{key}='{v}' 解析失败，保持原值。"),
        }
    }
}

/// 布尔环境变量覆盖：接受 true/false/1/0（大小写不敏感），解析失败告警保持原值
fn over_bool(base: &mut bool, key: &str) {
    if let Some(v) = env_value(key) {
        let lowered = v.to_ascii_lowercase();
        match lowered.as_str() {
            "true" | "1" => *base = true,
            "false" | "0" => *base = false,
            _ => tracing::warn!(
                "环境变量 {ENV_PREFIX}{key}='{v}' 不是布尔值（true/false/1/0），保持原值。"
            ),
        }
    }
}

/// 调度器环境变量覆盖
fn apply_env_overrides_scheduler(cfg: &mut SchedulerBootConfig) {
    // server
    over_str(&mut cfg.server.host, "SERVER_HOST");
    over_num(&mut cfg.server.port, "SERVER_PORT");
    // scheduler
    over_num(&mut cfg.scheduler.max_concurrent_tasks, "SCHEDULER_MAX_CONCURRENT_TASKS");
    over_num(&mut cfg.scheduler.queue_capacity, "SCHEDULER_QUEUE_CAPACITY");
    over_str(&mut cfg.scheduler.default_priority, "SCHEDULER_DEFAULT_PRIORITY");
    over_str(&mut cfg.scheduler.default_mode, "SCHEDULER_DEFAULT_MODE");
    over_str(
        &mut cfg.scheduler.default_fusion_strategy,
        "SCHEDULER_DEFAULT_FUSION_STRATEGY",
    );
    over_num(
        &mut cfg.scheduler.plan_generation_timeout_ms,
        "SCHEDULER_PLAN_GENERATION_TIMEOUT_MS",
    );
    // executor_bridge
    over_str(&mut cfg.executor_bridge.base_url, "EXECUTOR_BRIDGE_BASE_URL");
    over_num(&mut cfg.executor_bridge.timeout_ms, "EXECUTOR_BRIDGE_TIMEOUT_MS");
    // expert_service
    over_str(&mut cfg.expert_service.base_url, "EXPERT_SERVICE_BASE_URL");
    over_num(&mut cfg.expert_service.timeout_ms, "EXPERT_SERVICE_TIMEOUT_MS");
    over_bool(&mut cfg.expert_service.enabled, "EXPERT_SERVICE_ENABLED");
    // storage
    over_str(&mut cfg.storage.mode, "STORAGE_MODE");
    over_str(&mut cfg.storage.path, "STORAGE_PATH");
}

/// 执行器环境变量覆盖
fn apply_env_overrides_executor(cfg: &mut ExecutorBootConfig) {
    // server
    over_str(&mut cfg.server.host, "SERVER_HOST");
    over_num(&mut cfg.server.port, "SERVER_PORT");
    // executor
    // 执行器模式归一化：优先 `MOX_ALLIANCE_EXECUTOR_MODE`（新）；旧 `EXECUTOR_MODE` 兼容（deprecated，命中即告警并生效）
    match env_value("EXECUTOR_MODE") {
        Some(v) if !v.is_empty() => cfg.executor.mode = v,
        _ => {
            if let Ok(old) = std::env::var("EXECUTOR_MODE") {
                if !old.is_empty() {
                    tracing::warn!(
                        "环境变量 EXECUTOR_MODE 已废弃，请改用 MOX_ALLIANCE_EXECUTOR_MODE（旧值已生效兼容）"
                    );
                    cfg.executor.mode = old;
                }
            }
        }
    }
    over_num(
        &mut cfg.executor.max_concurrent_nodes,
        "EXECUTOR_MAX_CONCURRENT_NODES",
    );
    over_num(
        &mut cfg.executor.default_node_timeout_ms,
        "EXECUTOR_DEFAULT_NODE_TIMEOUT_MS",
    );
    over_num(
        &mut cfg.executor.default_max_retries,
        "EXECUTOR_DEFAULT_MAX_RETRIES",
    );
    over_num(&mut cfg.executor.poll_interval_ms, "EXECUTOR_POLL_INTERVAL_MS");
    over_num(
        &mut cfg.executor.progress_update_interval_ms,
        "EXECUTOR_PROGRESS_UPDATE_INTERVAL_MS",
    );
    // storage
    over_str(&mut cfg.storage.mode, "STORAGE_MODE");
    over_str(&mut cfg.storage.path, "STORAGE_PATH");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 默认值符合 PORT-NORM-001
    #[test]
    fn defaults_match_port_norm() {
        let s = SchedulerBootConfig::default();
        assert_eq!(s.server.port, 3100, "scheduler 核心端口应为 3100");
        assert_eq!(s.executor_bridge.base_url, "http://localhost:3200");
        assert_eq!(s.expert_service.base_url, "http://localhost:3300");
        assert!(
            !s.expert_service.enabled,
            "HTTP 专家桥接默认应关闭（expert_service.enabled=false）"
        );
        let e = ExecutorBootConfig::default();
        assert_eq!(e.server.port, 3200, "executor 核心端口应为 3200");
        assert_eq!(e.executor.mode, "expert", "生产默认应为 expert 模式");
    }

    /// expert_service.enabled 环境变量覆盖（布尔解析）
    #[test]
    fn expert_service_enabled_env_override() {
        std::env::set_var("MOX_ALLIANCE_EXPERT_SERVICE_ENABLED", "true");
        let mut cfg = SchedulerBootConfig::default();
        apply_env_overrides_scheduler(&mut cfg);
        assert!(cfg.expert_service.enabled, "env=true 应启用 HTTP 专家桥接");
        std::env::set_var("MOX_ALLIANCE_EXPERT_SERVICE_ENABLED", "0");
        let mut cfg = SchedulerBootConfig::default();
        apply_env_overrides_scheduler(&mut cfg);
        assert!(!cfg.expert_service.enabled, "env=0 应保持关闭");
        std::env::remove_var("MOX_ALLIANCE_EXPERT_SERVICE_ENABLED");
    }

    /// 配置解析失败必须显式报错（fail-fast，禁止静默降级为默认值）
    #[test]
    fn invalid_yaml_fails_fast() {
        let dir = std::env::temp_dir().join(format!("bootcfg_failfast_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("bad.yml");
        std::fs::write(&p, "server: [unclosed\n  bad: {").unwrap();
        let r = load_scheduler(p.to_str().unwrap());
        assert!(
            r.is_err(),
            "配置文件存在但解析失败时应返回错误（fail-fast），不得静默降级"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // 旧环境变量 EXECUTOR_MODE 兼容但标记 deprecated（新变量优先）
    // 注：该测试涉及全局 env 读写，与同文件内其他测试并行会竞态，
    //     故实现于 `tests/env_deprecated.rs`（独立进程隔离）。

    /// 环境变量覆盖生效
    #[test]
    fn env_overrides_port() {
        // 临时设置环境变量（测试进程内生效）
        std::env::set_var("MOX_ALLIANCE_SERVER_PORT", "3199");
        std::env::set_var("MOX_ALLIANCE_EXECUTOR_MODE", "mock");
        let s = SchedulerBootConfig::default();
        let mut cfg = s.clone();
        apply_env_overrides_scheduler(&mut cfg);
        assert_eq!(cfg.server.port, 3199);
        let mut ec = ExecutorBootConfig::default();
        apply_env_overrides_executor(&mut ec);
        assert_eq!(ec.executor.mode, "mock");
        // 清理，避免影响其他测试
        std::env::remove_var("MOX_ALLIANCE_SERVER_PORT");
        std::env::remove_var("MOX_ALLIANCE_EXECUTOR_MODE");
    }

    /// yml 解析 + 部分字段缺省使用默认值
    #[test]
    fn parses_yaml_with_defaults() {
        let yaml = r#"
server:
  port: 3123
scheduler:
  max_concurrent_tasks: 5
"#;
        let cfg: SchedulerBootConfig = serde_yaml::from_str(yaml).expect("yaml 应可解析");
        assert_eq!(cfg.server.port, 3123);
        assert_eq!(cfg.scheduler.max_concurrent_tasks, 5);
        // 缺省字段回退默认
        assert_eq!(cfg.scheduler.queue_capacity, 1000);
        assert_eq!(cfg.executor_bridge.base_url, "http://localhost:3200");
    }
}

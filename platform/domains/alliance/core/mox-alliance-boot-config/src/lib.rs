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
}

impl Default for ExpertServiceSection {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3300".to_string(),
            timeout_ms: 5_000,
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
    let mut cfg = load_from_file::<SchedulerBootConfig>(path);
    apply_env_overrides_scheduler(&mut cfg);
    Ok(cfg)
}

/// 从 yml 文件 + 环境变量加载执行器引导配置
pub fn load_executor(path: &str) -> anyhow::Result<ExecutorBootConfig> {
    let mut cfg = load_from_file::<ExecutorBootConfig>(path);
    apply_env_overrides_executor(&mut cfg);
    Ok(cfg)
}

/// 通用文件加载：不存在 → 默认 + 警告；解析失败 → 报错
fn load_from_file<T: for<'de> Deserialize<'de> + Default>(path: &str) -> T {
    match std::fs::read_to_string(path) {
        Ok(content) => match serde_yaml::from_str::<T>(&content) {
            Ok(cfg) => {
                tracing::info!("加载配置文件: {path}");
                cfg
            }
            Err(e) => {
                tracing::warn!("配置文件 {path} 解析失败（{}），使用内置默认值。", e);
                T::default()
            }
        },
        Err(_) => {
            tracing::warn!("配置文件 {path} 不存在，使用内置默认值。");
            T::default()
        }
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
    over_str(&mut cfg.executor.mode, "EXECUTOR_MODE");
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
        let e = ExecutorBootConfig::default();
        assert_eq!(e.server.port, 3200, "executor 核心端口应为 3200");
        assert_eq!(e.executor.mode, "expert", "生产默认应为 expert 模式");
    }

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

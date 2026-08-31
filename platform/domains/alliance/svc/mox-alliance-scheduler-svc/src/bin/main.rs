// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use std::net::SocketAddr;
use std::sync::Arc;

use mox_alliance_boot_config::{load_experts, load_scheduler};
use mox_alliance_common_proto::{AllianceMode, FusionStrategy, TaskPriority};
use mox_alliance_scheduler_core::{FileTaskRepository, InMemoryTaskRepository};
use mox_alliance_scheduler_proto::types::SchedulerConfig;
use mox_alliance_scheduler_svc::SchedulerServer;
use tracing_subscriber::EnvFilter;

/// 默认配置文件路径（可被环境变量 MOX_ALLIANCE_CONFIG_FILE 覆盖）
const DEFAULT_CONFIG_FILE: &str = "config/alliance-scheduler.yml";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,mox_alliance=debug")),
        )
        .init();

    // 加载引导配置：内置默认 < config/alliance-scheduler.yml < MOX_ALLIANCE_* 环境变量
    let config_file = std::env::var("MOX_ALLIANCE_CONFIG_FILE").unwrap_or_else(|_| {
        DEFAULT_CONFIG_FILE.to_string()
    });
    let boot = load_scheduler(&config_file)?;

    // 由引导配置构造调度器业务配置（SchedulerConfig）
    let config = SchedulerConfig {
        max_concurrent_tasks: boot.scheduler.max_concurrent_tasks,
        queue_capacity: boot.scheduler.queue_capacity,
        default_priority: parse_priority(&boot.scheduler.default_priority),
        default_mode: parse_mode(&boot.scheduler.default_mode),
        default_fusion_strategy: parse_fusion(&boot.scheduler.default_fusion_strategy),
        plan_generation_timeout_ms: boot.scheduler.plan_generation_timeout_ms,
    };

    // 监听地址：config/alliance-scheduler.yml → server.host/port（PORT-NORM-001: 3100）
    let addr: SocketAddr = format!("{}:{}", boot.server.host, boot.server.port).parse()?;

    // 任务仓库：按配置选择（file 快照 / 内存），显式注入
    let repository: Arc<dyn mox_alliance_scheduler_core::TaskRepository> =
        match boot.storage.mode.to_ascii_lowercase().as_str() {
            "memory" => {
                tracing::info!("任务仓库：内存模式（storage.mode=memory）");
                Arc::new(InMemoryTaskRepository::new())
            }
            _ => {
                let path = std::path::Path::new(&boot.storage.path);
                tracing::info!("任务仓库：文件快照模式 → {}", path.display());
                Arc::new(FileTaskRepository::new(path)?)
            }
        };

    // 专家配置外部化：config/alliance-experts.yml（全局 LLM 局部覆盖 + 模块按 module_id 合并）
    let experts_file = std::env::var("MOX_ALLIANCE_EXPERTS_FILE").unwrap_or_else(|_| {
        "config/alliance-experts.yml".to_string()
    });
    let experts = load_experts(&experts_file)?;

    // 构建服务器：Standalone 模式，桥接指向配置的执行器服务
    let server = SchedulerServer::new(config, addr)
        .with_executor_url(boot.executor_bridge.base_url.clone())
        .with_task_repository(repository)
        .with_experts(experts)
        .with_expert_service(boot.expert_service.clone());

    tracing::info!(
        "Scheduler 启动配置完成：监听 {}:{}，executor_bridge={}，expert_service={}",
        boot.server.host,
        boot.server.port,
        boot.executor_bridge.base_url,
        boot.expert_service.base_url,
    );

    server.run().await?;

    Ok(())
}

/// 解析任务优先级（normal | low | high | critical）
fn parse_priority(s: &str) -> TaskPriority {
    match s.to_ascii_lowercase().as_str() {
        "low" => TaskPriority::Low,
        "high" => TaskPriority::High,
        "critical" => TaskPriority::Critical,
        _ => TaskPriority::Normal,
    }
}

/// 解析协作模式（parallel | sequential | consult | debate | hierarchical | iterative | voting）
fn parse_mode(s: &str) -> AllianceMode {
    match s.to_ascii_lowercase().as_str() {
        "sequential" => AllianceMode::Sequential,
        "debate" => AllianceMode::Debate,
        "hierarchical" => AllianceMode::Hierarchical,
        "iterative" => AllianceMode::Iterative,
        "voting" => AllianceMode::Voting,
        _ => AllianceMode::Parallel,
    }
}

/// 解析融合策略（weighted | voting | confidence_weighted | concatenation | best_of | stacking | debate | map_reduce | iterative）
fn parse_fusion(s: &str) -> FusionStrategy {
    match s.to_ascii_lowercase().as_str() {
        "voting" => FusionStrategy::Voting,
        "confidence_weighted" => FusionStrategy::ConfidenceWeighted,
        "concatenation" => FusionStrategy::Concatenation,
        "best_of" => FusionStrategy::BestOf,
        "stacking" => FusionStrategy::Stacking,
        "debate" => FusionStrategy::Debate,
        "map_reduce" => FusionStrategy::MapReduce,
        "iterative" => FusionStrategy::Iterative,
        _ => FusionStrategy::Weighted,
    }
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use std::net::SocketAddr;

use mox_alliance_boot_config::load_executor;
use mox_alliance_executor_proto::types::ExecutorConfig;
use mox_alliance_executor_svc::{ExecutorMode, ExecutorServer};
use tracing_subscriber::EnvFilter;

/// 默认配置文件路径（可被环境变量 MOX_ALLIANCE_CONFIG_FILE 覆盖）
const DEFAULT_CONFIG_FILE: &str = "config/alliance-executor.yml";

/// 旧版执行器模式环境变量名（向后兼容；新规范为 MOX_ALLIANCE_EXECUTOR_MODE）
const LEGACY_EXECUTOR_MODE_ENV: &str = "EXECUTOR_MODE";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,mox_alliance=debug")),
        )
        .init();

    // 加载引导配置：内置默认 < config/alliance-executor.yml < MOX_ALLIANCE_* 环境变量
    let config_file = std::env::var("MOX_ALLIANCE_CONFIG_FILE").unwrap_or_else(|_| {
        DEFAULT_CONFIG_FILE.to_string()
    });
    let boot = load_executor(&config_file)?;

    // 由引导配置构造执行器业务配置（ExecutorConfig）
    let config = ExecutorConfig {
        max_concurrent_nodes: boot.executor.max_concurrent_nodes,
        default_node_timeout_ms: boot.executor.default_node_timeout_ms,
        default_max_retries: boot.executor.default_max_retries,
        poll_interval_ms: boot.executor.poll_interval_ms,
        progress_update_interval_ms: boot.executor.progress_update_interval_ms,
    };

    // 监听地址：config/alliance-executor.yml → server.host/port（PORT-NORM-001: 3200）
    let addr: SocketAddr = format!("{}:{}", boot.server.host, boot.server.port).parse()?;

    // 解析执行器模式（优先级：旧 EXECUTOR_MODE > yml/MOX_ALLIANCE_EXECUTOR_MODE > 默认 expert）
    // 默认 expert（生产级），仅显式 mock 才启用 Mock。
    // 严禁"声称生产实际走 Mock"——启动日志必须如实反映实际生效模式。
    let mode = std::env::var(LEGACY_EXECUTOR_MODE_ENV)
        .unwrap_or_else(|_| boot.executor.mode.clone())
        .to_ascii_lowercase();

    let server = match mode.as_str() {
        "mock" => {
            tracing::warn!(
                "executor.mode=mock：本实例使用 Mock 执行器，仅限开发/测试，严禁用于生产。"
            );
            ExecutorServer::new(config, addr).with_mode(ExecutorMode::Mock)
        }
        "expert" => {
            tracing::info!("executor.mode=expert：本实例使用真实 AI 专家执行器（生产模式）。");
            ExecutorServer::new(config, addr).with_mode(ExecutorMode::Expert)
        }
        other => {
            anyhow::bail!(
                "未知执行器模式 '{}'（仅支持 expert | mock），拒绝启动。",
                other
            );
        }
    };

    server.run().await?;

    Ok(())
}

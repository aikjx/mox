// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use std::net::SocketAddr;

use mox_alliance_boot_config::{load_executor_with_nacos, NamingRegistry};
use mox_alliance_executor_proto::types::ExecutorConfig;
use mox_alliance_executor_svc::ExecutorServer;
use tracing_subscriber::EnvFilter;

/// 默认配置文件路径（可被环境变量 MOX_ALLIANCE_CONFIG_FILE 覆盖）
const DEFAULT_CONFIG_FILE: &str = "config/alliance-executor.yml";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,mox_alliance=debug")),
        )
        .init();

    // 加载引导配置：内置默认 < config/alliance-executor.yml < Nacos(可选) < MOX_ALLIANCE_* 环境变量
    // Nacos 配置中心（阶段二）：nacos.enabled=true 时从 Nacos 拉取远程完整配置整体覆盖本地 yml，
    // 失败降级本地（不阻断启动）。
    let config_file = std::env::var("MOX_ALLIANCE_CONFIG_FILE").unwrap_or_else(|_| {
        DEFAULT_CONFIG_FILE.to_string()
    });
    let boot = load_executor_with_nacos(&config_file).await?;

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

    // 执行器模式：生产级真实 AI 专家执行器（无 Mock 路径）。
    // 严禁"声称生产实际走 Mock"——启动日志必须如实反映实际生效模式。
    tracing::info!("executor：真实 AI 专家执行器（生产模式，无 Mock 路径）。");
    let server = ExecutorServer::new(config, addr);

    // Nacos 注册中心（阶段三）：nacos.enabled + naming.enabled → 注册自身实例（失败告警不阻断）
    let naming = NamingRegistry::connect(&boot.nacos, &boot.naming).await?;
    if let Some(reg) = &naming {
        reg.register().await;
    }

    server.run().await?;

    // 服务退出：优雅注销注册中心实例（避免僵尸实例）
    if let Some(reg) = &naming {
        reg.deregister().await;
    }

    Ok(())
}

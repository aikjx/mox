// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use std::net::SocketAddr;

use mox_alliance_executor_proto::types::ExecutorConfig;
use mox_alliance_executor_svc::ExecutorServer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,mox_alliance=debug")),
        )
        .init();

    let config = ExecutorConfig::default();
    let addr: SocketAddr = "0.0.0.0:8082".parse()?;

    let server = ExecutorServer::new(config, addr);
    server.run().await?;

    Ok(())
}

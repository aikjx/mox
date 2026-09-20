// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家注册中心服务入口

use mox_alliance_registry_svc::{AppState, Config, run};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 tracing
    tracing_subscriber::fmt::init();

    let config = Config::default();
    let state = AppState::new(config)?;

    run(state).await
}

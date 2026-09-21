// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家注册中心服务入口
//!
//! 启动流程：初始化 tracing → 环境变量/默认配置加载 → 存储初始化
//! （目录库 + 注册中心快照）→ 后台心跳回收任务 → HTTP 服务启动。
//!
//! 配置经环境变量 `MOX_ALLIANCE_REGISTRY_*` 覆盖，见 [`Config::from_env`]。

use mox_alliance_registry_svc::{AppState, Config, run};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 tracing（可用 RUST_LOG 控制级别）
    tracing_subscriber::fmt::init();

    let config = Config::from_env();
    tracing::info!(
        bind = %config.bind_addr,
        snapshot = ?config.snapshot_path,
        reap_ms = config.reap_interval_ms,
        "启动专家注册中心服务"
    );

    let state = AppState::new(config)?;

    run(state).await
}

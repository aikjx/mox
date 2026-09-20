// Copyright (c) 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 服务器启动

use std::net::SocketAddr;
use tracing::info;

use crate::{AppState, routes::create_router};

/// 启动服务
pub async fn run(state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = state.config.bind_addr.parse()?;

    let app = create_router(state);

    info!("Registry Svc listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

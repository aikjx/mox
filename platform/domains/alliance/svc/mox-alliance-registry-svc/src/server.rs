// Copyright (c) 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 服务器启动

use std::net::SocketAddr;
use std::time::Duration;

use tracing::{error, info};

use crate::{AppState, routes::create_router};

/// 启动服务
///
/// 流程：解析监听地址 → 启动心跳租约后台回收任务 → 绑定端口并服务 HTTP。
pub async fn run(state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = state.config.bind_addr.parse()?;
    let reap_ms = state.config.reap_interval_ms;

    // 后台任务：周期扫描注册中心，摘除超过租约未续约的实例
    let reap_state = state.clone();
    let reap_task = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(reap_ms.max(500)));
        loop {
            ticker.tick().await;
            let evicted = reap_state.registry.reap_expired();
            if !evicted.is_empty() {
                info!(count = evicted.len(), "注册中心摘除过期实例: {:?}", evicted);
            }
        }
    });

    let app = create_router(state);

    info!("Registry Svc listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    // 主服务结束时取消后台回收任务
    let serve_result = axum::serve(listener, app).await;
    reap_task.abort();
    if let Err(e) = serve_result {
        error!("Registry Svc 服务异常退出: {e}");
        return Err(Box::new(e));
    }
    Ok(())
}

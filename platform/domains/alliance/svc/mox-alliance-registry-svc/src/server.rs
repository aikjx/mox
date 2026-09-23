// Copyright (c) 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 服务器启动

use std::net::SocketAddr;
use std::time::Duration;

use tracing::{error, info};

use crate::health_probe::{HttpHealthProbe, run_probe_cycle};
use crate::{AppState, routes::create_router};

/// 启动服务
///
/// 流程：解析监听地址 → 启动心跳租约后台回收任务 → （可选）启动主动健康探测任务
/// → 绑定端口并服务 HTTP。服务退出时回收与探测任务均被优雅终止。
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

    // 后台任务：主动健康探测（仅在 health_probe_enabled 时启动）。
    // 用 oneshot 做优雅关闭信号——serve 返回后 drop 发送端，探测循环 select 到即退出。
    let (probe_shutdown_tx, probe_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let mut probe_tx = Some(probe_shutdown_tx);
    let probe_task = if state.config.health_probe_enabled {
        let probe_state = state.clone();
        let interval = probe_state.config.health_probe_interval_ms;
        let timeout_ms = probe_state.config.health_probe_timeout_ms;
        info!(
            interval_ms = interval,
            timeout_ms = timeout_ms,
            "启动后台主动健康探测任务"
        );
        match HttpHealthProbe::new(timeout_ms) {
            Ok(probe) => Some(tokio::spawn(async move {
                let mut ticker =
                    tokio::time::interval(Duration::from_millis(interval.max(500)));
                // oneshot Receiver 不可 Copy：pin 后以 &mut 复用，循环多次 select。
                tokio::pin!(probe_shutdown_rx);
                loop {
                    tokio::select! {
                        _ = ticker.tick() => {
                            run_probe_cycle(&probe_state.registry, &probe).await;
                        }
                        _ = &mut probe_shutdown_rx => {
                            info!("后台主动健康探测任务已停止");
                            break;
                        }
                    }
                }
            })),
            Err(e) => {
                error!(error = %e, "构造 HTTP 健康探测器失败，本轮不启动主动探测");
                // 丢弃未使用的关闭通道，避免任务句柄悬空。
                drop(probe_tx.take());
                None
            }
        }
    } else {
        // 未启用：探测任务本就不启动，关闭通道无需保留。
        drop(probe_tx.take());
        None
    };

    let app = create_router(state);

    info!("Registry Svc listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await;
    // 主服务结束时取消后台回收任务；探测任务经 shutdown 信号优雅退出后 join。
    let serve_result = match listener {
        Ok(l) => axum::serve(l, app).await,
        Err(e) => Err(e),
    };
    // 触发探测任务退出并等待其收尾（drop 发送端即令 recv 就绪）。
    if let Some(t) = probe_task {
        drop(probe_tx.take());
        let _ = t.await;
    }
    reap_task.abort();
    if let Err(e) = serve_result {
        error!("Registry Svc 服务异常退出: {e}");
        return Err(Box::new(e));
    }
    Ok(())
}

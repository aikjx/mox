// =============================================================================
// 优雅停机（shutdown_signal）
// =============================================================================
//
// 监听 SIGTERM / SIGINT（Ctrl+C）信号，返回 Future，信号到达时完成。
// 用于 axum::serve().with_graceful_shutdown()。
// =============================================================================

use tokio::signal;

/// 等待停机信号（SIGTERM 或 SIGINT）
pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("收到 SIGINT (Ctrl+C) 信号，开始优雅停机");
        }
        _ = terminate => {
            tracing::info!("收到 SIGTERM 信号，开始优雅停机");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown_signal_compiles() {
        // 仅验证函数可调用（不会实际等待信号）
        // 实际信号测试需要发送信号，不在单元测试范围内
        let _ = shutdown_signal;
    }
}

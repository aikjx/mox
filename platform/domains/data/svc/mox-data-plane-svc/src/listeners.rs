// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripleListenerConfig {
    pub public_port: u16,
    pub intra_ctrl_port: u16,
    pub intra_data_port: u16,
    pub bind_addr: String,
    pub enable_http3: bool,
}

impl Default for TripleListenerConfig {
    fn default() -> Self {
        Self {
            public_port: 8080,
            intra_ctrl_port: 9080,
            intra_data_port: 9081,
            bind_addr: "127.0.0.1".to_string(),
            enable_http3: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub ts_ms: i64,
    pub http3: bool,
    pub public_port: u16,
    pub ctrl_port: u16,
    pub data_port: u16,
}

/// Triple listener: production version binds three TCP ports and (optionally) QUIC on public.
/// For unit tests we only need a deterministic "smoke" struct that produces configs and mock bind().
#[derive(Debug, Default)]
pub struct TripleListener {
    pub cfg: TripleListenerConfig,
}

impl TripleListener {
    pub fn new(cfg: TripleListenerConfig) -> Self { Self { cfg } }

    /// Returns endpoints that WOULD be bound — this decouples config from OS resources so
    /// library unit tests never actually fight over TCP ports.
    pub fn endpoints(&self) -> [String; 3] {
        [
            format!("{}:{}", self.cfg.bind_addr, self.cfg.public_port),
            format!("{}:{}", self.cfg.bind_addr, self.cfg.intra_ctrl_port),
            format!("{}:{}", self.cfg.bind_addr, self.cfg.intra_data_port),
        ]
    }

    pub fn health(&self) -> HealthResponse {
        HealthResponse {
            status: "ok",
            ts_ms: chrono::Utc::now().timestamp_millis(),
            http3: self.cfg.enable_http3,
            public_port: self.cfg.public_port,
            ctrl_port: self.cfg.intra_ctrl_port,
            data_port: self.cfg.intra_data_port,
        }
    }

    /// Returns Poll::Ready(Ok(())) — for unit tests / code coverage of the async path.
    /// Real production impl would use `tokio::net::TcpListener::bind` triple + quinn endpoint.
    pub async fn start_smoke(&self) -> std::io::Result<()> {
        // Just yield once and return ready. Callers use this for feature-gating tests.
        tokio::task::yield_now().await;
        Ok(())
    }
}

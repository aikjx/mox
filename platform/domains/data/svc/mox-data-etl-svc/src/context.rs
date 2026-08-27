// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use serde::{Deserialize, Serialize};

/// Immutable mox_platform_orchestrator_svc context delivered to every transform.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EtContext {
    pub uri: String,
    pub bucket: String,
    pub miji_level: Option<u8>,
    pub legal_hold_until_ms: Option<i64>,
    pub request_id: String,
    pub user_sub: Option<String>,
    /// Per-request timeout for wasm plugins
    pub timeout_ms: u64,
}

impl EtContext {
    pub fn new(uri: impl Into<String>, bucket: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            bucket: bucket.into(),
            timeout_ms: 1000,
            ..Default::default()
        }
    }
}

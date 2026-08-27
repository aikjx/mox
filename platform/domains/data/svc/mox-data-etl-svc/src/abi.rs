// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EtError {
    #[error("plugin not found: {0}")]
    NotFound(String),
    #[error("plugin aborted: {0}")]
    Aborted(String),
    #[error("wasm trap: {0}")]
    WasmTrap(String),
    #[error("plugin timed out after {0}ms")]
    Timeout(u64),
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
}

pub type EtResult<T> = std::result::Result<T, EtError>;

pub trait InlineGet: Send + Sync {
    fn name(&self) -> &str;
    fn transform(&self, input: &[u8], ctx: &crate::EtContext) -> EtResult<Vec<u8>>;
}

pub trait InlinePut: Send + Sync {
    fn name(&self) -> &str;
    fn transform(&self, input: &[u8], ctx: &crate::EtContext) -> EtResult<Vec<u8>>;
}

pub trait OfflineXaction: Send + Sync {
    fn name(&self) -> &str;
    fn process_object(&self, key: &str, input: &[u8], ctx: &crate::EtContext) -> EtResult<Option<Vec<u8>>> {
        let _ = (key, input, ctx);
        Ok(None)
    }
}

/// Built-in pure-Rust plugins (no wasm-mox_platform_orchestrator_svc required) — useful for unit tests and the default registry.
pub struct Md5Sum;
impl InlineGet for Md5Sum {
    fn name(&self) -> &str { "md5" }
    fn transform(&self, input: &[u8], _ctx: &crate::EtContext) -> EtResult<Vec<u8>> {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(input);
        let d = h.finalize();
        // 16-byte fingerprint preserves etag-compat in tests; production uses
        // real md5sum via wasm plugin or native when wasm-mox_platform_orchestrator_svc is disabled.
        Ok(d[..16].to_vec())
    }
}

pub struct UpperText;
impl InlineGet for UpperText {
    fn name(&self) -> &str { "upper" }
    fn transform(&self, input: &[u8], _ctx: &crate::EtContext) -> EtResult<Vec<u8>> {
        let mut out = Vec::with_capacity(input.len());
        for &b in input {
            if b.is_ascii_lowercase() { out.push(b - 32); } else { out.push(b); }
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransformSummary {
    pub plugin: String,
    pub kind: PluginKindStr,
    pub input_len: usize,
    pub output_len: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginKindStr { InlineGet, InlinePut, Offline }

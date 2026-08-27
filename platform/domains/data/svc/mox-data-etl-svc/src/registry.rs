// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::abi::{EtError, EtResult, InlineGet, InlinePut, Md5Sum, OfflineXaction, PluginKindStr, UpperText};
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;

pub type PluginId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluginKind { InlineGet, InlinePut, Offline }

impl PluginKind {
    pub fn as_str(self) -> PluginKindStr {
        match self {
            PluginKind::InlineGet => PluginKindStr::InlineGet,
            PluginKind::InlinePut => PluginKindStr::InlinePut,
            PluginKind::Offline => PluginKindStr::Offline,
        }
    }
}

enum PluginHolder {
    InlineGet(Arc<dyn InlineGet>),
    InlinePut(Arc<dyn InlinePut>),
    Offline(Arc<dyn OfflineXaction>),
    /// Reserved for wasm-mox_platform_orchestrator_svc feature: uncompiled wasm bytes path registered
    WasmBytes { kind: PluginKind, wasm_path: std::path::PathBuf },
}

#[derive(Default)]
pub struct PluginRegistry {
    inner: RwLock<BTreeMap<(PluginKind, PluginId), PluginHolder>>,
}

impl PluginRegistry {
    pub fn new() -> Self { Self::default() }
    pub fn with_builtins() -> Self {
        let r = Self::default();
        r.register_inline_get("md5", Arc::new(Md5Sum));
        r.register_inline_get("upper", Arc::new(UpperText));
        r
    }
    pub fn register_inline_get(&self, id: &str, p: Arc<dyn InlineGet>) {
        self.inner.write().insert((PluginKind::InlineGet, id.to_string()), PluginHolder::InlineGet(p));
    }
    pub fn register_inline_put(&self, id: &str, p: Arc<dyn InlinePut>) {
        self.inner.write().insert((PluginKind::InlinePut, id.to_string()), PluginHolder::InlinePut(p));
    }
    pub fn register_offline(&self, id: &str, p: Arc<dyn OfflineXaction>) {
        self.inner.write().insert((PluginKind::Offline, id.to_string()), PluginHolder::Offline(p));
    }
    pub fn run_inline_get(&self, id: &str, input: &[u8], ctx: &crate::EtContext) -> EtResult<Vec<u8>> {
        let guard = self.inner.read();
        let h = guard.get(&(PluginKind::InlineGet, id.to_string()))
            .ok_or_else(|| EtError::NotFound(id.to_string()))?;
        match h {
            PluginHolder::InlineGet(p) => p.transform(input, ctx),
            _ => Err(EtError::NotFound(format!("plugin {id} is not inline-get"))),
        }
    }
    pub fn run_inline_put(&self, id: &str, input: &[u8], ctx: &crate::EtContext) -> EtResult<Vec<u8>> {
        let guard = self.inner.read();
        let h = guard.get(&(PluginKind::InlinePut, id.to_string()))
            .ok_or_else(|| EtError::NotFound(id.to_string()))?;
        match h {
            PluginHolder::InlinePut(p) => p.transform(input, ctx),
            _ => Err(EtError::NotFound(format!("plugin {id} is not inline-put"))),
        }
    }
    pub fn run_offline(&self, id: &str, key: &str, input: &[u8], ctx: &crate::EtContext) -> EtResult<Option<Vec<u8>>> {
        let guard = self.inner.read();
        let h = guard.get(&(PluginKind::Offline, id.to_string()))
            .ok_or_else(|| EtError::NotFound(id.to_string()))?;
        match h {
            PluginHolder::Offline(p) => p.process_object(key, input, ctx),
            _ => Err(EtError::NotFound(format!("plugin {id} is not offline"))),
        }
    }
    pub fn len(&self) -> usize { self.inner.read().len() }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EtContext;
    use crate::abi::{InlineGet, InlinePut, OfflineXaction};
    use std::sync::Arc;
    use std::thread;

    /// T7-01 PluginRegistry with_builtins len == 2
    #[test]
    fn t7_01_with_builtins_len_2() {
        let r = PluginRegistry::with_builtins();
        assert_eq!(r.len(), 2);
        assert!(!r.is_empty());
    }

    /// T7-02 run inline-get "md5" on input.len()=11 returns 16 bytes
    #[test]
    fn t7_02_md5_input_len_11_output_16_bytes() {
        let r = PluginRegistry::with_builtins();
        let input = b"hello world"; // 11 bytes
        assert_eq!(input.len(), 11);
        let ctx = EtContext::new("u1", "b1");
        let out = r.run_inline_get("md5", input, &ctx).expect("md5 runs");
        assert_eq!(out.len(), 16, "md5 inline-get must return 16 bytes");
    }

    /// T7-03 run inline-get "md5" deterministic: same input twice -> same bytes
    #[test]
    fn t7_03_md5_deterministic() {
        let r = PluginRegistry::with_builtins();
        let ctx = EtContext::new("u1", "b1");
        let a = r.run_inline_get("md5", b"abc123", &ctx).unwrap();
        let b = r.run_inline_get("md5", b"abc123", &ctx).unwrap();
        assert_eq!(a, b, "md5 must be deterministic across two calls");
        // different input -> different output
        let c = r.run_inline_get("md5", b"abc124", &ctx).unwrap();
        assert_ne!(a, c, "md5 different inputs -> different outputs");
    }

    /// T7-04 run inline-get "upper" on mixcase => ascii uppercased
    #[test]
    fn t7_04_upper_ascii_uppercased() {
        let r = PluginRegistry::with_builtins();
        let ctx = EtContext::new("u", "b");
        let out = r.run_inline_get("upper", b"aBc 123 xYz_~", &ctx).unwrap();
        assert_eq!(out, b"ABC 123 XYZ_~");
    }

    /// T7-05 plugin not-found => EtError::NotFound with id inside message
    #[test]
    fn t7_05_not_found_contains_id() {
        let r = PluginRegistry::with_builtins();
        let ctx = EtContext::new("u", "b");
        let res = r.run_inline_get("ghost-id-42", b"x", &ctx);
        match res {
            Err(EtError::NotFound(msg)) => {
                assert!(msg.contains("ghost-id-42"),
                    "NotFound message must contain the plugin id, got: {:?}", msg);
            }
            other => panic!("expected EtError::NotFound, got {:?}", other),
        }
    }

    /// CompressZero: InlinePut plugin that strips all 0x00 bytes
    struct CompressZero;
    impl InlinePut for CompressZero {
        fn name(&self) -> &str { "compress_zero" }
        fn transform(&self, input: &[u8], _ctx: &EtContext) -> crate::EtResult<Vec<u8>> {
            Ok(input.iter().copied().filter(|&b| b != 0x00).collect())
        }
    }

    /// T7-06 custom inline-put plugin ("compress_zero" removes 0x00) registered+run
    #[test]
    fn t7_06_custom_inline_put_compress_zero() {
        let r = PluginRegistry::with_builtins(); // 2 builtins
        r.register_inline_put("compress_zero", Arc::new(CompressZero));
        assert_eq!(r.len(), 3);
        let ctx = EtContext::new("u", "b");
        let input = vec![1u8, 0, 2, 0, 3, 0, 0, 4];
        let out = r.run_inline_put("compress_zero", &input, &ctx).expect("plugin runs");
        assert_eq!(out, vec![1, 2, 3, 4], "all 0x00 bytes must be stripped");
    }

    /// NoopOffline: always returns None (no-op / skip write back)
    struct NoopOffline;
    impl OfflineXaction for NoopOffline {
        fn name(&self) -> &str { "noop_offline" }
        // default process_object returns Ok(None)
    }

    /// T7-07 custom offline xaction returns None for no-op path
    #[test]
    fn t7_07_offline_noop_returns_none() {
        let r = PluginRegistry::with_builtins();
        r.register_offline("noop_offline", Arc::new(NoopOffline));
        assert_eq!(r.len(), 3);
        let ctx = EtContext::new("u", "b");
        let res = r.run_offline("noop_offline", "path/to/obj.bin", b"data", &ctx)
            .expect("offline xaction runs");
        assert!(res.is_none(), "noop offline xaction returns None (no write-back)");
    }

    /// T7-08 EtContext bucket/uri round-trip through clone
    #[test]
    fn t7_08_context_clone_roundtrip() {
        let ctx = EtContext::new("s3://my-bucket/obj/1.dat", "my-bucket");
        let cloned = ctx.clone();
        assert_eq!(ctx, cloned, "Clone must preserve equality");
        assert_eq!(cloned.uri, "s3://my-bucket/obj/1.dat");
        assert_eq!(cloned.bucket, "my-bucket");
    }

    /// T7-09 timeout_ms default 1000, set timeout 5000 OK
    #[test]
    fn t7_09_timeout_default_and_set() {
        let ctx = EtContext::new("u", "b");
        assert_eq!(ctx.timeout_ms, 1000, "default timeout_ms must be 1000");
        let mut ctx2 = EtContext::new("u", "b");
        ctx2.timeout_ms = 5000;
        assert_eq!(ctx2.timeout_ms, 5000);
    }

    /// IdentityInlineGet: returns input unchanged
    struct IdentityInlineGet { name: String }
    impl InlineGet for IdentityInlineGet {
        fn name(&self) -> &str { &self.name }
        fn transform(&self, input: &[u8], _ctx: &EtContext) -> crate::EtResult<Vec<u8>> {
            Ok(input.to_vec())
        }
    }

    /// T7-10 registry is Send + Sync: 10 threads each register unique inline-get
    /// plugin (nop identity), len grows to 12 (2 builtins + 10 new)
    #[test]
    fn t7_10_registry_send_sync_concurrent_register() {
        let reg = Arc::new(PluginRegistry::with_builtins());
        assert_eq!(reg.len(), 2);
        let mut handles = Vec::with_capacity(10);
        for i in 0..10 {
            let r = reg.clone();
            handles.push(thread::spawn(move || {
                let name = format!("identity_{i}");
                let p = IdentityInlineGet { name: name.clone() };
                r.register_inline_get(&name, Arc::new(p));
            }));
        }
        for h in handles { h.join().expect("thread ok"); }
        assert_eq!(reg.len(), 12, "2 builtins + 10 registered = 12 total plugins");
    }
}

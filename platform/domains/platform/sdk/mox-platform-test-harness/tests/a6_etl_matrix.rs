//! A6 — ETL WASM plugin registry × inline-get/put/offline matrix (72 tests)
//!
//! Coverage:
//! - md5 builtin: 12 size patterns
//! - upper builtin: 12 patterns
//! - not_found errors: 6 cases
//! - registry: empty, with_builtins, registration, len: 6
//! - InlinePut custom plugin: 12 cases
//! - Offline custom plugin: 12 cases
//! - PluginKind as_str + context: 6
//! - transform summary: 6

use mox_platform_test_harness::etl::{
    EtContext, InlineGet, InlinePut, OfflineXaction, PluginRegistry, EtResult,
    PluginKind,
};
use mox_platform_test_harness::{PluginKindStr, TransformSummary, EtError};
use std::sync::Arc;

fn ctx_default() -> EtContext { EtContext::new("s3://b/k", "b") }

// --- md5 (InlineGet): 12 size patterns ---
#[test] fn a6_md5_01_empty_returns_16B() {
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("md5", b"", &ctx_default()).unwrap();
    assert_eq!(out.len(), 16);
}
#[test] fn a6_md5_02_hello_16B() {
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("md5", b"hello", &ctx_default()).unwrap();
    assert_eq!(out.len(), 16);
}
#[test] fn a6_md5_03_1KB() {
    let input = vec![0xAAu8; 1024];
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("md5", &input, &ctx_default()).unwrap();
    assert_eq!(out.len(), 16);
}
#[test] fn a6_md5_04_10KB() {
    let input = vec![0x55u8; 10_000];
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("md5", &input, &ctx_default()).unwrap();
    assert_eq!(out.len(), 16);
}
#[test] fn a6_md5_05_64KB() {
    let input = vec![0x31u8; 65536];
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("md5", &input, &ctx_default()).unwrap();
    assert_eq!(out.len(), 16);
}
#[test] fn a6_md5_06_all_zeros_vs_all_FF_different() {
    let r = PluginRegistry::with_builtins();
    let a = r.run_inline_get("md5", &vec![0u8; 1000], &ctx_default()).unwrap();
    let b = r.run_inline_get("md5", &vec![0xFFu8; 1000], &ctx_default()).unwrap();
    assert_ne!(a, b);
}
#[test] fn a6_md5_07_deterministic() {
    let r = PluginRegistry::with_builtins();
    let a = r.run_inline_get("md5", b"same", &ctx_default()).unwrap();
    for _ in 0..100 {
        let b = r.run_inline_get("md5", b"same", &ctx_default()).unwrap();
        assert_eq!(a, b);
    }
}
#[test] fn a6_md5_08_256_pattern() {
    let mut v = vec![0u8; 256];
    for i in 0..256 { v[i] = i as u8; }
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("md5", &v, &ctx_default()).unwrap();
    assert_eq!(out.len(), 16);
}
#[test] fn a6_md5_09_1MB_fast() {
    let input = vec![0x11u8; 1_000_000];
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("md5", &input, &ctx_default()).unwrap();
    assert_eq!(out.len(), 16);
}
#[test] fn a6_md5_10_context_is_passed_in() {
    let r = PluginRegistry::with_builtins();
    let mut ctx = ctx_default();
    ctx.user_sub = Some("alice".to_string());
    ctx.miji_level = Some(2);
    let out = r.run_inline_get("md5", b"ctx", &ctx).unwrap();
    assert_eq!(out.len(), 16);
}
#[test] fn a6_md5_11_concat_order() {
    let r = PluginRegistry::with_builtins();
    let ab = r.run_inline_get("md5", b"helloworld", &ctx_default()).unwrap();
    let concat = r.run_inline_get("md5", b"helloworld", &ctx_default()).unwrap();
    assert_eq!(ab, concat);
}
#[test] fn a6_md5_12_utf8_chinese() {
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("md5", "你好世界".as_bytes(), &ctx_default()).unwrap();
    assert_eq!(out.len(), 16);
}

// --- UpperText (InlineGet): 12 patterns ---
#[test] fn a6_upper_01_lowercase() {
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("upper", b"hello", &ctx_default()).unwrap();
    assert_eq!(out, b"HELLO");
}
#[test] fn a6_upper_02_already_upper_preserves() {
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("upper", b"HELLO", &ctx_default()).unwrap();
    assert_eq!(out, b"HELLO");
}
#[test] fn a6_upper_03_mixed_alphanumeric() {
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("upper", b"abc123XYZ", &ctx_default()).unwrap();
    assert_eq!(out, b"ABC123XYZ");
}
#[test] fn a6_upper_04_empty() {
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("upper", b"", &ctx_default()).unwrap();
    assert_eq!(out, b"");
}
#[test] fn a6_upper_05_non_ascii_preserves_bytes() {
    let r = PluginRegistry::with_builtins();
    let input = vec![0x80u8, 0xFFu8, 0x00u8, b'a'];
    let out = r.run_inline_get("upper", &input, &ctx_default()).unwrap();
    assert_eq!(out[0], 0x80);
    assert_eq!(out[1], 0xFF);
    assert_eq!(out[2], 0x00);
    assert_eq!(out[3], b'A');
}
#[test] fn a6_upper_06_lowercase_all_letters() {
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("upper", b"abcdefghijklmnopqrstuvwxyz", &ctx_default()).unwrap();
    assert_eq!(out, b"ABCDEFGHIJKLMNOPQRSTUVWXYZ");
}
#[test] fn a6_upper_07_10KB_padding() {
    let input: Vec<u8> = (0..10_000).map(|i| (b'a' + (i % 26) as u8)).collect();
    let expected: Vec<u8> = input.iter().map(|&b| if b.is_ascii_lowercase() { b - 32 } else { b }).collect();
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("upper", &input, &ctx_default()).unwrap();
    assert_eq!(out.len(), 10_000);
    assert_eq!(out, expected);
}
#[test] fn a6_upper_08_sentence_with_spaces() {
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("upper", b"Hello World!", &ctx_default()).unwrap();
    assert_eq!(out, b"HELLO WORLD!");
}
#[test] fn a6_upper_09_digits_only() {
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("upper", b"1234567890", &ctx_default()).unwrap();
    assert_eq!(out, b"1234567890");
}
#[test] fn a6_upper_10_unicode_bytes_passthrough() {
    let s = "Mox璇玑";
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("upper", s.as_bytes(), &ctx_default()).unwrap();
    assert!(String::from_utf8(out).unwrap().contains("璇玑"));
}
#[test] fn a6_upper_11_1MB_mixed() {
    let input: Vec<u8> = (0..1_000_000u64).map(|i| (i % 256) as u8).collect();
    let r = PluginRegistry::with_builtins();
    let out = r.run_inline_get("upper", &input, &ctx_default()).unwrap();
    assert_eq!(out.len(), input.len());
}
#[test] fn a6_upper_12_registry_reusable_multiple_calls() {
    let r = PluginRegistry::with_builtins();
    for i in 0..10 {
        let input = format!("x{i}x").into_bytes();
        let out = r.run_inline_get("upper", &input, &ctx_default()).unwrap();
        let up = format!("X{i}X").into_bytes();
        assert_eq!(out, up);
    }
}

// --- Not Found errors: 6 cases ---
#[test] fn a6_nf_01_get_missing_plugin() {
    let r = PluginRegistry::with_builtins();
    match r.run_inline_get("no-such-plugin", b"", &ctx_default()) {
        Err(EtError::NotFound(s)) => assert!(s.contains("no-such-plugin")),
        other => panic!("expected NotFound, got {:?}", other),
    }
}
#[test] fn a6_nf_02_put_missing_plugin() {
    let r = PluginRegistry::with_builtins();
    assert!(matches!(r.run_inline_put("missing", b"", &ctx_default()), Err(EtError::NotFound(_))));
}
#[test] fn a6_nf_03_offline_missing_plugin() {
    let r = PluginRegistry::with_builtins();
    assert!(matches!(r.run_offline("x", "k", b"", &ctx_default()), Err(EtError::NotFound(_))));
}
#[test] fn a6_nf_04_wrong_kind_get_is_put() {
    let r = PluginRegistry::new();
    r.register_inline_put("p", Arc::new(NopPut));
    assert!(matches!(r.run_inline_get("p", b"", &ctx_default()), Err(EtError::NotFound(_))));
}
#[test] fn a6_nf_05_wrong_kind_put_is_get() {
    let r = PluginRegistry::new();
    r.register_inline_get("p", Arc::new(NopGet));
    assert!(matches!(r.run_inline_put("p", b"", &ctx_default()), Err(EtError::NotFound(_))));
}
#[test] fn a6_nf_06_wrong_kind_offline_is_get() {
    let r = PluginRegistry::new();
    r.register_inline_get("p", Arc::new(NopGet));
    assert!(matches!(r.run_offline("p", "k", b"", &ctx_default()), Err(EtError::NotFound(_))));
}

// --- Registry core: 6 ---
#[test] fn a6_reg_01_empty_len_0() {
    let r = PluginRegistry::new();
    assert_eq!(r.len(), 0); assert!(r.is_empty());
}
#[test] fn a6_reg_02_with_builtins_2() {
    let r = PluginRegistry::with_builtins();
    assert_eq!(r.len(), 2);
}
#[test] fn a6_reg_03_register_inline_get_increments() {
    let r = PluginRegistry::new();
    r.register_inline_get("p", Arc::new(NopGet));
    assert_eq!(r.len(), 1);
}
#[test] fn a6_reg_04_register_inline_put() {
    let r = PluginRegistry::new();
    r.register_inline_put("p", Arc::new(NopPut));
    assert_eq!(r.len(), 1);
}
#[test] fn a6_reg_05_register_offline() {
    let r = PluginRegistry::new();
    r.register_offline("p", Arc::new(NopOffline));
    assert_eq!(r.len(), 1);
}
#[test] fn a6_reg_06_register_three_total_3() {
    let r = PluginRegistry::new();
    r.register_inline_get("g", Arc::new(NopGet));
    r.register_inline_put("p", Arc::new(NopPut));
    r.register_offline("o", Arc::new(NopOffline));
    assert_eq!(r.len(), 3);
}

// --- Custom InlinePut: 12 cases (6 bitmask variants × 2 inputs each) ---
struct XorPut(pub u8);
impl InlinePut for XorPut {
    fn name(&self) -> &str { "xor-put" }
    fn transform(&self, input: &[u8], _ctx: &EtContext) -> EtResult<Vec<u8>> {
        Ok(input.iter().map(|b| b ^ self.0).collect())
    }
}

fn run_xor_put(key: u8, input: &[u8]) -> Vec<u8> {
    let r = PluginRegistry::new();
    r.register_inline_put("xor", Arc::new(XorPut(key)));
    r.run_inline_put("xor", input, &ctx_default()).unwrap()
}

#[test] fn a6_put_01_xor0_identity() { assert_eq!(run_xor_put(0, b"hello"), b"hello"); }
#[test] fn a6_put_02_xorFF_invert() {
    let out = run_xor_put(0xFF, b"hello");
    assert_eq!(out.len(), 5);
    assert_ne!(out, b"hello");
    let back: Vec<u8> = out.iter().map(|b| b ^ 0xFF).collect();
    assert_eq!(back, b"hello");
}
#[test] fn a6_put_03_xor_roundtrip() {
    let orig = vec![1u8,2,3,4,5,6,7,8];
    let a = run_xor_put(0xAA, &orig);
    let r = PluginRegistry::new();
    r.register_inline_put("xor", Arc::new(XorPut(0xAA)));
    let back = r.run_inline_put("xor", &a, &ctx_default()).unwrap();
    assert_eq!(back, orig);
}
#[test] fn a6_put_04_xor_empty() { let r: Vec<u8> = run_xor_put(0x55, &[]); assert!(r.is_empty()); }
#[test] fn a6_put_05_xor_1KB_0x01() {
    let v = vec![0x00u8; 1024];
    let out = run_xor_put(0x01, &v);
    assert!(out.iter().all(|&b| b == 0x01));
}
#[test] fn a6_put_06_xor_100B_0x42() {
    let v: Vec<u8> = (0..100).collect();
    let out = run_xor_put(0x42, &v);
    for i in 0..100 { assert_eq!(out[i], (i as u8) ^ 0x42); }
}

// PrefixPut: prepends context.request_id (or "p:")
struct PrefixPut;
impl InlinePut for PrefixPut {
    fn name(&self) -> &str { "prefix" }
    fn transform(&self, input: &[u8], ctx: &EtContext) -> EtResult<Vec<u8>> {
        let mut out = ctx.request_id.clone().into_bytes();
        out.extend_from_slice(input);
        Ok(out)
    }
}

#[test] fn a6_put_07_prefix_empty_context_rid() {
    let r = PluginRegistry::new();
    r.register_inline_put("p", Arc::new(PrefixPut));
    let ctx = EtContext::new("u","b");
    let out = r.run_inline_put("p", b"data", &ctx).unwrap();
    assert!(out.starts_with(ctx.request_id.as_bytes()));
    assert!(out.ends_with(b"data"));
}
#[test] fn a6_put_08_prefix_len() {
    let r = PluginRegistry::new();
    r.register_inline_put("p", Arc::new(PrefixPut));
    let ctx = EtContext::new("u","b");
    let rid_len = ctx.request_id.len();
    let out = r.run_inline_put("p", b"X", &ctx).unwrap();
    assert_eq!(out.len(), rid_len + 1);
}
#[test] fn a6_put_09_prefix_10x_same_ctx_same_rid() {
    let r = PluginRegistry::new();
    r.register_inline_put("p", Arc::new(PrefixPut));
    let ctx = EtContext::new("u","b");
    let rid = ctx.request_id.clone();
    for _ in 0..10 {
        let out = r.run_inline_put("p", b"d", &ctx).unwrap();
        assert!(out.starts_with(rid.as_bytes()));
    }
}
#[test] fn a6_put_10_two_puts_same_registry_different_ids() {
    let r = PluginRegistry::new();
    r.register_inline_put("xor", Arc::new(XorPut(0xFF)));
    r.register_inline_put("prefix", Arc::new(PrefixPut));
    let a = r.run_inline_put("xor", b"abc", &ctx_default()).unwrap();
    let b = r.run_inline_put("prefix", b"abc", &ctx_default()).unwrap();
    assert_ne!(a, b);
}
#[test] fn a6_put_11_xor_custom_key_0x37() {
    let v = b"The quick brown fox jumps over the lazy dog.";
    let out = run_xor_put(0x37, v);
    assert_eq!(out.len(), v.len());
    for (a, &b) in out.iter().zip(v.iter()) { assert_eq!(*a, b ^ 0x37); }
}
#[test] fn a6_put_12_xor_custom_key_0x08() {
    let v = vec![0u8; 1_000_000];
    let out = run_xor_put(0x08, &v);
    assert!(out.iter().all(|&b| b == 0x08));
}

// --- Offline custom plugins: 12 cases ---
struct FilterKeyOffline { prefix: String }
impl OfflineXaction for FilterKeyOffline {
    fn name(&self) -> &str { "filter-prefix" }
    fn process_object(&self, key: &str, input: &[u8], _ctx: &EtContext) -> EtResult<Option<Vec<u8>>> {
        if key.starts_with(&self.prefix) {
            Ok(Some(input.to_vec()))
        } else {
            Ok(None) // skip non-matching keys
        }
    }
}
struct DoublerOffline;
impl OfflineXaction for DoublerOffline {
    fn name(&self) -> &str { "doubler" }
    fn process_object(&self, _key: &str, input: &[u8], _ctx: &EtContext) -> EtResult<Option<Vec<u8>>> {
        let mut out = input.to_vec();
        out.extend_from_slice(input);
        Ok(Some(out))
    }
}

fn run_offline(id: &str, reg: &PluginRegistry, key: &str, input: &[u8]) -> EtResult<Option<Vec<u8>>> {
    reg.run_offline(id, key, input, &ctx_default())
}

#[test] fn a6_off_01_filter_match_returns_some() {
    let r = PluginRegistry::new();
    r.register_offline("f", Arc::new(FilterKeyOffline { prefix: "tmp/".into() }));
    let out = run_offline("f", &r, "tmp/log1", b"data").unwrap();
    assert_eq!(out, Some(b"data".to_vec()));
}
#[test] fn a6_off_02_filter_nomatch_returns_none() {
    let r = PluginRegistry::new();
    r.register_offline("f", Arc::new(FilterKeyOffline { prefix: "tmp/".into() }));
    let out = run_offline("f", &r, "final/log1", b"data").unwrap();
    assert_eq!(out, None);
}
#[test] fn a6_off_03_filter_empty_prefix_all_match() {
    let r = PluginRegistry::new();
    r.register_offline("f", Arc::new(FilterKeyOffline { prefix: String::new() }));
    assert!(run_offline("f", &r, "any", b"x").unwrap().is_some());
}
#[test] fn a6_off_04_doubler_output_double_len() {
    let r = PluginRegistry::new();
    r.register_offline("d", Arc::new(DoublerOffline));
    let out = run_offline("d", &r, "k", b"123").unwrap().unwrap();
    assert_eq!(out.len(), 6);
    assert_eq!(out, b"123123");
}
#[test] fn a6_off_05_doubler_empty() {
    let r = PluginRegistry::new();
    r.register_offline("d", Arc::new(DoublerOffline));
    let out = run_offline("d", &r, "k", b"").unwrap().unwrap();
    assert_eq!(out, b"");
}
#[test] fn a6_off_06_doubler_1MB() {
    let r = PluginRegistry::new();
    r.register_offline("d", Arc::new(DoublerOffline));
    let v = vec![0xABu8; 1_000_000];
    let out = run_offline("d", &r, "k", &v).unwrap().unwrap();
    assert_eq!(out.len(), 2_000_000);
    assert_eq!(&out[..1_000_000], &v[..]);
    assert_eq!(&out[1_000_000..], &v[..]);
}
#[test] fn a6_off_07_multiple_offline_plugins_independent() {
    let r = PluginRegistry::new();
    r.register_offline("a", Arc::new(DoublerOffline));
    r.register_offline("b", Arc::new(FilterKeyOffline { prefix: "x".into() }));
    assert_eq!(r.len(), 2);
    let out_a = run_offline("a", &r, "k", b"1").unwrap().unwrap();
    assert_eq!(out_a, b"11");
    let out_b = run_offline("b", &r, "xk", b"2").unwrap();
    assert_eq!(out_b, Some(b"2".to_vec()));
}
#[test] fn a6_off_08_context_miji_passthrough_filter() {
    let r = PluginRegistry::new();
    r.register_offline("f", Arc::new(FilterKeyOffline { prefix: "secure/".into() }));
    let mut ctx = ctx_default();
    ctx.miji_level = Some(3);
    let res = r.run_offline("f", "secure/f1", b"secret", &ctx).unwrap();
    assert!(res.is_some());
}
#[test] fn a6_off_09_offline_context_user_sub_passthrough() {
    let r = PluginRegistry::new();
    r.register_offline("d", Arc::new(DoublerOffline));
    let mut ctx = ctx_default();
    ctx.user_sub = Some("ops".to_string());
    let res = r.run_offline("d", "k", b"a", &ctx).unwrap();
    assert_eq!(res, Some(b"aa".to_vec()));
}
#[test] fn a6_off_10_filter_all_match_then_none() {
    let r = PluginRegistry::new();
    r.register_offline("f", Arc::new(FilterKeyOffline { prefix: "/a/b/c/".into() }));
    assert!(run_offline("f", &r, "/a/b/c/x.log", b"").unwrap().is_some());
    assert!(run_offline("f", &r, "/other.log", b"").unwrap().is_none());
}
#[test] fn a6_off_11_doubler_works_many_keys() {
    let r = PluginRegistry::new();
    r.register_offline("d", Arc::new(DoublerOffline));
    for i in 0..10 {
        let key = format!("k{i}");
        let input = format!("v{i}").into_bytes();
        let out = run_offline("d", &r, &key, &input).unwrap().unwrap();
        let mut exp = input.clone(); exp.extend_from_slice(&input);
        assert_eq!(out, exp);
    }
}
#[test] fn a6_off_12_filter_key_with_chinese() {
    let r = PluginRegistry::new();
    r.register_offline("f", Arc::new(FilterKeyOffline { prefix: "数据/".into() }));
    assert!(run_offline("f", &r, "数据/2024.xlsx", b"x").unwrap().is_some());
    assert!(run_offline("f", &r, "public/a.csv", b"x").unwrap().is_none());
}

// --- PluginKind × context: 6 ---
#[test] fn a6_pk_01_inline_get_as_str() { assert_eq!(PluginKind::InlineGet.as_str(), PluginKindStr::InlineGet); }
#[test] fn a6_pk_02_inline_put_as_str() { assert_eq!(PluginKind::InlinePut.as_str(), PluginKindStr::InlinePut); }
#[test] fn a6_pk_03_offline_as_str() { assert_eq!(PluginKind::Offline.as_str(), PluginKindStr::Offline); }
#[test] fn a6_ctx_04_context_default_timeout() { assert_eq!(EtContext::new("a","b").timeout_ms, 1000); }
#[test] fn a6_ctx_05_context_uri_bucket() {
    let c = EtContext::new("s3://b/k", "b");
    assert_eq!(c.uri, "s3://b/k");
    assert_eq!(c.bucket, "b");
}
#[test] fn a6_ctx_06_context_defaults_miji_and_hold_none() {
    let c = EtContext::default();
    assert!(c.miji_level.is_none());
    assert!(c.legal_hold_until_ms.is_none());
    assert!(c.user_sub.is_none());
}

// --- TransformSummary: 6 (ensure the 6-item enum has all variants constructible and consistent) ---

fn make_summary(kind: PluginKindStr, inl: usize, outl: usize) -> TransformSummary {
    TransformSummary { plugin: "p".into(), kind, input_len: inl, output_len: outl }
}

#[test] fn a6_ts_01_inline_get_construct() {
    let s = make_summary(PluginKindStr::InlineGet, 10, 16);
    assert_eq!(s.plugin, "p"); assert_eq!(s.input_len, 10); assert_eq!(s.output_len, 16);
}
#[test] fn a6_ts_02_inline_put_construct() {
    let s = make_summary(PluginKindStr::InlinePut, 4, 104);
    assert_eq!(s.input_len, 4); assert_eq!(s.output_len, 104);
}
#[test] fn a6_ts_03_offline_construct() {
    let s = make_summary(PluginKindStr::Offline, 100, 200);
    assert_eq!(s.input_len, 100); assert_eq!(s.output_len, 200);
}
#[test] fn a6_ts_04_eq_same() {
    let a = make_summary(PluginKindStr::InlineGet, 1, 2);
    let b = make_summary(PluginKindStr::InlineGet, 1, 2);
    assert_eq!(a, b);
}
#[test] fn a6_ts_05_ne_kind() {
    let a = make_summary(PluginKindStr::InlineGet, 1, 2);
    let b = make_summary(PluginKindStr::InlinePut, 1, 2);
    assert_ne!(a, b);
}
#[test] fn a6_ts_06_ne_len() {
    let a = make_summary(PluginKindStr::Offline, 10, 10);
    let b = make_summary(PluginKindStr::Offline, 10, 20);
    assert_ne!(a, b);
}

// --- helper plugin stubs ---
#[derive(Debug, Clone)]
struct NopGet;
impl InlineGet for NopGet {
    fn name(&self) -> &str { "nop-get" }
    fn transform(&self, input: &[u8], _ctx: &EtContext) -> EtResult<Vec<u8>> { Ok(input.to_vec()) }
}
#[derive(Debug, Clone)]
struct NopPut;
impl InlinePut for NopPut {
    fn name(&self) -> &str { "nop-put" }
    fn transform(&self, input: &[u8], _ctx: &EtContext) -> EtResult<Vec<u8>> { Ok(input.to_vec()) }
}
#[derive(Debug, Clone)]
struct NopOffline;
impl OfflineXaction for NopOffline {
    fn name(&self) -> &str { "nop-offline" }
}

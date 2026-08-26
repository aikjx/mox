//! FFI 探测：运行时判断是否链接了 sherpa-rs / libsherpa-onnx.so / dll 并暴露 ContextConfig
//!
//! 默认实现：**永远返回 Ok(false)**（未启用真实链接时，降级 S1→S2+S3，这是允许的设计，
//! 因为 Python glue 层会在它那边真正做 S1 注入；Rust 层保证协议/数据结构一致）。
//!
//! 若启用 feature = ["sherpa-real"]，可在此处做 libloading 动态加载；实际集成留给下游。

use std::sync::atomic::{AtomicI8, Ordering};

static CACHED: AtomicI8 = AtomicI8::new(-1);

/// 探测是否可用：0 不可用，1 可用；结果缓存（避免重复 dlopen）
pub fn sherpa_rs_context_config_available() -> Result<bool, String> {
    let cached = CACHED.load(Ordering::Acquire);
    if cached >= 0 {
        return Ok(cached == 1);
    }
    // 真实实现：feature gate 尝试 libloading::Library::new("sherpa-onnx")
    // .and_then(|lib| lib.get::<unsafe extern "C" fn()>(b"SherpaOnnxContextConfig_Create\0").is_ok())
    // 默认实现：未启用 feature → 不可用，由调用方使用 S2+S3
    #[cfg(feature = "sherpa-real")]
    {
        let probe = sherpa_real_probe();
        CACHED.store(if probe { 1 } else { 0 }, Ordering::Release);
        return Ok(probe);
    }
    #[cfg(not(feature = "sherpa-real"))]
    {
        CACHED.store(0, Ordering::Release);
        Ok(false)
    }
}

#[cfg(feature = "sherpa-real")]
fn sherpa_real_probe() -> bool {
    // 保守实现：探测环境变量 XIAOBAI_SHERPA_S1_ENABLED=1
    std::env::var("XIAOBAI_SHERPA_S1_ENABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

//! # xiaobai-asr · FR-5 热词三层注入 Rust 实现
//!
//! 设计目标 1:1 对齐 Python `asr/hotwords.py`：
//! - validate_and_rank()：复用 xiaobai_core::hotword::validate_and_rank（已实现 Levenshtein 去重、max_score 饱和、max_length 检查、黑名单 PII 警告）
//! - inject_into_model(S1)：Feature-gate `sherpa-rs` ContextConfig。**若 sherpa-rs 未启用/链接失败 → S1 降级标记**。通过 `cargo:rustc-cfg` 或 opt-in feature `sherpa-real` 启用，默认实现仅探测是否有 `sherpa_ons` 符号可链接（运行时 dlopen 探测 — 这里我们保守提供"探测函数 stub"）
//! - rebuild_with_hotwords_file(S2)：写 `/tmp/xiaobai_hotwords_<pid>.txt`（Windows `%TEMP%`）UTF-8 `word\tscore\n` 格式。调用方 Python/Tcl/F# 侧用同一个 file_path 重建 recognizer（我们 Rust 不直接调 FFI，走协议写路径即可 — Engine 调 `hotword_file_path()` 返回当前文件路径）
//! - apply_post_hoc(S3)：调 xiaobai_core::hotword::apply_fuzzy 替换 + 命中热词 Vec
//!
//! 本 crate **不** 直接依赖 sherpa-onnx-sys（避免 CI 编译太重）；实际接入时在顶层 bin crate 开 feature `xiaobai-asr/sherpa-real` 并注册。

pub mod injector;
pub mod ffi_probe;

pub use injector::{HotwordInjector, InjectionReport, LayerStatus as HotwordLayerStatus};
pub use ffi_probe::sherpa_rs_context_config_available;

/// 方便调用方：一行创建默认 Injector（无热词，需 set_hotwords 后再用）
pub fn default_injector() -> HotwordInjector {
    HotwordInjector::new()
}

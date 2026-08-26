//! # xiaobai-operators · FR-13 8 大类系统算子 Rust 权威实现
//!
//! - 与 Python `operator/{app,file,volume,input,network,display,browser,notify}_operator.py` 1:1 对齐
//! - 每个动作都实现 1~N 层回退链（Windows/windows-rs/CoreAudio → macOS/osascript → Linux/pactl/.../xdg-open）
//! - 所有阻塞 Win32/Shell 调用封装在 `execute()` 内，由 Engine 通过 `spawn_blocking` 隔离
//!
//! ## 快速启动（BallWidget/voice_proxy 用）
//! ```ignore
//! use mox_voice_operator_svc::register_all_defaults;
//! let engine = OperatorEngine::new(...);
//! register_all_defaults(&engine);
//! ```

pub mod app;
pub mod file;
pub mod volume;
pub mod input;
pub mod network;
pub mod display;
pub mod browser;
pub mod notify;
pub mod helpers;

#[cfg(feature = "server-3717")]
pub mod server_3717;

pub use app::AppOperator;
pub use file::FileOperator;
pub use volume::VolumeOperator;
pub use input::InputOperator;
pub use network::NetworkOperator;
pub use display::DisplayOperator;
pub use browser::BrowserOperator;
pub use notify::NotifyOperator;
pub use helpers::platform_tag;

#[cfg(feature = "server-3717")]
pub use server_3717::{XiaobaiVoiceService, VoiceServiceConfig, serve, build_router, run_service_blocking};

use std::sync::Arc;
use mox_voice_core_svc::engine::OperatorEngine;

/// 把 FR-13 全量 8 大类算子注册到 OperatorEngine（1 行搞定默认算子矩阵）
pub fn register_all_defaults(engine: &OperatorEngine) {
    engine.register(Arc::new(AppOperator::default()));
    engine.register(Arc::new(FileOperator::default()));
    engine.register(Arc::new(VolumeOperator::default()));
    engine.register(Arc::new(InputOperator::default()));
    engine.register(Arc::new(NetworkOperator::default()));
    engine.register(Arc::new(DisplayOperator::default()));
    engine.register(Arc::new(BrowserOperator::default()));
    engine.register(Arc::new(NotifyOperator::default()));
}

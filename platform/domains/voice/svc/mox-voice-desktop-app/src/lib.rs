// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # xiaobai-desktop · 桌面小白助手 BallWidget（P1 骨架，P2 Slint 动画后续补）
//!
//! 提供：
//! - `BallWidgetState` 5 状态：Idle（灰）/Listen（红声波）/Think（蓝脑波）/Speak（绿波形）/Executing（橙 executing 彩虹弧+齿轮）
//! - `DesktopApp` 负责：spawn voice_proxy 30010 服务 → BallWidget 事件 loop → 回调 dispatch_text
//! - `Alt+X` 全局热键占位：global-hotkey crate 绑定 Alt+X → 切换录音（P2 后续接入真实录音 ASR）
//! - Slint UI 代码在 `ui/ballwidget.slint`，Cargo 里开 feature = ["slint-ui"] 时用 slint-build 编译

pub mod ball_widget;
pub mod global_hotkeys;

pub use ball_widget::{BallWidgetState, DesktopApp, WidgetMode};
pub use global_hotkeys::HotkeyBindings;
// P2 语音引擎由 mox-voice-operator-svc::voice_engine 提供（feature voice-engine）
pub use mox_voice_operator_svc::voice_engine;
pub use mox_voice_operator_svc::voice_engine::{Recorder, VoiceEngine};

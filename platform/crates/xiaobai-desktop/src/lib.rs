//! # xiaobai-desktop · 桌面小白助手 BallWidget（P1 骨架，P2 Slint 动画后续补）
//!
//! 提供：
//! - `BallWidgetState` 5 状态：Idle（灰）/Listen（红声波）/Think（蓝脑波）/Speak（绿波形）/Executing（橙 executing 彩虹弧+齿轮）
//! - `DesktopApp` 负责：spawn voice_proxy 3717 服务 → BallWidget 事件 loop → 回调 dispatch_text
//! - `Alt+X` 全局热键占位：global-hotkey crate 绑定 Alt+X → 切换录音（P2 后续接入真实录音 ASR）
//! - Slint UI 代码在 `ui/ballwidget.slint`，Cargo 里开 feature = ["slint-ui"] 时用 slint-build 编译

pub mod ball_widget;
pub mod global_hotkeys;

pub use ball_widget::{BallWidgetState, DesktopApp, WidgetMode};
pub use global_hotkeys::HotkeyBindings;

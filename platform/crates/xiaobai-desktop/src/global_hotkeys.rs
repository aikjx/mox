//! 全局热键占位（P2 实现：Alt+X 录音开始/停止、Alt+S 静音切换、Alt+Q 隐藏悬浮球）
//!
//! 使用 global-hotkey crate（跨平台 Win/macOS/Linux）。P1 只提供 bind 定义 + 探测，
//! 不做录音设备对接（录音 → ASR 数据流另一个模块负责）。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 绑定定义
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HotkeyAction {
    /// Alt+X：录音开始/停止
    ToggleRecord,
    /// Alt+S：音量静音切换
    ToggleMute,
    /// Alt+Q：悬浮球显示/隐藏
    ToggleWidgetVisible,
    /// Alt+Shift+S：打开设置窗口
    OpenSettings,
}

impl HotkeyAction {
    pub const ALL: [HotkeyAction; 4] = [
        Self::ToggleRecord, Self::ToggleMute, Self::ToggleWidgetVisible, Self::OpenSettings,
    ];
    pub fn default_binding(&self) -> &'static str {
        match self {
            HotkeyAction::ToggleRecord => "Alt+X",
            HotkeyAction::ToggleMute => "Alt+S",
            HotkeyAction::ToggleWidgetVisible => "Alt+Q",
            HotkeyAction::OpenSettings => "Alt+Shift+S",
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct HotkeyBindings {
    pub bindings: BTreeMap<HotkeyAction, String>,
}

impl HotkeyBindings {
    pub fn with_defaults() -> Self {
        let mut b = BTreeMap::new();
        for a in HotkeyAction::ALL.iter() {
            b.insert(*a, a.default_binding().into());
        }
        Self { bindings: b }
    }
    /// 探测系统是否支持全局热键（有些无桌面环境 X11/Wayland 缺失会失败）
    pub fn probe_supported(&self) -> Result<bool, String> {
        // global-hotkey crate 需要事件循环；在 CI/headless 环境下返回 false 是可接受的
        #[cfg(feature = "global-hotkey")]
        {
            use global_hotkey::GlobalHotKeyManager;
            match GlobalHotKeyManager::new() {
                Ok(_) => Ok(true),
                Err(e) => Err(format!("{e:?}")),
            }
        }
        #[cfg(not(feature = "global-hotkey"))]
        {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn defaults_have_4_bindings() {
        let b = HotkeyBindings::with_defaults();
        assert_eq!(b.bindings.len(), 4);
        assert_eq!(b.bindings.get(&HotkeyAction::ToggleRecord).unwrap(), "Alt+X");
    }
}

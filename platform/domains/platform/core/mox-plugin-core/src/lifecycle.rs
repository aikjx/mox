// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 插件生命周期状态机
//!
//! ```text
//!           load()
//!   ┌──────────────────┐
//!   │                  ▼
//! Unloaded ──► Loaded ──► Initialized ──► Running
//!   ▲             │             │             │
//!   │             │             │             │
//!   │             ▼             ▼             ▼
//!   └──────── Unloaded ◄── Stopped ◄─── Paused
//!                  unload()     stop()      pause()
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// 插件生命周期状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    /// 未加载（磁盘上，未载入内存）
    Unloaded,
    /// 已加载（WASM模块载入内存，未初始化）
    Loaded,
    /// 已初始化（on_load已调用，能力已注册）
    Initialized,
    /// 运行中（正常提供服务）
    Running,
    /// 已暂停（暂停处理请求，但保留状态）
    Paused,
    /// 已停止（on_stop已调用，释放资源）
    Stopped,
    /// 错误状态（加载/初始化/运行出错）
    Error,
}

impl PluginState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginState::Unloaded => "unloaded",
            PluginState::Loaded => "loaded",
            PluginState::Initialized => "initialized",
            PluginState::Running => "running",
            PluginState::Paused => "paused",
            PluginState::Stopped => "stopped",
            PluginState::Error => "error",
        }
    }

    /// 是否可以转换到目标状态
    pub fn can_transition_to(&self, target: PluginState) -> bool {
        matches!(
            (*self, target),
            (PluginState::Unloaded, PluginState::Loaded)
                | (PluginState::Loaded, PluginState::Initialized)
                | (PluginState::Loaded, PluginState::Unloaded)
                | (PluginState::Initialized, PluginState::Running)
                | (PluginState::Initialized, PluginState::Stopped)
                | (PluginState::Running, PluginState::Paused)
                | (PluginState::Running, PluginState::Stopped)
                | (PluginState::Paused, PluginState::Running)
                | (PluginState::Paused, PluginState::Stopped)
                | (PluginState::Stopped, PluginState::Unloaded)
                | (_, PluginState::Error)
                | (PluginState::Error, PluginState::Unloaded)
        )
    }

    /// 是否处于活动状态（已初始化或运行中）
    pub fn is_active(&self) -> bool {
        matches!(self, PluginState::Initialized | PluginState::Running | PluginState::Paused)
    }
}

impl fmt::Display for PluginState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 生命周期转换错误
#[derive(Debug, thiserror::Error)]
pub enum LifecycleError {
    #[error("invalid state transition: {from} -> {to}")]
    InvalidTransition { from: PluginState, to: PluginState },

    #[error("plugin not found: {0}")]
    NotFound(String),

    #[error("plugin load failed: {0}")]
    LoadFailed(String),

    #[error("plugin init failed: {0}")]
    InitFailed(String),

    #[error("plugin start failed: {0}")]
    StartFailed(String),

    #[error("plugin stop failed: {0}")]
    StopFailed(String),
}

/// 插件生命周期事件（用于事件总线通知）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleEvent {
    pub plugin_id: String,
    pub plugin_name: String,
    pub from: PluginState,
    pub to: PluginState,
    pub timestamp: i64,
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        assert!(PluginState::Unloaded.can_transition_to(PluginState::Loaded));
        assert!(PluginState::Loaded.can_transition_to(PluginState::Initialized));
        assert!(PluginState::Initialized.can_transition_to(PluginState::Running));
        assert!(PluginState::Running.can_transition_to(PluginState::Paused));
        assert!(PluginState::Paused.can_transition_to(PluginState::Running));
        assert!(PluginState::Running.can_transition_to(PluginState::Stopped));
        assert!(PluginState::Stopped.can_transition_to(PluginState::Unloaded));
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(!PluginState::Unloaded.can_transition_to(PluginState::Running));
        assert!(!PluginState::Running.can_transition_to(PluginState::Initialized));
        assert!(!PluginState::Stopped.can_transition_to(PluginState::Running));
    }

    #[test]
    fn test_is_active() {
        assert!(!PluginState::Unloaded.is_active());
        assert!(!PluginState::Loaded.is_active());
        assert!(PluginState::Initialized.is_active());
        assert!(PluginState::Running.is_active());
        assert!(PluginState::Paused.is_active());
        assert!(!PluginState::Stopped.is_active());
    }
}

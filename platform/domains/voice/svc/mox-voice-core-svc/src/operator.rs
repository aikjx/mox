// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 系统算子抽象层：替代 Python `operator/base.Operator` + `OperatorEngine` 抽象
//!
//! 8 大类算子（FR-13 企业级，当前全部 Rust 化交付）：
//! | 类别 | Rust 枚举 | 动作数 | 典型 L0/L1/L2/L3 动作 |
//! | ---- | --------- | ------ | ------------------- |
//! | 应用 | App | 5 | list_running(L0) / open_app(L1) / open_file_with_app(L1) / close_app(L3) / shell_exec(L3) |
//! | 文件 | File | 6 | file_exists(L0) / read_text_head(L0) / open_file_with_app(L1) / copy_to_clipboard(L2) / move_to_trash(L3, Own) / hard_delete(L3) |
//! | 音量 | Volume | 6 | get_volume(L0) / list_devices(L0) / set_volume(L1, mute 0 强制 L3) / mute/unmute(L1) / toggle_mute(L1) |
//! | 键鼠 | Input | 12 | mouse_position(L0) / mouse_move(L2) / click(L2) / double_click(L2) / type_text(L1 ASCII, L2 中文) / press_key(L2) / hotkey(L2) / key_sequence(L2) / mouse_drag(L3) / screenshot(L3) / scroll_wheel(L2) / move_cursor_to_center(L2) |
//! | 网络 | Network | 6 | ping(L0) / dns_lookup(L0) / traffic_usage(L0) / netstat(L0) / disable_iface(L3) / enable_iface(L3) |
//! | 显示 | Display | 5 | list_displays(L0) / set_resolution(L2) / set_brightness(L1) / screenshot_capture_region(L3, Own) / display_on_off(L2) |
//! | 浏览器 | Browser | 5 | open_url(L1) / search_query(L1) / list_tabs(L1, Own) / close_tab(L2) / bookmark_add(L2) |
//! | 通知 | Notify | 5 | toast_notify(L1) / balloon_notify(L1) / set_wallpaper(L2) / flash_taskbar(L2, Win-only) / lock_workstation(L3, Own) |

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::XiaobaiResult;
use crate::identity::OperatorIdentity;
use crate::rbac::ClearanceLevel;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum OperatorCategory {
    App,
    File,
    Volume,
    Input,
    Network,
    Display,
    Browser,
    Notify,
}

impl OperatorCategory {
    pub fn as_str(self) -> &'static str {
        use OperatorCategory::*;
        match self {
            App => "app",
            File => "file",
            Volume => "volume",
            Input => "input",
            Network => "network",
            Display => "display",
            Browser => "browser",
            Notify => "notify",
        }
    }
    pub fn label_zh(self) -> &'static str {
        use OperatorCategory::*;
        match self {
            App => "应用控制",
            File => "文件操作",
            Volume => "音量控制",
            Input => "键鼠输入",
            Network => "网络与代理",
            Display => "显示与分辨率",
            Browser => "浏览器",
            Notify => "系统通知",
        }
    }
}

/// 动作入参：动态 JSON（因为每个算子动作参数形状不同，ActionSignature 给校验器用）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionParam(pub Value);

impl ActionParam {
    pub fn new(v: Value) -> Self {
        Self(v)
    }
    pub fn null() -> Self {
        Self(Value::Null)
    }
    pub fn get_str(&self, k: &str) -> Option<&str> {
        self.0.get(k).and_then(|v| v.as_str())
    }
    pub fn get_i64(&self, k: &str) -> Option<i64> {
        self.0.get(k).and_then(|v| v.as_i64())
    }
    pub fn get_f64(&self, k: &str) -> Option<f64> {
        self.0.get(k).and_then(|v| v.as_f64())
    }
    pub fn get_bool(&self, k: &str) -> Option<bool> {
        self.0.get(k).and_then(|v| v.as_bool())
    }
}

/// 动作输出：给 UI Toast 看的 message + 结构化 payload（如 list_running 的进程列表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorOutput {
    /// 人类可读中文摘要（1 句话，UI Toast 直接展示）
    pub message: String,
    /// 结构化返回（list_running / get_volume / file_exists 等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    /// 实际使用的平台回退链名（如 ["pycaw", "waveOut"]），审计与问题定位用
    pub fallbacks_used: Vec<String>,
    /// 耗时毫秒
    pub elapsed_ms: u64,
}

impl OperatorOutput {
    pub fn quick(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            payload: None,
            fallbacks_used: Vec::new(),
            elapsed_ms: 0,
        }
    }
    pub fn with_payload(mut self, v: Value) -> Self {
        self.payload = Some(v);
        self
    }
    pub fn with_fallbacks(mut self, v: Vec<String>) -> Self {
        self.fallbacks_used = v;
        self
    }
    pub fn with_elapsed(mut self, ms: u64) -> Self {
        self.elapsed_ms = ms;
        self
    }
    pub fn push_fallback(&mut self, name: impl Into<String>) {
        self.fallbacks_used.push(name.into());
    }
}

/// 动作签名（供 OperatorEngine 校验参数、前端自动生成 UI 控件、selftest 参数一致性）
#[derive(Debug, Clone, Serialize)]
pub struct ActionSignature {
    /// 动作名（snake_case，唯一，如 "open_app" / "move_to_trash"）
    pub name: &'static str,
    /// 所属类别
    pub category: OperatorCategory,
    /// 最低需要的 clearance（RBAC 4 级；PII 命中时由 engine 再升一级）
    pub clearance: ClearanceLevel,
    /// 动作是否属于 "Own 语义"：是则 operator.is_owner=true 时 clearance + 1 宽容
    pub own_qualified: bool,
    /// 人类可读一句话说明（中文）
    pub description: &'static str,
    /// 入参校验：JSON schema 的极简 BTreeMap<字段名, 类型>；None 表示不做参数校验
    pub params: Option<BTreeMap<&'static str, &'static str>>,
}

/// 系统算子异步 trait（`async_trait` 给 tokio mox_platform_orchestrator_svc 用）
///
/// 实现方需保证：`execute()` 内部的 Win32 / filesystem / coreaudio 阻塞调用
/// 统一在 `tokio::task::spawn_blocking` 内执行，不要阻塞 async mox_platform_orchestrator_svc 线程。
#[async_trait]
pub trait SystemOperator: Send + Sync {
    /// 算子唯一 ID（snake_case，如 "app_operator_v1"）
    fn id(&self) -> &'static str;
    /// 所属类别
    fn category(&self) -> OperatorCategory;
    /// 返回本算子暴露的所有动作签名（由 Engine 注册到注册表）
    fn list_actions(&self) -> Vec<ActionSignature>;

    /// 返回某个动作的 clearance 要求（默认按 ActionSignature 返回）
    fn clearance_required(&self, action: &str) -> XiaobaiResult<ClearanceLevel> {
        self.list_actions()
            .iter()
            .find(|s| s.name == action)
            .map(|s| s.clearance)
            .ok_or_else(|| crate::XiaobaiError::IntentUnknown(action.into()))
    }

    /// 返回动作是否 own_qualified
    fn own_qualified(&self, action: &str) -> XiaobaiResult<bool> {
        Ok(self
            .list_actions()
            .iter()
            .find(|s| s.name == action)
            .map(|s| s.own_qualified)
            .unwrap_or(false))
    }

    /// 执行动作（Engine 保证：RBAC check 通过才调用）
    async fn execute(
        &self,
        action: &str,
        param: ActionParam,
        identity: &OperatorIdentity,
    ) -> XiaobaiResult<OperatorOutput>;
}

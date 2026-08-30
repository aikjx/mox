// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Notify 算子：系统通知 & 桌面反馈（toast_notify / balloon_notify / set_wallpaper / flash_taskbar / lock_workstation）
//!
//! 跨平台回退链：
//! - toast_notify：Windows（PowerShell BurntToast → msg → PowerShell Add-Type WinForms MessageBox）/ macOS(osascript display notification) / Linux(notify-send)
//! - balloon_notify：等同 toast_notify（现代系统统一 Toast；Windows 若任务栏托盘可用则 BalloonTip via WinForms NotifyIcon 尝试）
//! - set_wallpaper：Windows(SystemParametersInfoW SPI_SETDESKWALLPAPER via windows-rs → PowerShell reg+Win32) / Linux(gsettings) / macOS(osascript)
//! - flash_taskbar：Windows(FlashWindowEx user32 → PowerShell)；macOS/Linux 无对应窗口管理器 API → XB-007
//! - lock_workstation：Windows(rundll32 user32.dll,LockWorkStation) / Linux(loginctl lock-sessions) / macOS(pmset displaysleepnow)

mod common;
mod desktop;
mod toast;
mod wallpaper;

use std::collections::BTreeMap;
use std::time::Instant;

use async_trait::async_trait;

use mox_voice_core_svc::errors::{XiaobaiError, XiaobaiResult};
use mox_voice_core_svc::identity::OperatorIdentity;
use mox_voice_core_svc::operator::{
    ActionParam, ActionSignature, OperatorCategory, OperatorOutput, SystemOperator,
};
use mox_voice_core_svc::rbac::ClearanceLevel;

#[derive(Debug, Default, Clone)]
pub struct NotifyOperator;

#[async_trait]
impl SystemOperator for NotifyOperator {
    fn id(&self) -> &'static str {
        "notify_operator_v1"
    }
    fn category(&self) -> OperatorCategory {
        OperatorCategory::Notify
    }
    fn list_actions(&self) -> Vec<ActionSignature> {
        use ClearanceLevel::*;
        let mut p_toast = BTreeMap::new();
        p_toast.insert("title", "string，通知标题（短，1 行）");
        p_toast.insert("body", "string，通知正文；可选 icon 字符串（暂仅 Windows 预留）");
        let mut p_balloon = BTreeMap::new();
        p_balloon.insert("title", "同 toast_notify");
        p_balloon.insert("body", "同 toast_notify");
        p_balloon.insert("timeout_ms", "int，可选，气泡显示时长毫秒（默认 3000）");
        let mut p_wp = BTreeMap::new();
        p_wp.insert("abs_path", "string，壁纸图片绝对路径（JPG/PNG/BMP，需存在）");
        let mut p_flash = BTreeMap::new();
        p_flash.insert("duration_ms", "int，可选，闪烁时间毫秒（默认 2000，实际由 FlashWindowEx 决定）");
        vec![
            ActionSignature {
                name: "toast_notify",
                category: OperatorCategory::Notify,
                clearance: L1,
                own_qualified: false,
                description: "发送系统通知（BurntToast → msg.exe → WinForms；通知中心 → notify-send → zenity）",
                params: Some(p_toast),
            },
            ActionSignature {
                name: "balloon_notify",
                category: OperatorCategory::Notify,
                clearance: L1,
                own_qualified: false,
                description: "托盘气泡通知（Windows NotifyIcon ShowBalloonTip；非 Windows 等价 toast）",
                params: Some(p_balloon),
            },
            ActionSignature {
                name: "set_wallpaper",
                category: OperatorCategory::Notify,
                clearance: L2,
                own_qualified: false,
                description: "设置桌面壁纸：windows-rs SPI_SETDESKWALLPAPER → PowerShell P/Invoke → gsettings/KDE → System Events Finder",
                params: Some(p_wp),
            },
            ActionSignature {
                name: "flash_taskbar",
                category: OperatorCategory::Notify,
                clearance: L2,
                own_qualified: false,
                description: "Windows 前台窗口任务栏闪烁（FlashWindowEx）；macOS/Linux XB-007",
                params: Some(p_flash),
            },
            ActionSignature {
                name: "lock_workstation",
                category: OperatorCategory::Notify,
                clearance: L3,
                own_qualified: true,
                description: "锁定本机工作站（rundll32 LockWorkStation / loginctl lock / pmset displaysleepnow），L3 破坏性保护；Own 场景 L2 可",
                params: None,
            },
        ]
    }
    async fn execute(
        &self,
        action: &str,
        param: ActionParam,
        _identity: &OperatorIdentity,
    ) -> XiaobaiResult<OperatorOutput> {
        let t0 = Instant::now();
        match action {
            "toast_notify" => {
                let title = param.get_str("title").unwrap_or("小白语音通知");
                let body = param.get_str("body").unwrap_or("（无正文）");
                let icon = param.get_str("icon");
                let (fbs, msg) = self.toast_impl(title, body, icon)?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "balloon_notify" => {
                let title = param.get_str("title").unwrap_or("小白语音托盘提示");
                let body = param.get_str("body").unwrap_or("（无正文）");
                let timeout_ms = param.get_i64("timeout_ms").map(|v| v.clamp(500, 600_000) as u32);
                let (fbs, msg) = self.balloon_impl(title, body, timeout_ms)?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "set_wallpaper" => {
                let path = param.get_str("abs_path").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: "set_wallpaper".into(),
                    param: "abs_path".into(),
                    value: "<missing>".into(),
                    hint: "需要 abs_path 绝对路径".into(),
                })?;
                let (fbs, msg) = self.set_wallpaper_impl(path)?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "flash_taskbar" => {
                let dur = param.get_i64("duration_ms").map(|v| v.clamp(100, 60_000) as u32);
                let (fbs, msg) = self.flash_impl(dur)?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "lock_workstation" => {
                let (fbs, msg) = self.lock_impl()?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            other => Err(XiaobaiError::IntentUnknown(other.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::common::{escape_osa, escape_ps};
    use super::*;

    #[test]
    fn escape_ps_and_osa() {
        assert_eq!(escape_ps("a'b"), "a''b");
        assert_eq!(escape_osa(r#"say "hello\"#), r#"say \"hello\\"#);
    }

    #[test]
    fn list_actions_5_covered() {
        let op = NotifyOperator::default();
        let acts = op.list_actions();
        assert_eq!(acts.len(), 5);
        let names: Vec<_> = acts.iter().map(|a| a.name).collect();
        for n in ["toast_notify", "balloon_notify", "set_wallpaper", "flash_taskbar", "lock_workstation"] {
            assert!(names.contains(&n), "missing {n}");
        }
        // 锁屏：L3，Own=true
        let lock = acts.iter().find(|a| a.name == "lock_workstation").unwrap();
        assert_eq!(lock.clearance, ClearanceLevel::L3);
        assert!(lock.own_qualified);
    }
}

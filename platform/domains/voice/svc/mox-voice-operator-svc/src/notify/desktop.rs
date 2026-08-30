// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 桌面操作：任务栏闪烁 + 锁屏

use crate::helpers::{platform_tag, run_command, run_command_xb};
use mox_voice_core_svc::errors::{XiaobaiError, XiaobaiResult};
use mox_voice_core_svc::operator::OperatorCategory;

use super::NotifyOperator;

impl NotifyOperator {
    // ============ flash_taskbar ============
    pub(crate) fn flash_impl(&self, duration_ms: Option<u32>) -> XiaobaiResult<(Vec<&'static str>, String)> {
        let mut fbs = Vec::new();
        if !cfg!(windows) {
            return Err(XiaobaiError::OperatorUnsupported {
                category: OperatorCategory::Notify.as_str().to_string(),
                action: "flash_taskbar".into(),
                platform: platform_tag(),
                fallbacks_used: vec!["only_windows_supported".into()],
            });
        }
        // Windows：PowerShell FlashWindowEx
        fbs.push("powershell_flashwindowex");
        let ms = duration_ms.unwrap_or(2000).to_string();
        let ps = format!(
            r#"
$code = @'
using System;
using System.Runtime.InteropServices;
public class FX {{
    [StructLayout(LayoutKind.Sequential)] public struct FLASHWINFO {{
        public int cbSize; public IntPtr hwnd; public int dwFlags; public int uCount; public int dwTimeout;
    }}
    [DllImport("user32.dll")] public static extern bool FlashWindowEx(ref FLASHWINFO pfwi);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}}
'@
Add-Type $code;
$fwi = New-Object FX+FLASHWINFO;
$fwi.cbSize = [Runtime.InteropServices.Marshal]::SizeOf($fwi);
$fwi.hwnd = [FX]::GetForegroundWindow();
$fwi.dwFlags = 0x0000000C -bor 0x00000004; # FLASHW_ALL -bor FLASHW_TIMERNOFG
$fwi.uCount = 5;
$fwi.dwTimeout = 0;
[void][FX]::FlashWindowEx([ref]$fwi);
Start-Sleep -Milliseconds {ms};
"#
        );
        let (_, _, code) = run_command("powershell", &["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps])
            .map_err(|e| XiaobaiError::ExecutionError {
                category: OperatorCategory::Notify.as_str().into(),
                action: "flash_taskbar".into(),
                detail: format!("flash ps script failed: {e}"),
            })?;
        if code != 0 {
            return Err(XiaobaiError::ExecutionError {
                category: OperatorCategory::Notify.as_str().into(),
                action: "flash_taskbar".into(),
                detail: "flash taskbar exit != 0".into(),
            });
        }
        Ok((fbs, format!("前台窗口任务栏闪烁 {ms}ms（FLASHW_ALL until active）")))
    }

    // ============ lock_workstation ============
    pub(crate) fn lock_impl(&self) -> XiaobaiResult<(Vec<&'static str>, String)> {
        let mut fbs = Vec::new();
        let msg = if cfg!(windows) {
            fbs.push("rundll32_user32_lockworkstation");
            let (_, _, code) = run_command_xb("rundll32", &["user32.dll,LockWorkStation"], OperatorCategory::Notify, "lock_workstation")?;
            if code != 0 {
                // 回退：Logoff 类脚本不推荐；直接失败
                return Err(XiaobaiError::ExecutionError {
                    category: OperatorCategory::Notify.as_str().into(),
                    action: "lock_workstation".into(),
                    detail: "rundll32 LockWorkStation exit != 0".into(),
                });
            }
            "已调用 LockWorkStation 锁定桌面（需恢复需重新登录）".into()
        } else if cfg!(target_os = "linux") {
            fbs.push("loginctl_lock_sessions");
            let r = run_command("loginctl", &["lock-sessions"]);
            if let Ok((_, _, 0)) = r {
                "loginctl 已锁所有 session".into()
            } else {
                fbs.push("xdg_screensaver_lock");
                let r2 = run_command("xdg-screensaver", &["lock"]);
                if let Ok((_, _, 0)) = r2 {
                    "xdg-screensaver lock 已触发".into()
                } else {
                    return Err(XiaobaiError::OperatorUnsupported {
                        category: OperatorCategory::Notify.as_str().to_string(),
                        action: "lock_workstation".into(),
                        platform: platform_tag(),
                        fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                    });
                }
            }
        } else {
            // macOS：pmset displaysleepnow（如果已启用 "唤醒需密码" 即为锁屏）
            fbs.push("pmset_displaysleepnow");
            let (_, _, code) = run_command_xb("pmset", &["displaysleepnow"], OperatorCategory::Notify, "lock_workstation")?;
            if code != 0 {
                return Err(XiaobaiError::ExecutionError {
                    category: OperatorCategory::Notify.as_str().into(),
                    action: "lock_workstation".into(),
                    detail: "pmset displaysleepnow exit != 0".into(),
                });
            }
            "macOS displaysleepnow 已触发（若已勾选唤醒需要密码，即完成锁屏）".into()
        };
        Ok((fbs, msg))
    }
}

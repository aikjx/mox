// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Toast / Balloon 通知实现

use crate::helpers::{platform_tag, run_command, run_command_xb};
use mox_voice_core_svc::errors::{XiaobaiError, XiaobaiResult};
use mox_voice_core_svc::operator::OperatorCategory;

use super::common::{escape_osa, escape_ps};
use super::NotifyOperator;

impl NotifyOperator {
    // ============ toast_notify ============
    pub(crate) fn toast_impl(&self, title: &str, body: &str, _icon: Option<&str>) -> XiaobaiResult<(Vec<&'static str>, String)> {
        let mut fbs = Vec::new();
        let msg = if cfg!(windows) {
            // 回退 1：BurntToast（PowerShell Gallery 模块，已预装企业/家用不同）
            fbs.push("powershell_burnttoast");
            let ps_cmd = format!(
                "$BT = '{{\\n    \"title\" = \"{t}\"\\n    \"text\" = \"{b}\"\\n}}' ; try {{ New-BurntToastNotification -Text '{t}','{b}' -ErrorAction Stop }} catch {{ exit 1 }}",
                t = escape_ps(title),
                b = escape_ps(body),
            );
            let r = run_command("powershell", &["-NoProfile", "-Command", &ps_cmd]);
            if let Ok((_, _, 0)) = r {
                "BurntToast 通知已发送".into()
            } else {
                // 回退 2：msg.exe（所有 Win10/11 自带，弹窗式）
                fbs.push("msg_exe_star");
                let r2 = run_command("msg", &["*", "/TIME:5", &format!("{title}\n{body}")]);
                if let Ok((_, _, 0)) = r2 {
                    "msg.exe 弹窗通知已发送（5 秒自动关闭）".into()
                } else {
                    // 回退 3：WinForms MessageBox（模态，用户手动关闭，最后兜底）
                    fbs.push("winforms_messagebox");
                    let ps3 = format!(
                        "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('{b}','{t}') | Out-Null",
                        t = escape_ps(title),
                        b = escape_ps(body),
                    );
                    let r3 = run_command("powershell", &["-NoProfile", "-Command", &ps3]);
                    if let Ok((_, _, 0)) = r3 {
                        "WinForms MessageBox 已弹出".into()
                    } else {
                        return Err(XiaobaiError::OperatorUnsupported {
                            category: OperatorCategory::Notify.as_str().to_string(),
                            action: "toast_notify".into(),
                            platform: platform_tag(),
                            fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                        });
                    }
                }
            }
        } else if cfg!(target_os = "macos") {
            fbs.push("osascript_display_notification");
            let script = format!(
                "display notification \"{}\" with title \"{}\"",
                escape_osa(body),
                escape_osa(title),
            );
            let (_, _, code) = run_command_xb("osascript", &["-e", &script], OperatorCategory::Notify, "toast_notify")?;
            if code != 0 {
                return Err(XiaobaiError::ExecutionError {
                    category: OperatorCategory::Notify.as_str().into(),
                    action: "toast_notify".into(),
                    detail: "osascript notification exit != 0".into(),
                });
            }
            "macOS 通知中心通知已发送".into()
        } else {
            fbs.push("notify_send");
            let r = run_command("notify-send", &[title, body]);
            if let Ok((_, _, 0)) = r {
                "notify-send 桌面通知已发送".into()
            } else {
                fbs.push("zenity_info");
                let r2 = run_command("zenity", &["--info", "--title", title, "--text", body]);
                if let Ok((_, _, 0)) = r2 {
                    "zenity 信息弹窗已显示".into()
                } else {
                    return Err(XiaobaiError::OperatorUnsupported {
                        category: OperatorCategory::Notify.as_str().to_string(),
                        action: "toast_notify".into(),
                        platform: platform_tag(),
                        fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                    });
                }
            }
        };
        Ok((fbs, msg))
    }

    // ============ balloon_notify ============
    pub(crate) fn balloon_impl(&self, title: &str, body: &str, timeout_ms: Option<u32>) -> XiaobaiResult<(Vec<&'static str>, String)> {
        if cfg!(windows) {
            // 优先 WinForms NotifyIcon ShowBalloonTip（需要 PowerShell STA 模式）
            let mut fbs = Vec::new();
            fbs.push("powershell_sta_notifyicon_balloon");
            let tm = timeout_ms.unwrap_or(3000).to_string();
            let ps = format!(
                "Add-Type -AssemblyName System.Windows.Forms; $n=New-Object System.Windows.Forms.NotifyIcon; $n.Icon=[System.Drawing.SystemIcons]::Information; $n.Visible=$true; $n.ShowBalloonTip({},'{t}','{b}',[System.Windows.Forms.ToolTipIcon]::Info); Start-Sleep -Milliseconds {}; $n.Dispose()",
                tm,
                (timeout_ms.unwrap_or(3000) + 500).to_string(),
                t = escape_ps(title),
                b = escape_ps(body),
            );
            let r = run_command("powershell", &["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", &ps]);
            if let Ok((_, _, 0)) = r {
                return Ok((fbs, format!("托盘气泡通知已弹出 {}ms", tm)));
            }
            // 回退：复用 toast 链路
            let (fbs2, msg) = self.toast_impl(title, body, None)?;
            fbs.push("toast_fallback");
            fbs.append(&mut fbs2.iter().map(|s| Box::leak((*s).to_owned().into_boxed_str()) as &str).collect());
            Ok((fbs, format!("balloon 托盘不可用，已降级为 Toast：{msg}")))
        } else {
            // 非 Windows 系统没有"托盘气球"概念，全部走 toast 语义等价
            let (fbs, msg) = self.toast_impl(title, body, None)?;
            Ok((fbs, format!("balloon 已等价 toast：{msg}")))
        }
    }
}

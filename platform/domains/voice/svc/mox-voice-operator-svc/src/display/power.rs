// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 显示器电源：display_on_off

use crate::helpers::{platform_tag, run_command, run_command_xb};
use mox_voice_core_svc::errors::{XiaobaiError, XiaobaiResult};
use mox_voice_core_svc::operator::OperatorCategory;

use super::DisplayOperator;

impl DisplayOperator {
    // ============ display_on_off ============
    pub(crate) fn display_power_impl(&self, on: bool) -> XiaobaiResult<(Vec<&'static str>, String)> {
        let mut fbs = Vec::new();
        let msg = match () {
            _ if cfg!(windows) => {
                let sub = if on { "async on" } else { "async off" };
                fbs.push("nircmd_monitor_async");
                let r = run_command("nircmd", &["monitor", sub]);
                if let Ok((_, _, 0)) = r {
                    format!("已执行显示器 {on}（nircmd）")
                } else {
                    fbs.push("nircmdc_monitor_async");
                    let r2 = run_command("nircmdc", &["monitor", sub]);
                    if let Ok((_, _, 0)) = r2 {
                        format!("已执行显示器 {on}（nircmdc）")
                    } else if !on {
                        // 最后兜底：SendMessage SC_MONITORPOWER 2
                        fbs.push("powershell_addtype_monitorpower");
                        let ps = r#"
Add-Type '[DllImport("user32.dll")]public static extern int SendMessage(int hWnd,int hMsg,int wParam,int lParam);' -Name u -Na sp;
$hwnd=(New-Object -ComObject WScript.Shell).SendKeys("% n");
[sp.u]::SendMessage(0xffff,0x0112,0xF170,2) | Out-Null
"#;
                        let r3 = run_command("powershell", &["-NoProfile", "-Command", ps.trim()]);
                        if let Ok((_, _, 0)) = r3 {
                            "已通过 user32 SendMessage 关闭显示器".into()
                        } else {
                            return Err(XiaobaiError::OperatorUnsupported {
                                category: OperatorCategory::Display.as_str().to_string(),
                                action: "display_on_off".into(),
                                platform: platform_tag(),
                                fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                            });
                        }
                    } else {
                        return Err(XiaobaiError::OperatorUnsupported {
                            category: OperatorCategory::Display.as_str().to_string(),
                            action: "display_on_off".into(),
                            platform: platform_tag(),
                            fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                        });
                    }
                }
            }
            _ if cfg!(target_os = "linux") => {
                if on {
                    fbs.push("xset_dpms_force_on");
                    let _ = run_command("xset", &["dpms", "force", "on"]);
                    let _ = run_command("xset", &["s", "reset"]);
                    "已发送 xset dpms force on".into()
                } else {
                    fbs.push("xset_dpms_force_off");
                    let (_, _, code) = run_command_xb("xset", &["dpms", "force", "off"], OperatorCategory::Display, "display_on_off")?;
                    if code != 0 {
                        return Err(XiaobaiError::OperatorUnsupported {
                            category: OperatorCategory::Display.as_str().to_string(),
                            action: "display_on_off".into(),
                            platform: platform_tag(),
                            fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                        });
                    }
                    "xset 已关闭显示输出".into()
                }
            }
            _ => {
                // macOS：on 没可靠 CLI（直接 caffeinate 或点击）；off 走 displaysleepnow
                if on {
                    return Err(XiaobaiError::OperatorUnsupported {
                        category: OperatorCategory::Display.as_str().to_string(),
                        action: "display_on_off".into(),
                        platform: platform_tag(),
                        fallbacks_used: vec!["macos_on_not_supported_cli".into()],
                    });
                }
                fbs.push("pmset_displaysleepnow");
                let (_, _, code) = run_command_xb("pmset", &["displaysleepnow"], OperatorCategory::Display, "display_on_off")?;
                if code != 0 {
                    return Err(XiaobaiError::OperatorUnsupported {
                        category: OperatorCategory::Display.as_str().to_string(),
                        action: "display_on_off".into(),
                        platform: platform_tag(),
                        fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                    });
                }
                "pmset displaysleepnow 已触发".into()
            }
        };
        Ok((fbs, msg))
    }
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 亮度控制：set_brightness

use crate::helpers::{platform_tag, run_command};
use mox_voice_core_svc::errors::{XiaobaiError, XiaobaiResult};
use mox_voice_core_svc::operator::OperatorCategory;

use super::DisplayOperator;

impl DisplayOperator {
    // ============ set_brightness ============
    pub(crate) fn set_brightness_impl(&self, percent: u32) -> XiaobaiResult<(Vec<&'static str>, String)> {
        let mut fbs = Vec::new();
        let pct = (percent.clamp(0, 100) as f32 / 100.0).to_string();
        let msg = match () {
            _ if cfg!(windows) => {
                // 回退 1：PowerCfg（笔记本/可调节亮度面板）
                fbs.push("powercfg_setacvalueindex_brightness");
                let scheme = run_command("powercfg", &["/getactivescheme"]).ok().and_then(|(so, _, _)| {
                    so.split_whitespace().find(|t| t.contains('-')).map(|s| s.trim_matches(':').to_string())
                });
                if let Some(sch) = scheme {
                    for (idx, mode) in ["/SETACVALUEINDEX", "/SETDCVALUEINDEX"].iter().enumerate() {
                        let _ = run_command(
                            "powercfg",
                            &[mode, &sch, "SUB_VIDEO", "ADJUSTBRIGHTNESS", &percent.to_string()],
                        );
                        if idx == 1 {
                            let _ = run_command("powercfg", &["/S", &sch]);
                        }
                    }
                    format!("PowerCfg 亮度已设置为 {percent}%（交流+直流）")
                } else {
                    fbs.push("wmi_wmimonitorbrightness");
                    let ps = format!(
                        "(Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods).WmiSetBrightness(1, {percent})"
                    );
                    let r = run_command("powershell", &["-NoProfile", "-Command", &ps]);
                    if let Ok((_, _, 0)) = r {
                        format!("WMI 亮度已设置为 {percent}%")
                    } else {
                        return Err(XiaobaiError::OperatorUnsupported {
                            category: OperatorCategory::Display.as_str().to_string(),
                            action: "set_brightness".into(),
                            platform: platform_tag(),
                            fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                        });
                    }
                }
            }
            _ if cfg!(target_os = "linux") => {
                fbs.push("xbacklight_set_percent");
                let p = percent.to_string();
                let r = run_command("xbacklight", &["-set", &p]);
                if let Ok((_, _, 0)) = r {
                    format!("xbacklight 亮度：{p}%")
                } else {
                    fbs.push("brightnessctl_s_p");
                    let set_arg = format!("{p}%");
                    let r2 = run_command("brightnessctl", &["s", &set_arg]);
                    if let Ok((_, _, 0)) = r2 {
                        format!("brightnessctl：{set_arg}")
                    } else {
                        return Err(XiaobaiError::OperatorUnsupported {
                            category: OperatorCategory::Display.as_str().to_string(),
                            action: "set_brightness".into(),
                            platform: platform_tag(),
                            fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                        });
                    }
                }
            }
            _ => {
                // macOS：brightness CLI（需 brew）或 osascript 模拟按键
                fbs.push("brightness_cli");
                let r = run_command("brightness", &[&pct]);
                if let Ok((_, _, 0)) = r {
                    format!("brightness：{pct}")
                } else {
                    return Err(XiaobaiError::OperatorUnsupported {
                        category: OperatorCategory::Display.as_str().to_string(),
                        action: "set_brightness".into(),
                        platform: platform_tag(),
                        fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                    });
                }
            }
        };
        Ok((fbs, msg))
    }
}

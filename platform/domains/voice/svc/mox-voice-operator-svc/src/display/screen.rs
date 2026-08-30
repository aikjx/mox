// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 屏幕信息与分辨率：list_displays / set_resolution

use std::collections::BTreeMap;

use screenshots::Screen;
use serde_json::{json, Value};

use crate::helpers::{platform_tag, run_command};
use mox_voice_core_svc::errors::{XiaobaiError, XiaobaiResult};
use mox_voice_core_svc::operator::OperatorCategory;

use super::DisplayOperator;

impl DisplayOperator {
    // ============ list_displays ============
    pub(crate) fn list_displays_impl(&self) -> XiaobaiResult<(Vec<&'static str>, Vec<BTreeMap<String, Value>>)> {
        let mut fbs = Vec::new();
        // 回退 1：screenshots Screen::all
        let screens = Screen::all().map_err(|_e| XiaobaiError::OperatorUnsupported {
            category: OperatorCategory::Display.as_str().to_string(),
            action: "list_displays".into(),
            platform: platform_tag(),
            fallbacks_used: vec!["screen_crate_all".into()],
        })?;
        fbs.push("screenshots_screen_all");
        let mut rows = Vec::new();
        for (i, s) in screens.iter().enumerate() {
            let mut r = BTreeMap::new();
            r.insert("index".into(), json!(i));
            r.insert("id".into(), json!(s.display_info.id.to_string()));
            let (w, h) = (s.display_info.width, s.display_info.height);
            r.insert("width".into(), json!(w as i64));
            r.insert("height".into(), json!(h as i64));
            let (x, y) = (s.display_info.x, s.display_info.y);
            r.insert("x".into(), json!(x as i64));
            r.insert("y".into(), json!(y as i64));
            rows.push(r);
        }
        Ok((fbs, rows))
    }

    // ============ set_resolution ============
    pub(crate) fn set_resolution_impl(&self, display: Option<&str>, w: i64, h: i64) -> XiaobaiResult<(Vec<&'static str>, String)> {
        let mut fbs = Vec::new();
        let ws = w.to_string();
        let hs = h.to_string();
        let msg = match () {
            _ if cfg!(windows) => {
                fbs.push("nircmd_setdisplay");
                let args = match display {
                    Some(name) => vec!["setdisplay", name, &ws, &hs, "32"],
                    None => vec!["setdisplay", &ws, &hs, "32"],
                };
                let r1 = run_command("nircmd", &args);
                if let Ok((_, _, 0)) = r1 {
                    format!("已设置分辨率 {w}x{h}（nircmd）")
                } else {
                    fbs.push("nircmdc_setdisplay");
                    let r2 = run_command("nircmdc", &args);
                    if let Ok((_, _, 0)) = r2 {
                        format!("已设置分辨率 {w}x{h}（nircmdc）")
                    } else {
                        return Err(XiaobaiError::OperatorUnsupported {
                            category: OperatorCategory::Display.as_str().to_string(),
                            action: "set_resolution".into(),
                            platform: platform_tag(),
                            fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                        });
                    }
                }
            }
            _ if cfg!(target_os = "linux") => {
                let output = display.unwrap_or("eDP-1");
                fbs.push("xrandr_output_mode");
                let mode = format!("{w}x{h}");
                let (_, _, code) = run_command("xrandr", &["--output", output, "--mode", &mode])
                    .map_err(|e| XiaobaiError::ExecutionError {
                        category: OperatorCategory::Display.as_str().into(),
                        action: "set_resolution".into(),
                        detail: format!("xrandr failed: {e}"),
                    })?;
                if code != 0 {
                    return Err(XiaobaiError::OperatorUnsupported {
                        category: OperatorCategory::Display.as_str().to_string(),
                        action: "set_resolution".into(),
                        platform: platform_tag(),
                        fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                    });
                }
                format!("xrandr 已设置 {output} = {mode}")
            }
            _ if cfg!(target_os = "macos") => {
                fbs.push("displayplacer");
                let id = display.unwrap_or("main");
                let mode = format!("id:{id} res:{w}x{h}");
                let (_, _, code) = run_command("displayplacer", &[&mode]).map_err(|e| XiaobaiError::ExecutionError {
                    category: OperatorCategory::Display.as_str().into(),
                    action: "set_resolution".into(),
                    detail: format!("displayplacer failed: {e}"),
                })?;
                if code != 0 {
                    return Err(XiaobaiError::OperatorUnsupported {
                        category: OperatorCategory::Display.as_str().to_string(),
                        action: "set_resolution".into(),
                        platform: platform_tag(),
                        fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                    });
                }
                format!("displayplacer 已设置：{mode}")
            }
            _ => {
                return Err(XiaobaiError::OperatorUnsupported {
                    category: OperatorCategory::Display.as_str().to_string(),
                    action: "set_resolution".into(),
                    platform: platform_tag(),
                    fallbacks_used: vec!["unsupported_os".into()],
                });
            }
        };
        Ok((fbs, msg))
    }
}

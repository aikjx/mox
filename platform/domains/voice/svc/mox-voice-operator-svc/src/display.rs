// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Display 算子：显示与分辨率（list_displays / set_resolution / set_brightness / screenshot_capture_region / display_on_off）
//!
//! 跨平台回退链：
//! - list_displays：直接复用 screenshots crate 的 Screen::all() 提供尺寸/位置；若不可用 → Windows wmic / Linux xrandr / macOS system_profiler
//! - set_resolution：Windows(nircmd setdisplay) / Linux(xrandr --output --mode) / macOS(displayplacer)；失败统一 XB-007
//! - set_brightness：Windows(PowerCfg brightness) / Linux(xbacklight) / macOS(brightness CLI)
//! - screenshot_capture_region：screenshots crate → image::crop_imm（非主屏幕需 x/y 偏移）
//! - display_on_off：Windows(nircmd monitor async) / Linux(xset dpms force) / macOS(pmset displaysleepnow 仅 off)

use std::collections::BTreeMap;
use std::time::Instant;

use async_trait::async_trait;
use image::ImageFormat;
use screenshots::Screen;
use serde_json::{json, Value};

use crate::helpers::{platform_tag, run_command, run_command_xb};
use mox_voice_core_svc::errors::{XiaobaiError, XiaobaiResult};
use mox_voice_core_svc::identity::OperatorIdentity;
use mox_voice_core_svc::operator::{
    ActionParam, ActionSignature, OperatorCategory, OperatorOutput, SystemOperator,
};
use mox_voice_core_svc::rbac::ClearanceLevel;

#[derive(Debug, Default, Clone)]
pub struct DisplayOperator;

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

    // ============ screenshot_capture_region ============
    pub(crate) fn capture_region_impl(
        &self,
        x: i64,
        y: i64,
        w: i64,
        h: i64,
    ) -> XiaobaiResult<(Vec<&'static str>, String)> {
        let mut fbs = Vec::new();
        if w <= 0 || h <= 0 {
            return Err(XiaobaiError::InvalidArgument {
                action: "screenshot_capture_region".into(),
                param: "w/h".into(),
                value: format!("{w},{h}"),
                hint: "w 和 h 必须是正整数".into(),
            });
        }
        let screens = Screen::all().map_err(|e| XiaobaiError::ExecutionError {
            category: OperatorCategory::Display.as_str().into(),
            action: "screenshot_capture_region".into(),
            detail: format!("Screen::all failed: {e}"),
        })?;
        // 找到覆盖 (x,y) 的屏幕；找不到用主屏 0
        let screen_idx = screens
            .iter()
            .position(|s| {
                let (sx, sy) = (s.display_info.x, s.display_info.y);
                let (sw, sh) = (s.display_info.width, s.display_info.height);
                let sx = sx as i64;
                let sy = sy as i64;
                let sw = sw as i64;
                let sh = sh as i64;
                x >= sx && y >= sy && x < sx + sw && y < sy + sh
            })
            .unwrap_or(0);
        let screen = screens.get(screen_idx).ok_or_else(|| XiaobaiError::ExecutionError {
            category: OperatorCategory::Display.as_str().into(),
            action: "screenshot_capture_region".into(),
            detail: format!("屏幕索引越界 idx={screen_idx} total={}", screens.len()),
        })?;
        let (sx, sy) = (screen.display_info.x as i64, screen.display_info.y as i64);
        let shot = screen.capture().map_err(|e| XiaobaiError::ExecutionError {
            category: OperatorCategory::Display.as_str().into(),
            action: "screenshot_capture_region".into(),
            detail: format!("Screen::capture failed: {e}"),
        })?;
        fbs.push("screenshots_capture_crop");
        // 计算相对屏幕坐标
        let rel_x = (x - sx as i64).clamp(0, shot.width() as i64 - 1) as u32;
        let rel_y = (y - sy as i64).clamp(0, shot.height() as i64 - 1) as u32;
        let cw = (w as u32).min(shot.width().saturating_sub(rel_x));
        let ch = (h as u32).min(shot.height().saturating_sub(rel_y));
        if cw == 0 || ch == 0 {
            return Err(XiaobaiError::InvalidArgument {
                action: "screenshot_capture_region".into(),
                param: "rect".into(),
                value: format!("x={x} y={y} w={w} h={h} 超出屏幕范围"),
                hint: "请传入屏幕内的区域".into(),
            });
        }
        // 手动像素级裁剪（兼容 screenshots 依赖的 image 0.25 与我们直引的 image 0.24 跨版本差）
        let mut cropped = image::RgbaImage::new(cw, ch);
        for y2 in 0..ch {
            for x2 in 0..cw {
                let px_25 = shot.get_pixel(rel_x + x2, rel_y + y2);
                let bytes: [u8; 4] = px_25.0;
                cropped.put_pixel(x2, y2, image::Rgba(bytes));
            }
        }
        // 临时文件保存 PNG
        let tmp = std::env::temp_dir().join(format!("xiaobai_region_{}.png", std::process::id()));
        let mut f = std::fs::File::create(&tmp).map_err(|e| XiaobaiError::ExecutionError {
            category: OperatorCategory::Display.as_str().into(),
            action: "screenshot_capture_region".into(),
            detail: format!("create tmp png failed: {e}"),
        })?;
        cropped.write_to(&mut f, ImageFormat::Png).map_err(|e| XiaobaiError::ExecutionError {
            category: OperatorCategory::Display.as_str().into(),
            action: "screenshot_capture_region".into(),
            detail: format!("image png encode failed: {e}"),
        })?;
        drop(f);
        let path = tmp.to_string_lossy().to_string();
        Ok((fbs, path))
    }

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

#[async_trait]
impl SystemOperator for DisplayOperator {
    fn id(&self) -> &'static str {
        "display_operator_v1"
    }
    fn category(&self) -> OperatorCategory {
        OperatorCategory::Display
    }
    fn list_actions(&self) -> Vec<ActionSignature> {
        use ClearanceLevel::*;
        let mut p_res = BTreeMap::new();
        p_res.insert("width", "int，像素宽，如 1920");
        p_res.insert("height", "int，像素高，如 1080");
        p_res.insert("display", "string，可选，输出标识（xrandr output / displayplacer id / nircmd 屏幕号）");
        let mut p_bright = BTreeMap::new();
        p_bright.insert("percent", "int，0-100 亮度百分比");
        let mut p_region = BTreeMap::new();
        p_region.insert("x", "int，区域左上 X 坐标（虚拟屏坐标系）");
        p_region.insert("y", "int，区域左上 Y 坐标");
        p_region.insert("w", "int，区域宽（px）");
        p_region.insert("h", "int，区域高（px）");
        let mut p_power = BTreeMap::new();
        p_power.insert("on", "bool，true=点亮，false=熄灭；笔记本盒盖/息屏可能由电源策略覆盖");
        vec![
            ActionSignature {
                name: "list_displays",
                category: OperatorCategory::Display,
                clearance: L0,
                own_qualified: false,
                description: "列所有显示输出：index/id/width/height/x/y（screenshots crate 提供）",
                params: None,
            },
            ActionSignature {
                name: "set_resolution",
                category: OperatorCategory::Display,
                clearance: L2,
                own_qualified: false,
                description: "设置指定输出分辨率（nircmd/xrandr/displayplacer），失败回 XB-007",
                params: Some(p_res),
            },
            ActionSignature {
                name: "set_brightness",
                category: OperatorCategory::Display,
                clearance: L1,
                own_qualified: false,
                description: "设置主屏幕亮度（PowerCfg/WMI → xbacklight → brightnessctl → brightness）",
                params: Some(p_bright),
            },
            ActionSignature {
                name: "screenshot_capture_region",
                category: OperatorCategory::Display,
                clearance: L3,
                own_qualified: true,
                description: "截取指定屏幕区域并保存到临时 PNG 路径（L3：Own 场景 L2 可），可用于截图后送入多模态模型",
                params: Some(p_region),
            },
            ActionSignature {
                name: "display_on_off",
                category: OperatorCategory::Display,
                clearance: L2,
                own_qualified: false,
                description: "点亮/熄灭显示器（nircmd monitor async / xset dpms / pmset displaysleepnow）",
                params: Some(p_power),
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
            "list_displays" => {
                let (fbs, rows) = self.list_displays_impl()?;
                Ok(OperatorOutput::quick(format!("检测到 {} 个显示输出", rows.len()))
                    .with_payload(json!(rows))
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "set_resolution" => {
                let w = param.get_i64("width").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: "set_resolution".into(),
                    param: "width".into(),
                    value: "<missing>".into(),
                    hint: "需要 width 整数".into(),
                })?;
                let h = param.get_i64("height").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: "set_resolution".into(),
                    param: "height".into(),
                    value: "<missing>".into(),
                    hint: "需要 height 整数".into(),
                })?;
                let display = param.get_str("display");
                let (fbs, msg) = self.set_resolution_impl(display, w, h)?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "set_brightness" => {
                let percent = param.get_i64("percent").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: "set_brightness".into(),
                    param: "percent".into(),
                    value: "<missing>".into(),
                    hint: "需要 percent 整数 0-100".into(),
                })? as u32;
                let (fbs, msg) = self.set_brightness_impl(percent)?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "screenshot_capture_region" => {
                let x = param.get_i64("x").unwrap_or(0);
                let y = param.get_i64("y").unwrap_or(0);
                let w = param.get_i64("w").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: "screenshot_capture_region".into(),
                    param: "w".into(),
                    value: "<missing>".into(),
                    hint: "需要 w 宽度整数".into(),
                })?;
                let h = param.get_i64("h").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: "screenshot_capture_region".into(),
                    param: "h".into(),
                    value: "<missing>".into(),
                    hint: "需要 h 高度整数".into(),
                })?;
                let (fbs, path) = self.capture_region_impl(x, y, w, h)?;
                Ok(OperatorOutput::quick(format!("区域截图已保存：{path}"))
                    .with_payload(json!({"path": path, "rect": {"x":x,"y":y,"w":w,"h":h}}))
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "display_on_off" => {
                let on = param.get_bool("on").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: "display_on_off".into(),
                    param: "on".into(),
                    value: "<missing>".into(),
                    hint: "需要 on 布尔值 true/false".into(),
                })?;
                let (fbs, msg) = self.display_power_impl(on)?;
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
    use super::*;

    #[test]
    fn list_actions_5_covered() {
        let op = DisplayOperator::default();
        let acts = op.list_actions();
        assert_eq!(acts.len(), 5);
        let names: Vec<_> = acts.iter().map(|a| a.name).collect();
        for n in ["list_displays", "set_resolution", "set_brightness", "screenshot_capture_region", "display_on_off"] {
            assert!(names.contains(&n), "missing {n}");
        }
        let cap = acts.iter().find(|a| a.name == "screenshot_capture_region").unwrap();
        assert_eq!(cap.clearance, ClearanceLevel::L3, "截图属 L3，Own 场景 L2 可");
        assert!(cap.own_qualified);
    }
}

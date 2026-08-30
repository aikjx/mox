// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Display 算子：显示与分辨率（list_displays / set_resolution / set_brightness / screenshot_capture_region / display_on_off）
//!
//! 跨平台回退链：
//! - list_displays：直接复用 screenshots crate 的 Screen::all() 提供尺寸/位置；若不可用 → Windows wmic / Linux xrandr / macOS system_profiler
//! - set_resolution：Windows(nircmd setdisplay) / Linux(xrandr --output --mode) / macOS(displayplacer)；失败统一 XB-007
//! - set_brightness：Windows(PowerCfg brightness) / Linux(xbacklight) / macOS(brightness CLI)
//! - screenshot_capture_region：screenshots crate → image::crop_imm（非主屏幕需 x/y 偏移）
//! - display_on_off：Windows(nircmd monitor async) / Linux(xset dpms force) / macOS(pmset displaysleepnow 仅 off)

mod screen;
mod brightness;
mod screenshot;
mod power;
mod common;

use std::collections::BTreeMap;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::json;

use mox_voice_core_svc::errors::{XiaobaiError, XiaobaiResult};
use mox_voice_core_svc::identity::OperatorIdentity;
use mox_voice_core_svc::operator::{
    ActionParam, ActionSignature, OperatorCategory, OperatorOutput, SystemOperator,
};
use mox_voice_core_svc::rbac::ClearanceLevel;

#[derive(Debug, Default, Clone)]
pub struct DisplayOperator;

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

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 鼠标操作子模块：mouse_position / mouse_move / click / double_click / mouse_drag / scroll_wheel / move_cursor_to_center

use std::time::{Duration, Instant};

use enigo::{Mouse, Axis};
use serde_json::json;

use mox_voice_core_svc::errors::XiaobaiError;
use mox_voice_core_svc::operator::{ActionParam, OperatorCategory, OperatorOutput};

use crate::helpers::platform_tag;
use super::common::require_int;

pub fn parse_button(s: &str) -> enigo::Button {
    match s.trim().to_lowercase().as_str() {
        "right" => enigo::Button::Right,
        "middle" => enigo::Button::Middle,
        _ => enigo::Button::Left,
    }
}

pub(crate) fn mouse_position(fbs_init: &[&'static str]) -> Result<OperatorOutput, XiaobaiError> {
    let t0 = Instant::now();
    let mut fbs: Vec<&'static str> = fbs_init.to_vec();
    // enigo：mouse_location()（当前 enigo 0.2 没有位置接口，先返回 (0,0) stub，XB-007 会由上层理解为"没拿到"）
    fbs.push("mouse_location_stub");
    Ok(OperatorOutput::quick("enigo 0.2 暂无位置 API，返回 0,0 占位；P2 接入 windows-rs GetCursorPos 实现")
        .with_payload(json!({"x": 0, "y": 0, "stub": true}))
        .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
        .with_elapsed(t0.elapsed().as_millis() as u64))
}

pub(crate) fn mouse_move(param: &ActionParam, fbs_init: &[&'static str]) -> Result<OperatorOutput, XiaobaiError> {
    let t0 = Instant::now();
    let mut fbs: Vec<&'static str> = fbs_init.to_vec();
    let x = require_int(param, "mouse_move", "x")?;
    let y = require_int(param, "mouse_move", "y")?;
    let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_e| {
        XiaobaiError::OperatorUnsupported {
            category: OperatorCategory::Input.as_str().to_string(),
            action: "mouse_move".into(),
            platform: platform_tag(),
            fallbacks_used: vec!["enigo_new_failed".to_string()],
        }
    })?;
    en.move_mouse(x as i32, y as i32, enigo::Coordinate::Abs).map_err(|e| XiaobaiError::ExecutionError {
        category: OperatorCategory::Input.as_str().into(),
        action: "mouse_move".into(),
        detail: format!("enigo move_mouse err: {e:?}"),
    })?;
    fbs.push("enigo_move_mouse_abs");
    Ok(OperatorOutput::quick(format!("mouse_move → ({},{})", x, y))
        .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
        .with_elapsed(t0.elapsed().as_millis() as u64))
}

pub(crate) fn click_or_double(
    action: &str,
    param: &ActionParam,
    fbs_init: &[&'static str],
) -> Result<OperatorOutput, XiaobaiError> {
    let t0 = Instant::now();
    let mut fbs: Vec<&'static str> = fbs_init.to_vec();
    let button = param.get_str("button").unwrap_or("left");
    let x = param.get_i64("x");
    let y = param.get_i64("y");
    let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
        category: OperatorCategory::Input.as_str().to_string(),
        action: "click".into(),
        platform: platform_tag(),
        fallbacks_used: vec!["enigo_new_failed".to_string()],
    })?;
    if let (Some(xx), Some(yy)) = (x, y) {
        en.move_mouse(xx as i32, yy as i32, enigo::Coordinate::Abs).map_err(|e| XiaobaiError::ExecutionError {
            category: OperatorCategory::Input.as_str().into(),
            action: "click".into(),
            detail: format!("enigo move before click err: {e:?}"),
        })?;
    }
    let b = parse_button(button);
    if action == "double_click" {
        en.button(b, enigo::Direction::Press).ok();
        en.button(b, enigo::Direction::Release).ok();
        std::thread::sleep(Duration::from_millis(50));
        en.button(b, enigo::Direction::Press).ok();
        en.button(b, enigo::Direction::Release).ok();
        fbs.push("enigo_double_left_click");
    } else {
        en.button(b, enigo::Direction::Press).ok();
        en.button(b, enigo::Direction::Release).ok();
        fbs.push("enigo_single_click");
    }
    Ok(OperatorOutput::quick(format!("{action} done（button={button}）"))
        .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
        .with_elapsed(t0.elapsed().as_millis() as u64))
}

pub(crate) fn mouse_drag(param: &ActionParam, fbs_init: &[&'static str]) -> Result<OperatorOutput, XiaobaiError> {
    let t0 = Instant::now();
    let mut fbs: Vec<&'static str> = fbs_init.to_vec();
    let fx = require_int(param, "mouse_drag", "from_x")?;
    let fy = require_int(param, "mouse_drag", "from_y")?;
    let tx = require_int(param, "mouse_drag", "to_x")?;
    let ty = require_int(param, "mouse_drag", "to_y")?;
    let button = param.get_str("button").unwrap_or("left");
    let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
        category: OperatorCategory::Input.as_str().to_string(),
        action: "mouse_drag".into(),
        platform: platform_tag(),
        fallbacks_used: vec!["enigo_new_failed".to_string()],
    })?;
    let b = parse_button(button);
    en.move_mouse(fx as i32, fy as i32, enigo::Coordinate::Abs).ok();
    en.button(b, enigo::Direction::Press).ok();
    std::thread::sleep(Duration::from_millis(40));
    en.move_mouse(tx as i32, ty as i32, enigo::Coordinate::Abs).ok();
    std::thread::sleep(Duration::from_millis(40));
    en.button(b, enigo::Direction::Release).ok();
    fbs.push("enigo_press_move_release");
    Ok(OperatorOutput::quick(format!("拖拽 ({fx},{fy}) → ({tx},{ty})"))
        .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
        .with_elapsed(t0.elapsed().as_millis() as u64))
}

pub(crate) fn scroll_wheel(param: &ActionParam, fbs_init: &[&'static str]) -> Result<OperatorOutput, XiaobaiError> {
    let t0 = Instant::now();
    let mut fbs: Vec<&'static str> = fbs_init.to_vec();
    let delta = require_int(param, "scroll_wheel", "delta")?;
    let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
        category: OperatorCategory::Input.as_str().to_string(),
        action: "scroll_wheel".into(),
        platform: platform_tag(),
        fallbacks_used: vec!["enigo_new_failed".to_string()],
    })?;
    en.scroll(delta as i32, Axis::Vertical).map_err(|e| XiaobaiError::ExecutionError {
        category: OperatorCategory::Input.as_str().into(),
        action: "scroll_wheel".into(),
        detail: format!("enigo scroll err: {e:?}"),
    })?;
    fbs.push("enigo_scroll_v");
    Ok(OperatorOutput::quick(format!("滚轮滚动 {delta} ticks（负向上，正向下）"))
        .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
        .with_elapsed(t0.elapsed().as_millis() as u64))
}

pub(crate) fn move_cursor_to_center(fbs_init: &[&'static str]) -> Result<OperatorOutput, XiaobaiError> {
    let t0 = Instant::now();
    let mut fbs: Vec<&'static str> = fbs_init.to_vec();
    use screenshots::Screen;
    let screens = Screen::all().map_err(|_e| XiaobaiError::OperatorUnsupported {
        category: OperatorCategory::Input.as_str().to_string(),
        action: "move_cursor_to_center".into(),
        platform: platform_tag(),
        fallbacks_used: vec!["screens_enum_failed".to_string()],
    })?;
    let s = screens.first().ok_or_else(|| XiaobaiError::OperatorUnsupported {
        category: OperatorCategory::Input.as_str().to_string(),
        action: "move_cursor_to_center".into(),
        platform: platform_tag(),
        fallbacks_used: vec!["no_screens_found".to_string()],
    })?;
    // screenshots 0.8 Screen 不暴露 .width()/.height() 方法；先捕获一帧拿尺寸
    let sample = s.capture().map_err(|e| XiaobaiError::ExecutionError {
        category: OperatorCategory::Input.as_str().into(),
        action: "move_cursor_to_center".into(),
        detail: format!("探测屏幕尺寸失败: {e}"),
    })?;
    let (w, h) = (sample.width() as i64, sample.height() as i64);
    let cx = w / 2;
    let cy = h / 2;
    let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
        category: OperatorCategory::Input.as_str().to_string(),
        action: "move_cursor_to_center".into(),
        platform: platform_tag(),
        fallbacks_used: vec!["enigo_new_failed".to_string()],
    })?;
    en.move_mouse(cx as i32, cy as i32, enigo::Coordinate::Abs).map_err(|e| XiaobaiError::ExecutionError {
        category: OperatorCategory::Input.as_str().into(),
        action: "move_cursor_to_center".into(),
        detail: format!("enigo move_mouse err: {e:?}"),
    })?;
    fbs.push("enigo_move_center");
    Ok(OperatorOutput::quick(format!("屏幕中心 → ({cx},{cy})"))
        .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
        .with_elapsed(t0.elapsed().as_millis() as u64))
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 键盘操作子模块：press_key / hotkey / key_sequence

use std::time::Instant;

use enigo::Keyboard;

use mox_voice_core_svc::errors::XiaobaiError;
use mox_voice_core_svc::operator::{ActionParam, OperatorCategory, OperatorOutput};

use crate::helpers::platform_tag;

pub(crate) fn parse_modifier(s: &str) -> enigo::Key {
    match s.trim().to_lowercase().as_str() {
        "ctrl" | "control" => enigo::Key::Control,
        "shift" => enigo::Key::Shift,
        "alt" | "option" => enigo::Key::Alt,
        "meta" | "win" | "cmd" | "command" | "super" => enigo::Key::Meta,
        _ => enigo::Key::Control,
    }
}

pub(crate) fn parse_key(s: &str) -> enigo::Key {
    use enigo::Key::*;
    let t = s.trim().to_lowercase();
    match t.as_str() {
        "enter" | "return" => Return,
        "tab" => Tab,
        "space" => Space,
        "backspace" | "back" => Backspace,
        "escape" | "esc" => Escape,
        "capslock" | "caps" => CapsLock,
        "f1" => F1, "f2" => F2, "f3" => F3, "f4" => F4, "f5" => F5, "f6" => F6,
        "f7" => F7, "f8" => F8, "f9" => F9, "f10" => F10, "f11" => F11, "f12" => F12,
        "home" => Home, "end" => End, "pageup" => PageUp, "pagedown" => PageDown,
        "left" | "arrowleft" => LeftArrow, "right" | "arrowright" => RightArrow,
        "up" | "arrowup" => UpArrow, "down" | "arrowdown" => DownArrow,
        "delete" | "del" => Delete, "insert" | "ins" => Insert,
        "ctrl" | "control" => Control, "shift" => Shift, "alt" => Alt,
        "meta" | "win" | "super" | "cmd" => Meta,
        other => {
            // 单字符 fallback
            if other.chars().count() == 1 {
                let c = other.chars().next().unwrap();
                Unicode(c)
            } else {
                Unicode(' ') // 未知键：退化为空格（不中断操作）
            }
        }
    }
}

pub(crate) fn press_key(param: &ActionParam, fbs_init: &[&'static str]) -> Result<OperatorOutput, XiaobaiError> {
    let t0 = Instant::now();
    let mut fbs: Vec<&'static str> = fbs_init.to_vec();
    let key = param.get_str("key").ok_or_else(|| XiaobaiError::InvalidArgument {
        action: "press_key".into(),
        param: "key".into(),
        value: "<missing>".into(),
        hint: "key 名称如 enter/esc/a/f5 等".into(),
    })?;
    let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
        category: OperatorCategory::Input.as_str().into(),
        action: "press_key".into(),
        platform: platform_tag(),
        fallbacks_used: vec!["enigo_new_failed".to_string()],
    })?;
    let k = parse_key(key);
    en.key(k, enigo::Direction::Press).map_err(|e| XiaobaiError::ExecutionError {
        category: OperatorCategory::Input.as_str().into(),
        action: "press_key".into(),
        detail: format!("enigo press down err: {e:?}"),
    })?;
    en.key(k, enigo::Direction::Release).ok();
    fbs.push("enigo_press_release");
    Ok(OperatorOutput::quick(format!("按一下 {key}"))
        .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
        .with_elapsed(t0.elapsed().as_millis() as u64))
}

pub(crate) fn hotkey(param: &ActionParam, fbs_init: &[&'static str]) -> Result<OperatorOutput, XiaobaiError> {
    let t0 = Instant::now();
    let mut fbs: Vec<&'static str> = fbs_init.to_vec();
    let modifiers: Vec<String> = param.0.get("modifiers").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|s| s.as_str().map(|ss| ss.to_string())).collect())
        .unwrap_or_default();
    let key = param.get_str("key").ok_or_else(|| XiaobaiError::InvalidArgument {
        action: "hotkey".into(),
        param: "key".into(),
        value: "<missing>".into(),
        hint: "hotkey 需要 modifiers + key".into(),
    })?;
    let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
        category: OperatorCategory::Input.as_str().to_string(),
        action: "hotkey".into(),
        platform: platform_tag(),
        fallbacks_used: vec!["enigo_new_failed".to_string()],
    })?;
    for m in modifiers.iter() {
        let k = parse_modifier(m);
        en.key(k, enigo::Direction::Press).ok();
    }
    let k = parse_key(key);
    en.key(k, enigo::Direction::Press).ok();
    en.key(k, enigo::Direction::Release).ok();
    for m in modifiers.iter().rev() {
        let k = parse_modifier(m);
        en.key(k, enigo::Direction::Release).ok();
    }
    fbs.push("enigo_hotkey_modifiers_sequence");
    Ok(OperatorOutput::quick(format!("hotkey [{modifiers:?}] + {key}"))
        .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
        .with_elapsed(t0.elapsed().as_millis() as u64))
}

pub(crate) fn key_sequence(param: &ActionParam, fbs_init: &[&'static str]) -> Result<OperatorOutput, XiaobaiError> {
    let t0 = Instant::now();
    let mut fbs: Vec<&'static str> = fbs_init.to_vec();
    let keys: Vec<String> = param.0.get("keys").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|s| s.as_str().map(|ss| ss.to_string())).collect())
        .unwrap_or_default();
    if keys.is_empty() {
        return Err(XiaobaiError::InvalidArgument {
            action: "key_sequence".into(),
            param: "keys".into(),
            value: "[]".into(),
            hint: "keys 不能是空数组".into(),
        });
    }
    let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
        category: OperatorCategory::Input.as_str().to_string(),
        action: "key_sequence".into(),
        platform: platform_tag(),
        fallbacks_used: vec!["enigo_new_failed".to_string()],
    })?;
    for k in keys.iter() {
        let kk = parse_key(k);
        en.key(kk, enigo::Direction::Press).ok();
        en.key(kk, enigo::Direction::Release).ok();
    }
    fbs.push("enigo_key_sequence_n");
    Ok(OperatorOutput::quick(format!("按顺序 {n} 个键", n = keys.len()))
        .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
        .with_elapsed(t0.elapsed().as_millis() as u64))
}

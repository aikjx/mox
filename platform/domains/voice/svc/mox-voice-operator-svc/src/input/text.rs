// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 文本输入子模块：type_text（ASCII 走 enigo，中文走剪贴板粘贴 Ctrl+V 回退）

use std::time::Instant;

use enigo::Keyboard;

use mox_voice_core_svc::errors::XiaobaiError;
use mox_voice_core_svc::operator::{ActionParam, OperatorCategory, OperatorOutput};

use crate::helpers::platform_tag;

pub(crate) fn type_text(param: &ActionParam, fbs_init: &[&'static str]) -> Result<OperatorOutput, XiaobaiError> {
    let t0 = Instant::now();
    let mut fbs: Vec<&'static str> = fbs_init.to_vec();
    let text = param.get_str("text").ok_or_else(|| XiaobaiError::InvalidArgument {
        action: "type_text".into(),
        param: "text".into(),
        value: "<missing>".into(),
        hint: "需要 text 字符串".into(),
    })?.to_string();
    let has_chinese = text.chars().any(|c| c as u32 > 0x7F);
    // ASCII：直接 enigo.key_sequence
    if !has_chinese {
        let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
            category: OperatorCategory::Input.as_str().to_string(),
            action: "type_text(ascii)".into(),
            platform: platform_tag(),
            fallbacks_used: vec!["enigo_new_failed".to_string()],
        })?;
        en.text(&text).map_err(|e| XiaobaiError::ExecutionError {
            category: OperatorCategory::Input.as_str().into(),
            action: "type_text".into(),
            detail: format!("enigo text err: {e:?}"),
        })?;
        fbs.push("enigo_text_ascii");
        return Ok(OperatorOutput::quick(format!("输入文本 {len} 字符（ASCII）", len = text.chars().count()))
            .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
            .with_elapsed(t0.elapsed().as_millis() as u64));
    }
    // 中文：写入剪贴板 → Ctrl+V 粘贴（需要 L2 权限）
    fbs.push("chinese_clipboard_paste_backoff");
    let copy_op = crate::file::FileOperator::default();
    let fb_copy = copy_op.copy_impl(&text).map_err(|e| XiaobaiError::OperatorUnsupported {
        category: OperatorCategory::Input.as_str().to_string(),
        action: "type_text(chinese)".into(),
        platform: platform_tag(),
        fallbacks_used: vec!["copy_chinese_to_clipboard_failed".to_string()],
    })?;
    fbs.extend(fb_copy.iter().copied());
    // Ctrl+V
    let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
        category: OperatorCategory::Input.as_str().to_string(),
        action: "type_text(chinese)→paste".into(),
        platform: platform_tag(),
        fallbacks_used: vec!["enigo_new_failed_for_ctrl_v".to_string()],
    })?;
    let ctrl = if cfg!(target_os = "macos") { enigo::Key::Meta } else { enigo::Key::Control };
    en.key(ctrl, enigo::Direction::Press).ok();
    en.key(enigo::Key::Unicode('v'), enigo::Direction::Press).ok();
    en.key(enigo::Key::Unicode('v'), enigo::Direction::Release).ok();
    en.key(ctrl, enigo::Direction::Release).ok();
    fbs.push("ctrl_v_paste");
    Ok(OperatorOutput::quick(format!("输入文本 {len} 字符（中文剪贴板粘贴回退）", len = text.chars().count()))
        .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
        .with_elapsed(t0.elapsed().as_millis() as u64))
}

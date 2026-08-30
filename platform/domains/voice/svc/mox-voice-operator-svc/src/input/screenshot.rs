// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 截屏子模块：screenshot（screenshots crate 真实实现，跨平台）

use std::time::Instant;

use serde_json::json;

use mox_voice_core_svc::errors::XiaobaiError;
use mox_voice_core_svc::operator::{ActionParam, OperatorCategory, OperatorOutput};

use crate::helpers::platform_tag;

pub(crate) fn screenshot(_param: &ActionParam, fbs_init: &[&'static str]) -> Result<OperatorOutput, XiaobaiError> {
    let t0 = Instant::now();
    let mut fbs: Vec<&'static str> = fbs_init.to_vec();
    // screenshots crate 真实实现（Windows/macOS/Linux 支持）
    fbs.push("screenshots_crate_capture_display0");
    use screenshots::Screen;
    let screens = Screen::all().map_err(|e| XiaobaiError::ExecutionError {
        category: OperatorCategory::Input.as_str().into(),
        action: "screenshot".into(),
        detail: format!("枚举显示器失败: {e}"),
    })?;
    let first = screens.first().ok_or_else(|| XiaobaiError::OperatorUnsupported {
        category: OperatorCategory::Input.as_str().to_string(),
        action: "screenshot".into(),
        platform: platform_tag(),
        fallbacks_used: vec!["no_screens_found".to_string()],
    })?;
    let img = first.capture().map_err(|e| XiaobaiError::ExecutionError {
        category: OperatorCategory::Input.as_str().into(),
        action: "screenshot".into(),
        detail: format!("截屏失败: {e}"),
    })?;
    let mut png_bytes: Vec<u8> = Vec::new();
    {
        use image::{ImageEncoder, codecs::png::PngEncoder, ExtendedColorType};
        let enc = PngEncoder::new(&mut png_bytes);
        enc.write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            ExtendedColorType::Rgba8,
        ).map_err(|e| XiaobaiError::ExecutionError {
            category: OperatorCategory::Input.as_str().into(),
            action: "screenshot".into(),
            detail: format!("PNG 编码失败: {e}"),
        })?;
    }
    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
    Ok(OperatorOutput::quick(format!(
        "截屏完成 {}x{}，PNG {} 字节（base64 在 payload）",
        img.width(),
        img.height(),
        png_bytes.len()
    ))
    .with_payload(json!({
        "width": img.width(),
        "height": img.height(),
        "bytes": png_bytes.len(),
        "png_base64": b64,
    }))
    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
    .with_elapsed(t0.elapsed().as_millis() as u64))
}

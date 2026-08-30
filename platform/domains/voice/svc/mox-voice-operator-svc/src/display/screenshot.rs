// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 区域截图：screenshot_capture_region

use image::ImageFormat;
use screenshots::Screen;

use mox_voice_core_svc::errors::{XiaobaiError, XiaobaiResult};
use mox_voice_core_svc::operator::OperatorCategory;

use super::DisplayOperator;

impl DisplayOperator {
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
}

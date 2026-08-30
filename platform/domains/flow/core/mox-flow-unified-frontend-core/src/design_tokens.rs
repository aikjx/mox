// Copyright (c) 2026 璇玑 RelGraph · 前端功能归一化核心 (Unified Frontend Core)
// Licensed under the MIT License.

//! 设计令牌 (Design Tokens)
//!
//! 统一的设计变量：颜色、间距、字号、圆角、阴影、动效等
//! 作为整个前端系统的单一真值来源 (Single Source of Truth)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::*;

/// 设计令牌集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignTokens {
    /// 主色调
    pub primary: ColorPalette,
    /// 成功色
    pub success: ColorPalette,
    /// 警告色
    pub warning: ColorPalette,
    /// 错误色
    pub error: ColorPalette,
    /// 信息色
    pub info: ColorPalette,
    /// 中性色
    pub neutral: ColorPalette,

    /// 背景色
    pub backgrounds: BgColors,
    /// 文字颜色
    pub text_colors: TextColors,
    /// 边框颜色
    pub border_colors: BorderColors,

    /// 间距
    pub spacing: SpacingScale,
    /// 字号
    pub typography: TypographyScale,
    /// 圆角
    pub radii: RadiiScale,
    /// 阴影
    pub shadows: ShadowScale,
    /// 动效
    pub motion: MotionTokens,
    /// 断点
    pub breakpoints: Breakpoints,
    /// z-index
    pub z_index: ZIndexTokens,

    /// 自定义令牌
    pub custom: HashMap<String, String>,
}

/// 背景色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgColors {
    pub page: String,
    pub container: String,
    pub elevated: String,
    pub hover: String,
    pub active: String,
    pub disabled: String,
}

/// 文字颜色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextColors {
    pub primary: String,
    pub secondary: String,
    pub tertiary: String,
    pub disabled: String,
    pub inverse: String,
}

/// 边框颜色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderColors {
    pub default: String,
    pub hover: String,
    pub active: String,
    pub focus: String,
    pub disabled: String,
}

/// 圆角刻度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadiiScale {
    pub none: String,
    pub sm: String,
    pub md: String,
    pub lg: String,
    pub xl: String,
    pub full: String,
}

impl Default for RadiiScale {
    fn default() -> Self {
        Self {
            none: "0".to_string(),
            sm: "4px".to_string(),
            md: "6px".to_string(),
            lg: "8px".to_string(),
            xl: "12px".to_string(),
            full: "9999px".to_string(),
        }
    }
}

/// 阴影刻度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowScale {
    pub xs: String,
    pub sm: String,
    pub md: String,
    pub lg: String,
    pub xl: String,
    pub xxl: String,
    pub inner: String,
}

impl Default for ShadowScale {
    fn default() -> Self {
        Self {
            xs: "0 1px 2px rgba(0,0,0,0.05)".to_string(),
            sm: "0 1px 3px rgba(0,0,0,0.1)".to_string(),
            md: "0 4px 6px rgba(0,0,0,0.1)".to_string(),
            lg: "0 10px 15px rgba(0,0,0,0.1)".to_string(),
            xl: "0 20px 25px rgba(0,0,0,0.15)".to_string(),
            xxl: "0 25px 50px rgba(0,0,0,0.25)".to_string(),
            inner: "inset 0 2px 4px rgba(0,0,0,0.1)".to_string(),
        }
    }
}

/// 动效令牌
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionTokens {
    pub duration_fast: String,
    pub duration_normal: String,
    pub duration_slow: String,
    pub easing_standard: String,
    pub easing_enter: String,
    pub easing_exit: String,
}

impl Default for MotionTokens {
    fn default() -> Self {
        Self {
            duration_fast: "150ms".to_string(),
            duration_normal: "300ms".to_string(),
            duration_slow: "500ms".to_string(),
            easing_standard: "cubic-bezier(0.4, 0, 0.2, 1)".to_string(),
            easing_enter: "cubic-bezier(0, 0, 0.2, 1)".to_string(),
            easing_exit: "cubic-bezier(0.4, 0, 1, 1)".to_string(),
        }
    }
}

/// 断点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoints {
    pub sm: String,
    pub md: String,
    pub lg: String,
    pub xl: String,
    pub xxl: String,
}

impl Default for Breakpoints {
    fn default() -> Self {
        Self {
            sm: "640px".to_string(),
            md: "768px".to_string(),
            lg: "1024px".to_string(),
            xl: "1280px".to_string(),
            xxl: "1536px".to_string(),
        }
    }
}

/// Z-Index 令牌
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZIndexTokens {
    pub base: i32,
    pub dropdown: i32,
    pub sticky: i32,
    pub modal: i32,
    pub popover: i32,
    pub tooltip: i32,
    pub toast: i32,
}

impl Default for ZIndexTokens {
    fn default() -> Self {
        Self {
            base: 0,
            dropdown: 1000,
            sticky: 1020,
            modal: 1050,
            popover: 1060,
            tooltip: 1070,
            toast: 1080,
        }
    }
}

impl DesignTokens {
    /// 创建默认（亮色）设计令牌
    pub fn light() -> Self {
        let primary = ColorPalette::from_base("#3b82f6", "primary");
        let success = ColorPalette::from_base("#10b981", "success");
        let warning = ColorPalette::from_base("#f59e0b", "warning");
        let error = ColorPalette::from_base("#ef4444", "error");
        let info = ColorPalette::from_base("#06b6d4", "info");
        let neutral = ColorPalette::from_base("#6b7280", "neutral");

        Self {
            primary: primary.clone(),
            success: success.clone(),
            warning: warning.clone(),
            error: error.clone(),
            info: info.clone(),
            neutral: neutral.clone(),

            backgrounds: BgColors {
                page: "#f9fafb".to_string(),
                container: "#ffffff".to_string(),
                elevated: "#ffffff".to_string(),
                hover: "#f3f4f6".to_string(),
                active: "#e5e7eb".to_string(),
                disabled: "#f3f4f6".to_string(),
            },

            text_colors: TextColors {
                primary: "#111827".to_string(),
                secondary: "#4b5563".to_string(),
                tertiary: "#9ca3af".to_string(),
                disabled: "#d1d5db".to_string(),
                inverse: "#ffffff".to_string(),
            },

            border_colors: BorderColors {
                default: "#e5e7eb".to_string(),
                hover: "#d1d5db".to_string(),
                active: primary.c500.clone(),
                focus: primary.c500.clone(),
                disabled: "#e5e7eb".to_string(),
            },

            spacing: SpacingScale::default(),
            typography: TypographyScale::default(),
            radii: RadiiScale::default(),
            shadows: ShadowScale::default(),
            motion: MotionTokens::default(),
            breakpoints: Breakpoints::default(),
            z_index: ZIndexTokens::default(),

            custom: HashMap::new(),
        }
    }

    /// 创建暗色设计令牌
    pub fn dark() -> Self {
        let primary = ColorPalette::from_base("#60a5fa", "primary");
        let success = ColorPalette::from_base("#34d399", "success");
        let warning = ColorPalette::from_base("#fbbf24", "warning");
        let error = ColorPalette::from_base("#f87171", "error");
        let info = ColorPalette::from_base("#22d3ee", "info");
        let neutral = ColorPalette::from_base("#9ca3af", "neutral");

        Self {
            primary: primary.clone(),
            success: success.clone(),
            warning: warning.clone(),
            error: error.clone(),
            info: info.clone(),
            neutral: neutral.clone(),

            backgrounds: BgColors {
                page: "#0f172a".to_string(),
                container: "#1e293b".to_string(),
                elevated: "#334155".to_string(),
                hover: "#334155".to_string(),
                active: "#475569".to_string(),
                disabled: "#1e293b".to_string(),
            },

            text_colors: TextColors {
                primary: "#f1f5f9".to_string(),
                secondary: "#cbd5e1".to_string(),
                tertiary: "#94a3b8".to_string(),
                disabled: "#475569".to_string(),
                inverse: "#0f172a".to_string(),
            },

            border_colors: BorderColors {
                default: "#334155".to_string(),
                hover: "#475569".to_string(),
                active: primary.c500.clone(),
                focus: primary.c500.clone(),
                disabled: "#334155".to_string(),
            },

            spacing: SpacingScale::default(),
            typography: TypographyScale::default(),
            radii: RadiiScale::default(),
            shadows: ShadowScale::default(),
            motion: MotionTokens::default(),
            breakpoints: Breakpoints::default(),
            z_index: ZIndexTokens::default(),

            custom: HashMap::new(),
        }
    }

    /// 设置自定义令牌
    pub fn set_custom(&mut self, key: &str, value: &str) {
        self.custom.insert(key.to_string(), value.to_string());
    }

    /// 获取主色
    pub fn primary_color(&self, shade: u32) -> &str {
        self.primary.get(shade)
    }
}

impl Default for DesignTokens {
    fn default() -> Self {
        Self::light()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_tokens() {
        let tokens = DesignTokens::light();
        assert_eq!(tokens.primary.c500, "#3b82f6");
        assert_eq!(tokens.text_colors.primary, "#111827");
        assert_eq!(tokens.backgrounds.page, "#f9fafb");
    }

    #[test]
    fn test_dark_tokens() {
        let tokens = DesignTokens::dark();
        assert_eq!(tokens.backgrounds.page, "#0f172a");
        assert_eq!(tokens.text_colors.primary, "#f1f5f9");
    }

    #[test]
    fn test_radii_default() {
        let r = RadiiScale::default();
        assert_eq!(r.md, "6px");
        assert_eq!(r.full, "9999px");
    }

    #[test]
    fn test_shadows_default() {
        let s = ShadowScale::default();
        assert!(!s.md.is_empty());
        assert!(!s.inner.is_empty());
    }

    #[test]
    fn test_motion_default() {
        let m = MotionTokens::default();
        assert_eq!(m.duration_normal, "300ms");
    }

    #[test]
    fn test_zindex_default() {
        let z = ZIndexTokens::default();
        assert_eq!(z.modal, 1050);
        assert!(z.tooltip > z.modal);
    }

    #[test]
    fn test_custom_tokens() {
        let mut tokens = DesignTokens::light();
        tokens.set_custom("brand-gold", "#ffd700");
        assert_eq!(tokens.custom.get("brand-gold").unwrap(), "#ffd700");
    }

    #[test]
    fn test_primary_color() {
        let tokens = DesignTokens::light();
        assert_eq!(tokens.primary_color(500), "#3b82f6");
        assert_eq!(tokens.primary_color(300), tokens.primary.c300);
    }
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 主题配置 — UI主题定制（深色/浅色/自定义色板）

use serde::{Deserialize, Serialize};

/// 主题模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    Light,
    Dark,
    Auto,
}

impl Default for ThemeMode {
    fn default() -> Self { ThemeMode::Auto }
}

/// 色板配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    /// 主色
    pub primary: String,
    /// 主色悬浮
    pub primary_hover: String,
    /// 主色按下
    pub primary_active: String,
    /// 成功色
    pub success: String,
    /// 警告色
    pub warning: String,
    /// 错误色
    pub error: String,
    /// 信息色
    pub info: String,
    /// 背景色
    pub background: String,
    /// 表面色（卡片/弹窗）
    pub surface: String,
    /// 文字主色
    pub text_primary: String,
    /// 文字次色
    pub text_secondary: String,
    /// 边框色
    pub border: String,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            primary: "#1677ff".into(),
            primary_hover: "#4096ff".into(),
            primary_active: "#0958d9".into(),
            success: "#52c41a".into(),
            warning: "#faad14".into(),
            error: "#ff4d4f".into(),
            info: "#1677ff".into(),
            background: "#f5f5f5".into(),
            surface: "#ffffff".into(),
            text_primary: "#262626".into(),
            text_secondary: "#595959".into(),
            border: "#d9d9d9".into(),
        }
    }
}

/// 主题配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// 主题模式
    pub mode: ThemeMode,
    /// 浅色模式色板
    pub light_palette: ColorPalette,
    /// 深色模式色板
    pub dark_palette: ColorPalette,
    /// 圆角大小（px）
    pub border_radius: u32,
    /// 字体族
    pub font_family: String,
    /// 基础字号（px）
    pub font_size_base: u32,
    /// 紧凑模式
    pub compact: bool,
    /// 侧边栏宽度（px）
    pub sidebar_width: u32,
    /// 顶部导航高度（px）
    pub header_height: u32,
    /// 动画开关
    pub animations_enabled: bool,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        let mut dark = ColorPalette::default();
        dark.background = "#141414".into();
        dark.surface = "#1f1f1f".into();
        dark.text_primary = "#f0f0f0".into();
        dark.text_secondary = "#a0a0a0".into();
        dark.border = "#303030".into();

        Self {
            mode: ThemeMode::Auto,
            light_palette: ColorPalette::default(),
            dark_palette: dark,
            border_radius: 6,
            font_family: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif".into(),
            font_size_base: 14,
            compact: false,
            sidebar_width: 240,
            header_height: 56,
            animations_enabled: true,
        }
    }
}

impl ThemeConfig {
    /// 生成CSS变量（用于前端注入）
    pub fn to_css_vars(&self) -> String {
        let p = &self.light_palette;
        format!(
            ":root {{\n  --mox-primary: {};\n  --mox-success: {};\n  --mox-warning: {};\n  --mox-error: {};\n  --mox-background: {};\n  --mox-surface: {};\n  --mox-text-primary: {};\n  --mox-text-secondary: {};\n  --mox-border: {};\n  --mox-border-radius: {}px;\n  --mox-font-size-base: {}px;\n}}",
            p.primary, p.success, p.warning, p.error, p.background, p.surface,
            p.text_primary, p.text_secondary, p.border, self.border_radius, self.font_size_base
        )
    }
}

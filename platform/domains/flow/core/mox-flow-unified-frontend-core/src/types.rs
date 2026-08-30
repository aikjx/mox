// Copyright (c) 2026 璇玑 RelGraph · 前端功能归一化核心 (Unified Frontend Core)
// Licensed under the MIT License.

//! 核心类型定义

use serde::{Deserialize, Serialize};

/// 颜色调色板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    /// 50 (最浅)
    pub c50: String,
    pub c100: String,
    pub c200: String,
    pub c300: String,
    pub c400: String,
    /// 500 (基准)
    pub c500: String,
    pub c600: String,
    pub c700: String,
    pub c800: String,
    /// 900 (最深)
    pub c900: String,
}

impl ColorPalette {
    pub fn from_base(base_hex: &str, _name: &str) -> Self {
        // 简化：基于基准色生成渐变（实际用色彩算法）
        let base = hex_to_rgb(base_hex).unwrap_or((100, 100, 100));
        Self {
            c50: lighten(&base, 0.45),
            c100: lighten(&base, 0.35),
            c200: lighten(&base, 0.25),
            c300: lighten(&base, 0.15),
            c400: lighten(&base, 0.07),
            c500: base_hex.to_string(),
            c600: darken(&base, 0.07),
            c700: darken(&base, 0.15),
            c800: darken(&base, 0.25),
            c900: darken(&base, 0.35),
        }
    }

    pub fn get(&self, shade: u32) -> &str {
        match shade {
            50 => &self.c50,
            100 => &self.c100,
            200 => &self.c200,
            300 => &self.c300,
            400 => &self.c400,
            500 => &self.c500,
            600 => &self.c600,
            700 => &self.c700,
            800 => &self.c800,
            900 => &self.c900,
            _ => &self.c500,
        }
    }
}

fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

fn lighten(base: &(u8, u8, u8), amount: f64) -> String {
    let r = (base.0 as f64 + (255.0 - base.0 as f64) * amount).round() as u8;
    let g = (base.1 as f64 + (255.0 - base.1 as f64) * amount).round() as u8;
    let b = (base.2 as f64 + (255.0 - base.2 as f64) * amount).round() as u8;
    rgb_to_hex(r, g, b)
}

fn darken(base: &(u8, u8, u8), amount: f64) -> String {
    let r = (base.0 as f64 * (1.0 - amount)).round() as u8;
    let g = (base.1 as f64 * (1.0 - amount)).round() as u8;
    let b = (base.2 as f64 * (1.0 - amount)).round() as u8;
    rgb_to_hex(r, g, b)
}

/// 间距刻度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpacingScale {
    pub xs: String,
    pub sm: String,
    pub md: String,
    pub lg: String,
    pub xl: String,
    pub xxl: String,
    pub xxxl: String,
}

impl Default for SpacingScale {
    fn default() -> Self {
        Self {
            xs: "4px".to_string(),
            sm: "8px".to_string(),
            md: "16px".to_string(),
            lg: "24px".to_string(),
            xl: "32px".to_string(),
            xxl: "48px".to_string(),
            xxxl: "64px".to_string(),
        }
    }
}

/// 字号刻度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypographyScale {
    pub xs: String,
    pub sm: String,
    pub base: String,
    pub lg: String,
    pub xl: String,
    pub xxl: String,
    pub xxxl: String,
    pub h1: String,
    pub h2: String,
    pub h3: String,
    pub h4: String,
}

impl Default for TypographyScale {
    fn default() -> Self {
        Self {
            xs: "12px".to_string(),
            sm: "13px".to_string(),
            base: "14px".to_string(),
            lg: "16px".to_string(),
            xl: "18px".to_string(),
            xxl: "20px".to_string(),
            xxxl: "24px".to_string(),
            h1: "32px".to_string(),
            h2: "28px".to_string(),
            h3: "24px".to_string(),
            h4: "20px".to_string(),
        }
    }
}

/// 组件分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentCategory {
    /// 基础组件
    Basic,
    /// 表单组件
    Form,
    /// 数据展示
    DataDisplay,
    /// 反馈组件
    Feedback,
    /// 导航组件
    Navigation,
    /// 布局组件
    Layout,
    /// 业务组件
    Business,
    /// 图表组件
    Chart,
    /// 图谱组件
    Graph,
    /// 其他
    Other,
}

/// 组件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentType {
    /// 原子组件
    Atom,
    /// 分子组件
    Molecule,
    /// 有机体组件
    Organism,
    /// 模板组件
    Template,
    /// 页面组件
    Page,
}

/// 模块分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleCategory {
    /// 知识图谱
    KnowledgeGraph,
    /// 知识库
    KnowledgeBase,
    /// 云盘
    CloudDrive,
    /// 系统管理
    System,
    /// 数据分析
    Analytics,
    /// 算法中心
    Algorithm,
    /// 流程中心
    Workflow,
    /// 低代码平台
    Lowcode,
    /// 工作台
    Workspace,
    /// 个人中心
    Profile,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_palette() {
        let palette = ColorPalette::from_base("#3b82f6", "primary");
        assert_eq!(palette.c500, "#3b82f6");
        assert!(!palette.c50.is_empty());
        assert!(!palette.c900.is_empty());
        assert_eq!(palette.get(500), "#3b82f6");
        assert_eq!(palette.get(999), "#3b82f6"); // 未知返回基准
    }

    #[test]
    fn test_spacing_default() {
        let s = SpacingScale::default();
        assert_eq!(s.md, "16px");
        assert_eq!(s.lg, "24px");
    }

    #[test]
    fn test_typography_default() {
        let t = TypographyScale::default();
        assert_eq!(t.base, "14px");
        assert_eq!(t.h1, "32px");
    }

    #[test]
    fn test_hex_conversion() {
        assert_eq!(hex_to_rgb("#ff0000"), Some((255, 0, 0)));
        assert_eq!(hex_to_rgb("#00ff00"), Some((0, 255, 0)));
        assert_eq!(hex_to_rgb("#0000ff"), Some((0, 0, 255)));
        assert_eq!(hex_to_rgb("invalid"), None);
        assert_eq!(hex_to_rgb("#abc"), None);
    }
}

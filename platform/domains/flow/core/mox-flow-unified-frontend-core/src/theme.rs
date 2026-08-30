// Copyright (c) 2026 璇玑 RelGraph · 前端功能归一化核心 (Unified Frontend Core)
// Licensed under the MIT License.

//! 主题系统
//!
//! 支持多主题切换、亮/暗模式、品牌定制

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::design_tokens::DesignTokens;
use crate::error::{FrontendError, FrontendResult};

/// 主题类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeType {
    /// 亮色
    Light,
    /// 暗色
    Dark,
    /// 跟随系统
    Auto,
    /// 自定义
    Custom,
}

/// 主题定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// 主题 ID
    pub id: String,
    /// 主题名称
    pub name: String,
    /// 主题类型
    pub theme_type: ThemeType,
    /// 设计令牌
    pub tokens: DesignTokens,
    /// 品牌名称
    pub brand_name: String,
    /// Logo URL
    pub logo_url: Option<String>,
    /// 描述
    pub description: Option<String>,
    /// 是否启用
    pub enabled: bool,
    /// 是否为默认主题
    pub is_default: bool,
}

impl Theme {
    /// 创建亮色主题
    pub fn light() -> Self {
        Self {
            id: "light".to_string(),
            name: "亮色主题".to_string(),
            theme_type: ThemeType::Light,
            tokens: DesignTokens::light(),
            brand_name: "璇玑 RelGraph".to_string(),
            logo_url: None,
            description: Some("默认亮色主题".to_string()),
            enabled: true,
            is_default: true,
        }
    }

    /// 创建暗色主题
    pub fn dark() -> Self {
        Self {
            id: "dark".to_string(),
            name: "暗色主题".to_string(),
            theme_type: ThemeType::Dark,
            tokens: DesignTokens::dark(),
            brand_name: "璇玑 RelGraph".to_string(),
            logo_url: None,
            description: Some("暗色护眼主题".to_string()),
            enabled: true,
            is_default: false,
        }
    }
}

/// 主题管理器
pub struct ThemeManager {
    /// 主题表
    themes: RwLock<HashMap<String, Theme>>,
    /// 当前主题 ID
    current_theme_id: RwLock<String>,
    /// 主题切换次数
    switch_count: AtomicU64,
}

impl ThemeManager {
    /// 创建主题管理器（默认亮色主题）
    pub fn new() -> Self {
        let light = Theme::light();
        let dark = Theme::dark();
        let current_id = light.id.clone();

        let mut themes = HashMap::new();
        themes.insert(light.id.clone(), light);
        themes.insert(dark.id.clone(), dark);

        Self {
            themes: RwLock::new(themes),
            current_theme_id: RwLock::new(current_id),
            switch_count: AtomicU64::new(0),
        }
    }

    /// 获取当前主题
    pub fn current_theme(&self) -> Theme {
        let id = self.current_theme_id.read().clone();
        self.themes
            .read()
            .get(&id)
            .cloned()
            .expect("current theme not found")
    }

    /// 获取当前设计令牌
    pub fn current_tokens(&self) -> DesignTokens {
        self.current_theme().tokens
    }

    /// 切换主题
    pub fn switch_theme(&self, theme_id: &str) -> FrontendResult<Theme> {
        let theme = self
            .themes
            .read()
            .get(theme_id)
            .cloned()
            .ok_or_else(|| FrontendError::NotFound(format!("theme '{}' not found", theme_id)))?;

        if !theme.enabled {
            return Err(FrontendError::InvalidConfig(format!(
                "theme '{}' is disabled",
                theme_id
            )));
        }

        *self.current_theme_id.write() = theme_id.to_string();
        self.switch_count.fetch_add(1, Ordering::Relaxed);

        Ok(theme)
    }

    /// 注册主题
    pub fn register_theme(&self, theme: Theme) -> FrontendResult<Theme> {
        if self.themes.read().contains_key(&theme.id) {
            return Err(FrontendError::AlreadyExists(format!(
                "theme '{}' already exists",
                theme.id
            )));
        }
        self.themes
            .write()
            .insert(theme.id.clone(), theme.clone());
        Ok(theme)
    }

    /// 获取所有主题
    pub fn list_themes(&self) -> Vec<Theme> {
        self.themes.read().values().cloned().collect()
    }

    /// 获取启用的主题
    pub fn list_enabled_themes(&self) -> Vec<Theme> {
        self.themes
            .read()
            .values()
            .filter(|t| t.enabled)
            .cloned()
            .collect()
    }

    /// 获取默认主题
    pub fn default_theme(&self) -> Option<Theme> {
        self.themes
            .read()
            .values()
            .find(|t| t.is_default)
            .cloned()
    }

    /// 获取主题数量
    pub fn theme_count(&self) -> usize {
        self.themes.read().len()
    }

    /// 获取切换次数
    pub fn switch_count(&self) -> u64 {
        self.switch_count.load(Ordering::Relaxed)
    }

    /// 切换亮/暗模式
    pub fn toggle_dark_mode(&self) -> FrontendResult<Theme> {
        let current_id = self.current_theme_id.read().clone();
        let themes = self.themes.read();
        let current = themes.get(&current_id).unwrap();

        let target_id = match current.theme_type {
            ThemeType::Light => "dark",
            ThemeType::Dark => "light",
            _ => "light",
        };

        drop(themes);
        self.switch_theme(target_id)
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_manager_default() {
        let tm = ThemeManager::new();
        assert_eq!(tm.theme_count(), 2);
        let current = tm.current_theme();
        assert_eq!(current.theme_type, ThemeType::Light);
        assert!(current.is_default);
    }

    #[test]
    fn test_switch_theme() {
        let tm = ThemeManager::new();
        tm.switch_theme("dark").unwrap();

        let current = tm.current_theme();
        assert_eq!(current.theme_type, ThemeType::Dark);
        assert_eq!(tm.switch_count(), 1);
    }

    #[test]
    fn test_toggle_dark_mode() {
        let tm = ThemeManager::new();

        let t1 = tm.toggle_dark_mode().unwrap();
        assert_eq!(t1.theme_type, ThemeType::Dark);

        let t2 = tm.toggle_dark_mode().unwrap();
        assert_eq!(t2.theme_type, ThemeType::Light);

        assert_eq!(tm.switch_count(), 2);
    }

    #[test]
    fn test_register_custom_theme() {
        let tm = ThemeManager::new();

        let mut custom = Theme::light();
        custom.id = "brand".to_string();
        custom.name = "品牌定制".to_string();
        custom.theme_type = ThemeType::Custom;

        tm.register_theme(custom).unwrap();
        assert_eq!(tm.theme_count(), 3);
    }

    #[test]
    fn test_switch_nonexistent_theme() {
        let tm = ThemeManager::new();
        assert!(tm.switch_theme("nonexistent").is_err());
    }

    #[test]
    fn test_list_enabled_themes() {
        let tm = ThemeManager::new();
        let enabled = tm.list_enabled_themes();
        assert_eq!(enabled.len(), 2);
    }

    #[test]
    fn test_default_theme() {
        let tm = ThemeManager::new();
        let default = tm.default_theme().unwrap();
        assert_eq!(default.id, "light");
    }
}

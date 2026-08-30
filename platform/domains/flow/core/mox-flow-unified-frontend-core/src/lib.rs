// Copyright (c) 2026 璇玑 RelGraph · 前端功能归一化核心 (Unified Frontend Core)
// Licensed under the MIT License.

//! 前端功能归一化核心
//!
//! 统一设计系统 + 组件库 + 功能模块重整
//!
//! 核心能力：
//! - Design Tokens（设计令牌）：颜色、间距、字号、圆角、阴影等
//! - 主题系统：亮/暗模式、品牌定制、多主题切换
//! - 组件库：基础组件、业务组件、图表组件分类注册
//! - 功能模块：按领域组织的功能模块注册与发现
//! - 布局系统：统一的页面布局、导航、侧边栏模式

pub mod error;
pub mod types;
pub mod design_tokens;
pub mod theme;
pub mod component_registry;
pub mod feature_modules;
pub mod layout_system;

pub use error::{FrontendError, FrontendResult};
pub use types::{
    ColorPalette, SpacingScale, TypographyScale, ComponentCategory,
    ComponentType, ModuleCategory,
};
pub use design_tokens::DesignTokens;
pub use theme::{Theme, ThemeManager, ThemeType};
pub use component_registry::{ComponentRegistry, ComponentDef, ComponentProp};
pub use feature_modules::{FeatureModule, FeatureModuleRegistry, ModuleRoute};
pub use layout_system::{LayoutSystem, LayoutType, NavItem, NavGroup};

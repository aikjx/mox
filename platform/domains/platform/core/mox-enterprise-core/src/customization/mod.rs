// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 定制层 — 白标、主题、动态字段

pub mod dynamic_field;
pub mod theme;
pub mod whitelabel;

pub use dynamic_field::{DynamicFieldSchema, DynamicFieldType, DynamicFieldValue};
pub use theme::ThemeConfig;
pub use whitelabel::WhitelabelConfig;

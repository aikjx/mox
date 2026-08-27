//! 定制层 — 白标、主题、动态字段

pub mod dynamic_field;
pub mod theme;
pub mod whitelabel;

pub use dynamic_field::{DynamicFieldSchema, DynamicFieldType, DynamicFieldValue};
pub use theme::ThemeConfig;
pub use whitelabel::WhitelabelConfig;

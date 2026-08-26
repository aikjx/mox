//! 真实 Hermes 宿主侧适配目录（feature = "hermes" 时编译）。
//!
//! - `hermes_shim`：把本 crate 的纯钩子接入真实 Hermes Agent Ultra 的插件系统。

pub mod hermes_shim;

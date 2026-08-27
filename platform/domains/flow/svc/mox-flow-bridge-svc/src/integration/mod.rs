// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 真实 Hermes 宿主侧适配目录（feature = "hermes" 时编译）。
//!
//! - `hermes_shim`：把本 crate 的纯钩子接入真实 Hermes Agent Ultra 的插件系统。

pub mod hermes_shim;

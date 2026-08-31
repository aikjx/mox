// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 内置节点处理器
//!
//! 提供控制节点、数据节点、脚本节点等通用节点的默认实现。
//! 各服务可通过注册自定义 Handler 覆盖默认行为。

pub mod control;
pub mod data;
pub mod script;

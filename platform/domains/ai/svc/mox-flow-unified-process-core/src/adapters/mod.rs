// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 适配层示例代码
//!
//! 展示各服务如何桥接到统一核心库。
//! 实际使用时，各服务应在自己的 crate 中实现这些转换，
//! 而不是放在核心库中（避免循环依赖）。

pub mod agent_flow_engine;
pub mod agent_workflow_engine;
pub mod flow_svc_model;
pub mod expert_governance;

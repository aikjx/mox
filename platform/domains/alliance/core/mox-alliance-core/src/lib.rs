// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Mox Alliance Core — 专家联盟共享内核
//!
//! 本 crate 提供专家联盟各子服务共享的纯算法和工具函数：
//! - DAG 操作（拓扑排序、依赖分析、环检测）
//! - RRF 融合算法
//! - 通用工具函数
//!
//! ## 设计原则
//! - **纯算法**：无 IO、无外部依赖、可独立单测
//! - **零副作用**：输入输出确定，方便测试
//! - **高性能**：核心路径用最优算法

pub mod dag;
pub mod fusion;
pub mod utils;

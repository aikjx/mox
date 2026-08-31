// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 通用工具模块
//!
//! 从三套引擎中提取的通用工具函数：
//! - 模板变量替换
//! - 条件表达式求值
//! - DAG 校验 / 拓扑排序 / 循环检测

pub mod template;
pub mod condition;
pub mod dag;

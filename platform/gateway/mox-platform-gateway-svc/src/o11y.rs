// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # o11y（可观测性）模块占位符
//!
//! 本模块为历史遗留模块的占位符，待后续逐模块迁移后再启用完整实现。
//! 当前仅提供最小化的类型导出，以满足上游 crate 的编译依赖。

use serde::{Deserialize, Serialize};

/// 可观测性配置（占位）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObservabilityConfig {
    pub metrics_enabled: bool,
    pub tracing_enabled: bool,
    pub logging_enabled: bool,
}

/// 指标收集器（占位）
#[derive(Debug, Clone, Default)]
pub struct MetricsCollector {
    #[allow(dead_code)]
    config: ObservabilityConfig,
}

impl MetricsCollector {
    pub fn new(config: ObservabilityConfig) -> Self {
        Self { config }
    }

    pub fn increment_counter(&self, _name: &str, _labels: &[(&str, &str)]) {
        // 占位实现：待迁移后接入真实指标系统
    }

    pub fn record_histogram(&self, _name: &str, _value: f64, _labels: &[(&str, &str)]) {
        // 占位实现
    }
}

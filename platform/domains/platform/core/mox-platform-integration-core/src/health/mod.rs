// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 统一健康检查 — Health Check
//!
//! 集中检查4大对接能力的健康状态，支持细粒度到单个Provider/插件/连接器。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// 健康状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// 健康
    Healthy,
    /// 降级（部分功能不可用）
    Degraded,
    /// 不健康
    Unhealthy,
    /// 未知
    Unknown,
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Unhealthy => "unhealthy",
            HealthStatus::Unknown => "unknown",
        }
    }

    pub fn is_healthy(&self) -> bool { matches!(self, HealthStatus::Healthy) }
}

/// 单个能力的健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityHealth {
    /// 能力名称（ai / plugin / enterprise / connector）
    pub capability: String,
    /// 整体状态
    pub status: HealthStatus,
    /// 检查时间
    pub checked_at: String,
    /// 耗时（毫秒）
    pub latency_ms: u64,
    /// 详细信息（子组件状态）
    #[serde(default)]
    pub details: HashMap<String, HealthStatus>,
    /// 错误信息（不健康时）
    #[serde(default)]
    pub errors: Vec<String>,
    /// 额外指标
    #[serde(default)]
    pub metrics: HashMap<String, f64>,
}

/// 集成层整体健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationHealth {
    /// 运行时名称
    pub runtime_name: String,
    /// 整体状态
    pub overall_status: HealthStatus,
    /// 检查时间
    pub checked_at: String,
    /// 总耗时（毫秒）
    pub total_latency_ms: u64,
    /// 各能力健康状态
    pub capabilities: HashMap<String, CapabilityHealth>,
    /// 汇总指标
    pub summary: HealthSummary,
}

/// 健康汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub total_capabilities: u32,
    pub healthy_count: u32,
    pub degraded_count: u32,
    pub unhealthy_count: u32,
    pub unknown_count: u32,
    pub healthy_percentage: f32,
}

/// 健康检查器
pub struct IntegrationHealthChecker {
    /// 历史健康记录（用于趋势分析）
    history: Arc<RwLock<Vec<IntegrationHealth>>>,
    /// 最大历史记录数
    max_history: usize,
}

impl IntegrationHealthChecker {
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::new())),
            max_history: 100,
        }
    }

    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    /// 执行完整健康检查
    pub async fn check_all(&self, runtime_name: &str) -> IntegrationHealth {
        let start = std::time::Instant::now();
        let mut capabilities = HashMap::new();

        // 检查4大能力（简化版：实际应调用各能力的健康检查接口）
        capabilities.insert("ai".into(), self.check_ai().await);
        capabilities.insert("plugin".into(), self.check_plugin().await);
        capabilities.insert("enterprise".into(), self.check_enterprise().await);
        capabilities.insert("connector".into(), self.check_connector().await);

        // 计算整体状态
        let overall_status = self.calculate_overall(&capabilities);
        let summary = self.build_summary(&capabilities);
        let total_latency_ms = start.elapsed().as_millis() as u64;

        let health = IntegrationHealth {
            runtime_name: runtime_name.into(),
            overall_status,
            checked_at: chrono::Utc::now().to_rfc3339(),
            total_latency_ms,
            capabilities,
            summary,
        };

        // 记录历史
        let mut guard = self.history.write();
        guard.push(health.clone());
        if guard.len() > self.max_history {
            guard.remove(0);
        }

        health
    }

    /// 检查AI能力
    async fn check_ai(&self) -> CapabilityHealth {
        let start = std::time::Instant::now();
        // 简化：实际应调用mox_ai_core的健康检查
        CapabilityHealth {
            capability: "ai".into(),
            status: HealthStatus::Unknown,
            checked_at: chrono::Utc::now().to_rfc3339(),
            latency_ms: start.elapsed().as_millis() as u64,
            details: HashMap::new(),
            errors: Vec::new(),
            metrics: HashMap::new(),
        }
    }

    /// 检查插件能力
    async fn check_plugin(&self) -> CapabilityHealth {
        let start = std::time::Instant::now();
        CapabilityHealth {
            capability: "plugin".into(),
            status: HealthStatus::Unknown,
            checked_at: chrono::Utc::now().to_rfc3339(),
            latency_ms: start.elapsed().as_millis() as u64,
            details: HashMap::new(),
            errors: Vec::new(),
            metrics: HashMap::new(),
        }
    }

    /// 检查政企适配能力
    async fn check_enterprise(&self) -> CapabilityHealth {
        let start = std::time::Instant::now();
        CapabilityHealth {
            capability: "enterprise".into(),
            status: HealthStatus::Unknown,
            checked_at: chrono::Utc::now().to_rfc3339(),
            latency_ms: start.elapsed().as_millis() as u64,
            details: HashMap::new(),
            errors: Vec::new(),
            metrics: HashMap::new(),
        }
    }

    /// 检查连接器能力
    async fn check_connector(&self) -> CapabilityHealth {
        let start = std::time::Instant::now();
        CapabilityHealth {
            capability: "connector".into(),
            status: HealthStatus::Unknown,
            checked_at: chrono::Utc::now().to_rfc3339(),
            latency_ms: start.elapsed().as_millis() as u64,
            details: HashMap::new(),
            errors: Vec::new(),
            metrics: HashMap::new(),
        }
    }

    /// 计算整体状态
    fn calculate_overall(&self, capabilities: &HashMap<String, CapabilityHealth>) -> HealthStatus {
        let statuses: Vec<HealthStatus> = capabilities.values().map(|c| c.status).collect();
        if statuses.is_empty() {
            return HealthStatus::Unknown;
        }
        if statuses.iter().all(|s| s.is_healthy()) {
            HealthStatus::Healthy
        } else if statuses.iter().any(|s| matches!(s, HealthStatus::Unhealthy)) {
            HealthStatus::Unhealthy
        } else if statuses.iter().any(|s| matches!(s, HealthStatus::Degraded)) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unknown
        }
    }

    /// 构建汇总
    fn build_summary(&self, capabilities: &HashMap<String, CapabilityHealth>) -> HealthSummary {
        let total = capabilities.len() as u32;
        let mut healthy = 0;
        let mut degraded = 0;
        let mut unhealthy = 0;
        let mut unknown = 0;
        for c in capabilities.values() {
            match c.status {
                HealthStatus::Healthy => healthy += 1,
                HealthStatus::Degraded => degraded += 1,
                HealthStatus::Unhealthy => unhealthy += 1,
                HealthStatus::Unknown => unknown += 1,
            }
        }
        let healthy_percentage = if total > 0 { (healthy as f32 / total as f32) * 100.0 } else { 0.0 };
        HealthSummary {
            total_capabilities: total,
            healthy_count: healthy,
            degraded_count: degraded,
            unhealthy_count: unhealthy,
            unknown_count: unknown,
            healthy_percentage,
        }
    }

    /// 获取历史记录
    pub fn history(&self) -> Vec<IntegrationHealth> {
        self.history.read().clone()
    }

    /// 获取最近一次健康状态
    pub fn latest(&self) -> Option<IntegrationHealth> {
        self.history.read().last().cloned()
    }
}

impl Default for IntegrationHealthChecker {
    fn default() -> Self { Self::new() }
}

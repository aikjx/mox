// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 调度器应用状态

use std::sync::Arc;

use mox_alliance_scheduler_core::{AllianceMetrics, TaskSchedulerImpl, ExecutorBridge};
use mox_alliance_scheduler_proto::ExpertMatcher;
use tokio::sync::mpsc;

use mox_alliance_common_proto::Task;
use mox_alliance_scheduler_proto::types::SchedulerConfig;

/// 调度器应用状态（共享给所有 handler）
#[derive(Clone)]
pub struct SchedulerAppState {
    pub config: SchedulerConfig,
    pub scheduler: Arc<TaskSchedulerImpl>,
    /// 专家匹配器（trait 对象，可插拔：规则 / 模块化权重）
    pub matcher: Arc<dyn ExpertMatcher>,
    /// 执行器桥接
    pub executor_bridge: Arc<dyn ExecutorBridge>,
    /// 任务派发通道（保留，向后兼容）
    pub dispatch_tx: mpsc::UnboundedSender<Task>,
    /// 执行器服务基础 URL（仅用于 /health 对 executor 依赖的存活探测；
    /// 读路径不经调度器代理，由网关直连执行器）
    pub executor_base_url: String,
    /// 联盟运行指标（匹配/LLM/融合/DAG 计数器，经 /metrics 暴露）
    pub metrics: Arc<AllianceMetrics>,
    /// HA 选主状态机（`None` = 未开启多活；此时本副本即事实上的唯一执行者）
    pub leadership: Option<crate::ha::SharedElector>,
}

impl SchedulerAppState {
    /// 创建应用状态（使用 ExecutorBridge，推荐）
    pub fn new_with_bridge(
        config: SchedulerConfig,
        scheduler: Arc<TaskSchedulerImpl>,
        matcher: Arc<dyn ExpertMatcher>,
        executor_bridge: Arc<dyn ExecutorBridge>,
    ) -> Self {
        // 创建一个空的 dispatch_tx 用于向后兼容
        let (dispatch_tx, _dispatch_rx) = mpsc::unbounded_channel::<Task>();

        Self {
            config,
            scheduler,
            matcher,
            executor_bridge,
            dispatch_tx,
            executor_base_url: "http://127.0.0.1:3200".to_string(),
            metrics: Arc::new(AllianceMetrics::new()),
            leadership: None,
        }
    }

    /// 创建应用状态（旧版 API，向后兼容）
    pub fn new(
        config: SchedulerConfig,
        scheduler: Arc<TaskSchedulerImpl>,
        matcher: Arc<dyn ExpertMatcher>,
        dispatch_tx: mpsc::UnboundedSender<Task>,
    ) -> Self {
        // 从 scheduler 获取 bridge（如果是旧版构造的，就是 NoopExecutorBridge）
        let executor_bridge = scheduler.executor_bridge().clone();

        Self {
            config,
            scheduler,
            matcher,
            executor_bridge,
            dispatch_tx,
            executor_base_url: "http://127.0.0.1:3200".to_string(),
            metrics: Arc::new(AllianceMetrics::new()),
            leadership: None,
        }
    }

    /// 设置执行器服务基础 URL（用于代理端点）
    pub fn with_executor_base_url(mut self, url: impl Into<String>) -> Self {
        self.executor_base_url = url.into();
        self
    }

    /// 注入共享的指标收集器（让调度器内部与 HTTP handler 共用同一实例）
    pub fn with_metrics(mut self, metrics: Arc<AllianceMetrics>) -> Self {
        self.metrics = metrics;
        self
    }

    /// 注入 HA 选主状态机（仅 `/leadership` 观测端点读取；对账由后台循环驱动）
    pub fn with_leadership(mut self, elector: crate::ha::SharedElector) -> Self {
        self.leadership = Some(elector);
        self
    }
}

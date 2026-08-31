// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家联盟 SSOT 常量
//!
//! 所有联盟子服务共享的常量定义。

/// 系统租户 ID（内置专家和系统级资源）
pub const SYSTEM_TENANT_ID: &str = "system";

/// 默认任务超时时间（毫秒）
pub const DEFAULT_TASK_TIMEOUT_MS: u64 = 30 * 60 * 1000; // 30 分钟

/// 默认节点超时时间（毫秒）
pub const DEFAULT_NODE_TIMEOUT_MS: u64 = 5 * 60 * 1000; // 5 分钟

/// 默认最大重试次数
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// 默认最大并发任务数
pub const DEFAULT_MAX_CONCURRENT_TASKS: usize = 100;

/// 默认最大并发节点数
pub const DEFAULT_MAX_CONCURRENT_NODES: usize = 50;

/// 任务队列默认容量
pub const DEFAULT_TASK_QUEUE_CAPACITY: usize = 1000;

/// 专家健康检查间隔（秒）
pub const EXPERT_HEALTH_CHECK_INTERVAL_SECS: u64 = 30;

/// 专家心跳超时（秒）
pub const EXPERT_HEARTBEAT_TIMEOUT_SECS: u64 = 90;

/// DAG 调度器轮询间隔（毫秒）
pub const DAG_SCHEDULER_POLL_INTERVAL_MS: u64 = 100;

/// 进度更新最小间隔（毫秒）— 防止过频更新
pub const PROGRESS_UPDATE_MIN_INTERVAL_MS: u64 = 500;

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 企业级处理流程 — Enterprise Flow
//!
//! 包含：统一错误码体系、trace_id传播、限流、配置热更新。

pub mod config_hot_reload;
pub mod error_codes;
pub mod rate_limit;
pub mod trace;

// 重导出
pub use config_hot_reload::{ConfigHotReloader, ConfigUpdateEvent};
pub use error_codes::{ErrorCode, ErrorCategory, PlatformError, error_code};
pub use rate_limit::{RateLimiter, RateLimitConfig, RateLimitResult};
pub use trace::{TraceContext, TraceId, current_trace_id, with_trace};

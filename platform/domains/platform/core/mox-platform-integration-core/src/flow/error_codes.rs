// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 统一错误码体系 — Error Code System
//!
//! 企业级错误码规范：6位数字，前2位分类，后4位序号。
//! 10xxxx: 系统错误  20xxxx: AI错误  30xxxx: 插件错误
//! 40xxxx: 政企错误  50xxxx: 连接器错误  90xxxx: 集成错误

use serde::{Deserialize, Serialize};
use std::fmt;

/// 错误分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u16)]
pub enum ErrorCategory {
    /// 系统错误（10xxxx）
    System = 10,
    /// AI错误（20xxxx）
    Ai = 20,
    /// 插件错误（30xxxx）
    Plugin = 30,
    /// 政企错误（40xxxx）
    Enterprise = 40,
    /// 连接器错误（50xxxx）
    Connector = 50,
    /// 集成错误（90xxxx）
    Integration = 90,
}

impl ErrorCategory {
    pub fn code_prefix(&self) -> u16 { *self as u16 }
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCategory::System => "system",
            ErrorCategory::Ai => "ai",
            ErrorCategory::Plugin => "plugin",
            ErrorCategory::Enterprise => "enterprise",
            ErrorCategory::Connector => "connector",
            ErrorCategory::Integration => "integration",
        }
    }
}

/// 错误码（6位数字）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ErrorCode(pub u32);

impl ErrorCode {
    /// 创建错误码
    pub fn new(category: ErrorCategory, seq: u16) -> Self {
        ErrorCode((category.code_prefix() as u32) * 10000 + seq as u32)
    }

    /// 获取分类
    pub fn category(&self) -> ErrorCategory {
        match self.0 / 10000 {
            10 => ErrorCategory::System,
            20 => ErrorCategory::Ai,
            30 => ErrorCategory::Plugin,
            40 => ErrorCategory::Enterprise,
            50 => ErrorCategory::Connector,
            _ => ErrorCategory::Integration,
        }
    }

    /// 获取序号
    pub fn seq(&self) -> u16 { (self.0 % 10000) as u16 }

    /// 格式化显示（如 "E200001"）
    pub fn display(&self) -> String { format!("E{:06}", self.0) }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// 便捷宏：创建错误码
pub fn error_code(category: ErrorCategory, seq: u16) -> ErrorCode {
    ErrorCode::new(category, seq)
}

/// 平台统一错误类型
#[derive(Debug, thiserror::Error)]
pub struct PlatformError {
    pub code: ErrorCode,
    pub message: String,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
    pub trace_id: Option<String>,
    pub details: serde_json::Value,
}

impl PlatformError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            source: None,
            trace_id: None,
            details: serde_json::Value::Null,
        }
    }

    pub fn with_source(mut self, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(trace_id) = &self.trace_id {
            write!(f, " (trace: {})", trace_id)?;
        }
        Ok(())
    }
}

// 常用错误码常量
pub mod codes {
    use super::*;
    // 系统错误
    pub const SYSTEM_INTERNAL: ErrorCode = ErrorCode(100001);
    pub const SYSTEM_TIMEOUT: ErrorCode = ErrorCode(100002);
    pub const SYSTEM_UNAVAILABLE: ErrorCode = ErrorCode(100003);
    // AI错误
    pub const AI_PROVIDER_NOT_FOUND: ErrorCode = ErrorCode(200001);
    pub const AI_PROVIDER_ERROR: ErrorCode = ErrorCode(200002);
    pub const AI_RATE_LIMITED: ErrorCode = ErrorCode(200003);
    // 插件错误
    pub const PLUGIN_NOT_FOUND: ErrorCode = ErrorCode(300001);
    pub const PLUGIN_LOAD_FAILED: ErrorCode = ErrorCode(300002);
    pub const PLUGIN_PERMISSION_DENIED: ErrorCode = ErrorCode(300003);
    // 政企错误
    pub const ENTERPRISE_SSO_FAILED: ErrorCode = ErrorCode(400001);
    pub const ENTERPRISE_COMPLIANCE_VIOLATION: ErrorCode = ErrorCode(400002);
    // 连接器错误
    pub const CONNECTOR_NOT_FOUND: ErrorCode = ErrorCode(500001);
    pub const CONNECTOR_CONNECTION_FAILED: ErrorCode = ErrorCode(500002);
    pub const CONNECTOR_TIMEOUT: ErrorCode = ErrorCode(500003);
    // 集成错误
    pub const INTEGRATION_CONFIG_ERROR: ErrorCode = ErrorCode(900001);
    pub const INTEGRATION_FACTORY_NOT_FOUND: ErrorCode = ErrorCode(900002);
    pub const INTEGRATION_ASSEMBLY_FAILED: ErrorCode = ErrorCode(900003);
}

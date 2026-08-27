// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 统一错误类型 + 企业级错误码体系

use serde::{Deserialize, Serialize};
use std::fmt;

/// 企业级错误码（7位数字：2位模块+3位子码+2位严重度）
pub type ErrorCode = u32;

/// 错误严重度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Severity {
    Info = 0,
    Warning = 1,
    Error = 2,
    Critical = 3,
}

/// 框架统一错误
#[derive(Debug, thiserror::Error)]
#[error("[{code}] {message}")]
pub struct FrameworkError {
    pub code: ErrorCode,
    pub message: String,
    pub severity: Severity,
    pub source_text: Option<String>,
    pub details: Option<serde_json::Value>,
}

impl FrameworkError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            severity: Severity::Error,
            source_text: None,
            details: None,
        }
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source_text = Some(source.into());
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// 转换为 HTTP 状态码
    pub fn http_status(&self) -> u16 {
        match self.code / 10000 {
            10 => 400, // 验证错误
            11 => 401, // 认证错误
            12 => 403, // 授权错误
            13 => 404, // 未找到
            14 => 409, // 冲突
            15 => 429, // 限流
            20 => 500, // 内部错误
            21 => 502, // 网关错误
            22 => 503, // 服务不可用
            23 => 504, // 超时
            _ => 500,
        }
    }
}



// 常用错误构造函数
impl FrameworkError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::new(100002, msg).with_severity(Severity::Warning)
    }
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::new(110002, msg).with_severity(Severity::Warning)
    }
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::new(120002, msg).with_severity(Severity::Warning)
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new(130002, msg).with_severity(Severity::Warning)
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::new(140002, msg).with_severity(Severity::Warning)
    }
    pub fn rate_limited(msg: impl Into<String>) -> Self {
        Self::new(150002, msg).with_severity(Severity::Warning)
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(200003, msg).with_severity(Severity::Error)
    }
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::new(230003, msg).with_severity(Severity::Error)
    }
    pub fn unavailable(msg: impl Into<String>) -> Self {
        Self::new(220003, msg).with_severity(Severity::Critical)
    }
}

impl From<std::io::Error> for FrameworkError {
    fn from(e: std::io::Error) -> Self {
        Self::internal(format!("IO error: {}", e))
    }
}

impl From<serde_json::Error> for FrameworkError {
    fn from(e: serde_json::Error) -> Self {
        Self::validation(format!("JSON error: {}", e))
    }
}

impl From<config::ConfigError> for FrameworkError {
    fn from(e: config::ConfigError) -> Self {
        Self::internal(format!("Config error: {}", e))
    }
}

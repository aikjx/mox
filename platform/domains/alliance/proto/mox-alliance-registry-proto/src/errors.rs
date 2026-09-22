// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 注册中心契约层错误类型

use std::fmt;

/// 注册中心契约错误：包装存储层/参数校验失败的可移植描述。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryError {
    pub message: String,
}

impl RegistryError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "registry error: {}", self.message)
    }
}

impl std::error::Error for RegistryError {}

pub type RegistryResult<T> = Result<T, RegistryError>;

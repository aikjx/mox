// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! RBAC 错误类型

use std::fmt;

/// RBAC 引擎错误
#[derive(Debug)]
pub enum RbacError {
    /// 角色未定义
    RoleNotFound(String),
    /// 循环继承检测
    CyclicInheritance(String),
    /// 策略加载失败
    PolicyLoadFailed(String),
    /// 审计写入失败（不阻断权限检查，仅记录）
    AuditWriteFailed(String),
}

impl fmt::Display for RbacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RoleNotFound(r) => write!(f, "RBAC: role not found '{r}'"),
            Self::CyclicInheritance(r) => {
                write!(f, "RBAC: cyclic inheritance detected in role '{r}'")
            }
            Self::PolicyLoadFailed(msg) => write!(f, "RBAC: policy load failed — {msg}"),
            Self::AuditWriteFailed(msg) => {
                write!(f, "RBAC: audit write failed — {msg} (non-blocking)")
            }
        }
    }
}

impl std::error::Error for RbacError {}

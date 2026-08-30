// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! IAM 标准策略评估上下文类型

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 条件键（用于 P7/P8/P9/P10）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvalContext {
    pub principal: String,
    pub action: String,
    pub resource: String,
    /// 桶所有者 (用于 P2 BucketOwnerFull 隐式授权)
    pub bucket_owner: Option<String>,
    /// MFA 是否已通过
    pub mfa_authenticated: Option<bool>,
    /// 源 IP
    pub source_ip: Option<String>,
    /// 请求携带标签（k→v）
    pub tags: BTreeMap<String, String>,
    /// 是否来自 VPC / 内网
    pub from_vpc: Option<bool>,
}

// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家数据模型（registry-svc 内部简化版）

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 专家
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expert {
    /// 专家 ID
    pub id: String,
    /// 专家名称
    pub name: String,
    /// 专家标题
    pub title: String,
    /// 所属组织
    pub organization: String,
    /// 领域标签
    pub domains: Vec<String>,
    /// 技能标签
    pub skills: Vec<String>,
    /// 简介
    pub bio: String,
    /// 是否启用
    pub enabled: bool,
    /// 评分
    pub rating: f32,
    /// 总咨询次数
    pub total_consultations: u32,
}

impl Expert {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            title: String::new(),
            organization: String::new(),
            domains: Vec::new(),
            skills: Vec::new(),
            bio: String::new(),
            enabled: true,
            rating: 0.0,
            total_consultations: 0,
        }
    }
}

/// 创建专家请求
#[derive(Debug, Deserialize)]
pub struct CreateExpertRequest {
    pub name: String,
    pub title: Option<String>,
    pub organization: Option<String>,
    pub domains: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
    pub bio: Option<String>,
}

/// 更新专家请求
#[derive(Debug, Deserialize)]
pub struct UpdateExpertRequest {
    pub name: Option<String>,
    pub title: Option<String>,
    pub organization: Option<String>,
    pub domains: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
    pub bio: Option<String>,
    pub enabled: Option<bool>,
}

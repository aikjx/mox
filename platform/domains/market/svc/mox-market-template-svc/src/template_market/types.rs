// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 模板市场核心类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// 模板业务域标签（可无限扩展 → "所有模块都是通用的"）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Domain {
    Mall,          // 商城
    Novel,         // 小说
    Thesis,        // 论文
    Book,          // 图书/出版
    VideoDesign,   // 影视设计
    ProductDesign, // 产品设计
    SystemDesign,  // 系统设计
    Other(String),
}

impl Domain {
    pub fn as_str(&self) -> String {
        match self {
            Domain::Mall => "mall".into(),
            Domain::Novel => "novel".into(),
            Domain::Thesis => "thesis".into(),
            Domain::Book => "book".into(),
            Domain::VideoDesign => "video_design".into(),
            Domain::ProductDesign => "product_design".into(),
            Domain::SystemDesign => "system_design".into(),
            Domain::Other(s) => s.clone(),
        }
    }
    pub fn parse(s: &str) -> Domain {
        match s {
            "mall" => Domain::Mall,
            "novel" => Domain::Novel,
            "thesis" => Domain::Thesis,
            "book" => Domain::Book,
            "video_design" => Domain::VideoDesign,
            "product_design" => Domain::ProductDesign,
            "system_design" => Domain::SystemDesign,
            other => Domain::Other(other.to_string()),
        }
    }
}

/// 单个系统模板（草莓多平台的核心资产）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub domains: Vec<Domain>,
    /// 由对话/璇玑生成的流程图（业务功能 + 关联关系）
    pub graph_json: serde_json::Value,
    /// 由 codegen 生成的代码包（后端/DB/前端）；可留空待后续生成
    pub artifacts: BTreeMap<String, String>,
    /// 引用链：本模板派生自哪个模板（"引用下载"二开）
    pub derived_from: Option<String>,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// 复用次数（持续学习信号）
    pub reuse_count: u64,
    /// 平均评分 0..5（持续学习信号）
    pub rating: f32,
}

impl SystemTemplate {
    pub fn new(
        name: &str,
        description: &str,
        domains: Vec<Domain>,
        graph_json: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            domains,
            graph_json,
            artifacts: BTreeMap::new(),
            derived_from: None,
            version: 1,
            created_at: now,
            updated_at: now,
            reuse_count: 0,
            rating: 0.0,
        }
    }

    /// 绑定生成的代码包（后端/DB/前端）
    pub fn with_artifacts(mut self, artifacts: BTreeMap<String, String>) -> Self {
        self.artifacts = artifacts;
        self
    }

    /// "引用下载"：派生一个子模板，继承父模板的图与代码，重置评分/复用
    pub fn fork(&self, new_name: &str, new_description: &str) -> SystemTemplate {
        let mut child = SystemTemplate::new(
            new_name,
            new_description,
            self.domains.clone(),
            self.graph_json.clone(),
        );
        child.artifacts = self.artifacts.clone();
        child.derived_from = Some(self.id.clone());
        child.version = 1;
        child
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MarketError {
    #[error("模板不存在: {0}")]
    NotFound(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("通用错误: {0}")]
    Anyhow(#[from] anyhow::Error),
}

//! 数据模型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 内容数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// 内容标题
    pub title: String,
    /// 内容正文
    pub body: String,
    /// 内容分类
    pub category: String,
    /// 标签列表
    #[serde(default)]
    pub tags: Vec<String>,
    /// 作者
    #[serde(default)]
    pub author: String,
    /// 摘要
    #[serde(default)]
    pub summary: String,
    /// 封面图URL
    #[serde(default)]
    pub cover_image: String,
    /// 自定义元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// 内容ID（发布后回填）
    #[serde(default)]
    pub content_id: Option<String>,
}

impl Content {
    /// 创建新内容
    pub fn new(title: impl Into<String>, body: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            category: category.into(),
            tags: Vec::new(),
            author: String::new(),
            summary: String::new(),
            cover_image: String::new(),
            metadata: HashMap::new(),
            content_id: None,
        }
    }

    /// 设置作者
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    /// 设置标签
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// 设置摘要
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    /// 设置封面图
    pub fn with_cover_image(mut self, url: impl Into<String>) -> Self {
        self.cover_image = url.into();
        self
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// 验证内容完整性
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() {
            return Err("标题不能为空".into());
        }
        if self.body.trim().is_empty() {
            return Err("正文不能为空".into());
        }
        if self.category.trim().is_empty() {
            return Err("分类不能为空".into());
        }
        Ok(())
    }
}

/// 发布平台配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishPlatform {
    /// 平台ID（对应Connector ID）
    pub connector_id: String,
    /// 平台名称
    pub name: String,
    /// 平台类型
    pub platform_type: PlatformType,
    /// 发布操作名称
    pub publish_operation: String,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 超时时间（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_true() -> bool { true }
fn default_timeout() -> u64 { 30 }

/// 平台类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlatformType {
    /// 内容管理系统
    Cms,
    /// 电商平台
    Ecommerce,
    /// 社交媒体
    Social,
    /// 消息通知
    Notification,
    /// 自定义平台
    Custom,
}

impl PlatformType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlatformType::Cms => "cms",
            PlatformType::Ecommerce => "ecommerce",
            PlatformType::Social => "social",
            PlatformType::Notification => "notification",
            PlatformType::Custom => "custom",
        }
    }

    /// 获取默认发布操作
    pub fn default_operation(&self) -> &'static str {
        match self {
            PlatformType::Cms => "publish_content",
            PlatformType::Ecommerce => "post",
            PlatformType::Social => "post",
            PlatformType::Notification => "post",
            PlatformType::Custom => "publish",
        }
    }
}

/// 发布状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PublishStatus {
    /// 发布成功
    Success,
    /// 发布失败
    Failed,
    /// 发布中
    Publishing,
    /// 待发布
    Pending,
    /// 已跳过
    Skipped,
}

/// 单个平台发布结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    /// 平台连接器ID
    pub connector_id: String,
    /// 平台名称
    pub platform_name: String,
    /// 发布状态
    pub status: PublishStatus,
    /// 发布后内容ID（成功时）
    #[serde(default)]
    pub content_id: String,
    /// 发布后URL（成功时）
    #[serde(default)]
    pub url: String,
    /// 错误信息（失败时）
    #[serde(default)]
    pub error: Option<String>,
    /// 耗时（毫秒）
    pub latency_ms: u64,
    /// 发布时间
    pub published_at: String,
    /// 追踪ID
    #[serde(default)]
    pub trace_id: Option<String>,
    /// 重试次数
    #[serde(default)]
    pub retries: u32,
}

impl PublishResult {
    /// 创建成功结果
    pub fn success(
        connector_id: impl Into<String>,
        platform_name: impl Into<String>,
        content_id: impl Into<String>,
        latency_ms: u64,
    ) -> Self {
        Self {
            connector_id: connector_id.into(),
            platform_name: platform_name.into(),
            status: PublishStatus::Success,
            content_id: content_id.into(),
            url: String::new(),
            error: None,
            latency_ms,
            published_at: chrono::Utc::now().to_rfc3339(),
            trace_id: None,
            retries: 0,
        }
    }

    /// 创建失败结果
    pub fn failed(
        connector_id: impl Into<String>,
        platform_name: impl Into<String>,
        error: impl Into<String>,
        latency_ms: u64,
    ) -> Self {
        Self {
            connector_id: connector_id.into(),
            platform_name: platform_name.into(),
            status: PublishStatus::Failed,
            content_id: String::new(),
            url: String::new(),
            error: Some(error.into()),
            latency_ms,
            published_at: chrono::Utc::now().to_rfc3339(),
            trace_id: None,
            retries: 0,
        }
    }

    /// 创建跳过结果
    pub fn skipped(connector_id: impl Into<String>, platform_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            connector_id: connector_id.into(),
            platform_name: platform_name.into(),
            status: PublishStatus::Skipped,
            content_id: String::new(),
            url: String::new(),
            error: Some(reason.into()),
            latency_ms: 0,
            published_at: chrono::Utc::now().to_rfc3339(),
            trace_id: None,
            retries: 0,
        }
    }

    /// 是否成功
    pub fn is_success(&self) -> bool {
        matches!(self.status, PublishStatus::Success)
    }

    /// 设置追踪ID
    pub fn with_trace_id(mut self, trace_id: Option<String>) -> Self {
        self.trace_id = trace_id;
        self
    }
}

/// 发布汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishSummary {
    /// 总平台数
    pub total: usize,
    /// 成功数
    pub success: usize,
    /// 失败数
    pub failed: usize,
    /// 跳过数
    pub skipped: usize,
    /// 总耗时（毫秒）
    pub total_latency_ms: u64,
    /// 平均耗时（毫秒）
    pub avg_latency_ms: u64,
    /// 成功率（百分比）
    pub success_rate: f64,
    /// 是否全部成功
    pub all_success: bool,
}

impl PublishSummary {
    /// 从结果列表计算汇总
    pub fn from_results(results: &[PublishResult]) -> Self {
        let total = results.len();
        let success = results.iter().filter(|r| r.is_success()).count();
        let failed = results.iter().filter(|r| matches!(r.status, PublishStatus::Failed)).count();
        let skipped = results.iter().filter(|r| matches!(r.status, PublishStatus::Skipped)).count();
        let total_latency: u64 = results.iter().map(|r| r.latency_ms).sum();
        let avg_latency = if total > 0 { total_latency / total as u64 } else { 0 };
        let success_rate = if total > 0 { (success as f64 / total as f64) * 100.0 } else { 0.0 };

        Self {
            total,
            success,
            failed,
            skipped,
            total_latency_ms: total_latency,
            avg_latency_ms: avg_latency,
            success_rate,
            all_success: failed == 0 && skipped == 0 && total > 0,
        }
    }
}

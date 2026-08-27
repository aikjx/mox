//! REST API端点定义

use crate::model::*;
use crate::publisher::ContentPublisher;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 发布内容请求
#[derive(Debug, Deserialize)]
pub struct PublishContentRequest {
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
    pub metadata: std::collections::HashMap<String, String>,
    /// 指定发布平台（为空则发布到所有已启用平台）
    #[serde(default)]
    pub platforms: Vec<String>,
}

/// 发布内容响应
#[derive(Debug, Serialize)]
pub struct PublishContentResponse {
    /// 是否全部成功
    pub success: bool,
    /// 追踪ID
    pub trace_id: String,
    /// 各平台发布结果
    pub results: Vec<PublishResultDto>,
    /// 发布汇总
    pub summary: PublishSummaryDto,
}

/// 发布结果DTO
#[derive(Debug, Serialize)]
pub struct PublishResultDto {
    /// 平台连接器ID
    pub connector_id: String,
    /// 平台名称
    pub platform_name: String,
    /// 发布状态
    pub status: PublishStatus,
    /// 内容ID
    pub content_id: String,
    /// 发布URL
    pub url: String,
    /// 错误信息
    pub error: Option<String>,
    /// 耗时（毫秒）
    pub latency_ms: u64,
    /// 重试次数
    pub retries: u32,
}

/// 发布汇总DTO
#[derive(Debug, Serialize)]
pub struct PublishSummaryDto {
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

impl From<&PublishResult> for PublishResultDto {
    fn from(r: &PublishResult) -> Self {
        Self {
            connector_id: r.connector_id.clone(),
            platform_name: r.platform_name.clone(),
            status: r.status,
            content_id: r.content_id.clone(),
            url: r.url.clone(),
            error: r.error.clone(),
            latency_ms: r.latency_ms,
            retries: r.retries,
        }
    }
}

impl From<&PublishSummary> for PublishSummaryDto {
    fn from(s: &PublishSummary) -> Self {
        Self {
            total: s.total,
            success: s.success,
            failed: s.failed,
            skipped: s.skipped,
            total_latency_ms: s.total_latency_ms,
            avg_latency_ms: s.avg_latency_ms,
            success_rate: s.success_rate,
            all_success: s.all_success,
        }
    }
}

/// 发布内容API处理函数
pub async fn publish_content(
    State(publisher): State<Arc<ContentPublisher>>,
    Json(request): Json<PublishContentRequest>,
) -> Result<Json<PublishContentResponse>, (StatusCode, Json<serde_json::Value>)> {
    let trace_id = current_trace_id();

    // 构建内容
    let mut content = Content::new(request.title, request.body, request.category)
        .with_author(request.author)
        .with_tags(request.tags)
        .with_summary(request.summary)
        .with_cover_image(request.cover_image);

    for (k, v) in request.metadata {
        content = content.with_metadata(k, v);
    }

    // 验证内容
    if let Err(e) = content.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": {
                    "code": "E400001",
                    "message": format!("内容验证失败: {}", e),
                    "trace_id": trace_id,
                }
            })),
        ));
    }

    // 执行发布
    let results = if request.platforms.is_empty() {
        publisher.publish_to_all(&content).await
    } else {
        let platform_ids: Vec<&str> = request.platforms.iter().map(|s| s.as_str()).collect();
        publisher.publish_to_platforms(&content, &platform_ids).await
    };

    // 计算汇总
    let summary = PublishSummary::from_results(&results);

    // 构建响应
    let response = PublishContentResponse {
        success: summary.all_success,
        trace_id,
        results: results.iter().map(PublishResultDto::from).collect(),
        summary: PublishSummaryDto::from(&summary),
    };

    Ok(Json(response))
}

/// 获取发布平台列表API
pub async fn list_platforms(
    State(publisher): State<Arc<ContentPublisher>>,
) -> Json<serde_json::Value> {
    let platforms = publisher.list_all_platforms();
    Json(serde_json::json!({
        "platforms": platforms,
        "total": platforms.len(),
        "enabled": platforms.iter().filter(|p| p.enabled).count(),
    }))
}

/// 健康检查API
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "mox-content-publisher",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// 构建发布器的Axum Router
pub fn router(publisher: Arc<ContentPublisher>) -> axum::Router {
    axum::Router::new()
        .route("/publish", axum::routing::post(publish_content))
        .route("/platforms", axum::routing::get(list_platforms))
        .route("/health", axum::routing::get(health_check))
        .with_state(publisher)
}

/// 获取当前trace_id（简化实现，实际应从mox-platform-integration-core导入）
fn current_trace_id() -> String {
    // 尝试从集成层获取trace_id，失败则生成新的
    uuid::Uuid::new_v4().to_string()
}

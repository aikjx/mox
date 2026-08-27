//! 内容发布服务核心实现

use crate::model::*;
use mox_connector_core::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task;

/// 内容发布器
pub struct ContentPublisher {
    /// 连接器注册表
    connector_registry: Arc<ConnectorRegistry>,
    /// 平台配置列表
    platforms: parking_lot::RwLock<Vec<PublishPlatform>>,
}

impl ContentPublisher {
    /// 创建新的发布器
    pub fn new(connector_registry: Arc<ConnectorRegistry>) -> Self {
        Self {
            connector_registry,
            platforms: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// 创建带默认平台配置的发布器
    pub fn with_default_platforms(connector_registry: Arc<ConnectorRegistry>) -> Self {
        let publisher = Self::new(connector_registry);
        publisher.add_default_platforms();
        publisher
    }

    /// 添加默认平台配置
    fn add_default_platforms(&self) {
        let defaults = vec![
            PublishPlatform {
                connector_id: "cms-grpc".into(),
                name: "企业CMS系统".into(),
                platform_type: PlatformType::Cms,
                publish_operation: "publish_content".into(),
                enabled: true,
                timeout_secs: 30,
            },
            PublishPlatform {
                connector_id: "ecommerce-rest".into(),
                name: "电商平台".into(),
                platform_type: PlatformType::Ecommerce,
                publish_operation: "post".into(),
                enabled: true,
                timeout_secs: 15,
            },
            PublishPlatform {
                connector_id: "social-webhook".into(),
                name: "社交媒体".into(),
                platform_type: PlatformType::Social,
                publish_operation: "post".into(),
                enabled: true,
                timeout_secs: 10,
            },
        ];
        *self.platforms.write() = defaults;
    }

    /// 注册发布平台
    pub fn register_platform(&self, platform: PublishPlatform) {
        tracing::info!(
            connector_id = %platform.connector_id,
            name = %platform.name,
            "register publish platform"
        );
        let mut platforms = self.platforms.write();
        platforms.retain(|p| p.connector_id != platform.connector_id);
        platforms.push(platform);
    }

    /// 移除发布平台
    pub fn unregister_platform(&self, connector_id: &str) -> bool {
        let mut platforms = self.platforms.write();
        let len = platforms.len();
        platforms.retain(|p| p.connector_id != connector_id);
        platforms.len() != len
    }

    /// 获取所有已启用平台
    pub fn list_enabled_platforms(&self) -> Vec<PublishPlatform> {
        self.platforms.read()
            .iter()
            .filter(|p| p.enabled)
            .cloned()
            .collect()
    }

    /// 获取所有平台
    pub fn list_all_platforms(&self) -> Vec<PublishPlatform> {
        self.platforms.read().clone()
    }

    /// 发布内容到单个平台
    pub async fn publish_to_platform(&self, content: &Content, platform: &PublishPlatform) -> PublishResult {
        let trace_id = current_trace_id();
        let start = std::time::Instant::now();

        // 验证内容
        if let Err(e) = content.validate() {
            return PublishResult::failed(
                &platform.connector_id,
                &platform.name,
                format!("content validation failed: {}", e),
                0,
            ).with_trace_id(trace_id.clone());
        }

        // 获取连接器
        let connector = match self.connector_registry.get(&platform.connector_id) {
            Ok(c) => c,
            Err(_) => {
                return PublishResult::skipped(
                    &platform.connector_id,
                    &platform.name,
                    format!("connector '{}' not registered", platform.connector_id),
                ).with_trace_id(trace_id.clone());
            }
        };

        // 构建请求体
        let body = self.build_publish_params(content, platform);

        // 构建连接器请求
        let request = ConnectorRequest {
            operation: platform.publish_operation.clone(),
            body,
            params: HashMap::new(),
            headers: HashMap::new(),
            trace_id: trace_id.clone(),
            tenant_id: None,
        };

        // 执行发布
        let result = match connector.execute(&request).await {
            Ok(response) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                if response.success {
                    let content_id = response.body.get("content_id")
                        .or_else(|| response.body.get("id"))
                        .or_else(|| response.body.get("product_id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let url = response.body.get("url")
                        .or_else(|| response.body.get("link"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let mut result = PublishResult::success(
                        &platform.connector_id,
                        &platform.name,
                        content_id,
                        latency_ms,
                    );
                    result.url = url;
                    result.retries = response.retries;
                    result.with_trace_id(trace_id.clone())
                } else {
                    PublishResult::failed(
                        &platform.connector_id,
                        &platform.name,
                        response.error.unwrap_or_else(|| format!("HTTP {}", response.status_code)),
                        latency_ms,
                    ).with_trace_id(trace_id.clone())
                }
            }
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                PublishResult::failed(
                    &platform.connector_id,
                    &platform.name,
                    e.to_string(),
                    latency_ms,
                ).with_trace_id(trace_id.clone())
            }
        };

        // 记录发布日志
        if result.is_success() {
            tracing::info!(
                platform = %platform.name,
                content_id = %result.content_id,
                latency_ms = result.latency_ms,
                "content published successfully"
            );
        } else {
            tracing::error!(
                platform = %platform.name,
                error = ?result.error,
                latency_ms = result.latency_ms,
                "content publish failed"
            );
        }

        result
    }

    /// 并发发布到所有平台
    pub async fn publish_to_all(&self, content: &Content) -> Vec<PublishResult> {
        let platforms = self.list_enabled_platforms();
        self.publish_to_platforms_internal(content, &platforms).await
    }

    /// 发布到指定平台列表
    pub async fn publish_to_platforms(&self, content: &Content, connector_ids: &[&str]) -> Vec<PublishResult> {
        let all_platforms = self.list_enabled_platforms();
        let selected: Vec<PublishPlatform> = all_platforms
            .into_iter()
            .filter(|p| connector_ids.contains(&p.connector_id.as_str()))
            .collect();
        self.publish_to_platforms_internal(content, &selected).await
    }

    /// 内部并发发布实现
    async fn publish_to_platforms_internal(&self, content: &Content, platforms: &[PublishPlatform]) -> Vec<PublishResult> {
        if platforms.is_empty() {
            tracing::warn!("no publish platforms configured");
            return Vec::new();
        }

        tracing::info!(
            title = %content.title,
            platform_count = platforms.len(),
            "starting concurrent content publish"
        );

        let mut handles = Vec::new();

        for platform in platforms {
            let content = content.clone();
            let platform = platform.clone();
            let connector_registry = self.connector_registry.clone();

            let handle = task::spawn(async move {
                let publisher = ContentPublisher::new(connector_registry);
                publisher.publish_to_platform(&content, &platform).await
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::error!(error = %e, "publish task panicked");
                    results.push(PublishResult::failed(
                        "unknown",
                        "unknown platform",
                        format!("task panic: {}", e),
                        0,
                    ));
                }
            }
        }

        let summary = PublishSummary::from_results(&results);
        tracing::info!(
            total = summary.total,
            success = summary.success,
            failed = summary.failed,
            success_rate = summary.success_rate,
            total_latency_ms = summary.total_latency_ms,
            "content publish completed"
        );

        results
    }

    /// 发布并返回汇总
    pub async fn publish_with_summary(&self, content: &Content) -> (Vec<PublishResult>, PublishSummary) {
        let results = self.publish_to_all(content).await;
        let summary = PublishSummary::from_results(&results);
        (results, summary)
    }

    /// 构建发布参数（根据平台类型）
    fn build_publish_params(&self, content: &Content, platform: &PublishPlatform) -> serde_json::Value {
        match platform.platform_type {
            PlatformType::Cms => serde_json::json!({
                "title": content.title,
                "content": content.body,
                "category": content.category,
                "tags": content.tags,
                "author": content.author,
                "summary": content.summary,
                "cover_image": content.cover_image,
                "metadata": content.metadata,
            }),
            PlatformType::Ecommerce => serde_json::json!({
                "path": "/api/v1/products",
                "body": {
                    "name": content.title,
                    "description": content.body,
                    "category": content.category,
                    "tags": content.tags,
                    "summary": content.summary,
                    "images": if content.cover_image.is_empty() { Vec::<String>::new() } else { vec![content.cover_image.clone()] },
                    "metadata": content.metadata,
                }
            }),
            PlatformType::Social => serde_json::json!({
                "path": "/",
                "body": {
                    "text": if content.summary.is_empty() {
                        format!("{}\n\n{}", content.title, content.body)
                    } else {
                        content.summary.clone()
                    },
                    "title": content.title,
                    "tags": content.tags,
                    "cover_image": content.cover_image,
                }
            }),
            PlatformType::Notification => serde_json::json!({
                "path": "/notify",
                "body": {
                    "title": content.title,
                    "content": content.summary,
                    "category": content.category,
                    "tags": content.tags,
                }
            }),
            PlatformType::Custom => serde_json::json!({
                "title": content.title,
                "content": content.body,
                "category": content.category,
                "tags": content.tags,
                "author": content.author,
                "metadata": content.metadata,
            }),
        }
    }

    /// 获取连接器注册表引用
    pub fn connector_registry(&self) -> &Arc<ConnectorRegistry> {
        &self.connector_registry
    }
}

/// 获取当前trace_id
fn current_trace_id() -> Option<String> {
    Some(uuid::Uuid::new_v4().to_string())
}

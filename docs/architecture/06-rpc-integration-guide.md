# RPC快速对接手册 — 新系统接入指南

> **版本**: 1.0.0
> **适用场景**: 内容发布 / 系统集成 / 第三方对接
> **对接方式**: gRPC / REST / WebSocket（统一ProtocolHandler抽象）
> **核心原则**: 配置驱动 + 零改动核心架构 + 5分钟快速对接

---

## 目录

1. [概述](#1-概述)
2. [架构设计](#2-架构设计)
3. [前置准备](#3-前置准备)
4. [5步快速对接流程](#4-5步快速对接流程)
5. [gRPC对接详解](#5-grpc对接详解)
6. [REST对接详解](#6-rest对接详解)
7. [内容发布场景完整示例](#7-内容发布场景完整示例)
8. [配置驱动零代码对接](#8-配置驱动零代码对接)
9. [高级特性](#9-高级特性)
10. [最佳实践](#10-最佳实践)
11. [故障排查](#11-故障排查)
12. [附录](#12-附录)

---

## 1. 概述

### 1.1 什么是RPC快速对接

RPC（Remote Procedure Call，远程过程调用）快速对接是指通过**统一连接器框架（Connector Framework）+ 多协议网关（Protocol Gateway）**，在**不修改核心代码**的前提下，快速将新系统接入Mox Platform，实现内容发布、数据同步、服务调用等跨系统交互。

### 1.2 适用场景

| 场景 | 说明 | 推荐协议 |
|------|------|---------|
| **内容发布** | 将内容（文章/视频/商品）发布到第三方CMS/电商/社交平台 | gRPC / REST |
| **系统集成** | 与企业内部ERP/CRM/OA系统对接 | gRPC |
| **数据同步** | 定时/实时同步数据到外部系统 | REST / Webhook |
| **服务调用** | 调用外部AI/搜索/推荐服务 | gRPC / REST |
| **事件通知** | 将系统事件推送到外部消息队列 | WebSocket / Webhook |

### 1.3 核心优势

- **5分钟对接**: 配置 + 注册 = 完成对接
- **零改动核心**: 新增系统不需要修改平台核心代码
- **多协议统一**: gRPC/REST/WebSocket统一抽象，切换协议零成本
- **企业级保障**: 限流/熔断/重试/追踪/审计全链路覆盖
- **配置驱动**: 纯配置即可完成对接，无需写代码（内置Connector）

---

## 2. 架构设计

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Mox Platform                                  │
│                                                                       │
│  ┌─────────────┐    ┌──────────────────────────────────────────┐   │
│  │  业务模块    │───▶│  L5 集成层 (mox-platform-integration-core) │   │
│  │ (内容发布)   │    │                                          │   │
│  └─────────────┘    │  ┌──────────────┐  ┌──────────────┐   │   │
│                       │  │ Connector    │  │ Protocol     │   │   │
│                       │  │ Framework    │  │ Gateway      │   │   │
│                       │  │ (连接器框架)  │  │ (多协议网关)  │   │   │
│                       │  └──────┬───────┘  └──────┬───────┘   │   │
│                       └─────────┼───────────────────┼───────────┘   │
│                                 │                   │               │
│                    ┌────────────▼───────────┐  ┌───▼──────────┐   │
│                    │  Connector Registry     │  │ Protocol     │   │
│                    │  (连接器注册表)          │  │ Router       │   │
│                    │                          │  │ (协议路由器)  │   │
│                    │  - WebhookConnector     │  │              │   │
│                    │  - GrpcConnector        │  │  - gRPC      │   │
│                    │  - RestConnector        │  │  - REST      │   │
│                    │  - CustomConnector...   │  │  - WebSocket │   │
│                    └────────────┬───────────┘  └───┬──────────┘   │
│                                 │                   │               │
└─────────────────────────────────┼───────────────────┼───────────────┘
                                  │                   │
                    ┌─────────────▼─────┐  ┌────────▼──────────┐
                    │  新系统A (gRPC)    │  │  新系统B (REST)    │
                    │  - CMS内容管理      │  │  - 电商平台         │
                    │  - 企业ERP          │  │  - 社交平台         │
                    └────────────────────┘  └────────────────────┘
```

### 2.2 核心组件

| 组件 | Crate | 职责 |
|------|-------|------|
| **Connector Trait** | `mox-connector-core` | 连接器抽象接口，定义统一的连接/执行/健康检查 |
| **Connector Registry** | `mox-connector-core` | 连接器实例注册表，管理所有已注册连接器 |
| **Connector Factory** | `mox-platform-integration-core` | 从配置创建连接器实例，实现配置驱动自动组装 |
| **Protocol Handler** | `mox-platform-integration-core` | 协议处理器抽象，支持gRPC/REST/WebSocket等 |
| **Protocol Router** | `mox-platform-integration-core` | 协议路由器，按协议+路径路由到对应处理器 |
| **Grpc Service Registry** | `mox-platform-integration-core` | gRPC服务注册表，管理gRPC服务发现 |

### 2.3 对接模式

```
模式1: 平台调用新系统 (Outbound)
  业务模块 → Connector → 新系统API

模式2: 新系统调用平台 (Inbound)
  新系统 → Protocol Gateway → Protocol Handler → 业务模块

模式3: 双向对接 (Bidirectional)
  平台 ↔ Connector/Protocol ↔ 新系统
```

---

## 3. 前置准备

### 3.1 环境要求

| 组件 | 版本要求 | 说明 |
|------|---------|------|
| Rust | 1.75+ | 编译工具链 |
| tokio | 1.0+ | 异步运行时（已内置） |
| axum | 0.7+ | Web框架（已内置） |
| tonic | 0.11+ | gRPC框架（可选，gRPC对接需要） |
| prost | 0.12+ | Protobuf编译（可选，gRPC对接需要） |

### 3.2 依赖配置

在需要对接的crate的`Cargo.toml`中添加：

```toml
[dependencies]
# 连接器核心（必须）
mox-connector-core = { workspace = true }

# 集成层（必须，含Factory和Protocol）
mox-platform-integration-core = { workspace = true }

# gRPC支持（可选，gRPC对接需要）
tonic = "0.11"
prost = "0.12"

# 异步trait（必须）
async-trait = "0.1"

# 序列化（必须）
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### 3.3 获取新系统信息

对接前需要收集新系统的以下信息：

| 信息项 | 说明 | 示例 |
|--------|------|------|
| **系统名称** | 新系统的名称 | "企业CMS系统" |
| **对接协议** | gRPC / REST / WebSocket | "gRPC" |
| **服务地址** | 新系统的访问地址 | "cms.internal:50051" |
| **API定义** | Protobuf文件 / OpenAPI文档 | "cms.proto" |
| **认证方式** | Bearer Token / API Key / OAuth2 / mTLS | "Bearer Token" |
| **认证凭证** | Token / Key / Secret | (安全存储) |
| **超时设置** | 请求超时时间 | "30s" |
| **重试策略** | 重试次数 / 重试间隔 | "3次，间隔1s" |
| **限流配置** | QPS限制 | "100 QPS" |

---

## 4. 5步快速对接流程

### 步骤总览

```
第1步: 定义连接器配置 (YAML)
    ↓
第2步: 实现Connector Trait (如需要自定义协议)
    ↓
第3步: 实现Connector Factory (如需要自定义协议)
    ↓
第4步: 注册Factory + 加载配置
    ↓
第5步: 调用连接器执行操作
```

### 第1步: 定义连接器配置

在`config/integration.yaml`中添加新系统的连接器配置：

```yaml
connector:
  enabled: true
  global_timeout_secs: 30
  global_max_retries: 2

  connectors:
    # 新系统: 企业CMS (gRPC)
    - id: cms-grpc
      name: 企业CMS系统
      connector_type: grpc              # 连接器类型
      protocol: grpc                     # 协议类型
      endpoint: cms.internal:50051       # 服务地址
      auth_type: bearer                  # 认证方式
      credentials:
        token: ${CMS_GRPC_TOKEN}         # 从环境变量读取
      timeout_secs: 30
      max_retries: 3
      retry_interval_ms: 1000
      rate_limit:
        enabled: true
        max_qps: 100
      enabled: true
      metadata:
        system_owner: 内容运营部
        contact: cms-team@company.com

    # 新系统: 电商平台 (REST)
    - id: ecommerce-rest
      name: 电商平台
      connector_type: rest
      protocol: rest
      endpoint: https://api.ecommerce.com
      auth_type: api_key
      credentials:
        api_key: ${ECOMMERCE_API_KEY}
        api_key_header: X-API-Key
      timeout_secs: 15
      max_retries: 2
      enabled: true
```

### 第2步: 实现Connector Trait（自定义协议需要）

> **注意**: 如果使用内置的`grpc`/`rest`/`webhook`连接器类型，可跳过此步。
> 只有当新系统使用**自定义协议**时，才需要实现Connector Trait。

```rust
// src/connectors/cms_grpc_connector.rs
use async_trait::async_trait;
use mox_connector_core::prelude::*;
use std::collections::HashMap;

/// 企业CMS gRPC连接器
pub struct CmsGrpcConnector {
    config: ConnectorConfig,
    // gRPC客户端（实际使用tonic生成的客户端）
    // client: CmsClient<Channel>,
}

impl CmsGrpcConnector {
    pub fn new(config: ConnectorConfig) -> Self {
        Self {
            config,
            // client: ... (初始化gRPC客户端)
        }
    }
}

#[async_trait]
impl Connector for CmsGrpcConnector {
    fn connector_id(&self) -> &str {
        &self.config.id
    }

    fn connector_name(&self) -> &str {
        &self.config.name
    }

    fn connector_type(&self) -> ConnectorType {
        ConnectorType::Grpc  // 自定义类型
    }

    fn supported_protocols(&self) -> Vec<String> {
        vec!["grpc".into()]
    }

    fn supported_operations(&self) -> Vec<String> {
        vec![
            "publish_content".into(),    // 发布内容
            "update_content".into(),     // 更新内容
            "delete_content".into(),     // 删除内容
            "get_content".into(),        // 获取内容
            "list_contents".into(),      // 内容列表
        ]
    }

    async fn connect(&self) -> ConnectorResult<()> {
        // 建立gRPC连接
        tracing::info!("connecting to CMS gRPC server: {}", self.config.endpoint);
        Ok(())
    }

    async fn execute(&self, request: &ConnectorRequest) -> ConnectorResult<ConnectorResponse> {
        tracing::info!(
            operation = %request.operation,
            connector_id = %self.config.id,
            "executing CMS gRPC operation"
        );

        // 根据operation分发到具体的gRPC方法
        match request.operation.as_str() {
            "publish_content" => self.publish_content(request).await,
            "update_content" => self.update_content(request).await,
            "delete_content" => self.delete_content(request).await,
            "get_content" => self.get_content(request).await,
            "list_contents" => self.list_contents(request).await,
            _ => Err(ConnectorError::OperationNotSupported(request.operation.clone())),
        }
    }

    async fn health_check(&self) -> bool {
        // gRPC健康检查
        true
    }

    async fn close(&self) -> ConnectorResult<()> {
        // 关闭gRPC连接
        Ok(())
    }
}

// 具体操作实现
impl CmsGrpcConnector {
    async fn publish_content(&self, request: &ConnectorRequest) -> ConnectorResult<ConnectorResponse> {
        // 解析请求参数
        let title = request.params.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let content = request.params.get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let category = request.params.get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        // 调用gRPC方法（示例）
        // let grpc_request = PublishContentRequest { title, content, category };
        // let response = self.client.publish_content(grpc_request).await?;

        // 返回统一响应
        Ok(ConnectorResponse {
            success: true,
            status_code: 200,
            body: serde_json::json!({
                "content_id": "cms_123456",
                "title": title,
                "status": "published",
                "published_at": chrono::Utc::now().to_rfc3339(),
            }),
            headers: HashMap::new(),
            latency_ms: 45,
            error: None,
            retries: 0,
        })
    }

    async fn update_content(&self, request: &ConnectorRequest) -> ConnectorResult<ConnectorResponse> {
        // 实现更新内容逻辑
        Ok(ConnectorResponse::success(
            serde_json::json!({"status": "updated"}),
            20,
        ))
    }

    async fn delete_content(&self, request: &ConnectorRequest) -> ConnectorResult<ConnectorResponse> {
        // 实现删除内容逻辑
        Ok(ConnectorResponse::success(
            serde_json::json!({"status": "deleted"}),
            15,
        ))
    }

    async fn get_content(&self, request: &ConnectorRequest) -> ConnectorResult<ConnectorResponse> {
        // 实现获取内容逻辑
        Ok(ConnectorResponse::success(
            serde_json::json!({"content_id": "cms_123456", "title": "示例文章"}),
            10,
        ))
    }

    async fn list_contents(&self, request: &ConnectorRequest) -> ConnectorResult<ConnectorResponse> {
        // 实现内容列表逻辑
        Ok(ConnectorResponse::success(
            serde_json::json!({"items": [], "total": 0, "page": 1}),
            25,
        ))
    }
}
```

### 第3步: 实现Connector Factory（自定义协议需要）

```rust
// src/connectors/cms_grpc_factory.rs
use async_trait::async_trait;
use mox_platform_integration_core::prelude::*;
use std::sync::Arc;
use crate::connectors::cms_grpc_connector::CmsGrpcConnector;

/// CMS gRPC连接器工厂
pub struct CmsGrpcFactory;

#[async_trait]
impl ConnectorFactory for CmsGrpcFactory {
    fn factory_type(&self) -> &'static str {
        "cms_grpc"  // 与配置中的connector_type对应
    }

    async fn create(&self, config: &FactoryConfig) -> anyhow::Result<Arc<dyn Connector>> {
        // 从FactoryConfig构建ConnectorConfig
        let connector_config = ConnectorConfig {
            id: config.id.clone(),
            name: config.name.clone(),
            connector_type: "cms_grpc".into(),
            protocol: "grpc".into(),
            endpoint: config.get_str("endpoint").unwrap_or("").into(),
            auth_type: config.get_str("auth_type").unwrap_or("none").into(),
            credentials: config.config.get("credentials")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            timeout_secs: config.get_int("timeout_secs").unwrap_or(30) as u64,
            max_retries: config.get_int("max_retries").unwrap_or(2) as u32,
            retry_interval_ms: config.get_int("retry_interval_ms").unwrap_or(1000) as u64,
            rate_limit: config.config.get("rate_limit")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            enabled: config.enabled,
            metadata: config.metadata.clone(),
        };

        // 创建连接器实例
        let connector = CmsGrpcConnector::new(connector_config);

        // 建立连接
        connector.connect().await
            .map_err(|e| anyhow::anyhow!("failed to connect: {}", e))?;

        Ok(Arc::new(connector))
    }
}
```

### 第4步: 注册Factory + 加载配置

```rust
// src/main.rs 或 应用启动入口
use mox_platform_integration_core::prelude::*;
use crate::connectors::cms_grpc_factory::CmsGrpcFactory;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 加载配置
    let config = IntegrationConfig::load_from_file("config/integration.yaml").await?;

    // 2. 构建集成运行时
    let runtime = IntegrationRuntime::builder()
        .with_config(config)
        .with_all_capabilities()
        .build()
        .await?;

    // 3. 注册自定义Connector Factory
    runtime.factory_registry()
        .register_connector_factory(Arc::new(CmsGrpcFactory));

    // 4. 触发自动组装（从配置创建所有连接器实例）
    runtime.auto_assemble().await?;

    // 5. 验证连接器已注册
    let connectors = runtime.connector_registry().list();
    tracing::info!("已注册连接器数量: {}", connectors.len());
    for c in &connectors {
        tracing::info!("  - {} ({})", c.connector_name(), c.connector_type());
    }

    Ok(())
}
```

### 第5步: 调用连接器执行操作

```rust
// src/services/content_publish_service.rs
use mox_connector_core::prelude::*;
use mox_platform_integration_core::prelude::*;

/// 内容发布服务
pub struct ContentPublishService {
    connector_registry: Arc<ConnectorRegistry>,
}

impl ContentPublishService {
    pub fn new(connector_registry: Arc<ConnectorRegistry>) -> Self {
        Self { connector_registry }
    }

    /// 发布内容到CMS
    pub async fn publish_to_cms(&self, content: &Content) -> anyhow::Result<PublishResult> {
        // 1. 获取CMS连接器
        let connector = self.connector_registry.get("cms-grpc")
            .ok_or_else(|| anyhow::anyhow!("CMS连接器未注册"))?;

        // 2. 构建请求
        let request = ConnectorRequest {
            operation: "publish_content".into(),
            params: serde_json::json!({
                "title": content.title,
                "content": content.body,
                "category": content.category,
                "tags": content.tags,
                "author": content.author,
            }),
            headers: std::collections::HashMap::new(),
            timeout_secs: Some(30),
            trace_id: Some(current_trace_id()),
        };

        // 3. 执行调用
        let response = connector.execute(&request).await
            .map_err(|e| anyhow::anyhow!("CMS发布失败: {}", e))?;

        // 4. 解析响应
        if !response.success {
            return Err(anyhow::anyhow!(
                "CMS发布失败: status={}, error={:?}",
                response.status_code,
                response.error
            ));
        }

        let content_id = response.body.get("content_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(PublishResult {
            success: true,
            content_id,
            platform: "cms".into(),
            published_at: chrono::Utc::now(),
            latency_ms: response.latency_ms,
        })
    }

    /// 一键多平台发布
    pub async fn publish_to_all_platforms(&self, content: &Content) -> anyhow::Result<Vec<PublishResult>> {
        let platforms = ["cms-grpc", "ecommerce-rest", "social-webhook"];
        let mut results = Vec::new();

        for platform_id in &platforms {
            match self.connector_registry.get(platform_id) {
                Some(connector) => {
                    let request = ConnectorRequest {
                        operation: "publish_content".into(),
                        params: serde_json::json!({
                            "title": content.title,
                            "content": content.body,
                        }),
                        headers: std::collections::HashMap::new(),
                        timeout_secs: Some(30),
                        trace_id: Some(current_trace_id()),
                    };

                    match connector.execute(&request).await {
                        Ok(response) => {
                            results.push(PublishResult {
                                success: response.success,
                                content_id: response.body.get("content_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                platform: platform_id.to_string(),
                                published_at: chrono::Utc::now(),
                                latency_ms: response.latency_ms,
                            });
                        }
                        Err(e) => {
                            tracing::error!(platform = platform_id, error = %e, "发布失败");
                            results.push(PublishResult {
                                success: false,
                                content_id: String::new(),
                                platform: platform_id.to_string(),
                                published_at: chrono::Utc::now(),
                                latency_ms: 0,
                            });
                        }
                    }
                }
                None => {
                    tracing::warn!(platform = platform_id, "连接器未注册，跳过");
                }
            }
        }

        Ok(results)
    }
}

// 数据结构
pub struct Content {
    pub title: String,
    pub body: String,
    pub category: String,
    pub tags: Vec<String>,
    pub author: String,
}

pub struct PublishResult {
    pub success: bool,
    pub content_id: String,
    pub platform: String,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub latency_ms: u64,
}
```

---

## 5. gRPC对接详解

### 5.1 gRPC对接架构

```
Mox Platform                          新系统 (gRPC Server)
─────────────                         ─────────────────────
Connector Trait                       .proto 定义
    │                                     │
    ▼                                     ▼
GrpcConnector ──── tonic client ────▶ gRPC Server
    │                                     │
    ├── publish_content()                 ├── PublishContent()
    ├── update_content()                  ├── UpdateContent()
    ├── delete_content()                  ├── DeleteContent()
    └── get_content()                     └── GetContent()
```

### 5.2 步骤1: 定义Protobuf

创建`proto/cms.proto`：

```protobuf
syntax = "proto3";

package cms.v1;

option go_package = "github.com/company/cms/proto/v1";

// CMS服务
service CmsService {
  // 发布内容
  rpc PublishContent(PublishContentRequest) returns (PublishContentResponse);
  // 更新内容
  rpc UpdateContent(UpdateContentRequest) returns (UpdateContentResponse);
  // 删除内容
  rpc DeleteContent(DeleteContentRequest) returns (DeleteContentResponse);
  // 获取内容
  rpc GetContent(GetContentRequest) returns (GetContentResponse);
  // 内容列表（服务端流式）
  rpc ListContents(ListContentsRequest) returns (stream ContentItem);
}

// 发布内容请求
message PublishContentRequest {
  string title = 1;
  string content = 2;
  string category = 3;
  repeated string tags = 4;
  string author = 5;
  map<string, string> metadata = 6;
}

// 发布内容响应
message PublishContentResponse {
  string content_id = 1;
  string status = 2;
  string published_at = 3;
  string url = 4;
}

// ... 其他消息定义
```

### 5.3 步骤2: 编译Protobuf

在`build.rs`中添加：

```rust
// build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)  // 只构建客户端
        .build_client(true)
        .compile_protos(&["proto/cms.proto"], &["proto/"])?;
    Ok(())
}
```

### 5.4 步骤3: 实现gRPC连接器

```rust
// src/connectors/grpc_connector.rs
use async_trait::async_trait;
use mox_connector_core::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// 导入tonic生成的客户端
// pub mod cms {
//     pub mod v1 {
//         tonic::include_proto!("cms.v1");
//     }
// }

/// 通用gRPC连接器
pub struct GrpcConnector {
    config: ConnectorConfig,
    // gRPC客户端（使用Mutex保证线程安全）
    // client: Arc<Mutex<cms::v1::cms_service_client::CmsServiceClient<Channel>>>,
}

impl GrpcConnector {
    pub async fn new(config: ConnectorConfig) -> ConnectorResult<Self> {
        // 创建gRPC通道
        // let channel = Channel::from_shared(config.endpoint.clone())
        //     .map_err(|e| ConnectorError::ConnectionFailed(e.to_string()))?
        //     .connect()
        //     .await
        //     .map_err(|e| ConnectorError::ConnectionFailed(e.to_string()))?;

        // let client = cms::v1::cms_service_client::CmsServiceClient::new(channel);

        Ok(Self {
            config,
            // client: Arc::new(Mutex::new(client)),
        })
    }
}

#[async_trait]
impl Connector for GrpcConnector {
    fn connector_id(&self) -> &str { &self.config.id }
    fn connector_name(&self) -> &str { &self.config.name }
    fn connector_type(&self) -> ConnectorType { ConnectorType::Grpc }

    fn supported_protocols(&self) -> Vec<String> { vec!["grpc".into()] }

    fn supported_operations(&self) -> Vec<String> {
        vec![
            "publish_content".into(),
            "update_content".into(),
            "delete_content".into(),
            "get_content".into(),
            "list_contents".into(),
        ]
    }

    async fn connect(&self) -> ConnectorResult<()> {
        tracing::info!("gRPC connected to {}", self.config.endpoint);
        Ok(())
    }

    async fn execute(&self, request: &ConnectorRequest) -> ConnectorResult<ConnectorResponse> {
        let start = std::time::Instant::now();

        let result = match request.operation.as_str() {
            "publish_content" => self.publish_content(request).await,
            "update_content" => self.update_content(request).await,
            "delete_content" => self.delete_content(request).await,
            "get_content" => self.get_content(request).await,
            "list_contents" => self.list_contents(request).await,
            _ => Err(ConnectorError::OperationNotSupported(request.operation.clone())),
        };

        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(body) => Ok(ConnectorResponse {
                success: true,
                status_code: 200,
                body,
                headers: HashMap::new(),
                latency_ms,
                error: None,
                retries: 0,
            }),
            Err(e) => Ok(ConnectorResponse {
                success: false,
                status_code: 500,
                body: serde_json::Value::Null,
                headers: HashMap::new(),
                latency_ms,
                error: Some(e.to_string()),
                retries: 0,
            }),
        }
    }

    async fn health_check(&self) -> bool {
        // gRPC健康检查
        true
    }

    async fn close(&self) -> ConnectorResult<()> {
        Ok(())
    }
}

// gRPC方法实现
impl GrpcConnector {
    async fn publish_content(&self, request: &ConnectorRequest) -> Result<serde_json::Value, ConnectorError> {
        let title = request.params.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let content = request.params.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let category = request.params.get("category").and_then(|v| v.as_str()).unwrap_or("default").to_string();

        // 调用gRPC方法
        // let grpc_request = tonic::Request::new(cms::v1::PublishContentRequest {
        //     title,
        //     content,
        //     category,
        //     tags: vec![],
        //     author: "".into(),
        //     metadata: HashMap::new(),
        // });
        //
        // let mut client = self.client.lock().await;
        // let response = client.publish_content(grpc_request).await
        //     .map_err(|e| ConnectorError::ExecutionFailed(e.to_string()))?;
        // let inner = response.into_inner();

        // 返回结果（示例）
        Ok(serde_json::json!({
            "content_id": format!("cms_{}", uuid::Uuid::new_v4()),
            "title": title,
            "status": "published",
            "published_at": chrono::Utc::now().to_rfc3339(),
        }))
    }

    async fn update_content(&self, request: &ConnectorRequest) -> Result<serde_json::Value, ConnectorError> {
        // 实现更新逻辑
        Ok(serde_json::json!({"status": "updated"}))
    }

    async fn delete_content(&self, request: &ConnectorRequest) -> Result<serde_json::Value, ConnectorError> {
        // 实现删除逻辑
        Ok(serde_json::json!({"status": "deleted"}))
    }

    async fn get_content(&self, request: &ConnectorRequest) -> Result<serde_json::Value, ConnectorError> {
        // 实现获取逻辑
        Ok(serde_json::json!({"content_id": "cms_123", "title": "示例"}))
    }

    async fn list_contents(&self, request: &ConnectorRequest) -> Result<serde_json::Value, ConnectorError> {
        // 实现列表逻辑（支持流式）
        Ok(serde_json::json!({"items": [], "total": 0}))
    }
}
```

### 5.5 gRPC认证配置

```yaml
connector:
  connectors:
    - id: cms-grpc-secure
      name: 企业CMS (安全gRPC)
      connector_type: grpc
      protocol: grpc
      endpoint: cms.internal:50051
      auth_type: tls                     # TLS认证
      credentials:
        tls_cert: ${CMS_TLS_CERT}        # 客户端证书
        tls_key: ${CMS_TLS_KEY}          # 客户端私钥
        tls_ca: ${CMS_TLS_CA}            # CA证书
        tls_domain_name: cms.internal     # 域名验证
      timeout_secs: 30
      enabled: true
```

---

## 6. REST对接详解

### 6.1 REST连接器实现

```rust
// src/connectors/rest_connector.rs
use async_trait::async_trait;
use mox_connector_core::prelude::*;
use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;

/// 通用REST连接器
pub struct RestConnector {
    config: ConnectorConfig,
    client: Client,
}

impl RestConnector {
    pub fn new(config: ConnectorConfig) -> ConnectorResult<Self> {
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs));

        // 配置认证
        match config.auth_type.as_str() {
            "bearer" => {
                if let Some(token) = config.credentials.get("token") {
                    let mut headers = reqwest::header::HeaderMap::new();
                    headers.insert(
                        reqwest::header::AUTHORIZATION,
                        format!("Bearer {}", token).parse().unwrap(),
                    );
                    client_builder = client_builder.default_headers(headers);
                }
            }
            "api_key" => {
                if let Some(api_key) = config.credentials.get("api_key") {
                    let header_name = config.credentials.get("api_key_header")
                        .map(|s| s.as_str())
                        .unwrap_or("X-API-Key");
                    let mut headers = reqwest::header::HeaderMap::new();
                    headers.insert(
                        reqwest::header::HeaderName::from_bytes(header_name.as_bytes()).unwrap(),
                        api_key.parse().unwrap(),
                    );
                    client_builder = client_builder.default_headers(headers);
                }
            }
            _ => {}
        }

        let client = client_builder.build()
            .map_err(|e| ConnectorError::ConnectionFailed(e.to_string()))?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl Connector for RestConnector {
    fn connector_id(&self) -> &str { &self.config.id }
    fn connector_name(&self) -> &str { &self.config.name }
    fn connector_type(&self) -> ConnectorType { ConnectorType::Rest }

    fn supported_protocols(&self) -> Vec<String> { vec!["rest".into()] }

    fn supported_operations(&self) -> Vec<String> {
        vec![
            "get".into(),
            "post".into(),
            "put".into(),
            "patch".into(),
            "delete".into(),
        ]
    }

    async fn connect(&self) -> ConnectorResult<()> {
        tracing::info!("REST connector connected to {}", self.config.endpoint);
        Ok(())
    }

    async fn execute(&self, request: &ConnectorRequest) -> ConnectorResult<ConnectorResponse> {
        let start = std::time::Instant::now();

        // 从params中提取路径和方法
        let path = request.params.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("/")
            .to_string();
        let method = request.operation.to_uppercase();
        let body = request.params.get("body").cloned().unwrap_or(serde_json::Value::Null);

        let url = format!("{}{}", self.config.endpoint.trim_end_matches('/'), path);

        tracing::info!(method = %method, url = %url, "REST request");

        // 发送请求
        let request_builder = match method.as_str() {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url).json(&body),
            "PUT" => self.client.put(&url).json(&body),
            "PATCH" => self.client.patch(&url).json(&body),
            "DELETE" => self.client.delete(&url),
            _ => return Err(ConnectorError::OperationNotSupported(method)),
        };

        let response = request_builder.send().await
            .map_err(|e| ConnectorError::ExecutionFailed(e.to_string()))?;

        let status = response.status().as_u16();
        let response_body: serde_json::Value = response.json().await
            .unwrap_or(serde_json::Value::Null);

        let latency_ms = start.elapsed().as_millis() as u64;
        let success = status >= 200 && status < 300;

        Ok(ConnectorResponse {
            success,
            status_code: status,
            body: response_body,
            headers: HashMap::new(),
            latency_ms,
            error: if success { None } else { Some(format!("HTTP {}", status)) },
            retries: 0,
        })
    }

    async fn health_check(&self) -> bool {
        // 发送GET请求到健康检查端点
        true
    }

    async fn close(&self) -> ConnectorResult<()> {
        Ok(())
    }
}
```

### 6.2 REST调用示例

```rust
// 发布内容到电商平台
let request = ConnectorRequest {
    operation: "post".into(),
    params: serde_json::json!({
        "path": "/api/v1/products",
        "body": {
            "title": "新品发布",
            "description": "这是一款新产品",
            "price": 99.99,
            "category": "electronics",
        }
    }),
    headers: HashMap::new(),
    timeout_secs: Some(15),
    trace_id: Some(current_trace_id()),
};

let response = connector.execute(&request).await?;
```

---

## 7. 内容发布场景完整示例

### 7.1 场景描述

**需求**: 将Mox Platform中的内容一键发布到多个第三方系统：
- 企业CMS系统（gRPC）
- 电商平台（REST）
- 社交媒体（Webhook）

### 7.2 配置文件

```yaml
# config/integration.yaml
connector:
  enabled: true
  global_timeout_secs: 30
  global_max_retries: 2

  connectors:
    # 企业CMS (gRPC)
    - id: cms-grpc
      name: 企业CMS系统
      connector_type: grpc
      protocol: grpc
      endpoint: cms.internal:50051
      auth_type: bearer
      credentials:
        token: ${CMS_GRPC_TOKEN}
      timeout_secs: 30
      max_retries: 3
      rate_limit:
        enabled: true
        max_qps: 50
      enabled: true

    # 电商平台 (REST)
    - id: ecommerce-rest
      name: 电商平台
      connector_type: rest
      protocol: rest
      endpoint: https://api.ecommerce.com
      auth_type: api_key
      credentials:
        api_key: ${ECOMMERCE_API_KEY}
        api_key_header: X-API-Key
      timeout_secs: 15
      max_retries: 2
      enabled: true

    # 社交媒体 (Webhook)
    - id: social-webhook
      name: 社交媒体
      connector_type: webhook
      protocol: rest
      endpoint: https://hooks.social.com/publish
      auth_type: none
      timeout_secs: 10
      max_retries: 3
      enabled: true
```

### 7.3 内容发布服务

```rust
// src/services/content_publisher.rs
use async_trait::async_trait;
use mox_connector_core::prelude::*;
use mox_platform_integration_core::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task;

/// 内容发布器
pub struct ContentPublisher {
    connector_registry: Arc<ConnectorRegistry>,
}

impl ContentPublisher {
    pub fn new(connector_registry: Arc<ConnectorRegistry>) -> Self {
        Self { connector_registry }
    }

    /// 发布内容到指定平台
    pub async fn publish(&self, content: &Content, platform_id: &str) -> PublishResult {
        let connector = match self.connector_registry.get(platform_id) {
            Some(c) => c,
            None => {
                return PublishResult::failed(platform_id, "连接器未注册");
            }
        };

        let operation = self.get_publish_operation(platform_id);
        let params = self.build_publish_params(content, platform_id);

        let request = ConnectorRequest {
            operation,
            params,
            headers: HashMap::new(),
            timeout_secs: Some(30),
            trace_id: Some(current_trace_id()),
        };

        match connector.execute(&request).await {
            Ok(response) => {
                if response.success {
                    PublishResult::success(
                        platform_id,
                        response.body.get("content_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        response.latency_ms,
                    )
                } else {
                    PublishResult::failed(platform_id, &response.error.unwrap_or_else(|| "未知错误".into()))
                }
            }
            Err(e) => PublishResult::failed(platform_id, &e.to_string()),
        }
    }

    /// 并发发布到所有平台
    pub async fn publish_to_all(&self, content: &Content) -> Vec<PublishResult> {
        let platforms = self.get_publish_platforms();
        let mut handles = Vec::new();

        for platform_id in platforms {
            let content = content.clone();
            let connector_registry = self.connector_registry.clone();
            let platform_id = platform_id.to_string();

            let handle = task::spawn(async move {
                let publisher = ContentPublisher::new(connector_registry);
                publisher.publish(&content, &platform_id).await
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(PublishResult::failed("unknown", &e.to_string())),
            }
        }
        results
    }

    fn get_publish_operation(&self, platform_id: &str) -> String {
        match platform_id {
            "cms-grpc" => "publish_content".into(),
            "ecommerce-rest" => "post".into(),
            "social-webhook" => "post".into(),
            _ => "publish".into(),
        }
    }

    fn build_publish_params(&self, content: &Content, platform_id: &str) -> serde_json::Value {
        match platform_id {
            "cms-grpc" => serde_json::json!({
                "title": content.title,
                "content": content.body,
                "category": content.category,
                "tags": content.tags,
                "author": content.author,
            }),
            "ecommerce-rest" => serde_json::json!({
                "path": "/api/v1/products",
                "body": {
                    "name": content.title,
                    "description": content.body,
                    "category": content.category,
                    "tags": content.tags,
                }
            }),
            "social-webhook" => serde_json::json!({
                "path": "/",
                "body": {
                    "text": format!("{}\n\n{}", content.title, content.body),
                    "tags": content.tags,
                }
            }),
            _ => serde_json::json!({
                "title": content.title,
                "content": content.body,
            }),
        }
    }

    fn get_publish_platforms(&self) -> Vec<&str> {
        vec!["cms-grpc", "ecommerce-rest", "social-webhook"]
    }
}

// 数据结构
#[derive(Clone)]
pub struct Content {
    pub title: String,
    pub body: String,
    pub category: String,
    pub tags: Vec<String>,
    pub author: String,
}

pub struct PublishResult {
    pub success: bool,
    pub platform: String,
    pub content_id: String,
    pub error: Option<String>,
    pub latency_ms: u64,
    pub published_at: chrono::DateTime<chrono::Utc>,
}

impl PublishResult {
    pub fn success(platform: &str, content_id: String, latency_ms: u64) -> Self {
        Self {
            success: true,
            platform: platform.into(),
            content_id,
            error: None,
            latency_ms,
            published_at: chrono::Utc::now(),
        }
    }

    pub fn failed(platform: &str, error: &str) -> Self {
        Self {
            success: false,
            platform: platform.into(),
            content_id: String::new(),
            error: Some(error.into()),
            latency_ms: 0,
            published_at: chrono::Utc::now(),
        }
    }
}
```

### 7.4 API端点

```rust
// src/api/content_publish_api.rs
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::services::content_publisher::{Content, ContentPublisher};

/// 发布内容请求
#[derive(Deserialize)]
pub struct PublishContentRequest {
    pub title: String,
    pub body: String,
    pub category: String,
    pub tags: Vec<String>,
    pub author: String,
    /// 指定发布平台（为空则发布到所有平台）
    #[serde(default)]
    pub platforms: Vec<String>,
}

/// 发布内容响应
#[derive(Serialize)]
pub struct PublishContentResponse {
    pub success: bool,
    pub trace_id: String,
    pub results: Vec<PublishResultDto>,
    pub summary: PublishSummary,
}

#[derive(Serialize)]
pub struct PublishResultDto {
    pub platform: String,
    pub success: bool,
    pub content_id: String,
    pub error: Option<String>,
    pub latency_ms: u64,
}

#[derive(Serialize)]
pub struct PublishSummary {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub total_latency_ms: u64,
}

/// 发布内容API
pub async fn publish_content(
    State(publisher): State<Arc<ContentPublisher>>,
    Json(request): Json<PublishContentRequest>,
) -> Result<Json<PublishContentResponse>, StatusCode> {
    let content = Content {
        title: request.title,
        body: request.body,
        category: request.category,
        tags: request.tags,
        author: request.author,
    };

    let results = if request.platforms.is_empty() {
        publisher.publish_to_all(&content).await
    } else {
        let mut all_results = Vec::new();
        for platform in &request.platforms {
            let result = publisher.publish(&content, platform).await;
            all_results.push(result);
        }
        all_results
    };

    let success_count = results.iter().filter(|r| r.success).count();
    let failed_count = results.len() - success_count;
    let total_latency: u64 = results.iter().map(|r| r.latency_ms).sum();

    let response = PublishContentResponse {
        success: failed_count == 0,
        trace_id: current_trace_id(),
        results: results.iter().map(|r| PublishResultDto {
            platform: r.platform.clone(),
            success: r.success,
            content_id: r.content_id.clone(),
            error: r.error.clone(),
            latency_ms: r.latency_ms,
        }).collect(),
        summary: PublishSummary {
            total: results.len(),
            success: success_count,
            failed: failed_count,
            total_latency_ms: total_latency,
        },
    };

    Ok(Json(response))
}
```

### 7.5 调用示例

```bash
# 发布到所有平台
curl -X POST http://localhost:3000/api/content/publish \
  -H "Content-Type: application/json" \
  -d '{
    "title": "新品发布：Mox Platform v3.0",
    "body": "Mox Platform v3.0正式发布，带来全新的企业级架构...",
    "category": "product",
    "tags": ["发布", "企业级", "架构"],
    "author": "Mox Team"
  }'

# 只发布到CMS
curl -X POST http://localhost:3000/api/content/publish \
  -H "Content-Type: application/json" \
  -d '{
    "title": "内部公告",
    "body": "这是一条内部公告...",
    "category": "announcement",
    "tags": ["公告"],
    "author": "Admin",
    "platforms": ["cms-grpc"]
  }'
```

### 7.6 响应示例

```json
{
  "success": true,
  "trace_id": "a1b2c3d4e5f6",
  "results": [
    {
      "platform": "cms-grpc",
      "success": true,
      "content_id": "cms_abc123",
      "error": null,
      "latency_ms": 45
    },
    {
      "platform": "ecommerce-rest",
      "success": true,
      "content_id": "prod_789",
      "error": null,
      "latency_ms": 120
    },
    {
      "platform": "social-webhook",
      "success": false,
      "content_id": "",
      "error": "HTTP 500",
      "latency_ms": 3000
    }
  ],
  "summary": {
    "total": 3,
    "success": 2,
    "failed": 1,
    "total_latency_ms": 3165
  }
}
```

---

## 8. 配置驱动零代码对接

### 8.1 内置连接器类型

Mox Platform内置以下连接器类型，**无需写代码**，纯配置即可对接：

| 连接器类型 | connector_type | 适用场景 | 认证方式 |
|-----------|---------------|---------|---------|
| **REST连接器** | `rest` | 通用REST API对接 | none/bearer/api_key/basic |
| **Webhook连接器** | `webhook` | 事件通知/回调 | none/bearer/api_key |
| **gRPC连接器** | `grpc` | 高性能gRPC服务 | none/bearer/tls |
| **GraphQL连接器** | `graphql` | GraphQL API对接 | none/bearer/api_key |

### 8.2 零代码对接示例

**场景**: 对接一个新的REST API系统

**只需要3步**:

1. **在配置文件中添加连接器配置**
```yaml
connector:
  connectors:
    - id: new-system-rest
      name: 新系统
      connector_type: rest
      endpoint: https://api.newsystem.com
      auth_type: bearer
      credentials:
        token: ${NEW_SYSTEM_TOKEN}
      timeout_secs: 30
      enabled: true
```

2. **设置环境变量**
```bash
export NEW_SYSTEM_TOKEN="your-token-here"
```

3. **重启应用（或等待配置热更新）**

完成！现在可以通过Connector Registry调用新系统：

```rust
let connector = connector_registry.get("new-system-rest").unwrap();
let response = connector.execute(&ConnectorRequest {
    operation: "get".into(),
    params: serde_json::json!({"path": "/api/v1/data"}),
    headers: HashMap::new(),
    timeout_secs: Some(30),
    trace_id: Some(current_trace_id()),
}).await?;
```

### 8.3 配置热更新

修改配置后，无需重启应用，配置热更新自动生效：

```rust
// 配置热更新已内置在IntegrationRuntime中
let runtime = IntegrationRuntime::builder()
    .with_config(config)
    .with_config_hot_reload(true)  // 启用热更新
    .with_all_capabilities()
    .build()
    .await?;
```

配置文件变化后：
1. `ConfigHotReloader`检测到文件变化
2. 重新加载配置
3. 通知所有订阅者
4. Connector Registry更新连接器实例
5. 新请求使用新配置

---

## 9. 高级特性

### 9.1 限流

```yaml
connector:
  connectors:
    - id: cms-grpc
      rate_limit:
        enabled: true
        max_qps: 100           # 每秒最大请求数
        burst_size: 200        # 突发容量
        strategy: token_bucket  # 限流策略
```

### 9.2 熔断

```yaml
connector:
  connectors:
    - id: cms-grpc
      circuit_breaker:
        enabled: true
        failure_threshold: 5     # 连续失败5次触发熔断
        success_threshold: 3     # 连续成功3次恢复
        half_open_timeout_secs: 30  # 半开状态超时
        reset_timeout_secs: 60      # 熔断重置时间
```

### 9.3 重试

```yaml
connector:
  connectors:
    - id: cms-grpc
      max_retries: 3
      retry_interval_ms: 1000
      retry_strategy: exponential   # fixed/exponential
      retryable_status_codes: [500, 502, 503, 504]
```

### 9.4 全链路追踪

所有连接器调用自动携带trace_id：

```rust
// 自动从线程局部获取trace_id
let request = ConnectorRequest {
    operation: "publish_content".into(),
    params: serde_json::json!({...}),
    headers: HashMap::new(),
    timeout_secs: Some(30),
    trace_id: Some(current_trace_id()),  // 自动获取
};
```

trace_id通过HTTP头传播：
- `X-Trace-Id: a1b2c3d4e5f6`
- `traceparent: 00-a1b2c3d4e5f6-xxxxxxxx-01` (W3C标准)

### 9.5 审计日志

所有连接器调用自动记录审计日志：

```rust
// 审计日志包含
{
  "event_type": "connector.execute",
  "severity": "info",
  "actor_id": "user_123",
  "connector_id": "cms-grpc",
  "operation": "publish_content",
  "success": true,
  "latency_ms": 45,
  "trace_id": "a1b2c3d4e5f6",
  "timestamp": "2026-08-27T10:30:00Z"
}
```

---

## 10. 最佳实践

### 10.1 配置管理

- **使用环境变量管理敏感信息**：API Key、Token等不要硬编码在配置文件中
- **多环境配置分离**：dev/test/prod使用不同的配置文件
- **配置版本控制**：配置文件纳入Git管理（敏感信息除外）
- **配置热更新**：生产环境启用配置热更新，避免重启

### 10.2 安全实践

- **最小权限原则**：连接器使用的账号只授予必要的权限
- **传输加密**：生产环境必须使用TLS/HTTPS
- **认证轮换**：定期轮换API Key和Token
- **请求签名**：对敏感操作使用请求签名

### 10.3 性能优化

- **连接池复用**：gRPC/HTTP连接使用连接池
- **并发调用**：多平台发布使用并发调用，减少总耗时
- **超时设置**：合理设置超时，避免长时间阻塞
- **限流保护**：对外部系统调用设置限流，避免被封禁

### 10.4 可观测性

- **全链路追踪**：确保trace_id在所有调用中传播
- **结构化日志**：使用tracing记录结构化日志
- **指标监控**：监控连接器的成功率、延迟、错误率
- **告警设置**：对错误率过高、延迟过大设置告警

### 10.5 错误处理

- **统一错误码**：使用平台统一的错误码体系
- **错误重试**：对可重试错误设置合理的重试策略
- **熔断保护**：对连续失败的连接器启用熔断
- **降级方案**：为关键连接器准备降级方案

---

## 11. 故障排查

### 11.1 常见问题

| 问题 | 可能原因 | 解决方案 |
|------|---------|---------|
| **连接器未注册** | 配置错误 / Factory未注册 | 检查配置文件中的connector_type是否正确；检查Factory是否已注册 |
| **连接超时** | 网络问题 / 服务不可用 / 超时设置过短 | 检查网络连通性；检查目标服务状态；增大timeout_secs |
| **认证失败** | Token过期 / API Key错误 / 认证方式不匹配 | 检查凭证是否正确；检查auth_type是否与目标系统一致 |
| **操作不支持** | operation名称错误 | 检查connector.supported_operations()返回的操作列表 |
| **响应解析错误** | 响应格式不符合预期 | 检查目标系统返回的数据格式；调整响应解析逻辑 |
| **限流触发** | 请求频率过高 | 检查rate_limit配置；降低请求频率；增大max_qps |
| **熔断打开** | 连续失败次数过多 | 检查目标服务状态；等待熔断重置；检查circuit_breaker配置 |

### 11.2 调试技巧

**1. 启用详细日志**

```rust
// 设置日志级别
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

**2. 检查连接器状态**

```rust
// 列出所有已注册连接器
let connectors = connector_registry.list();
for c in &connectors {
    println!("{}: {} (type={:?})", c.connector_id(), c.connector_name(), c.connector_type());
    println!("  supported operations: {:?}", c.supported_operations());
    println!("  health: {}", c.health_check().await);
}
```

**3. 手动测试连接器**

```rust
let connector = connector_registry.get("cms-grpc").unwrap();

// 测试连接
match connector.connect().await {
    Ok(_) => println!("连接成功"),
    Err(e) => println!("连接失败: {}", e),
}

// 测试健康检查
println!("健康状态: {}", connector.health_check().await);

// 测试执行
let request = ConnectorRequest {
    operation: "get_content".into(),
    params: serde_json::json!({"content_id": "test_123"}),
    headers: HashMap::new(),
    timeout_secs: Some(10),
    trace_id: Some("debug-trace-001".into()),
};
match connector.execute(&request).await {
    Ok(response) => println!("执行成功: {:?}", response),
    Err(e) => println!("执行失败: {}", e),
}
```

### 11.3 网络排查

```bash
# 测试网络连通性
ping cms.internal
telnet cms.internal 50051

# 测试gRPC服务（使用grpcurl）
grpcurl -plaintext cms.internal:50051 list
grpcurl -plaintext cms.internal:50051 describe cms.v1.CmsService

# 测试REST API
curl -v https://api.ecommerce.com/health
curl -H "Authorization: Bearer $TOKEN" https://api.ecommerce.com/api/v1/products
```

---

## 12. 附录

### 12.1 Connector Trait完整定义

```rust
#[async_trait]
pub trait Connector: Send + Sync {
    /// 连接器ID
    fn connector_id(&self) -> &str;

    /// 连接器名称
    fn connector_name(&self) -> &str;

    /// 连接器类型
    fn connector_type(&self) -> ConnectorType;

    /// 支持的协议
    fn supported_protocols(&self) -> Vec<String>;

    /// 支持的操作
    fn supported_operations(&self) -> Vec<String>;

    /// 建立连接
    async fn connect(&self) -> ConnectorResult<()>;

    /// 执行操作
    async fn execute(&self, request: &ConnectorRequest) -> ConnectorResult<ConnectorResponse>;

    /// 健康检查
    async fn health_check(&self) -> bool;

    /// 关闭连接
    async fn close(&self) -> ConnectorResult<()>;
}
```

### 12.2 配置参考

完整的连接器配置字段：

```yaml
connector:
  connectors:
    - id: string                    # 连接器唯一标识（必须）
      name: string                  # 连接器名称（必须）
      connector_type: string        # 连接器类型：rest/grpc/webhook/graphql/custom（必须）
      protocol: string              # 协议：rest/grpc/websocket/graphql
      endpoint: string              # 服务地址（必须）
      auth_type: string             # 认证方式：none/bearer/api_key/basic/tls
      credentials:                  # 认证凭证
        token: string               # Bearer Token
        api_key: string             # API Key
        api_key_header: string      # API Key请求头名称
        username: string            # Basic Auth用户名
        password: string            # Basic Auth密码
        tls_cert: string            # TLS客户端证书
        tls_key: string             # TLS客户端私钥
        tls_ca: string              # TLS CA证书
      timeout_secs: int             # 请求超时（秒），默认30
      max_retries: int              # 最大重试次数，默认2
      retry_interval_ms: int        # 重试间隔（毫秒），默认1000
      retry_strategy: string        # 重试策略：fixed/exponential
      rate_limit:                   # 限流配置
        enabled: bool               # 是否启用限流
        max_qps: int                # 每秒最大请求数
        burst_size: int             # 突发容量
        strategy: string            # 限流策略：token_bucket
      circuit_breaker:              # 熔断配置
        enabled: bool               # 是否启用熔断
        failure_threshold: int      # 连续失败阈值
        success_threshold: int      # 连续成功阈值
        reset_timeout_secs: int     # 熔断重置时间
      enabled: bool                 # 是否启用，默认true
      metadata:                     # 元数据
        key: value
```

### 12.3 相关文档

- [架构主文档](../../ARCHITECTURE.md)
- [扩展开发指南](./02-extension-guide.md)
- [错误码参考手册](./04-error-code-reference.md)
- [归一化检查清单](./05-normalization-checklist.md)

### 12.4 快速对接检查清单

对接新系统前，确认以下事项：

- [ ] 收集新系统信息（地址/协议/认证/API定义）
- [ ] 确定对接模式（Outbound/Inbound/Bidirectional）
- [ ] 选择连接器类型（rest/grpc/webhook/custom）
- [ ] 编写连接器配置（YAML）
- [ ] 实现Connector Trait（自定义协议需要）
- [ ] 实现Connector Factory（自定义协议需要）
- [ ] 注册Factory到FactoryRegistry
- [ ] 加载配置并触发自动组装
- [ ] 验证连接器已注册
- [ ] 测试连接和健康检查
- [ ] 测试核心操作
- [ ] 配置限流/熔断/重试
- [ ] 配置审计日志
- [ ] 配置监控告警
- [ ] 编写对接文档

---

**文档版本**: 1.0.0
**最后更新**: 2026-08-27
**维护者**: 开发联盟
**适用版本**: Mox Platform v3.0.0+

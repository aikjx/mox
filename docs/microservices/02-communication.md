# 02 - 通信架构优化

> 版本：v1.0 | 日期：2026-08-26 | 状态：草案
>
> 前置阅读：[00-核心原则](./00-principles.md) | [01-服务边界优化](./01-service-boundaries.md)

## 一、现状诊断

### 1.1 当前通信方式

| 通信场景 | 当前方式 | 问题 |
|----------|----------|------|
| 前端→后端 | axum REST (JSON) | 无统一网关，REST 直接暴露 |
| 服务间调用 | 进程内函数调用（单体） | 无法独立部署，紧耦合 |
| 长连接 | tokio-tungstenite WebSocket | 有但未标准化 |
| 异步事件 | Redis Pub/Sub（简陋） | 无持久化、无重试、无死信 |
| AI 流式输出 | 无标准方式 | 可能用 SSE 或自定义 |
| 外部 API 调用 | reqwest | 无统一封装、无重试/熔断 |

### 1.2 核心问题

| 问题 | 影响 | 严重度 |
|------|------|--------|
| **无 gRPC/RPC** | 服务间无法高效通信，无法流式传输，无法独立部署 | 🔴 高 |
| **无 API 网关** | 认证/限流/路由分散在各服务，无统一入口 | 🔴 高 |
| **消息队列简陋** | Redis Pub/Sub 无持久化，消息可能丢失，无重试机制 | 🟡 中 |
| **无服务注册发现** | 服务地址硬编码，无法动态扩缩容 | 🟡 中 |
| **无统一通信契约** | 各服务接口定义不统一，无版本管理 | 🟡 中 |
| **无熔断降级** | 一个服务慢导致级联故障，雪崩风险 | 🟡 中 |

---

## 二、目标通信架构

### 2.1 通信协议分层

```
┌─────────────────────────────────────────────────────────────┐
│                    外部通信 (External)                         │
│  前端/第三方/移动端 → API Gateway                              │
│  协议：REST (JSON) + gRPC-Web + WebSocket + SSE              │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                 服务间通信 (Internal)                          │
│  服务 ↔ 服务                                                   │
│  协议：gRPC (Protobuf, HTTP/2)  ★首选★                       │
│  模式：Unary + Server Streaming + Client Streaming + Bidirectional │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                 异步事件通信 (Async)                           │
│  服务 → 消息队列 → 服务（解耦、削峰、最终一致性）              │
│  协议：NATS JetStream（推荐）/ RabbitMQ (lapin)              │
│  模式：Pub/Sub + Queue + Stream（持久化）                     │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                 Sidecar 通信 (Sidecar)                         │
│  Rust 服务 ↔ Python AI 推理 sidecar                            │
│  协议：gRPC (Unix Domain Socket 本地通信，延迟<1ms)           │
│  模式：Server Streaming（Token 流式输出）                       │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 技术选型

| 通信类型 | 技术选型 | 版本 | 理由 |
|----------|----------|------|------|
| **内部 RPC** | tonic (gRPC) | 0.12 | Rust 生态最成熟、流式原生、Protobuf 强类型、跨语言、性能高 |
| **外部 REST** | axum | 0.7 | 已有、tokio 生态、tower 中间件 |
| **gRPC-Web** | tonic-web | 0.12 | 浏览器直接调用 gRPC，无需 REST 转码 |
| **WebSocket** | tokio-tungstenite | 0.21 | 已有、长连接推送 |
| **消息队列** | NATS JetStream（推荐）/ RabbitMQ (lapin) | NATS 2.x / lapin 2.x | NATS 轻量 Rust 原生好；RabbitMQ 企业级成熟 |
| **服务发现** | K8s Service + CoreDNS（起步）→ Nacos（规模化） | - | K8s 原生零额外组件 |
| **配置中心** | K8s ConfigMap + Secret（起步）→ Nacos（规模化） | - | 渐进式 |
| **外部 HTTP** | reqwest | 0.12 | 已有、异步、流式 |

---

## 三、gRPC 通信架构

### 3.1 gRPC 技术栈

```toml
# Cargo.toml workspace.dependencies
tonic = { version = "0.12", features = ["tls", "prost", "gzip"] }
tonic-build = "0.12"
tonic-web = "0.12"
tonic-reflection = "0.12"
tonic-health = "0.12"
prost = "0.13"
prost-build = "0.13"
prost-types = "0.13"
```

### 3.2 Proto 目录结构

```
proto/
├── common/
│   ├── common.proto          # RequestMeta, PageRequest, PageResponse, ErrorCode, Empty
│   └── health.proto          # 健康检查（标准 grpc.health.v1）
├── gateway/
│   └── gateway.proto         # 网关专用
├── auth/
│   └── auth.proto            # 登录/登出/Token/权限/角色
├── tenant/
│   └── tenant.proto          # 租户CRUD/配额/用量
├── system/
│   └── system.proto          # 用户/角色/部门/菜单/字典
├── ai/
│   ├── ai.proto              # GenerateStream/Generate/Embed/Retrieve/Chat
│   ├── agent.proto           # Agent CRUD/Run/Tools
│   └── expert.proto          # 规则/推理/知识库
├── graph/
│   ├── storage.proto         # 顶点/边CRUD/邻居/扫描/CDC/分片管理
│   ├── graph.proto           # 图谱CRUD/查询/本体/摄入/合并
│   ├── algorithm.proto       # 路径/中心性/社区/连通性/子图
│   ├── streams.proto         # 流处理/订阅
│   └── meta.proto            # Schema/索引/约束/版本
├── storage/
│   └── storage.proto         # 文件/对象/桶/分片/分享
├── etl/
│   └── etl.proto             # 数据源/管道/执行/调度
├── flow/
│   ├── flow.proto            # 工作流定义/执行/节点
│   └── fusion.proto          # 多流程编排/事件驱动
├── operator/
│   └── operator.proto        # 算子注册/执行/市场
├── metering/
│   └── metering.proto        # 用量/配额/账单
├── notification/
│   └── notification.proto    # 通知/模板/订阅
├── search/
│   └── search.proto          # 全文/向量/图谱/联合搜索
├── compliance/
│   └── compliance.proto      # 审计/合规/数据主体请求
├── fusion/
│   └── fusion.proto          # 数据融合/实体对齐/冲突解决
├── catalog/
│   └── catalog.proto         # 数据资产/术语/血缘
├── market/
│   └── market.proto          # 模板/插件/市场
└── optimizer/
    └── optimizer.proto       # 查询优化/执行计划/成本模型
```

### 3.3 公共 Proto 定义

```protobuf
// proto/common/common.proto
syntax = "proto3";
package mox.common;

option java_multiple_files = true;
option java_package = "com.infotopograph.mox.common";

// 请求元数据：所有 gRPC 请求必须携带
message RequestMeta {
  string tenant_id = 1;          // 租户 ID（必填）
  string user_id = 2;            // 用户 ID
  string trace_id = 3;           // 链路追踪 ID
  string request_id = 4;         // 请求 ID（幂等用）
  map<string, string> headers = 5;
}

// 分页请求
message PageRequest {
  int32 page_num = 1;
  int32 page_size = 2;
  string order_by = 3;
}

// 分页响应
message PageResponse {
  int64 total = 1;
  int32 page_num = 2;
  int32 page_size = 3;
}

// 统一错误码
enum ErrorCode {
  OK = 0;
  UNAUTHENTICATED = 1;
  PERMISSION_DENIED = 2;
  TENANT_NOT_FOUND = 3;
  TENANT_SUSPENDED = 4;
  QUOTA_EXCEEDED = 5;
  RATE_LIMITED = 6;
  INVALID_ARGUMENT = 7;
  NOT_FOUND = 8;
  ALREADY_EXISTS = 9;
  INTERNAL = 10;
  UNAVAILABLE = 11;
  DEADLINE_EXCEEDED = 12;
  // AI 专用
  AI_TIMEOUT = 100;
  AI_RATE_LIMITED = 101;
  AI_MODEL_UNAVAILABLE = 102;
  AI_CONTENT_BLOCKED = 103;
  // 图谱专用
  GRAPH_SHARD_NOT_FOUND = 200;
  GRAPH_VID_NOT_FOUND = 201;
  GRAPH_EDGE_NOT_FOUND = 202;
}

// 统一响应状态
message Status {
  ErrorCode code = 1;
  string message = 2;
  map<string, string> details = 3;
}
```

### 3.4 gRPC 服务端标准模板

```rust
// services/ai/src/main.rs
use tonic::{transport::Server, Request, Response, Status};
use mox_rpc::interceptor::MoxInterceptorChain;
use mox_o11y::init_tracing;
use mox_config::MoxConfig;
use mox_discovery::register_service;
use ai_proto::ai_service_server::{AiService, AiServiceServer};
use ai_proto::{GenerateRequest, GenerateResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化可观测性
    init_tracing("mox-ai-svc").await?;

    // 2. 加载配置
    let config = MoxConfig::load("mox-ai-svc").await?;

    // 3. 注册服务发现
    let _lease = register_service("mox-ai-svc", &config.grpc_addr).await?;

    // 4. 构建拦截器链
    let interceptor = MoxInterceptorChain::builder()
        .tenant()
        .auth()
        .trace()
        .rate_limit()
        .log()
        .validation()
        .build(&config)
        .await?;

    // 5. 创建服务实现
    let svc = AiServiceImpl::new(&config).await?;

    // 6. 构建 gRPC 服务（带拦截器）
    let ai_svc = AiServiceServer::with_interceptor(svc, interceptor);

    // 7. 健康检查 + 反射
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter.set_serving::<AiServiceServer<AiServiceImpl>>().await;

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(ai_proto::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(mox_common::FILE_DESCRIPTOR_SET)
        .build()?;

    // 8. 启动服务
    let addr = config.grpc_addr.parse()?;
    tracing::info!("mox-ai-svc listening on {}", addr);

    Server::builder()
        .add_service(health_service)
        .add_service(reflection_service)
        .add_service(ai_svc)
        .serve(addr)
        .await?;

    Ok(())
}
```

### 3.5 gRPC 拦截器链

```rust
// libs/mox-rpc/src/interceptor/mod.rs
use tonic::{Request, Status, service::Interceptor};

pub struct MoxInterceptorChain {
    tenant: TenantInterceptor,
    auth: AuthInterceptor,
    trace: TraceInterceptor,
    rate_limit: RateLimitInterceptor,
    log: LogInterceptor,
    validation: ValidationInterceptor,
}

impl Interceptor for MoxInterceptorChain {
    fn call(&mut self, req: Request<()>) -> Result<Request<()>, Status> {
        // 执行顺序：租户 → 认证 → 追踪 → 限流 → 日志 → 校验
        let req = self.tenant.call(req)?;
        let req = self.auth.call(req)?;
        let req = self.trace.call(req)?;
        let req = self.rate_limit.call(req)?;
        let req = self.log.call(req)?;
        let req = self.validation.call(req)?;
        Ok(req)
    }
}
```

**各拦截器职责**：

| 拦截器 | 职责 | 失败处理 |
|--------|------|----------|
| TenantInterceptor | 从 metadata 提取 tenant_id，写入 extensions；验证租户存在且未停用 | UNAUTHENTICATED / TENANT_NOT_FOUND / TENANT_SUSPENDED |
| AuthInterceptor | 验证 JWT，提取用户信息，预校验权限 | UNAUTHENTICATED / PERMISSION_DENIED |
| TraceInterceptor | 注入/提取 trace_id，OTel 上下文传播 | 不阻断（追踪失败不影响业务） |
| RateLimitInterceptor | 按租户/用户/接口限流，检查配额 | RATE_LIMITED / QUOTA_EXCEEDED |
| LogInterceptor | 记录方法/参数/耗时/状态码/错误 | 不阻断 |
| ValidationInterceptor | 校验请求消息结构（必填字段/格式） | INVALID_ARGUMENT |

### 3.6 gRPC 客户端封装

```rust
// libs/mox-rpc/src/client.rs
use tonic::transport::Channel;
use mox_discovery::resolve;

pub struct MoxChannel {
    inner: Channel,
    service_name: String,
}

impl MoxChannel {
    pub fn builder(service_name: &str) -> MoxChannelBuilder {
        MoxChannelBuilder::new(service_name)
    }

    pub fn inner(&self) -> &Channel {
        &self.inner
    }
}

pub struct MoxChannelBuilder {
    service_name: String,
    load_balancer: Option<LoadBalancer>,
    retry_policy: Option<RetryPolicy>,
    timeout: Duration,
    circuit_breaker: bool,
}

impl MoxChannelBuilder {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
            load_balancer: None,
            retry_policy: None,
            timeout: Duration::from_secs(3),
            circuit_breaker: false,
        }
    }

    pub fn with_load_balancer(mut self) -> Self {
        self.load_balancer = Some(LoadBalancer::RoundRobin);
        self
    }

    pub fn with_retry(mut self, max_attempts: u32) -> Self {
        self.retry_policy = Some(RetryPolicy::new(max_attempts));
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_circuit_breaker(mut self) -> Self {
        self.circuit_breaker = true;
        self
    }

    pub async fn connect(self) -> Result<MoxChannel, RpcError> {
        // 从注册中心获取地址列表
        let addrs = resolve(&self.service_name).await?;
        if addrs.is_empty() {
            return Err(RpcError::ServiceNotFound(self.service_name));
        }

        // 构建带负载均衡的 channel
        let channel = Channel::balance_list(addrs.into_iter().map(|a| a.into()));

        Ok(MoxChannel {
            inner: channel,
            service_name: self.service_name,
        })
    }
}

// 使用方式
let channel = MoxChannel::builder("mox-graph-storage-svc")
    .with_load_balancer()
    .with_retry(3)
    .with_timeout(Duration::from_secs(5))
    .with_circuit_breaker()
    .connect()
    .await?;

let mut client = GraphStorageClient::new(channel.inner().clone());
let req = Request::new(AddVertexRequest { ... }).with_tenant(&ctx.tenant_id).with_trace();
let resp = client.add_vertex(req).await?;
```

### 3.7 流式通信模式

| 模式 | 用途 | 示例 |
|------|------|------|
| **Unary** | 普通请求响应 | GetVertex, CreateTenant |
| **Server Streaming** | 服务端持续推送 | AI GenerateStream, 图 ScanVertices, CDC Subscribe |
| **Client Streaming** | 客户端持续上传 | 大文件上传, 批量数据导入 |
| **Bidirectional** | 双向流式 | 实时对话, WebSocket 桥接 |

**AI 流式生成示例**：
```protobuf
service AIService {
  rpc GenerateStream(GenerateRequest) returns (stream GenerateResponse);
}

message GenerateRequest {
  mox.common.RequestMeta meta = 1;
  string prompt = 2;
  string model = 3;
  double temperature = 4;
  int32 max_tokens = 5;
}

message GenerateResponse {
  string chunk = 1;          // token 片段
  bool is_end = 2;
  string model = 3;
  int32 prompt_tokens = 4;
  int32 completion_tokens = 5;
}
```

```rust
// 服务端流式实现
async fn generate_stream(
    &self,
    req: Request<GenerateRequest>,
) -> Result<Response<Self::GenerateStreamStream>, Status> {
    let req = req.into_inner();
    let (tx, rx) = mpsc::channel(128);

    // 调用 Python 推理 sidecar（gRPC streaming）
    let mut inference = self.inference_client.generate_stream(req).await?.into_inner();

    tokio::spawn(async move {
        while let Some(chunk) = inference.message().await.unwrap() {
            // Guardrails 校验 + 成本统计 + 缓存写入
            let processed = process_chunk(chunk).await;
            if tx.send(Ok(processed)).await.is_err() {
                break;
            }
        }
    });

    let stream = ReceiverStream::new(rx);
    Ok(Response::new(Box::pin(stream) as Self::GenerateStreamStream))
}
```

---

## 四、API 网关通信架构

### 4.1 多协议单端口架构

```
                    单端口 8080
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
   REST (JSON)     gRPC-Web       WebSocket/SSE
   (axum routes)   (tonic-web)    (tokio-tungstenite)
        │               │               │
        └───────────────┼───────────────┘
                        ▼
              网关中间件链（统一处理）
              1. TenantMiddleware（租户识别）
              2. AuthMiddleware（JWT验证）
              3. RateLimitMiddleware（限流）
              4. TraceMiddleware（OTel追踪）
              5. LogMiddleware（访问日志）
              6. CorsMiddleware（跨域）
                        │
                        ▼
              路由分发
              ├── REST → gRPC 转码（REST endpoint → gRPC call）
              ├── gRPC-Web → gRPC（直接转发）
              └── WebSocket → 长连接服务（graph-streams / notification）
                        │
                        ▼
              后端 gRPC 服务（tonic client，负载均衡+重试+熔断）
```

### 4.2 REST → gRPC 转码

网关将 REST 请求自动转码为 gRPC 调用：

```
REST: GET /api/v1/graphs/{graph_id}/vertices?page_num=1&page_size=20
  ↓ 转码
gRPC: GraphService.GetVertices(GetVerticesRequest {
    meta: RequestMeta { tenant_id, user_id, trace_id },
    graph_id: "...",
    page: PageRequest { page_num: 1, page_size: 20 }
})
```

转码规则通过 proto 注解或配置文件定义：

```yaml
# gateway/routes.yaml
routes:
  - name: list_vertices
    rest:
      method: GET
      path: /api/v1/graphs/{graph_id}/vertices
    grpc:
      service: mox.graph.GraphService
      method: GetVertices
      request_mapping:
        graph_id: path.graph_id
        page.page_num: query.page_num
        page.page_size: query.page_size
    auth: required
    rate_limit: 100/min
```

### 4.3 网关核心能力

| 能力 | 实现 | 说明 |
|------|------|------|
| **多协议入口** | axum + tonic-web + tokio-tungstenite | 单端口支持 REST/gRPC-Web/WS |
| **租户识别** | subdomain / JWT claims / X-Tenant-Id header | 从请求提取 tenant_id |
| **统一认证** | JWT 验证 + 调用 auth-svc | 登录/登出/Token刷新入口 |
| **限流熔断** | 按租户/用户/接口/IP 限流 | Redis 滑动窗口 + 令牌桶 |
| **路由分发** | REST→gRPC 转码 / gRPC-Web 直转 | 基于配置的路由规则 |
| **灰度发布** | 按 header/cookie/租户比例路由 | 金丝雀发布/蓝绿部署 |
| **API 文档** | Swagger UI + gRPC Reflection | 自动生成 API 文档 |
| **健康检查** | /health + /ready + gRPC Health | K8s liveness/readiness |
| **请求聚合** | BFF 层并行调用多个服务 | 减少前端请求次数 |

---

## 五、异步事件通信架构

### 5.1 消息队列选型对比

| 维度 | NATS JetStream | RabbitMQ (lapin) | Redis Streams |
|------|----------------|-------------------|---------------|
| Rust 客户端 | async-nats（官方，优秀） | lapin（成熟） | redis（成熟） |
| 持久化 | ✅ JetStream | ✅ 持久化队列 | ✅ Streams |
| 消息确认 | ✅ Ack/Nak | ✅ Ack/Nack | ✅ XACK |
| 死信队列 | ✅ | ✅ | ❌ 需自建 |
| 延迟消息 | ✅ | ✅（插件） | ❌ |
| 顺序消息 | ✅ | ✅ | ✅ |
| 事务消息 | ❌ | ✅ | ❌ |
| 性能 | 极高（~10M msg/s） | 高（~100K msg/s） | 高 |
| 运维复杂度 | 低（单二进制） | 中（Erlang/OTP） | 低 |
| 企业级特性 | 中 | 高（管理界面/插件） | 中 |
| **推荐场景** | **高吞吐/云原生/微服务** | **企业级/复杂路由/事务** | **简单队列/缓存** |

**推荐：NATS JetStream**（轻量、高性能、Rust 原生支持好、云原生友好）

### 5.2 事件驱动架构

```
服务 A (发布事件)
  │
  ▼
NATS JetStream (消息总线)
  │
  ├──→ 服务 B (订阅事件，同步处理)
  ├──→ 服务 C (订阅事件，异步处理)
  ├──→ 服务 D (订阅事件，写入数据库)
  └──→ 审计服务 (订阅所有事件，不可篡改存储)
```

### 5.3 事件主题规范

```
主题格式：{tenant_id}.{domain}.{entity}.{action}

示例：
  tenant-123.ai.generation.completed
  tenant-123.graph.vertex.created
  tenant-123.graph.edge.updated
  tenant-123.storage.file.uploaded
  tenant-123.flow.workflow.started
  tenant-123.flow.workflow.completed
  tenant-123.system.user.created
  tenant-123.metering.usage.reported
  system.audit.log.recorded
```

### 5.4 事件消息格式

```protobuf
// proto/common/event.proto
message CloudEvent {
  string spec_version = 1;      // CloudEvents 规范版本
  string id = 2;                // 事件唯一 ID
  string source = 3;            // 事件来源（服务名）
  string type = 4;              // 事件类型（ai.generation.completed）
  string time = 5;              // 事件时间（RFC3339）
  string tenant_id = 6;         // 租户 ID
  string subject = 7;           // 事件主体（资源 ID）
  map<string, string> extensions = 8;  // 扩展属性
  bytes data = 9;               // 事件数据（JSON/Protobuf）
  string data_content_type = 10; // 数据类型
}
```

### 5.5 事件发布/订阅示例

```rust
// 发布事件
use async_nats::jetstream;

let js = jetstream::new(client);
let event = CloudEvent {
    spec_version: "1.0".into(),
    id: uuid::Uuid::new_v4().to_string(),
    source: "mox-ai-svc".into(),
    type_: "ai.generation.completed".into(),
    time: chrono::Utc::now().to_rfc3339(),
    tenant_id: ctx.tenant_id.clone(),
    subject: generation_id.into(),
    data: serde_json::to_vec(&result)?,
    data_content_type: "application/json".into(),
    ..Default::default()
};

let subject = format!("{}.ai.generation.completed", ctx.tenant_id);
js.publish(subject, serde_json::to_vec(&event)?).await?;
```

```rust
// 订阅事件
let consumer = js.create_consumer(
    "ai-events",
    jetstream::consumer::pull::Config {
        durable_name: Some("ai-event-consumer"),
        filter_subject: format!("{}.ai.>", tenant_id),
        ack_policy: jetstream::consumer::AckPolicy::Explicit,
        max_deliver: 3,
        ..Default::default()
    },
).await?;

let mut messages = consumer.messages().await?;
while let Some(message) = messages.next().await {
    let event: CloudEvent = serde_json::from_slice(&message.payload)?;
    match event.type_.as_str() {
        "ai.generation.completed" => handle_generation_completed(event).await?,
        _ => tracing::warn!("unhandled event type: {}", event.type_),
    }
    message.ack().await?;
}
```

### 5.6 Saga 模式（跨服务数据一致性）

对于需要跨服务一致性的操作，用 Saga 模式（事件驱动，最终一致性）：

```
示例：创建 AI Agent（涉及 agent-svc + ai-svc + metering-svc + notification-svc）

1. agent-svc: 创建 Agent（本地事务）
   → 发布事件: agent.created

2. ai-svc: 订阅 agent.created
   → 初始化 AI 配置（本地事务）
   → 发布事件: agent.ai.initialized

3. metering-svc: 订阅 agent.created
   → 创建用量记录（本地事务）
   → 发布事件: agent.metering.initialized

4. notification-svc: 订阅 agent.created
   → 发送创建通知（本地事务）

5. 如果任何步骤失败：
   → 发布补偿事件: agent.creation.failed
   → 各服务执行补偿操作（回滚）
```

---

## 六、服务注册与发现

### 6.1 两阶段方案

| 阶段 | 方案 | 适用场景 |
|------|------|----------|
| **阶段一（起步）** | K8s Service + CoreDNS | 全量部署在 K8s，零额外组件 |
| **阶段二（规模化）** | Nacos（注册中心+配置中心） | 多环境/多集群/权重路由/灰度 |

### 6.2 K8s Service 方案

每个服务一个 K8s Service，通过 DNS 名称访问：

```yaml
# mox-ai-svc Service
apiVersion: v1
kind: Service
metadata:
  name: mox-ai-svc
  labels:
    app: mox-ai-svc
spec:
  selector:
    app: mox-ai-svc
  ports:
  - name: grpc
    port: 50051
    targetPort: 50051
  - name: metrics
    port: 9090
    targetPort: 9090
```

客户端通过 `http://mox-ai-svc:50051` 访问，K8s Service 自动做负载均衡。

### 6.3 Nacos 方案（规模化）

```rust
// libs/mox-discovery/src/nacos.rs
use nacos_sdk::api::{
    config::ConfigService,
    naming::{NamingService, ServiceInstance},
};

pub async fn register_service(
    service_name: &str,
    addr: &str,
) -> Result<ServiceLease, DiscoveryError> {
    let naming = nacos_sdk::client::NamingServiceBuilder::new()
        .server_addr("nacos:8848")
        .build()?;

    let (host, port) = addr.split_once(':').unwrap();
    let instance = ServiceInstance {
        ip: host.to_string(),
        port: port.parse()?,
        service_name: service_name.to_string(),
        weight: 1.0,
        healthy: true,
        enabled: true,
        metadata: {
            let mut m = std::collections::HashMap::new();
            m.insert("version".into(), env!("CARGO_PKG_VERSION").into());
            m.insert("grpc".into(), "true".into());
            Some(m)
        },
        ..Default::default()
    };

    naming.register_instance(instance).await?;

    // 心跳保活
    let lease = ServiceLease::new(naming, service_name, addr);
    lease.start_heartbeat();

    Ok(lease)
}

pub async fn resolve(service_name: &str) -> Result<Vec<String>, DiscoveryError> {
    let naming = nacos_sdk::client::NamingServiceBuilder::new()
        .server_addr("nacos:8848")
        .build()?;

    let instances = naming.get_instances(service_name, None, true).await?;
    Ok(instances.iter().map(|i| format!("{}:{}", i.ip, i.port)).collect())
}
```

---

## 七、通信安全

### 7.1 服务间 mTLS

| 方案 | 实现 | 适用 |
|------|------|------|
| **Istio mTLS** | Sidecar 代理自动 mTLS | K8s + Istio 环境，零代码改动 |
| **tonic TLS** | tonic 原生 TLS（rustls） | 非 Istio 环境，代码级配置 |

**推荐：Istio mTLS**（零代码改动，统一管理证书，自动轮换）

```yaml
# Istio PeerAuthentication（强制 mTLS）
apiVersion: security.istio.io/v1beta1
kind: PeerAuthentication
metadata:
  name: default
  namespace: mox
spec:
  mtls:
    mode: STRICT
```

### 7.2 外部通信 TLS

- 网关入口：TLS 1.3（证书由 cert-manager 自动管理）
- API 调用：HTTPS（强制）
- WebSocket：WSS（强制）

### 7.3 服务身份认证

每个服务有独立的 ServiceAccount（K8s），通过 SPIFFE/SPIRE 颁发身份证书，用于 mTLS 和服务间认证。

---

## 八、通信性能优化

### 8.1 性能指标目标

| 指标 | 目标 |
|------|------|
| gRPC Unary P99 延迟 | < 10ms（同机房） |
| gRPC Streaming 首包延迟 | < 50ms（AI 生成） |
| 服务间吞吐量 | > 10K QPS/服务 |
| NATS 消息延迟 | < 1ms |
| 网关 P99 延迟 | < 20ms |

### 8.2 优化手段

| 优化点 | 手段 |
|--------|------|
| **连接复用** | tonic Channel 复用 HTTP/2 连接，连接池 |
| **负载均衡** | 客户端负载均衡（RoundRobin/LeastConnection） |
| **批量处理** | 批量写入/读取，减少 RPC 次数 |
| **压缩** | gRPC gzip 压缩（大消息） |
| **缓存** | 客户端缓存（热点数据）+ 服务端缓存 |
| **异步并行** | 并行调用多个无依赖服务 |
| **连接预热** | 服务启动时预建连接 |
| **背压控制** | 流式通信的背压（flow control） |
| **零拷贝** | 大文件传输用 sendfile/零拷贝 |
| **Protobuf 优化** | 合理设计消息结构，避免嵌套过深 |

---

## 九、总结

通信架构优化的核心是**"gRPC 优先、网关统一、事件解耦、渐进式基础设施"**：

1. **内部通信全面 gRPC 化**：tonic + Protobuf，支持 Unary + Streaming，服务间高效通信
2. **API 网关多协议单端口**：REST + gRPC-Web + WebSocket 统一入口，REST→gRPC 自动转码
3. **异步事件 NATS JetStream**：非核心路径事件驱动，解耦服务，削峰填谷，最终一致性
4. **服务发现两阶段**：K8s Service（起步）→ Nacos（规模化）
5. **通信安全 mTLS**：Istio 自动 mTLS，零代码改动
6. **性能目标明确**：gRPC P99 < 10ms，AI 流式首包 < 50ms

---

*下一篇：[03-数据架构优化](./03-data.md)*

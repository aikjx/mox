# 05 - 可观测性 · 安全 · 弹性

> 版本：v1.0 | 日期：2026-08-26 | 状态：草案
>
> 前置阅读：[00-核心原则](./00-principles.md) | [01-服务边界优化](./01-service-boundaries.md) | [02-通信架构优化](./02-communication.md) | [03-数据架构优化](./03-data.md) | [04-部署架构优化](./04-deployment.md)

---

## 第一部分：可观测性

### 一、现状诊断

| 维度 | 现状 | 问题 |
|------|------|------|
| 日志 | 文件日志（散落在根目录，30+ .log 文件） | 无结构化、无聚合、无检索 |
| 指标 | prometheus 0.13（部分服务） | 不完整、无统一仪表盘、无告警 |
| 链路追踪 | 无 | 分布式排查困难，无法定位瓶颈 |
| 告警 | 无 | 故障发现靠用户反馈，MTTR 高 |

### 二、目标可观测性架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        应用层（所有服务）                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │ gateway  │  │  ai-svc  │  │graph-svc │  │  ...     │       │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘       │
│       │ OTel SDK      │              │              │             │
│       ▼               ▼              ▼              ▼             │
│  ┌─────────────────────────────────────────────────────────┐     │
│  │              OpenTelemetry Collector                      │     │
│  │    (统一接收：Traces + Metrics + Logs)                    │     │
│  └────┬──────────────┬──────────────┬──────────────────────┘     │
│       │              │              │                             │
│       ▼              ▼              ▼                             │
│  ┌─────────┐   ┌──────────┐  ┌─────────┐                        │
│  │ Jaeger  │   │Prometheus│  │  Loki   │                        │
│  │(链路追踪)│   │ (指标)   │  │ (日志)  │                        │
│  └────┬────┘   └────┬─────┘  └────┬────┘                        │
│       │              │              │                             │
│       └──────────────┼──────────────┘                             │
│                      ▼                                             │
│              ┌───────────────┐                                     │
│              │   Grafana     │                                     │
│              │ (统一仪表盘)   │                                     │
│              └───────┬───────┘                                     │
│                      │                                             │
│                      ▼                                             │
│              ┌───────────────┐                                     │
│              │ Alertmanager  │                                     │
│              │  (告警路由)    │                                     │
│              └───────┬───────┘                                     │
│                      ▼                                             │
│         飞书/钉钉/邮件/PagerDuty                                   │
└─────────────────────────────────────────────────────────────────┘
```

### 三、三大支柱

#### 3.1 日志（Logs）

**技术选型**：tracing + tracing-subscriber（结构化日志）→ OTel Collector → Loki

**日志规范**：

```rust
// libs/mox-o11y/src/logging.rs
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt};
use tracing::info;

pub fn init_logging(service_name: &str) {
    // JSON 格式结构化日志
    let fmt_layer = fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .flatten_event(true);

    // 日志级别过滤
    let filter_layer = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set global subscriber");

    info!(service = service_name, "logging initialized");
}
```

**日志字段规范**（所有日志必须包含）：

| 字段 | 说明 | 示例 |
|------|------|------|
| timestamp | 时间（RFC3339） | 2026-08-26T10:00:00Z |
| level | 日志级别 | INFO / WARN / ERROR |
| service | 服务名 | mox-ai-svc |
| tenant_id | 租户ID | tenant-123 |
| user_id | 用户ID | user-456 |
| trace_id | 链路ID | abc123def456 |
| request_id | 请求ID | req-789 |
| message | 日志消息 | "AI generation completed" |
| duration_ms | 耗时（可选） | 1234 |
| error | 错误信息（可选） | "connection refused" |

**日志级别使用规范**：

| 级别 | 使用场景 | 示例 |
|------|----------|------|
| ERROR | 系统错误，需要人工介入 | 数据库连接失败、服务崩溃 |
| WARN | 潜在问题，可自动恢复 | 重试成功、缓存未命中率高 |
| INFO | 关键业务事件 | 请求完成、用户登录、任务创建 |
| DEBUG | 调试信息（生产默认关闭） | 详细参数、中间状态 |
| TRACE | 极详细追踪（默认关闭） | 函数级调用 |

**敏感数据脱敏**：
```rust
// libs/mox-o11y/src/sensitive.rs
pub fn sanitize(value: &str) -> String {
    // 手机号脱敏
    let value = regex!(r"1[3-9]\d{9}").replace_all(value, "1*******$1");
    // 邮箱脱敏
    let value = regex!(r"(\w)[\w.]*@(\w+\.\w+)").replace_all(&value, "$1***@$2");
    // 身份证脱敏
    let value = regex!(r"\d{17}[\dXx]").replace_all(&value, "*****************");
    // 密码/Token
    let value = regex!(r"(password|token|secret|api_key)\s*[:=]\s*\S+")
        .replace_all(&value, "$1=***REDACTED***");
    value.to_string()
}
```

#### 3.2 指标（Metrics）

**技术选型**：metrics + prometheus（Rust 客户端）→ Prometheus → Grafana

**核心指标分类**：

| 类别 | 指标 | 类型 | 说明 |
|------|------|------|------|
| **RED（请求）** | requests_total | Counter | 请求总数（按方法/状态/租户） |
| | request_duration_seconds | Histogram | 请求延迟分布 |
| | request_errors_total | Counter | 错误请求数（按错误码） |
| **USE（资源）** | cpu_usage_percent | Gauge | CPU 使用率 |
| | memory_usage_bytes | Gauge | 内存使用量 |
| | disk_io_bytes_total | Counter | 磁盘 IO |
| | network_io_bytes_total | Counter | 网络 IO |
| **业务指标** | ai_generations_total | Counter | AI 生成次数 |
| | ai_tokens_total | Counter | AI Token 消耗 |
| | graph_vertices_total | Gauge | 图谱顶点数 |
| | graph_edges_total | Gauge | 图谱边数 |
| | active_users | Gauge | 活跃用户数 |
| | tenant_quota_usage | Gauge | 租户配额使用率 |
| **JVM/Runtime** | tokio_tasks_count | Gauge | tokio 任务数 |
| | db_pool_connections | Gauge | 数据库连接池状态 |
| | redis_commands_total | Counter | Redis 命令数 |

**Rust 指标实现**：

```rust
// libs/mox-o11y/src/metrics.rs
use metrics::{counter, histogram, gauge};
use std::time::Instant;

pub struct RequestMetrics {
    service: String,
}

impl RequestMetrics {
    pub fn new(service: &str) -> Self {
        Self { service: service.to_string() }
    }

    pub fn record_request(&self, method: &str, status: &str, tenant_id: &str, duration: Instant) {
        let duration_secs = duration.elapsed().as_secs_f64();
        counter!("mox_requests_total",
            "service" => self.service.clone(),
            "method" => method.to_string(),
            "status" => status.to_string(),
            "tenant_id" => tenant_id.to_string(),
        ).increment(1);
        histogram!("mox_request_duration_seconds",
            "service" => self.service.clone(),
            "method" => method.to_string(),
            "tenant_id" => tenant_id.to_string(),
        ).record(duration_secs);
    }

    pub fn record_error(&self, method: &str, error_code: &str, tenant_id: &str) {
        counter!("mox_request_errors_total",
            "service" => self.service.clone(),
            "method" => method.to_string(),
            "error_code" => error_code.to_string(),
            "tenant_id" => tenant_id.to_string(),
        ).increment(1);
    }
}
```

**Prometheus 抓取配置**：
```yaml
# prometheus.yml
scrape_configs:
- job_name: 'mox-services'
  kubernetes_sd_configs:
  - role: pod
  relabel_configs:
  - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
    action: keep
    regex: true
  - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_port]
    action: replace
    target_label: __metrics_path__
    regex: (.+)
    replacement: /metrics
  - source_labels: [__address__, __meta_kubernetes_pod_annotation_prometheus_io_port]
    action: replace
    regex: ([^:]+)(?::\d+)?;(\d+)
    replacement: $1:$2
    target_label: __address__
```

#### 3.3 链路追踪（Traces）

**技术选型**：tracing-opentelemetry + opentelemetry-otlp → OTel Collector → Jaeger

**Rust 链路追踪实现**：

```rust
// libs/mox-o11y/src/tracing.rs
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_otlp::WithExportConfig;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::registry;

pub fn init_tracing(service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // OTLP 导出器（发送到 OTel Collector）
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://otel-collector:4317");

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            opentelemetry::sdk::trace::config()
                .with_resource(opentelemetry::sdk::Resource::new(vec![
                    opentelemetry::KeyValue::new("service.name", service_name),
                    opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ]))
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    global::set_tracer_provider(provider);

    // 集成到 tracing
    let tracer = global::tracer(service_name);
    let otel_layer = OpenTelemetryLayer::new(tracer);

    let subscriber = registry()
        .with(otel_layer);
    // ... 与日志层组合

    Ok(())
}
```

**gRPC 拦截器中的 Trace 传播**：

```rust
// libs/mox-rpc/src/interceptor/trace.rs
use tonic::{Request, Status, service::Interceptor};
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry::global;

pub struct TraceInterceptor;

impl Interceptor for TraceInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        // 从 gRPC metadata 提取 trace context（W3C TraceContext）
        let propagator = global::get_text_map_propagator(|prop| prop.clone());
        let parent_cx = propagator.extract(&MetadataCarrier(req.metadata()));

        // 设置当前上下文
        opentelemetry::Context::current_with_remote_span_context(
            parent_cx.span().span_context().clone()
        ).attach();

        // 生成新的 span
        let span = tracing::info_span!(
            "grpc_request",
            method = %req.method(),
            service = %req.service_name(),
        );
        let _enter = span.enter();

        Ok(req)
    }
}
```

**链路追踪关键 Span**：

```
gateway.request (入口)
  ├── auth.verify (认证)
  ├── tenant.check (租户配额)
  ├── ai.generate (AI 生成)
  │   ├── ai.prompt_render (Prompt 渲染)
  │   ├── ai.cache_lookup (缓存查找)
  │   ├── search.retrieve (RAG 检索)
  │   │   ├── graph-storage.query (图谱查询)
  │   │   └── pgvector.search (向量搜索)
  │   ├── inference.call (Python 推理)
  │   │   └── inference.stream (流式生成)
  │   ├── ai.guardrails (安全校验)
  │   └── metering.record (用量记录)
  └── notification.send (通知)
```

### 四、统一仪表盘（Grafana）

| 仪表盘 | 内容 | 受众 |
|--------|------|------|
| **全局概览** | 服务健康状态、QPS、错误率、延迟 P50/P95/P99、资源使用率 | 运维/管理层 |
| **服务详情** | 单服务的 RED 指标、慢请求、错误分布、依赖拓扑 | 开发/运维 |
| **业务指标** | AI 生成量、Token 消耗、图谱规模、活跃用户、租户配额 | 产品/管理层 |
| **基础设施** | K8s 节点状态、Pod 状态、资源使用、网络 IO | 运维 |
| **链路追踪** | 慢链路 Top N、错误链路、服务依赖图 | 开发 |
| **日志分析** | 错误日志趋势、异常模式、日志量 | 开发/运维 |
| **告警面板** | 当前告警、告警历史、告警趋势 | 运维 |

### 五、告警体系

**告警分级**：

| 级别 | 定义 | 响应时间 | 通知方式 | 示例 |
|------|------|----------|----------|------|
| **P0 紧急** | 核心服务不可用，影响所有用户 | 5 分钟 | 电话+短信+飞书+PagerDuty | 网关宕机、数据库主库故障 |
| **P1 严重** | 核心功能异常，影响部分用户 | 15 分钟 | 短信+飞书+PagerDuty | AI 生成失败率>10%、图谱查询超时 |
| **P2 警告** | 非核心功能异常，或性能下降 | 1 小时 | 飞书+邮件 | 单服务错误率>5%、磁盘使用率>80% |
| **P3 提示** | 潜在问题，需关注 | 4 小时 | 邮件 | 缓存命中率下降、慢查询增加 |

**核心告警规则**：

```yaml
# Prometheus AlertManager 规则
groups:
- name: mox-critical
  rules:
  - alert: ServiceDown
    expr: up{job="mox-services"} == 0
    for: 1m
    labels:
      severity: P0
    annotations:
      summary: "Service {{ $labels.service }} is down"
      description: "Service {{ $labels.service }} has been down for more than 1 minute"

  - alert: HighErrorRate
    expr: |
      sum(rate(mox_request_errors_total{service=~"mox-(gateway|ai|graph).*"}[5m]))
      /
      sum(rate(mox_requests_total{service=~"mox-(gateway|ai|graph).*"}[5m]))
      > 0.05
    for: 5m
    labels:
      severity: P1
    annotations:
      summary: "High error rate on {{ $labels.service }}"
      description: "Error rate is {{ $value | humanizePercentage }} for 5 minutes"

  - alert: HighLatency
    expr: |
      histogram_quantile(0.99,
        sum(rate(mox_request_duration_seconds_bucket[5m])) by (le, service)
      ) > 1
    for: 5m
    labels:
      severity: P1
    annotations:
      summary: "High P99 latency on {{ $labels.service }}"
      description: "P99 latency is {{ $value }}s for 5 minutes"

  - alert: DiskUsageHigh
    expr: node_filesystem_avail_bytes / node_filesystem_size_bytes < 0.2
    for: 10m
    labels:
      severity: P2
    annotations:
      summary: "Disk usage high on {{ $labels.instance }}"
      description: "Disk usage is above 80%"

  - alert: TenantQuotaExceeded
    expr: mox_tenant_quota_usage > 0.9
    for: 5m
    labels:
      severity: P2
    annotations:
      summary: "Tenant {{ $labels.tenant_id }} quota almost exceeded"
      description: "Quota usage is {{ $value | humanizePercentage }}"
```

---

## 第二部分：安全

### 六、安全架构总览

```
┌─────────────────────────────────────────────────────────────┐
│                     安全四件套                                 │
│                                                               │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌─────────┐ │
│  │  认证     │  │  授权     │  │  审计     │  │  加密   │ │
│  │Authentication│ │Authorization│ │  Audit    │  │Encryption│ │
│  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └────┬────┘ │
│        │                │                │               │      │
│        ▼                ▼                ▼               ▼      │
│  JWT/OIDC/SSO      RBAC/ABAC       不可篡改日志      TLS/mTLS  │
│  MFA/验证码         数据权限        操作追踪          字段加密  │
│  Token黑名单        接口权限        合规报告          密码哈希  │
└─────────────────────────────────────────────────────────────┘
```

### 七、认证（Authentication）

#### 7.1 认证方式

| 方式 | 适用场景 | 实现 |
|------|----------|------|
| **用户名密码** | 常规登录 | bcrypt/Argon2id 哈希 + 验证码 |
| **SSO (OIDC)** | 企业用户 | OAuth2 + OpenID Connect |
| **API Key** | 服务间/第三方 | 随机字符串 + HMAC 签名 |
| **JWT** | 会话管理 | RS256/ES256 签名 + 短期有效期 + Refresh Token |
| **MFA** | 高安全要求 | TOTP（Google Authenticator）/ 短信验证码 |
| **mTLS** | 服务间认证 | Istio 自动 mTLS / SPIFFE 身份 |

#### 7.2 JWT 规范

```rust
// libs/mox-auth/src/jwt.rs
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};
use chrono::{Utc, Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,           // 用户ID
    pub tenant_id: String,     // 租户ID
    pub roles: Vec<String>,    // 角色列表
    pub permissions: Vec<String>, // 权限列表
    pub exp: usize,            // 过期时间
    pub iat: usize,            // 签发时间
    pub jti: String,           // Token唯一ID（用于黑名单）
    pub iss: String,           // 签发者
    pub aud: String,           // 受众
}

pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtManager {
    pub fn new(private_key: &[u8], public_key: &[u8]) -> Result<Self, AuthError> {
        Ok(Self {
            encoding_key: EncodingKey::from_rsa_pem(private_key)?,
            decoding_key: DecodingKey::from_rsa_pem(public_key)?,
        })
    }

    pub fn generate(&self, user_id: &str, tenant_id: &str, roles: Vec<String>) -> Result<(String, String), AuthError> {
        let now = Utc::now();
        let access_exp = (now + Duration::minutes(15)).timestamp() as usize;
        let refresh_exp = (now + Duration::days(7)).timestamp() as usize;

        let access_claims = Claims {
            sub: user_id.into(),
            tenant_id: tenant_id.into(),
            roles: roles.clone(),
            permissions: vec![],
            exp: access_exp,
            iat: now.timestamp() as usize,
            jti: uuid::Uuid::new_v4().to_string(),
            iss: "mox-auth-svc".into(),
            aud: "mox-platform".into(),
        };

        let access_token = encode(&Header::new(jsonwebtoken::Algorithm::RS256), &access_claims, &self.encoding_key)?;
        let refresh_token = uuid::Uuid::new_v4().to_string(); // 存储在数据库，可撤销

        Ok((access_token, refresh_token))
    }

    pub fn verify(&self, token: &str) -> Result<Claims, AuthError> {
        let validation = Validation::new(jsonwebtoken::Algorithm::RS256);
        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;

        // 检查 Token 黑名单
        if is_token_blacklisted(&token_data.claims.jti).await? {
            return Err(AuthError::TokenRevoked);
        }

        Ok(token_data.claims)
    }
}
```

#### 7.3 Token 黑名单

```
登出/密码修改/权限变更 → JWT jti 加入 Redis 黑名单（TTL=Token剩余有效期）
→ 每次验证检查黑名单
```

### 八、授权（Authorization）

#### 8.1 RBAC + ABAC 混合模型

```
用户 (User)
  ├── 属于 租户 (Tenant)
  ├── 拥有 角色 (Role) [RBAC]
  │     └── 角色包含 权限 (Permission)
  └── 拥有 属性 (Attributes) [ABAC]
        ├── 部门
        ├── 职级
        └── 数据范围
```

#### 8.2 权限模型

```rust
// libs/mox-auth/src/rbac.rs

/// 权限定义
#[derive(Debug, Clone)]
pub struct Permission {
    pub resource: String,    // 资源：graph, ai, user, tenant
    pub action: String,      // 操作：create, read, update, delete, execute
    pub effect: Effect,      // 允许/拒绝
}

/// 角色定义
#[derive(Debug, Clone)]
pub struct Role {
    pub name: String,
    pub permissions: Vec<Permission>,
    pub is_system: bool,     // 系统内置角色不可删除
}

/// 系统内置角色
pub const SYSTEM_ROLES: &[(&str, &[(&str, &str)])] = &[
    ("tenant_admin", &[
        ("*", "*"),  // 租户内所有权限
    ]),
    ("developer", &[
        ("graph", "read"), ("graph", "create"), ("graph", "update"),
        ("ai", "read"), ("ai", "execute"),
        ("flow", "read"), ("flow", "create"), ("flow", "update"), ("flow", "execute"),
    ]),
    ("analyst", &[
        ("graph", "read"),
        ("ai", "read"), ("ai", "execute"),
        ("dashboard", "read"),
    ]),
    ("viewer", &[
        ("*", "read"),  // 只读所有
    ]),
];

/// 权限检查
pub fn check_permission(
    user_roles: &[Role],
    resource: &str,
    action: &str,
) -> bool {
    // 拒绝优先（Deny 覆盖 Allow）
    for role in user_roles {
        for perm in &role.permissions {
            if (perm.resource == "*" || perm.resource == resource)
                && (perm.action == "*" || perm.action == action)
            {
                match perm.effect {
                    Effect::Deny => return false,
                    Effect::Allow => {}
                }
            }
        }
    }
    // 检查是否有 Allow
    user_roles.iter().any(|role| {
        role.permissions.iter().any(|perm| {
            (perm.resource == "*" || perm.resource == resource)
                && (perm.action == "*" || perm.action == action)
                && perm.effect == Effect::Allow
        })
    })
}
```

#### 8.3 数据权限

| 级别 | 说明 | 实现 |
|------|------|------|
| **全部数据** | 可访问租户内所有数据 | 无额外过滤 |
| **本部门数据** | 可访问本部门及子部门数据 | WHERE department_id IN (子部门树) |
| **本人数据** | 只能访问自己创建的数据 | WHERE created_at = user_id |
| **自定义** | 按规则过滤 | ABAC 策略引擎 |

数据权限在 SQL 查询层自动追加过滤条件（通过 sqlx 中间件或 RLS 策略）。

### 九、审计（Audit）

#### 9.1 审计日志

所有关键操作记录不可篡改审计日志：

```rust
// libs/mox-audit/src/audit.rs
use serde::{Serialize, Deserialize};
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub timestamp: String,
    pub tenant_id: String,
    pub user_id: String,
    pub service: String,
    pub action: AuditAction,
    pub resource_type: String,
    pub resource_id: String,
    pub ip_address: String,
    pub user_agent: String,
    pub request_id: String,
    pub trace_id: String,
    pub result: AuditResult,
    pub reason: Option<String>,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub hash: String,          // 哈希链（前一条日志的哈希）
}

pub enum AuditAction {
    Create, Read, Update, Delete,
    Login, Logout,
    Export, Import,
    PermissionChange,
    ConfigChange,
}

pub enum AuditResult {
    Success, Failure,
}
```

#### 9.2 不可篡改存储

```
审计日志 → NATS → 审计服务 → 追加写存储（WORM）
  ├── 每条日志包含前一条的哈希（哈希链）
  ├── 定期 Merkle Root 上链/存证
  └── 不可修改、不可删除
```

### 十、加密（Encryption）

| 层级 | 加密方式 | 算法 |
|------|----------|------|
| **传输加密** | TLS 1.3（外部）+ mTLS（服务间） | AES-256-GCM |
| **存储加密** | 磁盘加密（LUKS） | AES-256-XTS |
| **字段加密** | 应用层加密（敏感字段） | AES-256-GCM / 国密 SM4 |
| **密码存储** | 加盐哈希 | Argon2id（推荐）/ bcrypt |
| **Token 签名** | 非对称签名 | RS256 / ES256 |
| **对象存储** | 服务端加密（SSE） | AES-256 / 国密 SM4 |
| **密钥管理** | KMS（密钥轮换/审计） | - |

---

## 第三部分：弹性

### 十一、弹性架构总览

```
┌─────────────────────────────────────────────────────────────┐
│                      弹性七件套                                │
│                                                               │
│  限流 → 熔断 → 降级 → 重试 → 超时 → 舱壁 → 死信队列          │
│  (Rate  (Circuit (Fallback)(Retry)(Timeout)(Bulkhead)(DLQ)  │
│   Limit) Breaker)                                             │
│                                                               │
│  目标：防止级联故障，保障核心功能可用                          │
└─────────────────────────────────────────────────────────────┘
```

### 十二、限流（Rate Limiting）

#### 12.1 限流层级

| 层级 | 限流对象 | 算法 | 实现位置 |
|------|----------|------|----------|
| **网关层** | 租户/用户/IP/接口 | 令牌桶 + 滑动窗口 | mox-gateway-svc |
| **服务层** | 接口/资源 | 信号量/并发数 | gRPC 拦截器 |
| **下游层** | 第三方API/数据库 | 连接池/并发控制 | 客户端封装 |

#### 12.2 网关限流实现

```rust
// libs/mox-gateway/src/rate_limit.rs
use redis::AsyncCommands;
use std::time::Duration;

pub struct RateLimiter {
    redis: redis::Client,
}

impl RateLimiter {
    /// 滑动窗口限流
    pub async fn check_sliding_window(
        &self,
        key: &str,           // "rate_limit:{tenant_id}:{api}"
        limit: usize,        // 窗口内最大请求数
        window: Duration,    // 窗口大小
    ) -> Result<bool, RateLimitError> {
        let mut conn = self.redis.get_async_connection().await?;
        let now = chrono::Utc::now().timestamp_millis();
        let window_ms = window.as_millis() as i64;

        // Lua 脚本：原子操作（移除过期记录 + 计数 + 添加当前记录）
        let script = r#"
            local key = KEYS[1]
            local now = tonumber(ARGV[1])
            local window = tonumber(ARGV[2])
            local limit = tonumber(ARGV[3])
            redis.call('ZREMRANGEBYSCORE', key, 0, now - window)
            local count = redis.call('ZCARD', key)
            if count < limit then
                redis.call('ZADD', key, now, now .. ':' .. math.random())
                redis.call('EXPIRE', key, window / 1000)
                return 1
            else
                return 0
            end
        "#;

        let allowed: i32 = redis::Script::new(script)
            .key(key)
            .arg(now)
            .arg(window_ms)
            .arg(limit)
            .invoke_async(&mut conn)
            .await?;

        Ok(allowed == 1)
    }
}
```

#### 12.3 限流配置

```yaml
# 网关限流配置
rate_limit:
  default:
    per_tenant: 1000/min     # 每租户默认1000次/分钟
    per_user: 100/min        # 每用户默认100次/分钟
    per_ip: 60/min           # 每IP默认60次/分钟
  custom:
    "ai.generate":
      per_tenant: 100/min    # AI生成接口限流更严
      per_user: 20/min
    "graph.import":
      per_tenant: 10/min     # 数据导入接口限流更严
  burst:
    enabled: true
    burst_size: 10           # 允许突发10个请求
```

### 十三、熔断（Circuit Breaker）

#### 13.1 熔断器状态机

```
         失败率>阈值
  ┌─────────────┐     ┌─────────────┐
  │   Closed    │────→│    Open     │
  │  (正常放行)  │     │  (快速失败)  │
  └─────────────┘     └──────┬──────┘
        ▲                     │ 冷却时间到期
        │                     ▼
        │              ┌─────────────┐
        └──────────────│  Half-Open  │
          探测成功      │  (探测放行)  │
                       └─────────────┘
```

#### 13.2 Rust 熔断器实现

```rust
// libs/mox-resilience/src/circuit_breaker.rs
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    config: CircuitBreakerConfig,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,       // 失败次数阈值
    pub success_threshold: u32,       // 半开状态成功次数阈值
    pub timeout: Duration,             // 熔断超时时间
    pub half_open_max_requests: u32,  // 半开状态最大请求数
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(30),
            half_open_max_requests: 1,
        }
    }
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            config,
        }
    }

    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: From<CircuitBreakerError>,
    {
        // 检查状态
        {
            let state = *self.state.read().await;
            match state {
                CircuitState::Open => {
                    // 检查是否超时，可以进入半开
                    let last_failure = *self.last_failure_time.read().await;
                    if let Some(time) = last_failure {
                        if time.elapsed() >= self.config.timeout {
                            *self.state.write().await = CircuitState::HalfOpen;
                            *self.success_count.write().await = 0;
                        } else {
                            return Err(E::from(CircuitBreakerError::CircuitOpen));
                        }
                    } else {
                        return Err(E::from(CircuitBreakerError::CircuitOpen));
                    }
                }
                CircuitState::HalfOpen => {
                    // 半开状态限制请求数
                    // ...
                }
                CircuitState::Closed => {}
            }
        }

        // 执行调用
        let result = f().await;

        // 更新状态
        match &result {
            Ok(_) => self.on_success().await,
            Err(_) => self.on_failure().await,
        }

        result
    }

    async fn on_success(&self) {
        let state = *self.state.read().await;
        match state {
            CircuitState::HalfOpen => {
                let mut success = self.success_count.write().await;
                *success += 1;
                if *success >= self.config.success_threshold {
                    *self.state.write().await = CircuitState::Closed;
                    *self.failure_count.write().await = 0;
                }
            }
            CircuitState::Closed => {
                *self.failure_count.write().await = 0;
            }
            _ => {}
        }
    }

    async fn on_failure(&self) {
        let state = *self.state.read().await;
        match state {
            CircuitState::Closed => {
                let mut failures = self.failure_count.write().await;
                *failures += 1;
                if *failures >= self.config.failure_threshold {
                    *self.state.write().await = CircuitState::Open;
                    *self.last_failure_time.write().await = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                *self.state.write().await = CircuitState::Open;
                *self.last_failure_time.write().await = Some(Instant::now());
            }
            _ => {}
        }
    }
}
```

### 十四、降级（Fallback）

#### 14.1 降级策略

| 降级类型 | 说明 | 示例 |
|----------|------|------|
| **默认值降级** | 返回默认值/空结果 | AI 生成失败返回缓存的相似回答 |
| **缓存降级** | 返回缓存数据 | 图谱查询超时返回最近一次缓存结果 |
| **简化功能降级** | 关闭非核心功能 | 高负载时关闭推荐功能，保留核心查询 |
| **排队降级** | 请求排队，异步处理 | 数据导入高负载时排队，返回任务ID |
| **限流降级** | 直接拒绝非核心请求 | 高负载时拒绝批量导出，保留实时查询 |

#### 14.2 降级实现

```rust
// libs/mox-resilience/src/fallback.rs

pub enum Fallback<T> {
    Value(T),                    // 返回固定值
    Cache(String, Duration),     // 返回缓存（key, TTL）
    Function(Box<dyn Fn() -> T + Send + Sync>), // 执行降级函数
    None,                        // 无降级（返回错误）
}

pub async fn with_fallback<F, Fut, T, E>(
    operation: F,
    fallback: Fallback<T>,
) -> Result<T, E>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    match operation().await {
        Ok(result) => Ok(result),
        Err(e) => {
            tracing::warn!("operation failed, using fallback: {:?}", e);
            match fallback {
                Fallback::Value(v) => Ok(v),
                Fallback::Cache(key, _ttl) => {
                    // 从缓存获取
                    if let Some(cached) = get_cache(&key).await {
                        Ok(cached)
                    } else {
                        Err(e)
                    }
                }
                Fallback::Function(f) => Ok(f()),
                Fallback::None => Err(e),
            }
        }
    }
}
```

### 十五、重试（Retry）

#### 15.1 重试策略

| 策略 | 说明 | 适用 |
|------|------|------|
| **固定间隔** | 每次重试间隔固定 | 简单场景 |
| **指数退避** | 间隔指数增长（1s, 2s, 4s, 8s） | 网络抖动/服务重启 |
| **指数退避+抖动** | 指数增长+随机偏移，避免惊群 | 高并发场景 |
| **不重试** | 立即失败 | 幂等性无法保证的写操作 |

#### 15.2 重试实现

```rust
// libs/mox-resilience/src/retry.rs
use std::time::Duration;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_factor: 2.0,
            jitter: true,
        }
    }
}

pub async fn with_retry<F, Fut, T, E>(
    config: RetryConfig,
    is_retryable: impl Fn(&E) -> bool,
    operation: impl Fn() -> Fut,
) -> Result<T, E>
where
    F: std::future::Future,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay;

    loop {
        attempt += 1;
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt >= config.max_attempts || !is_retryable(&e) {
                    return Err(e);
                }
                tracing::warn!(attempt, "retrying after {:?}", delay);
                tokio::time::sleep(delay).await;

                // 指数退避
                delay = Duration::from_secs_f64(
                    (delay.as_secs_f64() * config.backoff_factor)
                        .min(config.max_delay.as_secs_f64())
                );

                // 抖动
                if config.jitter {
                    let jitter = rand::thread_rng().gen_range(0.0..0.5);
                    delay = Duration::from_secs_f64(delay.as_secs_f64() * (1.0 + jitter));
                }
            }
        }
    }
}
```

#### 15.3 幂等性保证

重试必须保证幂等（重复调用结果相同）：
- 读操作：天然幂等
- 写操作：使用 request_id 去重（服务端记录已处理的 request_id）
- 删除操作：天然幂等
- 更新操作：使用乐观锁（版本号）

### 十六、超时（Timeout）

#### 16.1 超时层级

| 层级 | 超时设置 | 说明 |
|------|----------|------|
| **网关超时** | 30s（普通）/ 300s（流式） | 网关到后端服务 |
| **服务间超时** | 3-5s（普通）/ 30s（批量） | gRPC 客户端超时 |
| **数据库超时** | 2s（连接）/ 5s（查询） | sqlx 查询超时 |
| **缓存超时** | 500ms | Redis 操作超时 |
| **第三方API超时** | 10s | 外部 HTTP 调用 |

#### 16.2 超时实现

```rust
// gRPC 客户端超时
use std::time::Duration;
use tonic::Request;

let mut client = AiServiceClient::new(channel);
let request = Request::new(GenerateRequest { ... });
// 设置超时（deadline）
let request = request.timeout(Duration::from_secs(5));
let response = client.generate(request).await?;
```

```rust
// 数据库查询超时
use sqlx::postgres::PgPoolOptions;

let pool = PgPoolOptions::new()
    .max_connections(20)
    .acquire_timeout(Duration::from_secs(3))
    .idle_timeout(Duration::from_secs(300))
    .connect("postgres://...")
    .await?;

// 单条查询超时
let result = tokio::time::timeout(
    Duration::from_secs(5),
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
).await??;
```

### 十七、舱壁（Bulkhead）

#### 17.1 舱壁模式

将资源隔离，防止一个功能耗尽所有资源：

```
┌─────────────────────────────────────────┐
│              服务进程                      │
│                                          │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │ AI 生成  │  │ 图谱查询 │  │ 数据导入││
│  │ 线程池   │  │ 线程池   │  │ 线程池  ││
│  │ 最大10   │  │ 最大20   │  │ 最大5  ││
│  └──────────┘  └──────────┘  └────────┘│
│                                          │
│  ┌──────────┐  ┌──────────┐             │
│  │ DB连接池 │  │ Redis池  │             │
│  │ 最大20   │  │ 最大50   │             │
│  └──────────┘  └──────────┘             │
└─────────────────────────────────────────┘
```

#### 17.2 实现

```rust
// libs/mox-resilience/src/bulkhead.rs
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct Bulkhead {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl Bulkhead {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    pub async fn acquire(&self) -> Result<BulkheadGuard, BulkheadError> {
        let permit = self.semaphore.clone().try_acquire_owned()
            .map_err(|_| BulkheadError::CapacityExceeded(self.max_concurrent))?;
        Ok(BulkheadGuard { _permit: permit })
    }

    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
}

pub struct BulkheadGuard {
    _permit: tokio::sync::OwnedSemaphorePermit,
}
```

### 十八、死信队列（DLQ）

#### 18.1 消息处理流程

```
生产者 → 主队列 → 消费者
                │
                ├── 处理成功 → Ack
                │
                └── 处理失败 → Nack → 重试队列（延迟）
                                      │
                                      ├── 重试成功 → Ack
                                      │
                                      └── 重试次数超限 → 死信队列（DLQ）
                                                            │
                                                            ▼
                                                       人工处理/告警
```

#### 18.2 NATS JetStream DLQ 配置

```rust
// libs/mox-mq/src/dlq.rs
use async_nats::jetstream;

pub async fn create_stream_with_dlq(js: &jetstream::Context) -> Result<(), MqError> {
    // 主队列
    js.create_stream(jetstream::stream::Config {
        name: "mox-events".to_string(),
        subjects: vec!["mox.>".to_string()],
        max_consumers: 10,
        max_msgs_per_subject: 100000,
        retention: jetstream::stream::RetentionPolicy::WorkQueue,
        ..Default::default()
    }).await?;

    // 死信队列
    js.create_stream(jetstream::stream::Config {
        name: "mox-dlq".to_string(),
        subjects: vec!["mox.dlq.>".to_string()],
        max_msgs: 1000000,
        retention: jetstream::stream::RetentionPolicy::Limits,
        ..Default::default()
    }).await?;

    // 消费者配置（失败后转到 DLQ）
    js.create_consumer(
        "mox-events",
        jetstream::consumer::pull::Config {
            durable_name: Some("mox-event-consumer"),
            ack_policy: jetstream::consumer::AckPolicy::Explicit,
            max_deliver: 3,                    // 最多重试3次
            ack_wait: std::time::Duration::from_secs(30),
            backoff: vec![                      // 退避策略
                std::time::Duration::from_secs(1),
                std::time::Duration::from_secs(5),
                std::time::Duration::from_secs(30),
            ],
            ..Default::default()
        },
    ).await?;

    Ok(())
}
```

---

## 总结

可观测性·安全·弹性是企业级平台的三大支柱：

**可观测性**：
1. **三大支柱**：日志（tracing+Loki）+ 指标（prometheus+Grafana）+ 链路追踪（OTel+Jaeger）
2. **统一采集**：OpenTelemetry Collector 统一接收，统一标准
3. **告警体系**：P0-P3 四级告警，多渠道通知，自动升级
4. **统一仪表盘**：全局概览/服务详情/业务指标/基础设施/链路追踪/日志分析

**安全**：
1. **认证**：JWT + OIDC/SSO + MFA + mTLS，Token 黑名单
2. **授权**：RBAC + ABAC 混合模型，数据权限（全部/部门/本人/自定义）
3. **审计**：不可篡改审计日志（哈希链 + WORM 存储），合规报告
4. **加密**：全链路 TLS/mTLS + 字段加密 + 密码哈希 + KMS 密钥管理

**弹性**：
1. **限流**：网关层（令牌桶+滑动窗口）+ 服务层（并发控制）
2. **熔断**：三态状态机（Closed/Open/HalfOpen），失败率阈值触发
3. **降级**：默认值/缓存/简化功能/排队/限流，五种降级策略
4. **重试**：指数退避+抖动，幂等性保证，最多3次
5. **超时**：多层级超时（网关/服务间/数据库/缓存/第三方）
6. **舱壁**：资源隔离（线程池/连接池/信号量），防止级联故障
7. **死信队列**：消息重试超限转入 DLQ，人工处理+告警

---

*下一篇：[06-实施路线图](./06-roadmap.md)*

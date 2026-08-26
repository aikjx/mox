# 04 - 部署架构优化

> 版本：v1.0 | 日期：2026-08-26 | 状态：草案
>
> 前置阅读：[00-核心原则](./00-principles.md) | [01-服务边界优化](./01-service-boundaries.md) | [02-通信架构优化](./02-communication.md) | [03-数据架构优化](./03-data.md)

## 一、现状诊断

### 1.1 当前部署方式

| 维度 | 现状 | 问题 |
|------|------|------|
| 部署单元 | mox-server single-binary（单体） | 无法独立部署，发布风险大 |
| 容器化 | 有 Dockerfile（deploy/docker/） | 不完整，非每服务一个镜像 |
| 编排 | 无 K8s manifests | 无法弹性伸缩，无自愈 |
| CI/CD | GitHub Actions（.github/） | 不完整，无自动部署 |
| 配置管理 | application.yml 多环境文件 | 无配置中心，配置变更需重新部署 |
| 服务发现 | 无（单体不需要） | 微服务化后必须 |
| 监控告警 | prometheus（部分） | 不完整，无统一仪表盘/告警 |
| 日志 | 文件日志（散落在根目录） | 无日志聚合，排查困难 |

### 1.2 核心问题

| 问题 | 影响 | 严重度 |
|------|------|--------|
| **单体部署** | 任何变更全量部署，发布风险大，无法独立扩缩容 | 🔴 高 |
| **无 K8s 编排** | 无自愈、无弹性伸缩、无滚动更新 | 🔴 高 |
| **配置不外部化** | 配置变更需重新编译部署，环境管理混乱 | 🟡 中 |
| **无服务发现** | 微服务间无法动态发现彼此 | 🟡 中 |
| **CI/CD 不完整** | 发布依赖人工，效率低，易出错 | 🟡 中 |
| **日志不聚合** | 分布式排查困难，无法跨服务追踪 | 🟡 中 |
| **无灰度发布** | 发布即全量，风险不可控 | 🟡 中 |

---

## 二、目标部署架构

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                         开发者 / CI/CD                                 │
│                    Git push → GitHub Actions                           │
│                         ↓ 构建镜像                                      │
│                    Container Registry (Harbor / ECR)                   │
└──────────────────────────────┬──────────────────────────────────────┘
                               ↓
┌─────────────────────────────────────────────────────────────────────┐
│                         Kubernetes 集群                                 │
│                                                                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                │
│  │  Ingress    │  │  Istio      │  │  cert-manager│                │
│  │  (Nginx/ALB)│  │  (Service Mesh)│ │ (证书管理)   │                │
│  └──────┬──────┘  └──────┬──────┘  └─────────────┘                │
│         │                  │                                          │
│         ▼                  ▼                                          │
│  ┌─────────────────────────────────────────────────┐                  │
│  │              mox-gateway-svc (HPA 2-20)         │                  │
│  │         (多协议入口：REST/gRPC-Web/WS)            │                  │
│  └──────┬──────────┬──────────┬──────────┬────────┘                  │
│         │          │          │          │                            │
│         ▼          ▼          ▼          ▼                            │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐                   │
│  │ ai-svc  │ │graph-svc│ │auth-svc │ │tenant-svc│                   │
│  │(HPA 2-20)│ │(HPA 2-10)│ │(HPA 2-5)│ │(HPA 2-5)│                   │
│  └────┬────┘ └────┬────┘ └─────────┘ └─────────┘                   │
│       │           │                                                    │
│       ▼           ▼                                                    │
│  ┌─────────┐ ┌──────────────────────┐                                 │
│  │Python   │ │mox-graph-storage-svc │                                 │
│  │inference│ │(StatefulSet 3副本)    │                                 │
│  │sidecar  │ │(RocksDB+Raft+CDC)    │                                 │
│  │(GPU节点)│ └──────────────────────┘                                 │
│  └─────────┘                                                           │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────┐     │
│  │  平台服务 (K8s 托管 / Operator)                               │     │
│  │  PostgreSQL(主从) + Redis(哨兵) + NATS(集群) + MinIO        │     │
│  └──────────────────────────────────────────────────────────────┘     │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────┐     │
│  │  可观测性 (K8s 托管)                                          │     │
│  │  OTel Collector + Jaeger + Prometheus + Grafana + Loki       │     │
│  └──────────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 部署单元

| 部署单元 | 类型 | 副本数 | 扩缩容 | 说明 |
|----------|------|--------|--------|------|
| mox-gateway-svc | Deployment | 2-20 | HPA | 无状态，入口 |
| mox-auth-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-tenant-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-system-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-metering-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-notification-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-ai-svc | Deployment | 2-20 | HPA | 无状态，CPU密集 |
| mox-agent-svc | Deployment | 2-10 | HPA | 无状态 |
| mox-expert-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-graph-svc | Deployment | 2-10 | HPA | 无状态，CPU密集 |
| mox-graph-algo-svc | Deployment | 2-10 | HPA | 无状态，CPU密集 |
| mox-graph-streams-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-graph-meta-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-graph-storage-svc | StatefulSet | 3 | 手动/Operator | ★有状态★，RocksDB+Raft |
| mox-storage-svc | Deployment | 2-10 | HPA | 无状态，IO密集 |
| mox-etl-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-dataplane-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-search-svc | Deployment | 2-10 | HPA | 无状态 |
| mox-flow-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-flow-fusion-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-operator-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-compliance-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-fusion-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-catalog-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-market-svc | Deployment | 2-5 | HPA | 无状态 |
| mox-optimizer-svc | Deployment | 2-5 | HPA | 无状态 |
| ai-inference (Python) | Deployment | 1-10 | HPA | GPU节点，sidecar模式 |

---

## 三、Kubernetes 部署规范

### 3.1 每服务标准 K8s 资源

每个服务必须包含以下 K8s 资源：

```
deploy/k8s/mox-ai-svc/
├── deployment.yaml      # Deployment（副本、镜像、资源、探针）
├── service.yaml         # Service（gRPC + metrics 端口）
├── hpa.yaml             # HorizontalPodAutoscaler（自动扩缩容）
├── pdb.yaml             # PodDisruptionBudget（中断预算）
├── configmap.yaml       # ConfigMap（非敏感配置）
└── serviceaccount.yaml  # ServiceAccount（服务身份）
```

### 3.2 Deployment 标准模板

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mox-ai-svc
  namespace: mox
  labels:
    app: mox-ai-svc
    tier: business
    version: v3.0.0
spec:
  replicas: 2
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1          # 滚动更新时最多多出1个Pod
      maxUnavailable: 0    # 滚动更新时不可用Pod为0（零停机）
  selector:
    matchLabels:
      app: mox-ai-svc
  template:
    metadata:
      labels:
        app: mox-ai-svc
        tier: business
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: mox-ai-svc
      terminationGracePeriodSeconds: 30  # 优雅终止宽限期
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      containers:
      - name: mox-ai-svc
        image: registry/infotopograph/mox-ai-svc:v3.0.0
        imagePullPolicy: IfNotPresent
        ports:
        - name: grpc
          containerPort: 50051
          protocol: TCP
        - name: metrics
          containerPort: 9090
          protocol: TCP
        env:
        - name: ENV
          value: "production"
        - name: RUST_LOG
          value: "info"
        - name: SERVICE_NAME
          value: "mox-ai-svc"
        envFrom:
        - configMapRef:
            name: mox-common-config
        - secretRef:
            name: mox-secrets
        resources:
          requests:
            cpu: "500m"
            memory: "512Mi"
          limits:
            cpu: "2"
            memory: "2Gi"
        livenessProbe:               # 存活探针
          grpc:
            port: 50051
          initialDelaySeconds: 10
          periodSeconds: 30
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:              # 就绪探针
          grpc:
            port: 50051
          initialDelaySeconds: 5
          periodSeconds: 10
          timeoutSeconds: 3
          failureThreshold: 3
        startupProbe:                # 启动探针（慢启动服务）
          grpc:
            port: 50051
          initialDelaySeconds: 5
          periodSeconds: 10
          failureThreshold: 30
        volumeMounts:
        - name: tmp
          mountPath: /tmp
        - name: config
          mountPath: /etc/mox
          readOnly: true
      volumes:
      - name: tmp
        emptyDir: {}
      - name: config
        configMap:
          name: mox-ai-svc-config
      topologySpreadConstraints:     # 跨节点/可用区分布
      - maxSkew: 1
        topologyKey: topology.kubernetes.io/zone
        whenUnsatisfiable: DoNotSchedule
        labelSelector:
          matchLabels:
            app: mox-ai-svc
      - maxSkew: 1
        topologyKey: kubernetes.io/hostname
        whenUnsatisfiable: ScheduleAnyway
        labelSelector:
          matchLabels:
            app: mox-ai-svc
```

### 3.3 Service 标准模板

```yaml
apiVersion: v1
kind: Service
metadata:
  name: mox-ai-svc
  namespace: mox
  labels:
    app: mox-ai-svc
spec:
  type: ClusterIP
  selector:
    app: mox-ai-svc
  ports:
  - name: grpc
    port: 50051
    targetPort: grpc
    protocol: TCP
  - name: metrics
    port: 9090
    targetPort: metrics
    protocol: TCP
```

### 3.4 HPA 标准模板

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: mox-ai-svc
  namespace: mox
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: mox-ai-svc
  minReplicas: 2
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  - type: Pods
    pods:
      metric:
        name: grpc_requests_per_second
      target:
        type: AverageValue
        averageValue: "100"
  behavior:
    scaleUp:
      stabilizationWindowSeconds: 30
      policies:
      - type: Percent
        value: 100
        periodSeconds: 30
      - type: Pods
        value: 4
        periodSeconds: 30
      selectPolicy: Max
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 25
        periodSeconds: 60
      selectPolicy: Min
```

### 3.5 PDB 标准模板

```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: mox-ai-svc
  namespace: mox
spec:
  minAvailable: 1                    # 维护时至少1个可用
  selector:
    matchLabels:
      app: mox-ai-svc
```

### 3.6 有状态服务（图存储）StatefulSet

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: mox-graph-storage-svc
  namespace: mox
spec:
  serviceName: mox-graph-storage-svc
  replicas: 3
  selector:
    matchLabels:
      app: mox-graph-storage-svc
  template:
    metadata:
      labels:
        app: mox-graph-storage-svc
    spec:
      containers:
      - name: mox-graph-storage-svc
        image: registry/infotopograph/mox-graph-storage-svc:v3.0.0
        ports:
        - name: grpc
          containerPort: 50051
        - name: raft
          containerPort: 7000
        - name: metrics
          containerPort: 9090
        volumeMounts:
        - name: data
          mountPath: /data
        resources:
          requests:
            cpu: "1"
            memory: "2Gi"
          limits:
            cpu: "4"
            memory: "8Gi"
        livenessProbe:
          grpc:
            port: 50051
          initialDelaySeconds: 30
          periodSeconds: 30
        readinessProbe:
          grpc:
            port: 50051
          initialDelaySeconds: 15
          periodSeconds: 10
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: fast-ssd
      resources:
        requests:
          storage: 100Gi
```

---

## 四、优雅启停

### 4.1 优雅启动

```
Pod 启动流程：
  1. K8s 拉取镜像
  2. 容器启动，执行 init（如果有）
  3. 应用启动：
     a. 加载配置
     b. 初始化日志/追踪
     c. 连接数据库/缓存/消息队列
     d. 注册服务发现
     e. 启动 gRPC server
     f. 启动 metrics server
  4. startupProbe 检测通过（最多300s）
  5. readinessProbe 检测通过
  6. Pod 标记为 Ready，Service 开始转发流量
  7. livenessProbe 持续检测
```

### 4.2 优雅终止

```
Pod 终止流程（收到 SIGTERM）：
  1. Pod 标记为 Terminating，从 Service Endpoints 移除（不再接收新请求）
  2. 发送 SIGTERM 到容器
  3. 应用执行优雅关闭：
     a. 停止接收新 gRPC 请求（gRPC GOAWAY）
     b. 等待正在处理的请求完成（最多 25s）
     c. 刷新缓冲区/提交事务
     d. 关闭数据库连接
     e. 注销服务发现
     f. 刷新日志
  4. 如果超过 terminationGracePeriodSeconds（30s），发送 SIGKILL
  5. 容器退出
```

### 4.3 Rust 优雅关闭实现

```rust
// libs/mox-rpc/src/server.rs
use tonic::transport::Server;
use tokio::signal;
use std::time::Duration;

pub async fn serve_with_graceful_shutdown(
    addr: std::net::SocketAddr,
    router: tonic::transport::server::Router,
) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    // 监听终止信号
    tokio::spawn(async move {
        let ctrl_c = async {
            signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                tracing::info!("received Ctrl+C, initiating graceful shutdown");
            }
            _ = terminate => {
                tracing::info!("received SIGTERM, initiating graceful shutdown");
            }
        }

        let _ = tx.send(());
    });

    // 启动服务，带优雅关闭
    tracing::info!("gRPC server listening on {}", addr);
    Server::builder()
        .add_router(router)
        .serve_with_shutdown(addr, async move {
            let _ = rx.await;
            tracing::info!("gRPC server shutting down gracefully");
            // 等待正在处理的请求完成
            tokio::time::sleep(Duration::from_secs(2)).await;
        })
        .await?;

    tracing::info!("gRPC server stopped");
    Ok(())
}
```

---

## 五、配置中心

### 5.1 两阶段方案

| 阶段 | 方案 | 适用场景 |
|------|------|----------|
| **阶段一（起步）** | K8s ConfigMap + Secret | 全量 K8s 部署，零额外组件 |
| **阶段二（规模化）** | Nacos（注册中心+配置中心） | 多环境/多集群/灰度配置/热更新 |

### 5.2 K8s ConfigMap 方案

```yaml
# 公共配置（所有服务共享）
apiVersion: v1
kind: ConfigMap
metadata:
  name: mox-common-config
  namespace: mox
data:
  ENV: "production"
  RUST_LOG: "info"
  LOG_FORMAT: "json"
  TRACING_ENDPOINT: "http://otel-collector:4317"
  METRICS_ENDPOINT: "http://prometheus:9090"
  REDIS_URL: "redis://redis:6379"
  NATS_URL: "nats://nats:4222"
  DATABASE_HOST: "postgresql"
  DATABASE_PORT: "5432"
```

```yaml
# 服务专属配置
apiVersion: v1
kind: ConfigMap
metadata:
  name: mox-ai-svc-config
  namespace: mox
data:
  AI_MODEL_DEFAULT: "gpt-4o"
  AI_MAX_TOKENS: "4096"
  AI_TEMPERATURE: "0.7"
  AI_TIMEOUT_SECONDS: "30"
  AI_STREAM_TIMEOUT_SECONDS: "300"
  AI_CACHE_ENABLED: "true"
  AI_CACHE_TTL_SECONDS: "3600"
```

```yaml
# 敏感配置（Secret）
apiVersion: v1
kind: Secret
metadata:
  name: mox-secrets
  namespace: mox
type: Opaque
stringData:
  DATABASE_PASSWORD: "xxx"
  REDIS_PASSWORD: "xxx"
  JWT_SECRET: "xxx"
  OPENAI_API_KEY: "xxx"
```

### 5.3 配置热更新

Rust 服务支持配置热更新（无需重启）：

```rust
// libs/mox-config/src/hot_reload.rs
use notify::{Watcher, RecursiveMode, watcher};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct HotConfig<T: Clone + Send + Sync + 'static> {
    inner: Arc<RwLock<T>>,
}

impl<T: Clone + Send + Sync + 'static> HotConfig<T> {
    pub fn new(config: T) -> Self {
        Self { inner: Arc::new(RwLock::new(config)) }
    }

    pub async fn get(&self) -> T {
        self.inner.read().await.clone()
    }

    pub async fn update(&self, new_config: T) {
        let mut write = self.inner.write().await;
        *write = new_config;
        tracing::info!("config updated");
    }

    /// 监听配置文件变化，自动热更新
    pub fn watch_file(&self, path: &str, parser: impl Fn(&str) -> T + Send + 'static) {
        let inner = self.inner.clone();
        let path = path.to_string();
        std::thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = watcher(tx, std::time::Duration::from_secs(2)).unwrap();
            watcher.watch(&path, RecursiveMode::NonRecursive).unwrap();

            loop {
                match rx.recv() {
                    Ok(_event) => {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let new_config = parser(&content);
                            // 更新配置（需要在 tokio runtime 中）
                            // ...
                        }
                    }
                    Err(e) => tracing::error!("config watch error: {}", e),
                }
            }
        });
    }
}
```

---

## 六、服务注册与发现

### 6.1 两阶段方案

| 阶段 | 方案 | 说明 |
|------|------|------|
| **阶段一** | K8s Service + CoreDNS | K8s 原生，零额外组件，DNS 负载均衡 |
| **阶段二** | Nacos | 支持权重路由/灰度/健康检查/元数据 |

### 6.2 K8s Service 方案

客户端通过 DNS 名称访问服务，K8s Service 自动做负载均衡：

```rust
// libs/mox-discovery/src/k8s.rs
use tonic::transport::Channel;

pub async fn connect_k8s_service(service_name: &str) -> Result<Channel, DiscoveryError> {
    // K8s Service DNS 名称：{service-name}.{namespace}.svc.cluster.local
    let addr = format!("http://{}.mox.svc.cluster.local:50051", service_name);
    let channel = Channel::from_shared(addr)?
        .connect_timeout(std::time::Duration::from_secs(5))
        .connect()
        .await?;
    Ok(channel)
}
```

### 6.3 客户端负载均衡

tonic 支持客户端负载均衡（从 K8s Service 获取所有 Endpoints）：

```rust
use tonic::transport::{Channel, Endpoint};
use http::Uri;

pub async fn connect_with_lb(service_name: &str) -> Result<Channel, DiscoveryError> {
    // DNS 解析获取所有 Pod IP（K8s Headless Service）
    let addrs = resolve_dns(service_name).await?;

    // 构建负载均衡 channel
    let channel = Channel::balance_list(
        addrs.into_iter().map(|addr| {
            Endpoint::from(Uri::try_from(format!("http://{}:50051", addr)).unwrap())
        })
    );

    Ok(channel)
}
```

---

## 七、CI/CD（GitOps）

### 7.1 CI/CD 流程

```
代码提交 (Git push)
    ↓
GitHub Actions (CI)
    ├── 1. 代码检查 (clippy + fmt + cargo audit)
    ├── 2. 单元测试 (cargo test --workspace)
    ├── 3. 集成测试 (testcontainers)
    ├── 4. 构建 Docker 镜像 (多阶段构建)
    ├── 5. 推送镜像到 Registry (Harbor/ECR)
    └── 6. 更新 Helm chart 版本 (Git 提交到 deploy 仓库)
    ↓
ArgoCD (CD, GitOps)
    ├── 1. 检测到 deploy 仓库变更
    ├── 2. 同步到 K8s 集群
    ├── 3. 滚动更新 (零停机)
    ├── 4. 健康检查
    └── 5. 通知 (成功/失败)
```

### 7.2 GitHub Actions CI 模板

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  REGISTRY: harbor.example.com
  IMAGE_NAME: infotopograph

jobs:
  lint:
    name: Lint
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy
    - name: Format check
      run: cargo fmt --all -- --check
    - name: Clippy
      run: cargo clippy --workspace --all-targets -- -D warnings
    - name: Audit
      run: cargo audit

  test:
    name: Test
    runs-on: ubuntu-latest
    needs: lint
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - uses: Swatinem/rust-cache@v2
    - name: Unit tests
      run: cargo test --workspace --lib
    - name: Integration tests
      run: cargo test --workspace --test '*'

  build:
    name: Build & Push
    runs-on: ubuntu-latest
    needs: test
    if: github.ref == 'refs/heads/main'
    strategy:
      matrix:
        service: [gateway, auth, tenant, ai, graph, graph-storage, storage, flow, system]
    steps:
    - uses: actions/checkout@v4
    - name: Set up Docker Buildx
      uses: docker/setup-buildx-action@v3
    - name: Login to Registry
      uses: docker/login-action@v3
      with:
        registry: ${{ env.REGISTRY }}
        username: ${{ secrets.REGISTRY_USERNAME }}
        password: ${{ secrets.REGISTRY_PASSWORD }}
    - name: Build and push
      uses: docker/build-push-action@v5
      with:
        context: .
        file: deploy/docker/Dockerfile.${{ matrix.service }}
        push: true
        tags: |
          ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}/mox-${{ matrix.service }}-svc:${{ github.sha }}
          ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}/mox-${{ matrix.service }}-svc:latest
```

### 7.3 Docker 多阶段构建模板

```dockerfile
# deploy/docker/Dockerfile.ai
# Stage 1: Build
FROM rust:1.75-slim-bullseye AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# 先复制 Cargo.toml 缓存依赖
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY libs/ libs/
COPY services/ services/

# 构建特定服务
RUN cargo build --release -p mox-ai-svc

# Stage 2: Runtime (distroless)
FROM gcr.io/distroless/cc-debian12

WORKDIR /app
COPY --from=builder /app/target/release/mox-ai-svc /app/mox-ai-svc

# 非 root 用户
USER 1000:1000

EXPOSE 50051 9090

ENTRYPOINT ["/app/mox-ai-svc"]
```

### 7.4 ArgoCD Application

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: mox-platform
  namespace: argocd
spec:
  project: default
  source:
    repoURL: https://github.com/infotopograph/deploy.git
    targetRevision: main
    path: helm/mox-platform
    helm:
      valueFiles:
      - values-production.yaml
  destination:
    server: https://kubernetes.default.svc
    namespace: mox
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
    syncOptions:
    - CreateNamespace=true
    - Validate=true
```

---

## 八、灰度发布

### 8.1 灰度策略

| 策略 | 实现 | 适用场景 |
|------|------|----------|
| **滚动更新** | K8s RollingUpdate（maxSurge=1, maxUnavailable=0） | 常规发布，零停机 |
| **蓝绿部署** | 两套环境，切换 Service/Ingress | 重大版本，快速回滚 |
| **金丝雀发布** | Istio VirtualService 按比例/Header路由 | 小流量验证，逐步放量 |
| **A/B测试** | 按用户特征/租户路由 | 功能对比测试 |

### 8.2 金丝雀发布（Istio）

```yaml
# Istio VirtualService - 金丝雀发布
apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: mox-ai-svc
  namespace: mox
spec:
  hosts:
  - mox-ai-svc
  http:
  - match:
    - headers:
        x-canary:
          exact: "true"
    route:
    - destination:
        host: mox-ai-svc
        subset: canary
      weight: 100
  - route:
    - destination:
        host: mox-ai-svc
        subset: stable
      weight: 95
    - destination:
        host: mox-ai-svc
        subset: canary
      weight: 5
---
apiVersion: networking.istio.io/v1beta1
kind: DestinationRule
metadata:
  name: mox-ai-svc
  namespace: mox
spec:
  host: mox-ai-svc
  trafficPolicy:
    connectionPool:
      tcp:
        maxConnections: 100
    http:
      h2UpgradePolicy: UPGRADE
  subsets:
  - name: stable
    labels:
      version: v3.0.0
  - name: canary
    labels:
      version: v3.1.0-canary
```

### 8.3 灰度发布流程

```
1. 构建 canary 镜像 (v3.1.0-canary)
2. 部署 canary Deployment (1副本)
3. Istio 路由 5% 流量到 canary
4. 监控 canary 指标（错误率/延迟/业务指标）
   ├── 正常 → 逐步放量 (5%→20%→50%→100%)
   └── 异常 → 立即回滚（流量切回 stable）
5. 全量后，删除 stable 旧版本
```

---

## 九、环境管理

### 9.1 环境划分

| 环境 | 用途 | 规模 | 数据 |
|------|------|------|------|
| **dev** | 开发调试 | 单节点，最小副本 | 模拟数据 |
| **staging** | 集成测试/预发布 | 多节点，生产配置 | 脱敏生产数据 |
| **canary** | 金丝雀发布 | 生产集群子集 | 生产数据 |
| **production** | 生产 | 多可用区，高可用 | 生产数据 |
| **dr** | 灾备 | 异地集群 | 生产数据复制 |

### 9.2 环境隔离

- 每个环境独立 K8s Namespace / 独立集群
- 独立数据库实例
- 独立配置（ConfigMap/Secret）
- 独立域名（dev-api.example.com / api.example.com）
- 网络隔离（NetworkPolicy）

---

## 十、资源规划

### 10.1 资源配额（Namespace）

```yaml
apiVersion: v1
kind: ResourceQuota
metadata:
  name: mox-resource-quota
  namespace: mox
spec:
  hard:
    requests.cpu: "64"
    requests.memory: "128Gi"
    limits.cpu: "128"
    limits.memory: "256Gi"
    persistentvolumeclaims: "50"
    services.loadbalancers: "5"
```

### 10.2 LimitRange（默认资源）

```yaml
apiVersion: v1
kind: LimitRange
metadata:
  name: mox-limit-range
  namespace: mox
spec:
  limits:
  - type: Container
    default:
      cpu: "500m"
      memory: "512Mi"
    defaultRequest:
      cpu: "100m"
      memory: "128Mi"
    max:
      cpu: "8"
      memory: "16Gi"
```

---

## 十一、总结

部署架构优化的核心是**"K8s 原生 + GitOps + 零停机 + 渐进式基础设施"**：

1. **每服务独立部署**：Deployment（无状态）/ StatefulSet（有状态，如图存储），独立镜像、独立扩缩容
2. **K8s 标准资源**：每服务包含 Deployment + Service + HPA + PDB + ConfigMap + ServiceAccount
3. **优雅启停**：startupProbe/readinessProbe/livenessProbe 三探针 + SIGTERM 优雅关闭 + 30s 宽限期
4. **配置外部化**：K8s ConfigMap + Secret（起步）→ Nacos（规模化），支持热更新
5. **服务发现**：K8s Service + CoreDNS（起步）→ Nacos（规模化），客户端负载均衡
6. **CI/CD GitOps**：GitHub Actions（CI）+ ArgoCD（CD），自动构建测试部署
7. **灰度发布**：滚动更新（默认）+ Istio 金丝雀（重要版本），零停机、可回滚
8. **多环境管理**：dev/staging/canary/production/dr，环境隔离
9. **资源管控**：ResourceQuota + LimitRange，防止资源争抢

---

*下一篇：[05-可观测性·安全·弹性](./05-observability-security-resilience.md)*

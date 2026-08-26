# 全维架构审计报告

> 对比三方架构：ymkj-server / infotopograph / mox-dualrpc+专家联盟
>
> 日期：2026-08-26 | 审计维度：架构最优性 / 服务对接 / 功能业务区分

---

## 一、三方架构全景对比

### 1.1 ymkj-server（Java 若依二次开发）

```
ymkj-server/
├── ymkj-parent/                    # 基础框架层
│   ├── ymkj-framework/            # 核心框架（三层分离）
│   │   ├── framework-api/         #   对外 API 契约
│   │   ├── framework-service-api/ #   服务间 RPC 契约
│   │   └── framework-service/     #   业务实现
│   ├── ymkj-system/               # 系统管理（用户/角色/权限/菜单）
│   ├── ymkj-quartz/               # 定时任务调度
│   ├── ymkj-socket/               # Netty 双协议（TCP+WebSocket）
│   ├── ymkj-metadata/             # 元数据管理
│   └── ymkj-sms/                  # 短信服务
│
└── ymkj-service/                   # 业务服务层
    ├── ymkj-ai/                    # AI 服务（三层分离）
    │   ├── ai-api/
    │   ├── ai-service-api/
    │   └── ai-service/
    ├── ymkj-rpa/                   # RPA 机器人（混合架构）
    │   ├── rpa-api/ rpa-service-api/ rpa-service/
    │   ├── python-cli/             #   Python 客户端
    │   ├── robot-platform/         #   机器人平台
    │   ├── code/ doc/ docker/      #   工程化配套
    │   └── sql/
    ├── ymkj-hr/                    # 人力资源
    ├── ymkj-sso/                   # 单点登录
    ├── ymkj-sms/                   # 短信
    ├── ymkj-fy/                    # 分佣/费用
    ├── ymkj-nwdx/                  # 内网通信
    ├── ymkj-view/                  # 视图层
    └── ymkj-web/                   # Web 接入层
```

**核心特征**：
- ✅ **三层分离**：每个模块 `*-api / *-service-api / *-service`
- ✅ **业务域划分**：AI / RPA / HR / SSO / SMS / FY / NWDX
- ✅ **基础框架层**：framework / system / quartz / socket / metadata
- ✅ **RPA 混合架构**：Java 服务 + Python CLI + Docker + 机器人平台
- ✅ **Dubbo RPC**：服务间通过 Dubbo 调用（service-api 层定义契约）
- ⚠️ **单体部署**：所有模块打包在一个 JVM 运行
- ⚠️ **Java 技术栈**：Spring Boot 2.7 + MyBatis + Netty

### 1.2 infotopograph（Rust 全栈 AI 平台）

```
infotopograph/
├── platform/
│   ├── crates/                      # 核心计算层（11个）
│   │   ├── mox-formulas-core/      #   公式引擎
│   │   ├── mox-intent-core/        #   意图识别
│   │   ├── mox-norm-core/          #   数据归一化
│   │   ├── xiaobai-*/              #   小白语音系列（asr/dsp/intent/operators）
│   │   └── bindings/                #   FFI 绑定
│   │
│   ├── services/                    # 服务层（36个）
│   │   ├── AI引擎：ai-agent / mox-ai-core / flow-ai
│   │   ├── 知识图谱：kg-hub / mox-graph-* / graph-algorithms
│   │   ├── 数据存储：mox-data-plane / mox-etl-wasm / mox-cloud-drive-*
│   │   ├── 流程算子：operator-core / operator-wasm / primiflow-*
│   │   ├── 业务治理：mox-compliance / mox-fusion / business-catalog
│   │   ├── 系统：mox-server / mox-system / mox-standards / mox-common-meta
│   │   └── 其他：mox-expert / template-market / optimizer / mox-t21-harness
│   │
│   └── gateway/runtime/             # 网关（半成品）
```

**核心特征**：
- ✅ **Rust 全栈**：tokio + axum + sqlx + wasmer + petgraph
- ✅ **自研图存储**：mox-graph-storage（RocksDB + Raft，100%自研）
- ✅ **核心计算层**：crates/ 分离公式/意图/归一化/语音
- ✅ **36个服务**：覆盖 AI/图谱/数据/流程/治理
- ⚠️ **单层架构**：只有 service 层，无 api/service-api 分离
- ⚠️ **技术域划分**：按技术能力（graph/ai/flow）划分，非业务域
- ⚠️ **零 gRPC**：无 tonic/prost 依赖，服务间直接函数调用
- ⚠️ **零多租户**：无租户隔离代码
- ⚠️ **单体倾向**：mox-server single-binary
- ⚠️ **基础框架缺失**：无统一的 framework/system/quartz 层

### 1.3 mox-dualrpc + 专家联盟（新建）

```
projects/mox-dualrpc/               # 通信基础设施
├── mox-dualrpc-macro/              # #[dual_rpc] / #[dual_rpc_service] 宏
├── src/
│   ├── config.rs                    # 服务器配置
│   ├── error.rs                     # gRPC↔JSON-RPC 错误映射
│   ├── registry.rs                  # 路由注册表 + L1缓存 + make_route
│   ├── server.rs                    # 双协议服务器 (axum+tonic)
│   └── transcoder.rs                # JSON↔Protobuf 转码 (L2)
└── examples/hello-world/            # 示例

专家联盟（设计中，7服务+1Sidecar）：
├── gateway-http / gateway-grpc      # 双端口协议分流
├── alliance-scheduler                # 调度/匹配/计划
├── alliance-executor                 # DAG执行
├── alliance-fusion                   # 结果融合
├── expert-registry                   # 专家注册
├── expert-agent                      # Agent运行时
├── expert-memory                     # 统一记忆
└── ai-inference-sidecar              # Python推理
```

**核心特征**：
- ✅ **双协议**：gRPC + JSON-RPC 零配置自动转码
- ✅ **宏驱动**：#[dual_rpc_service] 自动生成 register_routes()
- ✅ **三级缓存**：L0编译期 / L1进程内 / L2请求级
- ✅ **MCP兼容**：JSON-RPC 2.0 原生支持 Model Context Protocol
- ✅ **专家联盟7服务**：按职责拆分（调度/执行/融合/注册/Agent/记忆）
- ⚠️ **无三层分离**：专家联盟服务只有实现层，无 api/service-api
- ⚠️ **gRPC未实跑**：当前 gRPC 服务端为占位
- ⚠️ **无基础框架层**：无统一的 system/quartz/metadata

---

## 二、五大架构差距

### 差距1：三层分离 vs 单层耦合

| 维度 | ymkj（优） | infotopograph/专家联盟（劣） |
|------|-----------|------------------------------|
| 接口契约 | `*-api` 独立模块，纯DTO+接口 | 接口与实现在同一 crate |
| 服务间契约 | `*-service-api` 定义 Dubbo RPC | 无独立服务契约 |
| 客户端依赖 | 只依赖 api 模块，零实现泄漏 | 依赖整个 service crate |
| 独立编译 | api 层可独立编译发布 | 修改实现影响接口消费者 |
| 版本管理 | api 层可独立版本化 | 无接口版本概念 |

**优化方案**：专家联盟每个服务拆为三层：
```
expert-alliance/
├── alliance-scheduler-api/          # 对外 API（DTO + REST/JSON-RPC 契约）
├── alliance-scheduler-service-api/  # 服务间 gRPC 契约（.proto + generated stub）
└── alliance-scheduler-service/      # 业务实现（依赖 api + service-api）
```

### 差距2：业务域划分 vs 技术域划分

| 维度 | ymkj（优） | infotopograph（劣） |
|------|-----------|---------------------|
| 划分依据 | 业务域（AI/RPA/HR/SSO） | 技术能力（graph/ai/flow） |
| 团队对齐 | 一个业务域 = 一个团队 | 技术域跨多个业务 |
| 演进独立 | AI 业务可独立演进 | 图谱技术变更影响所有业务 |
| 专家联盟 | 无（ymkj无此概念） | 按职责拆分（调度/执行/融合） |

**优化方案**：infotopograph 应按业务域重组，而非技术域：
```
业务域划分（目标）：
├── 知识图谱域（kg-domain）：graph-storage + graph-service + kg-hub + graph-algorithms
├── AI智能域（ai-domain）：ai-agent + mox-ai-core + flow-ai + mox-expert
├── 数据治理域（data-domain）：data-plane + etl-wasm + compliance + standards
├── 流程自动化域（flow-domain）：operator-core + primiflow-* + hermes-flow-bridge
├── 平台基础域（platform-domain）：system + common-meta + server + gateway
└── 专家联盟域（alliance-domain）：scheduler + executor + fusion + registry + agent + memory
```

### 差距3：基础框架层缺失

| ymkj 基础模块 | infotopograph 现状 | 差距 |
|---------------|-------------------|------|
| framework（核心框架） | 无统一框架，各服务自建 | 严重 |
| system（用户/角色/权限） | mox-system 存在但简陋 | 中等 |
| quartz（定时任务） | 无统一调度 | 严重 |
| socket（通信） | axum WebSocket，无统一框架 | 中等 |
| metadata（元数据） | mox-common-meta 存在 | 轻微 |
| sms（通知） | 无统一通知服务 | 中等 |

**优化方案**：建设 `mox-framework` 基础框架层：
```
mox-framework/
├── mox-framework-api/              # 框架 API 契约
├── mox-framework-service-api/      # 框架服务间契约
└── mox-framework-service/          # 框架实现
    ├── auth/                        #   认证授权（JWT+RBAC+SSO）
    ├── tenant/                      #   多租户（三档隔离）
    ├── scheduler/                   #   定时任务（替代 quartz）
    ├── notification/                #   通知（短信/邮件/站内信/WebSocket）
    ├── metadata/                    #   元数据管理
    ├── observability/               #   可观测性（OTel+Prometheus+Loki）
    └── resilience/                  #   弹性容错（限流/熔断/降级/重试）
```

### 差距4：服务对接方式

| 维度 | ymkj | infotopograph | mox-dualrpc |
|------|------|---------------|-------------|
| 服务间通信 | Dubbo RPC（TCP+Hessian2） | 直接函数调用（单体） | gRPC（tonic） |
| 对外通信 | HTTP REST + WebSocket | axum REST | JSON-RPC + MCP + REST |
| 协议转换 | 无（Dubbo专用） | 无 | 自动转码（JSON↔Protobuf） |
| 跨语言 | Java only（Dubbo） | Rust only | gRPC 跨语言 + JSON-RPC 通用 |
| 服务发现 | Dubbo 注册中心（Nacos/ZK） | 无（单体） | 待建设（K8s Service / Consul） |

**优化方案**：以 mox-dualrpc 为通信底座，建立三层服务对接：

```
服务对接三层架构：
┌─────────────────────────────────────────────────┐
│  对外层（External）                               │
│  JSON-RPC 2.0 / MCP / REST / WebSocket          │
│  → 网关自动转码为内部 gRPC                        │
├─────────────────────────────────────────────────┤
│  内部层（Internal）                               │
│  gRPC (tonic) + Protobuf                         │
│  → 服务间高性能二进制通信，P99 < 1ms             │
├─────────────────────────────────────────────────┤
│  事件层（Event）                                  │
│  NATS JetStream / RabbitMQ                       │
│  → 异步事件驱动，解耦服务（进度推送/状态变更）     │
└─────────────────────────────────────────────────┘

跨语言对接：
├── Java/Dubbo 3.x → Triple协议 = gRPC，直接 tonic 调用
├── Python → gRPC (grpcio) 或 JSON-RPC
├── Node.js → gRPC (@grpc/grpc-js) 或 JSON-RPC
└── Go → gRPC (google.golang.org/grpc)
```

### 差距5：RPA混合架构参考（AI推理 sidecar）

| 维度 | ymkj-rpa | 专家联盟 AI sidecar |
|------|----------|---------------------|
| 主服务 | Java (rpa-service) | Rust (expert-agent) |
| 执行端 | Python CLI + robot-platform | Python (ai-inference-sidecar) |
| 通信 | HTTP REST | UDS (Unix Domain Socket) / gRPC |
| 部署 | Docker 容器 | K8s Sidecar（同Pod） |
| 工程化 | code/ doc/ docker/ 配套 | 待建设 |

**优化方案**：参考 ymkj-rpa 的工程化配套，完善 AI sidecar：
```
ai-inference-sidecar/
├── python/                    # Python 推理服务
│   ├── models/                #   模型管理
│   ├── engines/               #   推理引擎（vLLM/Transformers）
│   ├── adapters/              #   模型适配器（OpenAI/Anthropic/本地）
│   └── server.py              #   gRPC/JSON-RPC 服务
├── docker/                    # Docker 镜像
├── proto/                     # .proto 契约
├── tests/                     # 测试
└── docs/                      # 文档
```

---

## 三、功能业务区分优化

### 3.1 当前问题

infotopograph 的 36 个服务按**技术能力**划分，导致：
1. **业务边界模糊**：mox-expert 和 ai-agent 职责重叠
2. **跨服务依赖复杂**：一个业务需求需要调用 5+ 技术服务
3. **团队职责不清**：没有明确的业务域负责人
4. **演进耦合**：图谱技术变更影响 AI/流程/治理所有业务

### 3.2 目标：按业务域 + 平台基础双层划分

```
┌─────────────────────────────────────────────────────────────────┐
│                      业务域层（Business Domains）                  │
│  每个业务域独立演进、独立部署、独立团队                            │
├──────────────┬──────────────┬──────────────┬───────────────────┤
│ 知识图谱域    │ AI智能域      │ 流程自动化域  │ 数据治理域        │
│ (kg-domain)  │ (ai-domain)  │ (flow-domain)│ (data-domain)     │
├──────────────┼──────────────┼──────────────┼───────────────────┤
│ graph-storage│ ai-agent     │ operator-core│ data-plane        │
│ graph-service│ mox-ai-core  │ operator-wasm│ etl-wasm          │
│ kg-hub       │ flow-ai      │ primiflow-*  │ compliance        │
│ graph-algo   │ mox-expert   │ flow-bridge  │ standards         │
│ graph-streams│              │              │ catalog           │
└──────────────┴──────────────┴──────────────┴───────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│                    专家联盟域（Alliance Domain）                   │
│  跨域协调层，调度其他业务域的能力                                  │
├─────────────────────────────────────────────────────────────────┤
│ scheduler / executor / fusion / registry / agent / memory        │
└─────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│                    平台基础域（Platform Domain）                   │
│  所有业务域共享的基础设施，不包含业务逻辑                          │
├──────────────┬──────────────┬──────────────┬───────────────────┤
│ mox-framework│ mox-system   │ 通信层         │ 基础设施          │
│ (认证/租户/  │ (用户/角色/  │ mox-dualrpc  │ PostgreSQL/Redis  │
│  调度/通知/  │  权限/菜单)  │ (gRPC+JSON)  │ NATS/MinIO       │
│  可观测/弹性) │              │              │ 自研图存储        │
└──────────────┴──────────────┴──────────────┴───────────────────┘
```

### 3.3 每个业务域的标准三层结构

```
<domain-name>/
├── <domain>-api/                    # 对外 API 契约
│   ├── dto/                         #   数据传输对象
│   ├── enums/                       #   枚举定义
│   └── interfaces/                  #   REST/JSON-RPC 接口定义
├── <domain>-service-api/            # 服务间 gRPC 契约
│   ├── proto/                       #   .proto 文件
│   └── generated/                   #   生成的 gRPC stub
└── <domain>-service/                # 业务实现
    ├── src/
    │   ├── controller/              #   REST/JSON-RPC 控制器
    │   ├── service/                 #   业务逻辑
    │   ├── repository/              #   数据访问
    │   ├── grpc/                    #   gRPC 服务端
    │   ├── config/                  #   配置
    │   └── main.rs
    └── Cargo.toml
```

---

## 四、最优架构目标态

### 4.1 架构原则

1. **业务域优先**：按业务域划分，不按技术能力
2. **三层分离**：每个服务 api / service-api / service
3. **通信统一**：mox-dualrpc 双协议（gRPC 内部 + JSON-RPC 对外）
4. **平台共享**：mox-framework 提供认证/租户/调度/通知/可观测/弹性
5. **独立部署**：每个业务域可独立部署、独立扩缩、独立演进
6. **事件驱动**：同步 gRPC + 异步 NATS，避免长同步链

### 4.2 目标架构图

```
┌──────────────────────────────────────────────────────────────────────┐
│                           客户端接入层                                  │
│   Web/Mobile │ MCP Client(Claude/Cursor) │ 第三方API │ gRPC Client   │
└───────┬───────────────┬───────────────────┬───────────────┬──────────┘
        │ JSON-RPC/REST  │ MCP(JSON-RPC)    │ REST          │ gRPC
        ▼                 ▼                   ▼               ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        网关层 (mox-gateway)                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │
│  │ gateway-http│  │ gateway-grpc│  │  协议路由/   │                  │
│  │ :8080       │  │ :50051      │  │  认证/限流   │                  │
│  └──────┬──────┘  └──────┬──────┘  └─────────────┘                  │
└─────────┼──────────────────┼───────────────────────────────────────────┘
          │ 内部 gRPC         │
          ▼                    ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        业务域层（独立部署）                              │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌─────────────┐ │
│  │ 知识图谱域    │ │ AI智能域      │ │ 流程自动化域  │ │ 数据治理域   │ │
│  │ kg-domain    │ │ ai-domain    │ │ flow-domain  │ │ data-domain │ │
│  │ (3层分离)    │ │ (3层分离)    │ │ (3层分离)    │ │ (3层分离)   │ │
│  └──────────────┘ └──────────────┘ └──────────────┘ └─────────────┘ │
│  ┌──────────────────────────────────────────────────────────────────┐ │
│  │              专家联盟域（跨域协调）alliance-domain                │ │
│  │  scheduler → executor → fusion → registry → agent → memory       │ │
│  └──────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
          │                    │
          ▼                    ▼
┌──────────────────────────────────────────────────────────────────────┐
│                      平台基础域（共享服务）                              │
│  ┌───────────────┐ ┌───────────────┐ ┌───────────────┐              │
│  │ mox-framework │ │  mox-system   │ │ mox-dualrpc   │              │
│  │ 认证/租户/    │ │ 用户/角色/    │ │ gRPC+JSON-RPC │              │
│  │ 调度/通知/    │ │ 权限/菜单/    │ │ 双协议通信     │              │
│  │ 可观测/弹性   │ │ 审计/字典     │ │                │              │
│  └───────────────┘ └───────────────┘ └───────────────┘              │
└──────────────────────────────────────────────────────────────────────┘
          │                    │
          ▼                    ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        数据基础设施层                                    │
│  PostgreSQL │ Redis │ NATS JetStream │ MinIO │ 自研图存储(RocksDB+Raft)│
└──────────────────────────────────────────────────────────────────────┘
```

---

## 五、服务对接详细方案

### 5.1 内部服务间对接（gRPC）

```rust
// 服务A调用服务B（gRPC，tonic）
use mox_dualrpc::prelude::*;

// 1. 定义 .proto 契约（放在 <service>-service-api/proto/）
// service ExpertRegistryService {
//   rpc ListExperts(ListExpertsRequest) returns (ListExpertsResponse);
// }

// 2. 生成客户端（build.rs 自动生成）
// 3. 调用（零转码，纯二进制）
let mut client = ExpertRegistryServiceClient::connect("http://expert-registry:50051").await?;
let response = client.list_experts(Request::new(req)).await?;
```

### 5.2 对外服务对接（JSON-RPC / MCP）

```rust
// 外部客户端调用（JSON-RPC 2.0，自动转码为内部 gRPC）
// 客户端只需发 JSON，网关自动转码

// curl -X POST http://gateway:8080/rpc \
//   -d '{"jsonrpc":"2.0","method":"expert.registry.ListExperts","params":{},"id":1}'

// MCP 客户端（Claude Desktop）自动发现工具
// mcpServers: { "mox": { "url": "http://gateway:8080/mcp" } }
```

### 5.3 跨语言对接

| 对端语言/框架 | 对接方式 | 协议 | 说明 |
|--------------|---------|------|------|
| Java Dubbo 3.x | 直接 gRPC | Triple (=gRPC) | Dubbo 3.x Triple 协议 wire format 与 gRPC 完全一致 |
| Java Dubbo 2.x | Java 桥接 sidecar | dubbo:// → gRPC | 遗留系统需协议转换 |
| Python | gRPC (grpcio) | gRPC | AI 推理 sidecar |
| Node.js | gRPC 或 JSON-RPC | gRPC/JSON | 前端 BFF 层 |
| Go | gRPC | gRPC | 高性能中间件 |
| 任意语言 | JSON-RPC 2.0 | HTTP | 零依赖通用接入 |

### 5.4 事件驱动对接（NATS）

```rust
// 异步事件（解耦服务，避免长同步链）
use mox_dualrpc::prelude::*;

// 发布事件
nats.publish("task.progress", json!({"task_id":"...","progress":0.5})).await?;

// 订阅事件
let mut sub = nats.subscribe("task.progress").await?;
while let Some(msg) = sub.next().await {
    // 处理进度更新，推送到 WebSocket
}
```

---

## 六、优化路线图

### 阶段1：通信底座（已完成 mox-dualrpc v0.2）

- [x] 双协议服务器（gRPC + JSON-RPC）
- [x] #[dual_rpc_service] 宏自动注册
- [x] 三级缓存 + 错误映射
- [x] MCP 兼容
- [ ] gRPC 服务端实跑（v0.3）
- [ ] 流式 RPC + WebSocket（v0.4）

### 阶段2：平台基础层（mox-framework）

- [ ] mox-framework-api / service-api / service 三层分离
- [ ] 认证授权（JWT + RBAC + SSO）
- [ ] 多租户（三档隔离：逻辑/Schema/集群）
- [ ] 定时任务调度
- [ ] 通知服务（短信/邮件/站内信/WebSocket）
- [ ] 可观测性（OTel + Prometheus + Loki）
- [ ] 弹性容错（限流/熔断/降级/重试/超时）

### 阶段3：业务域重组

- [ ] 知识图谱域（kg-domain）：合并 graph-* + kg-hub
- [ ] AI智能域（ai-domain）：合并 ai-* + mox-expert
- [ ] 流程自动化域（flow-domain）：合并 operator-* + primiflow-*
- [ ] 数据治理域（data-domain）：合并 data-* + compliance + standards
- [ ] 每个域三层分离（api / service-api / service）

### 阶段4：专家联盟落地

- [ ] 7服务全部迁移到 mox-dualrpc
- [ ] 知识图谱驱动的专家匹配
- [ ] DAG 执行引擎
- [ ] 6种协作模式 + 6种融合策略
- [ ] 与4个业务域对接（调用图谱/AI/流程/数据能力）

### 阶段5：独立部署 + 生产级

- [ ] K8s 部署模板（每服务 Deployment + HPA + PDB + 探针）
- [ ] GitOps CI/CD
- [ ] 服务发现（K8s Service / Consul）
- [ ] 链路追踪全链路
- [ ] 99.95% SLA 保障

---

## 七、结论

### 7.1 当前架构是否最优？

**否。** 存在 5 大差距：
1. ❌ 无三层分离（接口与实现耦合）
2. ❌ 技术域划分而非业务域（边界模糊）
3. ❌ 基础框架层缺失（认证/租户/调度/通知重复建设）
4. ❌ 服务对接不统一（无 gRPC，单体函数调用）
5. ❌ AI sidecar 工程化不足（参考 ymkj-rpa）

### 7.2 mox-dualrpc 是否最优？

**基本最优（通信层）。** 已实现：
- ✅ 双协议（gRPC + JSON-RPC）零配置自动转码
- ✅ 宏驱动（#[dual_rpc_service]）
- ✅ 三级缓存
- ✅ MCP 兼容
- ⚠️ gRPC 服务端待实跑
- ⚠️ 流式/WebSocket 待实现

### 7.3 下一步优先级

1. **最高优先级**：建设 mox-framework 基础框架层（解决差距3）
2. **高优先级**：专家联盟服务三层分离（解决差距1）
3. **中优先级**：业务域重组（解决差距2，渐进式）
4. **中优先级**：mox-dualrpc v0.3 gRPC 实跑 + v0.4 流式
5. **低优先级**：AI sidecar 工程化配套（参考 ymkj-rpa）

---

*文档导航：[README](./README.md) | [专家联盟集成](./EXPERT_ALLIANCE_INTEGRATION.md) | [架构审计](./ARCHITECTURE_AUDIT.md)*

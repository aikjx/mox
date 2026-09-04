# Mox Platform 架构文档 — 企业级mox 模块化系统架构归一化

> **版本**: 3.0.0-ai-powered
> **最后更新**: 2026-08-27
> **架构模式**: 6层企业级架构 + Workspace多模块 + Trait驱动零改动扩展

---

## 目录

1. [架构总览](#1-架构总览)
2. [6层分层架构](#2-6层分层架构)
3. [核心模块清单](#3-核心模块清单)
4. [命名规范归一化](#4-命名规范归一化)
5. [目录结构规范](#5-目录结构规范)
6. [零改动扩展指南](#6-零改动扩展指南)
7. [依赖关系图](#7-依赖关系图)
8. [企业级处理流程](#8-企业级处理流程)
9. [归一化检查清单](#9-归一化检查清单)
10. [配置参考](#10-配置参考)

---

## 1. 架构总览

### 1.1 设计哲学

| 原则 | 说明 |
|------|------|
| **语言优先** | Rust无万能框架，架构靠工程组织（Workspace + Trait + 手工DI） |
| **Trait驱动** | 所有可扩展点通过Trait抽象，实现可替换、可Mock |
| **工厂模式** | Factory trait从配置创建实例，实现配置驱动自动组装 |
| **零改动核心** | 新增能力 = 实现Trait + 注册Factory + 加配置，核心代码零改动 |
| **编译期安全** | 依赖注入、类型检查全部在编译期完成，无运行时反射 |
| **模块边界强制** | 每个crate独立编译单元，Workspace天然隔离，杜绝大泥球 |

### 1.2 技术栈

| 层级 | 技术选型 | 对标SpringBoot |
|------|---------|---------------|
| Web框架 | axum (基于tower) | Spring MVC |
| 异步运行时 | tokio | Spring事件循环 |
| 中间件栈 | tower layer | Interceptor/AOP |
| 数据库 | sqlx | MyBatis/JPA |
| 配置 | config-rs + serde | @Configuration |
| 错误处理 | thiserror + anyhow | 全局异常处理器 |
| 插件系统 | wasmer (WASM) | - |
| 序列化 | serde + serde_json | Jackson |
| 日志 | tracing + tracing-subscriber | SLF4J/Logback |

---

## 2. 6层分层架构

```
┌─────────────────────────────────────────────────────────────────────┐
│  L6 接入层 (Access Layer)                                            │
│      mox-platform-gateway-svc  +  8域 api crates                    │
│      协议: REST / gRPC / GraphQL / WebSocket                         │
├─────────────────────────────────────────────────────────────────────┤
│  L5 集成层 (Integration Layer)  ★核心枢纽                            │
│      mox-platform-integration-core                                   │
│      ┌──────────────┬──────────────┬──────────────┬──────────────┐ │
│      │ 内置Factory    │ 企业级流程    │ 多协议网关    │ 扩展点注册    │ │
│      │ (5个开箱即用)  │ 错误码/trace  │ gRPC/GQL/WS  │ 8种类型       │ │
│      ├──────────────┴──────────────┴──────────────┴──────────────┤ │
│      │ 统一启动组装 Bootstrap + 健康检查 + 跨能力协调器 + 配置热更新 │ │
│      └──────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────┤
│  L4 对接能力层 (Integration Capabilities)                            │
│      ┌──────────────┬──────────────┬──────────────┬──────────────┐ │
│      │ AI网关        │ 插件系统      │ 政企适配      │ 连接器框架    │ │
│      │ mox-ai-core   │ mox-plugin-  │ mox-enterprise│ mox-connector│ │
│      │              │ core         │ -core        │ -core         │ │
│      │ 多模型路由    │ WASM沙箱     │ SSO/合规/白标 │ 第三方系统    │ │
│      │ 降级/熔断     │ 热加载/市场   │ 动态字段      │ 即插即用      │ │
│      └──────────────┴──────────────┴──────────────┴──────────────┘ │
├─────────────────────────────────────────────────────────────────────┤
│  L3 领域服务层 (Domain Services)                                     │
│      8个业务域: kg / ai / flow / data / cloud / voice / market / platform│
│      每个域: api/ + core/ + svc/ + sdk/ + svcapi/                  │
├─────────────────────────────────────────────────────────────────────┤
│  L2 平台核心层 (Platform Core)                                       │
│      mox-platform-iam-core / system-core / meta-core                 │
│      mox-platform-orchestrator-core / datastore-core / operator-core │
├─────────────────────────────────────────────────────────────────────┤
│  L1 基础框架层 (Foundation & Framework)                              │
│      mox-framework / mox-platform-foundation / mox-cloud-foundation  │
│      mox-platform-observability / mox-platform-paths                  │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.1 层级依赖规则

| 规则 | 说明 |
|------|------|
| **单向依赖** | 上层可依赖下层，下层不可依赖上层 |
| **同层隔离** | L4各能力层之间通过L5集成层协调，不直接互相依赖 |
| **领域隔离** | L3各业务域之间通过API/SDK通信，不直接依赖内部实现 |
| **基础纯净** | L1/L2只依赖第三方库，不依赖任何业务模块 |

---

## 3. 核心模块清单

### 3.1 L5 集成层（mox-platform-integration-core）

| 模块 | 文件 | 职责 |
|------|------|------|
| 启动组装 | `bootstrap/mod.rs` | IntegrationRuntime + Builder模式，统一组装所有能力 |
| 扩展点 | `extension/registry.rs` | ExtensionPoint + Registry，8种扩展点类型 |
| 工厂中心 | `factory/mod.rs` | 4个Factory trait + FactoryRegistry + AutoAssembler |
| 内置Factory | `builtin/` | OpenAi/Qwen/Anthropic/Webhook/OAuth2 5个内置Factory |
| 企业级流程 | `flow/` | 错误码 + trace_id + 限流 + 配置热更新 |
| 多协议网关 | `protocol/` | ProtocolHandler + gRPC + GraphQL + WebSocket + Router |
| 统一配置 | `config/mod.rs` | IntegrationConfig，4大能力配置集中管理 |
| 健康检查 | `health/mod.rs` | 4能力健康检查 + 汇总 + 历史趋势 |
| 协调器 | `coordinator/mod.rs` | 跨能力事件总线 + 能力注册 + 监听 |

### 3.2 L4 对接能力层

| Crate | 核心Trait | 内置实现 | 注册表 |
|-------|----------|---------|--------|
| `mox-ai-core` | `AiProvider` | OpenAI/Anthropic/Qwen | `ProviderRegistry` |
| `mox-plugin-core` | - (WASM) | - | `PluginRegistry` + `PluginLoader` |
| `mox-enterprise-core` | `SsoProvider` + `AuditLogger` + `DataMasker` + `DataResidencyController` | OAuth2/钉钉/企微/飞书 | `SsoManager` |
| `mox-connector-core` | `Connector` | Webhook | `ConnectorRegistry` |

### 3.3 L3 领域服务层（8域）

| 域 | Core模块 | Svc模块 | SDK模块 |
|----|---------|---------|---------|
| kg (知识图谱) | mox-kg-algo-core, mox-kg-meta-core | storage/service/streams/spark/hub/fusion | mox-kg-sdk |
| ai (AI) | mox-ai-core, mox-ai-intent-core | flow/expert/agent | - |
| flow (流程) | mox-flow-operator-core, mox-flow-optimizer-core | operator-wasm/primiflow/fusion/bridge | - |
| data (数据) | formula/norm/standards-core | plane/etl/compliance/catalog | formula-native/norm-intent-native |
| cloud (云) | - | master/volume/s3/filer | mox-cloud-sdk |
| voice (语音) | mox-voice-dsp-core | core/asr/intent/operator/desktop-app | mox-voice-dsp-py |
| market (市场) | - | template-svc | - |
| platform (平台) | iam/system/meta/orchestrator/datastore/operator/plugin/enterprise/connector-core | orchestrator-svc/enterprise-svc | test-harness/plugin-sdk |

---

## 4. 命名规范归一化

### 4.1 Crate命名规范

| 类型 | 命名模式 | 示例 |
|------|---------|------|
| 平台基础能力 | `mox-platform-{能力}-core` | `mox-platform-iam-core` |
| 对接能力层 | `mox-{能力}-core` | `mox-ai-core`, `mox-plugin-core` |
| 领域核心 | `mox-{域}-{能力}-core` | `mox-kg-algo-core` |
| 领域服务 | `mox-{域}-{能力}-svc` | `mox-ai-flow-svc` |
| SDK | `mox-{域}-sdk` | `mox-kg-sdk`, `mox-plugin-sdk` |
| 网关 | `mox-platform-gateway-svc` | - |
| 框架 | `mox-framework` | - |
| 基础 | `mox-{platform/cloud}-foundation` | `mox-platform-foundation` |

### 4.2 模块命名规范

| 类型 | 命名模式 | 说明 |
|------|---------|------|
| Trait | `{能力}Provider` / `{能力}Factory` / `{能力}Handler` | 接口抽象 |
| 实现 | `{厂商}{能力}Provider` | 具体实现 |
| 注册表 | `{能力}Registry` | 实例管理 |
| 配置 | `{能力}Config` | 配置结构体 |
| DTO | `{动作}Request` / `{动作}Response` | 数据传输对象 |
| 错误 | `{能力}Error` | 错误类型 |
| 构建器 | `{能力}Builder` | Builder模式 |
| 管理器 | `{能力}Manager` | 生命周期管理 |

### 4.3 文件命名规范

| 类型 | 命名 | 说明 |
|------|------|------|
| Trait定义 | `traits.rs` | 统一放trait定义 |
| 实现 | `{厂商}.rs` | 如 `openai.rs`, `dingtalk.rs` |
| 注册表 | `registry.rs` | 实例注册表 |
| 配置 | `config.rs` | 配置结构体 |
| DTO | `dto.rs` | 数据传输对象 |
| 错误 | `error.rs` | 错误类型 |
| 模块入口 | `mod.rs` | 模块声明+重导出 |
| 库入口 | `lib.rs` | crate入口 |

### 4.4 错误码规范

**6位数字错误码**，前2位分类，后4位序号：

| 前缀 | 分类 | 示例 |
|------|------|------|
| 10xxxx | 系统错误 | `E100001` 内部错误, `E100002` 超时 |
| 20xxxx | AI错误 | `E200001` Provider不存在, `E200003` 限流 |
| 30xxxx | 插件错误 | `E300001` 插件不存在, `E300003` 权限拒绝 |
| 40xxxx | 政企错误 | `E400001` SSO失败, `E400002` 合规违规 |
| 50xxxx | 连接器错误 | `E500001` 连接器不存在, `E500003` 超时 |
| 90xxxx | 集成错误 | `E900001` 配置错误, `E900002` Factory不存在 |

---

## 5. 目录结构规范

### 5.1 标准Crate结构

```
mox-{name}-core/
├── Cargo.toml              # crate配置，依赖统一用workspace引用
├── src/
│   ├── lib.rs              # crate入口：模块声明 + 统一重导出 + prelude
│   ├── traits.rs           # 核心Trait定义（如有）
│   ├── registry.rs         # 注册表（如有）
│   ├── config.rs           # 配置结构体（如有）
│   ├── error.rs            # 错误类型（如有）
│   ├── dto.rs              # 数据传输对象（如有）
│   ├── {impl}/             # 具体实现目录（如有多个实现）
│   │   ├── mod.rs
│   │   ├── openai.rs
│   │   └── qwen.rs
│   └── {submodule}/        # 子模块目录
│       ├── mod.rs
│       └── ...
└── tests/                  # 集成测试（如有）
```

### 5.2 领域域标准结构

```
domains/{domain}/
├── api/                    # API定义（HTTP handler + DTO）
├── core/                   # 核心业务逻辑（crate）
│   └── mox-{domain}-{capability}-core/
├── svc/                    # 服务实现（可执行binary）
│   └── mox-{domain}-{capability}-svc/
├── sdk/                    # 对外SDK（可选）
│   └── mox-{domain}-sdk/
└── svcapi/                 # 服务间API（可选）
```

### 5.3 集成层标准结构

```
mox-platform-integration-core/src/
├── lib.rs                  # 入口
├── bootstrap/              # 启动组装
├── builtin/                # 内置Factory实现
├── config/                 # 统一配置
├── coordinator/            # 跨能力协调器
├── extension/              # 扩展点注册表
├── factory/                # 工厂注册中心
├── flow/                   # 企业级处理流程
│   ├── error_codes.rs      # 统一错误码
│   ├── trace.rs            # trace_id传播
│   ├── rate_limit.rs       # 限流
│   └── config_hot_reload.rs # 配置热更新
├── health/                 # 健康检查
└── protocol/               # 多协议网关
    ├── traits.rs           # ProtocolHandler trait
    ├── grpc.rs             # gRPC
    ├── graphql.rs          # GraphQL
    ├── websocket.rs        # WebSocket
    └── router.rs           # 统一路由
```

---

## 6. 零改动扩展指南

### 6.1 新增AI Provider

**步骤**：
1. 实现 `AiProvider` trait（业务代码）
2. 实现 `AiProviderFactory` trait（工厂代码）
3. 调用 `factory_registry.register_ai_factory(Arc::new(MyFactory))`
4. 配置文件加一段：
```yaml
ai:
  providers:
    - id: my-ai
      name: 我的AI
      provider_type: my_ai
      api_base: https://api.myai.com
      api_key: sk-xxx
      enabled: true
```
5. 启动时 `AutoAssembler` 自动创建并注册 → **核心零改动**

### 6.2 新增连接器

**步骤**：
1. 实现 `Connector` trait
2. 实现 `ConnectorFactory` trait
3. 注册 + 加配置 → **核心零改动**

### 6.3 新增SSO协议

**步骤**：
1. 实现 `SsoProvider` trait
2. 实现 `SsoFactory` trait
3. 注册 + 加配置 → **核心零改动**

### 6.4 新增插件

**步骤**：
1. 使用 `mox-plugin-sdk` 开发WASM插件
2. 构建 `plugin.wasm` + `manifest.json`
3. 放入 `plugins/` 目录 或 通过插件市场安装
4. `PluginLoader` 自动加载 → **核心零改动**

### 6.5 新增协议接入

**步骤**：
1. 实现 `ProtocolHandler` trait
2. 注册到 `ProtocolRouter` + 加路由规则 → **核心零改动**

### 6.6 替换合规实现

**步骤**：
1. 实现 `AuditLogger` / `DataMasker` / `DataResidencyController` trait
2. 依赖注入替换 → **核心零改动**

---

## 7. 依赖关系图

### 7.1 核心依赖链

```
mox-platform-gateway-svc
    └── mox-platform-integration-core
            ├── mox-ai-core
            ├── mox-plugin-core
            ├── mox-enterprise-core
            ├── mox-connector-core
            └── mox-framework
                    └── (第三方库: tokio/serde/tracing/...)
```

### 7.2 依赖规则

| 规则 | 说明 |
|------|------|
| **workspace统一版本** | 所有依赖在根Cargo.toml的`[workspace.dependencies]`中声明，子crate用`{ workspace = true }`引用 |
| **禁止循环依赖** | Rust编译器直接报错，架构上杜绝 |
| **面向接口依赖** | 业务模块之间依赖Trait，不依赖具体实现结构体 |
| **唯一组装点** | 所有实例new只在L5集成层的bootstrap中，业务crate内部不new底层组件 |

---

## 8. 企业级处理流程

### 8.1 统一错误处理

```
业务错误 (thiserror)
    ↓ 转换
PlatformError (含错误码+trace_id+详情)
    ↓ 中间件转换
HTTP JSON Response (统一格式)
```

### 8.2 全链路追踪

```
请求入口 → 生成trace_id → 线程局部存储
    ↓
日志/tracing自动携带trace_id
    ↓
跨服务调用通过HTTP头(X-Trace-Id / traceparent)传播
```

### 8.3 限流

```
令牌桶算法
    ├── 全局限流 (所有请求共享一个桶)
    └── 按key限流 (按用户/租户/IP分别限流)
```

### 8.4 配置热更新

```
配置文件变化 → 文件监听/轮询 → 重新加载 → 回调通知 → 运行时生效
```

### 8.5 优雅启停

```
启动: 加载配置 → 组装组件 → 注册服务 → 健康检查 → 接收流量
关闭: 停止接收新请求 → 等待处理中请求完成 → 释放资源 → 注销服务 → 退出
```

---

## 9. 归一化检查清单

### 9.1 命名归一化

- [ ] 所有crate名称符合命名规范
- [ ] 所有Trait名称符合命名规范（Provider/Factory/Handler后缀）
- [ ] 所有注册表名称符合命名规范（Registry后缀）
- [ ] 所有配置结构体名称符合命名规范（Config后缀）
- [ ] 所有错误类型名称符合命名规范（Error后缀）
- [ ] 所有文件名称符合命名规范（snake_case）

### 9.2 结构归一化

- [ ] 所有crate有标准目录结构
- [ ] 所有lib.rs有统一重导出 + prelude
- [ ] 所有Trait定义在traits.rs或独立文件
- [ ] 所有实现按厂商/类型分文件
- [ ] 所有模块有mod.rs入口

### 9.3 依赖归一化

- [ ] 所有依赖在workspace.dependencies中声明
- [ ] 子crate用`{ workspace = true }`引用依赖
- [ ] 无循环依赖
- [ ] 无未使用依赖
- [ ] 同层模块之间通过Trait依赖，不依赖具体实现

### 9.4 扩展归一化

- [ ] 所有可扩展点有Trait定义
- [ ] 所有可扩展点有Factory trait
- [ ] 所有Factory注册到FactoryRegistry
- [ ] 所有配置驱动自动组装
- [ ] 新增扩展不需要修改核心代码

### 9.5 文档归一化

- [ ] 所有crate有lib.rs文档注释
- [ ] 所有公开Trait有文档注释
- [ ] 所有公开方法有文档注释
- [ ] 架构文档完整
- [ ] 扩展指南完整

---

## 10. 配置参考

### 10.1 集成配置完整示例

```yaml
# config/integration.yaml
runtime_name: mox-integration
environment: prod

# AI配置
ai:
  enabled: true
  default_provider: openai
  default_model: gpt-4o
  routing_strategy: priority
  auto_fallback: true
  circuit_breaker: true
  request_timeout_secs: 60
  max_retries: 2
  providers:
    - id: openai
      name: OpenAI
      provider_type: openai
      api_base: https://api.openai.com/v1
      api_key: ${OPENAI_API_KEY}
      models: [gpt-4o, gpt-4o-mini]
      enabled: true
      priority: 100
    - id: qwen
      name: 通义千问
      provider_type: qwen
      api_base: https://dashscope.aliyuncs.com/compatible-mode/v1
      api_key: ${QWEN_API_KEY}
      models: [qwen-max, qwen-plus]
      enabled: true
      priority: 200

# 插件配置
plugin:
  enabled: true
  plugin_dir: ./plugins
  hot_reload: true
  hot_reload_interval_secs: 10
  max_plugins: 100
  max_memory_mb: 256
  market:
    enabled: false
    base_url: ""
    api_token: ""
    auto_update_check: true
    update_check_interval_hours: 24

# 政企配置
enterprise:
  enabled: true
  sso:
    enabled: true
    default_provider: oauth2
  compliance:
    audit_log_enabled: true
    data_masking_enabled: true
    data_residency_region: china_mainland
    cross_border_control: true
  whitelabel:
    enabled: false
    brand_name: ""
    theme: default

# 连接器配置
connector:
  enabled: true
  global_timeout_secs: 30
  global_max_retries: 2
  connectors:
    - id: webhook-notify
      name: 通知Webhook
      connector_type: webhook
      protocol: rest
      endpoint: https://hooks.example.com/notify
      auth_type: bearer
      credentials:
        token: ${WEBHOOK_TOKEN}
      enabled: true

# 扩展点配置
extensions:
  - id: custom.ai.hook
    name: 自定义AI钩子
    extension_type: custom
    version: 1.0.0
    enabled: true

# 全局配置
global_timeout_secs: 30
telemetry_enabled: true
```

---

## 附录

### A. 相关文档

- `docs/architecture/01-overview.md` — 架构总览（详细）
- `docs/architecture/02-extension-guide.md` — 扩展开发指南
- `docs/architecture/03-factory-pattern.md` — 工厂模式详解
- `docs/architecture/04-error-code-reference.md` — 错误码参考手册

### B. 快速启动

```rust
use mox_platform_integration_core::prelude::*;

// 3行启动全部对接能力
let config = IntegrationConfig::load_from_file("config/integration.yaml").await?;
let runtime = IntegrationBootstrap::from_config(config).await?;
let health = runtime.health_check().await;
```

---

**文档维护**: 架构变更时同步更新此文档
**归一化状态**: ✅ 已归一化

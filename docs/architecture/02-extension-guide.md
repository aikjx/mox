# 扩展开发指南 — 零改动核心架构

> 本文档详细说明如何在不修改核心代码的前提下，为Mox Platform扩展新能力。

---

## 目录

1. [扩展模式总览](#1-扩展模式总览)
2. [新增AI Provider](#2-新增ai-provider)
3. [新增连接器](#3-新增连接器)
4. [新增SSO协议](#4-新增sso协议)
5. [开发WASM插件](#5-开发wasm插件)
6. [新增协议接入](#6-新增协议接入)
7. [替换合规实现](#7-替换合规实现)
8. [测试指南](#8-测试指南)

---

## 1. 扩展模式总览

### 1.1 核心模式：Trait + Factory + 配置

所有扩展都遵循统一的三段式模式：

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  实现Trait   │ ──→ │ 实现Factory │ ──→ │  加配置      │
│  (业务逻辑)  │     │ (创建实例)  │     │ (yaml/json) │
└─────────────┘     └─────────────┘     └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │ 注册到Registry│
                    │ (启动时自动)  │
                    └─────────────┘
```

### 1.2 扩展类型对照表

| 扩展类型 | Trait | Factory | 配置前缀 | 注册表 |
|---------|-------|---------|---------|--------|
| AI Provider | `AiProvider` | `AiProviderFactory` | `ai.providers[]` | `ProviderRegistry` |
| 连接器 | `Connector` | `ConnectorFactory` | `connector.connectors[]` | `ConnectorRegistry` |
| SSO协议 | `SsoProvider` | `SsoFactory` | `enterprise.sso.providers[]` | `SsoManager` |
| 协议接入 | `ProtocolHandler` | - | 路由规则 | `ProtocolRouter` |
| 审计实现 | `AuditLogger` | - | 依赖注入 | - |
| 脱敏实现 | `DataMasker` | - | 依赖注入 | - |
| 数据主权 | `DataResidencyController` | - | 依赖注入 | - |
| WASM插件 | - (WASM) | - | manifest.json | `PluginRegistry` |

---

## 2. 新增AI Provider

### 2.1 完整示例

**步骤1: 实现AiProvider trait**

```rust
// src/providers/myai.rs
use async_trait::async_trait;
use mox_ai_core::prelude::*;

pub struct MyAiProvider {
    api_base: String,
    api_key: String,
    client: reqwest::Client,
    models: Vec<String>,
}

impl MyAiProvider {
    pub fn new(api_base: String, api_key: String, models: Vec<String>) -> Self {
        Self {
            api_base,
            api_key,
            client: reqwest::Client::new(),
            models,
        }
    }
}

#[async_trait]
impl AiProvider for MyAiProvider {
    fn provider_id(&self) -> &'static str { "myai" }
    fn provider_name(&self) -> &'static str { "MyAI" }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Chat, Capability::ChatStream]
    }

    fn available_models(&self) -> Vec<String> {
        self.models.clone()
    }

    async fn chat(&self, req: &ChatRequest) -> AiResult<ChatResponse> {
        // 实现对话逻辑
        let url = format!("{}/chat/completions", self.api_base);
        let resp = self.client.post(&url)
            .bearer_auth(&self.api_key)
            .json(req)
            .send()
            .await
            .map_err(|e| AiError::Other(e.to_string()))?;

        let response: ChatResponse = resp.json().await
            .map_err(|e| AiError::Other(e.to_string()))?;
        Ok(response)
    }

    async fn chat_stream(&self, req: &ChatRequest) -> AiResult<BoxStream<'static, AiResult<StreamChunk>>> {
        // 实现流式对话（可选）
        Err(AiError::UnsupportedCapability("chat_stream".into()))
    }

    async fn health_check(&self) -> HealthStatus {
        // 实现健康检查
        HealthStatus::Healthy
    }
}
```

**步骤2: 实现AiProviderFactory trait**

```rust
// src/providers/myai_factory.rs
use async_trait::async_trait;
use mox_platform_integration_core::prelude::*;
use std::sync::Arc;

pub struct MyAiFactory;

#[async_trait]
impl AiProviderFactory for MyAiFactory {
    fn factory_type(&self) -> &'static str { "myai" }

    async fn create(&self, config: &FactoryConfig) -> anyhow::Result<Arc<dyn AiProvider>> {
        let api_base = config.get_str("api_base")
            .unwrap_or("https://api.myai.com/v1")
            .to_string();
        let api_key = config.get_str("api_key")
            .unwrap_or("")
            .to_string();
        let models: Vec<String> = config.config.get("models")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_else(|| vec!["myai-model".into()]);

        Ok(Arc::new(MyAiProvider::new(api_base, api_key, models)))
    }
}
```

**步骤3: 注册Factory（在应用启动时）**

```rust
// main.rs
use mox_platform_integration_core::prelude::*;

let runtime = IntegrationRuntime::builder()
    .with_config(config)
    .with_all_capabilities()
    .build()
    .await?;

// 注册自定义Factory
runtime.factory_registry().register_ai_factory(Arc::new(MyAiFactory));
```

**步骤4: 配置文件**

```yaml
ai:
  providers:
    - id: myai-prod
      name: MyAI生产环境
      provider_type: myai
      api_base: https://api.myai.com/v1
      api_key: ${MYAI_API_KEY}
      models: [myai-pro, myai-lite]
      enabled: true
      priority: 150
```

**步骤5: 启动时自动组装**

`AutoAssembler`会自动：
1. 读取配置中的`ai.providers[]`
2. 查找`factory_type`对应的Factory
3. 调用`factory.create(config)`创建实例
4. 注册到`ProviderRegistry`

→ **核心代码零改动**

---

## 3. 新增连接器

### 3.1 实现Connector trait

```rust
use async_trait::async_trait;
use mox_connector_core::prelude::*;

pub struct MysqlConnector {
    config: ConnectorConfig,
    // 连接池等
}

#[async_trait]
impl Connector for MysqlConnector {
    fn connector_id(&self) -> &str { &self.config.id }
    fn connector_name(&self) -> &str { &self.config.name }
    fn connector_type(&self) -> ConnectorType { ConnectorType::Database }

    fn supported_protocols(&self) -> Vec<String> { vec!["mysql".into()] }
    fn supported_operations(&self) -> Vec<String> {
        vec!["query".into(), "insert".into(), "update".into(), "delete".into()]
    }

    async fn connect(&self) -> ConnectorResult<()> { Ok(()) }
    async fn execute(&self, request: &ConnectorRequest) -> ConnectorResult<ConnectorResponse> {
        // 实现执行逻辑
        Ok(ConnectorResponse {
            success: true,
            status_code: 200,
            body: serde_json::json!({"result": "ok"}),
            headers: HashMap::new(),
            latency_ms: 10,
            error: None,
            retries: 0,
        })
    }
    async fn health_check(&self) -> bool { true }
    async fn close(&self) -> ConnectorResult<()> { Ok(()) }
}
```

### 3.2 实现ConnectorFactory + 注册 + 配置

参考AI Provider的模式，实现`ConnectorFactory` trait，注册到`FactoryRegistry`，配置加在`connector.connectors[]`。

---

## 4. 新增SSO协议

### 4.1 实现SsoProvider trait

```rust
use async_trait::async_trait;
use mox_enterprise_core::prelude::*;

pub struct CasSsoProvider {
    config: SsoConfig,
}

#[async_trait]
impl SsoProvider for CasSsoProvider {
    fn sso_type(&self) -> SsoType { SsoType::Cas }
    fn provider_id(&self) -> &str { &self.config.client_id }

    async fn get_auth_url(&self, state: &str) -> Result<String, SsoError> {
        // 构建CAS认证URL
        Ok(format!("{}?service={}&state={}", self.config.auth_url, self.config.redirect_uri, state))
    }

    async fn exchange_token(&self, code: &str) -> Result<SsoToken, SsoError> {
        // CAS ticket验证
        Ok(SsoToken {
            access_token: code.into(),
            refresh_token: None,
            expires_in: Some(3600),
            token_type: "Bearer".into(),
            id_token: None,
        })
    }

    async fn get_user_info(&self, token: &SsoToken) -> Result<SsoUser, SsoError> {
        // 获取用户信息
        Ok(SsoUser {
            external_id: "user123".into(),
            username: "testuser".into(),
            email: Some("test@example.com".into()),
            phone: None,
            display_name: Some("测试用户".into()),
            avatar_url: None,
            department: None,
            roles: vec!["user".into()],
            raw: serde_json::Value::Null,
        })
    }
}
```

### 4.2 实现SsoFactory + 注册 + 配置

参考AI Provider的模式。

---

## 5. 开发WASM插件

### 5.1 使用mox-plugin-sdk

```rust
// plugin/src/lib.rs
use mox_plugin_sdk::prelude::*;

// 插件主入口
async fn plugin_main(ctx: PluginContext) -> PluginResult<()> {
    ctx.log_info("插件启动");

    // 调用AI能力
    let response = ctx.ai_chat("你好，请介绍一下自己").await?;
    ctx.log_info(&format!("AI回复: {}", response.content));

    // 发布事件
    ctx.publish_event("plugin.initialized", json!({"status": "ok"}))?;

    Ok(())
}
```

### 5.2 生成manifest.json

```rust
use mox_plugin_sdk::prelude::*;

fn main() {
    let manifest = PluginManifestBuilder::new("com.example.myplugin", "我的插件", "1.0.0")
        .author("Example Inc")
        .description("这是一个示例插件")
        .permission(PluginPermission::AiChat)
        .permission(PluginPermission::EventPublish)
        .tag("example")
        .tag("demo")
        .build();

    let json = manifest.build_json().unwrap();
    std::fs::write("manifest.json", json).unwrap();
}
```

### 5.3 插件目录结构

```
my-plugin/
├── Cargo.toml          # 依赖 mox-plugin-sdk
├── src/
│   └── lib.rs          # 插件入口
├── manifest.json       # 插件描述符
└── plugin.wasm         # 编译产物（cargo build --target wasm32-unknown-unknown）
```

### 5.4 安装插件

**方式1: 手动安装**
```bash
cp -r my-plugin/ /path/to/plugins/
# PluginLoader自动热加载
```

**方式2: 插件市场安装**
```rust
use mox_plugin_core::prelude::*;

let installer = PluginInstaller::new(market_client, "./plugins");
installer.install("com.example.myplugin", "1.0.0").await?;
```

---

## 6. 新增协议接入

### 6.1 实现ProtocolHandler trait

```rust
use async_trait::async_trait;
use mox_platform_integration_core::prelude::*;

pub struct MqttHandler;

#[async_trait]
impl ProtocolHandler for MqttHandler {
    fn protocol_type(&self) -> ProtocolType { ProtocolType::Mqtt }

    async fn handle(&self, request: ProtocolRequest) -> ProtocolResponse {
        // 处理MQTT请求
        ProtocolResponse::ok(request.request_id, serde_json::json!({"ok": true}))
    }

    fn supported_routes(&self) -> Vec<String> {
        vec!["/mqtt/#".into()]
    }
}
```

### 6.2 注册处理器和路由规则

```rust
let router = ProtocolRouter::new();
router.register_handler("mqtt", Arc::new(MqttHandler));
router.add_route(RouteRule {
    rule_id: "mqtt-route".into(),
    protocol: ProtocolType::Mqtt,
    path_prefix: "/mqtt/".into(),
    handler_name: "mqtt".into(),
    rewrite_path: None,
    priority: 100,
    enabled: true,
});
```

---

## 7. 替换合规实现

### 7.1 替换审计日志实现

```rust
use async_trait::async_trait;
use mox_enterprise_core::prelude::*;

pub struct DatabaseAuditLogger {
    db_pool: sqlx::PgPool,
}

#[async_trait]
impl AuditLogger for DatabaseAuditLogger {
    async fn log(&self, event: AuditEvent) -> AuditResult {
        sqlx::query(
            "INSERT INTO audit_logs (event_id, event_type, severity, actor_id, ...)
             VALUES ($1, $2, $3, $4, ...)"
        )
        .bind(&event.event_id)
        .bind(&event.event_type)
        // ...
        .execute(&self.db_pool)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }
}
```

### 7.2 依赖注入替换

在应用启动时，将自定义实现注入到需要审计日志的组件中，替换默认的文件审计实现。

---

## 8. 测试指南

### 8.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_myai_chat() {
        let provider = MyAiProvider::new(
            "https://api.myai.com/v1".into(),
            "test-key".into(),
            vec!["test-model".into()],
        );
        // mock测试...
    }

    #[test]
    fn test_factory_create() {
        let factory = MyAiFactory;
        let config = FactoryConfig {
            id: "test".into(),
            name: "Test".into(),
            factory_type: "myai".into(),
            enabled: true,
            priority: 100,
            config: serde_json::json!({"api_base": "https://test.com", "api_key": "key"}),
            metadata: HashMap::new(),
        };
        // 测试工厂创建...
    }
}
```

### 8.2 集成测试

使用`mox-platform-test-harness`进行集成测试，验证扩展在完整运行时中的行为。

---

## 附录: 快速检查清单

新增扩展时，确认以下事项：

- [ ] 实现了对应的Trait
- [ ] 实现了对应的Factory（如需要）
- [ ] Factory注册到了FactoryRegistry
- [ ] 配置文件添加了对应配置
- [ ] 单元测试覆盖了核心逻辑
- [ ] 集成测试验证了端到端行为
- [ ] 文档更新了扩展说明
- [ ] **核心代码未修改** ✅

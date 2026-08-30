# mox-market-api · 市场领域 trait 契约层

MOX 平台市场领域的 API 契约 crate，定义插件注册、插件管理、扩展点注册与插件执行等核心能力的 trait 接口与数据结构，为市场/插件系统提供统一抽象。

## 功能特性

- **插件注册中心**：插件的注册、注销、查询、搜索、列举与版本更新
- **插件生命周期管理**：安装、卸载、启用、禁用、配置，支持租户级隔离
- **扩展点机制**：按领域注册扩展点，插件可挂载/卸载到指定扩展点
- **插件执行器**：同步执行插件逻辑，支持配置校验
- **丰富的插件元数据**：类型、状态、标签、配置 Schema、作者、版本等
- **统一错误类型**：`MarketApiError` 涵盖 NotFound / Conflict / Validation / Installation / Internal 五类错误

## 架构定位

本 crate 属于 MOX 平台 **market 领域 API 层**，位于：

```
platform/domains/market/
└── api/                    ← 本 crate（trait 契约 / DTO）
```

- 向上：供市场领域服务实现对应 trait
- 向下：供平台各业务域接入插件扩展能力
- 横向：作为市场领域各子模块之间的解耦契约

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-market-api = { path = "../api" }
```

### 基本用法

实现 `PluginRegistry` trait 示例：

```rust
use mox_market_api::{PluginRegistry, PluginInfo, MarketApiResult, MarketApiError};
use async_trait::async_trait;

struct MyPluginRegistry;

#[async_trait]
impl PluginRegistry for MyPluginRegistry {
    async fn register(&self, plugin: PluginInfo) -> MarketApiResult<()> {
        // 注册插件逻辑...
        Ok(())
    }

    async fn get(&self, plugin_id: &str) -> MarketApiResult<Option<PluginInfo>> {
        // 查询插件逻辑...
        Ok(None)
    }

    async fn search(
        &self,
        query: &str,
        plugin_type: Option<mox_market_api::PluginType>,
    ) -> MarketApiResult<Vec<PluginInfo>> {
        Ok(vec![])
    }

    // ... 其他方法
}
```

## 核心模块 / 类型

### 错误与结果
- `MarketApiError` — 市场领域统一错误枚举（NotFound / Conflict / Validation / Installation / Internal）
- `MarketApiResult<T>` — 结果类型别名

### 插件枚举
- `PluginStatus` — 插件状态（Available / Installed / Enabled / Disabled / Updating / Error）
- `PluginType` — 插件类型（Source / Transform / Sink / Filter / Enrich / Auth / Storage / Analytics / Ui / Other）

### 插件数据结构
- `PluginInfo` — 插件信息（ID、名称、版本、描述、作者、类型、状态、标签、配置 Schema 等）
- `PluginInstallation` — 插件安装记录（插件 ID、租户 ID、配置、安装时间、安装者）

### 扩展点
- `ExtensionPoint` — 扩展点（ID、名称、描述、所属域、所需接口、已注册插件列表）

### Trait 接口
- `PluginRegistry` — 插件注册中心 trait（register / unregister / get / search / list / update）
- `PluginManager` — 插件管理器 trait（install / uninstall / enable / disable / configure / list_installed）
- `ExtensionPointRegistry` — 扩展点注册表 trait（register / get / list / attach / detach）
- `PluginExecutor` — 插件执行器 trait（execute / validate）

## License

Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟

Licensed under the MIT License.

- GitHub 主仓: <https://github.com/aikjx/mox.git>
- GitCode 镜像: <https://gitcode.com/aikjx/mox>

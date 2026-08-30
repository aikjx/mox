# mox-project-graph-svc · 项目需求知识图谱 HTTP 服务

项目需求知识图谱的 HTTP API 服务层，基于 `mox-project-graph-core` 核心引擎，对外提供项目/需求/任务/人员/里程碑/问题/文档的图谱化管理 RESTful 接口。

## 功能特性

- **完整的 RESTful API**：项目、需求、任务、里程碑、人员、问题、文档的增删改查接口
- **图谱关系管理**：依赖关系、分配关系、包含关系的创建与解除
- **项目统计接口**：进度计算、任务统计、风险汇总
- **人员负载查询**：按人员维度统计任务分配与进度
- **影响范围分析接口**：变更传播链查询
- **统一 DTO 层**：请求/响应数据结构与核心领域模型解耦

## 架构定位

本 crate 属于 MOX 平台 **project 领域服务层**，位于：

```
platform/domains/project/
├── core/
│   └── mox-project-graph-core/  ← 图谱核心引擎
└── svc/
    └── mox-project-graph-svc/   ← 本 crate（HTTP 服务）
```

- 向上：面向前端 / 调用方提供 HTTP REST API
- 向下：依赖 `mox-project-graph-core` 核心引擎执行业务逻辑
- 定位：project 领域的对外服务入口，将核心引擎能力封装为 HTTP 接口

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-project-graph-svc = { path = "../svc/mox-project-graph-svc" }
```

### 基本用法

```rust
use mox_project_graph_svc::{AppState, router};
use std::sync::Arc;

// 构建应用状态
let state = AppState::new();

// 构建 Axum 路由
let app = router(Arc::new(state));

// 启动服务
let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
axum::serve(listener, app).await?;
```

## 核心模块 / 类型

### `dto` 模块
- 请求与响应数据传输对象（DTO）
- 输入校验与领域模型转换

### `server` 模块
- `AppState` — 应用状态，持有图谱引擎与共享资源
- `router` — 构建完整的 Axum 路由，包含所有 API 端点

## License

Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟

Licensed under the MIT License.

- GitHub 主仓: <https://github.com/aikjx/mox.git>
- GitCode 镜像: <https://gitcode.com/aikjx/mox>

# mox-kg-service-svc

Mox R3 Graph Service — 零第三方成品图数据库依赖的内嵌式图服务，nGQL / openCypher 双解析器 + 规则代价优化器 + 7 算法纯 Rust 实现。

## 功能特性

- **双查询语言解析器** — 支持 60 条标准 nGQL 语句 + 20 条 openCypher 语句解析，100% 自研
- **轻量查询优化器** — 规则 + 代价的混合优化，支持投影下推、5 跳空剪枝、重排序等优化策略
- **7 算法纯内联实现** — PPR / CNM / BC / HC / DC / Density / rawBDE，精度与 T5 single-source 一致
- **内嵌存储引擎** — 基于 `mox-kg-storage-svc` 的内嵌 StorageEngine，零外部数据库依赖
- **AC-15 故障注入与质量门禁** — 完整的故障注入框架与质量护栏，保障企业级可靠性
- **HTTP 适配层** — 可选 feature `http-adapter` 提供 6 个 KG + 4 个 AI 引擎 HTTP 接口
- **投影算子矩阵** — 20 种投影算子支持图数据投影与转换
- **8 阶段追踪** — 完整的查询执行追踪与可观测性

## 架构定位

本 crate 属于 MOX 平台 **L2/L3 服务层**，是 KG 域的核心服务实现：

```text
L1 Gateway (mox-platform-gateway-svc)
    │ HTTP
L2/L3 Service ← 本 crate（GraphServer / NgqlParser / CypherParser / Optimizer / AlgoBridge）
    │ uses
L4 Core (mox-kg-meta-core / mox-kg-algo-core / mox-kg-storage-svc)
    │
L5 Storage / Kernel
```

作为网关之后的 KG 业务入口，承接 nGQL 和 openCypher 查询，经优化器优化后下发到存储与算法层。

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-kg-service-svc = { path = "../mox-kg-service-svc" }

# 可选：启用 HTTP 适配层
mox-kg-service-svc = { path = "../mox-kg-service-svc", features = ["http-adapter"] }
```

### 基本用法示例

使用 GraphServer 执行 nGQL 查询：

```rust
use mox_kg_service_svc::{GraphServer, NgqlParser, ResultSet, GraphResult};

fn main() -> GraphResult<()> {
    // 创建图服务器实例
    let server = GraphServer::new();

    // 解析并执行 nGQL 语句
    let plan = NgqlParser::parse("GO FROM 'user-001' OVER knows YIELD $$.user.name;")?;
    let result = server.execute_plan(&plan)?;

    // 处理结果集
    match result {
        ResultSet::Rows(rows) => {
            for row in rows {
                println!("行: {:?}", row);
            }
        }
        ResultSet::Empty => println!("无结果"),
    }

    Ok(())
}
```

使用算法桥执行社区发现：

```rust
use mox_kg_service_svc::{AlgoBridge, AlgoGraph, Communities};

fn community_detection() {
    // 构建图
    let mut graph = AlgoGraph::new();
    graph.add_edge("a", "b", 1.0);
    graph.add_edge("b", "c", 1.0);
    graph.add_edge("c", "a", 1.0);
    graph.add_edge("d", "e", 1.0);

    // CNM 社区发现算法
    let algo = AlgoBridge::new();
    let communities = algo.cnm(&graph);
    println!("发现 {} 个社区", communities.len());
}
```

使用 HTTP 适配层（需启用 `http-adapter` feature）：

```rust
use mox_kg_service_svc::http_adapter::build_kg_ai_router;

#[tokio::main]
async fn main() {
    let app = build_kg_ai_router();
    // 绑定到 axum 服务器...
}
```

## 核心模块/类型列表

### `graph_server` 模块
- `GraphServer` — 图服务器主入口，执行查询计划
- `StorageEngine` — 存储引擎 trait 抽象
- `Direction` — 遍历方向枚举
- `Neighbor` — 邻接顶点结构体
- `EdgeRow` — 边行结构体

### `ngql_parser` 模块
- `NgqlParser` — nGQL 解析器，支持 60 条标准语句
- `PlanNode` — 查询计划节点

### `cypher_parser` 模块
- `CypherParser` — openCypher 解析器，支持 20 条标准语句

### `optimizer` 模块
- `Optimizer` — 查询优化器（规则 + 代价混合）
- `PlanOutput` — 优化后计划输出

### `algo_bridge` 模块
- `AlgoBridge` — 算法桥接器，封装 7 种图算法
- `AlgoGraph` — 算法用图数据结构
- `Communities` — 社区发现结果类型

### `result_set` 模块
- `ResultSet` — 标准化结果集枚举（Rows / Empty / ...）
- `PropValue` — 属性值原子类型枚举

### `projection_20` 模块
- `ProjectionOperator` — 投影算子 trait
- `ProjectionContext` — 投影上下文
- `ProjectionResult` — 投影结果
- `PROJECTION_OPERATORS` — 20 种投影算子注册表
- `projection_20_matrix()` — 投影算子矩阵函数

### `community_cnm` 模块
- CNM（Clauset-Newman-Moore）社区发现算法实现

### `ac15_faults` 模块
- `Ac15Fault` — AC-15 故障类型
- `FaultInjector` — 故障注入器
- `FaultPoint` — 故障点定义
- `FaultReport` — 故障报告
- `QualityGate` — 质量门禁

### `trace_8stages` 模块
- 8 阶段查询追踪系统，记录查询生命周期各阶段耗时与状态

### `error` 模块
- `GraphError` — 图服务错误类型
- `GraphResult<T>` — 统一结果类型别名

### `http_adapter` 模块（feature = "http-adapter"）
- `build_kg_ai_router()` — 构建 KG + AI HTTP 路由
- 6 个 KG 接口 + 4 个 AI 引擎接口

## License

Licensed under the MIT License.

See the LICENSE file in the workspace root for details.

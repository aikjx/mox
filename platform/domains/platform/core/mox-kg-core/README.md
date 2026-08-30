# mox-kg-core

Mox 自研知识图谱核心引擎 — 基于 Rust + RocksDB 架构，与 mox-dsql-core 深度融合的轻量级图数据库核心。

## 功能特性

- **自研图存储引擎** — 基于 RocksDB 的图存储层，支持顶点/边的高效增删改查
- **DSL 查询语言** — 自研图查询 DSL，支持 GET / SEARCH / COUNT 等多种查询模式
- **多跳遍历** — 支持单跳与多跳图遍历，可按边类型和方向过滤
- **路径查找** — 支持两顶点间最短路径查找，带深度限制
- **双层缓存** — 顶点缓存 + 遍历结果缓存，基于 parking_lot 实现高性能并发访问
- **企业级实体模型** — 内置企业官网实体关系模型初始化，快速构建示例图谱

## 架构定位

本 crate 属于 MOX 平台 **L4 服务核心层**，提供知识图谱领域的核心引擎能力：

```text
L1 Gateway (mox-platform-gateway-svc)
    │
L2/L3 Service (mox-kg-service-svc / mox-kg-hub-svc)
    │ uses
L4 Core ← 本 crate（KgManager / QueryEngine / GraphStorage / DslParser）
    │
L5 Storage (RocksDB / mox-dsql-core)
```

作为 KG 域的核心引擎，向上为 KG 服务层提供图操作 API，向下对接存储引擎。

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-kg-core = { path = "../mox-kg-core" }
```

### 基本用法示例

打开图谱并创建顶点：

```rust
use mox_kg_core::{KgManager, CreateVertexRequest, KgResult};

fn main() -> KgResult<()> {
    // 打开内存模式（测试用）
    let kg = KgManager::open_memory()?;

    // 创建顶点
    let vertex = kg.create_vertex(&CreateVertexRequest {
        id: "user-001".to_string(),
        vertex_type: "user".to_string(),
        properties: serde_json::json!({
            "name": "张三",
            "age": 28,
            "email": "zhangsan@example.com"
        }),
    })?;

    println!("创建顶点: {:?}", vertex);
    Ok(())
}
```

图遍历与 DSL 查询：

```rust
use mox_kg_core::{KgManager, TraverseDirection, KgResult};

fn query_example() -> KgResult<()> {
    let kg = KgManager::open_memory()?;
    kg.init_enterprise_website_model()?;

    // 单跳遍历：查询产品所属分类
    let neighbors = kg.traverse("product_1", TraverseDirection::Out, None)?;
    println!("产品出边邻接顶点: {} 个", neighbors.len());

    // DSL 查询：查找所有状态为 ACTIVE 的产品
    let result = kg.query_dsl("GET product WHERE status = 'ACTIVE'")?;
    println!("查询结果: {} 条", result.total);

    // 两跳路径查找
    let path = kg.find_path(
        "case_1",
        "cat_laptop",
        TraverseDirection::Out,
        None,
        3,
    )?;
    if let Some(p) = path {
        println!("找到路径，长度: {}", p.length);
    }

    Ok(())
}
```

## 核心模块/类型列表

### `dsl` 模块
- `DslParser` — DSL 解析器，支持 GET / SEARCH / COUNT 等查询语法
- 支持条件过滤、多跳路径表达式、属性比较

### `engine` 模块
- `QueryEngine` — 查询引擎，负责 DSL 执行与路径查找
- `QueryResult` — 查询结果结构体（success / total / vertices / edges / message）

### `error` 模块
- `KgError` — KG 核心错误枚举
- `KgResult<T>` — 统一结果类型别名

### `model` 模块
- `Vertex` — 顶点结构体（id / vertex_type / properties / created_at / updated_at）
- `Edge` — 边结构体（edge_type / source / target / properties / created_at）
- `CreateVertexRequest` — 创建顶点请求
- `CreateEdgeRequest` — 创建边请求
- `TraverseDirection` — 遍历方向枚举（In / Out / Both）
- `PathResult` — 路径查找结果
- `entity_types` — 内建实体类型常量（PRODUCT / PRODUCT_CATEGORY / CASE / NEWS / TEAM / FAQ）
- `edge_types` — 内建边类型常量（BELONGS_TO / PARENT_OF / SIMILAR_TO / USES / RELATED_TO / RESPONSIBLE_FOR / WORKS_WITH / REFERENCES）

### `storage` 模块
- `GraphStorage` — 图存储引擎，基于 RocksDB / SQLite 实现
- 支持顶点/边 CRUD、按类型列表、遍历、多跳、统计

### 顶层类型
- `KgManager` — 知识图谱管理器，高层 API 入口
  - `open(path)` — 打开或创建图谱数据库
  - `open_memory()` — 内存模式（测试用）
  - `create_vertex()` / `upsert_vertex()` / `get_vertex()` / `delete_vertex()`
  - `create_edge()` / `upsert_edge()` / `get_edge()` / `delete_edge()`
  - `traverse()` — 单跳遍历
  - `multi_hop_traverse()` — 多跳遍历
  - `find_path()` — 路径查找
  - `query_dsl()` / `query_dsl_with_params()` — DSL 查询
  - `init_enterprise_website_model()` — 初始化企业官网示例模型
  - `stats()` — 图谱统计
  - `benchmark()` — 性能基准测试
  - `clear_cache()` / `cache_status()` — 缓存管理

## License

Licensed under the MIT License.

See the LICENSE file in the workspace root for details.

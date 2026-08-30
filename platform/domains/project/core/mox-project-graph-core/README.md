# mox-project-graph-core · 项目需求知识图谱核心引擎

基于知识图谱的项目管理核心引擎，将项目、需求、任务、人员、里程碑、问题、文档、标签建模为图谱节点，通过关系边表达依赖、分配、包含、阻塞等关联，提供项目进度计算、影响分析、人员负载与关键路径识别等能力。

## 功能特性

- **全实体 CRUD**：项目 / 需求 / 任务 / 里程碑 / 人员 / 问题 / 文档 / 标签 的图谱化增删改查
- **依赖关系管理**：需求依赖、任务阻塞、项目包含、人员分配等多类型关系边
- **项目进度自动计算**：基于加权平均的进度自动汇总与统计
- **影响范围分析**：变更传播链追踪，评估需求/任务变更对上下游的影响
- **人员负载分析**：统计人员任务分配情况，识别过载与空闲资源
- **关键路径识别**：基于图谱遍历的项目关键路径自动识别
- **图谱遍历查询**：灵活的多跳遍历与条件过滤查询

## 架构定位

本 crate 属于 MOX 平台 **project 领域核心层**，位于：

```
platform/domains/project/
├── core/
│   └── mox-project-graph-core/  ← 本 crate（图谱核心引擎）
└── svc/
    └── mox-project-graph-svc/   ← HTTP 服务层
```

- 向上：被 project svc 层（HTTP API 服务）调用
- 向下：基于图数据库或内存图谱存储（由具体后端实现适配）
- 定位：项目领域的领域核心，封装项目管理的业务规则与图谱操作

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-project-graph-core = { path = "../core/mox-project-graph-core" }
```

### 基本用法

```rust
use mox_project_graph_core::{
    ProjectGraphEngine,
    ProjectProps, RequirementProps, TaskProps,
    ProjectStatus, RequirementStatus, TaskStatus,
    Priority,
};

// 创建图谱引擎
let engine = ProjectGraphEngine::new();

// 创建项目
let project = engine.create_project(ProjectProps {
    name: "AI 平台重构".into(),
    status: ProjectStatus::Active,
    // ... 其他属性
})?;

// 添加需求
let req = engine.add_requirement(project.id, RequirementProps {
    title: "语音识别热词支持".into(),
    status: RequirementStatus::InProgress,
    priority: Priority::High,
    // ...
})?;

// 添加任务并分配给人员
let task = engine.add_task(req.id, TaskProps {
    title: "实现 S1 模型层注入".into(),
    status: TaskStatus::Todo,
    // ...
})?;

// 计算项目统计
let stats = engine.project_stats(project.id)?;
println!("进度: {}%", stats.progress_percent);
println!("任务总数: {}", stats.total_tasks);

// 人员负载分析
let workloads = engine.person_workload()?;
for w in &workloads {
    println!("{}: {} 个任务", w.person_name, w.task_count);
}

// 影响范围分析
let impact = engine.impact_analysis(req.id)?;
```

## 核心模块 / 类型

### `schema` 模块 — 数据模型

#### 实体与关系类型
- `entity_types` — 实体类型常量集合
- `edge_types` — 关系类型常量集合

#### 状态枚举
- `ProjectStatus` — 项目状态
- `RequirementStatus` — 需求状态
- `TaskStatus` — 任务状态
- `Priority` — 优先级
- `RiskLevel` — 风险等级
- `IssueStatus` — 问题状态

#### 属性结构体
- `ProjectProps` — 项目属性
- `RequirementProps` — 需求属性
- `TaskProps` — 任务属性
- `MilestoneProps` — 里程碑属性
- `PersonProps` — 人员属性
- `IssueProps` — 问题属性
- `DocumentProps` — 文档属性
- `TagProps` — 标签属性

### `engine` 模块 — 图谱引擎

- `ProjectGraphEngine` — 项目图谱引擎主结构体，封装全部领域操作
- `ProjectStats` — 项目统计信息（进度、任务数、风险数等）
- `PersonWorkload` — 人员负载信息（任务数、进度分布等）

## License

Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟

Licensed under the MIT License.

- GitHub 主仓: <https://github.com/aikjx/mox.git>
- GitCode 镜像: <https://gitcode.com/aikjx/mox>

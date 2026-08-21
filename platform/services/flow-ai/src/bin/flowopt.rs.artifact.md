# 任务：为算子统一系统新建 flow-ai Rust crate，实现业务流程图优化 AI 核心算法

## 目标
在现有 Rust workspace（operator-unified-system）中新增 `crates/flow-ai`，用纯 Rust 实现
「流程图 + 六维关系拓扑网」双载体 Agent 内核的全部核心算法，覆盖：
分层并行网关、冲突检测/自动修复、关键路径、资源受限调度、关系网最短路/衰减、
流程图⇄代码双向映射。

## 交付内容（已落盘并验证）
- `crates/flow-ai/Cargo.toml` —— 版本 3.0.0-ai-powered，依赖 serde/serde_json/anyhow
- `crates/flow-ai/src/model.rs` —— 流程图 IR：Kahn 拓扑、位图传递闭包、节点/边/工具/访问/规则
- `crates/flow-ai/src/dataflow.rs` —— 自动并行化：RAW/WAR/WAW 冒险 + 传递归约 + 并行网关插入
- `crates/flow-ai/src/critpath.rs` —— CPM 关键路径 + 浮动时间
- `crates/flow-ai/src/conflict.rs` —— 冲突检测（事务/浏览器/文件锁/合规/环/悬垂） + 5 类自动修复
- `crates/flow-ai/src/schedule.rs` —— RCPSP 列表调度 + 算力路由（轻量/通用/Hermes3）
- `crates/flow-ai/src/topology.rs` —— 六维关系网：Dijkstra 最短路 + 权重衰减归档 + 级联影响
- `crates/flow-ai/src/codegen.rs` —— 流程图→分层 Python 工程 + Python 代码逆向反解析
- `crates/flow-ai/src/pipeline.rs` —— 六阶段端到端编排
- `crates/flow-ai/src/lib.rs` —— 汇总导出 + JSON/Mermaid 工具
- `crates/flow-ai/src/bin/flowopt.rs` —— CLI（demo/optimize/reverse/mermaid）
- `crates/flow-ai/README.md` —— 模块说明与实测数据

## 验证结果
- 编译：cargo build --workspace 通过，flow-ai 零警告
- 测试：**50 单元测试 + 1 文档测试，全部通过**
- 端到端实测：
  - 政务演示：串行 1825ms → 并行 1020ms（1.79x，省 44.1%），剪除 3 条伪依赖，0 剩余冲突（自动修复 5 处）
  - 生成的 Python 工程真实执行：并发调度生效（1.32x 实测加速），异常触发正确路由到处理器并执行一次、流程不中断
  - 逆向解析：Python RPA 代码 → 流程图，恢复循环/分支/工具分类，并报告缺失异常保护与 else 分支的缺陷
  - 浏览器容量=1 严格约束（web1 0-500ms、web2 500-900ms 零重叠）
  - 脱敏 Guard 0-5ms 支配数据库节点（db 从 5ms 起跑）

## 过程中发现并修复的真实缺陷
1. 自动修复插入的串行边被数据流分析当伪依赖剪掉 → 引入 `EdgeKind::Mutex` 硬约束边，且修复器对已有软边做「升级」而非跳过
2. 零耗时控制节点（Start）独占一层，把可执行任务挤到下一层、破坏并行 → 分层按「可执行节点深度」而非「节点深度」聚合
3. Guard 被当伪依赖剪除、与被保护节点并发 → Guard 强制支配其后继
4. 逆向解析误把 docstring/return 语句当节点 → 跳过文档字符串、`return XxxTool.call()` 识别
5. 异常处理节点泄漏进正常执行层、无错也触发 → 纯异常入边的节点排除出 LAYERS，仅在失败时路由触发
6. 双下划线标识符触发 Python 名称修饰导致引用失败 → 标识符生成统一加 `op_` 前缀

## 结论
flow-ai crate 已完整实现需求中「流程图优化 + 关系网」全部核心算法，并通过编译、
单元测试、端到端执行三重验证。可直接并入 runtime 或前端可视化链路。建议下一步：
把 optimize 报告通过 HTTP API 暴露给前端 Three.js 力导向图做实时联动展示。

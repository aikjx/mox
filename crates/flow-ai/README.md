# flow-ai —— 业务流程图优化 AI 核心算法库（Rust）

面向「流程图 + 关系网」双载体 Agent 内核，把原生线性 ReAct 架构升级为
**「图谱优先 → 流程约束 → 推理兜底」** 三层架构。

纯 Rust 实现，零重型依赖（仅 serde / serde_json / anyhow），可嵌入 runtime、
编译进 WASM，或作为 CLI 独立使用。

## 一、模块与算法

| 模块 | 解决的问题 | 核心算法 |
|------|-----------|---------|
| `model` | 流程图统一 IR | Kahn 拓扑排序、位图压缩传递闭包 |
| `dataflow` | 串行流程自动并行化 | RAW/WAR/WAW 冒险分析 + 传递归约 |
| `critpath` | 找瓶颈、算工期 | CPM 双向遍历 + 总浮动时间 |
| `conflict` | 异常/合规前置拦截 | 并发资源冲突检测 + 自动修复 |
| `schedule` | 真实资源下的排程 | RCPSP 列表调度（upward rank 优先级） |
| `topology` | 六维实体关系网 | Dijkstra 加权最短路 + 权重衰减归档 |
| `codegen` | 流程 ⇄ 代码双向映射 | 分层代码生成 + 缩进敏感结构反解析 |
| `pipeline` | 端到端编排 | 六阶段流水线 |

### 1. 自动并行化（核心价值）

原生流程图把**书写顺序**当成**执行依赖**，这是串行的根因。
本库用编译器的三类冒险分析，把顺序边分解为真依赖与伪依赖：

- **RAW**（A 写 x，B 读 x）→ 必须保序，真数据依赖
- **WAR / WAW** → 保序以保证确定性
- **无共享资源** → 判定为伪依赖，**剪掉并自动插入并行网关**

叠加**副作用序**保护：`Shell`/`Human`/非幂等 `Browser|Http` 之间保持原始
相对顺序，避免「优化出正确性事故」。

### 2. 冲突检测与自动修复

覆盖数据库事务、浏览器多实例抢占、文件读写锁、政务合规脱敏，以及环、
不可达、悬垂节点、分支不完整、缺失异常路径等结构缺陷。

命中 `Blocking` 级冲突时**拒绝出码** —— 这就是「异常分支前置拦截」。
`auto_repair` 可自动注入脱敏 Guard、互斥边、异常边。

> 关键设计：修复注入的是 `EdgeKind::Mutex` **硬约束**边。若用普通顺序边，
> 下一轮数据流分析会正确地把它当成伪依赖剪掉，导致冲突「死灰复燃」。
> 同理，`Guard` 节点强制支配其后继，否则校验会与被保护节点并发执行而失效。

### 3. 资源受限调度

关键路径是**无限资源**下的理论下界；真实场景浏览器只有 1 个实例。
列表调度以「剩余关键路径长度」为优先级，在资源约束下求近似最优排程，
输出每个节点的 start/finish、各池峰值与利用率。

## 二、CLI 用法

```bash
cargo run -p flow-ai --bin flowopt -- demo                    # 内置政务场景演示
cargo run -p flow-ai --bin flowopt -- optimize flow.json --out ./proj
cargo run -p flow-ai --bin flowopt -- reverse legacy.py --out flow.json
cargo run -p flow-ai --bin flowopt -- mermaid flow.json
```

## 三、库用法

```rust
use flow_ai::prelude::*;

let mut g = FlowGraph::new("demo", "示例");
g.add_node(FlowNode::task("a", "读文件", ToolKind::File, 300)
    .with_access(Access::write("var:x")));
g.add_node(FlowNode::task("b", "查库", ToolKind::Database, 400)
    .with_access(Access::write("var:y")));
g.add_node(FlowNode::task("c", "汇总", ToolKind::Compute, 100)
    .with_access(Access::read("var:x"))
    .with_access(Access::read("var:y")));
g.add_edge(FlowEdge::seq("a", "b"));   // 人为串联
g.add_edge(FlowEdge::seq("b", "c"));

let rep = optimize(&g, &OptimizeConfig::default());
println!("{}", rep.summary());   // a、b 自动并行
```

## 四、实测结果（内置政务演示）

```
串行耗时      : 1825 ms
关键路径下界  : 1020 ms
资源受限排程  : 1020 ms
实际加速比    : 1.79x (节省 44.1%)
剪除伪依赖    : 3 条
并行层 / 峰值 : 4 层 / 3 并发
冲突          : 0 项（阻断 0，自动修复 5）
代码生成      : 6 个文件 / 300 行
优先优化节点  : web1 > web2 > merge
```

排程验证浏览器容量=1 被严格遵守（web1 `0-500`，web2 `500-900`，零重叠），
脱敏 Guard 在 `0-5ms` 支配数据库节点（db 从 `5ms` 起跑）。

## 五、生成代码的分层结构

```
generated/
├── tools.py      工具层：各工具适配器 + Semaphore 资源池（与 pools 配置一一对应）
├── tasks.py      业务层：每节点一函数，docstring 标注读写集
├── errors.py     异常层：异常类型 + EXCEPTION_ROUTES 路由表
├── scheduler.py  调度层：LAYERS 并行层 + ThreadPoolExecutor 下发
└── main.py       入口
```

生成代码已实测可执行：并发调度生效（1.32x 实测加速），异常触发时正确路由到
处理器并执行一次，且流程不中断。

## 六、测试

```bash
cargo test -p flow-ai        # 50 单元测试 + 1 文档测试，全绿
```

覆盖：伪依赖识别、RAW/WAW 保留、副作用保序、CPM 浮动时间、四类冲突检测、
自动修复幂等性、资源容量约束、图谱最短路/衰减/级联影响、代码生成与反解析
round-trip、异常处理器不得进入正常执行层、Python 标识符名称修饰规避。

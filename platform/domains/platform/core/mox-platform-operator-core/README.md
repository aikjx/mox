# mox-platform-operator-core

MOX 平台算子核心抽象 — 跨域通用算子模型，从 `mox-flow-operator-core` 下沉，供 ai / voice / kg / flow 等多域共享，消除跨域逆向依赖。

## 功能特性

- **纯内核层（L6 Kernel）** — 零外部依赖（仅 std），定义纯数据结构与守恒律数学核心
- **类型系统** — `TypeIdentifier` / `TypeCheck` / `TypeTag` / `TypePair` 构建强类型算子契约
- **资源守恒** — `ResourceCost` / `ResourceUsage` / `ResourceLimits` + `L2Conservation` 守恒律校验
- **守恒检查器** — `ConservationChecker` 对算子输入输出进行资源守恒验证
- **Serde 扩展** — `kernel_ext` 以 DIP 方式为 kernel 类型提供序列化/反序列化能力
- **Monad 抽象** — `monad` 模块提供算子组合的函数式抽象

## 架构定位

本 crate 属于 MOX 平台 **L6 Kernel 层**，是算子统一系统（OUS）的跨域共享核心：

```text
L3 Business (ai-agent-svc / kg-hub / flow-fusion-svc)
    │ uses
L4 Operator Impl (domain-specific operators)
    │ impls
L6 Kernel ← 本 crate（TypeIdentifier / ConservationChecker / ResourceCost / ...）
    │
L7 Infrastructure (std only)
```

从 `mox-flow-operator-core` 下沉后，各业务域（AI / KG / Flow / Voice）可直接依赖本 crate，
避免了所有域都依赖 flow-operator-core 造成的逆向依赖问题。

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-platform-operator-core = { path = "../mox-platform-operator-core" }
```

### 基本用法示例

类型检查与守恒验证：

```rust
use mox_platform_operator_core::{
    TypeIdentifier, TypeCheck, TypeTag, TypePair,
    ResourceCost, ResourceUsage, ResourceLimits,
    ConservationChecker, L2Conservation,
};

// 定义算子的输入输出类型契约
let input_type = TypeIdentifier::new(vec![TypeTag::Text, TypeTag::Vector]);
let output_type = TypeIdentifier::new(vec![TypeTag::Graph]);

// 类型兼容性检查
let checker = TypeCheck::new();
assert!(checker.compatible(&input_type, &output_type).is_ok());

// 资源守恒校验
let input_cost = ResourceCost {
    compute: 10.0,
    memory: 256.0,
    io: 5.0,
};
let output_cost = ResourceCost {
    compute: 8.0,
    memory: 128.0,
    io: 3.0,
};

let limits = ResourceLimits {
    max_compute: 100.0,
    max_memory: 1024.0,
    max_io: 50.0,
};

let conservation = ConservationChecker::new(limits);
assert!(conservation.check_l2(&input_cost, &output_cost).is_ok());
```

使用内建类型：

```rust
use mox_platform_operator_core::builtin;

// 使用预定义的标准类型
let text_type = builtin::types::text();
let graph_type = builtin::types::graph();
let vector_type = builtin::types::vector();
```

## 核心模块/类型列表

### `kernel` 模块（L6 纯内核）
- `TypeIdentifier` — 类型标识符，由一组 `TypeTag` 组成
- `TypeTag` — 原子类型标签（Text / Vector / Graph / Image / Audio / ...）
- `TypePair` — 输入输出类型对
- `TypeCheck` — 类型兼容性检查器
- `ResourceCost` — 资源消耗描述（compute / memory / io）
- `ResourceUsage` — 资源使用量统计
- `ResourceLimits` — 资源上限约束
- `ConservationChecker` — 守恒律检查器
- `L2Conservation` — L2 范数守恒律
- `builtin` — 内建标准类型与常量

### `kernel_ext` 模块（Serde 扩展）
- 为 kernel 中所有核心类型提供 `Serialize` / `Deserialize` 实现
- 采用 DIP（Dependency Inversion Principle）方式，保持 kernel 零外部依赖

### `monad` 模块
- 算子组合的函数式抽象
- 支持链式调用与算子管道

### 顶层类型
- `OperatorError` — 算子错误类型（TypeMismatch / ExecutionError / Other）
- `Result<T>` — 算子结果类型别名
- `CRATE_ID` — Crate 标识符常量
- `CRATE_VERSION` — Crate 版本常量

## License

Licensed under the MIT License.

See the LICENSE file in the workspace root for details.

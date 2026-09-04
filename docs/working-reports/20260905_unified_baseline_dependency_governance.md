# 统一动态基座后续依赖治理（2026-09-05）

## 交付范围

本轮继续治理统一基座之上的历史依赖，保留动态 SQL、动态流程和业务数据库基线。不是全仓迁移完成证明，也不代表四领域独立部署验收通过。

### 已实施

1. **流程算法下沉**：新增 `platform/domains/ai/core/mox-ai-flow-core`，承接原 `mox-ai-flow-svc` 的 10 个算法模块及库函数、单元测试。原服务包保留公开模块路径、服务元数据和 `flowopt` CLI，通过重导出共享同一套类型。`mox-ai-expert-core` 直接依赖算法 Core，消除 Core → Service。
2. **云存储依赖纠正**：`mox-cloud-store-core` 的可选 `erasure` 功能直接依赖 `mox-cloud-kernel`。原 Volume 服务早已重导出该内核的 `EcProfile`、`ReedSolomonEngine`，此次切换没有复制算法或改变类型。
3. **共享算子错误**：完整 `OperatorError` 下沉到现有 `mox-platform-operator-core`，流程算子保留旧错误及 Result 的重导出。AI Agent 改为依赖平台算子核心，消除 ai → flow 的这条逆向依赖。平台原先占位的 TypeMismatch 字段由 String 统一为流程算子既有的 TypeId；仓内没有发现占位字符串构造调用，仓外使用者需留意此处源码兼容变化。
4. **联盟测试依赖归一**：混合匹配器和执行器的 `bench_alliance.rs` 从 scheduler-core/tests 移到 executor-core/tests；移除 scheduler-core 对 executor-core 的 dev-dependency。双向测试引用变为单向，保留全部基准代码。两个联盟服务各移除未使用的 alliance-core、platform-foundation、mox-error 直接依赖。
5. **检查工具修复**：`architecture_constraint_test.py` 使用实际 Cargo manifest 路径识别层和域，替代迁移前的旧 crate 名表；显式读取 UTF-8、从脚本位置定位仓库、合并重复依赖边；P1 层违规真正阻断；列出未分类 crate；API/proto 和 SDK 不再计作跨域内部直连。补充分类、层违规、契约边界、循环依赖回归测试。

## 门禁口径与未完成事项

两个 Python 工具的口径不同，不能直接相加或互相替代：

| 工具 | 当前结果 | 含义 |
|---|---|---|
| `tools/arch_test.py --quiet` | P0=2、P1=3、P2=0，退出 1 | 沿用原命名和域方向规则；原结果为 P0=3、P1=5 |
| `tools/architecture_constraint_test.py` | P0=0、P1=4、P2=17，退出 1 | 实际路径分类，含 normal/dev/build 与可选依赖的保守声明图；128 crates、283 条去重内部边 |

声明图已无循环依赖；扇出 ≥10 的模块从 4 个降至 2 个（gateway=13、orchestrator=17）。这是去除未使用依赖后的指标改善，不能称为已经拆分两个服务的内部业务实现。新旧层检查结果不可直接比较：旧名称表曾漏掉大多数新命名的 crate。

原主门禁剩余：

- P0：kg-hub → flow-fusion；platform-integration-core → ai-core。
- P1：kg-hub → flow-fusion、primiflow → ai-flow-svc、ai-agent → kg-hub 的跨域服务直连。

修正后的路径检查另检出两个 P1 层依赖：cloud-api → cloud-store-core、shared/unified-algo-core → flow-operator-core；加上两个高扇出模块，共 4 个 P1。另有 16 条跨域内部直连告警和 1 条测试工具反向依赖告警。共享目录按基础层检查，API/proto 按契约层检查；仅 `mox-arch-test` 未分类并显式列出。

后续需要提取图谱/融合的公共契约、将平台集成装配迁到上层运行时，并通过 API/SDK 或远程适配器切断服务实现直连。四领域独立进程启动、租户/权限隔离、数据迁移和事务补偿仍须各自验收；不能以本轮 crate 测试代替。移动浏览器和完整 workspace/PyO3 验收本轮没有执行。

## 验证

| 范围 / 命令 | 结果 |
|---|---|
| `cargo test -p mox-ai-flow-core -p mox-ai-flow-svc -p mox-ai-expert-core -p mox-ai-expert-svc --lib` | 算法 Core 83、专家 Core 101、专家 Service 191 通过；专家 Service 原有 1 项 ignored；兼容门面无内嵌单测 |
| `cargo test -p mox-ai-agent-svc -p mox-flow-operator-core -p mox-platform-operator-core --lib` | 133 + 41 + 26 = 200 通过 |
| `cargo test -p mox-cloud-store-core --all-targets --features erasure` | 58 通过，包括分片丢失重建、字节一致性和装饰器组合矩阵 |
| 联盟 scheduler/executor 的 Core、Service 四包 `--all-targets` | 136 通过，包括 1 项性能基准；随后基于新算法门面再跑功能集 `-- --skip bench_alliance_round7`，135 通过、1 项过滤 |
| `cargo test -p mox-ai-flow-svc --test core_compatibility` | 1 通过，验证旧服务路径与 Core 类型身份、JSON 和 Mermaid 一致 |
| `cargo test -p mox-flow-operator-core --test shared_error_compatibility` | 1 通过，验证旧错误路径与共享平台错误类型身份一致 |
| `cargo test -p mox-ai-flow-core --doc` | 1 通过 |
| `cargo check -p mox-ai-flow-svc --bin flowopt` | 通过 |
| `python -m unittest discover -s tools -p test_architecture_constraint.py` | 4 通过 |
| `git diff --check` | 通过 |

本轮去重计数：772 项通过，另有 1 项原有 ignored。772 = 375（流程/专家）+ 200（Agent/算子）+ 58（云）+ 136（联盟含基准）+ 3（兼容性/文档） 。此外 Python 门禁回归 4 项通过，合计 776；重复执行的联盟功能集未重复计数。

性能基准完成耗时 445.41 秒，3/5/10 节点、每组 20 次端到端执行成功率均为 1.0。结果单独保存在 `20260905_alliance_benchmark.json`；旧基准程序写死了环境日期和 Rust 版本，这些字段不能作为本次环境证明。初次计划停止长时间采样时，基准已自行完成并以退出码 0 结束。

构建出现已有 `default-features`、未来 Rust 兼容性及 Windows 增量编译目录访问警告，以上定向测试退出码均为 0。两项架构门禁仍退出 1，未降低阈值或增加 crate 豁免来取得通过。

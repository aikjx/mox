# info-graph —— 关图规范（GR-STD-V1.0）参考实现

纯 Rust std、零外部依赖，可离线编译。对「信息关联关系图开发规范」的最小可运行工具：
扫描工程 → 一切信息抽象为节点、依赖/交互抽象为边 → 校验 → 导出 Mermaid → 同步比对（CI 门禁）。

## 构建

```bash
cd tools/info-graph
cargo build --release        # 无需网络
# 产物：target/release/info-graph
```

（本 crate 为独立工具，未加入上层 operator-unified-system workspace，靠 Cargo.toml 内空 `[workspace]` 表隔离。）

## 命令

| 命令 | 作用 |
|---|---|
| `build --root <dir> --out graph.json` | 扫描目录，生成信息关联关系图骨架 |
| `validate --graph graph.json` | 运行 GR-E1~E8 校验，非零退出=不通过（CI 门禁） |
| `export --graph graph.json --format mermaid` | 导出 Mermaid 供文档/画布 |
| `query --graph graph.json [--kind CodeFile] [--name foo]` | 检索节点与关联子图（影响面分析） |
| `snapshot --graph graph.json --out ids.txt` | 导出 id 快照 |
| `sync --old a.txt --new b.txt` | 比对两次快照漂移，有差异则 exit 1（阻断未同步提交） |

## 节点类型（12 类）

`Business / Data / Function / Interface / CodeFile / Script / ScheduleTask / Config / Dependency / ThirdParty / Doc / Runtime`
当前扫描器覆盖：`CodeFile / Script / Config / Doc / Data(SQL CREATE TABLE) / ScheduleTask(含 cron/spawn 特征) / Dependency(import·use·require·mod·Cargo 依赖)`。

## 边类型（7 类）

`Call / ReadWrite / Reference / Dependency / Inheritance / ConfigRef / Deploy`
当前扫描器覆盖：`Reference(import/use/mod) / Dependency(Cargo 依赖·SQL 外键) / ReadWrite(SQL 建表)`。

## 在本工程已生成骨架

- `graph.json`：352 节点 / 730 边（12 crate 代码、配置、文档、SQL、外部依赖全覆盖）
- `graph.mmd`：Mermaid 渲染文件
- `validate` 当前输出 39 项 GR-E5 信息孤岛（文档/配置未被任何代码引用），即规范要消灭的「信息孤岛」，需逐步补链接或确认基线。

## 已知边界（扫描器为静态启发式）

- 仅解析 `use/import/require/mod/#include` 与 `mod` 声明、`Cargo.toml` 依赖、`SQL` 表与外键；函数级 `Call`、接口 `Interface`、运行态 `Runtime` 需后续接入 AST/运行时采集。
- workspace 内部 crate 已做 连字符↔下划线 归一并链接到 `src/lib.rs`；其余无法解析的依赖建模为 `external` 的 `Dependency` 节点。
- 仅建模「可引用、可依赖」的信息，构建日志等噪声已排除。

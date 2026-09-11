# info-graph —— 关图规范（GR-STD-V1.0）参考实现

纯 Rust std、零外部依赖，可离线编译。对「信息关联关系图开发规范」的最小可运行工具：
扫描工程 → 一切信息抽象为节点、依赖/交互抽象为边 → 校验 → 导出 Mermaid → 同步比对（CI 门禁）。

## 构建

```bash
cd tools/info-graph
cargo build --release        # 无需网络
# 产物：target/release/info-graph（Windows 为 info-graph.exe）
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
| `dedup --graph graph.json --spec req.json [--fail-on-new]` | **需求判重（P9 先判重后立项）**：以关图为能力指纹库，子图匹配新需求候选能力/边，输出 `reuse`/`incremental`/`new` 判定与相似度；`--fail-on-new` 时未命中即 exit 1（CI 阻断重复造系统） |
| `skeleton --graph graph.json --spec guantu.req.json --out graph.enterprise.json` | 注入 REQ 需求根 + 六维绑定骨架（Bind 边） |
| `deviate --graph graph.enterprise.json` | REQ 根可达性 / 需求对齐偏离检测（GR-E6） |

## 节点类型（12 类）

`Business / Data / Function / Interface / CodeFile / Script / ScheduleTask / Config / Dependency / ThirdParty / Doc / Runtime`
当前扫描器覆盖：`CodeFile / Script / Config / Doc / Data(SQL CREATE TABLE) / ScheduleTask(含 cron/spawn 特征) / Dependency(import·use·require·mod·Cargo 依赖)`。

## 边类型（7 类）

`Call / ReadWrite / Reference / Dependency / Inheritance / ConfigRef / Deploy`
当前扫描器覆盖：`Reference(import/use/mod) / Dependency(Cargo 依赖·SQL 外键) / ReadWrite(SQL 建表)`。

## 需求判重工作流（P9 先判重后立项）

新需求不得直接开工，必须先在关图判重，从机制上杜绝重复造系统：

```bash
# 1) 重建关图并注入 REQ 骨架
info-graph build    --root . --out graph.json
info-graph skeleton --graph graph.json --spec docs/graph/guantu.req.json \
                    --out graph.enterprise.json
# 2) 判重（未命中即阻断，强制人工确认是否确有必要立项）
info-graph dedup    --graph graph.enterprise.json \
                    --spec docs/graph/requests/<新需求>.json --fail-on-new
```

| 判定 | 条件 | 应采取的动作 |
|---|---|---|
| `reuse` | 候选能力节点与关系边全部已存在 | 直接编排现有能力，**不写新代码** |
| `incremental` | 部分能力已存在（或能力齐备但连接方式不同） | 在既有子图上局部扩展 |
| `new` | 无任何对应能力（`--fail-on-new` 时 exit 1） | 确认确有必要后才新立项 |

规格格式与示例见 `docs/graph/requests/README.md`。CI 侧由 `tools/guantu_gate.py` step6 自动逐条执行。

## 在本工程的当前实测（2026-08-18）

- `build --root .`：**1251 节点 / 1019 边**（15 crate 代码、前后端、配置、文档、SQL、外部依赖全覆盖）
- `skeleton`：注入 **22 个 REQ 根 / 75 条 Bind 绑定边**
- `deviate`：需求对齐覆盖率 **98.0%**（门禁基线已棘轮至该值，且已知问题清零）
- `validate`：GR-E5 信息孤岛（文档/配置未被任何代码引用）仍在逐步补链，由基线机制管控不回退
- 自带单测 **8 个**（判重三类判定 + JSON 中文/`\uXXXX` 代理对/BOM 回归）：`cargo test`

## 稳健性说明（历史真实缺陷已修复并加回归测试）

- **中文不再乱码**：字符串解析按原始字节累积后统一 UTF-8 解码（旧实现 `byte as char` 会把多字节序列拆成 Latin-1 码位）。
- **完整 `\uXXXX` 支持**：含 UTF-16 代理对（emoji / 补充平面）。
- **容忍 UTF-8 BOM**：Windows 编辑器写入的 BOM 曾导致 `skeleton` 解析失败并**静默注入 0 个 REQ**，下游门禁误判"通过"。现自动剥离 BOM；且 `skeleton` 解析到 0 个需求根时以 exit 2 **响亮失败**，禁止静默空跑。

## 已知边界（扫描器为静态启发式）

- 仅解析 `use/import/require/mod/#include` 与 `mod` 声明、`Cargo.toml` 依赖、`SQL` 表与外键；函数级 `Call`、接口 `Interface`、运行态 `Runtime` 需后续接入 AST/运行时采集。
- workspace 内部 crate 已做 连字符↔下划线 归一并链接到 `src/lib.rs`；其余无法解析的依赖建模为 `external` 的 `Dependency` 节点。
- 仅建模「可引用、可依赖」的信息，构建日志等噪声已排除。

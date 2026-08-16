# PrimiFlow 三层递进开发验证总结（层2 需求解析 / 层6 真实持久化 / 层3 API 服务）

> 日期：2026-08-16
> 工程：`operator-unified-system/crates/primiflow`
> 目标：在既有 `crates/primiflow`（已具备 `assoc.rs` 六维溯源图谱 + `generate.rs` 骨架/文档生成 + `runner.rs` 企业级闭环）之上，按顺序递进补齐 **层2 需求解析层 → 层6 真实持久化层 → 层3 API 服务层**，每步可编译、可测试、可运行。

---

## 1. 交付清单

| 模块 | 文件 | 类型 | 职责 |
|---|---|---|---|
| 层2 需求解析层 | `src/parse.rs` | 新增 | 自然语言 → 结构化需求树 → `Spec`（`run_pipeline` 直喂） |
| 层6 真实持久化层 | `src/persistence.rs` | 新增 | 资产库 / 知识图谱 / 六维溯源图谱的 Memory 与 SQLite 双后端真实存储 |
| 层3 API 服务层 | `src/server.rs` | 新增 | axum REST 服务，串起 parse→runner→persistence 全闭环 |
| 示例 | `examples/server_demo.rs` | 新增 | 单机 `0.0.0.0:3000` 监听，打印 API 契约 |
| 集成测试 | `tests/api_server.rs` | 新增 | 5 个 L5 HTTP 端到端用例 |
| 装配 | `src/lib.rs` / `src/runner.rs` / `Cargo.toml` | 修改 | 注册新模块；`PipelineReport`/`Step` 加 `Serialize`；加 rusqlite/axum/tokio 依赖与 dev-dep reqwest |

---

## 2. 层2 需求解析层（`src/parse.rs`）

**核心类型**
- `Category`：Fetch / Compute / Llm / Database / Shell，含 `key()`、`tool()`、`default_ms()`（为后续 κ 复用同源同键做准备）。
- `ParsedRequirement`：raw、goal、roles、inputs、outputs、rules、constraints、schedule、external_systems、subtasks、policy。
- `ParsedSubtask`：label、categories、tool。
- `ProjectRecord` 不在此层（属于持久化层）。

**核心函数**
- `pub fn parse(text: &str) -> ParsedRequirement`
- `pub fn parse_to_spec(text: &str) -> Spec`
- `ParsedRequirement::to_spec()`：子任务映射到 `Spec`，同类别复用同一 key（命中 κ‑τ 引擎复用机制）。

**解析启发式**
- `classify()`：从句可命中**多个**类别（如"清洗对账后生成图表报告" → Database+Compute+Llm 多个子任务），故返回 `Vec<Category>`。
- `detect_schedule()`：识别定时/周期调度。
- `detect_policy()`：Urgent / Exploratory / Balanced。
- `extract_list()` / `split_clauses()` / `normalize_label()` / `slug()`。

**测试**：5 个单元测试全绿。

---

## 3. 层6 真实持久化层（`src/persistence.rs`）

**双后端枚举**
```rust
pub enum Persistence {
    Memory { assets, kb_graph_json, trace_graph_json, projects },
    Sqlite { conn },
}
```
> 关键修正：Memory 变体最初用**单一** `graph_json` 字段同时存「知识图谱（TopologyGraph JSON）」与「六维溯源图谱（AssocGraph JSON）」，导致 `save_graph` 覆盖 KB JSON 而反序列化失败（"missing field entities"）。拆分为 `kb_graph_json` / `trace_graph_json` 两个独立字段后修复。

**核心结构**
- `ProjectRecord`：`from_report()` 由 `PipelineReport` 构造，含 id / name / policy / κ / τ / conserved / acyclic / reused / regularized / q_before / q_after / bound_nodes / bound_edges / created_at。

**方法**
- `memory()`、`sqlite(path)`、`sqlite_memory()`
- `save_kb` / `load_kb`（`TopologyGraph` 精确存取）
- `save_graph` / `load_graph`（`AssocGraph` 精确存取）
- `save_project` / `list_projects`
- `persist_pipeline(&mut self, engine, master, project_id, rep)`：闭环结果真实落库
- `replay_into(&mut self, engine, master)`：从存储回灌引擎重演

**测试**：4 个集成用例全绿 —— `sqlite_roundtrip_kb_and_graph`、`memory_store_is_functional`、`sqlite_memory_roundtrip`、`file_persistence_survives_reopen`（落盘后重开仍可读取）。

---

## 4. 层3 API 服务层（`src/server.rs`）

**状态机**
```rust
pub struct AppState {
    engine: Mutex<PrimiEngine>,
    master: Mutex<AssocGraph>,
    store: Mutex<Persistence>,
    out_dir: PathBuf,
    topologies: Mutex<HashMap<String,String>>,   // project_id -> mermaid
    last_input: Mutex<HashMap<String,String>>,
}
```

**主链路**
```rust
async fn run_requirement(state, description) -> Result<(String, PipelineReport), String>
```
`parse` → `to_spec` → `run_pipeline` → `persist_pipeline` → 读取拓扑 Mermaid。

**REST 契约（与 `gen/c5.rs` 对齐）**
| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/api/projects` | 提交需求，跑 κτ 闭环 |
| POST | `/api/projects/:id/messages` | 追加需求描述 |
| GET | `/api/topologies/:id` | 查询拓扑 Mermaid |
| POST | `/api/topologies/:id/regularize` | 重跑 κτ 自涌现 |
| POST | `/api/topologies/:id/freeze` | 冻结资产到知识库 |
| GET | `/api/assets?q=&domain=` | 检索知识库资产 |

**导出**
- `build_router(state) -> Router`
- `serve(state, addr)`（阻塞）
- `spawn_serve(state, addr) -> Result<SocketAddr>`（非阻塞，供测试）
- `pub const API_CONTRACT`

---

## 5. 验证结果（本轮）

| 项 | 命令 | 结果 |
|---|---|---|
| 静态检查 | `cargo clippy -p primiflow --all-targets` | 仅 2 个非阻断告警（`new_without_default`，位于 `gen/c1.rs` Orchestrator 与 `gen/c7.rs` CanvasState，属外部生成的骨架，非本层代码） |
| 企业级端到端 | `cargo run -p primiflow --example enterprise_demo` | 全绿：15 代码骨架 + 4 DAG + 溯源矩阵 + DDL + 资产固化 Q=5.60 |
| HTTP 服务 | `cargo run -p primiflow --example server_demo` | 监听 `0.0.0.0:3000`，打印 API 契约，正常接受请求 |
| 单元测试 | `cargo test -p primiflow --lib` | **46 passed** |
| API 集成测试 | `tests/api_server.rs` | **5 passed**（L5：建项目跑全环 / 取拓扑 / 冻结增量资产 / 资产检索过滤 / 正则化重跑） |
| 企业级集成测试 | `tests/enterprise_validation.rs` | **8 passed** |
| **合计** | | **59 passed / 0 failed** |

---

## 6. 关键技术决策与坑位

1. **持久化双 JSON 字段**：`TopologyGraph`（KB）与 `AssocGraph`（溯源）结构不同，必须分字段存储，否则 serde 反序列化报 "missing field entities"。
2. **`save_*` 需 `&mut self`**：Memory 变体要写回内部 `Vec`/`String` 字段，故所有写方法取可变借用。
3. **`ResourceBudget` 构造**：用 `ResourceBudget::default()`（字段为 `total_ms` + `per_pool: HashMap`），避免沿用旧字段名 `max_total_ms`/`max_parallel` 报错。
4. **路径处理**：`dir.as_path()` 替代 `Path::new(&dir.to_string_lossy())` 规避 `Cow<OsStr>` 借用问题。
5. **测试路径隔离**：SQLite 文件与 `run_all` 输出目录必须不同路径，否则 Windows "os error 183 文件已存在"。
6. **API 测试改用真实 HTTP**：弃用 `tower::util::ServiceExt::oneshot`（不在依赖内），改用 `reqwest` + `spawn_serve(127.0.0.1:0)` 发真实请求。
7. **CLAUDE 瞬时态误报**：曾因后台进程改写源文件 + cargo 增量缓存读到陈旧版 parse/persistence 报假错，`cargo clean -p primiflow` 后磁盘实际正确。外部进程持续改写 `gen/*`，以"可编译可运行"为底线，不侵入式改造生成骨架。

---

## 7. 结论

层2 → 层6 → 层3 三层递进全部交付，遵循用户「**开发好、测试验证好、一定要可以运行**」的口径：
- 需求解析可把自然语言转成引擎可消费的结构化 `Spec`；
- 持久化层对资产库 / 知识图谱 / 六维溯源图谱实现 Memory + SQLite 真实落盘与回灌；
- API 服务层把三者串成可对外调用的 REST 闭环，单机即可 `0.0.0.0:3000` 起服务。

全部 59 项测试 0 失败，clippy 无本层新增告警，两个示例均可运行。

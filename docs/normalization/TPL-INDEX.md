# TPL-INDEX · 模块模板清单归一化（TPL-xx）

> 编号：**DOC-NORM-TPL-V1.0** · 归属：[README.md](README.md)（SSoT 枢纽）
> 内容：mox 代码生成模板目录——把"建系统"降级为"选模板 + 配参数 + AI 生成"。

---

## 1. 模板目录（业务形态 → 生成产物）

| 编号 | 业务形态 | 生成产物 | 对齐现有模块 | 成熟度 |
|------|----------|----------|--------------|:--:|
| `TPL-01` | 单表 CRUD | DDL / Rust 模型 / TS API / 列表页 / 表单页 / 路由 / 菜单 | `ProjectsView` / `ResourcesView` | ✅ codegen 已实现 |
| `TPL-02` | 树表 | DDL(parent_id) / Rust 树模型 / 树 API / 拖拽树页 / 路由 / 菜单 | `GraphView` 节点树 / 资源管理 | ✅ codegen 已实现 |
| `TPL-03` | 主子表 | 双 DDL(REFERENCES) / 嵌套 Rust 模型 / API / 主子页 / 路由 / 菜单 | `TaskView`（任务+子步骤）/ 工作流 | ✅ codegen 已实现 |
| `TPL-04` | 图谱实体 | 节点/边双 DDL / Rust 模型 / 图 API / 3D 画布 / graph.mmd / 路由 / 菜单 | `GraphView` / `MoxFusionView` | ✅ codegen 已实现 |
| `TPL-05` | 工作流 | workflow.json DAG 定义 / TS Runner / 路由 / 菜单 | `WorkflowView` / `primiflow` | ✅ codegen 已实现 |
| `TPL-06` | AI 对话域 | Chat 页 / 会话 API / 路由 / 菜单 | `ChatView` / `CaomeiView` | ✅ codegen 已实现 |

---

## 2. 生成引擎（事实来源）

- **`meta` 模块 `codegen` 能力**：声明于 `docs/database/mox_sys/module-registry.yml`（capabilities: catalog, metadata, lowcode, **codegen**）。**状态：✅ 已实现（2026-09-04）**——落点 `platform/domains/platform/core/mox-platform-meta-core/src/codegen/`（mod 分派 + naming 归一化 + tpl_crud 模板）；TPL-01 单表 CRUD 已可从 `EntityWithFields` 元数据确定性产出 7 类工件（DDL / Rust 模型 / TS API / 列表页 / 表单页 / 路由 / 菜单），同输入字节级一致，12 项单测全绿，clippy 零告警；TPL-02~06 按"新增 `tpl_*` 模块 + 分派 match 追加 1 个 arm"扩展。
- **`mox-flow-primiflow-svc`**：真实身份是 **DAG 流程编排执行引擎**（engine/scheduler/executor/dag 四件套，Server `:8787`），非代码生成器。负责出码产物的流程执行与编排。
- **primiflow 示例产物**：`examples/out/*`（mod.rs / schema.rs / ddl.sql / graph.mmd / topo）为种子示例，证明"DDL→代码→图谱"链路可行。

---

## 3. 模板扩展机制

- 新增一类形态：新增 `gen/tpl-XX.rs` + 分派 match 追加 1 个 arm。
- TDD：先 RED 后 GREEN，产物字节级对比。
- 出码必经 `verify` + 治理闸门（见 `VAL-INDEX`）。

---

## 4. 一键生成管线（manifest 驱动）

```
登记 Manifest → 选模板(TPL) → 装插件/领域包 → 一键生成
  → ⛨verify+闸门 → 完成页面 → 发布/入图/入 KB
```

输入 `module-manifest.json`（SSoT 对齐 `frontend-ui/src/MODULE-MANIFEST.md`），输出成型项目（代码+页面+路由+菜单+文档）。

---

## 5. 登记规则

- 模板文档 `TPL-{两位序号}-{中文短名}.md` 放 `docs/normalization/template/`，须含产物清单与对齐模块。
- 跨文档引用 `docs/normalization/TPL-INDEX.md#章节`。
- 命名 `{前缀}-{两位序号}-{中文短名}.md`，与 DOC-GOV-V1.0 一致。

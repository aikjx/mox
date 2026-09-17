# MOX PR 类型门禁规范（PR-TYPES v1.0）

> **版本**：v1.0 · **生效**：2026-09-17 · **性质**：L5 标准（规范，非权威事实源）
> **来源**：对标 RustFS `docs/architecture/crate-boundaries.md`（PR 10 类门禁）落地的工程治理规范；
> 配套：`AGENTS.md`（仓库维护约定）、`docs/architecture/NORMALIZED_ARCHITECTURE.md`（结构权威）
> **目标**：一次 PR 只做一件事；架构迁移类改动可审查、可回滚、可追溯，防止"目录移动 + 安全收紧 + 行为变更"混提。

---

## 1. 为什么需要 PR 类型门禁

本仓库 143 crate / 12 域 / 六层架构下，跨域、跨层、跨文档的混提是文档漂移与回归的主要来源。
声明 PR 类型后：

- 评审者一眼可知改动意图与风险面；
- 架构迁移（拆分/移动/解耦）与功能开发隔离，迁移 PR 可单独回滚；
- 兼容性/安全类改动不夹带在其他 PR 中静默发生。

## 2. PR 类型定义（一次 PR 必须且只能声明一种）

| # | 类型 | 含义 | 典型内容（MOX 语境） |
|---|------|------|----------------------|
| 1 | `docs-only` | 仅文档改动 | 更新 `docs/architecture/`、`API-REGISTRY.md`、工作报告 |
| 2 | `test-only` | 仅测试改动 | 补单测/集成测试，不改生产代码（除测试夹具） |
| 3 | `contract` | 契约层改动 | `mox-<域>-api` DTO、`proto` gRPC、`mox-cloud-domain-traits` trait 变更 |
| 4 | `api-extraction` | 从实现中提取 API/契约 | 新建契约 crate、把 `*_boundary.rs` 边界外提 |
| 5 | `pure-move` | 纯代码搬迁（不改行为） | 模块/文件移动、目录重组、crate 改名（编译等价） |
| 6 | `consumer-migration` | 迁移消费方到新契约 | 调用方从旧 API 切到新 API，兼容 shim 保留 |
| 7 | `dependency-migration` | 依赖关系调整 | 增删 crate 依赖、解除跨域直连、feature 门控调整 |
| 8 | `security-change` | 安全相关改动 | 鉴权、密钥、审计、TLS、输入校验收紧 |
| 9 | `behavior-change` | 行为变更 | 路由语义、数据面行为、队列/调度逻辑修改 |
| 10 | `ci-gate` | 仅 CI/门禁改动 | 工作流、`verify-ports.py`、`gen-api-registry.py`、lint 配置 |

## 3. 强制规则

1. **单一类型**：一个 PR 只能声明一种类型；跨类型的改动拆成多个 PR。
2. **禁止组合**（以下组合必须拆开）：
   - `pure-move` + `behavior-change`（搬迁时不得顺手改行为）；
   - `security-change` + `behavior-change`；
   - `docs-only` + 生产代码改动；
   - `contract` + `consumer-migration`（契约变更与消费方迁移分 PR，保持每个 PR 可独立通过）。
3. **兼容性红线**（架构迁移类 PR 生效）：
   - 迁移期间不得改变对象放置、quorum、读语义、生命周期/复制队列、审计/通知事件、scanner 修复行为；
   - 临时兼容代码必须带 `MOX_COMPAT_TODO(ARC-xxx): 原因 + 移除条件` 标记，并在 `docs/working-reports/_norm_research/compat-cleanup-register.md` 登记；
   - 兼容层删除必须走独立 cleanup PR。
4. **声明方式**：PR 标题前缀 `type(scope): 描述`（沿用 conventional commits），如 `contract(cloud): 新增 RustFsEcstore 契约`。

## 4. 架构迁移类 PR 评审 checklist（三视角）

架构迁移类 PR（`contract` / `api-extraction` / `pure-move` / `consumer-migration` / `dependency-migration` / `ci-gate`）推送前按三视角检查：

| 视角 | 必查项 |
|---|---|
| **结构/质量** | 类型声明正确；依赖方向单向（`foundation→api→proto→core→svc→gateway`）；命名符合 `mox-<域>-<层>-<角色>`；无过度抽象；改动范围与声明一致 |
| **迁移保持** | 启动顺序/readiness/quorum/读语义不变；全局状态走 AppContext/runtime_sources 边界；跨域访问经 SDK/事件/平台编排；兼容 shim 有登记 |
| **测试/验证** | 聚焦测试先行；回归测试通过；`cargo check -p <受影响 crate>`；相关门禁（`verify-ports.py` / `gen-api-registry.py` diff / `check-doc-links.py`）通过 |

任一必查项不满足 → 打回修正，不允许带 blocker 合并。

## 5. 与 CI 的关系

- `ci.yml` `architecture` 作业已包含：端口契约校验、**API 注册表重生成 + diff 门禁**；
- `gen-api-registry.py` 每次改动 `actuator.rs`/`routes.rs` 后必须重跑（CI 强制）；
- 本规范本身为人工评审依据，后续可按需加自动化（PR 标题类型解析）。

---

*L5 标准 · 2026-09-17 · 对标 RustFS crate-boundaries.md 落地。*

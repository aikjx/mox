# 规格层入口 — Specifications

> **层定位**：L6 企业级与规格层（规格半边）。存放**领域规范**与**规格驱动任务包**（spec / tasks / review 三件套）。
> 上层入口：[文档中心](../README.md) · 结构规范：[ARCHITECTURE-OF-DOCS.md](../ARCHITECTURE-OF-DOCS.md) · 企业级半边：[`../enterprise/00-INDEX.md`](../enterprise/00-INDEX.md)

---

## 一、领域规范（🟢 权威）

| 文档 | 说明 |
|------|------|
| [`GR-STD-信息关联关系图开发规范-V1.0.md`](./GR-STD-信息关联关系图开发规范-V1.0.md) | 信息关联关系图（关图）开发规范 |
| [`PT-Primi-架构规范-V1.0-完整版.md`](./PT-Primi-架构规范-V1.0-完整版.md) | Primi 架构规范完整版 |
| [`OUS-业务功能规划与架构数据关系分析.md`](./OUS-业务功能规划与架构数据关系分析.md) | 业务功能规划与架构数据关系分析 |

> 关图的**机器产物**在 [`../architecture/graph/`](../architecture/graph/)；关图**需求基线**在 [`../architecture/full-dimensional/`](../architecture/full-dimensional/)。

## 二、规格驱动任务包（`tasks/`）

命名：`tasks/<YYYYMMDD>-<slug>/{spec.md, tasks.md, review.md, ...}`，日期为立项目期，slug 为任务短名。

| 索引 | 说明 |
|------|------|
| [`tasks/README.md`](./tasks/README.md) | **任务包总索引**（含状态与产出物） |

任务包内部约定：

| 文件 | 作用 |
|------|------|
| `spec.md` | 需求与验收标准（要做什么、完成定义） |
| `tasks.md` | 拆解后的可执行清单（含状态勾选） |
| `review.md` | 复盘、偏差与遗留项 |
| `*.csv` / `*.json` / `*.js` / `*.ps1` | 任务配套数据、脚本与校验产物（🟡 过程物） |

## 三、新增规格怎么提

1. **任务包**：`tasks/` 下建目录，先写 `spec.md`（验收标准可判定），再写 `tasks.md`。
2. **领域规范**：编号 `<域]-<类型>-<序号>`（如 `GR-STD-`、`PT-Primi-`），正文含适用范围/约束/校验方式。
3. 任务闭环后：把结论沉淀到对应层权威文档（L2~L5），`review.md` 记录遗留项；条目登记到 [`tasks/README.md`](./tasks/README.md)。
4. 长期有效的规范从任务包**晋升**为领域规范或 [`../standards/`](../standards/README.md) 标准，避免任务包被长期引用。

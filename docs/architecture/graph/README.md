# 信息关联关系图（关图） — Graph Artifacts

> **层定位**：L2 架构层 → 关图子域。存放**关图的机器产物与需求基线**（🟡 产物/基线）。
> 规范正文：[`../../specifications/GR-STD-信息关联关系图开发规范-V1.0.md`](../../specifications/GR-STD-信息关联关系图开发规范-V1.0.md)
> 上层入口：[架构层入口](../README.md) · [文档中心](../../README.md)

---

## 一、本目录内容

| 文件/目录 | 类型 | 说明 |
|-----------|------|------|
| [`graph.mmd`](./graph.mmd) | 🟡 产物 | 关系图 Mermaid 源（可渲染为文档内嵌图） |
| [`guantu.req.json`](./guantu.req.json) | 🟡 基线 | 关图需求基线（结构化） |
| [`requests/`](./requests/README.md) | 🟡 基线 | 分需求条目（如 `REQ-2026-001-治理台指标看板.json`） |

## 二、约定

1. **产物不入权威链**：关图的权威定义在 `GR-STD` 规范与 L4 数据文档；本目录为可复算产物。
2. **机器可读优先**：新增关系图以 `.mmd` / `.json` 落盘，不用截图代替。
3. 全维 TraceMatrix（六维追溯）见 [`../full-dimensional/00-README.md`](../full-dimensional/00-README.md)，两者互补：本目录=关系图，full-dimensional=六维追溯矩阵。
4. 生成脚本若存在，须在 [`../14-REPOSITORY-FULL-MAP.md`](../14-REPOSITORY-FULL-MAP.md) 或 `scripts/` 中登记，保证可复现。

<a id="lowcode-directory-design"></a>
## 目录架构设计卡：低代码与动态配置（2026-10-01）

> 目标设计接缝；既有正文按原日期/类型解释，未实施能力不标已完成。

| 设计项 | 本目录约定 |
|---|---|
| 输入 | 需求/配置/模块/测试/制品来源及稳定ID |
| 处理与边界 | 建立来源明确的关系；交易状态由owner维护，图谱为可重建投影 |
| 输出 | 可溯源关联图与来源/版本查询 |
| 维护角色 | 图谱模型owner（角色建议，未指派个人） |
| 配置语义 | [统一规范](docs/standards/lowcode-dynamic-configuration.md#model)，本目录不复制覆盖/生命周期规则 |
| 本目录设计 | [详细接缝](docs/standards/lowcode-dynamic-configuration.md#dimensions) |
| 验收 | 类型/依赖/权限/版本/异常/恢复按相关LC-Q条目补证；设计完成与运行验证分开 |

全目录关系见 [目录矩阵](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories)。新增架构文档应符合 [文档设计契约](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#document-contract)，各主题拥有自己的事实主源。

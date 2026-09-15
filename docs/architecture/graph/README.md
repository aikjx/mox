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

# 验证证据 — Verification Evidence

> **层定位**：L7 报告与验证层 → 验证证据子目录（🟡 证据，非权威）。
> 由 `docs/enterprise-verification/` 收敛而来（2026-09-13，见 [ARCHITECTURE-OF-DOCS.md §6.1](../../ARCHITECTURE-OF-DOCS.md)）。
> 上层入口：[工作报告索引](../README.md) · [文档中心](../../README.md)

---

## 一、内容清单

| 文件 | 类型 | 说明 |
|------|------|------|
| [`verification-report.html`](./verification-report.html) | 🌐 报告 | 验证汇总报告（可读版） |
| [`full-test-output.txt`](./full-test-output.txt) | 📄 原始输出 | 全量测试 stdout/stderr |
| [`quickstart-output.txt`](./quickstart-output.txt) | 📄 原始输出 | 快速开始（quickstart）实跑输出 |
| [`test-algo.txt`](./test-algo.txt) | 📄 原始输出 | algorithm 域测试输出 |
| [`test-core.txt`](./test-core.txt) | 📄 原始输出 | core 域测试输出 |
| [`test-operator.txt`](./test-operator.txt) | 📄 原始输出 | operator 域测试输出 |
| [`test-unified-platform.txt`](./test-unified-platform.txt) | 📄 原始输出 | 统一平台测试输出 |

## 二、使用约定

1. 本目录**只存证据**：原始命令输出不加工、不美化，保留时间与上下文。
2. 引用方式：`docs/working-reports/verification/<file>`，必须注明"🟡 证据"。
3. 结论要进权威文档时，回填到 L2 架构层或 L6 企业级层，并在结论处标注证据文件名。
4. 新增证据：文件名 `<域>-<类型>-<YYYYMMDD>.txt|html`，并在上表登记。

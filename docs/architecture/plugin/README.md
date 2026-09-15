# 插件架构（Plugin Runtime）

> **层定位**：L2 架构层 → 插件子域。平台**双运行时混合架构**（原生 WASM 插件 + VSCode 兼容扩展）。
> 上层入口：[架构层入口](../README.md) · [文档中心](../../README.md)

---

## 一、本目录文档

| 文档 | 说明 |
|------|------|
| [`PLUGIN-ARCHITECTURE.md`](./PLUGIN-ARCHITECTURE.md) | 统一插件架构：通过统一的 `Runtime` trait 抽象实现生命周期、权限、能力系统的归一化管理 |
| [`VSCODE-COMPATIBILITY.md`](./VSCODE-COMPATIBILITY.md) | VSCode 插件（VSIX）兼容性：WASM 运行时（高性能安全沙箱）与 VSCode 兼容运行时的分工 |
| [`VSCODE-API-STATUS.md`](./VSCODE-API-STATUS.md) | VSCode Extension API 实现状态表（阶段 2 核心子集 → 阶段 3 完善） |

## 二、架构要点速览

```
插件宿主
├─ WASM Runtime      → 原生 MOX 插件：强隔离 / 高性能 / 能力白名单
└─ VSCode Runtime    → VSIX 扩展：复用既有生态（API 子集，见 VSCODE-API-STATUS）
统一抽象：Runtime trait（生命周期 + 权限 + 能力注册）
```

## 三、相关文档

- 扩展开发指南：[`../02-extension-guide.md`](../02-extension-guide.md)（实现 Trait → Factory → Registry → 配置 → 自动组装）
- 引擎插件化：[`../../standards/engine-kernel.md`](../../standards/engine-kernel.md)
- 引擎图谱：[`../../standards/engine-universe.md`](../../standards/engine-universe.md)

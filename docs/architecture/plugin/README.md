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

<a id="lowcode-directory-design"></a>
## 目录架构设计卡：低代码与动态配置（2026-10-01）

> 目标设计接缝；既有正文按原日期/类型解释，未实施能力不标已完成。

| 设计项 | 本目录约定 |
|---|---|
| 输入 | 版本化插件/工具声明与宿主能力白名单 |
| 处理与边界 | 验证契约、权限、允许网络范围、预算、启停与升级；禁止配置加载任意代码 |
| 输出 | 受控注册能力与禁用/兼容证据 |
| 维护角色 | 扩展协议owner（角色建议，未指派个人） |
| 配置语义 | [统一规范](docs/standards/lowcode-dynamic-configuration.md#model)，本目录不复制覆盖/生命周期规则 |
| 本目录设计 | [详细接缝](docs/standards/lowcode-dynamic-configuration.md#dimensions) |
| 验收 | 类型/依赖/权限/版本/异常/恢复按相关LC-Q条目补证；设计完成与运行验证分开 |

全目录关系见 [目录矩阵](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories)。新增架构文档应符合 [文档设计契约](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#document-contract)，各主题拥有自己的事实主源。

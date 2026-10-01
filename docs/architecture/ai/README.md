# AI 统一智能系统架构（AI Architecture）

> **层定位**：L2 架构层 → AI 子域。AI 统一智能系统的架构事实与可视化。
> 上层入口：[架构层入口](../README.md) · [文档中心](../../README.md)

---

## 一、本目录内容

| 文件 | 类型 | 说明 |
|------|------|------|
| [`ai-unified-intelligent-system-architecture.html`](./ai-unified-intelligent-system-architecture.html) | 🌐 可视化 | AI 统一智能系统架构（对话中心 / Agent 运行时 / 能力编排） |
| [`agentic_loop_minimal.py`](./agentic_loop_minimal.py) | 🟡 示例脚本 | 最小 Agentic Loop 参考实现（教学/验证用途，非生产代码） |

## 二、相关文档

| 主题 | 位置 |
|------|------|
| AI 引擎深度分析 | [`../../modules/ai-engine-master-analysis.md`](../../modules/ai-engine-master-analysis.md) |
| AI 原生架构规范 | [`../../standards/ai-native-architecture-standard.md`](../../standards/ai-native-architecture-standard.md)（AINA-STD-001） |
| AI 统一优化计划 | [`../AI-UNIFIED-OPTIMIZATION-PLAN.md`](../AI-UNIFIED-OPTIMIZATION-PLAN.md) |
| 专家联盟（AI 编排主场景） | [`../../expert-alliance/INDEX.md`](../../expert-alliance/INDEX.md) |

<a id="lowcode-directory-design"></a>
## 目录架构设计卡：低代码与动态配置（2026-10-01）

> 目标设计接缝；既有正文按原日期/类型解释，未实施能力不标已完成。

| 设计项 | 本目录约定 |
|---|---|
| 输入 | 模型/提示词/工具引用、领域输入与预算政策 |
| 处理与边界 | 单Agent和provider适配负责调用；联盟负责多专家编排；不混用业务权限与模型建议 |
| 输出 | 类型化能力、质量/成本/时延证据和错误 |
| 维护角色 | ai模型与工具owner（角色建议，未指派个人） |
| 配置语义 | [统一规范](docs/standards/lowcode-dynamic-configuration.md#model)，本目录不复制覆盖/生命周期规则 |
| 本目录设计 | [详细接缝](docs/standards/lowcode-dynamic-configuration.md#dimensions) |
| 验收 | 类型/依赖/权限/版本/异常/恢复按相关LC-Q条目补证；设计完成与运行验证分开 |

全目录关系见 [目录矩阵](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories)。新增架构文档应符合 [文档设计契约](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#document-contract)，各主题拥有自己的事实主源。

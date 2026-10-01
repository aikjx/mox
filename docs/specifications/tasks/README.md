# 任务规格文档

本目录存放各阶段任务的规格说明（spec）、任务清单（tasks）和评审记录（review）。

## 目录结构

每个任务规格目录包含：
- `spec.md` — 任务规格说明
- `tasks.md` — 任务分解清单
- `review.md` — 评审记录（如有）
- 其他附属文件（测试数据、脚本等）

## 任务索引

按日期排列的任务规格：

| 日期 | 任务代号 | 说明 |
|------|----------|------|
| 2026-08-23 | enterprise-10task-scoring-checklist | 企业级10任务评分清单 |
| 2026-08-23 | enterprise-compare-top-oss-ai-products-optimize | 企业级对比顶级OSS AI产品优化 |
| 2026-08-23 | enterprise-ready-build-verify | 企业级就绪构建验证 |
| 2026-08-23 | enterprise-real-code-normalize-and-verify | 企业级真实代码归一化与验证 |
| 2026-08-23 | mox-cloud-graph-standardized-selfdev | MOX云图标准化自研 |
| 2026-08-23 | mox-full-enterprise-architecture | MOX全企业级架构 |
| 2026-08-23 | mox-storage-distributed-ai-unified-query | MOX存储分布式AI统一查询 |
| 2026-08-23 | mox-top-master-doc-restructure | MOX顶级主文档重构 |
| 2026-08-23 | rust-ais-normalization-full-develop | Rust AIS归一化全开发 |
| 2026-08-23 | t4-dependency-governance-workspace | T4依赖治理工作空间 |
| 2026-08-24 | ais-grade-fusion | AIS等级融合 |
| 2026-08-24 | t10-t11-t17-cloud-graph-sdk-ef-ops | T10/T11/T17云图SDK EF操作 |
| 2026-08-24 | t17-sdk-ef-ops | T17 SDK EF操作 |
| 2026-08-24 | t4-cloud-drive-m1-acceptance | T4云盘M1验收 |
| 2026-08-24 | v2.1-t22-t23-t24-t25-simd-graph-gm-glacier | V2.1 SIMD图GM冰川 |
| 2026-08-25 | ai-chat-actions-full-layout | AI对话动作全布局 |
| 2026-08-25 | mox-all-core-rust-max-algo | MOX全核心Rust最大算法 |
| 2026-08-25 | rust-expert-alliance-platform | Rust专家联盟平台 |
| 2026-08-25 | xiaobai-voice-integration | 小白语音集成 |
| 2026-08-26 | xiaobai-mox-full-arch | 小白MOX全架构 |
| 2026-08-26 | xiaobai-mox-full-landing | 小白MOX全落地 |
| 2026-08-26 | xiaobai-voice-mox-enterprise-spec | 小白语音MOX企业级规格 |

<a id="lowcode-directory-design"></a>
## 目录架构设计卡：低代码与动态配置（2026-10-01）

> 目标设计接缝；既有正文按原日期/类型解释，未实施能力不标已完成。

| 设计项 | 本目录约定 |
|---|---|
| 输入 | 已有规格、设计决策与依赖关系 |
| 处理与边界 | 逐任务引用长期规范而不复制；先复用后实现，结束记录真实测试与遗留项 |
| 输出 | 任务状态、证据、偏差和目录源回写 |
| 维护角色 | 开发交付owner（角色建议，未指派个人） |
| 配置语义 | [统一规范](docs/standards/lowcode-dynamic-configuration.md#model)，本目录不复制覆盖/生命周期规则 |
| 本目录设计 | [详细接缝](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#progress) |
| 验收 | 类型/依赖/权限/版本/异常/恢复按相关LC-Q条目补证；设计完成与运行验证分开 |

全目录关系见 [目录矩阵](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories)。新增架构文档应符合 [文档设计契约](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#document-contract)，各主题拥有自己的事实主源。

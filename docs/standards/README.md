# 规范与标准层入口 — Standards

> **层定位**：L5 规范与治理层。定义"必须怎么做"，是代码与文档的**约束源**。
> 上层入口：[文档中心](../README.md) · 结构规范：[ARCHITECTURE-OF-DOCS.md](../ARCHITECTURE-OF-DOCS.md) · 归一化索引：[`../normalization/README.md`](../normalization/README.md)

---

## 一、本层文档

| 编号 | 文档 | 说明 | 状态 |
|------|------|------|:----:|
| **AINA-STD-001** | [`ai-native-architecture-standard.md`](./ai-native-architecture-standard.md) | AI 第一性原理企业架构规范标准：分层模型、域包结构（`routes → application → domain ← infrastructure`）、门禁 | 🟢 生效 |
| — | [`architecture-data-separation.md`](./architecture-data-separation.md) | 架构-数据分离规范 v1.0 | 🟢 生效 |
| — | [`engine-kernel.md`](./engine-kernel.md) | 引擎内核：一切皆可插件化（遵循 AINA-STD-001 域包模式） | 🟢 已上线 |
| — | [`engine-universe.md`](./engine-universe.md) | 引擎宇宙图谱：引擎链接关系的**唯一权威**（AINA-STD-001 §9） | 🟢 生效 |
| **EAF-STD-001**<br>(EA-DOC-063) | [`expert-alliance-flow-standard.md`](./expert-alliance-flow-standard.md) | 通用 AI 知识图谱专家联盟业务处理流程行业规范标准 | 🟢 权威 |
| **EA-NORM-001** | [`expert-alliance-normalization-mode.md`](./expert-alliance-normalization-mode.md) | 开发专家联盟·归一化处理模式规范 | 🟢 生效 |
| **PORT-NORM-001** | [`expert-alliance-port-norm.md`](./expert-alliance-port-norm.md) | 开发专家联盟·核心服务端口规划规范（与 [`../api/PORT-REGISTRY.md`](../api/PORT-REGISTRY.md) 联动） | 🟢 生效 |
| — | [`feature-flags-v2.1.md`](./feature-flags-v2.1.md) | MOX v2.1 Feature Flag 参考：影响编译产物的 Cargo `[features]` 开关清单（原 `docs/standards/feature-flags-v2.1.md`） | 🟢 参考 |
| — | [`mox-studio.md`](./mox-studio.md) | 璇玑工作台（Mox Studio）：用户视角融合层、低门槛交互 | 🟢 生效 |
| — | [`project-atlas.md`](./project-atlas.md) | 项目全息图谱：项目机器图谱化的唯一权威（AINA-STD-001 §10） | 🟢 生效 |
| **PR-STD-V2.0** | [`project-registry-v2.md`](./project-registry-v2.md) | 项目注册表 V2：一切皆是项目（生效 2026-08-26） | 🟢 生效 |

## 二、与归一化治理的关系

| 位置 | 职责 |
|------|------|
| 本目录 | **规范正文**：定义标准本身（可执行约束） |
| [`../normalization/`](../normalization/README.md) | **索引与映射**：BP / API / ARC / VAL / TPL 五类索引，把规范落到具体文档 |
| [`../enterprise/22-全文档归一化总控卡与权威链单源映射表-V1.0.md`](../enterprise/22-全文档归一化总控卡与权威链单源映射表-V1.0.md) | **权威链总控**：跨文档等价关系与优先级 |

## 三、新增标准怎么提

1. 编号（`<域缩写>-STD-<序号>` 或 `<域缩写>-<类型>-<序号>`），在 `../enterprise/00-INDEX.md` 登记。
2. 标准正文必须含：适用范围 / 约束条款（可判定）/ 校验方式（脚本或人工）/ 违规处置。
3. 若影响 CI：同步 `scripts/gate/ci-gate.ps1` 的门禁项。
4. 在本页表格登记，并在 [`../normalization/`](../normalization/README.md) 建对应索引条目。

<a id="lowcode-directory-design"></a>
## 目录架构设计卡：低代码与动态配置（2026-10-01）

> 目标设计接缝；既有正文按原日期/类型解释，未实施能力不标已完成。

| 设计项 | 本目录约定 |
|---|---|
| 输入 | 全维配置类别、业务/技术约束与质量目标 |
| 处理与边界 | 定义配置身份、作用域覆盖、生命周期、不可变发布与验收；来源统一LC-STD-001 |
| 输出 | LC-D01–20与LC-Q01–12可判定约束 |
| 维护角色 | 规范维护角色（角色建议，未指派个人） |
| 配置语义 | [统一规范](docs/standards/lowcode-dynamic-configuration.md#model)，本目录不复制覆盖/生命周期规则 |
| 本目录设计 | [详细接缝](docs/standards/lowcode-dynamic-configuration.md#scope) |
| 验收 | 类型/依赖/权限/版本/异常/恢复按相关LC-Q条目补证；设计完成与运行验证分开 |

全目录关系见 [目录矩阵](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories)。新增架构文档应符合 [文档设计契约](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#document-contract)，各主题拥有自己的事实主源。

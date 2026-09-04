# mox_sys 多公司 / 多团队数据库治理规范

> 定位：`mox_sys` 母版是**跨公司共享的数据库内核**。本文件规定不同公司、不同团队如何在同一母版之上安全并行：认领命名空间、划分所有权、走通从建表到上线的全链路。
> 配套：契约 `module-contract.md`；注册清单 `module-registry.yml`；DDL `mox_sys-universal-template.sql`。

---

## 1. 组织模型（公司 → 团队 → 模块 → 表前缀）

```text
公司 Company（如 acme / beta-corp）
 └─ 团队 Team（如 acme-infra / acme-risk）
     └─ 模块 Module（code 全局唯一，如 acme_risk）
         └─ 表前缀 Namespace（如 acme_risk_*，全局唯一，一次认领永久保留）
```

铁律：
- **一个前缀只有一个 owner 团队**；前缀内全部表、迁移、数据质量归 owner 负责。
- **前缀全局唯一、禁止复用**：模块下线后前缀进入保留期（≥ 12 个月），防止语义漂移。
- 平台保留前缀：`sys_*`（系统能力）与 `mox_sys_*`（模块注册/图谱）归**平台团队**，任何公司/团队不得新增 `sys_*` 表。

## 2. 命名空间认领流程（全链路第 1 步）

1. 团队提交认领申请：`module_code`（建议 `<company>_<domain>`）、所属公司、owner 联系人、tenant_mode、依赖（`requires`）。
2. 平台团队做**冲突检查**：前缀未占用、不与保留前缀冲突、依赖模块已存在且版本区间可满足。
3. 通过后登记两处（缺一无效）：
   - `mox_sys_module`：`module_code` / `owner_code` / `module_kind` / `manifest`；
   - `module-registry.yml`：code / owner / company / version / requires / capabilities。
4. 生效：团队获得该前缀的 DDL 与迁移提交权。

## 3. 所有权矩阵（RACI）

| 事项 | 平台团队 | 模块 owner 团队 | 其他团队 |
|---|---|---|---|
| `sys_*` / `mox_sys_*` 表结构 | **A/R** | I | 只读 |
| 命名空间冲突仲裁 | **A/R** | C | I |
| 自有前缀表结构 | C（评审） | **A/R** | 只读 |
| 自有前缀迁移与回滚 | I | **A/R** | — |
| 跨模块引用（ID/事件/图谱） | C | R（owner 校验） | 只消费 |
| 全局 verification 验证 | **A/R** | R（本模块部分） | I |

R=执行 A=拍板 C=被咨询 I=知会。唯一不可协商项：**任何团队不得直写他模块的表**。

## 4. 多公司部署隔离

| 公司规模 | 拓扑 | 要点 |
|---|---|---|
| 中小公司 | 共享实例 + `L` 逻辑隔离 | 同一 `mox_v3` 库，`tenant_id` 隔离；跨公司数据仅经授权 API |
| 中大型 | 每公司独立库 | 每公司一个 `mox_v3`（母版重复执行即可）；ID 为 UUID v7，库间合并无冲突 |
| 强合规/私有化 | 独立部署 | 公司自有实例；模块安装仍走 `module-registry.yml` 认领记录 |

同一公司内部多团队**共享同一 mox_v3**，靠 §1 前缀所有权 + §6 全链路评审并行开发，互不阻塞。

## 5. 环境晋级矩阵（全链路）

| 环境 | DDL 来源 | 必过关卡 |
|---|---|---|
| dev | 团队自装母版 + 本模块迁移 | `mox-v3.0-verification.sql` 通过 |
| test | CI 流水线安装母版 → 按依赖序装模块 | 迁移可重复执行；checksum 一致 |
| stage | 与 prod 同一发布包 | 回滚脚本演练一次；孤儿检查 0 行 |
| prod | 发布包 + 审批单 | expand→backfill→switch→contract 分步执行；`mox_sys_schema_version` 落账 |

## 6. 模块贡献全链路（Checklist，逐项打勾才可合入）

- [ ] 1. 命名空间已认领（§2），`module_code` 进 registry
- [ ] 2. 每张表满足：`BINARY(16)` UUID v7 主键、`tenant_id`（租户表必带）、生命周期列、软删除 `deleted_at`
- [ ] 3. 类别字段全部显式英文短码 + `CHECK (col IN (...))` + `COMMENT`；**无任何数值型类别**；二值 `Y/N`
- [ ] 4. 归一化达标：无重复列组、关联拆独立实体（2NF）、字典/配置外提（3NF/BCNF）、多值拆表（4NF）
- [ ] 5. 未复制任何 `sys_*` / 他模块表；未直写他模块表；跨模块只走 ID / outbox 事件 / 图谱关系
- [ ] 6. 迁移脚本符合 expand→backfill→switch→contract，含回滚脚本与 checksum
- [ ] 7. 索引最左前缀含 `tenant_id`；高吞吐表有 `trace_id` 与归档策略
- [ ] 8. `module-registry.yml` 与 `mox_sys_module` 登记**一致**（tables / requires / capabilities / owner）
- [ ] 9. dev→test→stage 验证全绿，`verification.sql` 无新增告警

## 7. 变更冲突与仲裁

- 前缀冲突：先登记者胜出，后申请者换前缀；对结果有异议由平台团队终审。
- 需要改动他模块行为：**不改他的表**，改为：订阅其 outbox 事件，或在图谱 `mox_sys_edge` 写关系（带 `source_module` 溯源）。
- 平台能力缺口（需要新 `sys_*` 表/列）：向平台团队提需求单，由平台团队评审后进母版下个版本，禁止各团队自建影子表。

## 8. 原创性与合规

- 本母版 DDL 为**100% 原创设计**：未复制任何第三方系统的表结构或代码；`mox-v3.0-standards-review.md` 中列出的外部系统仅作为公开设计原则的对照来源。
- 各公司模块自带 `license`（SPDX）声明；共享注册清单中不得提交真实租户数据、密钥或生产连接串。

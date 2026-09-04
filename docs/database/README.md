# mox 数据库 v3.0 基线

这是一套面向“开发专家联盟/低代码/企业治理”平台的 MySQL 8.3 关系模型基线。它不是把旧库 79 张表机械改名，而是把身份、租户、企业组织、权限、审计、元数据、流程、RPA、文件、通知、专家联盟、知识图谱和 AI 运行记录按边界重新分层。

## 交付物

- [**`mox 模块化系统架构企业级数据库模板.md`**](mox 模块化系统架构企业级数据库模板.md)：mox 模块化系统架构分析 + 归一化规范（BCNF）+ 19 维度包地图 + 可扩展模型。**本目录的总纲与企业级母版说明。**
- [`mox_sys/mox_sys-universal-template.sql`](mox_sys/mox_sys-universal-template.sql)：**唯一权威归一化母版 DDL**（56 张标准化表：19 维度包系统内核 + P17 模块注册与知识图谱）。自带 `CREATE DATABASE IF NOT EXISTS mox_v3`，**一键安装**即全量落库，全目录无第二份并行定义。
- [`mox_sys/install.ps1`](mox_sys/install.ps1) / [`mox_sys/install.sh`](mox_sys/install.sh)：一键安装脚本（Windows / Linux·macOS），把上面单一母版灌入 MySQL。
- [`mox-v3.0-baseline.sql`](mox-v3.0-baseline.sql)：mox 产品**现状落库**（76 张核心表），是迁移起点而非母版；与归一化母版不是安装关系。
- [`mox-v3.0-migration-plan.md`](mox-v3.0-migration-plan.md)：现状库 → 归一化母版的映射、顺序、回滚和兼容策略。
- [`mox-v3.0-verification.sql`](mox-v3.0-verification.sql)：上线前只读结构/隔离性检查。
- [`mox-v3.0-standards-review.md`](mox-v3.0-standards-review.md)：v3.0 规范与开源系统对照结论。
- [`mox_sys/`](mox_sys/README.md)：面向后续所有系统复用的模块化数据库母版、知识关系层和跨库契约。
- [`mox_sys/governance.md`](mox_sys/governance.md)：**多公司/多团队治理**——命名空间认领、所有权矩阵、多公司部署隔离、环境晋级与贡献全链路 Checklist。
- [`mox_sys/mox_sys-seed.sql`](mox_sys/mox_sys-seed.sql)：可选平台引导种子（30 类字典 + platform/admin 引导），装完即用。

## 权威模板关系与一键安装

`mox_sys` 是规范与模块契约的权威入口；其唯一可安装 DDL 是 `mox_sys-universal-template.sql`（已合并 P17 模块注册与知识图谱，共 56 张表）。`mox-v3.0-baseline.sql` 是产品现状落库，仅作为迁移起点。

**一键安装**（全目录唯一真相）：

```text
# Windows
cd docs/database/mox_sys && .\install.ps1 -User root -Password <pwd>
# Linux / macOS
cd docs/database/mox_sys && ./install.sh 127.0.0.1 3306 root <pwd>
```

等价于把 `mox_sys-universal-template.sql` 直接灌入 MySQL（该文件自带建库与 `USE mox_v3`）。**不存在“模式 A/模式 B”两套并行母版，也不存在重复表定义**——旧 `mox_sys-template.sql`（P17）与 `mox_sys-extension-pack.sql` 已废弃并合并进本母版，仅保留为历史指针。

以后新增系统先安装本母版，再选择 `iam/meta/flow/rpa/ea/kg/ai` 等领域模块，不再从业务表反向复制系统能力。

## 关键设计结论

1. `tenant_id` 是唯一租户隔离键；`enterprise_id` 是租户内企业主体；`org_unit_id` 是企业内组织节点。三者不混用。
2. `sys_user` 是全局身份主体；`sys_tenant_member`、`sys_org_member` 负责多租户、多企业、多部门关联。
3. 所有新主键和外键引用使用 `BINARY(16)`，由应用层生成 UUID v7。数据库不再使用自增、`UUID_SHORT()`、nanoid 触发器。
4. 业务状态用 `CHAR(1)` 或 `VARCHAR(24)` 英文短码，布尔值统一 `Y/N`；金额只用 `DECIMAL`；JSON 只保存可演进配置，不替代可查询的关系字段。
5. 业务表默认 `deleted_at` 软删除；操作审计写入统一 `sys_audit_event`，不再为每张业务表复制一套 `*_record`/`*_link_record`。
6. DDL 默认不建立跨服务物理外键。服务内由应用事务、唯一索引、定期孤儿检查保证引用一致性；如部署为单体，可在发布配置中增加物理 FK。
7. Quartz 的 `QRTZ_*` 表属于调度器内部协议，保持官方表结构、独立前缀和独立生命周期，不强行加租户列；租户业务调度通过 `rpa_job` 的 `tenant_id` 关联。

## 短码约定

| 语义 | 约定 |
|---|---|
| 二值 | `Y` / `N` |
| 启用状态 | `A` active、`I` inactive、`D` disabled |
| 任务状态 | `P` pending、`R` running、`S` success、`F` failed、`C` cancelled |
| 删除 | `deleted_at IS NULL` 表示有效 |
| 数据范围 | `S` self、`D` department、`E` enterprise、`T` tenant、`C` custom |
| 租户模式 | `L` logical、`P` physical、`H` hybrid |

## 现有库的处理原则

现有导出实际包含约 79 张表（不是报告标题中的 58 张），有 `utf8mb3`、多套 ID、两套时间类型、重复 demo 表、备份表、Quartz 表和大量快照表。旧库先冻结为 legacy；完成双写/回填/校验后再切读，禁止在生产直接 `DROP TABLE` 或关闭外键检查后导入。

本基线是 MySQL 8.3 目标模型。若运行时仍以 SQLite 为默认后端，应通过 repository/adapter 映射，不要把 MySQL DDL 原样塞进 SQLite。

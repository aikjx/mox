# v3.0 规范与开源系统对照结论

> **原创性声明（100% 原创）**：本套 DDL 与文档为原创设计，未复制任何第三方系统的表结构、代码或文档文本。下表外部系统仅作为公开设计**原则**的对照参考（原则本身为业界公知，不构成复制）；全部表结构、命名、约束与治理规范均为本项目独立设计。

本报告现作为 `mox_sys` 母版的依据之一；可复用的模块契约、关系图谱模型和跨库矩阵见 [`mox_sys/`](mox_sys/README.md)。

“所有最伟大的开源系统”无法作为一个可穷举的验证集合，因此本次采用代表性、可复核的基线：MySQL 官方 DDL/字符集/JSON/CHECK 文档，Quartz 官方 MySQL InnoDB 表，Keycloak 的版本化数据库变更机制，RuoYi 的传统 RBAC 业务分表，以及 OpenFGA/Casbin 的关系授权思想。

## 对照结果

| 参考 | 提取的可复用原则 | v3 落地 |
|---|---|---|
| MySQL 8.x | 使用 `utf8mb4`；JSON 做文档校验，查询字段仍应有生成列/关系列；CHECK 可用于值域约束 | 全库 utf8mb4；配置字段 JSON；状态使用英文短码并在关键表加 CHECK |
| Quartz | 调度器内部表是协议表，复合主键、锁和集群状态不能随业务表任意改造 | `QRTZ_*` 独立安装、独立生命周期；租户调度另建 `rpa_job/rpa_job_run` |
| Keycloak | Schema 变更必须是可审阅、可测试、能转换旧数据的 change-set，而非直接覆盖 | 单独 v3 schema + 显式迁移映射 + 双写/校验/切读 |
| RuoYi | 用户、部门、角色、岗位、关系表分离，业务授权不应依赖一张宽用户表 | `sys_user`、`sys_tenant_member`、`sys_org_member`、`sys_role`、`sys_post` 分离 |
| OpenFGA | 授权模型与关系事实分开；关系可以表达组织/资源继承和细粒度关系 | SQL 保存角色、权限、组织范围；复杂资源关系可投影到 OpenFGA，不把权限塞进 JSON |
| Casbin | 域/租户维度应进入授权匹配，而不是把所有租户策略混在一起 | `tenant_id` 是权限关系索引前缀，`data_scope` 与组织范围单独建模 |

## 关键取舍

### UUID v7 与 BINARY(16)

新库使用应用生成 UUID v7，存储为 `BINARY(16)`。这把全局唯一、跨库合并和较好的时间局部性结合起来，同时避免 `CHAR(36)` 在所有二级索引中额外占用字节。排障时由驱动或 SQL `UUID_TO_BIN/BIN_TO_UUID` 转换展示。旧 nanoid 不强行转换为新主键，只保留 legacy 映射。

### DATETIME(3) 与旧毫秒 long

MySQL 目标库采用 UTC `DATETIME(3)`，便于范围查询、分区和运维排障。现有 Rust/Java API 若需要 epoch 毫秒，在 repository 层转换；不要让同一张表同时存在两个时间语义。

### 物理外键

跨服务、分片和在线 DDL 场景默认不建物理 FK，但不等于放弃一致性：应用事务负责写入，唯一索引负责局部约束，定时任务负责孤儿检测，审计负责追溯。若某个部署是单体且不分片，可在部署 profile 中补充 FK，不修改领域模型。

## 可复核来源

- [MySQL 8.4 CHECK Constraints](https://dev.mysql.com/doc/refman/8.4/en/create-table-check-constraints.html)
- [MySQL Character Sets and Unicode](https://dev.mysql.com/doc/refman/8.4/en/charset.html)
- [MySQL JSON Data Type](https://dev.mysql.com/doc/refman/8.4/en/json.html)
- [Quartz 官方 MySQL InnoDB 表](https://github.com/quartz-scheduler/quartz/blob/main/quartz/src/main/resources/org/quartz/impl/jdbcjobstore/tables_mysql_innodb.sql)
- [Keycloak 官方数据库变更说明](https://github.com/keycloak/keycloak/blob/main/docs/updating-database-schema.md)
- [RuoYi 官方用户/部门/角色关联代码](https://github.com/yangzongzhuan/RuoYi/blob/master/ruoyi-admin/src/main/java/com/ruoyi/web/controller/system/SysUserController.java)
- [OpenFGA 官方关系模型概念](https://github.com/openfga/openfga.dev/blob/main/docs/content/concepts.mdx)
- [Casbin 官方域与角色管理说明](https://github.com/casbin/casbin.github.io/blob/master/index.html)

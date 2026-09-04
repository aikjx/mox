# mox v3.0 迁移与兼容方案

## 1. 现状判定

用户提供的 Navicat 文件实际解析出 79 个 `CREATE TABLE`：核心业务、Quartz 11 表、备份表、审计快照表和 demo 表混在一起。报告写的 58 张表是模块统计口径，不是物理表数量。必须先以 `information_schema` 作为事实源。

主要问题：

- `utf8mb3` 与 `utf8mb4` 混用，且排序规则有 `general_ci`、`unicode_ci`、`0900_ai_ci` 多套。
- 主键混用 bigint 自增、nanoid、UUID_SHORT；触发器参与发号，迁移和多库复制困难。
- 时间混用 bigint 毫秒、DATETIME，且有 `CAST(SYSDATE() AS UNSIGNED)` 这种非毫秒值。
- `sys_org_code/org_id` 同时承担机构、部门、租户含义；无法严格证明跨租户不可见。
- `demo`/`demo_info` 重复，`gen_cloud_*` 与 `gen_module_*` 两套生成器重叠，`*_record` 快照散落。
- `sys_oss_relation` 无主键、`sys_user` 复合主键冗余、金额使用 double、FTP 级联删除风险、备份表在业务库。

## 2. 目标映射

| 旧对象 | v3 目标 | 处理 |
|---|---|---|
| `sys_user`、`sys_user_record` | `sys_user` + `sys_audit_event` | 用户只保留当前态；历史写统一审计事件 |
| `sys_org`、`sys_dept` | `sys_enterprise` + `sys_org_unit` | 先识别企业主体，再导入组织树 |
| `org_id`、`sys_org_code` | `sys_tenant_member.tenant_id` / `sys_org_unit.id` | 不允许按字符串猜测；需人工确认映射表 |
| `sys_role*`、`sys_menu*` | `sys_role`、`sys_permission`、`sys_menu` 及关系表 | 权限由资源+动作表达，菜单不再等同权限 |
| `sys_dict_type/data` | `sys_enum_type`、`sys_enum_item` | 代码字段从数字改为英文短码 |
| `sys_config`、`sys_param*` | `sys_setting` | 作用域统一 `G/T/E/U`，密文迁移到 secret provider |
| `gen_cloud_*`、`gen_module_*`、`gen_*` | `meta_*` + `gen_project/template/artifact` | 两套生成器合并为一个元数据真源 |
| `meta_connection/database/table/column` | `meta_connection/catalog/table/column` | 数据库和 schema 作为 catalog，不复制业务元数据 |
| `*_record`、`*_link_record` | `sys_audit_event` | before/after JSON + hash 链 + 归档策略 |
| `sys_oss`、`sys_oss_relation` | `sys_file_object`、`sys_file_link` | 先文件对象，再多态资源关联；不使用无主键关系表 |
| `sys_ftp_*` | `connector_endpoint/task/task_item` | FTP/SFTP/HTTP/S3 共用连接抽象 |
| `rpa_distributed_*` | `rpa_node/assignment/workflow/job/job_run` | 节点注册、租户授权、业务作业分离 |
| `qrtz_*` | 官方 Quartz 表 | 独立脚本、独立清理，不改成租户业务表 |
| `demo*` | 不迁移 | 测试数据另放 fixture；生产库删除前先归档 |
| `sys_menu_bak_expert_alliance_*` | 备份库/对象存储 | 禁止留在应用 schema |

## 3. 执行顺序

1. 对旧库做只读盘点：表、列、字符集、索引、触发器、行数、空值率、孤儿引用和时间范围。
2. 创建 `mox_v3`，执行 baseline DDL；安装官方 Quartz 脚本，不把旧表直接改名。
3. 建立显式映射表：`legacy_table`、`legacy_id`、`new_table`、`new_id`、`mapping_status`。不要依赖 bigint/nanoid 数值可转换。
4. 先迁移全局用户，再迁移租户/企业/组织，再迁移成员关系和权限；没有租户归属的数据进入隔离的 quarantine 表，禁止默认归入某租户。
5. 迁移业务主数据，再迁移审计/历史/文件引用；旧快照先转成 `sys_audit_event`，保留来源表和原始主键在 `after_data` 的 `_legacy` 节点。
6. 双写期间由应用生成 UUID v7；禁止在数据库增加新发号触发器。读路径增加租户断言：任何 tenant-owned 查询必须带 `tenant_id`。
7. 校验计数、哈希、租户交叉泄露、孤儿引用、时间转换、金额合计和权限回归；通过后切读。
8. 观察一个完整保留周期后，才归档旧库。删除必须由独立审批和备份策略执行。

## 4. 时间和 ID 兼容

- 旧 bigint 时间若为 13 位毫秒：`FROM_UNIXTIME(old_time / 1000.0)` 转 UTC `DATETIME(3)`；14 位 `YYYYMMDDHHMMSS` 先人工校验，禁止当作毫秒直接导入。
- 旧 nanoid/UUID_SHORT 保留在迁移日志中的 `legacy_id`，不写入新主键。
- v3 新 ID 由服务层生成 UUID v7，驱动绑定为 16 字节；不要调用 MySQL `UUID()` 或触发器。
- 金额迁移前检查 `double` 误差和币种；以原始 decimal 文本或审计凭证为准，不对浮点值直接四舍五入覆盖。

## 5. 不采用的做法

- 不执行“全库 `SET FOREIGN_KEY_CHECKS=0` 后导入”作为生产迁移方案。
- 不把所有表都添加一个含义不明的 `org_id` 来假装多租户。
- 不用 `UNIQUE(code, deleted_at)` 伪造软删除唯一性；业务编码要么不可复用，要么用生成的 active key/归档策略保证。
- 不把所有字段塞入 JSON；能过滤、排序、授权、关联的字段必须是关系列。
- 不把日志、outbox、运行轨迹和交易主表放在同一套无限增长的快照结构中；大表按时间归档并保留索引前缀。

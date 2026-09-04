# mox_sys 模块契约

## 1. 模块边界

模块是可独立部署或安装的 bounded context。模块只拥有自己的表前缀和迁移目录：

```text
mox_sys_*       平台基座
iam_*           身份与访问控制扩展
meta_*          元数据/低代码
flow_*          流程
rpa_*           自动化
ea_*            专家联盟
kg_*            图谱索引
ai_*            AI 运行记录
<domain>_*      业务域
```

禁止：

- 业务模块复制系统用户、租户、组织、审计表。
- 模块直接更新别的模块的表。
- 用 `tenant_id` 为空表示“所有租户”；平台共享资源必须由 `scope_kind=G` 或明确的共享关系表达。
- 用多态字符串外键伪装强一致交易关系；需要事务一致性的关系应保留明确 ID 并由 owner 模块校验。

## 2. 最小模块元数据

每个模块必须声明：

| 字段 | 规则 |
|---|---|
| `module_code` | 小写 ASCII、稳定、不复用 |
| `module_version` | SemVer；数据库迁移版本单调递增 |
| `owner` | 维护团队/联系人 |
| `requires` | 依赖模块和版本范围 |
| `tenant_mode` | `G/P/H`：全局/租户/混合 |
| `tables` | 自有表清单和敏感级别 |
| `events` | 发布/订阅事件及 schema 版本 |
| `capabilities` | 权限资源和动作 |
| `migration_checksum` | 发布包 checksum |
| `license` | SPDX 标识 |

## 3. 数据生命周期

每张业务表至少具备：

```sql
id BINARY(16)              -- application generated UUID v7
tenant_id BINARY(16)       -- tenant-owned table required
created_at DATETIME(3)
updated_at DATETIME(3)
deleted_at DATETIME(3)
row_version BIGINT UNSIGNED
```

高吞吐事件表还必须具备：

```text
occurred_at、partition_key、trace_id、retention_class、archive_state
```

删除采用“业务软删除 → 保留期归档 → 审批物理清理”。不可变审计、账务凭证、许可证签名和迁移记录不得更新覆盖。

## 4. 事务与事件

模块内写事务和 `mox_sys_outbox_event` 同库提交；消费者使用 `event_id` 幂等。事件 envelope 固定包含：

```json
{
  "event_id": "uuid",
  "event_type": "module.entity.action",
  "schema_version": "1.0.0",
  "tenant_id": "uuid-or-null",
  "aggregate_type": "entity",
  "aggregate_id": "uuid",
  "occurred_at": "RFC3339",
  "trace_id": "trace",
  "payload": {}
}
```

跨模块不使用分布式数据库事务；需要协同写入时使用 outbox + inbox/idempotency + Saga/补偿。

## 5. 查询与索引

- 租户查询索引最左前缀优先：`(tenant_id, status, created_at)` 或 `(tenant_id, owner_id, created_at)`。
- 关系/明细表必须有单独主键，禁止无主键 InnoDB。
- 高频分页使用稳定排序键和 keyset pagination，不使用深度 `OFFSET`。
- JSON 只承载扩展配置；需要过滤/排序/授权的属性必须提升为列或生成列。
- 日志、outbox、消息、运行轨迹按时间归档；禁止无限增长的统一快照表。

## 6. 版本兼容

数据库迁移遵循 expand → backfill → switch → contract：先加新结构，回填并双读/双写，切换后观察保留周期，最后再清理旧结构。任何破坏性变更必须有备份、回滚脚本、数据校验和停机/在线变更说明。

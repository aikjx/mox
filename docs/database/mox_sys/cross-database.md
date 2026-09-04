# mox_sys 跨数据库兼容矩阵

## 目标后端

| 能力 | MySQL 8.3+ | PostgreSQL 14+ | SQLite 3.40+ | 设计策略 |
|---|---|---|---|---|
| 事务主库 | 生产首选 | 生产首选 | 单机/测试 | repository 层隔离方言 |
| UUID v7 | `BINARY(16)` | `BYTEA` 或原生 UUID | `BLOB` | 应用层生成，驱动转换 |
| 时间 | `DATETIME(3)` UTC | `TIMESTAMPTZ(3)` UTC | ISO 文本/整数适配 | API 统一 RFC3339 |
| JSON | `JSON` | `JSONB` | TEXT + JSON 校验 | 核心字段不依赖 JSON 查询方言 |
| CHECK | 支持 | 支持 | 版本相关 | 代码校验 + 数据库约束双保险 |
| 生成列 | 支持 | 支持 | 版本相关 | 仅作为索引优化，不作为业务真相 |
| 行级隔离 | 应用/视图 | 可选 RLS | 应用层 | `tenant_id` 永不省略 |
| 全文/向量 | 外部搜索/向量库 | 可选扩展 | 外部索引 | 只存 `embedding_ref` 和索引版本 |
| 调度/锁 | 独立 Quartz/worker | 独立 Quartz/worker | 不承担集群锁 | 业务调度表与引擎协议表分离 |

## 兼容规则

1. 领域 SQL 只使用 ANSI 基础语法、参数绑定和明确列名；MySQL/PostgreSQL/SQLite 差异集中到 adapter。
2. `BINARY(16)`、JSON、生成列、分区、全文索引、RLS 是 capability，不得写入 portable core 的必需路径。
3. 每个模块发布包至少执行 MySQL + SQLite migration smoke test；涉及复杂查询时增加 PostgreSQL test matrix。
4. 不能兼容的能力必须在 `module-registry.yml` 标注 `requires_capability`，安装器拒绝静默降级。
5. 外部图数据库和 RocksDB 是投影目标，不参与核心事务提交；丢失后可由 SQL/outbox 全量重建。

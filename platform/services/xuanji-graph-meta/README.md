# xuanji-graph-meta（T5 关系图 R1 Meta Service）

L4 服务层实现：3 节点 Raft 共识 + 图元数据（Schema / 权限 / 分区路由）
状态机，对接 L5 `xuanji-domain-abstractions` 中定义的 `GraphMetaProvider` 语义。

## 架构

```
┌──────────────── MetaServer ────────────────┐
│  pub API: create_space / create_tag / ...  │
│     ├─ authorize (AuthStore)               │
│     └─ propose_log(RaftLog)                │
│          ├─ cluster -> async-raft 0.6      │
│          └─ standalone -> apply_direct     │
│                                            │
│  MetaStateMachine                          │
│     ├─ schema_store (Space/Tag/EdgeType)   │
│     ├─ auth_store   (User/Role/Policy)     │
│     └─ partition_store (Host/Shard/VID→节点)│
│  快照：可选 rocksdb 0.25 持久化             │
└────────────────────────────────────────────┘
```

## 许可白名单（全部 Apache-2.0 兼容）

| crate        | version | license  |
|--------------|---------|----------|
| async-raft   | 0.6     | Apache-2.0 |
| rocksdb      | 0.25    | Apache-2.0（feature `persist-rocksdb`，默认关） |
| serde        | 1       | MIT / Apache-2.0 |
| tokio        | 1       | MIT |
| parking_lot  | 0.12    | MIT / Apache-2.0 |
| sha2 / hmac  | 0.10    | MIT / Apache-2.0 |
| hex / rand / tracing / async-trait / thiserror / anyhow | — | MIT/Apache-2.0 |

明确禁用：NebulaGraph、Neo4j、JanusGraph，以及任何 GPL/AGPL 许可证 crate。

## 严格 TDD

- **RED 阶段（`cfg(red_phase)`）**：25/25 测试强制失败。
- **GREEN 阶段**：`cargo test -p xuanji-graph-meta --test t5_r1_meta_raft -- --test-threads=1`
  - 25 passed, 0 failed。

### 覆盖矩阵

| 需求点    | 测试用例 |
|-----------|----------|
| TR5.2 Raft 3 节点选举 ≤5s | `tr5_2_election_{round_1..3}_within_5s`, `tr5_2_election_3rounds_max_within_5s` |
| TR5.3 Schema createSpace/list/dup/drop/alter | `tr5_3_create_space_and_list`, `_duplicate_error`, `_drop_space_clears_schema`, `_create_tag_and_list_tags`, `_alter_tag_add_field`, `_create_edge_type_and_list`, `_drop_notfound_tag_returns_error`, `_create_tag_unknown_space`, `_schema_synced_3followers_consistent` |
| TR5.4 Auth 用户/授权/撤销 | `tr5_4_create_user_authenticate`, `_grant_spaceadmin_allows_tag_create`, `_revoke_spaceadmin_denies_tag_create`, `_readonly_denies_write_but_allows_read` |
| TR5.5 分区路由 + VID hash | `tr5_5_register_host_and_get_route`, `_vid_hash_1000_uniform_le_15pct_cv` |
| TR5.6 白名单依赖存在 | `tr5_6_cargo_toml_contains_async_raft_and_rocksdb` |
| TR5.7 无禁用品牌字符串 | `tr5_7_src_rs_no_forbidden_graph_brands` |
| 内部辅助（`xt_*`） | `_authorize_admin_on_anything`, `_partition_store_no_host_errors`, `_schema_store_space_validation`, `_space_partition_default_16_power_of_two` |

## 独立运行

```sh
# 基础（无 rocksdb 持久化）
cargo test -p xuanji-graph-meta --test t5_r1_meta_raft

# 启用 rocksdb 持久化快照
cargo test -p xuanji-graph-meta --features persist-rocksdb --test t5_r1_meta_raft
```

## 与 L5 对齐（`xuanji-domain-abstractions::GraphMetaProvider`）

已实现语义：`createSpace`、`dropSpace`、`listSpaces`、`createTag`、`createEdgeType`、`alterTag`、`dropTag`、`dropEdgeType`、`showHosts`、`listTags`、`listEdgeTypes`。鉴权与分区路由作为附加能力在 `MetaServer` 上暴露为独立 API，`GraphMetaProvider` 适配层可直接映射。

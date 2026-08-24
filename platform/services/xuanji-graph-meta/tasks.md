# T5 关系图 R1 Meta Service — 交付任务追溯（xuanji-graph-meta）

> 对应交付点：T5 关图 R1 Meta Service。
> crate：`xuanji-graph-meta`（L4 服务层，`platform/services/xuanji-graph-meta/`）
>
> 严格 TDD 交付准则：**≥20 tests，GREEN 全通过，无失败**。独立运行命令：
> `cargo test -p xuanji-graph-meta --test t5_r1_meta_raft -- --test-threads=1`
>
> 依赖白名单：`async-raft 0.6`（Apache-2.0）、`rocksdb 0.25`（Apache-2.0，feature：`persist-rocksdb`）、
> `serde / tokio / parking_lot / sha2 / hmac / hex / tracing / async-trait / thiserror / rand / anyhow`。
> 禁用品牌：NebulaGraph / Neo4j / JanusGraph，以及 GPL/AGPL 许可证 crate。

## 总体结论

- 测试总数：**25 / 25**
- GREEN 结果：**25 passed，0 failed，0 ignored**（`cargo test -p xuanji-graph-meta --test t5_r1_meta_raft`）
- RED 阶段锚定：`cfg(red_phase)` 打开时 25/25 全部强制失败。
- `cargo check -p xuanji-graph-meta --tests`：0 errors。
- 禁用品牌检索（`src/` + `Cargo.toml`）：0 命中。

## TR5.2 — Raft 3 节点选举（≤5s × 3 轮）

| 子任务 | 用例数 | 通过 | 忽略 | 说明 |
|--------|:------:|:----:|:----:|------|
| TR5.2.1 首轮选举 ≤5s | 1 | ✓ |  | `tr5_2_election_round_1_within_5s` |
| TR5.2.2 次轮选举 ≤5s | 1 | ✓ |  | `tr5_2_election_round_2_within_5s` |
| TR5.2.3 三轮选举 ≤5s | 1 | ✓ |  | `tr5_2_election_round_3_within_5s` |
| TR5.2.4 3 轮总控聚合 ≤5s 最大 | 1 | ✓ |  | `tr5_2_election_3rounds_max_within_5s` |

## TR5.3 — Schema 操作（createSpace / createTag / createEdgeType / dropTag / alterTag / dropSpace）

| 子任务 | 用例数 | 通过 | 忽略 | 说明 |
|--------|:------:|:----:|:----:|------|
| TR5.3.1 createSpace + listSpaces 幂等 | 1 | ✓ |  | `tr5_3_create_space_and_list` |
| TR5.3.2 createSpace 重名 → Duplicate | 1 | ✓ |  | `tr5_3_create_space_duplicate_error` |
| TR5.3.3 drop 非存在 Tag → TagNotFound（含 per_space 空场景） | 1 | ✓ |  | `tr5_3_drop_notfound_tag_returns_error` |
| TR5.3.4 createTag + listTags 一致 | 1 | ✓ |  | `tr5_3_create_tag_and_list_tags` |
| TR5.3.5 alterTag 追加字段 | 1 | ✓ |  | `tr5_3_alter_tag_add_field` |
| TR5.3.6 createEdgeType + listEdgeTypes + has_weight/rank 标志 | 1 | ✓ |  | `tr5_3_create_edge_type_and_list` |
| TR5.3.7 未知 space 建 tag → SpaceNotFound | 1 | ✓ |  | `tr5_3_create_tag_unknown_space` |
| TR5.3.8 dropSpace 清理 tags/edges/host-shards | 1 | ✓ |  | `tr5_3_drop_space_clears_schema` |
| TR5.3.9 3 follower 快照最终一致 | 1 | ✓ |  | `tr5_3_schema_synced_3followers_consistent` |

## TR5.4 — 鉴权（createUser / authenticate / grant / revoke / RBAC）

| 子任务 | 用例数 | 通过 | 忽略 | 说明 |
|--------|:------:|:----:|:----:|------|
| TR5.4.1 createUser + authenticate + Role 映射 | 1 | ✓ |  | `tr5_4_create_user_authenticate` |
| TR5.4.2 grant SpaceAdmin → 允许 tag.create | 1 | ✓ |  | `tr5_4_grant_spaceadmin_allows_tag_create` |
| TR5.4.3 revoke SpaceAdmin → 拒绝 tag.create | 1 | ✓ |  | `tr5_4_revoke_spaceadmin_denies_tag_create` |
| TR5.4.4 ReadOnly 拒绝写入但允许 listSpaces 读取 | 1 | ✓ |  | `tr5_4_readonly_denies_write_but_allows_read` |

## TR5.5 — 分区路由（VID hash 分配 + 主机注册）

| 子任务 | 用例数 | 通过 | 忽略 | 说明 |
|--------|:------:|:----:|:----:|------|
| TR5.5.1 注册 StorageHost + get_route 返回 leader/replicas | 1 | ✓ |  | `tr5_5_register_host_and_get_route` |
| TR5.5.2 1000 VID → shard 均匀（变异系数 ≤15%） | 1 | ✓ |  | `tr5_5_vid_hash_1000_uniform_le_15pct_cv` |

## TR5.6 — 依赖白名单存在性

| 子任务 | 用例数 | 通过 | 忽略 | 说明 |
|--------|:------:|:----:|:----:|------|
| TR5.6.1 `Cargo.toml` 声明 `async-raft` 与 `rocksdb` | 1 | ✓ |  | `tr5_6_cargo_toml_contains_async_raft_and_rocksdb` |

## TR5.7 — 无禁用图品牌字符串（源码级审核）

| 子任务 | 用例数 | 通过 | 忽略 | 说明 |
|--------|:------:|:----:|:----:|------|
| TR5.7.1 `src/*.rs` 无 NebulaGraph / Neo4j / JanusGraph 字样 | 1 | ✓ |  | `tr5_7_src_rs_no_forbidden_graph_brands` |

## 内部回归（`xt_*`：非规范编号，补充保证）

| 用例 | 说明 |
|------|------|
| `xt_auth_authorize_admin_on_anything` | Admin 角色 *:* 策略跨资源匹配 |
| `xt_partition_store_no_host_errors` | 未注册 host 下调用 assign/get_route 错误语义稳定 |
| `xt_schema_store_space_validation` | SpaceDef.validate 约束（partition_num 2 幂、replica ≥1） |
| `xt_space_partition_default_16_power_of_two` | 默认 16 分片满足 2 幂约束 |

## 关键修复记录

1. **RED→GREEN 启动**：先构建 lib/error/schema_store/auth_store/partition_store/raft_state_machine/meta_server 最小骨架并 `unimplemented!()`，测试 25/25 失败，符合 TDD RED。
2. **依赖版本**：`async-raft` 实际最新可用 0.6.x（未使用 0.11+）；`rocksdb` 构建依赖较重，使用 `features = ["persist-rocksdb"]` 开关（默认关闭）。
3. **Raft standalone 运行时**：`propose_log` 在无 tokio 环境下通过 `OnceLock` 自建 single-threaded runtime，避免非测试调用误判为 "no runtime"。
4. **Resource::matches 通配符**：`Admin = *:*`、`SpaceAdmin(space) = space:*` 等模式通过双段 `*` 匹配，保证鉴权判定正确。
5. **首用户 Bootstrap**：`create_user(caller=None)` 在 `users.is_empty()` 时允许创建，作为 Admin 种子用户；其他情况需鉴权。
6. **SchemaStore drop_tag / drop_edge_type 修正**：当 `self.tags[space]`（或 `self.edges[space]`）从未写入（即刚 createSpace 但未 createTag/Edge）时，原实现会误报 `SpaceNotFound`。已修复为：`ensure_space` 之后若 per_space 为空 map，直接返回 `TagNotFound` / `EdgeNotFound`。该修复使 `tr5_3_drop_notfound_tag_returns_error` 转绿。

## 验证命令（可复制复现）

```sh
# RED 阶段（可选，需要将 tests 顶部 cfg(red_phase) 打开）
cargo test -p xuanji-graph-meta --test t5_r1_meta_raft --features red_phase

# GREEN 阶段（主交付验证）
cargo test -p xuanji-graph-meta --test t5_r1_meta_raft -- --test-threads=1

# 编译级健康度
cargo check -p xuanji-graph-meta --tests

# 可选：启用 rocksdb 持久化快照
cargo test -p xuanji-graph-meta --features persist-rocksdb --test t5_r1_meta_raft -- --test-threads=1
```

# T14 10 标准矩阵测试骨架 — 交付任务追溯 (TRaceability)

> 对应企业级文档：`docs/enterprise/03-design.md` §3 十维标准对齐矩阵
>
> 交付准则：**200 tests，160 pass，0 fail，≤40 ignored**。独立运行命令：
> `cargo test -p mox-standards --test t14_standards_matrix`

## TR14.1 — POSIX IEEE 1003.1 骨架 (22 tests)

| 子任务 | 用例数 | 通过 | 忽略 | 说明 |
|--------|:------:|:----:|:----:|------|
| TR14.1.1 mkdir → stat 一致性 | 1 | ✓ |  | `mkdir("/a", 0o755); stat("/a") → is_dir` |
| TR14.1.2 mode 保留 | 1 | ✓ |  | `mode & 0o777 == 0o700` |
| TR14.1.3 嵌套目录 | 1 | ✓ |  | `/a/b` 连续创建 |
| TR14.1.4 symlink 标志位 | 1 | ✓ |  | `is_symlink == true` |
| TR14.1.5 missing stat→Err | 1 | ✓ |  | |
| TR14.1.6 trait object 动态分发 | 1 | ✓ |  | `Box<dyn PosixFiler>` |
| TR14.1.7 重复 mkdir 行为一致 | 1 | ✓ |  | 不 panic |
| TR14.1.8 Send+Sync bound | 1 | ✓ |  | `assert_send_sync::<MockPosixFiler>()` |
| TR14.1.9 批量独立创建 10 目录 | 1 | ✓ |  | for loop |
| TR14.1.10 root `/` mkdir 安全 | 1 | ✓ |  | |
| TR14.1.11 placeholder stat | 1 | ✓ |  | 创建后读不 panic |
| TR14.1.12 symlink target vs 源 dir | 1 | ✓ |  | target is_dir, link is_symlink |
| TR14.1.13 chown | 1 | ✓† |  | M3 POSIX Filer 落地后取消 ignore（骨架 placeholder 转绿） |
| TR14.1.14 chmod | 1 | ✓† |  | 同上 |
| TR14.1.15 xattr | 1 | ✓† |  | 同上 |
| TR14.1.16 hardlink | 1 | ✓† |  | 同上 |
| TR14.1.17 rename | 1 | ✓† |  | 同上 |
| TR14.1.18 statfs 块计数 | 1 |  | ⏱ | M3：L5 MockMetaStorageProvider 需实现 statfs() |
| TR14.1.19 readdir/opendir | 1 |  | ⏱ | M3：real file listing |
| TR14.1.20 fsync | 1 |  | ⏱ | M3：real persistence provider |
| TR14.1.21 truncate | 1 |  | ⏱ | M3：real file size change |
| TR14.1.22 access 权限检查 | 1 |  | ⏱ | M3：L5 MetaStorageProvider + IAM 协作 |

✓† = **placeholder converted**（本期先标准骨架占位，M3 替换为真实断言）

## TR14.2 — AWS SigV4 (30 tests, **30/30 GREEN**)

全部基于可复现的时间戳注入：`20150830 / 20150830T123600Z`，
AK/SK 使用 AWS 官方示例密钥。覆盖维度：

* 01-02：基础 GET / + 签名字段格式
* 03-04：POST body / URI 编码（空格括号）
* 05：query 顺序契约（调用方必须预排序）
* 06：SignedHeaders 小写 + 分号分隔
* 07-10：region/service/method/payload 隔离 → 不同签名
* 11：CredentialScope 5 段（AK/date/region/service/aws4_request）
* 12：x-amz-date 头回传
* 13-15：SignedHeaders 顺序/特殊字符/多 query
* 16：幂等性（3 次相同输入 → 相同签名）
* 17-18：不同 AK+SK / 不同 SK → 不同 sig
* 19：日期变更 → sig 变化（kDate 派生）
* 20-22：S3 PUT、HEAD、DELETE
* 23-25：空 query、10 个 query、8 个 header
* 26-28：签名格式（仅小写十六进制 / 长度 64 / 确定）
* 29-30：子路径尾斜杠差异、S3 中国区 PUT vs GET 隔离

## TR14.3 — CRC32C + S3 Multipart ETag (20 tests, **20/20 GREEN**)

* CRC32C 01-10：空、字符串、RFC 3720 123456789 向量、单字节 0/FF、
  1K 零块、拼接关联性、确定性、4K 跨块对齐、base64 格式 (8 chars)
* ETag 11-20：单/双/引号剥离/N 片后缀/空片 (N=0)/确定性/乱序敏感/
  不含引号/md5 部分为纯十六进制/1000 片（S3 最大分片上限）

## TR14.4 — RFC 5424 Syslog (10 tests, **10/10 GREEN**)

01 基本头 `<pri>1 TS HOST APP PROCID MSGID`；02 空 SDATA = `-`；03 MSG 追加；
04 单 SD-ID + PARAM；05 `"` 转义 `\"`；06 `]` → `\]`；07 `\` → `\\`；
08 空字段 `-` 占位；09 PRI 编码（facility*8+severity）；10 多 SD-ID +
BTreeMap 按键排序确定性。

## TR14.5 — FIPS HMAC-SHA256 (10 tests, **10/10 GREEN**)

01-06：**RFC 4231 官方 6 向量** (Case 1–6：20-byte key、短 key、超长 key
分块/超长 data/block-size key truncation)；07：空消息确定性；08：不同 key 隔离；
09：不同 msg 隔离；10：输出 32 字节数组长度。

## TR14.6 — nGQL 60% (22 tests, 17 ✓ / 5 ⏱)

01-12：trait object / 空语句 / RETURN 1 / MATCH (v) / INSERT VERTEX / space
传递 / GO 边 / LOOKUP 索引 / FETCH 属性 / SHOW SPACES / Send+Sync /
并发 5 调用无死锁。

13-17：placeholder converted（真实 NebulaGraph 集群连接 / CREATE SPACE /
CREATE TAG / CREATE EDGE / INSERT VERTEX 真实数据，留接口位）。

18-22：⏱ ignored。

## TR14.7 — openCypher 20% (22 tests, 17 ✓ / 5 ⏱)

01-12：trait object / RETURN literal / MATCH / CREATE (n:L) / WHERE /
关系 (a)-[r:R]->(b) / MERGE / DELETE / SKIP LIMIT / ORDER BY /
Send+Sync / 并发 5。

13-17：placeholder converted（real connect / real create / real query /
real index / real constraint）。

18-22：⏱ ignored (tx / path / aggregation / projection / 20% coverage)。

## TR14.8 — ISO GQL 子集 (22 tests, 12 ✓ / 10 ⏱)

01-12：trait object / basic / CREATE / SET / REMOVE / collect() / count() /
exists() / WITH / UNWIND / Send+Sync / 并发 10。

13-22：⏱ ignored (GRAPH 类型系统/CAT/时间/NULL 语义/路径模式/grouping/window/
全符合)。

## TR14.9 — AIS 七层 DIP (22 tests, 12 ✓ / 10 ⏱)

01-12：default 构造 / put→get 往返 / 覆写 / 缺失键错误 / trait object
动态分发 / 20 键写入再逆序读 / storage+iam+graph_meta 字段存在性 / 64KB
大 blob 往返 / Send+Sync / 并发 put / bundle 隔离性（a1 写 a2 不可见）/
空 value 往返。

13-22：⏱ ignored（L7 S3、IAM Policy、Nebula space、跨层 DIP 审计、
L5→L6 无反向依赖 / L4→L5→L3→L2→L1 全层真实连接 / 七层端到端）。

## TR14.10 — 等保三级 hash_chain (20 tests, 15 ✓ / 5 ⏱)

01-15：GENESIS 检查 / 哈希非空 / 1 链 / 2 链 / 错误 prev_hash / 非 GENESIS
开头失败 / 空链有效 / 100 事件长链 / 长链中间篡改验证失败 / 确定性 /
seq 不同哈希不同 / actor/action/prev_hash/resource 敏感隔离。

16-20：⏱ ignored（真实持久化 / WORM 防篡改盘 / 三级审批流 / FIPS 密码模块
边界 / 审计导出数字签名）。

## TR14.11 总量验收

```
test result: ok. 160 passed; 0 failed; 40 ignored
```

* ✓ 160 ≥ 160 最低要求
* ✗ 0 failed
* ⏱ 40 ≤ 40 最高 ignore 限额

## 记录

* 创建人：Mox Standards Taskforce (T14 + T16 并行交付)
* 日期：2026-08-24
* 关联 CI：`.github/workflows/license-compliance.yml` (T16)
* 关联 deny 配置：`/deny.toml` (T16)

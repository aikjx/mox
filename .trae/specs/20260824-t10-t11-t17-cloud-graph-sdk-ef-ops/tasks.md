# 璇玑 T10/T11/T17/E/F 企业级落地：任务队列 (tasks.md)

> **关联 SPEC**：[spec.md](./spec.md)（DOC-SPEC-20260824-T10T11T17-EF v1.0 ENT）  
> **创建时间**：2026-08-24  
> **批次数序**：A → B → C → E → F → Review Final

---

## 依赖关系概览（DAG）

```
          ┌─────────────────┐
          │  Task 1: Setup  │
          │  Projects+Dirs  │
          └────────┬────────┘
                   ▼
  ┌────────────────┼──────────────────────┐
  ▼                ▼                      ▼
Batch A         Batch B                Batch C
(T10 12 tasks)  (T11 11 tasks)         (T17 12 tasks)
  │                │                      │
  └───────┬────────┴───────┬──────────────┘
          ▼                ▼
     Task A/B/C       Batch E (T12-T18)
     Joint Test       8 tasks
     Guard (1)            │
          │               ▼
          │          Batch F (T19-T20)
          └────────► 6 tasks + 1 Final Gate
                       │
                       ▼
                  Task Final: Review Launcher
```

---

## Task 1：项目数据目录 + 脚手架文件创建

- **Status**：pending
- **Priority**：high
- **对应 AC**：（基础设施，所有 NFR-16 的前提）
- **依赖**：（无，根任务）
- **工作内容**：
  1. 创建 `projects/t10-cloud-artifacts/`、`projects/t11-graph-artifacts/`、`projects/t17-sdk-examples/`、`projects/t19-regression-report/`、`projects/t20-canary-metrics/`
  2. 每个目录放一个 `.gitkeep` + `README.md`（说明目录用途、产出格式样例）
  3. 创建 spec 交付物占位骨架目录：`deploy/helm/xuanji-dr/templates/`、`deploy/helm/xuanji/templates/`、`platform/sdk/nodejs/{xuanji-sdk-cloud,xuanji-sdk-graph,test,examples/cloud,examples/graph}`、`platform/sdk/python/{xuanji_sdk_cloud,xuanji_sdk_graph,test,examples/cloud,examples/graph}`
- **本地 TR**：
  - **[rule] TR-1.1**：5 个 `projects/*` 目录均存在，`README.md` 文件可读
  - **[rule] TR-1.2**：`platform/sdk/nodejs/` 与 `python/` 子目录均存在（≥8 个叶子目录）
  - **[rule] TR-1.3**：`ls deploy/helm/` 下 `xuanji-dr/` 与 `xuanji/` 均含 templates 子目录
- **Completion Evidence**：`tree projects deploy/helm platform/sdk/nodejs platform/sdk/python -L 3 > artifacts/setup_tree.txt` 内容校验

---

## Batch A：T10 云盘 M4

### Task A-1：冷热分层引擎 lifecycle.rs

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T10-1 / AC-T10-2 / AC-T10-20
- **依赖**：Task 1
- **工作内容**：
  1. 在 `xuanji-cloud-drive-s3/src/lifecycle.rs` 实现 `HotWarmColdLifecycle`：HOT(0-30d) / WARM(30-90d) / COLD(90d+)
  2. `transition_scan(&self) -> Vec<TransitionPlan>`：每日 02:00 UTC 定时；WARM 读自动 `touch_and_restore_to_hot`
  3. 暴露 `CloudLifecycleStats` 结构 + JSON 序列化
- **本地 TR**：
  - **[rule] A-1.1**：写入 t=0 对象 class=HOT；模拟 `now+31d` 扫描 → class 转为 WARM
  - **[rule] A-1.2**：WARM 对象 touch 读一次 → class=HOT 且 `last_accessed_at` 更新
  - **[rule] A-1.3**：`lifecycle.stats()` 返回对象计数 JSON 可 parse
  - **[rubric 0-2, threshold=2] A-1.4**：cargo 单测通过数 ≥5

### Task A-2：IAM 10 条标准 Policy

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T10-3 / AC-T10-4 / AC-T10-21
- **依赖**：Task 1
- **工作内容**：
  1. 新增 `xuanji-domain-abstractions/src/iam_standard_policies.rs`：10 条 `PolicyStatement` 常量数组 `STANDARD_10_POLICIES`
  2. 每条 SID 命名：P1 AdminFull / P2 BucketOwner / P3 EditorWrite / P4 ViewerRO / P5 GuestList / P6 PublicRead / P7 DenyNonMFA / P8 DenyIP / P9 TagConditional / P10 VPCOnly
  3. 扩展 `MockIamProvider::evaluate_policies(policies, principal, action, resource) -> Result<bool, _>`：**Deny 优先短路**
- **本地 TR**：
  - **[rule] A-2.1**：10 × 3 = 30 场景单测（match/mismatch/prefix）；cargo test → 30/30 green
  - **[rule] A-2.2**：Deny 覆盖 Allow 测试：(Allow+Deny Delete) × Delete action = false；(Allow+No Deny) × Delete = true
  - **[rule] A-2.3**：P7 DenyNonMFA 在 MFA=false 时全部拒绝

### Task A-3：STS AssumeRole 硬 TTL=900 秒 + session_token verify

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T10-5 / AC-T10-6 / AC-T10-7
- **依赖**：Task 1, A-2（复用 HMAC）
- **工作内容**：
  1. 新增 `xuanji-domain-abstractions/src/sts_ttl900.rs`：`assume_role(role_id, session_name, duration_secs=900) -> StsCredentials`
  2. duration != 900 → Err；`session_token = base64(HMAC-SHA256(secret, role_id||session_name||expiration))`
  3. 实现 `StsCredentials::verify(&self, secret) -> bool`；MockIamProvider 凭据过期校验
- **本地 TR**：
  - **[rule] A-3.1**：duration=900 → expiration - now_ms ∈ [899000,901000]
  - **[rule] A-3.2**：duration=3600 → 返回 Err
  - **[rule] A-3.3**：verify(刚签发 token)=true；篡改 session_token 末字节 → false
  - **[rule] A-3.4**：伪造 expiration 前移 16min → authorize → Err("STS token expired")

### Task A-4：HTTP Quota 429 中间件

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T10-8 / AC-T10-9 / AC-T10-10 / AC-T10-23
- **依赖**：Task 1
- **工作内容**：
  1. 新增 `platform/backend-node/src/middleware/quota429.js`：`QuotaMiddleware(quotaProvider) -> (req,res,next)`
  2. Quota 超限 → HTTP 429 + `Retry-After: N` + `X-Quota-Used/Limit/Reset`；body `{code:"QuotaExceeded", ...}`
  3. 与现有 RateLimit 区分：Rate 头为 `X-RateLimit-*`，body code=`RateExceeded`；并发写入使用原子互斥（`Mutex` 或 sqlite 事务）
  4. 接入 `api-server.js` 的路由前链（所有 `/cloud/**` 与 `/graph/**` 前）
- **本地 TR**：
  - **[rule] A-4.1**：`max_bytes=1024` 写 2048 → res.statusCode===429
  - **[rule] A-4.2**：响应头 X-Quota-Used/Limit/Reset 全部存在且值非负整数
  - **[rule] A-4.3**：RateLimit 超限使用 X-RateLimit-* 头（≠ X-Quota）
  - **[rule] A-4.4**：并发 200 任务原子写 Quota，最终 used_bytes 无 double-count（Δ=0）

### Task A-5：等保三级 hash_chain 替换 dengbao_skeleton

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T10-11 / AC-T10-12 / AC-T10-22
- **依赖**：Task 1
- **工作内容**：
  1. 重写 `xuanji-standards/src/lib.rs`：`pub mod dengbao_hash_chain`（替换 skeleton）
  2. `HashChainBlock { idx, ts_ms, actor, action, resource, outcome, payload_hash, prev_hash, block_hash, hmac_signature }`
  3. 链式 hash：`block_hash = sha256(prev_hash || idx || ts_ms || actor || action || resource || outcome || payload_hash)`；`append` 原子；`verify` 全链校验返回 broken_at
  4. 示例 `examples/verify-hash-chain.rs`：读 JSON 文件 → 输出 `{blocks, integrity, broken_at?}`；exit 0 当且仅当 integrity=true
- **本地 TR**：
  - **[rule] A-5.1**：`HashChain::new(genesis)` 后 append 1000 条 → verify().integrity=true
  - **[rule] A-5.2**：篡改第 500 块 1 字节 → verify().integrity=false 且 broken_at=Some(500)
  - **[rule] A-5.3**：`cargo run --example verify-hash-chain chain.json` → stdout JSON 合法且 integrity 字段一致

### Task A-6：WORM SQLite 触发器 + S3 Object Lock 集成

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T10-14 / AC-T10-15 / AC-T10-20
- **依赖**：A-5
- **工作内容**：
  1. SQLite 表 `audit_chain` 创建时带 `INSTEAD OF UPDATE/DELETE` 触发器 → RAISE(ABORT, "WORM: readonly")
  2. `AuditContext.with_dengbao_chain(Arc<Mutex<HashChain>>)` 集成；写 Sink 失败整体 Err
  3. `GET /cloud/hash_chain/stats` 路由返回 `{blocks, last_ts, integrity}`
- **本地 TR**：
  - **[rule] A-6.1**：尝试 `DELETE FROM audit_chain` → SQLiteError (ABORT WORM)
  - **[rule] A-6.2**：AuditContext 写 100 条 → chain.len == 100
  - **[rule] A-6.3**：S3Sink mock_error 注入 → ctx.log 返回 Err 且本地 0 半写

### Task A-7：Node 侧 5 份测试脚本（≥60 tests 总合）

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T10-1 / AC-T10-2 / AC-T10-20 / AC-T10-24
- **依赖**：A-1..A-6 全部 + Rust 侧暴露 wasm 或 Node Addon（或 Node 侧通过 HTTP mock + 同构算法实现，优先用后者，零原生编译依赖）
- **工作内容**：创建：
  1. `test-t10-m4-lifecycle.js`（12 it：热→温→冷→回温→restore→stats→双类并行→类计数正确→空图安全→配置变更→错误路径→mock S3 迁移异常回滚）
  2. `test-t10-m4-iam10.js`（18 it：10 policy × 1 + 8 DenyOverride / 隐含 Deny / MFA / IP / Tag / VPC / prefix 匹配 / 不匹配）
  3. `test-t10-m4-sts.js`（10 it：TTL 900±1 / duration≠900 rej / verify ok / 篡改 fail / 过期 fail / 用 STS 凭据签名请求 / session 名哈希含入 / 不同 role id 区分 / 并发 50 assume 独立 / 空 session 名报错）
  4. `test-t10-m4-quota429.js`（12 it：字节超/对象数超/双超限/双超限头/retry-after 数值/并发计数一致/超限后再写一致失败/解除超限成功/Rate vs Quota 头区分/Rate 体 code 区分/匿名默认配额/租户级覆盖）
  5. `test-t10-m4-dengbao.js`（10 it：块 append / integrity / 篡改定位 / verify 工具 / WORM 触发器 / Audit 集成 / S3 失败不旁路 / 链式 hash 公式手工验证 Oracle / 空链 / 10 万块性能 <2s）
- **本地 TR**：
  - **[rule] A-7.1**：`mocha test-t10-m4-lifecycle.js` → 12/12
  - **[rule] A-7.2**：`mocha test-t10-m4-iam10.js` → 18/18
  - **[rule] A-7.3**：`mocha test-t10-m4-sts.js` → 10/10
  - **[rule] A-7.4**：`mocha test-t10-m4-quota429.js` → 12/12
  - **[rule] A-7.5**：`mocha test-t10-m4-dengbao.js` → 10/10
  - **[rule] A-7.6**：合计 62 tests ≥ 60（满足 AC-T10-24 score=2）

### Task A-8：T10 cloud artifacts JSON 产出

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-T10-18
- **依赖**：A-7
- **工作内容**：所有 T10 测试通过后，后处理脚本写：
  - `projects/t10-cloud-artifacts/lifecycle_report.json`
  - `iam10_matrix.json`
  - `sts_ttl900_report.json`
  - `quota429_limits.json`
  - `dengbao_chain_sample.json`
- **TR**：**[rule] A-8.1** 5 JSON 文件存在且均可 `JSON.parse`

### Task A-9：回归测试（既有基线不退化）

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T10-16 / AC-T10-17
- **依赖**：A-1..A-8
- **工作内容**：
  1. 运行 `mocha test-enterprise-10task-t10-cloud.js` → 基线
  2. 运行 `cargo test -p xuanji-domain-abstractions --test t1_t2_t3_red_green` → 0 red
- **TR**：
  - **[rule] A-9.1** t10 baseline 全部绿
  - **[rule] A-9.2** t1_t2_t3_red_green 0 red

### Task A-10：Clippy + 代码质量

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T10-19
- **依赖**：A-1..A-9
- **TR**：**[rule] A-10.1** `cargo clippy -p xuanji-standards -p xuanji-cloud-drive-s3 -p xuanji-domain-abstractions -p xuanji-expert -- -D warnings` exit 0

### Task A-11：T10 验收自测脚本（一键 run all 60 tests）

- **Status**：pending
- **Priority**：medium
- **依赖**：A-7, A-9
- **工作内容**：`scripts/run-t10-m4.ps1` + `.sh` 一键运行 A-7 + A-9 合计 ≥ 62 tests
- **TR**：**[rule] A-11.1** 一键 exit 0 且显示 `passed=62 failed=0`

### Task A-12：T10 自检 rubric 证据

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-T10-21 / AC-T10-22 / AC-T10-23
- **依赖**：A-2 / A-6 / A-4
- **工作内容**：在 `projects/t10-cloud-artifacts/rubric_evidence.md` 记录：
  - IAM 10 条每条的 SID 注释（≥3 分证据）
  - WORM 1000 块 integrity=true 截图 / 命令输出（≥3 分证据）
  - Quota429 5 维度头清单（≥4 分）

---

## Batch B：T11 关系图 R4

### Task B-1：新 crate 注册 + FlinkCdcSource

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T11-1
- **依赖**：Task 1
- **工作内容**：
  1. 创建 `platform/services/xuanji-graph-streams/`（Cargo.toml + src/lib.rs + src/flink_source.rs）
  2. `Cargo.toml` workspace member 注册；`FlinkCdcSource::new(Arc<CdcSource>)` + `next_blocking() + resume(offset)`
- **TR**：
  - **[rule] B-1.1** `cargo metadata --format-version=1` 中 `xuanji-graph-streams` 存在
  - **[rule] B-1.2** 100 event 本地 emit → 100 个 next_blocking() 返回 Some，第 101 个 None（挂起）

### Task B-2：10 万事件无丢重 harness

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T11-2 / AC-T11-15
- **依赖**：B-1
- **工作内容**：
  1. `src/bin/cdc_100k_harness.rs`：生成 70_000 Vertex + 30_000 Edge（每个 payload ≤512B，避免 OOM）
  2. idempotent_writer：按 `raft_index` upsert；最终输出 `{total_in,total_out,duplicates,lost}`
  3. 报告写入 `projects/t11-graph-artifacts/cdc_100k_report.json`
- **TR**：
  - **[rule] B-2.1** total_in == total_out == 100000
  - **[rule] B-2.2** lost == 0 ∧ duplicates_in_upsert == 0

### Task B-3：Spark Connector Reader + Writer（Rust 契约 + JS Harness）

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T11-3 / AC-T11-4 / AC-T11-5 / AC-T11-17
- **依赖**：Task 1
- **工作内容**：
  1. 新 crate `platform/services/xuanji-graph-spark/`：`src/graph_spark_reader.rs` / `graph_spark_writer.rs`
  2. `GraphSparkReader.paged_nodes(page,size) -> NodeFrame`；schema 含 `id:Long / label:String / type_:String / attr:Map`
  3. `GraphSparkWriter.bulk(df) -> Result<WrittenStats>`；幂等键 `(source,target,label)`
  4. Round-trip 测试写 2000/3000 再读回集合对称差为空
  5. Node 侧 `test-t11-r4-spark.js` 调用 HTTP bulk endpoint 模拟 Spark 行为
- **TR**：
  - **[rule] B-3.1** Rust roundtrip：set 差 = 0
  - **[rule] B-3.2** Node HTTP 模拟：`mocha test-t11-r4-spark.js` 8/8 pass

### Task B-4：Projection 20 算子实现

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T11-5 / AC-T11-6 / AC-T11-7 / AC-T11-16
- **依赖**：Task 1
- **工作内容**：
  1. `xuanji-graph-service/src/projection_20.rs`
  2. 20 具体函数命名 `proj_{filter_id}_{dir}_{hop}`：filter_id 5 类（type/community/attr/degree/label）× dir（in/out）× hop（1/2）= 20
  3. `PROJECTION_OPERATORS: [(&str, fn); 20]` 静态注册
  4. 200 节点手工 oracle 测试集
- **TR**：
  - **[rule] B-4.1** 注册表长度 = 20
  - **[rule] B-4.2** 20 算子 × oracle 哈希 = 20 match，0 mismatch
  - **[rule] B-4.3** and 组合 3 层交集大小正确

### Task B-5：AC-15 14 故障注入框架

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T11-8 / AC-T11-9 / AC-T11-18
- **依赖**：B-1, B-4
- **工作内容**：
  1. `xuanji-graph-service/src/ac15_faults.rs`：`FaultInjector` + 14 故障枚举（F1-F14 对应 spec 命名）
  2. 每个故障可注入至 `emit, source.next, writer.write, projection.eval`
  3. 质量门：lost==0 或 circuit_breaker=true 且 audit ∈ chain
- **TR**：
  - **[rule] B-5.1** 14 故障枚举完整（id 匹配 spec F1-F14）
  - **[rule] B-5.2** 14 × 3 runs = 42 场景全部通过质量门
  - **[rule] B-5.3** 报告 `projects/t11-graph-artifacts/fault_14_report.json` 可 parse

### Task B-6：CDC stream HTTP SSE 端点

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-T11-10
- **依赖**：B-1
- **工作内容**：
  1. `routes/graph.js` 新增 `GET /graph/cdc/stream`：`Content-Type: text/event-stream`；NDJSON chunked；支持 `?since_offset=`
- **TR**：**[rule] B-6.1** Node http 客户端接收 10 events 用时 <5s（mock emit 触发）

### Task B-7：T11 Mocha 测试套件（4 份 ≥40 tests 总合）

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T11-14
- **依赖**：B-2..B-6
- **工作内容**：新增：
  1. `test-t11-r4-cdc100k.js`（11 it：总计数 / lost=0 / dup=0 / 顺序单调 raft_index / resume offset=50000 / 重复订阅隔离 / 多 topic / lag_ms 计算 / flush 200ms 超时 / empty topic / 错误恢复）
  2. `test-t11-r4-spark.js`（8 it：写 nodes/edges / 分页读 / roundtrip set diff / 幂等键 / 空写 / 大页 / schema 字段 / schema 中文类型）
  3. `test-t11-r4-proj20.js`（12 it：5 filter 单测 × 类型各 1 = 5；方向 2；hop 2；and 组合；or 组合；空图；大图 1000 节点性能；ProjectionOperator 注册表=20；不识别 id 报错；反向投影；跨社区投影）
  4. `test-t11-r4-ac15.js`（10 it：14 故障至少命中每类 1 次断言 + 质量门 + 熔断器 + audit 入链 + 恢复后无丢 + 恢复后无重 + 慢请求超时 + disk_full 路径 + OOM drop + leader kill）
- **TR**：
  - **[rule] B-7.1** 11+8+12+10 = 41 tests 全部绿 ≥ 40（AC-T11-14 score=2）
  - **[rule] B-7.2** 单份 mocha 各自 exit 0

### Task B-8：T11 artifacts 产出

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-T11-12
- **依赖**：B-2, B-4, B-5
- **工作内容**：写 3 份 JSON + `projection_20_matrix.json`
- **TR**：**[rule] B-8.1** 4 JSON 文件存在

### Task B-9：基线回归（graph formulas + algorithm）

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T11-13
- **依赖**：B-1..B-8
- **TR**：
  - **[rule] B-9.1** `mocha test/test-graph-formulas.js test/test-enterprise-10task-t2-algorithm.js` → 全绿
  - **[rule] B-9.2** `cargo test -p xuanji-graph-storage -p xuanji-graph-service --test t7_r2_storage t9_r3_graph_service` → 0 失败

### Task B-10：T11 rubric 证据

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-T11-15 / AC-T11-16 / AC-T11-17 / AC-T11-18
- **依赖**：B-7, B-5, B-3
- **工作内容**：`projects/t11-graph-artifacts/rubric_evidence.md` 写入评分证据
- **TR**：**[rule] B-10.1** 4 rubric 均声明 score ≥ threshold（≥4+3+3+3）

### Task B-11：T11 一键验收脚本

- **Status**：pending
- **Priority**：medium
- **依赖**：B-7, B-9
- **TR**：**[rule] B-11.1** `scripts/run-t11-r4.ps1` exit 0 显示 `total=41`

---

## Batch C：T17 官方 SDK（×3 语言）

### Task C-1：Rust SDK Cloud（完整版）

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T17-1 / AC-T17-12
- **依赖**：Task 1
- **工作内容**：
  1. 重写 `xuanji-sdk-cloud/src/lib.rs`：`CloudClient { new(), list_buckets(), put/get/del, MPU, sts_assume_role, iam_evaluate, healthz }`
  2. SigV4 签名复用 `xuanji-standards::sigv4`（或同算法 port 版，避免循环依赖）
  3. `examples/ex_c1.rs` 到 `ex_c15.rs` 15 个示例
- **TR**：
  - **[rule] C-1.1** pub fn 数 ≥ 10（非 trait）
  - **[rule] C-1.2** 15 examples 文件存在；`cargo build --examples` 成功
  - **[rule] C-1.3** ≥15 单测（mock axum 服务器）通过

### Task C-2：Rust SDK Graph（完整版）

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T17-1 / AC-T17-12
- **依赖**：Task 1
- **工作内容**：
  1. 重写 `xuanji-sdk-graph/src/lib.rs`：`GraphClient { get_graph, stats, centrality, communities, pagerank, bulk, project(...), cdc_stream, search }`
  2. `examples/ex_g1.rs` 到 `ex_g15.rs` 15 个示例
- **TR**：
  - **[rule] C-2.1** pub fn ≥ 9
  - **[rule] C-2.2** 15 examples 文件存在；cargo build --examples 成功
  - **[rule] C-2.3** ≥15 单测通过（合计 Rust ≥30，AC-T17-8 满足）

### Task C-3：Node.js SDK Cloud + Graph

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T17-2 / AC-T17-5 / AC-T17-9 / AC-T17-13
- **依赖**：Task 1
- **工作内容**：
  1. `xuanji-sdk-cloud/`：index.js（class CloudClient，零外部 aws-sdk 依赖；纯 `https` + 自实现 SigV4）+ package.json
  2. `xuanji-sdk-graph/`：同构 GraphClient
  3. `examples/cloud/ex_c{1-15}.js`（15）+ `examples/graph/ex_g{1-15}.js`（15）
  4. `test/cloud/*.js` ≥15 + `test/graph/*.js` ≥15（nock 离线 mock）
- **TR**：
  - **[rule] C-3.1** 2 package.json 存在且 main = index.js
  - **[rule] C-3.2** 30 examples 存在
  - **[rule] C-3.3** `mocha platform/sdk/nodejs/test/**/*.js` → ≥ 30 passed

### Task C-4：Python SDK Cloud + Graph

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T17-3 / AC-T17-6 / AC-T17-10 / AC-T17-14
- **依赖**：Task 1
- **工作内容**：
  1. `xuanji_sdk_cloud/`：`client.py`（基于标准库 `urllib` + `hmac` + `hashlib`，零第三方依赖）+ `__init__.py` + `pyproject.toml`（setuptools）
  2. `xuanji_sdk_graph/`：同构
  3. `examples/cloud/ex_c{1-15}.py` 15 + `examples/graph/ex_g{1-15}.py` 15
  4. `test/test_cloud_*.py` ≥10 + `test/test_graph_*.py` ≥10（unittest + mock）
- **TR**：
  - **[rule] C-4.1** `python -c "import xuanji_sdk_cloud; print(xuanji_sdk_cloud.__version__)"` 成功
  - **[rule] C-4.2** 30 examples 存在
  - **[rule] C-4.3** `python -m unittest discover -s platform/sdk/python/test -v` → ≥ 20 passed

### Task C-5：90 示例计数脚本 + 三语言一致性 JSON SoT

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T17-7 / AC-T17-11 / AC-T17-17
- **依赖**：C-1..C-4
- **工作内容**：
  1. 新增 `scripts/count_sdk_artifacts.js`：`node scripts/count_sdk_artifacts.js` 输出 JSON：`{examples_rs, examples_js, examples_py, tests_rs, tests_js, tests_py, total_examples, total_tests}`
  2. 新增 `platform/sdk/specs/example_specs.json`（8 场景共享 SoT），脚本 tri-generate 避免 drift（可选：实际生成示例文件）
- **TR**：
  - **[rule] C-5.1** total_examples ≥ 90（30 × 3）
  - **[rule] C-5.2** total_tests ≥ 80（30+30+20）
  - **[rule] C-5.3** 三语言 C1/G1/G8 场景输出 JSON schema 字段 90% 匹配（忽略 lang 专属）

### Task C-6：Rust SDK 离线网络证明

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T17-12
- **依赖**：C-1, C-2
- **TR**：**[rule] C-6.1** `CARGO_NET_OFFLINE=1 cargo test -p xuanji-sdk-cloud -p xuanji-sdk-graph` passed ≥ 25

### Task C-7：Python/Node 离线证明

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-T17-13 / AC-T17-14
- **依赖**：C-3, C-4
- **TR**：
  - **[rule] C-7.1** Node 测试 `test-offline-all.js`（断网模拟 + nock）0 network 访问
  - **[rule] C-7.2** Python 测试本地 http.server mock 运行；无 urllib 外部请求

### Task C-8：SDK README 6 要素（×6）

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-T17-18
- **依赖**：C-1..C-4
- **工作内容**：每个 SDK crate/包 README：安装/快速开始/配置/示例链接/错误码/FAQ 6 要素
- **TR**：**[rule] C-8.1** 6 份 README 均有 6 要素章节

### Task C-9：项目数据目录产物

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-T17-15
- **依赖**：C-1..C-4
- **工作内容**：每个语言运行 5 个示例 → 输出 JSON 至 `projects/t17-sdk-examples/`
- **TR**：**[rule] C-9.1** `projects/t17-sdk-examples/` 下 ≥15 JSON（5 × 3 语言）

### Task C-10：T17 合计自测一键脚本

- **Status**：pending
- **Priority**：medium
- **依赖**：C-5
- **TR**：**[rule] C-10.1** `scripts/run-t17-sdk.ps1` 输出 examples=90 tests≥80，exit 0

### Task C-11：Rust Clippy 零警告

- **Status**：pending
- **Priority**：high
- **依赖**：C-1, C-2
- **TR**：**[rule] C-11.1** `cargo clippy -p xuanji-sdk-cloud -p xuanji-sdk-graph -- -D warnings` exit 0

### Task C-12：全量测试汇总脚本

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-T17-16
- **依赖**：C-1..C-11
- **TR**：
  - **[rule] C-12.1** 实测 rust + node + python 通过数 ≥ 80（脚本输出断言）

---

## Batch E：运维批次（T12/T13/T15/T18）

### Task E-1：T12 Helm DR Chart + 模板

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-E-1
- **依赖**：Task 1
- **TR**：**[rule] E-1.1** Chart.yaml + values.yaml + 4+ templates 存在；helm lint 或 yaml parse 通过

### Task E-2：T12 DR Failover 脚本

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-E-2
- **依赖**：Task 1
- **TR**：
  - **[rule] E-2.1** bash -n 通过
  - **[rule] E-2.2** PowerShell AST parse Errors.Count=0

### Task E-3：T13 信创适配清单 6 适配组合

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-E-3
- **依赖**：Task 1
- **TR**：**[rule] E-3.1** 文档存在 ≥6 行 ×4 列

### Task E-4：T13 部署手册 7 章

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-E-4
- **依赖**：Task 1
- **TR**：**[rule] E-4.1** 7 章 × 每章 ≥3 小节齐全；≥2 张 Mermaid/ASCII 图

### Task E-5：T15 HA SLA 99.95% 断言 + 容量 Rubric

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-E-5 / AC-E-6 / AC-E-7
- **依赖**：Task 1
- **工作内容**：增强 `test-enterprise-slo-capacity-tco.js` 含 `sla_9995_ok=true`；`scripts/tco-rubric.js` 生成 4 × 3 表
- **TR**：
  - **[rule] E-5.1** mocha 断言通过
  - **[rule] E-5.2** tco-rubric.js gen → JSON 可 parse，4 档齐全

### Task E-6：T18 8 阶段 trace 中间件

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-E-7 / AC-E-8 / AC-E-9
- **依赖**：Task 1
- **工作内容**：新增 `src/middleware/trace_8phase.js`；8 span Accept/IAM/Quota/Plan/Storage/Algorithm/Audit/Response；`GET /system/traces/:id` 路由
- **TR**：
  - **[rule] E-6.1** mocha test-three-flows-trace-e2e.js：8 span 齐全
  - **[rule] E-6.2** `GET /system/traces/:id` spans.length ≥ 8
  - **[rule] E-6.3** grep 证明每阶段有 tracing 埋点代码

### Task E-7：T18 trace 可视化 JSON 导出

- **Status**：pending
- **Priority**：medium
- **工作内容**：trace_id 查 span 层级（parent_id）正确
- **TR**：**[rule] E-7.1** 8 span parent 链路：Response → Audit → Algorithm → Storage → Plan → Quota → IAM → Accept（或等价正确拓扑）

### Task E-8：E 批次质量 & 脚本幂等（rubric）

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-E-10
- **依赖**：E-1..E-7
- **TR**：
  - **[rubric 0-3, threshold=2] E-8.1**：0=缺脚本；1=语法过但不幂等；2=幂等+dry-run；3=幂等+dry-run+CI hook。**score≥2**

---

## Batch F：运维批次（T19/T20 Final Gate）

### Task F-1：T19 全量回归脚本

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-F-1 / AC-F-9
- **依赖**：Task 1
- **工作内容**：`scripts/run-full-regression.ps1` + `.sh`；执行 cargo test workspace + mocha 所有 + frontend vitest；写入 `projects/t19-regression-report/last_summary.json`
- **TR**：
  - **[rule] F-1.1** 语法通过（bash -n + ps1 AST）
  - **[rule] F-1.2** last_summary.json 字段齐全

### Task F-2：count_test_specs 增强 + ≥706 断言

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-F-2
- **依赖**：Batch A/B/C 全部完成
- **工作内容**：增强 `scripts/count_test_specs.js` 统计 Rust 单元、mocha、vitest、playwright；断言 `total >= 706`
- **TR**：**[rule] F-2.1** 脚本输出 total ≥ 706

### Task F-3：T20 Helm 一键 Chart

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-F-3 / AC-F-8
- **依赖**：Task 1
- **TR**：
  - **[rule] F-3.1** `deploy/helm/xuanji/Chart.yaml` 存在
  - **[rule] F-3.2** 渲染模板 YAML ≥10 Deployment + ≥1 Service + ≥1 ConfigMap

### Task F-4：T20 灰度脚本 4 阶段 + values 文件

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-F-4
- **依赖**：Task 1
- **TR**：
  - **[rule] F-4.1** 4 canary 脚本存在 + 4 values-canary-*.yaml 存在
  - **[rule] F-4.2** values 权重分别为 1/10/50/100

### Task F-5：Warmup + 95% 成功率门 + metrics

- **Status**：pending
- **Priority**：high
- **对应 AC**：AC-F-5 / AC-F-6 / AC-F-7
- **依赖**：F-4
- **工作内容**：每阶段 warmup 100 /healthz + 10 /ai/engine/metrics；success≥95%；metrics 写入 projects
- **TR**：
  - **[rule] F-5.1** 4 阶段 metrics JSON 存在
  - **[rule] F-5.2** phase4 error_rate < 0.001（0.1%）

### Task F-6：T20 可观察性进度条 + HTML 摘要

- **Status**：pending
- **Priority**：medium
- **对应 AC**：AC-F-10
- **依赖**：F-1..F-5
- **TR**：
  - **[rubric 0-4, threshold=3] F-6.1**：0=无进度；1=文本计数；2=进度条+JSON；3=2+console summary；4=3+HTML report。**score≥3**

---

## Review 触发门（Gate G-1..G-4 必须过才能开 Review）

| Gate | 验证 | 对应任务 |
|------|------|---------|
| G-1 | T0 18 烟测 18/18 PASS | A-11/B-11 脚本中嵌入 |
| G-2 | 7×8 算法对账 = 56 绿 | B-9 基线回归 |
| G-3 | 棘轮不下降（≥ last meta_latest.json） | F-2 |
| G-4 | 四闸门（代码/文档/测试/安全）签名齐全 | 27 §T6 |

---

## 状态跟踪表总览（初始状态全部 pending）

本 tasks.md 包含：Setup 1 项 + BatchA(T10) 12 + BatchB(T11) 11 + BatchC(T17) 12 + BatchE 8 + BatchF 6 + Gate 4 = 共 **54 项任务**。  
完成标准 = 所有任务 status ∈ {completed, user-approved cancelled}，且每条 AC 的 rule/rubric 证据齐全，最终 Review pass。

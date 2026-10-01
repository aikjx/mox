# Clippy 技术债清单 — gateway / alliance 域

> 生成日期：2026-09-29
> 工具：`cargo clippy`（nightly 1.98），`--message-format=json` 解析
> 性质：**只读静态分析，未修改任何源码**
> 范围方法：`cargo clippy -p mox-platform-gateway-svc` 会把传递依赖 crate 的 warning 一并输出；本清单按文件路径归属拆分，只把路径属于该 crate 自身 `src/` 的计为"自身债"，依赖 crate 的另列附表。

---

## 一、结论

| 范围 | clippy warning | 说明 |
|---|---|---|
| 本轮 alliance 新增/修改核心 crate（scheduler-core / registry-core / registry-svc / executor-* / scheduler-svc / http-sdk） | **0** | `approval_gate.rs` 上轮已修复唯一 1 个 `if_same_then_else`，现为零 warning |
| **mox-platform-gateway-svc 自身源码** | **34** | 全部为**历史基线**，无一条来自本轮 alliance 新增逻辑 |
| 传递依赖 crate（kg-algo / kg-service / kg-storage / ai-expert / flow-process / cloud-foundation） | 86 | 编译 gateway 时连带输出，归属各 crate，不在 gateway 名下 |

> 关键判断：本轮专家联盟安全改动（鉴权中间件 / 出站令牌 / 审计 actor / SQLite 迁移）**未引入任何 clippy warning**；其风险点是**授权合规与 fail-open 默认策略**，属于安全评审范畴，不在本静态风格清单内。

---

## 二、gateway-svc 自身 34 条（按 lint 类型）

| 数量 | lint | 风险/处置建议 | 建议批次 |
|---|---|---|---|
| 14 | `empty_line_after_doc_comments` | 纯格式（doc 注释后多空行），`cargo clippy --fix` 可自动修 | P1 零风险批量 |
| 6 | `field_reassign_with_default` | 可改为结构体更新语法 `..Default::default()`，机械重构 | P1 |
| 4 | `unnecessary_sort_by` | 改用 `sort_by_key`，行为等价 | P1 |
| 3 | `result_large_err` | Result::Err 类型过大，建议 Box 或精简错误枚举——**需评估，非机械** | P2 |
| 2 | `format_in_format_args` | 嵌套 format! 可合并 | P1 |
| 1 | `if_same_then_else` | **可能是真实逻辑缺陷**（两分支相同），须人工确认 monitor.rs:430 | **P0 人工** |
| 1 | `manual_checked_ops` | 改用 checked_add 系列，注意语义 | P2 |
| 1 | `manual_clamp` | 改 `.clamp()`，等价 | P1 |
| 1 | `doc_lazy_continuation` | 文档注释格式 | P1 |
| 1 | `needless_lifetimes` | 可省略生命周期标注 | P1 |

---

## 三、gateway-svc 自身逐条明细（文件 : 行 : lint）

### alliance 模块（专家联盟，9 条）
- `src/alliance/experts_collaboration.rs:479` manual_clamp
- `src/alliance/experts_collaboration.rs:556` format_in_format_args
- `src/alliance/experts_db.rs:95` doc_lazy_continuation
- `src/alliance/experts_graph.rs:375` unnecessary_sort_by
- `src/alliance/experts_registry.rs:532` unnecessary_sort_by
- `src/alliance/experts_session.rs:250` manual_checked_ops
- `src/alliance/experts_session.rs:323` unnecessary_sort_by

### system 模块（13 条，多为格式）
- `src/system/dept.rs:40`、`:71` empty_line_after_doc_comments
- `src/system/role.rs:41`、`:71` empty_line_after_doc_comments
- `src/system/security.rs:38`、`:56`、`:85`、`:105`、`:123` empty_line_after_doc_comments
- `src/system/mod.rs:75`、`:94`、`:104`、`:168` empty_line_after_doc_comments
- `src/system/mod.rs:146` match_like_matches_macro
- `src/system/permission.rs:39` empty_line_after_doc_comments
- `src/system/mfa.rs:26`（见明细行）、`:258` needless_lifetimes

### 其它业务模块（12 条）
- `src/api_permission/api.rs:476` field_reassign_with_default
- `src/document/api.rs:269` field_reassign_with_default
- `src/enterprise/api_response.rs:187`、`:195`、`:207` result_large_err
- `src/file_storage/api.rs:132` field_reassign_with_default
- `src/message_center/api.rs:195` field_reassign_with_default
- `src/scheduler/api.rs:660` field_reassign_with_default
- `src/system_config/api.rs:478` field_reassign_with_default
- `src/monitor.rs:430` **if_same_then_else（P0 人工确认是否逻辑缺陷）**
- `src/melody.rs:105` format_in_format_args

> 注：以上行号取自 2026-09-29 工作树当前状态；工作区存在既存未提交改动，修复前需以最新代码重新 `cargo clippy` 定位。

---

## 四、建议处置批次

- **P0（先人工看）**：`monitor.rs:430 if_same_then_else`——两分支体相同可能是复制粘贴漏改，需读代码确认是冗余还是 bug。
- **P1（零风险、可自动/机械修）**：empty_line_after_doc_comments、field_reassign_with_default、unnecessary_sort_by、format_in_format_args、manual_clamp、doc_lazy_continuation、needless_lifetimes、match_like_matches_macro。
  - 可先 `cargo clippy --fix --allow-dirty -p mox-platform-gateway-svc` 让编译器自动应用，再 `cargo test -p mox-platform-gateway-svc` 全量回归；**但工作区当前 dirty，必须先确认未提交改动归属后再跑 --fix，避免混入无关变更。**
- **P2（需评审）**：result_large_err（改错误类型签名，影响调用方）、manual_checked_ops（确认溢出语义）。

---

## 五、附表：传递依赖 crate 的 86 条（不归 gateway，仅记录）

| crate | 条数 | 主要类型 |
|---|---|---|
| mox-kg-algo-core | 54 | needless_range_loop（图算法索引循环，集中在 graph_embedding/centrality/community/csr）、unnecessary_sort_by |
| mox-kg-service-svc | 16 | manual_clamp（optimizer）、needless_range_loop、if_same_then_else、while_let_loop、manual_strip |
| mox-ai-expert-core | 5 | result_large_err（engine/mod.rs）、manual_clamp |
| mox-kg-storage-svc | 4 | match_like_matches_macro、should_implement_trait、ptr_arg |
| mox-flow-unified-process-core | 3 | unnecessary_sort_by |
| mox-ai-expert-svc | 1 | manual_checked_ops |
| mox-cloud-foundation | 1 | unnecessary_unwrap（evaluator.rs:46，建议优先看，IAM 策略评估路径） |
| mox-ai-expert-svc/mox-ai-expert-core 其余 | 2 | 见 JSON 原始输出 |

> 其中建议**安全相关优先关注**：`mox-cloud-foundation/src/iam_standard_policies/evaluator.rs:46 unnecessary_unwrap`（IAM 评估路径上的 unwrap，理论上可能 panic）与 `mox-kg-service-svc/src/graph_query_engine.rs` 的 2 个 if_same_then_else。这两处虽不在 alliance 改动范围，但属潜在健壮性问题，建议另立任务。

---

## 六、复现命令

```powershell
cd D:\a10\aikjx\gitcode\infotopograph\platform
# 人类可读
cargo clippy -p mox-platform-gateway-svc
# 机器可解析（按 message.spans[].file_name 归属 crate）
cargo clippy -p mox-platform-gateway-svc --message-format=json
```

---

*本清单为只读分析产物，未对任何源码执行修改；是否修复、何时修复、按何批次，均待负责人决策。*

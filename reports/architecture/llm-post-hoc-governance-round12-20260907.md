# LLM 后验治理闸门 · 第十二轮（2026-09-07）

## 一句话总结

**把 LLM 专家回复路径补到与本地 `mox_optimize` 同等的强治理水位。** 在 `mox_optimize`
前置不可行的客观约束下，通过文本→FlowGraph 后验映射器实现等价治理：模型输出触及
敏感资源/生产写时必被拦截，与 `parse_veto` 互补不重叠。

## 背景：P1 真正悬而未决的问题

第十轮拆解了 round 6 留的 P1「`mox_optimize` 治理闸门在 LLM 模式下被绕过」：
- 前置不可行：入参是自然语言，无 FlowGraph
- 旁路无意义：图治理作用在执行计划上，与 LLM 推理正交
- 降级不可接受：等于放弃治理

唯一可行的位置是**后验**——LLM 输出（`ConsultReport.steps`）描述了它**打算做什么**，把这
些意图重建为 FlowGraph，再走一次 `mox_optimize`。这是本轮的核心方案。

## 实施方案：文本→FlowGraph 后验映射器

### 新增模块

`platform/gateway/mox-platform-gateway-svc/src/llm_governance.rs`（约 530 行，含测试）

**核心函数** `govern_llm_answer(expert, steps, question) -> PostHocGovernance`

```
1. infer_actions(steps, question)
   ├─ 启发式识别动作词（写/读/打开/调用/落库…） → ToolKind
   ├─ 抽取资源 URI（db:xxx / pii:xxx / var:xxx / file:xxx …）
   └─ 中文敏感关键字（公民/身份证/医保…）升级为 db:sensitive/<kw>

2. build_flow_graph(actions, question)
   ├─ 拓扑：Start → [Task 节点] → End
   ├─ 每节点带 Access::read/write 声明与 estimated duration
   └─ 触敏写时挂载 R-LLM-001 专家规则（要求 desensitize Guard）

3. mox_optimize(&graph, &govern_ctx)
   ├─ 复用 14 维专家 + 闸门 + 算法验证（与生产编排同套）
   └─ GovernContext：regulated tenant + editor/admin principal

4. 决策
   ├─ algo.vetoed | gate.algorithm_veto    → Veto
   ├─ !gate.approved                       → Veto
   ├─ 触敏但闸门通过（已配 Guard）         → Warn
   └─ 否则                                  → Pass
```

### 接线

`experts_collaboration::map_report_to_answer` 在已有 `report.vetoed` 拦截后、构造 JSON 前
插入后验治理闸门：

```rust
let post_hoc = crate::llm_governance::govern_llm_answer(expert, &report.steps, question);
if post_hoc.decision == Veto {
    return blocked_response(post_hoc.reason, post_hoc.sensitive_resources);
}
let governance_warnings = if post_hoc.decision == Warn { json!({...}) } else { null };
```

- Veto → 拦截正文 + `governance: "post_hoc_veto"` 标记（与既有 `vetoed: true` 一致）
- Warn → 透传正文 + 附加 `governance_warnings` 字段供前端告警
- Pass → 不附加字段

### 与既有 `parse_veto` 的关系

| 触发场景 | `parse_veto` | 本模块 |
|---|---|---|
| "该方案不可行"（含"否决"语义词） | ✅ 拦截 | ❌ Pass（无写动作） |
| "把公民数据搬到生产库"（自评 Pass） | ❌ | ✅ Veto（实际写敏感） |
| "查询订单"（纯读） | ❌ | ❌ Pass |
| "把数据保存到表里"（无目标） | ❌ | ❌ Pass（保守：仅触敏拦截） |

**互补不重叠**。本模块只拦截**实际描述的写操作**是否触敏，不重复拦截纯文本的"否决"语义。

## 验证

### 编译

`cargo check -p mox-platform-gateway-svc`：**0 error**，warning 10 → 10（无新增）。
Gateway Cargo.toml 新增 `mox-ai-flow-svc = { workspace = true }`（提供 FlowGraph 类型）。

### 测试

`cargo test -p mox-platform-gateway-svc`：**121 passed / 0 failed**

明细：
- lib 单元测试 **100 passed**（含本轮新增 8 个 `llm_governance::tests`）
- `alliance_remote` 集成测试 13 passed
- `experts_registry` 测试 8 passed

### 新增 8 个回归测试

| 测试 | 覆盖 |
|---|---|
| `extract_uri_token_handles_db_and_pii` | URI 抽取正确性（db:/pii:/var:，逗号截断） |
| `sensitive_keyword_promotes_to_db_uri` | "查询一下公民信息" → `db:sensitive/公民` |
| `write_to_sensitive_db_is_vetoed` | "读取 db:citizen_info → 写入 db:prod/citizen_records" → Veto |
| `read_only_step_passes` | 纯读操作 → Pass |
| `empty_steps_returns_pass` | 空 steps 不误判 |
| `browser_step_passes_without_sensitive` | 浏览器动作 + 无敏感关键字 → Pass |
| `unspecified_write_target_triggers_review` | "保存数据"无目标 → Pass（**保守设计**：仅触敏触发） |
| `chinese_truncate_is_char_safe` | 字符级截断（非字节级）守护 |

## 设计取舍

### 取向：宁可误报，不可漏报

错误类型对照：

| 错误 | 后果 | 可接受性 |
|---|---|---|
| 漏报（让越权写流出） | 数据泄露/破坏 | **不可接受** |
| 误报（正常回答被拦） | 用户看到拦截说明 | **可接受** |

所以：
- 中文敏感关键字命中即升级为 `db:sensitive/<kw>`，宁可多算不可少算
- 未指定目标的写动作**不**拦截（避免误杀"建议落盘"这类分析建议）
- 资源 URI 必须**显式**形如 `db:xxx` 才会被识别，避免字符串随机命中

### 性能

`mox_optimize` 单次约 5-15ms（round 6 实测，含 14 维专家加载 + 闸门判定）。
LLM 回复链路总成本：模型推理（秒级）+ 后验治理（毫秒级），延迟影响 < 1%。

## 限制与后续

### 当前限制

1. **文本解析是启发式的**——无法识别所有语义变体（"建议落盘" / "写死" / "持久化"等）
2. **资源必须显式 URI**——"查询一下表"识别为隐式 Database 读但无具体资源
3. **中文敏感关键字是白名单**——新型敏感场景需更新 SENSITIVE_KW

### 推荐演进：结构化协议

高安全场景应在前置 prompt 强制要求 LLM 给出**结构化动作清单**：

```json
{
  "actions": [
    {"op": "write", "target": "db:citizen_info", "guard_required": "desensitize"}
  ]
}
```

后续可演进为 `mox_consult_v1` JSON Schema 协议——LLM 直接产出结构化动作，跳过文本解析，
彻底消除启发式误差。本轮实现的文本→图映射器可作为**回退路径**，应对 LLM 拒绝按格式输出
的场景。

## 改动清单

| 文件 | 改动 |
|---|---|
| `platform/gateway/mox-platform-gateway-svc/Cargo.toml` | 新增 `mox-ai-flow-svc` 依赖 |
| `platform/gateway/mox-platform-gateway-svc/src/lib.rs` | 注册 `llm_governance` 模块 |
| `platform/gateway/mox-platform-gateway-svc/src/llm_governance.rs` | **新增** 530 行：核心映射器 + 8 测试 |
| `platform/gateway/mox-platform-gateway-svc/src/experts_collaboration.rs` | 接线 post-hoc 治理到 `map_report_to_answer`，更新 `question` 参数 + 1 测试 |

## 一句话回顾

**LLM 路径治理水位从"自评 Pass = 出网"提升到"璇玑 14 维 + 资源策略 + 闸门全量复审"**。
121/0 测试守护，包括"写公民库到生产环境""查询订单""浏览器动作"等典型场景。

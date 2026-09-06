# 开发专家联盟 · 第十一轮：gateway 代码质量与未接线能力盘点（2026-09-06）

> 承接第十轮治理闸门专项。本轮处理 gateway（8080 唯一入口）的存量编译告警，
> 并在排查过程中识别出 3 处「代码在、能力不在」的未接线缺口。

## 一、结论

`cargo check -p mox-platform-gateway-svc`：**25 条 warning → 10 条**。

但**价值不在数字下降，而在分类处置**——无脑清零反而会破坏接口：

| 类别 | 数量 | 处置 | 理由 |
|---|---|---|---|
| 真·未使用 import | 13 | **删除** | 纯噪音 |
| 重复计算的公式 | 1 | **重构去重** | 两处各写一遍，易改漏一处 |
| 未接线能力（函数/参数） | 3 | **保留 + 写明状态** | 是功能缺口信号，静默即遗忘 |
| 契约字段（never read） | 7 | **不动** | 删了会静默破坏 API |
| 跨 target 使用的 import | 1 | **保留** | lib 未用但 tests 需要 |

## 二、真实修复：`rate_limit` 的重复公式

`RateLimiter::new()` 算出 `max_tokens` / `refill_rate` 后**从未使用**（死变量），
而 `check()` 里把**同一个公式**又写了一遍：

```rust
// new()  —— 算完即弃
let max_tokens  = (config.max_requests + config.burst) as f64;
let refill_rate = config.max_requests as f64 / config.window_secs as f64;
// check() —— 同式重算
.or_insert_with(|| TokenBucket::new(
    (self.config.max_requests + self.config.burst) as f64,
    self.config.max_requests as f64 / self.config.window_secs as f64,
));
```

改为收敛到 `fn bucket_params(&self) -> (f64, f64)`，两处共用。
**行为不变**——此前两个死变量不参与任何计算，删掉不影响限流语义。

> 注：`cargo fix` 对这类告警的默认处理是加 `_` 前缀（`let _max_tokens = ...`），
> 那是**把「算错了地方」变成「故意不算」**，重复公式依然存在。故手工重构。

## 三、刻意保留的 3 处未接线能力

这 3 处不删、也不加 `#[allow]`。它们不是死代码，是**「代码在、能力不在」的缺口**，
warning 是目前唯一机器可检的提醒，静默掉就会永远尘封。

| 位置 | 缺口 | 实际后果 |
|---|---|---|
| `workspace.rs` `save_workspace_history` | 历史**只读不写**：`new()` 从磁盘加载，全模块无任何追加记录的调用点 | 历史接口恒返回磁盘存量数据，全新部署时恒为空 |
| `misc.rs` `save_misc_data` | tasks / projects 只有 `.lock().clone()` 读取，无增删改写入点 | misc 数据是只读静态数据，**没有 CRUD 接口** |
| `experts_dispatcher.rs` `dispatch_task(task_type, …)` | `task_type` 未参与调度，匹配仅靠 `input` 关键词 | 不同任务类型（代码生成 vs 数据分析）走同一套匹配逻辑 |

三者均已就地补写文档注释说明状态与补齐方向。

补充说明第二项的性质：它是**功能未接线，不是数据丢失**——
无写入即无可丢失，不应按「持久化缺失」处理。

## 四、为什么 7 个 never-read 字段一个都不能删

典型例子 `experts_collaboration.rs`：

```rust
#[derive(Debug, Deserialize)]
struct ConsultBody {
    question: String,
    #[serde(default)]
    context: Option<String>,   // never read —— 但这是客户端传入的字段
    session_id: Option<String>,
    #[serde(default)]
    priority: Option<String>,  // 同上
}
```

这是**请求体**。删掉 `context` / `priority` 的后果是：
客户端照常传参，服务端**静默丢弃**——不报错、不 400，只是行为悄悄变了。
这是最难排查的一类破坏。`never read` 只说明本 crate 没读，
不代表调用方没传。

同类字段：`workspace.rs` 的 `viewport` / `description`、
`projects_ext.rs` 的 `context`、`experts_collaboration.rs` 的 `stance` / `history`、
`experts_graph.rs` 的 `constraints`。

## 五、验证

| 项 | 结果 |
|---|---|
| `cargo check -p mox-platform-gateway-svc` | **0 error**，warning 25 → 10（剩余均为上表刻意保留项） |
| `cargo test -p mox-platform-gateway-svc` | **113 passed / 0 failed**（92 + 13 + 8） |

## 六、待办（按价值排序）

1. **补齐 workspace 历史写入路径** —— 需先定义「什么操作记一条历史」的语义，属产品设计决策
2. **misc CRUD 接口** —— 决定 tasks / projects 是否该支持增删改；若不该，则删掉 `save_misc_data`
3. **`task_type` 接入调度** —— 需定义「任务类型 → 专家领域」映射规则
4. 上述三项一旦接线，对应 warning 会自动消失——**它们本就是待办清单**

## 七、方法论沉淀

编译器 `dead_code` / `unused_*` 告警要**先分类再动手**，不能一律清零：

- 指向**请求/响应体字段** → 极可能是契约，删了静默破坏 API，不动
- 指向**「语义上应该被调用」的函数** → 是未接线信号（与上一轮 `norm_fusion` 同型），
  查清后**接线或标注**，不要删
- 指向**重复计算** → 重构去重，而不是让工具加 `_` 前缀掩盖
- 只有**纯未使用 import** 才适合直接删除

# 开发专家联盟 · 治理闸门专项（第十轮 · 2026-09-06）

> 承接第九轮「完成度盘点」（13 crate 编译 0 error、307 测试 0 失败）。
> 本轮聚焦第九轮遗留的**唯一未决 P1**：本地璇玑 `mox_optimize` 治理闸门在 LLM 模式下被绕过。

## 一、结论

P1 的原始描述是「是否需要保留前置治理，待用户确认」。**深入代码后这个问题不成立**——
前置治理在 LLM 咨询路径上物理不可行（`mox_optimize` 的入参是 `FlowGraph`，
而专家咨询的入参是一句自然语言，没有图可治）。

真正的缺陷在别处，且比原描述严重：**治理契约 `ConsultReport.vetoed`
写明「veto=true 时下游应强制拦截」，但唯一的消费方 gateway 只是把标志透传给前端，
被否决的正文照常出网，还会经 `fuse_answers` 污染融合结论。**

本轮修复 3 处，并给出一个可推翻的推荐方案。

| # | 缺陷 | 位置 | 性质 |
|---|---|---|---|
| D1 | `vetoed=true` 不拦截，正文照常返回 | gateway `map_report_to_answer` | 契约违反（P0） |
| D2 | 被否决回复仍参与融合，可成为主导观点 | gateway `fuse_answers` | 契约违反（P0） |
| D3 | `parse_veto` 全文裸子串匹配，误判率极高 | ai-expert-svc `parse_veto` | 标志不可信（P1） |

## 二、证据链

### D1 — 拦截契约被无视

`mox-ai-expert-proto::ConsultReport` 的字段文档：

```rust
/// 是否被算法验证或治理闸门否决（veto=true 时下游应强制拦截）
pub vetoed: bool,
```

gateway 的实际实现（修复前）：

```rust
json!({
    "analysis": analysis,        // 被否决的正文，原样返回
    "solution": solution,
    "confidence": report.score,
    "vetoed": report.vetoed,     // 只透传标志，不拦截
    "veto_reason": report.reason,
})
```

### D2 — 被否决内容污染融合结论

`fuse_answers` 按 `match_score × avg_rating` 选 `dominant_view`，**不看 `vetoed`**。
一条被否决的回复若匹配分最高，会成为对外输出的主导方案。

### D3 — 否决标志本身不可信

LLM 路径的 `vetoed` 来自 `parse_veto(&r.final_answer)`，其兜底分支（修复前）：

```rust
if answer.contains("否决") || answer.contains("不可行") || answer.contains("无法处理") {
    (true, Some(snippet(answer)))
}
```

**全文裸子串匹配**。后果：

- `方案 A 不可行，建议改用方案 B` → 命中「不可行」→ `vetoed=true`
- `该设计曾因成本被否决，现改用折中方案` → 命中「否决」→ `vetoed=true`

这类「否定旧方案 + 给出更好替代」恰恰是最有价值的回答。
若只修 D1/D2 而不修 D3，强制拦截会把它们全部吞掉——**修复反而造成可用性回归**。

这决定了修复顺序：**先让标志可信（D3），再让下游拦截（D1/D2）**。

## 三、修复方案

### D3：收紧语义兜底为「整体否定」强模式

去掉裸子串，改为限定主语/结论性的模式表：

```
方案不可行 / 该方案不可行 / 此方案不可行 / 整体不可行
无法处理该 / 无法完成该 / 无法给出 / 无法提供
建议否决 / 予以否决 / 应予否决 / 决定否决
不予采纳 / 存在重大风险 / 严重违反
```

关键区分（`"方案 A 不可行"` 不含连续串 `"方案不可行"`，故不命中）：

| 输入 | 修复前 | 修复后 | 正确性 |
|---|---|---|---|
| `方案 A 不可行，建议改用方案 B` | vetoed | 正常 | ✅ 局部否定不误判 |
| `该设计曾因成本被否决，现改用折中方案` | vetoed | 正常 | ✅ 陈述历史不误判 |
| `该方案不可行，建议整体重构` | vetoed | vetoed | ✅ 整体否定仍命中 |

**权衡取向：宁可漏报（用户仍看到内容），不可误报（正常回答被拦截）。**
显式控制行「是否否决：是」仍保持最高优先级，不受此表影响。

### D1：履行强制拦截契约

`vetoed=true` 时不输出被否决正文，改为回执拦截说明 + 原因，置信度归零，附 `blocked: true`。
保留响应结构（`analysis` / `solution` 仍为字符串），避免前端渲染崩溃。

### D2：被否决回复不得参与融合

`fuse_answers` 先过滤 `vetoed`，再算权重与主导观点；全部被否决时整体拦截
（返回 `blocked: true`、置信度 0），而非给出伪结论。

## 四、回归测试

| 测试 | 守护对象 |
|---|---|
| `vetoed_report_is_blocked_not_passthrough` | D1：拦截且不泄露被否决正文 |
| `fuse_answers_excludes_vetoed_replies` | D2：被否决者即便 match_score 更高也不得成为主导 |
| `fuse_answers_all_vetoed_returns_blocked` | D2：全否决时整体拦截 |
| `parse_veto_does_not_flag_partial_rejection` | D3：局部否定不误判 |

## 五、关于 P1：推荐方案与真实权衡

原 P1 给了三个选项（前置 / 旁路 / 降级），**三个都不对**：

- **前置**：`mox_optimize(&FlowGraph, &GovernContext)` 需要图结构。LLM 咨询只有一句自然语言，
  为它构造一张图是编造输入；且**前置也挡不住输出违规**——你无法预知模型会说什么。
- **旁路**：等于放弃治理，不可接受。
- **降级**（现状）：LLM 成功路径裸奔，仅失败回退时才带闸门。

**推荐：后验治理（post-hoc governance）。** 这是 LLM 场景唯一正确的治理位置——
先拿到输出，再判定是否放行。当前形态是「模型显式自评 + 本地强模式兜底」，
即本轮加固后的 D3 + D1 + D2 组合。

**必须承认的强度差距**（这是需要你确认的真实权衡，不是缺陷）：

| 路径 | 治理强度 |
|---|---|
| 本地 `mox_optimize` | 强——权限 / 安全 / 合规专家 + 算法验证 + 审计链 |
| LLM 咨询 | 弱——仅后验文本规则，无权限与合规专家审查 |

若业务要求 LLM 路径也具备强治理，可选路径是：**在 ReAct 结束后，把 LLM 产出
（工具调用序列 + 结论）映射为一张 `FlowGraph`，再跑一次 `mox_optimize` 做后验审查**。
这是可行的，但需要新增「文本 → 图」的映射器，属于 2-3 人日的独立开发项。

## 六、验证结果

编译与测试均在隔离 target 目录（`CARGO_TARGET_DIR=D:/cargo-target`、`CARGO_INCREMENTAL=0`）执行，
规避 Windows Defender 对 `target/` 的并发持锁。

| 目标 | 结果 |
|---|---|
| `cargo check -p mox-platform-gateway-svc` | 0 error（25 warning 均为存量未使用 import） |
| `cargo test -p mox-platform-gateway-svc` | **92 passed / 0 failed**（lib 92 + 集成 22） |
| `cargo test -p mox-ai-expert-svc` | **194 passed / 0 failed**（lib 194 + 集成 66） |
| 合计 | **374 passed / 0 failed** |

新增 4 项测试全部通过：`vetoed_report_is_blocked_not_passthrough`、
`fuse_answers_excludes_vetoed_replies`、`fuse_answers_all_vetoed_returns_blocked`、
`parse_veto_does_not_flag_partial_rejection`。

### 过程中修正的一处自身缺陷

`fuse_answers` 首版过滤后仍用 `answers[dominant_idx]` 取主导专家，而 `dominant_idx`
是过滤后 `admitted` 的下标 —— **下标空间不一致**，过滤掉被否决项后会错位取到原数组里
的其他专家（测试观察到主导观点仍是「专家甲」）。已改为 `admitted[dominant_idx]`。

这正好印证了为什么这三项必须配套回归测试：过滤类改动的下标语义极易出错，
而这类错误在功能上表现为「静默给出错误的专家结论」，不会崩溃、不会报错。

## 七、遗留

1. **LLM 路径强治理**：是否需要「产出 → FlowGraph → mox_optimize 后验」映射器（2-3 人日）
2. `test_reset_all` 观察为 flaky（本轮两次运行中一次失败一次通过），与本次改动无关，建议单独排查
3. gateway 存量 25 条 warning（未使用 import / 未读字段）未在本轮清理，属独立治理项

# 自研 AI 代码引擎 · 开发专家联盟处理模式（CODEENGINE-ALLIANCE-V1.1）

> 权威源：本文件描述 `mox-codeengine-core` / `mox-codeengine-svc` 两 crate 的设计与契约。
> 联盟总体架构见 [CURRENT-ARCHITECTURE.md](./CURRENT-ARCHITECTURE.md)；处理模式定义见 [README.md](./README.md)。

## 1. 定位

**全自研**（zero external AI dependency）的端到端代码引擎：自然语言需求 → 可交付代码工程。
全部推理为确定性自研算法——规则 IR 解析器、四角色专家联盟、加权裁决、三证守恒闸门、
诊断驱动的自修复收敛循环、diff-first 增量补丁；
不依赖任何外部 LLM API，离线可跑、结果可复现、链路可审计。

| crate | 层 | 路径 | 职责 |
|---|---|---|---|
| `mox-codeengine-core` | L5 core | `platform/domains/ai/core/mox-codeengine-core` | 引擎本体（纯计算，复用 `mox-ai-flow-core` 求解与出码） |
| `mox-codeengine-svc` | L4 svc | `platform/domains/ai/svc/mox-codeengine-svc` | HTTP 服务 `:3210`（`MOX_CODEENGINE_PORT` 覆盖） |

## 2. 联盟处理模式管线（对齐联盟语义，V1.1 含自修复收敛循环）

```
Intent → Build → ⟲[ Solve → Team → Diagnose → Verdict → Generate → Gate → (Refine) ]→ Deliver → Learn
  IR归一   建图      求解    组队    并行诊断    加权裁决    裁决后出码  三证闸门   自修复重入   交付     案例沉淀
```

每阶段产出一条 `StageEvent{stage, ok, note}`，全轨迹随 `EngineResult` 返回（可审计）。
评审-修复循环最多 `max_repair_rounds`（默认 3）轮：任一轮 **裁决通过且三证闸门全过** 即收敛退出；
否则将诊断意见映射为确定性图突变（见 §2.1）后重入 Solve——即 2026 主流 code agent 的
「生成→验证→失败→修复→重试」自修复闭环，全部为自研规则，零 LLM 依赖。

- **Intent**（`ir.rs`）：中文/英文需求句法切分 → 步骤 + 工具类别推断 + 资源读写抽取 + 敏感资源打标。
- **Build**（`graph.rs`）：IR → `FlowGraph`；敏感步骤前自动注入 `desensitize` 护栏、Shell 步骤注入 `path_check` 护栏。
- **Solve**（复用 `mox-ai-flow-core::pipeline::optimize`）：数据流并行化 → 冲突检测 → 自动修复 → CPM/RCPSP 排程 → 算力分级路由。
- **Team/Diagnose**（`experts.rs`）：四角色专家独立出具意见（`Finding{code,severity,suggestion,confidence,target}`，
  `target` 指明缺陷所在节点/边，供自修复定位）：
  analyst（结构完整性）/ builder（可优化性）/ auditor（安全合规）/ coordinator（协作质量）。
- **Verdict**（`verdict.rs`）：加权投票融合（analyst .25 / builder .20 / auditor .35 / coordinator .20，threshold .75）；
  任一专家 Blocking 意见禁止出码，auditor 的 Blocking 另记一票否决（vetoed）。
- **Generate**：仅在裁决通过后调用 `codegen::generate`（C1 红线：未审不过不得生成）。
- **Gate**（`gate.rs`）三证守恒（始终对照**原始建图** `raw_graph`，防止修复循环偷偷改变需求语义）：
  1. 拓扑守恒——优化图保留全部可执行节点；
  2. 数据守恒——读写资源集合前后不变；
  3. 往返守恒——生成代码覆盖每个节点且可被 `reverse_from_python` 重建。
- **Refine**（`repair.rs`）：仅当应用了修复动作时产生事件，记录轮次与突变清单。
- **Deliver**：`delivered = approved && gates.passed`；`EngineResult.rounds/repairs` 暴露收敛过程。
- **Learn**（`case.rs`）：`CodeCase`（含 `rounds/repairs_applied`）入 `CaseBook`，导出 JSONL，
  交付率与平均收敛轮数作为后续调权学习信号。

### 2.1 自修复规则表（repair.rs，只修结构缺陷、绝不放宽审计标准）

| 诊断码 | 缺陷 | 确定性突变 |
|---|---|---|
| `CE-D-001` | 敏感节点缺前置脱敏护栏 | 在 `target` 前注入 `desensitize` Guard 并重接入边 |
| `CE-D-003` | Shell 节点缺命令白名单护栏 | 在 `target` 前注入 `path_check` Guard |
| `CE-A-003` | 孤立不可达节点 | `start→id`、`id→end` 顺序边接回主链 |

修复全部**幂等**（已有同 tag 前置护栏则跳过），无可适用规则即一轮终止不空转。

### 2.2 Diff-first 增量编辑（patch.rs，Aider 式 SEARCH/REPLACE）

交付时对调用方提供的基线文件（`known_files`）产出**最小压缩编辑块**而非全量覆写：
公共前后缀行剥除；应用侧按 精确唯一命中 → 全文匹配 → 歧义拒绝（多处命中）→ 缺文件失败
的确定性顺序匹配，保证可回滚可审计。`render()/parse_blocks()` 支持标准
`<<<<<<< SEARCH path=X / ======= / >>>>>>> REPLACE` 文本互转。

## 3. HTTP 契约（V1.1）

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/api/codeengine/run` | body `{text, id?, name?, auto_guard?, threshold?, self_repair?, max_repair_rounds?, known_files?, with_bundle?}` → 裁决/闸门/事件轨迹/代码包 + `rounds/repairs/patches` |
| POST | `/api/codeengine/patch` | body `{base, blocks?, blocks_text?}` → 对基线应用 SEARCH/REPLACE 编辑块，返回逐块 `exact/whole_file/no_change/not_found/ambiguous/missing_file` 状态与新文件集 |
| GET | `/api/codeengine/health` | 引擎名、处理模式、能力位（含 `self-repair-loop` / `diff-first-patching`）、十一阶段与四专家阵容 |
| GET | `/api/codeengine/cases` | Learn 案例库：交付率与**平均收敛轮数** `avg_rounds_to_converge` |

## 4. 联盟接入

引擎以专家 **`expert-code-engine`**（`code-engine-001` · 自研AI代码引擎）注册于
`mox-alliance-config-core::examples::domain_experts::build_domain_experts()`，
tags `code / codegen / self-hosted / programming`，provider `mox-selfhosted` 指向
`http://127.0.0.1:3210/api/codeengine`，零 token 成本、回退链 deepseek → openai-code。
调度器（`:3100`）启动时经 `build_domain_experts()` 种子自动纳入专家匹配。

## 5. 验证（V1.1）

- 单测：core 29 项 + svc 冒烟 2 项 + doctest 1 项全绿；`cargo clippy --all-targets` 零告警；
- 自修复收敛实测（`:3210` 在线）：`auto_guard=false` + 敏感需求 → 事件轨迹
  `…gate → refine → solve…gate` 二轮收敛，`rounds=2`、修复动作含 desensitize 护栏注入、正常交付；
- diff-first 实测：`known_files` 基线（一份过期 + 一份一致）→ 产出 2 个编辑块，
  `/api/codeengine/patch` 应用状态 `["no_change","exact"]`，roundtrip 还原生成内容与全量覆写逐字节一致；
  Aider 式 `blocks_text` 文本块解析应用同样实测通过；
- Learn：`/api/codeengine/cases` 输出 `avg_rounds_to_converge`（本 demo 1.5）与交付率；
- 端口门禁：`3210` 已登记 `docs/api/PORT-REGISTRY.md` §3.2 与 `scripts/verify-ports.py` CANONICAL（ERROR=0）；
- 否决路径：敏感资源 + `auto_guard=false` + `self_repair=false` → auditor 一票否决、不出码、不交付（测试覆盖）。

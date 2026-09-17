# alliance 专家联盟域「智能自动决策」现状只读分析

- 日期：2026-09-16
- 范围：`platform/domains/alliance/`，只读，未改任何代码
- 链路：goal → 专家匹配 → 融合策略 → DAG 规划 → 视觉/布局场景可达性
- 生产实际接线：`ModularWeightMatcher`（`svc/mox-alliance-scheduler-svc/src/server.rs:251`）+ `SimplePlanGenerator`（`scheduler-core/src/planner.rs`）+ executor 侧 `mox-alliance-executor-core/src/fusion.rs`（scheduler-core 内的 `fusion.rs` 模块头注释自承"未接线"，见 `scheduler-core/src/fusion.rs:15-22`）

---

## 总判定

| 链路 | 判定 | 一句话 |
|---|---|---|
| 1. 专家自动匹配/组队 | **半智能（规则+词典，非模型）** | 中英 bigram 分词 + 领域词典推断有证据门槛，但全为硬编码规则；空 query 时退化为随机选 5 人，无兜底 |
| 2. 融合策略选择 | **硬编码/缺失（无自动选型）** | 策略 100% 来自请求字段，缺省固定 `Weighted(weighted_voting)`，无任何按任务类型/专家构成的自动选择 |
| 3. DAG 自动规划 | **硬编码（纯模板，无规划）** | 不拆 goal，每个节点跑整段任务描述；仅按请求 mode 套 6 种固定模板 |
| 4. 布局/视觉分析入口 | **半断链** | vision 专家存在且词典含图像类词，但无"布局/layout/文档/OCR 版面"词；且全链路无图片输入字段，匹配到也拿不到图 |

---

## 链路 1：专家自动匹配/组队 —— 半智能

**流程**：`scheduler.rs:267` `matcher.infer_domains(&task.description)` → 作为 `required_domains`（`scheduler.rs:275`）→ `ModularWeightMatcher.match_experts` → 截断 `max_results: 5`（`scheduler.rs:278`，硬编码）。

**权重从哪来**：
- 评分权重来自 `MatchingWeights`（默认 0.35/0.30/0.20/0.10/0.05，见 `modular_matcher.rs:388-395` 与 proto 默认值），可按 expert_id 覆写（`modular_matcher.rs:93-98`），但内置 10 专家全部用默认权重（`domain_experts.rs:694` `matching_weights: MatchingWeights::default()`）。
- 领域推断为**硬编码词典**：`matching.rs:53-139` `DOMAIN_KEYWORDS`（约 200 条中英关键词），证据 ≥2 才推断（`matching.rs:415`），并有通用 bigram 黑名单降噪（`matching.rs:42-50`）与英文词干匹配（`matching.rs:256-269`）。**无 LLM/向量打分**。

**选错专家的风险点（均有证据）**：
1. **空/弱 query 时全量匹配 + 排序随机**：`infer_domains` 空描述返回 `vec![]`（`matching.rs:385-387`）→ `required_domains` 为空 → `ModularWeightMatcher` 跳过领域硬过滤（`modular_matcher.rs:254`），所有专家 domain 分=1.0；空描述时 `description_overlap` 给中性分 0.3（`matching.rs:327-329`），10 个内置专家 priority 全为 5（`server.rs:212`）、health 全为 default → **除 HashMap 遍历顺序外无任何区分度，截断 5 人实际是"任意 5 人"**。
2. **推断失败即静默空计划**：一旦推断出的领域没有任何专家 tag 命中（如新增词典词但无对应专家），硬过滤把全部专家刷掉（`modular_matcher.rs:254-264`）→ `matches` 为空 → planner 生成 0 节点计划，而 `CollaborationPlan::validate()` 对 0 节点直接 Ok（`common-proto/src/types.rs:292-335`，`count==0==nodes.len()`）→ 任务提交后"成功"瞬间完成、融合输出空（`executor-core/src/fusion.rs:123-132`）。**无"未匹配到专家"错误、无默认专家兜底**。
3. **能力词库空转**：内置专家 `capability_weights` 全空（`domain_experts.rs:693`），`required_capabilities` 恒为 `vec![]`（`scheduler.rs:276`）→ 能力分恒 0.5（`modular_matcher.rs:141-142`），`required_domains` 与专家能力词库对不上时无兜底，只能靠描述重叠（权重仅占 0.3）。
4. **组队无集合覆盖/去重**：`match_experts` 逐专家独立打分排序截断（`modular_matcher.rs:305-316`），**没有任何 set-cover/互补性组队逻辑**；同一域会一次拉满（如 finance+investment 双 tag 只算一个专家）。

## 链路 2：融合策略选择 —— 硬编码/缺失

- 策略来源全链路透传：请求 `fusion_strategy: Option<FusionStrategy>`（`api/src/dto.rs:29`；`scheduler-proto/src/scheduler.rs:22`）→ 缺省 `config.default_fusion_strategy` → 再缺省 `FusionStrategy::Weighted`（`scheduler.rs:237-239`；`scheduler-proto/src/types.rs:34`；boot 配置 `"weighted"` 硬编码 `boot-config/src/lib.rs:118`）→ 原样写进 plan（`planner.rs:61`）→ DAG 尾部 `let strategy = state.plan.fusion_strategy;`（`executor-core/src/dag_engine.rs:384`）。
- **全仓 grep 无任何"按任务类型/专家数自动选策略"逻辑**（无 `select_strategy`/`auto_strategy` 类函数；9 个策略仅被按 enum 分发 `executor-core/src/fusion.rs:137-147`）。
- 即：**该用 confidence_weighted/stacking 的多专家高方差任务，只要调用方不显式传，就一律退回 weighted_voting**——这正是问题中担心的缺失，已确认存在。
- 附带问题：尾部融合时 `expert_weights: HashMap::new()` 恒为空（`executor-core/src/dag_engine.rs:396`），即 `weighted_voting`/`confidence_weighted` 实际全部按等权 1.0 跑（`executor-core/src/fusion.rs:71-77`），匹配阶段算出的专家分根本没回流到融合。

## 链路 3：DAG 自动规划 —— 硬编码（纯模板）

- `SimplePlanGenerator::generate` 只按 `request.preferred_mode.unwrap_or(Parallel)` 分派 6 个函数（`planner.rs:47-56`），**不做 goal 拆分**：每个节点的任务描述都是整段 `request.task_description`（`planner.rs:91/116/157/187/270`），节点之间没有子任务划分、没有输入输出传递（`input_refs: vec![]` 硬编码，`planner.rs:282`）。
- 6 个模板：并行（无依赖）/串行（按分排序链式）/分层（按首 tag 分组 BTreeMap 字典序，`planner.rs:130-164`）/辩论（硬取第 1、2、3 名当正/反/裁判，`planner.rs:181-210`）/迭代（硬编码 3 轮、只用前 2 名专家，`planner.rs:221-226`）。
- 文件头自承 "Phase 1 实现…后续可以接入 AI 生成更复杂的计划"（`planner.rs:8-13`）；全 alliance 域无 `todo!()`/`unimplemented!()`（grep 0 命中）——不是占位未实现，而是**有意的规则模板，规划能力本身缺失**。

## 链路 4：布局/视觉分析入口 —— 半断链

- 专家本身在：`expert-vision`，tags `["vision","image"]`（`domain_experts.rs:440-486`，tag 见 `:485`），生产经 `expert_from_module` 把 system_prompt 拼进可检索文本（`server.rs:203-206`）。
- 词典命中：vision 词条有 图像/图片/视觉/识别/画面/照片/图表解读/ocr/image/vision/photo/picture/visual/detect（`matching.rs:116-119`）。**但没有"布局/layout/排版/文档/document/pdf"任何词**。
- 推演："分析这份 PDF 文档的版面布局" → token 为 文档/版面/布局（"分析"在通用 bigram 黑名单 `matching.rs:43`）→ 词典 0 命中，vision 专家文本（"图像描述/OCR文字识别/图表解读/视觉问答/产品识别/场景分析"）0 重叠 → 证据 <2 不推断（`matching.rs:415`）→ `required_domains=[]` → 退化为链路 1 的"任意 5 人"，**expert-vision 不被定向选中**。
- 更底层断链：全 alliance 域 grep `image_url|multimodal|attach_image` **0 命中**——任务提交 DTO 没有任何图片字段，planner 只把文字任务描述塞进节点（`planner.rs:270`），即使匹配到 expert-vision，**图片也传不进执行链路**。

---

## 最值得做的低风险优化（只给方向，未改代码）

1. **修"空匹配=静默空计划"与"空 query=随机 5 人"**（风险最低、收益最直接）
   - 位置：`scheduler-core/src/scheduler.rs:281-289`。
   - 改法方向：`match_result.matches.is_empty()` 时不要把空 plan 交给 executor；二选一——(a) 记录 warning 并退回 `required_domains=[]` 重匹配一次（此时领域分全 1.0，至少有专家干活）；(b) 显式失败（`TaskFailed` + "no expert matched"），消除"假成功"。
   - 空 query 分支：`required_domains` 为空时 10 个专家分数趋同、截断随机（证据见链路 1-1）。建议空 query 时固定返回最高 priority 的 1 个"通用"专家或直接报错要求补充描述，而不是随机 5 人。
2. **fusion 权重闭环 + 最小自动策略规则**（低风险增量，不动 9 策略实现）
   - 位置：`executor-core/src/dag_engine.rs:396`（`expert_weights: HashMap::new()`）。改法方向：把匹配阶段 `MatchedExpert.score`（`matcher` 已算好）随 plan/节点带到尾部融合，替代空 HashMap——`confidence_weighted`/`weighted` 立刻从"伪加权"变真加权。
   - 同文件附近加一条规则函数：`matches.len()==1 → BestOf`；`matches.len()>=3 且描述歧义高 → ConfidenceWeighted`；仅当请求显式传 `fusion_strategy` 时才覆盖。规则表 5~10 行，可配置、可单测，不触碰策略内部。
3. **（可选，小改动）补 vision 布局词**：`matching.rs:116-119` 的 vision 词条补 `布局/layout/排版/版面/文档分析/document`，并在 `domain_experts.rs:475-480` vision prompt 的"擅长"里加"文档版面/布局分析"——让"PDF 排版分析"这类 query 能命中 expert-vision。注意：这只修路由，**图片输入通道（链路 4 底层断链）需另行立项**，不在本次范围。

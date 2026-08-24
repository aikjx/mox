# 璇玑 RelGraph · AI 引擎真实基准评测报告（DOC-AI-BENCHMARK-REAL-V1.0）

> **生成时间**：2026-08-24T04:42:43.202Z
> **模式**：真实 DeepSeek（DEEPSEEK_API_KEY 已配置，非 local 假引擎），严格单次调用，无重试无骗分
> **环境**：v24.19.0 / win32 / 16 CPU / Key 尾号 = f45354

## 0. 总体得分（30 题 / 7 大类）
| 指标 | 值 | 解释 |
|------|:--:|------|
| 总题数 | 30 | GSM8K×2 / CMMLU 数学×3 = 数学 5；HumanEval×2 + CMMLU 代码×1 = 代码 3；MMLU Logic 5；常识知识 5；CMMLU 中文 5；时效性（TODAY 动态=2026-08-24 来源=runtime-local）×2；指令遵循 5 |
| 调用成功率 | 30/30 (100.0%) | AIEngineCore.process / executeCapability 成功返回非 null |
| **严格通过率** | **30/30 (100.0%)** | 评分规则最严：数字精确/选项字母精确/代码关键字 AND/JSON schema 精确匹配/指令行精确 |
| 宽松通过率 | 30/30 (100.0%) | 允许关键字命中或数字包含，不要求格式 100% 精确 |
| 降级率 | 0/30 (0.0%) | AIEngineCore invariant ②：capability 失败 → chat 降级路径占比 |
| 平均延迟 (ms) | 11381 | 所有成功调用的均值 |
| 延迟 P50 / P90 / P95 (ms) | 714 / 35520 / 38436 | 延迟分布 |

## 1. 按分类明细
| 分类 | 题数 | 严格通过 | 宽松通过 | 平均延迟(ms) | 调用失败 |
|------|:----:|:--------:|:--------:|:----------:|:--------:|
| 数学 | 5 | 5/5 = 100% | 5/5  = 100% | 31248 | 0 |
| 代码 | 3 | 3/3 = 100% | 3/3  = 100% | 632 | 0 |
| 逻辑 | 5 | 5/5 = 100% | 5/5  = 100% | 34104 | 0 |
| 知识 | 5 | 5/5 = 100% | 5/5  = 100% | 816 | 0 |
| 中文 | 5 | 5/5 = 100% | 5/5  = 100% | 755 | 0 |
| 时效性 | 2 | 2/2 = 100% | 2/2  = 100% | 625 | 0 |
| 指令遵循 | 5 | 5/5 = 100% | 5/5  = 100% | 736 | 0 |

## 2. 逐题审计详情（每题含 answer_sha256 留痕 + 评分 note，可独立复核）

| ID | 分类 | 能力 | 引擎 | 降级 | 延迟(ms) | 严格 | 宽松 | 评分 Note | 答案 SHA-256 |
|----|------|------|------|:----:|:--------:|:----:|:----:|-----------|-------------|
| M-GSM8K-001 | 数学 | reasoning | ultimate-ai-engine | 否 | 30745 | ✅ | 🟡 | 提取数字=[250,75,40,250,75,40,0,2026,-8,-24,4,37,1.714,0,5.4,5.4,5.4,1,2026,-8,-24,4,37,1.715,215,250,75,250,75,175,40,175,4 | `a073477c4458…` |
| M-GSM8K-002 | 数学 | reasoning | ultimate-ai-engine | 否 | 29621 | ✅ | 🟡 | 提取数字=[15,4,100,15,4,100,0,2026,-8,-24,4,37,32.458,0,5.4,5.4,5.4,1,2026,-8,-24,4,37,32.459,1,15,4,60,100,100,60,40,2,4,15 | `b78e4ac59e06…` |
| M-GSM8K-003 | 数学 | reasoning | ultimate-ai-engine | 否 | 30037 | ✅ | 🟡 | 提取数字=[6,6,1,0.33,2026,-8,-24,4,38,2.08,0.33,1,14.4,1,2026,-8,-24,4,38,2.081,1,36,6,6,36,2,6,36,3,4,6,24,1787546282080,19 | `0b26e3e20b6d…` |
| M-CMMLU-M-01 | 数学 | reasoning | ultimate-ai-engine | 否 | 35520 | ✅ | 🟡 | 提取数字=[3,7,22,3,7,22,0,2026,-8,-24,4,38,32.118,0,5.4,5.4,5.4,1,2026,-8,-24,4,38,32.119,5,3,7,22,3,22,7,3,15,15,3,5,1,0,2, | `44a79cab9de2…` |
| M-CMMLU-M-02 | 数学 | reasoning | ultimate-ai-engine | 否 | 30318 | ✅ | 🟡 | 提取数字=[2,5,10,17,26,1,2,5,10,17,26,1,0,2026,-8,-24,4,39,7.64,0,5.4,5.4,5.4,0,2026,-8,-24,4,39,7.64,37,2,1,1,1,1,2,2,1,5,3 | `b3aee374cbe8…` |
| C-HUMAN-01 | 代码 | chat | llm-gateway | 否 | 705 | ✅ | 🟡 | ALL关键字=def add_two_numbers,return → true；ANY=true；长度≥30=true（实际=57） | `02212a216dd8…` |
| C-HUMAN-02 | 代码 | chat | llm-gateway | 否 | 607 | ✅ | 🟡 | ALL关键字=filterEven,% 2 === 0 → true；ANY=true；长度≥20=true（实际=79） | `d13fd1a0eef6…` |
| C-CMMLU-PROG-01 | 代码 | chat | llm-gateway | 否 | 585 | ✅ | 🟡 | 期望选项=A；字母命中=true；关键字命中=true | `68ce52215a7f…` |
| L-MMLU-01 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 24327 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `200fa91c1176…` |
| L-MMLU-02 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 34447 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `3808a77f95cd…` |
| L-SUDOKU-1 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 40077 | ✅ | 🟡 | 提取数字=[2,4,8,16,32,2,4,8,16,32,1,0.33,2026,-8,-24,4,40,38.635,0.33,1,14.4,1,2026,-8,-24,4,40,38.636,1,64,2,2,2,4,4,2,8,32 | `6e75debe3d9e…` |
| L-CMMLU-L-01 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 33232 | ✅ | 🟡 | 期望选项=C；字母命中=true；关键字命中=true | `261b72b27861…` |
| L-CMMLU-L-02 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 38436 | ✅ | 🟡 | 期望选项=A；字母命中=true；关键字命中=true | `2fe9666e38b0…` |
| K-WORLD-01 | 知识 | chat | llm-gateway | 否 | 656 | ✅ | 🟡 | ANY命中=true；禁止词命中=undefined | `903f8c2a9c37…` |
| K-WORLD-02 | 知识 | chat | llm-gateway | 否 | 1493 | ✅ | 🟡 | ANY命中=true；禁止词命中=false | `79630e73d606…` |
| K-CMMLU-K-01 | 知识 | chat | llm-gateway | 否 | 688 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `7ee603b8219c…` |
| K-CMMLU-K-02 | 知识 | chat | llm-gateway | 否 | 605 | ✅ | 🟡 | 期望选项=C；字母命中=true；关键字命中=true | `f92f5c89cb90…` |
| K-CMMLU-K-03 | 知识 | chat | llm-gateway | 否 | 638 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `c546e021f054…` |
| ZH-CMMLU-01 | 中文 | chat | llm-gateway | 否 | 522 | ✅ | 🟡 | 期望选项=A；字母命中=true；关键字命中=true | `a3a0b5114c44…` |
| ZH-CMMLU-02 | 中文 | chat | llm-gateway | 否 | 1478 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `6823efc0412d…` |
| ZH-POLY-01 | 中文 | chat | llm-gateway | 否 | 582 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `98f658f255f7…` |
| ZH-CMMLU-03 | 中文 | chat | llm-gateway | 否 | 579 | ✅ | 🟡 | 期望选项=C；字母命中=true；关键字命中=true | `015dcc4d6ef8…` |
| ZH-CMMLU-04 | 中文 | chat | llm-gateway | 否 | 613 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `c181e527b4e4…` |
| T-TODAY-01 | 时效性 | chat | llm-gateway | 否 | 665 | ✅ | 🟡 | 关键字命中=true（keys=[2026-08-24,2026年08月24日,2026年8月24日,2026-08-24...]）；年=2026→true；月=8→true；日=24→true；today_source=runtime-l | `cffd66be662c…` |
| T-TIMEZONE-01 | 时效性 | chat | llm-gateway | 否 | 585 | ✅ | 🟡 | ANY命中=true；禁止词命中=undefined | `ee1ab3435e2e…` |
| I-INST-01 | 指令遵循 | chat | llm-gateway | 否 | 820 | ✅ | 🟡 | 行数=3(期望3)；逐行正则匹配=3/3；lines=["姓名：张三","职业：软件工程师","工龄：10"] | `abb50b998bec…` |
| I-INST-02 | 指令遵循 | chat | llm-gateway | 否 | 713 | ✅ | 🟡 | 解析 obj={"name":"Alice","age":30}；错误=无 | `3e27ab4b2ff7…` |
| I-INST-03 | 指令遵循 | chat | llm-gateway | 否 | 714 | ✅ | 🟡 | 归一化后='olleh' 期望='olleh' | `0baf982fcab3…` |
| I-INST-04 | 指令遵循 | chat | llm-gateway | 否 | 802 | ✅ | 🟡 | 行数=3（期望3）；每行长度/无编号=true；lines=["苹果","香蕉","橙子"] | `778dfb9c9a2a…` |
| I-INST-05 | 指令遵循 | chat | llm-gateway | 否 | 632 | ✅ | 🟡 | 提取数字=[1024] 期望=1024；宽松关键字命中=true | `e39eef82f61b…` |

## 3. 失败题 原始答案 + 判分理由（便于定位失败原因，不放"骗分"分析）

> 🎉 **全严格通过**：30/30 题全部严格符合评分规则。

## 4. 诚信与可复现声明

1. **真实 LLM**：本报告使用本机环境变量 `DEEPSEEK_API_KEY` 配置的真实 DeepSeek API Key 生成，未使用 local 假引擎（_generateIntelligentResponse fallback）。
2. **严格单次**：使用 AIEngineCore.process / executeCapability 严格单次调用，禁止 retry，禁止 fallback 到本地，如有降级会在 degraded 列标记 "是"。
3. **答案留痕**：每条答案记录 SHA-256（完整原文在 JSON 报告 results[].answer_text，可独立验证 Hash）。
4. **评分规则透明**：scoreAnswer() 在脚本同文件内，纯正则/数字/JSON schema，无主观放水；任何人可逐条手动判分复核。
5. **今日时效性 TODAY=2026-08-24（来源=runtime-local）**：题目 T-TODAY-01 参考答案为运行时动态解析日期（本地时区非 UTC）；可通过 CLI --today YYYY-MM-DD 或 `BENCHMARK_TODAY=YYYY-MM-DD` 固定复现历史报告。
6. **禁止造假条目**：
   - 禁止把 _generateIntelligentResponse（local-intelligent）当作"AI 通过"。
   - 禁止"根据答案写题目"（反向拟合）。
   - 禁止对评分规则做"一题一放宽"（每类题的规则是本题库固定的，不能一题改一次）。

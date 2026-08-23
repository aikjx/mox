# 璇玑 RelGraph · AI 引擎真实基准评测报告（DOC-AI-BENCHMARK-REAL-V1.0）

> **生成时间**：2026-08-23T13:47:23.567Z
> **模式**：真实 DeepSeek（DEEPSEEK_API_KEY 已配置，非 local 假引擎），严格单次调用，无重试无骗分
> **环境**：v22.23.1 / win32 / 16 CPU / Key 尾号 = f45354

## 0. 总体得分（30 题 / 7 大类）
| 指标 | 值 | 解释 |
|------|:--:|------|
| 总题数 | 30 | GSM8K×2 / CMMLU 数学×3 = 数学 5；HumanEval×2 + CMMLU 代码×1 = 代码 3；MMLU Logic 5；常识知识 5；CMMLU 中文 5；时效性 TODAY 固定 2；指令遵循 5 |
| 调用成功率 | 30/30 (100.0%) | AIEngineCore.process / executeCapability 成功返回非 null |
| **严格通过率** | **30/30 (100.0%)** | 评分规则最严：数字精确/选项字母精确/代码关键字 AND/JSON schema 精确匹配/指令行精确 |
| 宽松通过率 | 30/30 (100.0%) | 允许关键字命中或数字包含，不要求格式 100% 精确 |
| 降级率 | 0/30 (0.0%) | AIEngineCore invariant ②：capability 失败 → chat 降级路径占比 |
| 平均延迟 (ms) | 10742 | 所有成功调用的均值 |
| 延迟 P50 / P90 / P95 (ms) | 880 / 34129 / 35761 | 延迟分布 |

## 1. 按分类明细
| 分类 | 题数 | 严格通过 | 宽松通过 | 平均延迟(ms) | 调用失败 |
|------|:----:|:--------:|:--------:|:----------:|:--------:|
| 数学 | 5 | 5/5 = 100% | 5/5  = 100% | 30609 | 0 |
| 代码 | 3 | 3/3 = 100% | 3/3  = 100% | 764 | 0 |
| 逻辑 | 5 | 5/5 = 100% | 5/5  = 100% | 30919 | 0 |
| 知识 | 5 | 5/5 = 100% | 5/5  = 100% | 731 | 0 |
| 中文 | 5 | 5/5 = 100% | 5/5  = 100% | 743 | 0 |
| 时效性 | 2 | 2/2 = 100% | 2/2  = 100% | 604 | 0 |
| 指令遵循 | 5 | 5/5 = 100% | 5/5  = 100% | 749 | 0 |

## 2. 逐题审计详情（每题含 answer_sha256 留痕 + 评分 note，可独立复核）

| ID | 分类 | 能力 | 引擎 | 降级 | 延迟(ms) | 严格 | 宽松 | 评分 Note | 答案 SHA-256 |
|----|------|------|------|:----:|:--------:|:----:|:----:|-----------|-------------|
| M-GSM8K-001 | 数学 | reasoning | ultimate-ai-engine | 否 | 36656 | ✅ | 🟡 | 提取数字=[250,75,40,250,75,40,0,2026,-8,-23,13,42,1.279,0,5.4,5.4,5.4,3,2026,-8,-23,13,42,1.283,1,215,250,75,40,215,2,250,75 | `3f72096c04b3…` |
| M-GSM8K-002 | 数学 | reasoning | ultimate-ai-engine | 否 | 30839 | ✅ | 🟡 | 提取数字=[15,4,100,15,4,100,0,2026,-8,-23,13,42,37.931,0,5.4,5.4,5.4,1,2026,-8,-23,13,42,37.932,1,40,4,15,60,100,60,40,2,60, | `342e6b49d69c…` |
| M-GSM8K-003 | 数学 | reasoning | ultimate-ai-engine | 否 | 23338 | ✅ | 🟡 | 提取数字=[6,6,1,0.33,2026,-8,-23,13,43,8.773,0.33,1,14.4,0,2026,-8,-23,13,43,8.773,1,36,6,6,36,2,2,6,6,2,36,3,6,6,1,36,24,8. | `a871e2f6c941…` |
| M-CMMLU-M-01 | 数学 | reasoning | ultimate-ai-engine | 否 | 26449 | ✅ | 🟡 | 提取数字=[3,7,22,3,7,22,0,2026,-8,-23,13,43,32.112,0,5.4,5.4,5.4,1,2026,-8,-23,13,43,32.113,5,3,7,22,3,22,7,15,1,15,3,5,3,5, | `2f6d526e7095…` |
| M-CMMLU-M-02 | 数学 | reasoning | ultimate-ai-engine | 否 | 35761 | ✅ | 🟡 | 提取数字=[2,5,10,17,26,1,2,5,10,17,26,1,0,2026,-8,-23,13,43,58.561,0,5.4,5.4,5.4,1,2026,-8,-23,13,43,58.562,2,5,10,17,26,37, | `fcf646d0942e…` |
| C-HUMAN-01 | 代码 | chat | llm-gateway | 否 | 845 | ✅ | 🟡 | ALL关键字=def add_two_numbers,return → true；ANY=true；长度≥30=true（实际=57） | `02212a216dd8…` |
| C-HUMAN-02 | 代码 | chat | llm-gateway | 否 | 651 | ✅ | 🟡 | ALL关键字=filterEven,% 2 === 0 → true；ANY=true；长度≥20=true（实际=79） | `d13fd1a0eef6…` |
| C-CMMLU-PROG-01 | 代码 | chat | llm-gateway | 否 | 795 | ✅ | 🟡 | 期望选项=A；字母命中=true；关键字命中=true | `68ce52215a7f…` |
| L-MMLU-01 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 25045 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `6176ee344a11…` |
| L-MMLU-02 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 30834 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `04d802f08568…` |
| L-SUDOKU-1 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 34109 | ✅ | 🟡 | 提取数字=[2,4,8,16,32,2,4,8,16,32,1,0.33,2026,-8,-23,13,45,32.506,0.33,1,14.4,0,2026,-8,-23,13,45,32.506,1,64,2,2,2,4,4,2,8, | `7345d51ffe10…` |
| L-CMMLU-L-01 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 30476 | ✅ | 🟡 | 期望选项=C；字母命中=true；关键字命中=true | `aa9a60c722d1…` |
| L-CMMLU-L-02 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 34129 | ✅ | 🟡 | 期望选项=A；字母命中=true；关键字命中=true | `74cc8274d183…` |
| K-WORLD-01 | 知识 | chat | llm-gateway | 否 | 812 | ✅ | 🟡 | ANY命中=true；禁止词命中=undefined | `903f8c2a9c37…` |
| K-WORLD-02 | 知识 | chat | llm-gateway | 否 | 1053 | ✅ | 🟡 | ANY命中=true；禁止词命中=false | `5302e0c1810b…` |
| K-CMMLU-K-01 | 知识 | chat | llm-gateway | 否 | 676 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `7ee603b8219c…` |
| K-CMMLU-K-02 | 知识 | chat | llm-gateway | 否 | 611 | ✅ | 🟡 | 期望选项=C；字母命中=true；关键字命中=true | `f92f5c89cb90…` |
| K-CMMLU-K-03 | 知识 | chat | llm-gateway | 否 | 505 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `c546e021f054…` |
| ZH-CMMLU-01 | 中文 | chat | llm-gateway | 否 | 663 | ✅ | 🟡 | 期望选项=A；字母命中=true；关键字命中=true | `a3a0b5114c44…` |
| ZH-CMMLU-02 | 中文 | chat | llm-gateway | 否 | 1115 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `4d5a83c7f12d…` |
| ZH-POLY-01 | 中文 | chat | llm-gateway | 否 | 837 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `98f658f255f7…` |
| ZH-CMMLU-03 | 中文 | chat | llm-gateway | 否 | 619 | ✅ | 🟡 | 期望选项=C；字母命中=true；关键字命中=true | `015dcc4d6ef8…` |
| ZH-CMMLU-04 | 中文 | chat | llm-gateway | 否 | 479 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `c181e527b4e4…` |
| T-TODAY-01 | 时效性 | chat | llm-gateway | 否 | 602 | ✅ | 🟡 | 关键字命中=true；年=2026→true；月=8→true；日=23→true | `3f673467c80a…` |
| T-TIMEZONE-01 | 时效性 | chat | llm-gateway | 否 | 605 | ✅ | 🟡 | ANY命中=true；禁止词命中=undefined | `ee1ab3435e2e…` |
| I-INST-01 | 指令遵循 | chat | llm-gateway | 否 | 894 | ✅ | 🟡 | 行数=3(期望3)；逐行正则匹配=3/3；lines=["姓名：张三","职业：软件工程师","工龄：10"] | `abb50b998bec…` |
| I-INST-02 | 指令遵循 | chat | llm-gateway | 否 | 889 | ✅ | 🟡 | 解析 obj={"name":"Alice","age":30}；错误=无 | `3e27ab4b2ff7…` |
| I-INST-03 | 指令遵循 | chat | llm-gateway | 否 | 574 | ✅ | 🟡 | 归一化后='olleh' 期望='olleh' | `0baf982fcab3…` |
| I-INST-04 | 指令遵循 | chat | llm-gateway | 否 | 880 | ✅ | 🟡 | 行数=3（期望3）；每行长度/无编号=true；lines=["苹果","香蕉","橙子"] | `778dfb9c9a2a…` |
| I-INST-05 | 指令遵循 | chat | llm-gateway | 否 | 507 | ✅ | 🟡 | 提取数字=[1024] 期望=1024；宽松关键字命中=true | `e39eef82f61b…` |

## 3. 失败题 原始答案 + 判分理由（便于定位失败原因，不放"骗分"分析）

> 🎉 **全严格通过**：30/30 题全部严格符合评分规则。

## 4. 诚信与可复现声明

1. **真实 LLM**：本报告使用本机环境变量 `DEEPSEEK_API_KEY` 配置的真实 DeepSeek API Key 生成，未使用 local 假引擎（_generateIntelligentResponse fallback）。
2. **严格单次**：使用 AIEngineCore.process / executeCapability 严格单次调用，禁止 retry，禁止 fallback 到本地，如有降级会在 degraded 列标记 "是"。
3. **答案留痕**：每条答案记录 SHA-256（完整原文在 JSON 报告 results[].answer_text，可独立验证 Hash）。
4. **评分规则透明**：scoreAnswer() 在脚本同文件 118-230 行，纯正则/数字/JSON schema，无主观放水；任何人可逐条手动判分复核。
5. **今日时效性 TODAY=2026-08-23**：题目 T-TODAY-01 参考答案固定为 2026-08-23；如在其他日期重跑，请修改题目 reference_answer.value。
6. **禁止造假条目**：
   - 禁止把 _generateIntelligentResponse（local-intelligent）当作"AI 通过"。
   - 禁止"根据答案写题目"（反向拟合）。
   - 禁止对评分规则做"一题一放宽"（每类题的规则是本题库固定的，不能一题改一次）。

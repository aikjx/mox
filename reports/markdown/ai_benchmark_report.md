# 璇玑 RelGraph · AI 引擎真实基准评测报告（DOC-AI-BENCHMARK-REAL-V1.0）

> **生成时间**：2026-08-24T04:05:45.072Z
> **模式**：真实 DeepSeek（DEEPSEEK_API_KEY 已配置，非 local 假引擎），严格单次调用，无重试无骗分
> **环境**：v24.19.0 / win32 / 16 CPU / Key 尾号 = f45354

## 0. 总体得分（30 题 / 7 大类）
| 指标 | 值 | 解释 |
|------|:--:|------|
| 总题数 | 30 | GSM8K×2 / CMMLU 数学×3 = 数学 5；HumanEval×2 + CMMLU 代码×1 = 代码 3；MMLU Logic 5；常识知识 5；CMMLU 中文 5；时效性 TODAY 固定 2；指令遵循 5 |
| 调用成功率 | 30/30 (100.0%) | AIEngineCore.process / executeCapability 成功返回非 null |
| **严格通过率** | **29/30 (96.7%)** | 评分规则最严：数字精确/选项字母精确/代码关键字 AND/JSON schema 精确匹配/指令行精确 |
| 宽松通过率 | 29/30 (96.7%) | 允许关键字命中或数字包含，不要求格式 100% 精确 |
| 降级率 | 0/30 (0.0%) | AIEngineCore invariant ②：capability 失败 → chat 降级路径占比 |
| 平均延迟 (ms) | 10449 | 所有成功调用的均值 |
| 延迟 P50 / P90 / P95 (ms) | 845 / 34368 / 34721 | 延迟分布 |

## 1. 按分类明细
| 分类 | 题数 | 严格通过 | 宽松通过 | 平均延迟(ms) | 调用失败 |
|------|:----:|:--------:|:--------:|:----------:|:--------:|
| 数学 | 5 | 5/5 = 100% | 5/5  = 100% | 26565 | 0 |
| 代码 | 3 | 3/3 = 100% | 3/3  = 100% | 780 | 0 |
| 逻辑 | 5 | 5/5 = 100% | 5/5  = 100% | 33250 | 0 |
| 知识 | 5 | 5/5 = 100% | 5/5  = 100% | 918 | 0 |
| 中文 | 5 | 5/5 = 100% | 5/5  = 100% | 656 | 0 |
| 时效性 | 2 | 1/2 = 50% | 1/2  = 50% | 610 | 0 |
| 指令遵循 | 5 | 5/5 = 100% | 5/5  = 100% | 591 | 0 |

## 2. 逐题审计详情（每题含 answer_sha256 留痕 + 评分 note，可独立复核）

| ID | 分类 | 能力 | 引擎 | 降级 | 延迟(ms) | 严格 | 宽松 | 评分 Note | 答案 SHA-256 |
|----|------|------|------|:----:|:--------:|:----:|:----:|-----------|-------------|
| M-GSM8K-001 | 数学 | reasoning | ultimate-ai-engine | 否 | 29552 | ✅ | 🟡 | 提取数字=[250,75,40,250,75,40,0,2026,-8,-24,4,0,31.581,0,5.4,5.4,5.4,0,2026,-8,-24,4,0,31.582,1,215,250,75,40,215,2,250,75,1 | `2fd227020222…` |
| M-GSM8K-002 | 数学 | reasoning | ultimate-ai-engine | 否 | 25599 | ✅ | 🟡 | 提取数字=[15,4,100,15,4,100,0,2026,-8,-24,4,1,1.133,0,5.4,5.4,5.4,1,2026,-8,-24,4,1,1.134,1,40,2,4,15,60,100,100,60,40,3,178 | `acb860df2f66…` |
| M-GSM8K-003 | 数学 | reasoning | ultimate-ai-engine | 否 | 24367 | ✅ | 🟡 | 提取数字=[6,6,1,0.33,2026,-8,-24,4,1,26.733,0.33,1,14.4,1,2026,-8,-24,4,1,26.734,1,36,6,6,36,2,6,6,36,3,36,36,6,0.33,1787544 | `85cdad72981a…` |
| M-CMMLU-M-01 | 数学 | reasoning | ultimate-ai-engine | 否 | 21881 | ✅ | 🟡 | 提取数字=[3,7,22,3,7,22,0,2026,-8,-24,4,1,51.102,0,5.4,5.4,5.4,0,2026,-8,-24,4,1,51.102,5,3,7,22,3,22,7,3,15,15,3,5,1,5,3,5, | `ba5f06233757…` |
| M-CMMLU-M-02 | 数学 | reasoning | ultimate-ai-engine | 否 | 31427 | ✅ | 🟡 | 提取数字=[2,5,10,17,26,1,2,5,10,17,26,1,0,2026,-8,-24,4,2,12.985,0,5.4,5.4,5.4,1,2026,-8,-24,4,2,12.986,1,37,2,5,10,17,26,1, | `972383b6f312…` |
| C-HUMAN-01 | 代码 | chat | llm-gateway | 否 | 872 | ✅ | 🟡 | ALL关键字=def add_two_numbers,return → true；ANY=true；长度≥30=true（实际=57） | `02212a216dd8…` |
| C-HUMAN-02 | 代码 | chat | llm-gateway | 否 | 845 | ✅ | 🟡 | ALL关键字=filterEven,% 2 === 0 → true；ANY=true；长度≥20=true（实际=79） | `d13fd1a0eef6…` |
| C-CMMLU-PROG-01 | 代码 | chat | llm-gateway | 否 | 623 | ✅ | 🟡 | 期望选项=A；字母命中=true；关键字命中=true | `68ce52215a7f…` |
| L-MMLU-01 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 34848 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `aac3246f219f…` |
| L-MMLU-02 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 34358 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `a70cd104b91b…` |
| L-SUDOKU-1 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 34368 | ✅ | 🟡 | 提取数字=[2,4,8,16,32,2,4,8,16,32,1,0.33,2026,-8,-24,4,3,55.966,0.33,1,14.4,0,2026,-8,-24,4,3,55.967,1,64,2,2,2,4,4,2,8,8,2, | `c2e288db2010…` |
| L-CMMLU-L-01 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 27953 | ✅ | 🟡 | 期望选项=C；字母命中=true；关键字命中=true | `7feb3d76d479…` |
| L-CMMLU-L-02 | 逻辑 | reasoning | ultimate-ai-engine | 否 | 34721 | ✅ | 🟡 | 期望选项=A；字母命中=true；关键字命中=true | `bbd0f89b3789…` |
| K-WORLD-01 | 知识 | chat | llm-gateway | 否 | 679 | ✅ | 🟡 | ANY命中=true；禁止词命中=undefined | `903f8c2a9c37…` |
| K-WORLD-02 | 知识 | chat | llm-gateway | 否 | 1157 | ✅ | 🟡 | ANY命中=true；禁止词命中=false | `bbc6d565decd…` |
| K-CMMLU-K-01 | 知识 | chat | llm-gateway | 否 | 540 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `7ee603b8219c…` |
| K-CMMLU-K-02 | 知识 | chat | llm-gateway | 否 | 957 | ✅ | 🟡 | 期望选项=C；字母命中=true；关键字命中=true | `f92f5c89cb90…` |
| K-CMMLU-K-03 | 知识 | chat | llm-gateway | 否 | 1257 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `c546e021f054…` |
| ZH-CMMLU-01 | 中文 | chat | llm-gateway | 否 | 558 | ✅ | 🟡 | 期望选项=A；字母命中=true；关键字命中=true | `a3a0b5114c44…` |
| ZH-CMMLU-02 | 中文 | chat | llm-gateway | 否 | 743 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `6796d295d047…` |
| ZH-POLY-01 | 中文 | chat | llm-gateway | 否 | 604 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `98f658f255f7…` |
| ZH-CMMLU-03 | 中文 | chat | llm-gateway | 否 | 802 | ✅ | 🟡 | 期望选项=C；字母命中=true；关键字命中=true | `015dcc4d6ef8…` |
| ZH-CMMLU-04 | 中文 | chat | llm-gateway | 否 | 574 | ✅ | 🟡 | 期望选项=B；字母命中=true；关键字命中=true | `c181e527b4e4…` |
| T-TODAY-01 | 时效性 | chat | llm-gateway | 否 | 604 | ❌ | ❌ | 关键字命中=false；年=2026→true；月=8→true；日=23→false | `cffd66be662c…` |
| T-TIMEZONE-01 | 时效性 | chat | llm-gateway | 否 | 616 | ✅ | 🟡 | ANY命中=true；禁止词命中=undefined | `ee1ab3435e2e…` |
| I-INST-01 | 指令遵循 | chat | llm-gateway | 否 | 631 | ✅ | 🟡 | 行数=3(期望3)；逐行正则匹配=3/3；lines=["姓名：张三","职业：软件工程师","工龄：10"] | `abb50b998bec…` |
| I-INST-02 | 指令遵循 | chat | llm-gateway | 否 | 639 | ✅ | 🟡 | 解析 obj={"name":"Alice","age":30}；错误=无 | `3e27ab4b2ff7…` |
| I-INST-03 | 指令遵循 | chat | llm-gateway | 否 | 565 | ✅ | 🟡 | 归一化后='olleh' 期望='olleh' | `0baf982fcab3…` |
| I-INST-04 | 指令遵循 | chat | llm-gateway | 否 | 570 | ✅ | 🟡 | 行数=3（期望3）；每行长度/无编号=true；lines=["苹果","香蕉","橙子"] | `778dfb9c9a2a…` |
| I-INST-05 | 指令遵循 | chat | llm-gateway | 否 | 550 | ✅ | 🟡 | 提取数字=[1024] 期望=1024；宽松关键字命中=true | `e39eef82f61b…` |

## 3. 失败题 原始答案 + 判分理由（便于定位失败原因，不放"骗分"分析）

### T-TODAY-01【时效性】严格=false 宽松=false 延迟=604ms

- **题目**：今天是哪一天？请用 YYYY年MM月DD日 或 YYYY-MM-DD 的格式写出具体日期。注意：回答必须基于你获得的当前时间信息，不要凭记忆猜测。
- **期望答案类型**：date_exact
- **评分 Note**：关键字命中=false；年=2026→true；月=8→true；日=23→false
- **实际答案原文（限 800 字符）**：
```
2026年08月24日
```


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

# 专家联盟全域文档设计与治理整理交付

日期：2026-10-01（Asia/Shanghai）。范围：用户确认“完整文档设计与治理整理”。本报告为交付证据，不替代架构与运行事实源。

## 交付内容

| 交付 | 内容 | 入口 |
|---|---|---|
| 全目录盘点 | 所有文件元数据、标题、章节摘要、原文摘录、图源、显式引用和内部关联边 | [全量目录](docs-architecture-corpus.md)、[机读图谱](../data/docs-architecture-corpus.json) |
| 架构与业务归一 | 主要架构家族、三种分层映射、权威关系、10类业务流程、异常/恢复图、7类已发现冲突 | [17图谱](../../docs/expert-alliance/17-docs-architecture-and-flow-atlas.md) |
| 产品与模块设计 | 10个逻辑模块、命令/查询/事件边界、数据主源、16个质量场景、10个有依赖任务 | [18设计](../../docs/expert-alliance/18-modular-product-design.md) |
| 决策记录 | 保留混合运行拓扑、先统一逻辑与证据、按瓶颈拆分；状态Proposed | [ADR-17](../../docs/enterprise/45-专家联盟模块化归一与证据治理-ADR-17.md) |
| 入口治理 | docs/README、架构中心、HTML中心、联盟INDEX/README/index.html、归一化枢纽和企业索引 | [联盟入口](../../docs/expert-alliance/README.md) |
| 口径修复 | N4/N7/N8后端闭环与完整验收分离；登记状态与健康分离；主/备用匹配器分离；旧流程来源提示；政务样例状态 | [当前实现增量](../../docs/expert-alliance/CURRENT-ARCHITECTURE.md#matching-20261001) |

最终盘点分母与统计以机读JSON为准：576文件，549可读文本，155份Mermaid图源，3,128条显式引用，1,632条指向docs内文件的EXTRACTED关联边。包含345活动候选、127归档、104过程证据。活动候选不等于已确认权威。新增架构与流程图13张。全量结构覆盖不等于逐篇语义核验或业务生产验收；图源提取不等于所有历史图语法均通过。

脚本使用Python标准库，复用现有链接门禁解析。图边只来自实际引用，并带来源文件/行号；无推断调用关系或虚构成熟度评分。无额外模型API调用，结构提取的模型API费用为0；当前会话token消耗未单独计量。

## 核验结果

| 检查 | 结果 | 证据与限制 |
|---|---|---|
| docs链接门禁 | **43断链→0** | [整理前](../data/20261001-doc-links-before.json)、[整理后](../data/20261001-doc-links-after.json)；默认门禁，不是strict全绿 |
| 链接门禁自检 | 14 PASS / 0 FAIL | `python scripts/gate/check-doc-links.py --selftest`；门禁不验证章节锚点 |
| 新文档章节锚点 | 39 PASS / 0 FAIL | [锚点核对](../data/20261001-ea-anchor-verification.json)；四份新文档中的本地md引用，按标题slug与显式id校验 |
| 新Mermaid图 | 13 PASS / 0 FAIL | [图语法核对](../data/20261001-ea-mermaid-verification.json)；本地Mermaid11.17.2 parse，未做浏览器视觉验收 |
| 盘点自检 | PASS | CRLF/BOM、归档分型、图章节、代码示例不误计、引用去向、源变更导致对账变化 |
| 盘点可复跑 | `--check` PASS | 输出固定排序、源文件SHA-256；新增/修改资料须重生成 |
| 全仓代码目录 | 重生成后`--check` PASS | 从143过期计数更新为实际149；前端迁移引用同步 |
| 端口门禁 | ERROR=0，WARN=284，扫描397 | [端口报告](../data/20261001-port-verification.json)；警告未逐条分类，不能称无端口技术债 |
| 差异空白检查 | PASS | 本轮涉及docs与盘点脚本的`git diff --check`；CRLF转换提示不是失败 |

本轮不涉及Rust/Vue运行行为变更，未运行cargo全量构建/测试或模型调用E2E。代码中已有测试和总账记录是来源证据，未冒认本轮重新执行。新增脚本做了正确性、可读性、边界、安全、性能五轴检查：离线读取、输出仅reports、无新依赖、全语料约数秒执行、明确覆盖局限。

## 修正的关键事实

1. **主匹配器**ModularWeightMatcher：健康分1.0/0.2，读取实际权重，默认健康权重0.05；有租户、Active、优先级、领域与总分低于0.2的过滤。
2. **备用匹配器**matcher：健康分1.0/0.3，健康权重0.15。此前多个文档混用，已同步CURRENT/08/13/15。登记availability不等于调度健康或ExpertStatus。
3. **实例内并发护栏**不等于每专家预算、租户配额、QPS限制或集群并发；不健康不是独立硬过滤，但健康变化可能使总分低于门槛。
4. **当前存储/协议**与SQL目标模板、gRPC框架能力、早期WS设计分开；图谱投影不能替代业务交易事实。
5. **产品最优性结论**：现有混合模块化方向合理，但无全局最优/容量/恢复实测证明。优先事实→体验→契约→安全接纳→恢复→事件投影；更多服务不自动提高质量。

具体代码复核点：modular_matcher.rs:170–171/197–207/228–250/279–282、common-proto/types.rs:1213–1217、matcher.rs:122/166、dag_engine.rs:32/158/584–587、experts_graph.rs:1321–1324、actuator.rs:627–632。行号为本轮工作区核对快照，后续变动时重新定位。

## 残余治理工作与实施边界

默认链接门禁仍有207条反引号路径告警、132条file://引用技术债；本轮修复现行导航，不无依据重写所有历史文本。旧业务流程正文仍需按实际模块重核；L1/L2和过程报告里的日期快照不可自动升格为实时事实。

目标设计待实施项：专家目录唯一写源及同步规则、幂等/预算、计划与尝试持久化、恢复对账、SSE持久重连、画布编辑体验、联盟MCP接线、私有交付包。质量场景EA-Q01–16中的建议数值不是本轮基准。ADR-17保持Proposed，不代表组织已会签。

所有原有未提交工作、运行时数据与归档保留；本轮新增报告只放reports。本轮未提交或推送，工作区可继续评审。

## 后续复跑

```powershell
python scripts/doc/inventory-architecture-docs.py --selftest
python scripts/doc/inventory-architecture-docs.py
python scripts/doc/inventory-architecture-docs.py --check
python scripts/registry/module_catalog.py --check
python scripts/gate/check-doc-links.py --selftest
python scripts/gate/check-doc-links.py
```

外部方法核证日期2026-10-01：[C4](https://c4model.com/diagrams)、[arc42](https://arc42.org/overview/)、[WCAG2.2](https://www.w3.org/TR/WCAG22/)、[OpenTelemetry signals](https://opentelemetry.io/docs/concepts/signals/)。用于组织视图与可验证质量目标，不构成外部认证。

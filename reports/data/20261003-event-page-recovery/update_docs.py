# -*- coding: utf-8 -*-
from pathlib import Path

root = Path(__file__).resolve().parents[3]
items = {
    'docs/expert-alliance/CURRENT-ARCHITECTURE.md': '''
### 2026-10-03 页面恢复与模块回归增量

专家页与编排页共用事件恢复控制器、可见状态栏和手动重连；开流/缺口重新查询权威数据，身份变化清游标并阻止选定查询的迟到回填。订阅禁止 HTTP 缓存，避免 410 阻断实际重试。低代码声明与全局 store 的循环依赖已切断，消息中心导航已补齐，编排来源按当前 Rust 的模型分析/真实终态/真实计时修正。行为权威见 [事件契约 §9](21-event-delivery-contract.md#9-页面恢复与迟到响应归属2026-10-03-增量)，验证见 [页面恢复报告](../../reports/markdown/20261003-event-page-recovery.md)。该浏览器验收限定于专家页面恢复共享组件及 store，不代表全部业务页面、真实外部模型、OSS 或多实例验收。
''',
    'docs/expert-alliance/INDEX.md': '''
页面恢复与迟到响应归属权威：[21 事件契约 §9](21-event-delivery-contract.md#9-页面恢复与迟到响应归属2026-10-03-增量)，运行证据：[页面恢复与全量前端回归报告](../../reports/markdown/20261003-event-page-recovery.md)。
''',
    'docs/expert-alliance/16-decision-and-state-ledger.md': '''
## 2026-10-03 页面事件恢复与模块融合记录

EA-EVT-12/13：统一恢复控制器、状态组件、开流与缺口刷新、显式内存游标重连、禁止缓存、身份及请求归属保护。低代码循环依赖与消息中心导航漏装配已修复，编排来源与当前 Rust 对齐。需求流程权威仍在 [21 §9](21-event-delivery-contract.md#9-页面恢复与迟到响应归属2026-10-03-增量)；结果与开放边界见 [验证报告](../../reports/markdown/20261003-event-page-recovery.md)。不以全量前端单元回归替代全模块真实依赖验收。
''',
    'docs/modules/REAL-IMPLEMENTATION-STATUS.md': '''
页面恢复增量以 [事件交付契约 §9](../expert-alliance/21-event-delivery-contract.md#9-页面恢复与迟到响应归属2026-10-03-增量) 为主源：共享恢复模块、专家与编排页面装配、选定请求迟到保护、缓存隔离和导航修复。前端全量测试与真实 Rust/浏览器证据见 [页面恢复报告](../../reports/markdown/20261003-event-page-recovery.md)。单元测试仍有历史局部替身，不作为真实外部模型/OSS/跨实例功能完成证据；26 模块的未闭合退出条件保持开放。
''',
    'docs/specifications/tasks/20261002-enterprise-module-normalization/todo.md': '''
- [x] T9e 专家与编排页面统一恢复模块、开流/缺口合并刷新、可见错误、手动续传、身份清游标、禁止缓存及选定迟到响应保护；模块循环依赖、消息中心漏导航和编排来源误标修复。验证范围与证据见 reports/markdown/20261003-event-page-recovery.md。
- [ ] T11 继续按 26 模块退出条件补齐真实模型与工具执行、知识源版本/加工/图检索、OSS、持久投递与跨实例恢复，逐模块做完整业务页面验收；全量单元回归不替代这些条件。
''',
}
for rel, addition in items.items():
    file = root / rel
    source = file.read_text(encoding='utf-8')
    if addition.strip() not in source:
        source += '\n' + addition
    if rel.endswith('CURRENT-ARCHITECTURE.md'):
        source = source.replace('没有 outbox、历史重放、签名、DNS', '没有 Webhook outbox、签名、DNS')
    file.write_text(source, encoding='utf-8')

report = root / 'reports/markdown/20261003-event-page-recovery.md'
if not report.exists():
    report.write_text('''# 2026-10-03 页面事件恢复、模块融合与回归

本轮范围为事件恢复共享模块及专家/编排页面装配、指定 store 请求归属、低代码声明依赖、消息导航和编排字段来源。所有生产功能沿用真实接口，不添加业务成功兜底。

最终命令、退出码、数量与边界以 [证据汇总](../data/20261003-event-page-recovery/summary.json) 为准；首次失败及诊断日志保留在同目录。事件权威与业务流程见 [21 §9](../../docs/expert-alliance/21-event-delivery-contract.md#9-页面恢复与迟到响应归属2026-10-03-增量)。

新增 Rust 验收启动真实 JWT、生产路由和临时 SQLite；Node 恢复控制器验证实际广播积压/失效游标/身份隔离，浏览器对真实响应验证状态提示、重连、事件触发列表刷新及旧租户统计不回填。测试专用广播故障路由只属于 integration fixture，不进入产品。浏览器独立上下文预置测试签发 JWT，只验证业务恢复，不覆盖登录表单或完整页面路由。

历史单元套件含局部 API/组件替身，新收藏批量读取已经接入真实 Rust/JWT/SQLite。全量测试通过只表示其声明覆盖；不能据此称所有功能模块已经生产可用。

仍开放：Webhook durable outbox/签名/死信与恢复、计划事务及跨实例 CAS/广播、真实外部模型和工具执行、统一知识源与 OSS、26 模块完整业务页面和生产性能验收。默认 Cargo 测试不包含所有非 default-members 绑定，存量 ignored 测试另列，不计为通过。
''', encoding='utf-8')
print('Updated canonical links and scoped evidence report')

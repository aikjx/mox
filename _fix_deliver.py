f = "d:/a10/aikjx/gitcode/infotopograph/reports/markdown/专家联盟mox 模块化系统架构开发交付报告.md"
c = open(f, encoding="utf-8").read()

old1 = '只保留中间件分层职责。联盟域状态映射归一为'
new1 = '只保留中间件分层职责；④ 全量域状态纳管：6 套模块私有状态（monitor/workspace/projects/misc/kb_ext/notification）收口到注册中心，`build_*_router` 改为接收 `Arc<State>`、不再各自 `new`，新增 `test_module_states_owns_all_domain_states` 用例验证唯一真源，全量 80 + 8 + 10 全绿。联盟域状态映射归一为'
c = c.replace(old1, new1)

c = c.replace('新增 4 个用例，全量 79 + 8 + 10 全绿', '新增 5 个用例（映射归一 2 + 状态纳管 3），全量 80 + 8 + 10 全绿')

open(f, "w", encoding="utf-8").write(c)
print("done_deliver")

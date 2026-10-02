# 事务收件箱与企业健康报告验证

2026-10-02。增量范围：网关消息中心、企业健康接口、API 清单生成器及关联文档。实际实现与退出条件以[消息中心契约](../../docs/modules/message-center/README.md)为准；竞品增量分析见[分项对标](../../docs/modules/resource-knowledge/COMPETITIVE-DESIGN.md)。

消息与回执从内存迁入真实文件 SQLite 事务；增加作用域幂等键、数据库分页/统计和已读回执一致性。生产 JWT + 两个真实 TCP 服务验证服务重建后的消息可见、重试同 ID、越权 404/伪造租户 403、输入校验以及健康探测。企业健康接口不再把全部模块固定标为 ready；API 生成器不再把静态注册数当作已验收功能数。

API 生成器原先漏渲染 5 个域的 23 条接口，现按实际分组补齐；243 条静态接口全部渲染且无重复，生成两次内容一致并保留非 HTTP 的 MCP 附录。仓库级链接检查由 40 条断链收敛至 0：修复部署文档层级、模块证据链接与设计页控制台入口；历史报告里的原始断链文本改用代码块展示，保留原始字面记录，没有改写当时结论。

首轮 12 实例并发初始化失败，保留 concurrent-red.log 和 concurrent-localized.log。定位为 WAL 设置期间数据库锁冲突，增加仅针对初始化锁的有界重试；不通过跳过并发测试或降低断言获得通过。回执写入失败通过真实 SQLite 触发器检验整笔事务回滚。

## 验证范围

| 项目 | 证据与结论 |
|---|---|
| Rust 专项回归 | final-tests.log：12 项通过，包含 5 个持久化用例、2 个真实 HTTP 用例、3 个交付真值回归、2 个可信租户回归 |
| 并发复测 | concurrency-repeat.log；重复首次初始化竞争，结果见 summary.json |
| 全目标 lint | clippy-final.log；结果及存量警告见 summary.json |
| 文档与目录治理 | doc-links-final.log：断链 0、路径告警 215；module-catalog.log、script-paths-final.log、frontend-gate.log、ports.log 通过，端口仍有 284 条告警 |
| 注册表覆盖与重复生成 | registry-consistency.log：1 项 Python 回归通过；验证静态接口完整性、唯一性、稳定生成与附录保留 |
| 流程图 | mermaid.log：消息事务流程语法通过；不是视觉或浏览器业务验收 |
| 编译与源码边界 | lint 编译所有目标；没有新增依赖，没有提交、部署或删除运行数据 |

本批专项通过不代表全 workspace、前端浏览器 E2E 或所有企业功能完成。文件数据库测试模拟服务重建，不包含断电、主机宕机或跨主机数据库故障转移。SMTP、LLM、OSS 正向真实提供方链路未在本批验收。当前 SQLite 部署限定同主机，跨用户交付、集群主源、outbox、租约/取消、容量与灾备仍在模块退出条件中登记。

治理命令发现旧 `tools/module_catalog.py` 已迁到 `scripts/registry/module_catalog.py`；使用实际路径验证，历史命令不作为功能失败结论。完整机器可读结果及运行范围保存在 `reports/data/20261002-inbox-consistency/summary.json`。

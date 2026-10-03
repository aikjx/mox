# Webhook 模块开发、验证与控制台融合

日期：2026-10-03。范围为专家联盟事件订阅管理这一条纵向链路。功能需求仍取自企业功能登记，主题权威为 [事件契约 §5](../../docs/expert-alliance/21-event-delivery-contract.md#5-2026-10-03-webhook-管理增量与业务流程)，部署变量仍归 [部署模板](../../docs/expert-alliance/09-deployment-templates.md)。未提交、推送、部署或改动业务运行数据库；测试使用独立临时 SQLite 和随机本机端口。

## 1. 修复与模块关系

原管理接口只校验 URL 前缀，普通登录用户可以配置外部事件投递；内存先改变、SQLite best-effort 失败仍返回成功；过滤 JSON 损坏被恢复为全部事件。新增真实测试先复现普通用户创建返回 200，而要求是 403，原失败日志保留在 [red.log](../data/20261003-webhook-management/red.log)。

修复后 GET/POST/DELETE 共用现有 JWT、认证租户和 RBAC；SQLite 提交成功才更新投递投影，失败 503 且原状态保留；损坏过滤不恢复。URL 使用标准解析和运维精确 origin 授权，默认全部拒绝；投递检查历史目标、禁止 302 跟随和环境代理，错误日志不输出目标 URL。没有新增依赖、模拟服务或削弱断言。

前端沿用端点目录 → API 信封 → 严格对象投影 → 独立 store → `WebhookSubscriptions` → 联盟控制台。显式读取订阅、创建、确认删除、分页和失败状态；身份变化清空地址、表单、删除确认，迟到响应不能覆盖新租户。已确认提交与后续刷新失败分别反馈，防止因刷新失败误导重复创建。

## 2. 实际验证

| 项目 | 结果 | 可复核证据 |
|---|---|---|
| 7 个 Rust 集成测试目标 | **23 passed / 0 failed**；其中 webhook 管理与持久恢复各 1 个综合测试，不将内部断言重复计为用例 | [命令与退出码](../data/20261003-webhook-management/rust_release.json)、[完整日志](../data/20261003-webhook-management/rust_release.log) |
| 实际前端 store 联调 | 实际 Pinia/Axios → TCP 代理 → Rust/JWT/SQLite，11 次真实请求；CRUD、迟到响应、跨租户、伪造本地角色被真实服务器拒绝、注销阻断 | [探针](../data/20261003-webhook-management/verify-store.mjs)，输出在上述 Rust 日志 |
| 定向前端契约 | **157 passed / 0 failed**；101 接口契约、37 会话契约、19 词表归属 | [命令与退出码](../data/20261003-webhook-management/contracts_release.json) |
| 接口接线门槛 | 实际挂载新增两个 registry ID，**74/85 = 87.06%**，原门槛 ≥85% 保持不变；移除相应未接入条目 | `contract.test.js` 真实源码接线扫描，见契约日志 |
| 前端生产构建 | 通过；保留依赖 PURE 注释警告，未宣称全项目零警告 | [build.log](../data/20261003-webhook-management/build.log) |
| 定向 Clippy | 退出 0；库仍有 **69 项既有 warning**，不是 `-D warnings` 通过；新管理测试无 warning | [命令与退出码](../data/20261003-webhook-management/clippy.json) |
| 模块、框架 import、API binding、反馈 import、路径、端口、文档、图表、差异检查 | 按最终结构化摘要逐条记录，失败保留并修复后重跑 | [本轮摘要](../data/20261003-webhook-management/summary.json) |

真实网络接收端收到了 A 租户 `ExpertRegistered` 事件；B 租户和不匹配事件无投递。受信 origin 的 302 接收端没有把载荷转发给目标。INSERT/DELETE SQLite 触发器真实报错时接口 503，当前热投影及重新构造 state 后的内容一致。重新构造 state 是同进程持久恢复验证，不能描述为进程 kill 或跨主机恢复。

第一次扩大回归命令误用了不存在的 `trusted_tenant` 测试目标，退出 101；记录保留在 `rust.json`，更正为真实 `trusted_tenant_http` 后上述 7 个目标完整通过。首次前端选择命令还包含不存在的 vocabulary 文件名，实际只跑了 138 项；最终使用 `vocabulary-ownership.test.js` 并确认 157 项，未把首次漏选误报为完成。

## 3. 五维复核及后续边界

正确性：CRUD 和保存失败有实际 HTTP/SQLite 故障证据；创建响应不确定时未实现幂等，必须重新读取核对。架构：复用现有身份、权限、存储和前端出口，无第二套路由或需求权威。安全：默认禁止出站、精确 origin、禁止凭据/query/fragment、禁重定向和代理；域名目标仍依赖受信 DNS，尚无 DNS 地址钉住和签名。性能：查询分页但内存列表扫描及同步 SQLite 仍存在；串行投递无持久背压/死信。可维护性：独立出站策略、store 与组件，主题文档包含真实业务流程。

这一增量只是 EM-02/EM-10/EM-26 的部分要求通过，仍未完成完整浏览器交互、远程 CI、压测、创建配额/限流、幂等、独立健康、跨实例同步、outbox 与可靠投递。其他模块逐项继续验证，不能据此声明全部功能或企业生产验收完成。下一项应以持久事件 outbox 的提交与消费边界为先，单独测试后再融合。

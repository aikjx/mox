# 企业功能、需求与模块关系总入口

> ENT-CAP-01 · 2026-10-02。范围：platform、frontend-ui 与其产品接缝。功能需求主源为 [registry.json](registry.json)，模块卡及关系图由该源生成；现状事实保留在各模块权威，不在此重写完成状态。

## 阅读顺序与权威

| 要回答的问题 | 入口与权威 |
|---|---|
| 各模块负责什么、输入输出和验收需求是什么 | [逐模块需求与流程图](MODULES.md)，EM 模块编号及 R 需求编号 |
| 哪些能力必须先做、模块如何关联 | [前置能力与模块关系图](RELATIONSHIPS.md) |
| 用户任务怎样贯穿模块、失败与恢复在哪里 | [跨模块业务主链](BUSINESS-FLOWS.md) |
| 实际有哪些 crate、入口和依赖 | [代码目录](../CODE-CATALOG.md)，从 workspace 生成 |
| 接口和端口是什么 | [API 注册表](../../API-REGISTRY.md) · [端口注册表](../../api/PORT-REGISTRY.md) |
| 哪些已经真实验证、哪些尚未完成 | [平台实现台账](../REAL-IMPLEMENTATION-STATUS.md) · [资源知识台账](../resource-knowledge/IMPLEMENTATION-STATUS.md) · [联盟状态总账](../../expert-alliance/16-decision-and-state-ledger.md) |
| 低代码配置如何发布、冻结和执行 | [配置标准](../../standards/lowcode-dynamic-configuration.md) · [页面运行架构](../../architecture/frontend/LOWCODE-PAGE-RUNTIME.md) |
| 本轮按什么顺序开发 | [实施计划](../../specifications/tasks/20261002-enterprise-module-normalization/plan.md) · [待办](../../specifications/tasks/20261002-enterprise-module-normalization/todo.md) |

登记源覆盖当前 platform/domains 的全部实际域。逻辑能力可以对应多个 crate，同一 crate 也可提供多个接缝；逻辑模块、编译单元、路由分组和部署进程不是可相加的同一种计量。数量由生成校验器读取，不在概览手写另一份。

## 产品任务与责任

业务用户提出目标、管理项目和知识并接受成果；管理员管理实际租户/权限与业务配置；专家维护能力、执行证据和质量；知识维护者管理源版本、发布和事实确认；运维维护运行依赖、秘密、容量、灰度与恢复。角色是业务责任建议，未指派个人。系统权限仍由 IAM 实际数据裁决，不能以页面角色标签替代授权。

每项需求以 EM-xx-Rnn 稳定标识。既有需求顺序和编号视为引用契约，追加需求时只在尾部添加；删除或拆分需保留迁移映射，不能挪动编号让旧证据指向另一项。每个业务模块进一步细化案例时记录：角色、触发、前置条件、输入/输出、状态变迁、权限、幂等、一致性、失败/未知/恢复、性能与证据。命令、查询、事件和副作用回执分别建模。

## 企业交付判据

需求定义 → 接口与数据契约 → 真实实现 → 正反权限与故障验证 → 页面真实流程 → 重启/恢复 → 容量/SLO → 发布回滚。每个出口独立闭合；生成图、构建成功、路由注册、HTTP 200 与单元测试数量都不能提升为“企业级完成”。

真实分布式要求跨主机故障、数据库约束、租约/fencing、撤权新鲜度与灾备证据。SQLite 同主机多连接只证明其局部一致性。模型分析不代表工具执行，提供方接受不代表最终交付；未知状态不能填成功或假进度。无真实凭据的提供方保持未验收，不用 mock 接口补齐交付。

优化以用户任务完成率、授权正确性、故障恢复、实测延迟/吞吐/成本和维护复杂度衡量。先闭合主源与契约，再按测量拆分服务；保留已有模块能力，不为每个逻辑模块新建微服务或新语言运行时。

## 持续维护

```powershell
python scripts/registry/enterprise_capabilities.py
python scripts/registry/enterprise_capabilities.py --check
python -m unittest discover -s scripts/tests -p test_enterprise_capabilities.py
python scripts/gate/check-doc-links.py --repo
```

校验器检查域覆盖、ID、代码和权威路径、前置引用、前置环以及生成漂移。它校验文档结构与关联，不检验生产 readiness、业务算法正确性或服务可达性。验证报告归档到 reports，不写回旧验证批次。

现有 CI 的 frontend-governance 已加入登记校验、登记器单元测试和真实发送/凭证/审计 Node 契约测试。CI 云端执行结果须由实际流水线返回；本轮记录本地同命令验证，不声称已运行远程 CI。

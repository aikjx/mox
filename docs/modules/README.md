# 功能与模块导航

本页说明功能应放在哪里；[全仓代码目录](CODE-CATALOG.md)列出实际模块、依赖、可执行入口和前端文件。[API 注册表](../API-REGISTRY.md)维护接口映射，[端口注册表](../api/PORT-REGISTRY.md)维护部署端口。

## 按功能查找

平台与前端当前开发状态、真实链路及未完成项见[真实实现验收台账](./REAL-IMPLEMENTATION-STATUS.md)。

[消息中心](../README.md)：事务站内箱、幂等、已读回执、实际健康探测与集群演进边界。

[IAM 业务权限管理](../README.md)：真实权限主源、版本化角色授权、事务审计、旧入口保护与统一有效权限策略。

| 功能 | 后端归属 | 前端页面 | 路由模块 |
|---|---|---|---|
| 项目、任务与资源 | project / data | project | project |
| AI 对话与智能体 | ai | ai | ai |
| 知识图谱与知识库 | kg / kb | graph | graph |
| 流程设计与执行 | flow；AI 优化由 ai SDK 提供 | workflow | workflow |
| 专家任务、执行与结果 | alliance；专家能力由 ai 提供 | expert / workspace | alliance |
| 应用与算子商城 | market | market | market |
| 算子目录与执行能力 | flow / platform | operators | operators |
| 租户、权限与系统配置 | platform / foundation / shared | admin | system |
| 登录、门户与错误页 | 网关认证与宿主 | auth / misc | public / fallback |
| 云盘与对象存储 | cloud | 见代码目录与 API 注册表 | 按实际路由挂载 |
| 语音与独立产品 | voice / projects | 独立项目入口 | 按产品配置 |
| 通用模型、存储与基础能力 | base / foundation / shared | 不独立定义业务页面 | 无 |

前端路由定义位于 `frontend-ui/src/router/modules/`，路由宿主 `router/index.js` 统一组装顺序、鉴权、标题及导航生命周期。页面仍位于现有 views 业务目录。跨域调用通过 API/SDK；领域实现不能导入路由宿主或前端组件。

## 新功能落位规则

1. 先确定所属业务域，复用已有模块；一个功能不因多个入口而复制多套实现。
2. 外部契约放 api/proto，领域计算放 core，客户端适配放 sdk，服务生命周期与 IO 组装放 svc。存量 core 并非全部无 IO，应以实际依赖检查结果为准。
3. 网关统一入口、认证与转发；宿主组装模块，不新增领域算法。
4. 页面、API 客户端、状态与可复用组合逻辑分别放 views、api、stores、composables；在所属路由模块注册页面，权限策略仍由统一路由宿主管理。
5. 独立产品放 projects；验收产物进入 reports，运行数据留在受忽略的运行时目录。代码目录中的无入口子目录需逐项确认，不能直接当成已上线产品。
6. 文档进入对应 docs 分类，报告进入 reports/html、reports/markdown 或 reports/data。端口修改同步权威注册表。

## 持续维护

```powershell
python scripts/registry/module_catalog.py
python scripts/registry/module_catalog.py --check
python scripts/gate/check-frontend-module.py
python scripts/gate/verify-ports.py
```

代码目录自动读取完整 workspace（包括非默认成员），并列出运行、开发、构建依赖。前端扫描覆盖路由、页面、API、状态和组合逻辑；文件存在不表示已接通后端。目录检查负责发现清单漂移，架构门禁负责检查依赖边界，两者不能代替功能验收。

<a id="lowcode-directory-design"></a>
## 目录架构设计卡：低代码与动态配置（2026-10-01）

> 目标设计接缝；既有正文按原日期/类型解释，未实施能力不标已完成。

| 设计项 | 本目录约定 |
|---|---|
| 输入 | 领域能力、版本化业务配置与行业场景 |
| 处理与边界 | 定义最小业务包、引擎适配、装配依赖、安装升级退出和业务样例 |
| 输出 | 可组合模块与正常/失败/恢复流程 |
| 维护角色 | 业务模块owner（角色建议，未指派个人） |
| 配置语义 | [统一规范](docs/standards/lowcode-dynamic-configuration.md#model)，本目录不复制覆盖/生命周期规则 |
| 本目录设计 | [详细接缝](docs/modules/LOWCODE-MODULE-ASSEMBLY.md#assembly) |
| 验收 | 类型/依赖/权限/版本/异常/恢复按相关LC-Q条目补证；设计完成与运行验证分开 |

全目录关系见 [目录矩阵](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories)。新增架构文档应符合 [文档设计契约](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#document-contract)，各主题拥有自己的事实主源。

## 资源知识主题设计

[系统、云盘、知识库、图谱与OSS](docs/modules/resource-knowledge/README.md#map)：七个逻辑模块、数据归属、六条业务流程与分阶段验收；目标设计待评审。

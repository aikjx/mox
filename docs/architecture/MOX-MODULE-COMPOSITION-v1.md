# MOX 可复用模块装配契约 v1

目标是让项目复用明确的契约和领域能力，并能选择部署组合。统一入口不意味着所有业务必须编译进同一个服务；数据库驱动也不意味着复杂算法、外部调用和补偿逻辑可以取消领域代码。

## 模块边界

| 边界 | 职责 | 本轮实现 |
| --- | --- | --- |
| 基础契约 | 公共类型、错误身份，不引用业务实现 | `mox-platform-foundation::operator_error`，旧算子路径保持类型再导出 |
| 领域核心 | 业务算法、验证、资源约束，可脱离 HTTP 测试 | `mox-flow-optimizer-core::execution` |
| 领域适配 | 把领域能力接入 HTTP 或存储，不依赖具体宿主 | `mox-alliance-http-sdk`、`mox-cloud-admin-sdk` |
| 装配策略 | 校验模块标识、契约主版本、依赖、路由归属，记录实际初始化结果 | `mox-platform-module-core`，仅依赖 serde/thiserror |
| 宿主 | 配置、认证、并发接纳、审计、监听和生命周期 | operator-server、平台网关 |

跨域的流程图模型与优化能力使用 `mox-ai-flow-sdk`。仅需要公共错误的算法库依赖基础契约，不再拉入完整算子执行库。Cloud API 只保留 DTO 与 trait；具体存储管理调用方改用 `mox_cloud_admin_sdk::StoreAdmin` 和 `assemble_backend`。原 Cloud API 的 `admin` feature 与实现路径已移除，这是需要调用方迁移的接口调整。

联盟 HTTP 适配器可由任意 Axum 宿主挂载：

```rust,ignore
use mox_alliance_http_sdk::{build_alliance_router_with, RemoteAllianceClient};
let remote = RemoteAllianceClient::explicit(
    Some("http://127.0.0.1:33100".into()),
    Some("http://127.0.0.1:33200".into()),
);
let router = build_alliance_router_with(remote);
// 宿主须安装认证和租户授权后才能对外提供服务。
```

适配器保持旧网关的公开再导出和 HTTP 路径。远程故障不会切换到本地数据源；历史本地预览仍有兼容代码，不能把该模式当作真实模型执行。

## 部署清单与初始化

清单结构见 `docs/api/mox-module-manifest.schema.json`。每个模块声明标识、契约主版本、是否必需、依赖与路由前缀。校验必须先于初始化：

1. 拒绝未知字段、无效标识、重复标识、路由前缀重叠。
2. 拒绝依赖缺失、契约主版本不兼容和依赖环。
3. 按确定的拓扑顺序初始化；依赖没有成功初始化时，不执行下游初始化。
4. 必需模块失败，宿主启动失败；可选模块失败，宿主可继续，但初始化报告标为降级。
5. 启动报告只证明初始化结果。持续健康检查、服务调用可用性和数据持久化应分别验收。

operator-server 已接入清单，支持 `mox-viz`、`mox-system`、`primiflow`、`fusion` 四个内嵌模块，路径不可由清单任意覆盖。指定其他模块需要实现宿主适配器，清单不会执行任意代码。

```powershell
$env:MOX_MODULES_CONFIG = (Resolve-Path deploy/modules/flow.json).Path
$env:MOX_ORCHESTRATOR_HOST = '127.0.0.1'
$env:MOX_ORCHESTRATOR_PORT = '33300'
cargo run -p mox-platform-orchestrator-svc --bin operator-server
```

`GET /api/runtime/modules` 返回初始化快照，沿用宿主鉴权。清单只选择四个内嵌子服务；编排器原有的其他业务路由仍存在，不是已经完成裁剪的最小独立服务。

没有提供清单时，原 `OUS_ENABLE_*` 开关继续生效，已启用模块默认必需。**行为变更：必需模块初始化失败不再只记日志并跳过。** 空的 `mox-system` 不再自动创建管理员或把管理员令牌打印到启动日志；应由受控初始化流程调用 `MoxSystem::bootstrap`，或在不需要该能力的部署清单中省略该模块。

## 执行处理模式

算子执行的公共输入为步骤列表、输入向量与参数，资源限制由宿主传入，领域核心不读取环境变量。

- 每个步骤使用独立执行标识；重复 `linear` 等算子不再误判成依赖环。
- 预检和执行复用同一批算子实例，避免重复分配矩阵。
- 构造矩阵前检查维度、步骤数、总矩阵分配预算与有限数值；执行后检查结果是否可计算。
- 宿主把计算交给阻塞工作线程，并用并发槽限制接纳量。资源繁忙明确返回失败，不堆积无界等待队列。
- 宿主记录执行结果与拒绝结果。当前该记录仍为进程内日志，不等于持久化审计或事务补偿。

默认维度上限 1024，步骤上限 128，矩阵分配预算 64 MiB，同时执行数 2。宿主可通过 `OUS_EXEC_MAX_DIM`、`OUS_EXEC_MAX_STEPS`、`OUS_EXEC_MAX_ALLOC_BYTES`、`OUS_EXEC_MAX_CPU`、`OUS_EXEC_MAX_MEM`、`OUS_EXEC_MAX_CONCURRENT` 配置；并发值接受 1–64。它们是接纳边界和估算约束，不是操作系统级资源隔离。

## 架构验收

### 蓝图转换归一化

治理、发布和代码生成的设计器蓝图转换统一由 `mox-ai-flow-sdk::blueprint::normalize_blueprint` 提供。宿主只传入场景名称，不维护节点、边或工具类型的第二套转换规则。代码发布现在保留蓝图中的工具类型，与治理入口一致；原有宽松字段默认值保留，转换本身不代替领域校验。

编排服务运行时代码通过 `mox-ai-flow-sdk` 使用流程模型和自动化能力。`mox-ai-flow-svc` 仅保留为服务身份与兼容性测试的开发依赖，因此不能据此宣称声明依赖总数减少或全仓架构门禁通过。

补充配置边界：路由前缀仅接受 ASCII 字母、数字及 `/ - . _ ~`，不接受动态占位符、空白或编码路径；重复依赖声明直接报错。流程输入必须非空且不超过维度上限，检查先于状态向量构造；输入向量直接转移所有权，避免额外复制。空输入请求现在明确失败。

```powershell
python tools/architecture_gate.py --output reports/architecture/current.json
```

统一命令同时执行原正常依赖检查和按实际路径识别的声明依赖检查。任何一套发现 P0/P1 都返回非零；P2 和未分类项保留在报告中。没有提高高扇出阈值，也没有将可选依赖或测试依赖隐藏。

尚需继续拆分的主要聚合点是 `mox-platform-orchestrator-svc`：它仍直接集成多个业务域，并含跨 crate 元数据验证依赖。模块启动策略已统一，但这不等于其职责拆分已经完成。

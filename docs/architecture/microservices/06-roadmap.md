# 06 - 实施路线图

> 版本：v1.0 | 日期：2026-08-26 | 状态：草案
>
> 前置阅读：[00-核心原则](./00-principles.md) | [01-服务边界优化](./01-service-boundaries.md) | [02-通信架构优化](./02-communication.md) | [03-数据架构优化](./03-data.md) | [04-部署架构优化](./04-deployment.md) | [05-可观测性·安全·弹性](./05-observability-security-resilience.md)

---

## 一、总体路线图

```
阶段一：基础建设期（第1-4周）
  ├── 工程化改造（目录重组/命名统一/CI/CD）
  ├── 共享库建设（mox-rpc/mox-config/mox-o11y/mox-db/mox-tenant）
  ├── Proto 定义（所有服务 gRPC 接口）
  └── 基础设施（K8s/PostgreSQL/Redis/NATS/MinIO）

阶段二：核心拆分期（第5-10周）
  ├── 网关拆分（mox-gateway-svc）
  ├── 平台能力（auth/tenant/system/metering/notification）
  ├── 图存储 gRPC 化（mox-graph-storage-svc，零修改引擎）
  ├── AI 服务拆分（mox-ai-svc + Python inference sidecar）
  └── 多租户隔离（L1 逻辑隔离 + RLS）

阶段三：服务化推进（第11-16周）
  ├── 知识图谱服务（graph/graph-algo/graph-streams/graph-meta）
  ├── 数据存储服务（storage/etl/dataplane/search）
  ├── 流程算子服务（flow/flow-fusion/operator）
  ├── 业务治理服务（compliance/fusion/catalog/market/optimizer）
  └── 服务注册发现（K8s Service → Nacos）

阶段四：企业级增强（第17-20周）
  ├── 可观测性完善（OTel/Jaeger/Prometheus/Grafana/Loki）
  ├── 安全加固（mTLS/RBAC/审计/加密/等保三级）
  ├── 弹性容错（限流/熔断/降级/重试/超时/舱壁/DLQ）
  ├── 灰度发布（Istio 金丝雀/蓝绿）
  └── 性能优化（压测/调优/缓存）

阶段五：稳定运营期（第21周+）
  ├── 全链路压测
  ├── 混沌工程
  ├── SLA 保障（99.95%）
  ├── 持续优化
  └── 文档完善
```

---

## 二、阶段一：基础建设期（第1-4周）

### 2.1 目标

- 完成工程化改造，建立统一的开发规范和基础设施
- 建设共享库，为后续服务拆分提供基础
- 定义所有服务的 gRPC 接口契约
- 搭建 K8s 开发/测试环境

### 2.2 任务清单

| 周 | 任务 | 交付物 | 负责人 |
|----|------|--------|--------|
| W1 | 目录重组：crates/libs/services/proto/sdk/deploy | 新目录结构 | 架构师 |
| W1 | 命名统一：所有服务 mox-{domain}-svc | 重命名映射表+代码 | 全组 |
| W1 | 清理大文件：graph.json/graph.enterprise.json 移出版本控制 | .gitignore + Git LFS | 运维 |
| W1 | 清理 .log/.runtime/.db 等运行时文件 | .gitignore | 运维 |
| W2 | 建设 mox-rpc：gRPC 拦截器链/客户端封装/服务端模板 | libs/mox-rpc/ | 后端 |
| W2 | 建设 mox-config：配置加载/热更新/K8s ConfigMap 集成 | libs/mox-config/ | 后端 |
| W2 | 建设 mox-o11y：日志/指标/链路追踪初始化 | libs/mox-o11y/ | 后端 |
| W3 | 建设 mox-db：PostgreSQL 连接池/租户上下文/RLS 集成/迁移工具 | libs/mox-db/ | 后端 |
| W3 | 建设 mox-tenant：租户上下文/VID 编解码/配额检查 | libs/mox-tenant/ | 后端 |
| W3 | 建设 mox-resilience：限流/熔断/降级/重试/超时/舱壁 | libs/mox-resilience/ | 后端 |
| W3 | 建设 mox-auth：JWT 管理/RBAC/权限检查 | libs/mox-auth/ | 后端 |
| W4 | 定义所有服务 Proto 接口（31个服务） | proto/ 目录 | 全组 |
| W4 | 搭建 K8s 开发环境（minikube/kind） | deploy/k8s/ 基础模板 | 运维 |
| W4 | 搭建 CI/CD（GitHub Actions + ArgoCD） | .github/workflows/ | 运维 |
| W4 | 基础设施部署（PostgreSQL/Redis/NATS/MinIO） | deploy/infra/ | 运维 |

### 2.3 里程碑

- **M1（W2末）**：目录重组完成，命名统一，共享库框架搭建
- **M2（W4末）**：所有共享库可用，Proto 接口定义完成，CI/CD 跑通，K8s 开发环境可用

### 2.4 验收标准

- [ ] 所有代码在新目录结构下编译通过
- [ ] 所有服务命名统一为 mox-{domain}-svc
- [ ] 大文件和运行时文件不再入库
- [ ] mox-rpc/mox-config/mox-o11y/mox-db/mox-tenant/mox-resilience/mox-auth 共享库可用
- [ ] 31个服务的 Proto 接口定义完成
- [ ] GitHub Actions CI 跑通（lint/test/build）
- [ ] K8s 开发环境可部署示例服务

---

## 三、阶段二：核心拆分期（第5-10周）

### 3.1 目标

- 拆分核心服务（网关/平台能力/图存储/AI），验证微服务架构可行性
- 实现多租户 L1 逻辑隔离
- 图存储引擎零修改，仅增加 gRPC 薄包装层
- AI 服务拆分 + Python 推理 sidecar

### 3.2 任务清单

| 周 | 任务 | 交付物 | 负责人 |
|----|------|--------|--------|
| W5 | 拆分 mox-gateway-svc：多协议单端口/REST→gRPC 转码/限流/路由 | services/gateway/ | 后端 |
| W5 | 拆分 mox-auth-svc：登录/登出/JWT/角色/权限 | services/auth/ | 后端 |
| W5 | 拆分 mox-tenant-svc：租户CRUD/配额管理/用量统计 | services/tenant/ | 后端 |
| W6 | 拆分 mox-system-svc：用户/角色/部门/菜单/字典/审计 | services/system/ | 后端 |
| W6 | 拆分 mox-metering-svc：用量采集/配额扣减/账单 | services/metering/ | 后端 |
| W6 | 拆分 mox-notification-svc：多渠道通知/模板/订阅 | services/notification/ | 后端 |
| W7 | 图存储 gRPC 化：增加 tonic Server 薄包装层（引擎零修改） | services/graph-storage/ | 后端 |
| W7 | 图存储多租户：VID 租户前缀编解码/CDC 租户过滤 | services/graph-storage/ | 后端 |
| W7 | 图存储 K8s StatefulSet 部署（RocksDB PVC + Raft） | deploy/k8s/graph-storage/ | 运维 |
| W8 | 拆分 mox-ai-svc：合并 flow-ai+mox-ai-core，AI 编排/Prompt/RAG/Guardrails | services/ai/ | 后端 |
| W8 | Python 推理 sidecar：gRPC streaming 接口/模型加载/Token 流式输出 | sidecar/inference/ | AI |
| W8 | AI 服务与 sidecar 集成（Unix Domain Socket 本地通信） | services/ai/ | 后端 |
| W9 | 多租户 L1 隔离：所有表加 tenant_id + PostgreSQL RLS + 拦截器 | 所有服务 | 全组 |
| W9 | 租户配额检查：网关/服务拦截器集成 tenant-svc | 所有服务 | 全组 |
| W9 | 数据迁移：现有数据添加 tenant_id（默认租户） | 迁移脚本 | 后端 |
| W10 | 核心服务集成测试：网关→auth→tenant→ai→graph-storage 全链路 | 测试用例 | 测试 |
| W10 | 核心服务压测：QPS/延迟/错误率/资源使用 | 压测报告 | 测试 |
| W10 | Bug 修复 + 性能调优 | - | 全组 |

### 3.3 里程碑

- **M3（W7末）**：网关+平台能力服务拆分完成，图存储 gRPC 化完成
- **M4（W10末）**：AI 服务拆分完成，多租户 L1 隔离完成，核心链路跑通

### 3.4 验收标准

- [ ] mox-gateway-svc 支持 REST/gRPC-Web/WebSocket 单端口
- [ ] mox-auth-svc 支持 JWT/OIDC/RBAC
- [ ] mox-tenant-svc 支持租户CRUD/配额/用量
- [ ] mox-system/metering/notification 服务可用
- [ ] mox-graph-storage-svc gRPC 接口可用，引擎代码零修改
- [ ] 图存储支持多租户（VID 前缀隔离）
- [ ] mox-ai-svc 支持 AI 编排/RAG/Guardrails/流式输出
- [ ] Python 推理 sidecar 与 Rust 服务通过 gRPC 通信
- [ ] 所有服务支持多租户 L1 逻辑隔离（tenant_id + RLS）
- [ ] 核心链路（网关→认证→AI→图谱）端到端跑通
- [ ] 核心服务压测达标（QPS/延迟目标）

---

## 四、阶段三：服务化推进（第11-16周）

### 4.1 目标

- 完成所有剩余服务的拆分（知识图谱/数据存储/流程算子/业务治理）
- 实现服务注册发现（K8s Service → Nacos）
- 实现异步事件通信（NATS JetStream）
- 所有服务独立部署、独立扩缩容

### 4.2 任务清单

| 周 | 任务 | 交付物 | 负责人 |
|----|------|--------|--------|
| W11 | 拆分 mox-graph-svc：合并 kg-hub+mox-graph-service，图谱CRUD/查询/本体/摄入 | services/graph/ | 后端 |
| W11 | 拆分 mox-graph-algo-svc：合并 graph-algorithms+mox-graph-spark，图算法 | services/graph-algo/ | 后端 |
| W12 | 拆分 mox-graph-streams-svc：CDC 订阅/实时增量计算/流式图处理 | services/graph-streams/ | 后端 |
| W12 | 拆分 mox-graph-meta-svc：Schema/索引/约束/版本/统计 | services/graph-meta/ | 后端 |
| W13 | 拆分 mox-storage-svc：合并4个 cloud-drive 模块，对象存储/国密加密/S3兼容 | services/storage/ | 后端 |
| W13 | 拆分 mox-etl-svc：数据源/管道/执行/调度（WASM 算子） | services/etl/ | 后端 |
| W14 | 拆分 mox-dataplane-svc：数据路由/同步/血缘/脱敏 | services/dataplane/ | 后端 |
| W14 | 拆分 mox-search-svc：全文+向量+图谱联合搜索 | services/search/ | 后端 |
| W15 | 拆分 mox-flow-svc：工作流定义/执行/节点/版本 | services/flow/ | 后端 |
| W15 | 拆分 mox-flow-fusion-svc：合并 primiflow-fusion+hermes-flow-bridge | services/flow-fusion/ | 后端 |
| W15 | 拆分 mox-operator-svc：算子注册/WASM管理/执行/市场 | services/operator/ | 后端 |
| W16 | 拆分 mox-compliance-svc：审计/合规/数据主体请求 | services/compliance/ | 后端 |
| W16 | 拆分 mox-fusion-svc：数据融合/实体对齐/冲突解决 | services/fusion/ | 后端 |
| W16 | 拆分 mox-catalog-svc：数据资产/术语/血缘 | services/catalog/ | 后端 |
| W16 | 拆分 mox-market-svc：模板/插件市场 | services/market/ | 后端 |
| W16 | 拆分 mox-optimizer-svc：查询优化/执行计划/成本模型 | services/optimizer/ | 后端 |
| W16 | NATS JetStream 集成：事件发布/订阅/Saga/DLQ | 所有服务 | 全组 |
| W16 | 服务注册发现：K8s Service（起步），预留 Nacos 接口 | 所有服务 | 运维 |

### 4.3 里程碑

- **M5（W13末）**：知识图谱+数据存储服务拆分完成
- **M6（W16末）**：所有31个服务拆分完成，异步事件通信可用

### 4.4 验收标准

- [ ] 所有31个服务独立编译、独立部署
- [ ] 知识图谱5个服务（graph/graph-storage/graph-algo/graph-streams/graph-meta）可用
- [ ] 数据存储4个服务（storage/etl/dataplane/search）可用
- [ ] 流程算子3个服务（flow/flow-fusion/operator）可用
- [ ] 业务治理5个服务（compliance/fusion/catalog/market/optimizer）可用
- [ ] NATS JetStream 事件通信可用（发布/订阅/重试/DLQ）
- [ ] 所有服务通过 K8s Service 互相发现
- [ ] 所有服务支持独立扩缩容（HPA）
- [ ] 全链路集成测试通过

---

## 五、阶段四：企业级增强（第17-20周）

### 5.1 目标

- 完善可观测性（日志/指标/链路追踪/告警）
- 安全加固（mTLS/RBAC/审计/加密/等保三级）
- 弹性容错（限流/熔断/降级/重试/超时/舱壁/DLQ）
- 灰度发布能力
- 性能优化

### 5.2 任务清单

| 周 | 任务 | 交付物 | 负责人 |
|----|------|--------|--------|
| W17 | 可观测性：OTel Collector 部署，所有服务集成 OTel SDK | deploy/o11y/ | 运维 |
| W17 | 可观测性：Jaeger 链路追踪部署 + 全链路 Trace 打通 | deploy/o11y/ | 运维 |
| W17 | 可观测性：Prometheus 指标采集 + Grafana 仪表盘（全局/服务/业务） | deploy/o11y/ | 运维 |
| W18 | 可观测性：Loki 日志聚合 + 日志规范统一 | deploy/o11y/ | 运维 |
| W18 | 可观测性：Alertmanager 告警规则（P0-P3）+ 多渠道通知 | deploy/o11y/ | 运维 |
| W18 | 安全：Istio mTLS 部署（服务间双向 TLS） | deploy/security/ | 运维 |
| W19 | 安全：RBAC 完善（系统角色/自定义角色/数据权限） | services/auth/ | 后端 |
| W19 | 安全：审计日志（不可篡改存储/哈希链/合规报告） | services/compliance/ | 后端 |
| W19 | 安全：加密（字段加密/国密SM4/密钥管理KMS） | 所有服务 | 后端 |
| W19 | 弹性：网关限流（租户/用户/IP/接口）+ 服务层限流 | gateway + 所有服务 | 后端 |
| W20 | 弹性：熔断器（所有 gRPC 客户端）+ 降级策略 | 所有服务 | 后端 |
| W20 | 弹性：重试（指数退避+抖动）+ 超时（多层级）+ 舱壁（资源隔离） | 所有服务 | 后端 |
| W20 | 灰度发布：Istio VirtualService 金丝雀发布 + 蓝绿部署 | deploy/istio/ | 运维 |
| W20 | 性能优化：全链路压测 + 瓶颈分析 + 调优（缓存/连接池/批量） | 压测报告 | 全组 |

### 5.3 里程碑

- **M7（W18末）**：可观测性完善，安全加固完成
- **M8（W20末）**：弹性容错完成，灰度发布可用，性能达标

### 5.4 验收标准

- [ ] 所有服务日志结构化、聚合到 Loki，可检索
- [ ] 所有服务指标采集到 Prometheus，Grafana 仪表盘可用
- [ ] 全链路 Trace 打通（网关→服务→数据库→第三方），Jaeger 可查
- [ ] P0-P3 告警规则配置完成，多渠道通知可用
- [ ] 服务间 mTLS 启用（Istio STRICT 模式）
- [ ] RBAC 完善（系统角色+自定义角色+数据权限）
- [ ] 审计日志不可篡改，合规报告可生成
- [ ] 敏感字段加密，国密算法支持
- [ ] 网关限流+服务层限流可用
- [ ] 熔断器+降级策略可用
- [ ] 重试+超时+舱壁可用
- [ ] 死信队列可用
- [ ] Istio 金丝雀发布可用（按比例/Header路由）
- [ ] 全链路压测达标（SLA 99.95%，P99<100ms）

---

## 六、阶段五：稳定运营期（第21周+）

### 6.1 目标

- 系统稳定运行，SLA 达标
- 持续优化和迭代
- 完善文档和知识转移

### 6.2 持续任务

| 任务 | 频率 | 说明 |
|------|------|------|
| 全链路压测 | 每月 | 验证容量和性能 |
| 混沌工程 | 每季度 | 注入故障，验证弹性 |
| 安全审计 | 每季度 | 漏洞扫描/渗透测试/合规检查 |
| 容量规划 | 每季度 | 根据增长预测调整资源 |
| 成本优化 | 每月 | 资源使用率分析，降本增效 |
| 文档更新 | 持续 | 架构文档/API文档/运维手册 |
| 技术债清理 | 持续 | 代码质量/重构/依赖升级 |

### 6.3 SLA 目标

| 指标 | 目标 |
|------|------|
| 服务可用性 | 99.95%（月停机<22分钟） |
| API P99 延迟 | <100ms（普通请求） |
| AI 流式首包延迟 | <500ms |
| 数据持久性 | 99.999999999%（11个9） |
| RPO（恢复点目标） | <1分钟 |
| RTO（恢复时间目标） | <15分钟 |
| 故障恢复时间 | <30分钟 |

---

## 七、风险与应对

| 风险 | 概率 | 影响 | 应对措施 |
|------|------|------|----------|
| 图存储 gRPC 化引入性能损耗 | 中 | 高 | 压测验证，Unix Domain Socket 本地通信，批量接口优化 |
| 多租户隔离导致性能下降 | 中 | 中 | RLS 索引优化，租户前缀分片，L2/L3 隔离升级路径 |
| 服务拆分后分布式事务复杂 | 高 | 高 | Saga 模式（事件驱动），最终一致性，幂等性保证 |
| 服务间调用延迟增加 | 中 | 中 | gRPC 高性能，连接复用，客户端负载均衡，批量接口 |
| Python sidecar 部署复杂 | 中 | 中 | 容器化，K8s sidecar 模式，gRPC 标准化接口 |
| 团队 Rust 技能不足 | 中 | 高 | 培训/代码规范/Code Review/共享库降低开发门槛 |
| 迁移期间数据不一致 | 中 | 高 | 双写过渡期，数据对账，回滚方案 |
| 性能不达标 | 中 | 高 | 提前压测，性能预算，缓存优化，异步化 |
| 安全合规不达标 | 低 | 高 | 等保三级标准设计，安全审计，渗透测试 |
| 工期延误 | 高 | 中 | 优先级管理，MVP 先行，渐进式拆分，可随时回退 |

---

## 八、团队配置建议

| 角色 | 人数 | 职责 |
|------|------|------|
| 架构师 | 1 | 整体架构设计/技术选型/代码规范/关键模块 |
| 后端工程师（Rust） | 4-6 | 服务拆分/共享库/业务逻辑 |
| AI 工程师（Python） | 1-2 | 推理 sidecar/模型优化/RAG |
| 运维/DevOps | 1-2 | K8s/CI/CD/监控/基础设施 |
| 测试工程师 | 1-2 | 集成测试/压测/混沌工程 |
| 产品经理 | 1 | 需求管理/优先级/验收 |

**总计：10-15人**

---

## 九、总结

实施路线图遵循**"渐进式拆分、MVP 先行、可随时回退"**原则：

1. **阶段一（1-4周）**：基础建设——工程化改造+共享库+Proto+基础设施
2. **阶段二（5-10周）**：核心拆分——网关+平台能力+图存储gRPC化+AI+多租户
3. **阶段三（11-16周）**：服务化推进——所有31个服务拆分+异步事件通信
4. **阶段四（17-20周）**：企业级增强——可观测性+安全+弹性+灰度+性能
5. **阶段五（21周+）**：稳定运营——SLA保障+持续优化+混沌工程

**关键原则**：
- 图存储引擎零修改，仅加 gRPC 薄包装层
- 多租户从 L1 逻辑隔离起步，预留 L2/L3 升级路径
- 每个服务拆分后必须可独立部署、独立测试、独立回滚
- 渐进式拆分，任何阶段都可以暂停或回退
- 优先拆分高价值/高负载/故障影响大的服务

---

*文档导航：[README](./README.md) | [00-核心原则](./00-principles.md) | [01-服务边界](./01-service-boundaries.md) | [02-通信架构](./02-communication.md) | [03-数据架构](./03-data.md) | [04-部署架构](./04-deployment.md) | [05-可观测性·安全·弹性](./05-observability-security-resilience.md) | [06-实施路线图](./06-roadmap.md)*

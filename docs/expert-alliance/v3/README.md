---
title: Mox 专家联盟 v3 — mox 模块化系统架构优化架构
version: V3.0
authority: 🟡参考
doc_id: EA-DOC-023
last_updated: 2026-08-31
source_of_truth: 参考（导航页）
---
# Mox 专家联盟 v3 — mox 模块化系统架构优化架构

> **版本**：v3.0（mox 模块化系统架构优化版）
> **日期**：2026-08-26
> **基于**：v2.0 企业级mox 模块化系统架构版
> **状态**：优化定稿

---

## v3 核心优化

v2 → v3 的关键改进：

| 优化项 | v2 | v3 | 收益 |
|--------|----|----|------|
| **服务拆分** | 5服务（alliance过重） | 7服务（拆分alliance为scheduler/executor/fusion） | 独立扩缩/故障隔离/迭代独立 |
| **专家匹配** | 2次RPC（scheduler→registry→kg） | 1次RPC（scheduler内嵌图谱查询） | 匹配延迟降60% |
| **记忆管理** | 分散3处（alliance内存+Redis+图谱） | 统一记忆服务expert-memory | 一致性提升/查询简化 |
| **有状态服务** | agent-svc有状态（Agent实例在内存） | 全无状态（状态外部化到Redis/PG） | Pod可随时重启/HPA自由缩容 |
| **协议端口** | 单端口混合HTTP/1.1+HTTP/2 | 双端口分流（:8080 HTTP / :50051 gRPC） | 内部gRPC性能最优/消除ALPN兼容问题 |
| **结果融合** | 同步阻塞在alliance-svc | 独立fusion服务可异步 | 不阻塞主执行链路 |
| **并发任务** | 预估100 | 预估200 | 提升100% |

---

## 文档导航

### 架构设计文档（正式发布）

| # | 文档 | 格式 | 核心内容 |
|---|------|------|----------|
| 01 | [系统架构设计文档](docs/expert-alliance/architecture/system-architecture-design.html) | 🌐 HTML | 9章完整架构设计：分层架构/7服务设计/数据架构/接口规范/非功能设计/ADR |
| 02 | [部署指南](docs/expert-alliance/architecture/deployment-guide.html) | 🌐 HTML | 9章完整部署：资源要求/Helm一键部署/高可用配置/升级回滚/验证验收/FAQ |
| 03 | [运维手册](docs/expert-alliance/architecture/ops-manual.html) | 🌐 HTML | 8章运维全册：4级巡检/监控告警/备份恢复/7条Runbook/容量管理/安全运维 |

### 架构优化分析（设计过程）

| # | 文档 | 核心内容 |
|---|------|----------|
| 01 | [架构优化分析](docs/expert-alliance/v3/01-architecture-optimization.md) | v2问题mox 模块化系统架构审计/5大优化点详解/v2 vs v3对比/性能预估 |
| 02 | [架构需求矩阵](docs/expert-alliance/v3/02-requirements-matrix.md) | 42项功能需求/40项非功能需求/服务-需求映射/依赖矩阵 |
| 03 | [mox 模块化系统架构业务流程图](docs/expert-alliance/v3/03-business-flow-diagrams.md) | **16张Mermaid图**：架构图/主流程/匹配/执行/ReAct/融合/部署/状态机 |

---

## v3 架构总览

### 7服务 + 1 Sidecar

```
接入层
  gateway-http  :8080  REST/JSON-RPC/MCP/WebSocket（对外）
  gateway-grpc  :50051 gRPC（内部服务间）

联盟核心层
  alliance-scheduler   任务调度/专家匹配(合并图谱推理)/计划生成/案例检索
  alliance-executor    DAG执行/节点调度/进度推送/人工干预（全无状态）
  alliance-fusion      结果融合(6种策略)/质量评估/迭代精炼（独立扩展）

能力层
  expert-registry      专家CRUD/定义验证/健康检查/工具自动发现（精简，匹配移走）
  expert-agent         Agent运行时/ReAct循环/工具调用/AI推理（全无状态）
  expert-memory        统一记忆抽象/案例库/图谱学习/边权重更新

Sidecar
  ai-inference         Python AI推理（UDS通信，与agent同Pod）
```

### 关键数据流

```
用户 → gateway-http → scheduler(匹配+计划) → executor(DAG执行)
                                          ↓
                                    agent(ReAct+工具+AI)
                                          ↓
                                    底层31个微服务
                                          ↓
                                    fusion(结果融合)
                                          ↓
                                    memory(记忆+案例+图谱学习)
```

---

## 16张业务流程图速览

| # | 图名 | 类型 | 说明 |
|---|------|------|------|
| 1 | 系统总体架构图 | graph TB | 7服务+数据层+底层服务的完整关系 |
| 2 | 端到端主流程 | flowchart TD | 任务创建→匹配→计划→执行→融合→记忆→交付，11步全链路 |
| 3 | 专家匹配流程 | flowchart TD | 任务解析→图谱推理→6维评分→筛选输出 |
| 4 | 协作计划生成 | flowchart TD | 模式选择→任务分解→依赖分析→验证输出 |
| 5 | DAG执行引擎 | flowchart TD | 初始化→调度循环→节点执行→依赖更新→进度推送 |
| 6 | Agent ReAct循环 | flowchart TD | 理解→规划→执行→观察→审核，5步迭代 |
| 7 | 结果融合流程 | flowchart TD | 6种策略路由→质量评估→迭代精炼 |
| 8 | 协作记忆与图谱学习 | flowchart TD | 工作记忆归档→会话更新→案例提升→图谱边权重更新 |
| 9 | 异常处理流程 | flowchart TD | 错误分类→重试→替代专家→降级跳过→任务失败 |
| 10 | 人工干预流程 | flowchart TD | 暂停→审核页面→6种操作（通过/拒绝/修改/指定/跳过/取消） |
| 11 | MCP调用流程 | sequenceDiagram | 初始化→工具列表→工具调用→JSON-RPC转gRPC |
| 12 | 多协议网关路由 | flowchart TD | 端口分流→Path路由→5种协议处理路径 |
| 13 | 服务间调用时序图 | sequenceDiagram | 完整端到端gRPC/NATS/WebSocket调用时序 |
| 14 | 知识图谱关联关系图 | graph LR | 7种顶点+12种边的完整关联网络 |
| 15 | 部署架构图 | graph TB | K8s命名空间/StatefulSet/HPA/可观测全栈 |
| 16 | 状态机总图 | stateDiagram-v2 | 任务状态机+节点子状态机 |

---

## 性能指标对比

| 指标 | v2 预估 | v3 预估 | 提升 |
|------|---------|---------|------|
| 任务创建→计划生成 | ~300ms | ~150ms | **50%** |
| 专家匹配延迟 | ~200ms | ~80ms | **60%** |
| 节点调度延迟 | ~50ms | ~20ms | **60%** |
| 内部gRPC调用 | ~1ms(含路由) | ~0.5ms | **50%** |
| 并发任务数 | 100 | 200 | **100%** |
| 故障恢复时间 | ~5min | ~30s | **90%** |

---

## 不变的核心设计

v3 保持 v2 的核心设计不变：

- **知识图谱驱动**：六元关联网络（7顶点+12边）驱动匹配/编排/融合/学习
- **6种协作模式**：串行/并行/辩论/分层/迭代/动态
- **6种融合策略**：多数投票/加权投票/拼接合并/择优选择/辩论仲裁/迭代精炼
- **10个内置专家**：图谱构建/数据分析/AI推理/安全审计/流程自动化/数据治理/知识融合/搜索推荐/运维监控/联盟协调
- **5协议支持**：gRPC/JSON-RPC/MCP/REST/WebSocket（仅端口分流优化）
- **图存储零修改**：自研图存储引擎不改，gRPC薄包装
- **多租户三档隔离**：L1逻辑（默认）/L2 Schema/L3集群
- **复用31个现有微服务**：不重复开发底层能力
- **mox 模块化系统架构Rust后端**：核心Rust，AI推理Python sidecar

---

## 快速开始

### REST API

```bash
# 创建任务
curl -X POST http://localhost:8080/api/v1/expert/tasks \
  -H "Authorization: Bearer <jwt>" \
  -H "Content-Type: application/json" \
  -d '{"title":"图谱构建","description":"把data.csv构建成知识图谱","preference":{"mode":"AUTO"}}'

# 查询任务
curl http://localhost:8080/api/v1/expert/tasks/<task_id> \
  -H "Authorization: Bearer <jwt>"

# WebSocket实时进度
ws://localhost:8080/ws/v1/expert/tasks/<task_id>/progress?token=<jwt>
```

### gRPC（内部服务间）

```bash
# 内部服务直接走 :50051 gRPC，零路由开销
grpcurl -plaintext localhost:50051 mox.expert.alliance.v1.ExpertAllianceService/CreateTask
```

### MCP（Claude Desktop）

```json
{
  "mcpServers": {
    "mox-expert": {
      "url": "http://localhost:8080/mcp",
      "headers": { "Authorization": "Bearer <jwt>" }
    }
  }
}
```

---

## 版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| v1.0 | 2026-08-26 | 初始设计：3篇文档（总览/注册协议/图谱Schema） |
| v2.0 | 2026-08-26 | 企业级mox 模块化系统架构版：9篇文档，mox 模块化系统架构需求/归一化模型/全路径流程/多协议/数据/安全/路线图 |
| **v3.0** | **2026-08-26** | **mox 模块化系统架构优化版：4篇文档，v2问题审计/5大架构优化/42+40需求矩阵/16张Mermaidmox 模块化系统架构流程图** |

---

*文档导航：[01-架构优化分析](docs/expert-alliance/v3/01-architecture-optimization.md) | [02-架构需求矩阵](docs/expert-alliance/v3/02-requirements-matrix.md) | [03-mox 模块化系统架构业务流程图](docs/expert-alliance/v3/03-business-flow-diagrams.md)*

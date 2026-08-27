# 架构文档索引 — Architecture Documentation Index

> 本文档索引列出Mox Platform所有架构相关文档，便于快速定位。

---

## 核心文档

| 文档 | 路径 | 说明 |
|------|------|------|
| 架构总览 | [`ARCHITECTURE.md`](../../ARCHITECTURE.md) | 企业级全维归一化架构文档（主文档） |
| 扩展开发指南 | [`02-extension-guide.md`](./02-extension-guide.md) | 零改动核心架构的扩展开发完整指南 |
| RPC快速对接手册 | [`06-rpc-integration-guide.md`](./06-rpc-integration-guide.md) | 新系统RPC/gRPC/REST快速对接详细手册（内容发布场景） |
| 错误码参考手册 | [`04-error-code-reference.md`](./04-error-code-reference.md) | 6位错误码体系完整参考 |
| 归一化检查清单 | [`05-normalization-checklist.md`](./05-normalization-checklist.md) | 架构归一化验证清单（10大类） |

---

## 文档导航

### 1. 架构总览 (ARCHITECTURE.md)
- 设计哲学与技术栈
- 6层分层架构详解
- 核心模块清单（L1-L6）
- 命名规范归一化
- 目录结构规范
- 零改动扩展指南
- 依赖关系图
- 企业级处理流程
- 配置参考

### 2. 扩展开发指南 (02-extension-guide.md)
- 扩展模式总览（Trait + Factory + 配置）
- 新增AI Provider完整示例
- 新增连接器
- 新增SSO协议
- 开发WASM插件（mox-plugin-sdk）
- 新增协议接入
- 替换合规实现
- 测试指南

### 3. 错误码参考手册 (04-error-code-reference.md)
- 错误码编码规则
- 系统错误 (10xxxx)
- AI错误 (20xxxx)
- 插件错误 (30xxxx)
- 政企错误 (40xxxx)
- 连接器错误 (50xxxx)
- 集成错误 (90xxxx)
- HTTP状态码映射
- 错误响应格式

### 4. 归一化检查清单 (05-normalization-checklist.md)
- 命名归一化
- 结构归一化
- 依赖归一化
- 扩展归一化
- 错误处理归一化
- 配置归一化
- 文档归一化
- 测试归一化
- 安全归一化
- 性能归一化

---

## 快速参考

### 架构分层
```
L6 接入层     → Gateway + API
L5 集成层     → mox-platform-integration-core (核心枢纽)
L4 对接能力层 → AI / Plugin / Enterprise / Connector
L3 领域服务层 → 8域 (kg/ai/flow/data/cloud/voice/market/platform)
L2 平台核心层 → iam/system/meta/orchestrator/datastore/operator
L1 基础框架层 → framework/foundation/observability
```

### 扩展模式
```
实现Trait → 实现Factory → 注册到Registry → 加配置 → 自动组装
核心代码零改动 ✅
```

### 错误码分类
```
10xxxx 系统  | 20xxxx AI  | 30xxxx 插件
40xxxx 政企  | 50xxxx 连接器 | 90xxxx 集成
```

---

## 文档维护

- **更新频率**: 架构变更时同步更新
- **负责人**: 架构开发联盟
- **最后更新**: 2026-08-27
- **版本**: 3.0.0-ai-powered

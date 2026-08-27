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
| KG驱动动态SQL架构 | [`07-KG-DYNAMIC-SQL-ARCHITECTURE.md`](./07-KG-DYNAMIC-SQL-ARCHITECTURE.md) | 知识图谱+动态SQL配置平台，字段级权限，全维架构设计 |
| 全维低代码架构 | [`08-FULL-DIMENSION-LOWCODE-ARCHITECTURE.md`](./08-FULL-DIMENSION-LOWCODE-ARCHITECTURE.md) | 全维低代码模块，自研KG融合，九层架构，企业级全链路 |
| RocksDB性能优化 | [`09-ROCKSDB-PERFORMANCE-OPTIMIZATION.md`](./09-ROCKSDB-PERFORMANCE-OPTIMIZATION.md) | rust-rocksdb FFI开销全维分析与优化，生产级配置 |

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

### 5. KG驱动动态SQL架构 (07-KG-DYNAMIC-SQL-ARCHITECTURE.md)
- 七层架构模型（智能层/行业层/扩展层/配置层/编排层/缓存层/执行层/适配层/元数据层）
- 自研KG深度集成（12类节点/18类关系/4大核心能力）
- 字段级权限控制体系（三层模型/5种权限类型/8种脱敏函数）
- 自定义权限配置引擎（4种策略类型/冲突解决/仿真测试）
- 动态执行引擎（AOT编译+多级缓存+超越写死SQL）
- 企业级全链路处理流程（配置/执行/运维）
- 实施路线图（3阶段11周）

### 6. 全维低代码架构 (08-FULL-DIMENSION-LOWCODE-ARCHITECTURE.md)
- 主流低代码平台全维分析（Mendix/OutSystems/Appian/宜搭/微搭/明道云/简道云）
- 八大共性问题与解决方案矩阵
- 九层架构模型
- 自研KG深度集成（复用mox-kg-storage-svc/hub/algo等8个模块）
- 元数据驱动的全维建模（8层颗粒度）
- 动态执行引擎（超越写死SQL的五大策略）
- 全维处理中心（4大处理通道）
- 企业级全链路处理流程
- 无限扩展机制（12类SPI扩展点+插件运行时+动态Schema）
- 行业融合引擎（行业包体系+多包自动融合）
- 实施路线图（4阶段18周）
- 可行性全维评估

### 7. RocksDB性能优化 (09-ROCKSDB-PERFORMANCE-OPTIMIZATION.md)
- rust-rocksdb vs 原生C++ RocksDB性能真相
- FFI开销量级分析（6类场景）
- 已实施的优化措施（Release LTO/生产级Options/CF缓存/MultiGet等12项）
- 待实施优化建议（Rust侧内存缓存/避免回调/批量写入等）
- 性能预期对比表
- 快速部署指南（编译/环境变量/系统调优）
- 选型结论

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

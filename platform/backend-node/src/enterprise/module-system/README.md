# MOX Enterprise · 模块化架构系统

> 将 A–O 共 34 个企业级文件从"分散脚本"升级为"统一注册、统一生命周期、统一依赖注入、统一健康聚合"的模块化系统。

## 架构总览

```
┌─────────────────────────────────────────────────────────────┐
│                      AppBootstrap (编排器)                    │
│  pre-bootstrap → bootstrap → module-init → module-start     │
│  → post-bootstrap → ready  |  shutdown: drain → stop → exit │
└──────────────┬──────────────────────────────────────────────┘
               │
    ┌──────────┼──────────┬──────────┬──────────┐
    ▼          ▼          ▼          ▼          ▼
┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐
│Registry│ │DI Cont │ │ Config │ │EventBus│ │Health  │
│ 注册中心│ │ 依赖注入│ │ 配置中心│ │ 事件总线│ │ 健康聚合│
└───┬────┘ └───┬────┘ └───┬────┘ └───┬────┘ └───┬────┘
    │           │           │           │           │
    └───────────┴─────┬─────┴───────────┴───────────┘
                        ▼
              ┌──────────────────┐
              │ ModuleLifecycle  │
              │  生命周期管理器    │
              │  init→start→stop │
              └────────┬─────────┘
                       │
          ┌────────────┼────────────┐
          ▼            ▼            ▼
    ┌──────────┐ ┌──────────┐ ┌──────────┐
    │Middleware│ │  Router  │ │Dependency│
    │ 中间件装配│ │ 路由聚合  │ │ 依赖图    │
    └──────────┘ └──────────┘ └──────────┘
```

## 核心组件

| 组件 | 文件 | 职责 |
|---|---|---|
| 模块注册中心 | `module-registry.js` | 统一注册、发现、查询所有模块；能力/标签索引；状态机管理 |
| 依赖注入容器 | `di-container.js` | 服务注册/解析；4种作用域；循环依赖检测；子容器 |
| 企业级配置中心 | `enterprise-config.js` | 多环境配置；5级优先级；热更新；AES-256加密；Schema校验；版本回滚 |
| 依赖图与拓扑排序 | `dependency-graph.js` | DAG构建；Kahn拓扑排序；循环检测；并行启动组；影响分析；Mermaid/DOT导出 |
| 模块生命周期管理器 | `module-lifecycle.js` | 按拓扑顺序init/start；超时控制；健康检查调度；优雅关闭；降级模式 |
| 事件总线 | `event-bus.js` | 发布/订阅；通配符；5级优先级；死信队列；重试；RPC over EventBus |
| 健康检查聚合器 | `health-aggregator.js` | liveness/readiness/startup/deep；K8s探针；可用性统计；健康历史 |
| 中间件装配器 | `middleware-assembler.js` | 8阶段优先级；条件挂载；热更新；性能监控；依赖排序 |
| 路由聚合器 | `router-aggregator.js` | API版本控制；权限声明；OpenAPI 3.0自动生成；冲突检测 |
| 应用启动编排器 | `app-bootstrap.js` | 6阶段启动；优雅关闭；信号处理；未捕获异常；完整启动报告 |
| 模块统一索引 | `module-index.js` | A–O全部34模块的注册清单；一键注册；依赖图数据导出 |

## 快速开始

```javascript
const { AppBootstrap } = require('./src/enterprise/module-system/app-bootstrap');
const { registerAllModules } = require('./src/enterprise/module-system/module-index');

// 1. 创建应用
const app = new AppBootstrap({
  appName: 'MOX Enterprise',
  version: '2.0.0',
  shutdownTimeoutMs: 30000,
});

// 2. 注册所有模块（A–O + 模块系统自身，共34个）
const result = registerAllModules(app);
console.log(`注册了 ${result.registered}/${result.total} 个模块`);

// 3. 启动（自动按拓扑顺序初始化+启动）
const report = await app.bootstrap();
console.log(`应用就绪，耗时 ${report.totalDurationMs}ms`);

// 4. 健康检查
const health = await app.health.getAggregatedHealth();

// 5. 优雅关闭（SIGTERM/SIGINT 自动触发）
// await app.shutdown('SIGTERM');
```

## 启动流程

```
Phase 1: pre-bootstrap
  ├── 执行预启动钩子
  ├── 加载配置（默认→文件→环境变量→远程→运行时）
  └── 环境检查（Node版本/内存/必要环境变量）

Phase 2: bootstrap
  ├── 注册核心健康检查器（app/module-registry/event-bus）
  └── 注册核心事件处理器

Phase 3: module-init (按拓扑顺序，同层并行)
  ├── Layer 0: 无依赖模块（config/registry/di/event-bus...）
  ├── Layer 1: 依赖Layer 0的模块
  ├── Layer 2: 依赖Layer 1的模块
  └── ...

Phase 4: module-start (按拓扑顺序，同层并行)
  ├── 调用每个模块的 start()
  ├── 启动健康检查循环（30s间隔）
  └── 标记模块为 READY

Phase 5: post-bootstrap
  ├── 执行后启动钩子
  ├── 检测路由冲突
  └── 挂载中间件和路由

Phase 6: ready
  └── 应用就绪，接收流量
```

## 关闭流程（优雅关闭）

```
1. pre-shutdown: 执行预关闭钩子
2. drain: 停止接收新请求，等待进行中请求完成（10s超时）
3. module-stop: 按反向拓扑顺序停止模块（15s/模块超时）
4. shutdown: 关闭事件总线/健康聚合/DI容器
5. exit: 进程退出
```

## 模块状态机

```
unregistered → registered → initializing → ready
                                    ↓
                                  degraded
                                    ↓
                            stopping → stopped
                                    ↓
                                  error
```

## 依赖图统计

- **总模块数**: 34（A–O 交付的24个业务模块 + 模块系统自身10个核心组件）
- **分类数**: 12（core/storage/compute/security/observability/multi_region/data_lake/finops/multi_tenant/disaster_recovery/devops/module_system）
- **最大依赖深度**: 4 层（app-bootstrap → module-lifecycle → dependency-graph → module-registry）
- **无依赖模块**: 8 个（config/registry/di/event-bus/health/middleware/prometheus/alertmanager）

## 与 MOX 现有代码的集成

模块系统不替换 MOX 现有代码，而是在其上叠加一层统一管理：

1. **现有 `src/storage/`、`src/file-store.js`、`src/config.js`** → 通过模块描述符的 `init` 函数延迟加载
2. **现有 Express 应用** → 通过 `middleware-assembler` 和 `router-aggregator` 统一装配
3. **现有 SQLite/PG 切换** → 通过 `enterprise-config` 统一管理配置，`pg-shard-router` 提供分片能力
4. **现有 FS/S3 chunk 存储** → 通过 `module-registry` 注册为 storage 类模块，纳入健康检查

## 扩展新模块

```javascript
// 1. 在 module-index.js 的 MODULE_MANIFEST 中添加
{
  name: 'my-new-module',
  version: '1.0.0',
  category: MODULE_CATEGORY.COMPUTE,
  description: '我的新模块',
  path: '../my-module/index.js',
  capabilities: ['my-capability'],
  tags: ['custom'],
  dependencies: ['enterprise-config', 'event-bus'],
}

// 2. 模块文件导出标准接口
module.exports = {
  async start() { /* 启动逻辑 */ },
  async stop() { /* 关闭逻辑 */ },
  async healthCheck() { return { status: 'healthy' }; },
};

// 3. registerAllModules(app) 会自动发现并注册
```

## 设计原则

1. **零侵入**: 不修改现有业务代码，通过描述符包装
2. **可降级**: 非核心模块加载失败不阻断启动，标记为 degraded
3. **可观测**: 每个模块都有健康检查、状态追踪、性能统计
4. **可演进**: 支持热更新配置、动态注册/注销模块
5. **一致性**: 所有模块遵循统一的生命周期、配置、事件规范
